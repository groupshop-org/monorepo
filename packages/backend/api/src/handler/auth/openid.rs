use base64::Engine;
use serde::Deserialize;
use worker::{Fetch, Headers, Method, Request, RequestInit};

use crate::{
    config::LandingRoute,
    db::auth::{email::UserAuthEmailDb, openid::UserAuthOpenIdDb, role::UserAuthRoleDb},
    durable::auth::once::AuthTokenOnceKind,
    handler::auth::{register::register_user, validation::Validation},
    prelude::*,
    token_signing::sign_once_token,
    utils::{random_bytes, redirect_response, req_to_json},
};

pub async fn handle_openid_connect(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AuthOpenIdConnectResponse> {
    let req: AuthOpenIdConnectRequest = req_to_json(req).await?;

    match req.provider {
        OpenIdProvider::Google => {
            let token = sign_once_token(
                ctx,
                AuthTokenOnceKind::OpenIdHook {
                    provider: OpenIdProvider::Google,
                },
            )
            .await?;

            let url = format!(
                "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile&prompt=select_account&state={}",
                urlencoding::encode(&ctx.config.openid_google_client_id),
                urlencoding::encode(&ctx.config.openid_google_redirect_uri),
                urlencoding::encode(&token.encode_str()?),
            );

            Ok(AuthOpenIdConnectResponse { url })
        }
    }
}

pub async fn handle_openid_token_hook(
    ctx: &mut ApiContext,
    _req: HttpRequest,
    token_hook: OpenIdTokenAccessTokenHook,
) -> ApiResult<HttpResponse> {
    match inner_handle_openid_token_hook(ctx, token_hook).await {
        Ok(token) => Ok(redirect_response(
            &ctx.config
                .landing_url(LandingRoute::OpenIdFinalize { token }),
        )),
        Err(err) => Ok(redirect_response(
            &ctx.config.landing_url(LandingRoute::Error {
                message: ApiError::Auth(AuthError::OpenId(format!("{err}")))
                    .encode_str()
                    .unwrap_or_else(|_| "openid-failed".to_string()),
            }),
        )),
    }
}

async fn inner_handle_openid_token_hook(
    ctx: &mut ApiContext,
    token_hook: OpenIdTokenAccessTokenHook,
) -> ApiResult<AuthToken> {
    match token_hook {
        OpenIdTokenAccessTokenHook::Google { code, state } => {
            let kind = Validation::once_token_consume(ctx, &state).await?;

            let identity = match kind {
                AuthTokenOnceKind::OpenIdHook { provider }
                    if provider == OpenIdProvider::Google =>
                {
                    GoogleIdentity::new(ctx, &code).await?
                }
                _ => {
                    return Err(ApiError::Auth(AuthError::OpenId(
                        "invalid state token".to_string(),
                    )))
                }
            };

            let uid = if let Ok(existing_mapping) =
                UserAuthOpenIdDb::load(ctx, OpenIdProvider::Google, &identity.subject).await
            {
                Some(existing_mapping.user_id)
            } else if let Ok(existing_email) = UserAuthEmailDb::load(ctx, &identity.email).await {
                UserAuthOpenIdDb::insert_or_update_email(
                    ctx,
                    &OpenIdProvider::Google,
                    &identity.subject,
                    &identity.email,
                    &existing_email.user_id,
                )
                .await?;
                if ctx.config.is_default_admin_email(&identity.email) {
                    UserAuthRoleDb::insert_roles(ctx, &existing_email.user_id, &[UserRole::Admin])
                        .await?;
                }
                Some(existing_email.user_id)
            } else {
                None
            };

            sign_once_token(
                ctx,
                AuthTokenOnceKind::OpenIdFinalize {
                    uid,
                    subject: identity.subject,
                    email: identity.email,
                    email_verified: identity.email_verified,
                    provider: OpenIdProvider::Google,
                },
            )
            .await
        }
    }
}

pub async fn handle_openid_finalize_query(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AuthOpenIdFinalizeQueryResponse> {
    let req: AuthOpenIdFinalizeQueryRequest = req_to_json(req).await?;

    match Validation::once_token_peek(ctx, &req.token).await? {
        AuthTokenOnceKind::OpenIdFinalize { uid, .. } => match uid {
            Some(uid) => Ok(AuthOpenIdFinalizeQueryResponse::UserExists {
                roles: UserAuthRoleDb::load_roles(ctx, &uid).await?,
                uid,
            }),
            None => Ok(AuthOpenIdFinalizeQueryResponse::UserDoesNotExist),
        },
        _ => Err(AuthError::OpenId("invalid finalize token".to_string()).into()),
    }
}

pub async fn handle_openid_finalize_exec(ctx: &mut ApiContext, req: HttpRequest) -> ApiResult<()> {
    let req: AuthOpenIdFinalizeExecRequest = req_to_json(req).await?;

    let kind = Validation::once_token_consume(ctx, &req.token).await?;

    let (uid, subject, email, email_verified, provider) = match kind {
        AuthTokenOnceKind::OpenIdFinalize {
            uid,
            subject,
            email,
            email_verified,
            provider,
        } => (uid, subject, email, email_verified, provider),
        _ => {
            return Err(AuthError::OpenId("invalid finalize token".to_string()).into());
        }
    };

    let user = match uid {
        Some(uid) => UserAuthOpenIdDb::load_any_by_user_id(ctx, &uid).await.ok(),
        None => None,
    };

    let uid = match user {
        Some(user) => {
            let uid = user.user_id;
            let roles = UserAuthRoleDb::load_roles(ctx, &uid).await?;
            if !roles.contains(&UserRole::EmailVerified) && email_verified {
                UserAuthRoleDb::insert_roles(ctx, &uid, &[UserRole::EmailVerified]).await?;
            }
            if ctx.config.is_default_admin_email(&email) {
                UserAuthRoleDb::insert_roles(ctx, &uid, &[UserRole::Admin]).await?;
            }
            uid
        }
        None => {
            let consent = req.consent.unwrap_or_default();
            let password =
                base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes::<32>());

            register_user(
                ctx,
                &email,
                email_verified,
                &password,
                &consent,
                Some((provider, subject)),
            )
            .await?
        }
    };

    ctx.updated_tokens = Validation::sign_user_in(ctx, uid).await?;
    Ok(())
}

#[derive(Debug)]
struct GoogleIdentity {
    subject: String,
    email: String,
    email_verified: bool,
}

impl GoogleIdentity {
    async fn new(ctx: &ApiContext, code: &str) -> ApiResult<Self> {
        let body = format!(
            "code={}&client_id={}&client_secret={}&redirect_uri={}&grant_type=authorization_code",
            urlencoding::encode(code),
            urlencoding::encode(&ctx.config.openid_google_client_id),
            urlencoding::encode(&ctx.config.openid_google_client_secret),
            urlencoding::encode(&ctx.config.openid_google_redirect_uri),
        );

        let headers = Headers::new();
        headers.set("Accept", "application/json").unwrap();
        headers
            .set("Content-Type", "application/x-www-form-urlencoded")
            .unwrap();

        let mut init = RequestInit::new();
        init.with_method(Method::Post);
        init.with_headers(headers);
        init.with_body(Some(body.into()));

        let token_req = Request::new_with_init("https://oauth2.googleapis.com/token", &init)
            .map_err(|err| ApiError::Request(err.to_string()))?;
        let mut token_res = Fetch::Request(token_req)
            .send()
            .await
            .map_err(|err| ApiError::Request(err.to_string()))?;

        if !(200..300).contains(&token_res.status_code()) {
            return Err(ApiError::Auth(AuthError::OpenId(
                token_res
                    .text()
                    .await
                    .unwrap_or_else(|_| "google token exchange failed".to_string()),
            )));
        }

        let token_data = token_res
            .json::<GoogleTokenResponse>()
            .await
            .map_err(|err| ApiError::ParseBody(err.to_string()))?;
        let id_token = token_data
            .id_token
            .ok_or_else(|| AuthError::OpenId("missing id_token".to_string()))?;

        let token_info_url = format!(
            "https://oauth2.googleapis.com/tokeninfo?id_token={}",
            urlencoding::encode(&id_token)
        );
        let token_info_req = Request::new(&token_info_url, Method::Get)
            .map_err(|err| ApiError::Request(err.to_string()))?;
        let mut token_info_res = Fetch::Request(token_info_req)
            .send()
            .await
            .map_err(|err| ApiError::Request(err.to_string()))?;

        if !(200..300).contains(&token_info_res.status_code()) {
            return Err(ApiError::Auth(AuthError::OpenId(
                token_info_res
                    .text()
                    .await
                    .unwrap_or_else(|_| "google token validation failed".to_string()),
            )));
        }

        let token_info = token_info_res
            .json::<GoogleTokenInfoResponse>()
            .await
            .map_err(|err| ApiError::ParseBody(err.to_string()))?;

        validate_google_token_info(
            &token_info,
            &ctx.config.openid_google_client_id,
            (js_sys::Date::now() as u64) / 1000,
        )?;

        Ok(Self {
            subject: token_info.sub,
            email: token_info.email,
            email_verified: token_info.email_verified,
        })
    }
}

fn validate_google_token_info(
    token_info: &GoogleTokenInfoResponse,
    client_id: &str,
    now_secs: u64,
) -> ApiResult<()> {
    if token_info.aud != client_id {
        return Err(AuthError::OpenId("google audience mismatch".to_string()).into());
    }
    if token_info.iss != "https://accounts.google.com" && token_info.iss != "accounts.google.com" {
        return Err(AuthError::OpenId("google issuer mismatch".to_string()).into());
    }
    if token_info.exp <= now_secs {
        return Err(AuthError::OpenId("google token expired".to_string()).into());
    }
    if token_info.iat > now_secs.saturating_add(300) {
        return Err(AuthError::OpenId("google token issued in the future".to_string()).into());
    }
    if token_info.email.trim().is_empty() || token_info.sub.trim().is_empty() {
        return Err(AuthError::OpenId("google token missing required claims".to_string()).into());
    }
    Ok(())
}

#[derive(Deserialize)]
struct GoogleTokenResponse {
    id_token: Option<String>,
}

#[derive(Deserialize)]
struct GoogleTokenInfoResponse {
    sub: String,
    email: String,
    #[serde(default, deserialize_with = "deserialize_google_email_verified")]
    email_verified: bool,
    iss: String,
    aud: String,
    #[serde(deserialize_with = "deserialize_google_unix_secs")]
    exp: u64,
    #[serde(deserialize_with = "deserialize_google_unix_secs")]
    iat: u64,
}

fn deserialize_google_email_verified<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum BoolOrString {
        Bool(bool),
        String(String),
    }

    let value = BoolOrString::deserialize(deserializer)?;
    Ok(match value {
        BoolOrString::Bool(value) => value,
        BoolOrString::String(value) => value.eq_ignore_ascii_case("true"),
    })
}

fn deserialize_google_unix_secs<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumOrString {
        Num(u64),
        String(String),
    }

    match NumOrString::deserialize(deserializer)? {
        NumOrString::Num(value) => Ok(value),
        NumOrString::String(value) => value
            .parse::<u64>()
            .map_err(|_| serde::de::Error::custom("invalid unix seconds value")),
    }
}
