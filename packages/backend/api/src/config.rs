use groupshop_backend_shared::{prelude::AuthToken, GroupshopCodec};
use worker::Env;

pub const REFRESH_TOKEN_GRACE_PERIOD_SECONDS: u64 = 60;
pub const ONCE_TOKEN_LIFETIME_SECONDS: u64 = 60 * 60;
pub const SESSION_TOKEN_LIFETIME_MILLIS: u64 = 15 * 60 * 1000;
pub const ESCROW_WALLET_PROOF_LIFETIME_MILLIS: u64 = 5 * 60 * 1000;

#[derive(Debug, Clone)]
pub struct Config {
    pub allowed_origins: Vec<String>,
    pub cookie_same_site: CookieSameSitePolicy,
    pub cookie_domain: Option<String>,
    pub allow_local_cors_bypass: bool,
    pub db_binding: String,
    pub do_binding_auth_token_refresh: String,
    pub do_binding_auth_token_once: String,
    pub openid_google_client_id: String,
    pub openid_google_client_secret: String,
    pub openid_google_redirect_uri: String,
    pub token_signing_key: Vec<u8>,
    pub default_admin_emails: Vec<String>,
    pub debug_auth_links_in_console: bool,
    pub postmark_server_token: Option<String>,
    pub postmark_from_email: Option<String>,
    pub postmark_message_stream: Option<String>,
    pub solana: SolanaConfig,
    landing_url: String,
}

#[derive(Debug, Clone)]
pub struct SolanaConfig {
    pub network: String,
    /// Internal RPC used for server-side Solana calls (may be a private endpoint).
    pub rpc_url: String,
    /// RPC URL returned to clients in responses and embedded in the frontend manifest.
    /// Falls back to `rpc_url` when `SOLANA_RPC_URL_CLIENT` is not set.
    pub rpc_url_client: String,
    pub market_program_id: String,
    pub usdc_mint: String,
    pub authority_signing_key: [u8; 32],
}

pub enum LandingRoute {
    OpenIdFinalize { token: AuthToken },
    Error { message: String },
    VerifyEmailConfirm { token: String },
    ResetPassword { token: String },
}

impl std::fmt::Display for LandingRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenIdFinalize { token } => {
                write!(f, "openid-finalize/{}", token.encode_str().unwrap())
            }
            Self::Error { message } => {
                write!(f, "error/{}", urlencoding::encode(message))
            }
            Self::VerifyEmailConfirm { token } => {
                write!(f, "verify-email-confirm/{token}")
            }
            Self::ResetPassword { token } => {
                write!(f, "reset-password/{token}")
            }
        }
    }
}

