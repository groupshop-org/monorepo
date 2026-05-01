use wasm_bindgen::prelude::*;

use crate::db::tables::{
    SQL_TABLE_PRODUCT_BRAND, SQL_TABLE_PRODUCT_CATALOG, SQL_TABLE_PRODUCT_CATEGORY,
    SQL_TABLE_USER_PARTICIPATION,
};
use crate::{
    prelude::*,
    utils::{db_execute, db_load, db_load_all, db_prepare, deserialize_d1_bool, get_d1},
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProductCatalogDb {
    pub id: ProductId,
    pub gtin: String,
    pub name: String,
    pub category_id: ProductCategoryId,
    pub brand_id: ProductBrandId,
    pub price_cents: u32,
    pub currency: String,
    pub minimum_order_quantity: u32,
    pub inventory: u32,
    #[serde(deserialize_with = "deserialize_d1_bool")]
    pub is_preorder: bool,
    pub estimated_delivery_weeks: Option<u32>,
    pub supplier_url: String,
    pub image_url: String,
    #[serde(deserialize_with = "deserialize_d1_bool")]
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

const COLUMNS: &str = "id, gtin, name, category_id, brand_id, price_cents, currency, minimum_order_quantity, inventory, is_preorder, estimated_delivery_weeks, supplier_url, image_url, is_active, created_at, updated_at";

impl ProductCatalogDb {
    pub async fn load_by_id(ctx: &ApiContext, id: &ProductId) -> ApiResult<Self> {
        db_load(
            db_prepare(
                &get_d1(&ctx.env)?,
                format!("SELECT {COLUMNS} FROM {SQL_TABLE_PRODUCT_CATALOG} WHERE id = ?1"),
                &[JsValue::from_str(id.as_str())],
            )?,
            "product not found",
        )
        .await
    }

    pub async fn load_page(
        ctx: &ApiContext,
        page: u32,
        per_page: u32,
        category_id: Option<&ProductCategoryId>,
        brand_id: Option<&ProductBrandId>,
        search: Option<&str>,
        active_only: bool,
        with_participants_only: bool,
    ) -> ApiResult<Vec<Self>> {
        let offset = (page.saturating_sub(1)) * per_page;
        let (where_clause, mut bindings, mut idx) = build_filter_clauses(
            category_id,
            brand_id,
            search,
            active_only,
            with_participants_only,
        );

        bindings.push(JsValue::from_f64(per_page as f64));
        let limit_idx = idx;
        idx += 1;
        bindings.push(JsValue::from_f64(offset as f64));
        let offset_idx = idx;

        db_load_all(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "SELECT {COLUMNS} FROM {SQL_TABLE_PRODUCT_CATALOG} {where_clause} ORDER BY name ASC LIMIT ?{limit_idx} OFFSET ?{offset_idx}"
            ),
            &bindings,
        )?)
        .await
    }

    pub async fn count(
        ctx: &ApiContext,
        category_id: Option<&ProductCategoryId>,
        brand_id: Option<&ProductBrandId>,
        search: Option<&str>,
        active_only: bool,
        with_participants_only: bool,
    ) -> ApiResult<u32> {
        #[derive(serde::Deserialize)]
        struct CountRow {
            n: f64,
        }

        let (where_clause, bindings, _) = build_filter_clauses(
            category_id,
            brand_id,
            search,
            active_only,
            with_participants_only,
        );

        let row: CountRow = db_load(
            db_prepare(
                &get_d1(&ctx.env)?,
                format!("SELECT count(*) AS n FROM {SQL_TABLE_PRODUCT_CATALOG} {where_clause}"),
                &bindings,
            )?,
            "count query failed",
        )
        .await?;

        Ok(row.n as u32)
    }

    pub async fn insert(
        ctx: &ApiContext,
        id: &ProductId,
        gtin: &str,
        name: &str,
        category_id: &ProductCategoryId,
        brand_id: &ProductBrandId,
        price_cents: u32,
        currency: &str,
        minimum_order_quantity: u32,
        inventory: u32,
        is_preorder: bool,
        estimated_delivery_weeks: Option<u32>,
        supplier_url: &str,
        image_url: &str,
    ) -> ApiResult<()> {
        let product_hash = id.hash();
        let hash_js = js_sys::Uint8Array::from(product_hash.as_ref()).into();
        db_execute(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "INSERT INTO {SQL_TABLE_PRODUCT_CATALOG} (id, product_hash, gtin, name, category_id, brand_id, price_cents, currency, minimum_order_quantity, inventory, is_preorder, estimated_delivery_weeks, supplier_url, image_url) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"
            ),
            &[
                JsValue::from_str(id.as_str()),
                hash_js,
                JsValue::from_str(gtin),
                JsValue::from_str(name),
                JsValue::from_str(category_id.as_str()),
                JsValue::from_str(brand_id.as_str()),
                JsValue::from_f64(price_cents as f64),
                JsValue::from_str(currency),
                JsValue::from_f64(minimum_order_quantity as f64),
                JsValue::from_f64(inventory as f64),
                JsValue::from_f64(if is_preorder { 1.0 } else { 0.0 }),
                match estimated_delivery_weeks {
                    Some(w) => JsValue::from_f64(w as f64),
                    None => JsValue::NULL,
                },
                JsValue::from_str(supplier_url),
                JsValue::from_str(image_url),
            ],
        )?)
        .await
    }

    pub async fn update(
        ctx: &ApiContext,
        id: &ProductId,
        name: Option<&str>,
        price_cents: Option<u32>,
        currency: Option<&str>,
        minimum_order_quantity: Option<u32>,
        inventory: Option<u32>,
        is_preorder: Option<bool>,
        estimated_delivery_weeks: Option<Option<u32>>,
        supplier_url: Option<&str>,
        image_url: Option<&str>,
        is_active: Option<bool>,
    ) -> ApiResult<()> {
        let mut sets = Vec::new();
        let mut bindings: Vec<JsValue> = Vec::new();
        let mut idx = 1u32;

        macro_rules! push_set {
            ($field:expr, $val:expr) => {
                sets.push(format!("{} = ?{idx}", $field));
                bindings.push($val);
                idx += 1;
            };
        }

        if let Some(v) = name {
            push_set!("name", JsValue::from_str(v));
        }
        if let Some(v) = price_cents {
            push_set!("price_cents", JsValue::from_f64(v as f64));
        }
        if let Some(v) = currency {
            push_set!("currency", JsValue::from_str(v));
        }
        if let Some(v) = minimum_order_quantity {
            push_set!("minimum_order_quantity", JsValue::from_f64(v as f64));
        }
        if let Some(v) = inventory {
            push_set!("inventory", JsValue::from_f64(v as f64));
        }
        if let Some(v) = is_preorder {
            push_set!("is_preorder", JsValue::from_f64(if v { 1.0 } else { 0.0 }));
        }
        if let Some(v) = estimated_delivery_weeks {
            push_set!(
                "estimated_delivery_weeks",
                match v {
                    Some(w) => JsValue::from_f64(w as f64),
                    None => JsValue::NULL,
                }
            );
        }
        if let Some(v) = supplier_url {
            push_set!("supplier_url", JsValue::from_str(v));
        }
        if let Some(v) = image_url {
            push_set!("image_url", JsValue::from_str(v));
        }
        if let Some(v) = is_active {
            push_set!("is_active", JsValue::from_f64(if v { 1.0 } else { 0.0 }));
        }

        if sets.is_empty() {
            return Ok(());
        }

        sets.push("updated_at = CURRENT_TIMESTAMP".to_string());
        let set_clause = sets.join(", ");
        bindings.push(JsValue::from_str(id.as_str()));

        db_execute(db_prepare(
            &get_d1(&ctx.env)?,
            format!("UPDATE {SQL_TABLE_PRODUCT_CATALOG} SET {set_clause} WHERE id = ?{idx}"),
            &bindings,
        )?)
        .await
    }

    pub async fn delete(ctx: &ApiContext, id: &ProductId) -> ApiResult<()> {
        db_execute(db_prepare(
            &get_d1(&ctx.env)?,
            format!("DELETE FROM {SQL_TABLE_PRODUCT_CATALOG} WHERE id = ?1"),
            &[JsValue::from_str(id.as_str())],
        )?)
        .await
    }

    pub async fn delete_all(ctx: &ApiContext) -> ApiResult<()> {
        let db = get_d1(&ctx.env)?;
        db_execute(db_prepare(&db, format!("DELETE FROM {SQL_TABLE_PRODUCT_CATALOG}"), &[])?)
            .await?;
        db_execute(db_prepare(&db, format!("DELETE FROM {SQL_TABLE_PRODUCT_BRAND}"), &[])?)
            .await?;
        db_execute(db_prepare(&db, format!("DELETE FROM {SQL_TABLE_PRODUCT_CATEGORY}"), &[])?)
            .await
    }
}

