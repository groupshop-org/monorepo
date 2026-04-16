use groupshop_frontend_shared::required_build_env;

pub fn api_url() -> &'static str {
    required_build_env!("URL_API")
}

pub fn landing_url() -> &'static str {
    required_build_env!("URL_LANDING")
}
