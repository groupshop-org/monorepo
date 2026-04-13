use std::sync::LazyLock;

use groupshop_frontend_shared::required_build_env;

pub struct Config {
    pub health_api_url: &'static str,
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| Config {
    health_api_url: required_build_env!("URL_HEALTH_API"),
});