/// Builds the shared `WHERE` clause for `load_page` and `count` so the two
/// stay in sync. Returns the formatted clause, the bound JsValue arguments,
/// and the next free placeholder index for the caller to extend (e.g. with
/// LIMIT/OFFSET binds).
fn build_filter_clauses(
    category_id: Option<&ProductCategoryId>,
    brand_id: Option<&ProductBrandId>,
    search: Option<&str>,
    active_only: bool,
    with_participants_only: bool,
) -> (String, Vec<JsValue>, u32) {
    let mut where_parts: Vec<String> = Vec::new();
    let mut bindings: Vec<JsValue> = Vec::new();
    let mut idx = 1u32;

    if active_only {
        where_parts.push("is_active = 1".to_string());
    }

    if let Some(cat) = category_id {
        where_parts.push(format!("category_id = ?{idx}"));
        bindings.push(JsValue::from_str(cat.as_str()));
        idx += 1;
    }

    if let Some(brand) = brand_id {
        where_parts.push(format!("brand_id = ?{idx}"));
        bindings.push(JsValue::from_str(brand.as_str()));
        idx += 1;
    }

    if let Some(search) = search {
        where_parts.push(format!("name LIKE ?{idx}"));
        bindings.push(JsValue::from_str(&format!("%{search}%")));
        idx += 1;
    }

    if with_participants_only {
        // Subquery on user_participation; the index on (product_id) keeps this
        // cheap even when the catalog grows. This is the data path that
        // backs the "deals gaining traction" landing-page filter.
        where_parts.push(format!(
            "EXISTS (SELECT 1 FROM {SQL_TABLE_USER_PARTICIPATION} up WHERE up.product_id = {SQL_TABLE_PRODUCT_CATALOG}.id)"
        ));
    }

    let where_clause = if where_parts.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_parts.join(" AND "))
    };

    (where_clause, bindings, idx)
}

pub fn is_gtin_unique_violation(db_message: &str) -> bool {
    let lower = db_message.to_ascii_lowercase();
    lower.contains("unique")
        && (lower.contains("product_catalog.gtin") || lower.contains("idx_product_catalog_gtin"))
}

pub fn is_id_unique_violation(db_message: &str) -> bool {
    let lower = db_message.to_ascii_lowercase();
    lower.contains("unique") && lower.contains("product_catalog.id")
}
