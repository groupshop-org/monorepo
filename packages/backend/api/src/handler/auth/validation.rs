use http::header::InvalidHeaderValue;

use crate::{
    context::UpdatedTokens,
    db::auth::role::UserAuthRoleDb,
    durable::auth::{
        once::{auth_consume_once_token, auth_peek_once_token, AuthTokenOnceKind},
        refresh::auth_refresh_token_update,
    },
    prelude::*,
    token_signing::{
        sign_existing_refresh_token_value, sign_new_refresh_token, sign_session_token,
    },
    utils::{random_bytes, verify_bytes},
};

pub struct Validation;

impl Validation {
    pub async fn root_handler(
        ctx: &mut ApiContext,
        route: &ApiRoute,
        req: &HttpRequest,
    ) -> ApiResult<Self> {
        if let Some(auth_requirement) = route.auth_requirement() {
            match auth_requirement {
                AuthRequirement::Session => {
                    let header_token = extract_authorization_bearer_header::<AuthToken>(req)
                        .ok_or(AuthError::MissingSessionTokenHeader)?;
                    let cookie_token = extract_cookie::<AuthToken>(req, COOKIE_AUTH_SESSION_TOKEN)
                        .ok_or(AuthError::MissingSessionTokenCookie)?;

                    let header_uid = match &header_token.claims {
                        AuthTokenClaims::Session { uid, .. } => uid.clone(),
                        _ => return Err(AuthError::WrongClaims.into()),
                    };
                    let cookie_uid = match &cookie_token.claims {
                        AuthTokenClaims::Session { uid, .. } => uid.clone(),
                        _ => return Err(AuthError::WrongClaims.into()),
                    };

                    match (
                        validate_signature(ctx, &header_token).await,
                        validate_signature(ctx, &cookie_token).await,
                    ) {
                        (Ok(_), Ok(_)) => {
                            if header_uid != cookie_uid {
                                return Err(AuthError::InvalidSignature(
                                    "UID mismatch between header and cookie".into(),
                                )
                                .into());
                            }
                            ctx.uid = Some(header_uid);
                        }
                        (Err(ApiError::Auth(AuthError::Expired)), Ok(_))
                        | (Ok(_), Err(ApiError::Auth(AuthError::Expired)))
                        | (
                            Err(ApiError::Auth(AuthError::Expired)),
                            Err(ApiError::Auth(AuthError::Expired)),
                        ) => {
                            let (refresh_uid, updated_tokens) =
                                UpdatedTokens::update_all(ctx, req).await?;
                            ctx.uid = Some(refresh_uid);
                            ctx.updated_tokens = updated_tokens;
                        }
                        (Err(err), _) => return Err(err),
                        (_, Err(err)) => return Err(err),
                    }
                }
                AuthRequirement::Refresh => {
                    let (refresh_uid, updated_tokens) = UpdatedTokens::update_all(ctx, req).await?;
                    ctx.uid = Some(refresh_uid);
                    ctx.updated_tokens = updated_tokens;
                }
            }
        }

        if let Some(auth_roles) = route.role_requirement() {
            let uid = ctx.uid.as_ref().ok_or_else(|| {
                ApiError::Auth(AuthError::InvalidSignature(
                    "No valid UID found for role check".into(),
                ))
            })?;
            UserAuthRoleDb::check_roles(ctx, uid, &auth_roles).await?;
        }

        Ok(Self)
    }

    pub async fn once_token_peek(
        ctx: &ApiContext,
        token: &AuthToken,
    ) -> ApiResult<AuthTokenOnceKind> {
        validate_signature(ctx, token).await?;
        match &token.claims {
            AuthTokenClaims::Once {
                token_id,
                token_value,
            } => auth_peek_once_token(ctx, token_id, token_value).await,
            _ => Err(AuthError::WrongClaims.into()),
        }
    }

    pub async fn once_token_consume(
        ctx: &ApiContext,
        token: &AuthToken,
    ) -> ApiResult<AuthTokenOnceKind> {
        validate_signature(ctx, token).await?;
        match &token.claims {
            AuthTokenClaims::Once {
                token_id,
                token_value,
            } => auth_consume_once_token(ctx, token_id, token_value).await,
            _ => Err(AuthError::WrongClaims.into()),
        }
    }

    pub async fn sign_user_in(ctx: &ApiContext, uid: UserId) -> ApiResult<UpdatedTokens> {
        let refresh_token = sign_new_refresh_token(ctx, uid.clone()).await?;
        Ok(UpdatedTokens {
            session_header_token: Some(new_session_token(ctx, uid.clone()).await?),
            session_cookie_token: Some(new_session_token(ctx, uid).await?),
            refresh_token: Some(refresh_token),
            clear_cookies: false,
        })
    }
}

