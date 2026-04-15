mod account;
mod admin;
mod auth;

use http::{Method, StatusCode};
use serde_json::json;

use crate::cors::valid_cors_request;
use crate::{
    context::ApiContext,
    handler::auth::validation::{
        clear_auth_cookie, set_auth_cookie, set_auth_session_header, Validation,
    },
    prelude::*,
    utils::{empty_response, json_response},
};

pub async fn handle_request(mut ctx: ApiContext, req: HttpRequest) -> worker::Result<HttpResponse> {
    if req.method() == Method::OPTIONS {
        return Ok(empty_response(None));
    }

    if !valid_cors_request(&req, &ctx.config) {
        return Ok(json_response(
            &json!({"error": "CORS origin not allowed"}),
            Some(StatusCode::FORBIDDEN),
        )
        .await
        .unwrap());
    }

    let route: ApiRoute = match req.uri().try_into() {
        Ok(route) => route,
        Err(err) => {
            return Ok(json_response(&err, Some(StatusCode::NOT_FOUND))
                .await
                .unwrap());
        }
    };

    if let Err(err) = Validation::root_handler(&mut ctx, &route, &req).await {
        return Ok(json_response(&err, Some(StatusCode::UNAUTHORIZED))
            .await
            .unwrap());
    }

    let res = handle_route(&mut ctx, route, req).await;

    let mut response = match res {
        Ok(ApiHandlerResponse::Json(value)) => json_response(&value, None).await.unwrap(),
        Ok(ApiHandlerResponse::Raw(response)) => response,
        Err(err) => json_response(&err, Some(StatusCode::BAD_REQUEST))
            .await
            .unwrap(),
    };

    if ctx.updated_tokens.clear_cookies {
        for key in [COOKIE_AUTH_REFRESH_TOKEN, COOKIE_AUTH_SESSION_TOKEN] {
            if let Err(err) = clear_auth_cookie(&mut response, &ctx.config, key) {
                return Ok(json_response(&err, Some(StatusCode::INTERNAL_SERVER_ERROR))
                    .await
                    .unwrap());
            }
        }
    } else {
        if let Some(token) = ctx.updated_tokens.refresh_token {
            if let Err(err) = set_auth_cookie(
                &mut response,
                &ctx.config,
                COOKIE_AUTH_REFRESH_TOKEN,
                &token,
            ) {
                return Ok(json_response(&err, Some(StatusCode::INTERNAL_SERVER_ERROR))
                    .await
                    .unwrap());
            }
        }

        if let Some(token) = ctx.updated_tokens.session_cookie_token {
            if let Err(err) = set_auth_cookie(
                &mut response,
                &ctx.config,
                COOKIE_AUTH_SESSION_TOKEN,
                &token,
            ) {
                return Ok(json_response(&err, Some(StatusCode::INTERNAL_SERVER_ERROR))
                    .await
                    .unwrap());
            }
        }

        if let Some(token) = ctx.updated_tokens.session_header_token {
            if let Err(err) = set_auth_session_header(&mut response, &token) {
                return Ok(json_response(&err, Some(StatusCode::INTERNAL_SERVER_ERROR))
                    .await
                    .unwrap());
            }
        }
    }

    Ok(response)
}

async fn handle_route(
    ctx: &mut ApiContext,
    route: ApiRoute,
    req: HttpRequest,
) -> ApiResult<ApiHandlerResponse> {
    match route {
        ApiRoute::Auth(auth_route) => match auth_route {
            ApiAuthRoute::OpenIdConnect => {
                Ok(auth::openid::handle_openid_connect(ctx, req).await?.boxed())
            }
            ApiAuthRoute::OpenIdAccessTokenHook(token_hook) => Ok(ApiHandlerResponse::raw(
                auth::openid::handle_openid_token_hook(ctx, req, token_hook).await?,
            )),
            ApiAuthRoute::OpenIdFinalizeQuery => {
                Ok(auth::openid::handle_openid_finalize_query(ctx, req)
                    .await?
                    .boxed())
            }
            ApiAuthRoute::OpenIdFinalizeExec => {
                auth::openid::handle_openid_finalize_exec(ctx, req).await?;
                Ok(ApiHandlerResponse::raw(empty_response(None)))
            }
            ApiAuthRoute::Refresh => Ok(ApiHandlerResponse::raw(empty_response(None))),
            ApiAuthRoute::Signout => {
                ctx.updated_tokens.clear_cookies = true;
                Ok(ApiHandlerResponse::raw(empty_response(None)))
            }
            ApiAuthRoute::EmailPasswordRegister => {
                auth::email_password::handle_email_password_register(ctx, req).await?;
                Ok(ApiHandlerResponse::raw(empty_response(None)))
            }
            ApiAuthRoute::EmailPasswordSignin => {
                auth::email_password::handle_email_password_signin(ctx, req).await?;
                Ok(ApiHandlerResponse::raw(empty_response(None)))
            }
            ApiAuthRoute::EmailPasswordSendResetAny => {
                auth::email_password::handle_email_password_send_reset_any(ctx, req).await?;
                Ok(ApiHandlerResponse::raw(empty_response(None)))
            }
            ApiAuthRoute::EmailPasswordSendResetMe => {
                auth::email_password::handle_email_password_send_reset_me(ctx, req).await?;
                Ok(ApiHandlerResponse::raw(empty_response(None)))
            }
            ApiAuthRoute::EmailPasswordConfirmReset => {
                auth::email_password::handle_email_password_confirm_reset(ctx, req).await?;
                Ok(ApiHandlerResponse::raw(empty_response(None)))
            }
            ApiAuthRoute::EmailAddressSendVerification => {
                auth::email_password::handle_email_address_send_verification(ctx, req).await?;
                Ok(ApiHandlerResponse::raw(empty_response(None)))
            }
            ApiAuthRoute::EmailAddressConfirmVerification => {
                auth::email_password::handle_email_address_confirm_verification(ctx, req).await?;
                Ok(ApiHandlerResponse::raw(empty_response(None)))
            }
        },
        ApiRoute::Admin(admin_route) => match admin_route {
            ApiAdminRoute::ListUsers => Ok(admin::user::handle_list_users(ctx, req).await?.boxed()),
            ApiAdminRoute::UpdateUser => {
                Ok(admin::user::handle_update_user(ctx, req).await?.boxed())
            }
            ApiAdminRoute::DeleteUser => {
                admin::user::handle_delete_user(ctx, req).await?;
                Ok(ApiHandlerResponse::raw(empty_response(None)))
            }
        },
        ApiRoute::Account(account_route) => match account_route {
            ApiAccountRoute::Profile => {
                Ok(account::profile::handle_profile(ctx, req).await?.boxed())
            }
            ApiAccountRoute::ProfileUpdate => {
                account::profile::handle_profile_update(ctx, req).await?;
                Ok(ApiHandlerResponse::raw(empty_response(None)))
            }
            ApiAccountRoute::UsernameCheck => {
                Ok(account::username::handle_username_check(ctx, req)
                    .await?
                    .boxed())
            }
            ApiAccountRoute::UsernameUpdate => {
                Ok(account::username::handle_username_update(ctx, req)
                    .await?
                    .boxed())
            }
        },
    }
}
