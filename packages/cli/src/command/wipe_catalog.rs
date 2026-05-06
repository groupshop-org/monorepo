use std::time::Duration;

use anyhow::{bail, Context, Result};
use reqwest::Client;

const HEADER_SESSION_TOKEN: &str = "x-groupshop-session-token";

pub async fn run(api_url: &str, email: &str, password: &str) -> Result<()> {
    let client = Client::builder()
        .cookie_store(true)
        .timeout(Duration::from_secs(30))
        .build()?;

    println!("Signing in as {email}...");
    let signin_resp = client
        .post(format!(
            "{}/auth/email-password/signin",
            api_url.trim_end_matches('/')
        ))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "email": email, "password": password }))
        .send()
        .await
        .context("signin request")?;

    if !signin_resp.status().is_success() {
        bail!("Sign-in failed: {}", signin_resp.text().await?);
    }

    let refresh_resp = client
        .post(format!("{}/auth/refresh", api_url.trim_end_matches('/')))
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
        bail!("Refresh failed: {}", refresh_resp.text().await?);
    }

    println!("Authenticated successfully");
    println!("Wiping catalog (products, brands, categories)...");

    let resp = client
        .post(format!(
            "{}/admin/wipe-catalog",
            api_url.trim_end_matches('/')
        ))
        .header("Authorization", format!("Bearer {session_token}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({}))
        .send()
        .await
        .context("wipe-catalog request")?;

    if !resp.status().is_success() {
        bail!("Wipe failed: {}", resp.text().await?);
    }

    println!("Catalog wiped successfully");
    Ok(())
}
