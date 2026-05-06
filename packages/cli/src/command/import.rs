use std::collections::{HashMap, HashSet};
use std::time::Duration;

use tokio::time::sleep;

use anyhow::{bail, Context, Result};
use groupshop_backend_shared::prelude::*;
use reqwest::Client;
use serde::Deserialize;

const HEADER_SESSION_TOKEN: &str = "x-groupshop-session-token";

#[derive(Debug, Deserialize)]
struct CsvRow {
    #[serde(rename = "GTIN")]
    gtin: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Category")]
    category: String,
    #[serde(rename = "Brand")]
    brand: String,
    #[serde(rename = "$ Lowest Price inc. shipping")]
    price: String,
    #[serde(rename = "Unit")]
    unit: String,
    #[serde(rename = "Lowest Priced Offer Inventory")]
    inventory: String,
    #[serde(rename = "Is a pre-order?")]
    is_preorder: String,
    #[serde(rename = "Estimated Delivery Time (weeks)")]
    delivery_weeks: String,
    #[serde(rename = "Product URL")]
    supplier_url: String,
    #[serde(rename = "Image URL")]
    image_url: String,
}

struct ApiSession {
    client: Client,
    api_url: String,
    session_token: String,
}

impl ApiSession {
    async fn post_json<Req: serde::Serialize, Res: serde::de::DeserializeOwned>(
        &self,
        route: &str,
        body: &Req,
    ) -> Result<Res> {
        let url = format!("{}/{}", self.api_url.trim_end_matches('/'), route);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.session_token))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .with_context(|| format!("POST {route}"))?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            bail!("POST {route} returned {status}: {text}");
        }
        serde_json::from_str(&text).with_context(|| format!("parse response from {route}"))
    }

    async fn post_no_response<Req: serde::Serialize>(&self, route: &str, body: &Req) -> Result<()> {
        let url = format!("{}/{}", self.api_url.trim_end_matches('/'), route);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.session_token))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .with_context(|| format!("POST {route}"))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await?;
            bail!("POST {route} returned {status}: {text}");
        }
        Ok(())
    }
}

fn slugify(input: &str) -> String {
    let slug: String = input
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();

    // Collapse consecutive hyphens
    let mut result = String::with_capacity(slug.len());
    let mut prev_hyphen = false;
    for c in slug.chars() {
        if c == '-' {
            if !prev_hyphen {
                result.push(c);
            }
            prev_hyphen = true;
        } else {
            result.push(c);
            prev_hyphen = false;
        }
    }

    result.trim_matches('-').to_string()
}

