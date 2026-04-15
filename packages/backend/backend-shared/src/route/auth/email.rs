use http::Method;

use crate::route::{
    ApiAuthRoute, ApiRoute, ApiRouteEmpty, ApiRouteRequest, AuthRegistrationConsent, AuthToken,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthEmailAddressSendVerificationRoute;

impl ApiRouteEmpty for AuthEmailAddressSendVerificationRoute {
    const ROUTE: ApiRoute = ApiRoute::Auth(ApiAuthRoute::EmailAddressSendVerification);
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthEmailAddressConfirmVerificationRoute;

impl ApiRouteRequest for AuthEmailAddressConfirmVerificationRoute {
    const ROUTE: ApiRoute = ApiRoute::Auth(ApiAuthRoute::EmailAddressConfirmVerification);
    type Req = AuthEmailAddressConfirmVerificationRequest;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthEmailAddressConfirmVerificationRequest {
    pub token: AuthToken,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthEmailPasswordRegisterRoute;

impl ApiRouteRequest for AuthEmailPasswordRegisterRoute {
    const ROUTE: ApiRoute = ApiRoute::Auth(ApiAuthRoute::EmailPasswordRegister);
    type Req = AuthEmailPasswordRegisterRequest;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthEmailPasswordRegisterRequest {
    pub email: String,
    pub password: String,
    pub consent: AuthRegistrationConsent,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthEmailPasswordSigninRoute;

impl ApiRouteRequest for AuthEmailPasswordSigninRoute {
    const ROUTE: ApiRoute = ApiRoute::Auth(ApiAuthRoute::EmailPasswordSignin);
    type Req = AuthEmailPasswordSigninRequest;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthEmailPasswordSigninRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthEmailPasswordConfirmResetRoute;

impl ApiRouteRequest for AuthEmailPasswordConfirmResetRoute {
    const ROUTE: ApiRoute = ApiRoute::Auth(ApiAuthRoute::EmailPasswordConfirmReset);
    type Req = AuthEmailPasswordConfirmResetRequest;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthEmailPasswordConfirmResetRequest {
    pub token: AuthToken,
    pub new_password: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthEmailPasswordSendResetAnyRoute;

impl ApiRouteRequest for AuthEmailPasswordSendResetAnyRoute {
    const ROUTE: ApiRoute = ApiRoute::Auth(ApiAuthRoute::EmailPasswordSendResetAny);
    type Req = AuthEmailPasswordSendResetAnyRequest;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthEmailPasswordSendResetAnyRequest {
    pub email: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthEmailPasswordSendResetMeRoute;

impl ApiRouteEmpty for AuthEmailPasswordSendResetMeRoute {
    const ROUTE: ApiRoute = ApiRoute::Auth(ApiAuthRoute::EmailPasswordSendResetMe);
    const METHOD: Method = Method::POST;
}