impl Config {
    pub fn new(env: &Env) -> Self {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        enum AppEnv {
            Dev,
            Prod,
        }

        let app_env = match required_env_var(env, "APP_ENV").as_str() {
            "dev" => AppEnv::Dev,
            "prod" => AppEnv::Prod,
            other => panic!("APP_ENV must be `dev` or `prod`, got `{other}`"),
        };

        let allowed_origins = required_env_var(env, "ALLOWED_ORIGINS")
            .split(',')
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();

        let cookie_same_site = match required_env_var(env, "AUTH_COOKIE_SAMESITE").as_str() {
            "none" => CookieSameSitePolicy::None,
            "strict" => CookieSameSitePolicy::Strict,
            other => panic!("AUTH_COOKIE_SAMESITE must be `none` or `strict`, got `{other}`"),
        };

        let cookie_domain = optional_env_var(env, "AUTH_COOKIE_DOMAIN");

        let default_admin_emails = optional_env_var(env, "API_DEFAULT_ADMIN_EMAILS")
            .map(|value| {
                value
                    .split(';')
                    .map(|segment| segment.trim().to_ascii_lowercase())
                    .filter(|segment| !segment.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        Self {
            allowed_origins,
            cookie_same_site,
            cookie_domain,
            allow_local_cors_bypass: app_env == AppEnv::Dev,
            db_binding: required_env_var(env, "DB_BINDING"),
            do_binding_auth_token_refresh: required_env_var(env, "DO_BINDING_AUTH_TOKEN_REFRESH"),
            do_binding_auth_token_once: required_env_var(env, "DO_BINDING_AUTH_TOKEN_ONCE"),
            openid_google_client_id: required_env_var(env, "API_OPENID_GOOGLE_CLIENT_ID"),
            openid_google_client_secret: required_env_var(env, "API_OPENID_GOOGLE_CLIENT_SECRET"),
            openid_google_redirect_uri: required_env_var(env, "OPENID_GOOGLE_REDIRECT_URI"),
            token_signing_key: const_hex::decode(required_env_var(env, "API_TOKEN_SIGNING_KEY"))
                .expect("API_TOKEN_SIGNING_KEY must be 32-byte hex"),
            default_admin_emails,
            debug_auth_links_in_console: app_env == AppEnv::Dev,
            postmark_server_token: optional_env_var(env, "API_POSTMARK_SERVER_TOKEN"),
            postmark_from_email: optional_env_var(env, "POSTMARK_FROM_EMAIL"),
            postmark_message_stream: optional_env_var(env, "POSTMARK_MESSAGE_STREAM"),
            solana: SolanaConfig::new(env),
            landing_url: required_env_var(env, "URL_LANDING"),
        }
    }

    pub fn is_default_admin_email(&self, email: &str) -> bool {
        let normalized = email.trim().to_ascii_lowercase();
        self.default_admin_emails
            .iter()
            .any(|candidate| candidate == &normalized)
    }

    pub fn landing_url(&self, route: LandingRoute) -> String {
        format!("{}/{}", self.landing_url.trim_end_matches('/'), route)
    }
}

impl SolanaConfig {
    fn new(env: &Env) -> Self {
        let rpc_url = required_env_var(env, "SOLANA_RPC_URL");
        let rpc_url_client =
            optional_env_var(env, "SOLANA_RPC_URL_CLIENT").unwrap_or_else(|| rpc_url.clone());
        Self {
            network: required_env_var(env, "SOLANA_NETWORK"),
            rpc_url,
            rpc_url_client,
            market_program_id: required_env_var(env, "SOLANA_MARKET_PROGRAM_ID"),
            usdc_mint: required_env_var(env, "SOLANA_USDC_MINT"),
            authority_signing_key: parse_authority_keypair(&required_env_var(
                env,
                "SOLANA_AUTHORITY_KEYPAIR_JSON",
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CookieSameSitePolicy {
    None,
    Strict,
}

impl CookieSameSitePolicy {
    pub fn as_cookie_attr(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Strict => "Strict",
        }
    }
}

fn required_env_var(env: &Env, key: &str) -> String {
    env.var(key)
        .ok()
        .map(|value| normalize_env_value(value.to_string()))
        .or_else(|| {
            env.secret(key)
                .ok()
                .map(|value| normalize_env_value(value.to_string()))
        })
        .or_else(|| std::env::var(key).ok().map(normalize_env_value))
        .unwrap_or_else(|| panic!("{key} must be set"))
}

fn optional_env_var(env: &Env, key: &str) -> Option<String> {
    env.var(key)
        .ok()
        .map(|value| normalize_env_value(value.to_string()))
        .or_else(|| {
            env.secret(key)
                .ok()
                .map(|value| normalize_env_value(value.to_string()))
        })
        .or_else(|| std::env::var(key).ok().map(normalize_env_value))
}

fn normalize_env_value(value: String) -> String {
    value
        .trim()
        .trim_matches('\'')
        .trim_matches('"')
        .to_string()
}

fn parse_authority_keypair(raw: &str) -> [u8; 32] {
    let bytes: Vec<u8> =
        serde_json::from_str(raw).expect("SOLANA_AUTHORITY_KEYPAIR_JSON must be a JSON array");

    if bytes.len() < 32 {
        panic!(
            "SOLANA_AUTHORITY_KEYPAIR_JSON must contain at least 32 bytes, got {}",
            bytes.len()
        );
    }

    let mut secret = [0u8; 32];
    secret.copy_from_slice(&bytes[..32]);
    secret
}
