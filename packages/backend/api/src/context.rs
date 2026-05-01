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
    /// `None` when the context was constructed for a scheduled event —
    /// scheduled handlers don't get a `worker::Context`. Always `Some`
    /// for fetch-driven flows.
    #[allow(dead_code)]
    pub worker_ctx: Option<Context>,
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
            worker_ctx: Some(worker_ctx),
            config,
            updated_tokens: UpdatedTokens::default(),
        }
    }

    /// Constructor for non-fetch entry points (scheduled events) where
    /// the worker runtime doesn't hand us a `Context`.
    pub fn new_without_worker_ctx(env: Env) -> Self {
        let config = Arc::new(Config::new(&env));
        Self {
            uid: None,
            env,
            worker_ctx: None,
            config,
            updated_tokens: UpdatedTokens::default(),
        }
    }

    pub fn unchecked_uid(&self) -> &UserId {
        self.uid.as_ref().expect("uid should be set in context")
    }
}
