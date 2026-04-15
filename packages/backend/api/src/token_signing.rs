use groupshop_backend_shared::prelude::*;

use crate::{
    config::SESSION_TOKEN_LIFETIME_MILLIS,
    context::ApiContext,
    durable::auth::{
        once::{auth_produce_once_token, AuthTokenOnceKind},
        refresh::auth_refresh_token_create,
    },
    utils::sign_bytes,
};

pub async fn sign_once_token(ctx: &ApiContext, kind: AuthTokenOnceKind) -> ApiResult<AuthToken> {
    let (token_id, token_value) = auth_produce_once_token(ctx, kind).await?;

    let claims = AuthTokenClaims::Once {
        token_id,
        token_value,
    };

    let signature = AuthTokenSignature::from(
        sign_bytes(&ctx.config.token_signing_key, claims.encode()?).await?,
    );

    Ok(AuthToken { claims, signature })
}

pub async fn sign_new_refresh_token(ctx: &ApiContext, uid: UserId) -> ApiResult<AuthToken> {
    let (token_id, token_value) = auth_refresh_token_create(ctx, uid).await?;
    sign_existing_refresh_token_value(ctx, token_id, token_value).await
}

pub async fn sign_existing_refresh_token_value(
    ctx: &ApiContext,
    token_id: AuthTokenId,
    token_value: AuthTokenValue,
) -> ApiResult<AuthToken> {
    let claims = AuthTokenClaims::Refresh {
        token_id,
        token_value,
    };
    let signature = AuthTokenSignature::from(
        sign_bytes(&ctx.config.token_signing_key, claims.encode()?).await?,
    );
    Ok(AuthToken { claims, signature })
}

pub async fn sign_session_token(
    ctx: &ApiContext,
    uid: UserId,
    token_value: AuthTokenValue,
) -> ApiResult<AuthToken> {
    let claims = AuthTokenClaims::Session {
        uid,
        expires_at: js_sys::Date::now() as u64 + SESSION_TOKEN_LIFETIME_MILLIS,
        token_value,
    };
    let signature = AuthTokenSignature::from(
        sign_bytes(&ctx.config.token_signing_key, claims.encode()?).await?,
    );
    Ok(AuthToken { claims, signature })
}
