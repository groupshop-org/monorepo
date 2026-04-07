use worker::Env;

#[derive(Debug, Clone)]
pub struct Config {
    pub db_binding: String,
}

impl Config {
    pub fn new(env: &Env) -> Self {
        Self {
            db_binding: required_env_var(env, "DB_BINDING"),
        }
    }
}

fn required_env_var(env: &Env, key: &str) -> String {
    env.var(key)
        .ok()
        .map(|v| normalize_env_value(v.to_string()))
        .or_else(|| std::env::var(key).ok().map(normalize_env_value))
        .unwrap_or_else(|| panic!("{key} must be set"))
}

fn optional_env_var(env: &Env, key: &str) -> Option<String> {
    env.var(key)
        .ok()
        .map(|v| normalize_env_value(v.to_string()))
        .or_else(|| std::env::var(key).ok().map(normalize_env_value))
}

fn normalize_env_value(value: String) -> String {
    value
        .trim()
        .trim_matches('\'')
        .trim_matches('"')
        .to_string()
}
