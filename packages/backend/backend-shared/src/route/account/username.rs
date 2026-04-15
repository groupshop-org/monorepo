use http::Method;

use crate::{
    error::{ApiError, ApiResult, AuthError},
    id::AccountUsername,
    route::{ApiAccountRoute, ApiRoute, ApiRouteRequestResponse},
};

pub const USERNAME_MIN_LEN: usize = 3;
pub const USERNAME_MAX_LEN: usize = 32;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountUsernameCheckRoute;

impl ApiRouteRequestResponse for AccountUsernameCheckRoute {
    const ROUTE: ApiRoute = ApiRoute::Account(ApiAccountRoute::UsernameCheck);
    type Req = AccountUsernameCheckRequest;
    type Res = AccountUsernameCheckResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountUsernameCheckRequest {
    pub username: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountUsernameCheckResponse {
    pub available: bool,
    pub valid: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountUsernameUpdateRoute;

impl ApiRouteRequestResponse for AccountUsernameUpdateRoute {
    const ROUTE: ApiRoute = ApiRoute::Account(ApiAccountRoute::UsernameUpdate);
    type Req = AccountUsernameUpdateRequest;
    type Res = AccountUsernameUpdateResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountUsernameUpdateRequest {
    pub username: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountUsernameUpdateResponse {}

impl AccountUsername {
    pub fn validated(username: impl Into<String>) -> ApiResult<Self> {
        let username = username.into();
        let username = username.trim();

        if username.is_empty() {
            return Err(ApiError::Auth(AuthError::UsernameEmpty));
        }

        let len = username.chars().count();
        if !(USERNAME_MIN_LEN..=USERNAME_MAX_LEN).contains(&len) {
            return Err(ApiError::Auth(AuthError::UsernameInvalid));
        }

        Self::new(username.to_string()).map_err(|_| ApiError::Auth(AuthError::UsernameInvalid))
    }
}
