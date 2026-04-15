use std::sync::OnceLock;

use futures_signals::signal::Mutable;
use groupshop_backend_shared::prelude::AccountProfile;
use groupshop_frontend_shared::{
    api::client::ApiClient, constants::AUTH_SESSION_TOKEN_STORAGE_KEY, error::FrontendResult,
    util::storage::delete_local_storage,
};
use wasm_bindgen_futures::spawn_local;

use crate::{config, route::Route};

static API_CTX: OnceLock<ApiCtx> = OnceLock::new();

pub struct ApiCtx {
    pub profile: Mutable<Option<AccountProfile>>,
    pub client: ApiClient,
}

impl ApiCtx {
    pub async fn init() -> FrontendResult<()> {
        let client = ApiClient::new(config::api_url());
        let _ = client.auth_refresh().await;
        let profile = client.account_profile().await.ok();

        API_CTX
            .set(Self {
                profile: Mutable::new(profile),
                client,
            })
            .map_err(|_| {
                groupshop_frontend_shared::error::FrontendError::Other(
                    "ApiCtx already initialized".to_string(),
                )
            })?;

        Ok(())
    }

    pub async fn refresh_profile() -> FrontendResult<()> {
        let ctx = Self::get();
        let profile = ctx.client.account_profile().await.ok();
        ctx.profile.set(profile);
        Ok(())
    }

    pub fn get() -> &'static Self {
        API_CTX.get().expect("ApiCtx is not initialized")
    }

    pub fn sign_out() {
        spawn_local(async {
            let _ = Self::get().client.auth_signout().await;
            let _ = delete_local_storage(AUTH_SESSION_TOKEN_STORAGE_KEY);
            Self::get().profile.set(None);
            Route::Home.go_to_url();
        });
    }
}
