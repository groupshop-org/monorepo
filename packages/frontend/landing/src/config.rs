use groupshop_frontend_shared::required_build_env;

pub fn media_url() -> &'static str {
    required_build_env!("URL_MEDIA")
}

pub fn media_link(path: &str) -> String {
    format!("{}/{}", media_url(), path)
}
