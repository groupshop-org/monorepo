use http::Method;

use crate::{
    id::UserId,
    route::{
        ApiAuthRoute, ApiRoute, ApiRouteRequest, ApiRouteRequestResponse, AuthRegistrationConsent,
        AuthToken, UserRole,
    },
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthOpenIdConnectRoute;

impl ApiRouteRequestResponse for AuthOpenIdConnectRoute {
    const ROUTE: ApiRoute = ApiRoute::Auth(ApiAuthRoute::OpenIdConnect);
    type Req = AuthOpenIdConnectRequest;
    type Res = AuthOpenIdConnectResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthOpenIdConnectRequest {
    pub provider: OpenIdProvider,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthOpenIdConnectResponse {
    pub url: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthOpenIdFinalizeQueryRoute;

impl ApiRouteRequestResponse for AuthOpenIdFinalizeQueryRoute {
    const ROUTE: ApiRoute = ApiRoute::Auth(ApiAuthRoute::OpenIdFinalizeQuery);
    type Req = AuthOpenIdFinalizeQueryRequest;
    type Res = AuthOpenIdFinalizeQueryResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthOpenIdFinalizeQueryRequest {
    pub token: AuthToken,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AuthOpenIdFinalizeQueryResponse {
    UserExists { uid: UserId, roles: Vec<UserRole> },
    UserDoesNotExist,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthOpenIdFinalizeExecRoute;

impl ApiRouteRequest for AuthOpenIdFinalizeExecRoute {
    const ROUTE: ApiRoute = ApiRoute::Auth(ApiAuthRoute::OpenIdFinalizeExec);
    type Req = AuthOpenIdFinalizeExecRequest;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthOpenIdFinalizeExecRequest {
    pub token: AuthToken,
    pub consent: Option<AuthRegistrationConsent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OpenIdProvider {
    Google,
}

impl OpenIdProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Google => "google",
        }
    }
}