pub async fn run(
    csv_path: &str,
    api_url: &str,
    email: &str,
    password: &str,
    max_per_category: usize,
    min_moq: u32,
    dry_run: bool,
) -> Result<()> {
    // 1. Parse CSV
    println!("Reading CSV: {csv_path}");
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(csv_path)?;
    let mut rows: Vec<CsvRow> = Vec::new();
    for result in reader.deserialize() {
        match result {
            Ok(row) => rows.push(row),
            Err(e) => {
                eprintln!("  Skipping malformed row: {e}");
            }
        }
    }
    println!("Parsed {} rows", rows.len());

    // 1b. Filter by minimum MOQ
    if min_moq > 0 {
        let before = rows.len();
        rows.retain(|row| row.unit.parse::<u32>().unwrap_or(1) >= min_moq);
        println!(
            "Filtered by min MOQ {min_moq}: {} -> {} rows ({} removed)",
            before,
            rows.len(),
            before - rows.len()
        );
    }

    // 1c. Filter out placeholder/missing images
    {
        let before = rows.len();
        rows.retain(|row| {
            !row.image_url
                .contains("6f37ced36498c7df3a3897a9dbbb3384.jpg")
        });
        let removed = before - rows.len();
        if removed > 0 {
            println!(
                "Filtered by valid image: {} -> {} rows ({} removed)",
                before,
                rows.len(),
                removed
            );
        }
    }

    // 2. Extract unique categories and brands
    let mut category_names: HashSet<String> = HashSet::new();
    let mut brand_names: HashSet<String> = HashSet::new();
    for row in &rows {
        category_names.insert(row.category.clone());
        brand_names.insert(row.brand.clone());
    }

    // Build slug maps
    let category_slugs: HashMap<String, String> = category_names
        .iter()
        .map(|name| (name.clone(), slugify(name)))
        .collect();
    let brand_slugs: HashMap<String, String> = brand_names
        .iter()
        .map(|name| (name.clone(), slugify(name)))
        .collect();

    // 3. Group products by category and apply max_per_category
    let mut by_category: HashMap<String, Vec<&CsvRow>> = HashMap::new();
    for row in &rows {
        by_category
            .entry(row.category.clone())
            .or_default()
            .push(row);
    }

    // Sort each category by inventory descending, then take top N
    let mut selected_rows: Vec<&CsvRow> = Vec::new();
    for (_cat, mut products) in by_category {
        products.sort_by(|a, b| {
            let inv_a: u32 = a.inventory.parse().unwrap_or(0);
            let inv_b: u32 = b.inventory.parse().unwrap_or(0);
            inv_b.cmp(&inv_a)
        });
        if max_per_category > 0 {
            products.truncate(max_per_category);
        }
        selected_rows.extend(products);
    }

    // Deduplicate categories/brands actually used
    let mut used_categories: HashSet<String> = HashSet::new();
    let mut used_brands: HashSet<String> = HashSet::new();
    for row in &selected_rows {
        used_categories.insert(row.category.clone());
        used_brands.insert(row.brand.clone());
    }

    println!(
        "Selected {} products across {} categories, {} brands",
        selected_rows.len(),
        used_categories.len(),
        used_brands.len()
    );

    if dry_run {
        println!("Dry run complete. No changes made.");
        return Ok(());
    }

    // 4. Authenticate
    println!("Signing in as {email}...");
    let client = Client::builder()
        .cookie_store(true)
        .timeout(Duration::from_secs(30))
        .build()?;

    let signin_url = format!(
        "{}/auth/email-password/signin",
        api_url.trim_end_matches('/')
    );
    let signin_resp = client
        .post(&signin_url)
        .header("Content-Type", "application/json")
        .json(&AuthEmailPasswordSigninRequest {
            email: email.to_string(),
            password: password.to_string(),
        })
        .send()
        .await
        .context("signin request")?;

    if !signin_resp.status().is_success() {
        let text = signin_resp.text().await?;
        bail!("Sign-in failed: {text}");
    }

    // 5. Refresh to get session token
    let refresh_url = format!("{}/auth/refresh", api_url.trim_end_matches('/'));
    let refresh_resp = client
        .post(&refresh_url)
        .send()
        .await
        .context("refresh request")?;

    let session_token = refresh_resp
        .headers()
        .get(HEADER_SESSION_TOKEN)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .context("no session token in refresh response")?;

    if !refresh_resp.status().is_success() {
        let text = refresh_resp.text().await?;
        bail!("Refresh failed: {text}");
    }

    println!("Authenticated successfully");

    let api = ApiSession {
        client,
        api_url: api_url.to_string(),
        session_token,
    };

    // 7. Create categories
    let mut cat_created = 0u32;
    let mut cat_skipped = 0u32;
    let cat_total = used_categories.len();
    for (cat_i, name) in used_categories.iter().enumerate() {
        if cat_i % 50 == 0 {
            println!("  Categories: {cat_i}/{cat_total}...");
        }
        let slug = &category_slugs[name];
        let id = match ProductCategoryId::new(slug) {
            Ok(id) => id,
            Err(e) => {
                eprintln!("  Skip category {name:?} (bad slug {slug:?}): {e}");
                cat_skipped += 1;
                continue;
            }
        };

        match api
            .post_json::<_, AdminCreateCategoryResponse>(
                "admin/create-category",
                &AdminCreateCategoryRequest {
                    id,
                    name: name.clone(),
                    parent_id: None,
                },
            )
            .await
        {
            Ok(_) => cat_created += 1,
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("UNIQUE") || msg.contains("unique") || msg.contains("already") {
                    cat_skipped += 1;
                } else {
                    eprintln!("  Error creating category {name:?}: {e}");
                    cat_skipped += 1;
                }
            }
        }
    }
    println!("Categories: {cat_created} created, {cat_skipped} skipped");

    // 8. Create brands
    let mut brand_created = 0u32;
    let mut brand_skipped = 0u32;
    let brand_total = used_brands.len();
    for (brand_i, name) in used_brands.iter().enumerate() {
        if brand_i % 100 == 0 {
            println!("  Brands: {brand_i}/{brand_total}...");
        }
        let slug = &brand_slugs[name];
        let id = match ProductBrandId::new(slug) {
            Ok(id) => id,
            Err(e) => {
                eprintln!("  Skip brand {name:?} (bad slug {slug:?}): {e}");
                brand_skipped += 1;
                continue;
            }
        };

        match api
            .post_json::<_, AdminCreateBrandResponse>(
                "admin/create-brand",
                &AdminCreateBrandRequest {
                    id,
                    name: name.clone(),
                },
            )
            .await
        {
            Ok(_) => brand_created += 1,
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("UNIQUE") || msg.contains("unique") || msg.contains("already") {
                    brand_skipped += 1;
                } else {
                    eprintln!("  Error creating brand {name:?}: {e}");
                    brand_skipped += 1;
                }
            }
        }
        sleep(Duration::from_millis(50)).await;
    }
    println!("Brands: {brand_created} created, {brand_skipped} skipped");

    // 9. Create products
    let mut prod_created = 0u32;
    let mut prod_skipped = 0u32;
    let total = selected_rows.len();
    let mut seen_slugs: HashSet<String> = HashSet::new();

    for (i, row) in selected_rows.iter().enumerate() {
        if (i + 1) % 100 == 0 || i + 1 == total {
            println!("  Products: {}/{total}...", i + 1);
        }

        let cat_slug = &category_slugs[&row.category];
        let brand_slug = &brand_slugs[&row.brand];

        let category_id = match ProductCategoryId::new(cat_slug) {
            Ok(id) => id,
            Err(_) => {
                prod_skipped += 1;
                continue;
            }
        };
        let brand_id = match ProductBrandId::new(brand_slug) {
            Ok(id) => id,
            Err(_) => {
                prod_skipped += 1;
                continue;
            }
        };

        // Generate a unique product slug from the name
        let mut base_slug = slugify(&row.name);
        if base_slug.is_empty() {
            base_slug = format!("product-{}", row.gtin);
        }
        // Truncate to reasonable length
        if base_slug.len() > 80 {
            base_slug = base_slug[..80].trim_end_matches('-').to_string();
        }

        let mut slug = base_slug.clone();
        let mut counter = 1u32;
        while seen_slugs.contains(&slug) {
            slug = format!("{base_slug}-{counter}");
            counter += 1;
        }
        seen_slugs.insert(slug.clone());

        let id = match ProductId::new(&slug) {
            Ok(id) => id,
            Err(e) => {
                eprintln!("  Skip product {}: bad slug {slug:?}: {e}", row.name);
                prod_skipped += 1;
                continue;
            }
        };

        // Parse price: CSV has dollar string like "1.02"
        let price_cents = match row.price.parse::<f64>() {
            Ok(v) => (v * 100.0).round() as u32,
            Err(_) => {
                prod_skipped += 1;
                continue;
            }
        };

        let moq: u32 = row.unit.parse().unwrap_or(1);
        let inventory: u32 = row.inventory.parse().unwrap_or(0);
        let is_preorder = row.is_preorder.eq_ignore_ascii_case("yes");
        let delivery_weeks: Option<u32> = row.delivery_weeks.parse().ok();

        match api
            .post_no_response(
                "admin/create-product",
                &AdminCreateProductRequest {
                    id,
                    gtin: row.gtin.clone(),
                    name: row.name.clone(),
                    category_id,
                    brand_id,
                    price_cents,
                    currency: "USD".to_string(),
                    minimum_order_quantity: moq,
                    inventory,
                    is_preorder,
                    estimated_delivery_weeks: delivery_weeks,
                    supplier_url: row.supplier_url.clone(),
                    image_url: row.image_url.clone(),
                },
            )
            .await
        {
            Ok(_) => prod_created += 1,
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("UNIQUE") || msg.contains("unique") || msg.contains("already") {
                    prod_skipped += 1;
                } else {
                    eprintln!("  Error creating product {:?}: {e}", row.name);
                    prod_skipped += 1;
                }
            }
        }
        sleep(Duration::from_millis(50)).await;
    }
    println!("Products: {prod_created} created, {prod_skipped} skipped");

    println!(
        "\nImport complete: {} categories, {} brands, {} products",
        cat_created, brand_created, prod_created
    );

    Ok(())
}
