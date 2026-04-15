use worker::{durable_object, wasm_bindgen::prelude::*, DurableObject, Env, State};

use crate::{
    config::REFRESH_TOKEN_GRACE_PERIOD_SECONDS,
    prelude::*,
    utils::{do_json_response, do_send, random_bytes, sha256_hash},
};

pub async fn auth_refresh_token_create(
    ctx: &ApiContext,
    uid: UserId,
) -> ApiResult<(AuthTokenId, AuthTokenValue)> {
    let token_value = AuthTokenValue::from(random_bytes::<32>());
    let token_value_hash = AuthTokenValueHash::from(sha256_hash(&token_value.inner()).await?);

    let (token_id, _) = AuthTokenRefreshCommand::Create {
        uid,
        token_value_hash,
    }
    .send::<CreateResponse>(ctx, None)
    .await?;

    Ok((AuthTokenId::new(token_id), token_value))
}

pub async fn auth_refresh_token_update(
    ctx: &ApiContext,
    token_id: AuthTokenId,
    token_value: AuthTokenValue,
) -> ApiResult<(UserId, Option<AuthTokenValue>)> {
    let current_token_value_hash =
        AuthTokenValueHash::from(sha256_hash(&token_value.inner()).await?);
    let new_token_value = AuthTokenValue::from(random_bytes::<32>());
    let new_token_value_hash =
        AuthTokenValueHash::from(sha256_hash(&new_token_value.inner()).await?);

    let (_, resp) = AuthTokenRefreshCommand::Refresh {
        current_token_value_hash,
        new_token_value_hash,
    }
    .send::<RefreshResponse>(ctx, Some(token_id.clone()))
    .await?;

    if resp.updated {
        Ok((resp.uid, Some(new_token_value)))
    } else {
        Ok((resp.uid, None))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum AuthTokenRefreshCommand {
    Create {
        uid: UserId,
        token_value_hash: AuthTokenValueHash,
    },
    Refresh {
        current_token_value_hash: AuthTokenValueHash,
        new_token_value_hash: AuthTokenValueHash,
    },
}

impl AuthTokenRefreshCommand {
    async fn send<T: GroupshopCodec>(
        self,
        ctx: &ApiContext,
        token_id: Option<AuthTokenId>,
    ) -> ApiResult<(String, T)> {
        do_send(
            &ctx.env,
            "https://auth-refresh-token/internal",
            &ctx.config.do_binding_auth_token_refresh,
            token_id.map(|id| id.to_string()),
            self,
        )
        .await
    }
}

#[durable_object]
pub struct AuthTokenRefreshDo {
    state: State,
    _env: Env,
}

impl DurableObject for AuthTokenRefreshDo {
    fn new(state: State, env: Env) -> Self {
        Self { state, _env: env }
    }

    async fn fetch(&self, req: worker::Request) -> worker::Result<worker::Response> {
        match self.fetch_inner(req).await {
            Ok(resp) => Ok(resp),
            Err(err) => worker::Response::error(err.to_string(), 401),
        }
    }

    async fn alarm(&self) -> worker::Result<worker::Response> {
        match self.alarm_inner().await {
            Ok(resp) => Ok(resp),
            Err(err) => worker::Response::error(err.to_string(), 500),
        }
    }
}

impl AuthTokenRefreshDo {
    async fn fetch_inner(&self, mut req: worker::Request) -> ApiResult<worker::Response> {
        let body = req
            .bytes()
            .await
            .map_err(|err| DurableObjectError::Fetch(err.to_string()))?;

        match AuthTokenRefreshCommand::decode(&body)? {
            AuthTokenRefreshCommand::Create {
                uid,
                token_value_hash,
            } => {
                self.state
                    .storage()
                    .put(
                        "state",
                        AuthTokenRefreshState {
                            uid,
                            token_value_hash,
                            expired_token_value_hash: None,
                        },
                    )
                    .await
                    .map_err(|err| DurableObjectError::StoragePut(err.to_string()))?;

                do_json_response(&CreateResponse {})
                    .map_err(|err| DurableObjectError::EncodeResponse(err.to_string()).into())
            }
            AuthTokenRefreshCommand::Refresh {
                current_token_value_hash,
                new_token_value_hash,
            } => {
                let mut state: AuthTokenRefreshState = self
                    .state
                    .storage()
                    .get("state")
                    .await
                    .map_err(|err| DurableObjectError::StorageGet(err.to_string()))?
                    .ok_or_else(|| DurableObjectError::StorageGet("missing state".to_string()))?;

                let uid = state.uid.clone();
                if state.token_value_hash == current_token_value_hash {
                    state.expired_token_value_hash = Some(state.token_value_hash.clone());
                    state.token_value_hash = new_token_value_hash;
                    self.state
                        .storage()
                        .put("state", state)
                        .await
                        .map_err(|err| DurableObjectError::StoragePut(err.to_string()))?;
                    self.state
                        .storage()
                        .set_alarm(REFRESH_TOKEN_GRACE_PERIOD_SECONDS as i64 * 1000)
                        .await
                        .map_err(|err| DurableObjectError::Alarm(err.to_string()))?;

                    do_json_response(&RefreshResponse { uid, updated: true })
                        .map_err(|err| DurableObjectError::EncodeResponse(err.to_string()).into())
                } else if state.expired_token_value_hash.as_ref() == Some(&current_token_value_hash)
                {
                    do_json_response(&RefreshResponse {
                        uid,
                        updated: false,
                    })
                    .map_err(|err| DurableObjectError::EncodeResponse(err.to_string()).into())
                } else {
                    Err(AuthError::RefreshTokenMismatch {
                        uid: state.uid.to_string(),
                    }
                    .into())
                }
            }
        }
    }

    async fn alarm_inner(&self) -> ApiResult<worker::Response> {
        let mut state: AuthTokenRefreshState = self
            .state
            .storage()
            .get("state")
            .await
            .map_err(|err| DurableObjectError::StorageGet(err.to_string()))?
            .ok_or_else(|| DurableObjectError::StorageGet("missing state".to_string()))?;

        state.expired_token_value_hash = None;
        self.state
            .storage()
            .put("state", state)
            .await
            .map_err(|err| DurableObjectError::StoragePut(err.to_string()))?;

        do_json_response(&AlarmResponse { purged: true })
            .map_err(|err| DurableObjectError::EncodeResponse(err.to_string()).into())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct AuthTokenRefreshState {
    uid: UserId,
    token_value_hash: AuthTokenValueHash,
    expired_token_value_hash: Option<AuthTokenValueHash>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CreateResponse {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RefreshResponse {
    uid: UserId,
    updated: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct AlarmResponse {
    purged: bool,
}