impl UpdatedTokens {
    async fn update_all(ctx: &ApiContext, req: &HttpRequest) -> ApiResult<(UserId, Self)> {
        let (uid, refresh_token) = update_refresh_token(ctx, req).await?;
        Ok((
            uid.clone(),
            Self {
                session_header_token: Some(new_session_token(ctx, uid.clone()).await?),
                session_cookie_token: Some(new_session_token(ctx, uid).await?),
                refresh_token,
                clear_cookies: false,
            },
        ))
    }
}

async fn update_refresh_token(
    ctx: &ApiContext,
    req: &HttpRequest,
) -> ApiResult<(UserId, Option<AuthToken>)> {
    let refresh_token = extract_cookie::<AuthToken>(req, COOKIE_AUTH_REFRESH_TOKEN)
        .ok_or(AuthError::MissingRefreshToken)?;
    validate_signature(ctx, &refresh_token).await?;

    match refresh_token.claims {
        AuthTokenClaims::Refresh {
            token_id,
            token_value,
        } => {
            let (uid, token_value) =
                auth_refresh_token_update(ctx, token_id.clone(), token_value).await?;
            match token_value {
                Some(token_value) => Ok((
                    uid,
                    Some(sign_existing_refresh_token_value(ctx, token_id, token_value).await?),
                )),
                None => Ok((uid, None)),
            }
        }
        _ => Err(AuthError::WrongClaims.into()),
    }
}

async fn new_session_token(ctx: &ApiContext, uid: UserId) -> ApiResult<AuthToken> {
    let token_value = AuthTokenValue::from(random_bytes::<32>());
    sign_session_token(ctx, uid, token_value).await
}

pub fn set_auth_cookie(
    res: &mut HttpResponse,
    config: &Config,
    key: &str,
    token: &AuthToken,
) -> ApiResult<()> {
    let domain = config
        .cookie_domain
        .as_ref()
        .map(|domain| format!("; Domain={domain}"))
        .unwrap_or_default();
    let samesite = config.cookie_same_site.as_cookie_attr();
    let token = token.encode_str()?;

    let value = format!(
        "{key}={token}; Path=/{domain}; HttpOnly; Secure; SameSite={samesite}; Max-Age=1209600"
    );

    res.headers_mut().append(
        "Set-Cookie",
        value
            .parse()
            .map_err(|err: InvalidHeaderValue| ApiError::InvalidHeaderValue(err.to_string()))?,
    );
    Ok(())
}

pub fn clear_auth_cookie(res: &mut HttpResponse, config: &Config, key: &str) -> ApiResult<()> {
    let domain = config
        .cookie_domain
        .as_ref()
        .map(|domain| format!("; Domain={domain}"))
        .unwrap_or_default();
    let samesite = config.cookie_same_site.as_cookie_attr();

    let value = format!("{key}=; Path=/{domain}; HttpOnly; Secure; SameSite={samesite}; Max-Age=0");
    res.headers_mut().append(
        "Set-Cookie",
        value
            .parse()
            .map_err(|err: InvalidHeaderValue| ApiError::InvalidHeaderValue(err.to_string()))?,
    );
    Ok(())
}

pub fn set_auth_session_header(res: &mut HttpResponse, token: &AuthToken) -> ApiResult<()> {
    let value = token.encode_str()?;
    res.headers_mut().insert(
        HEADER_AUTH_SESSION_TOKEN_UPDATE,
        value
            .parse()
            .map_err(|err: InvalidHeaderValue| ApiError::InvalidHeaderValue(err.to_string()))?,
    );
    Ok(())
}

async fn validate_signature(ctx: &ApiContext, token: &AuthToken) -> ApiResult<()> {
    let AuthToken { claims, signature } = token;

    if let AuthTokenClaims::Session { expires_at, .. } = claims {
        if (js_sys::Date::now() as u64) > *expires_at {
            return Err(AuthError::Expired.into());
        }
    }

    if verify_bytes(
        &ctx.config.token_signing_key,
        claims.encode()?,
        signature.as_ref(),
    )
    .await?
    {
        Ok(())
    } else {
        Err(AuthError::InvalidSignature("Mismatch".to_string()).into())
    }
}

fn extract_authorization_bearer_header<T: GroupshopCodec>(req: &HttpRequest) -> Option<T> {
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|value| value.to_str().ok())?;

    auth_header
        .strip_prefix("Bearer ")
        .and_then(|token| T::decode_str(token).ok())
}

fn extract_cookie<T: GroupshopCodec>(req: &HttpRequest, key: &'static str) -> Option<T> {
    fn extract_cookie_value<'a>(cookie_header: &'a str, key: &str) -> Option<&'a str> {
        for segment in cookie_header.split(';') {
            let trimmed = segment.trim();
            if let Some((k, v)) = trimmed.split_once('=') {
                if k == key {
                    return Some(v);
                }
            }
        }
        None
    }

    let cookie_header = req
        .headers()
        .get("cookie")
        .or_else(|| req.headers().get("Cookie"))?
        .to_str()
        .ok()?;

    extract_cookie_value(cookie_header, key).and_then(|value| T::decode_str(value).ok())
}
