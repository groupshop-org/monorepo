use worker::{durable_object, wasm_bindgen::prelude::*, DurableObject, Env, State};

use crate::{
    config::ONCE_TOKEN_LIFETIME_SECONDS,
    prelude::*,
    utils::{do_json_response, do_send, random_bytes, sha256_hash},
};

pub async fn auth_produce_once_token(
    ctx: &ApiContext,
    kind: AuthTokenOnceKind,
) -> ApiResult<(AuthTokenId, AuthTokenValue)> {
    let token_value = AuthTokenValue::from(random_bytes::<32>());
    let token_value_hash = AuthTokenValueHash::from(sha256_hash(&token_value.inner()).await?);

    let (token_id, _) = AuthTokenOnceCommand::Produce {
        kind,
        token_value_hash,
    }
    .send::<ProduceResponse>(ctx, None)
    .await?;

    Ok((AuthTokenId::new(token_id), token_value))
}

pub async fn auth_consume_once_token(
    ctx: &ApiContext,
    token_id: &AuthTokenId,
    token_value: &AuthTokenValue,
) -> ApiResult<AuthTokenOnceKind> {
    let token_value_hash = AuthTokenValueHash::from(sha256_hash(&token_value.inner()).await?);
    let (_, resp) = AuthTokenOnceCommand::Consume { token_value_hash }
        .send::<ConsumeResponse>(ctx, Some(token_id.clone()))
        .await?;
    Ok(resp.kind)
}

pub async fn auth_peek_once_token(
    ctx: &ApiContext,
    token_id: &AuthTokenId,
    token_value: &AuthTokenValue,
) -> ApiResult<AuthTokenOnceKind> {
    let token_value_hash = AuthTokenValueHash::from(sha256_hash(&token_value.inner()).await?);
    let (_, resp) = AuthTokenOnceCommand::Peek { token_value_hash }
        .send::<PeekResponse>(ctx, Some(token_id.clone()))
        .await?;
    Ok(resp.kind)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum AuthTokenOnceCommand {
    Produce {
        kind: AuthTokenOnceKind,
        token_value_hash: AuthTokenValueHash,
    },
    Consume {
        token_value_hash: AuthTokenValueHash,
    },
    Peek {
        token_value_hash: AuthTokenValueHash,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AuthTokenOnceKind {
    ResetPassword {
        uid: UserId,
    },
    VerifyEmail {
        uid: UserId,
    },
    OpenIdHook {
        provider: OpenIdProvider,
    },
    OpenIdFinalize {
        uid: Option<UserId>,
        subject: String,
        email: String,
        email_verified: bool,
        provider: OpenIdProvider,
    },
}

impl AuthTokenOnceCommand {
    async fn send<T: GroupshopCodec>(
        self,
        ctx: &ApiContext,
        token_id: Option<AuthTokenId>,
    ) -> ApiResult<(String, T)> {
        do_send(
            &ctx.env,
            "https://auth-once-token/internal",
            &ctx.config.do_binding_auth_token_once,
            token_id.map(|id| id.to_string()),
            self,
        )
        .await
    }
}

#[durable_object]
pub struct AuthTokenOnceDo {
    state: State,
    _env: Env,
}

impl DurableObject for AuthTokenOnceDo {
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

impl AuthTokenOnceDo {
    async fn fetch_inner(&self, mut req: worker::Request) -> ApiResult<worker::Response> {
        let body = req
            .bytes()
            .await
            .map_err(|err| DurableObjectError::Fetch(err.to_string()))?;

        match AuthTokenOnceCommand::decode(&body)? {
            AuthTokenOnceCommand::Produce {
                kind,
                token_value_hash,
            } => {
                self.state
                    .storage()
                    .put(
                        "state",
                        AuthTokenOnceState {
                            kind,
                            token_value_hash,
                        },
                    )
                    .await
                    .map_err(|err| DurableObjectError::StoragePut(err.to_string()))?;
                self.state
                    .storage()
                    .set_alarm(ONCE_TOKEN_LIFETIME_SECONDS as i64 * 1000)
                    .await
                    .map_err(|err| DurableObjectError::Alarm(err.to_string()))?;

                do_json_response(&ProduceResponse {})
                    .map_err(|err| DurableObjectError::EncodeResponse(err.to_string()).into())
            }
            AuthTokenOnceCommand::Consume { token_value_hash } => {
                let state: AuthTokenOnceState = self
                    .state
                    .storage()
                    .get("state")
                    .await
                    .map_err(|err| DurableObjectError::StorageGet(err.to_string()))?
                    .ok_or_else(|| DurableObjectError::StorageGet("state not found".to_string()))?;

                if state.token_value_hash != token_value_hash {
                    return Err(AuthError::OnceTokenMismatch.into());
                }

                let _ = self.state.storage().delete("state").await;
                do_json_response(&ConsumeResponse { kind: state.kind })
                    .map_err(|err| DurableObjectError::EncodeResponse(err.to_string()).into())
            }
            AuthTokenOnceCommand::Peek { token_value_hash } => {
                let state: AuthTokenOnceState = self
                    .state
                    .storage()
                    .get("state")
                    .await
                    .map_err(|err| DurableObjectError::StorageGet(err.to_string()))?
                    .ok_or_else(|| DurableObjectError::StorageGet("state not found".to_string()))?;

                if state.token_value_hash != token_value_hash {
                    return Err(AuthError::OnceTokenMismatch.into());
                }

                do_json_response(&PeekResponse { kind: state.kind })
                    .map_err(|err| DurableObjectError::EncodeResponse(err.to_string()).into())
            }
        }
    }

    async fn alarm_inner(&self) -> ApiResult<worker::Response> {
        let purged = self
            .state
            .storage()
            .delete("state")
            .await
            .map_err(|err| DurableObjectError::StorageDelete(err.to_string()))?;
        do_json_response(&AlarmResponse { purged })
            .map_err(|err| DurableObjectError::EncodeResponse(err.to_string()).into())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct AuthTokenOnceState {
    kind: AuthTokenOnceKind,
    token_value_hash: AuthTokenValueHash,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ProduceResponse {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ConsumeResponse {
    kind: AuthTokenOnceKind,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PeekResponse {
    kind: AuthTokenOnceKind,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct AlarmResponse {
    purged: bool,
}
