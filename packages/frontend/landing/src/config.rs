use groupshop_frontend_shared::required_build_env;

pub fn api_url() -> &'static str {
    required_build_env!("URL_API")
}

pub fn media_url() -> &'static str {
    required_build_env!("URL_MEDIA")
}

pub fn media_link(path: &str) -> String {
    format!("{}/{}", media_url(), path)
}

pub fn admin_url() -> &'static str {
    required_build_env!("URL_ADMIN")
}

pub fn solana_network() -> &'static str {
    required_build_env!("SOLANA_NETWORK")
}

pub fn solana_deployments_url() -> &'static str {
    "/assets/solana-deployments.json"
}

/// Polling cadence for any view that displays "X of N orders" against a
/// group-deal threshold (product detail, My Orders, the landing-page deal
/// grid). Tuned so the UI feels live without hammering the API: with the
/// default 12s interval, an idle user sees an updated count within ~one
/// minute of a new buyer joining, and a busy threshold view costs roughly
/// 5 reqs/min/user.
pub const REFRESH_THRESHHOLD_VIEW_MS: u32 = 12_000;
