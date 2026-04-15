use std::sync::Arc;

use groupshop_backend_shared::prelude::{AuthToken, UserId};
use worker::{Context, Env};

use crate::config::Config;

#[derive(Debug, Default, Clone)]
pub struct UpdatedTokens {
    pub session_header_token: Option<AuthToken>,
    pub session_cookie_token: Option<AuthToken>,
    pub refresh_token: Option<AuthToken>,
    pub clear_cookies: bool,
}

pub struct ApiContext {
    pub env: Env,
    #[allow(dead_code)]
    pub worker_ctx: Context,
    pub config: Arc<Config>,
    pub updated_tokens: UpdatedTokens,
    pub uid: Option<UserId>,
}

impl ApiContext {
    pub fn new(env: Env, worker_ctx: Context) -> Self {
        let config = Arc::new(Config::new(&env));
        Self {
            uid: None,
            env,
            worker_ctx,
            config,
            updated_tokens: UpdatedTokens::default(),
        }
    }

    pub fn unchecked_uid(&self) -> &UserId {
        self.uid.as_ref().expect("uid should be set in context")
    }
}
