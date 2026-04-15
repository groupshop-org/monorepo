use http::Method;

use crate::{
    id::{AccountUsername, UserId},
    route::{ApiAccountRoute, ApiRoute, ApiRouteRequest, ApiRouteResponse, UserRole},
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountProfileRoute;

impl ApiRouteResponse for AccountProfileRoute {
    const ROUTE: ApiRoute = ApiRoute::Account(ApiAccountRoute::Profile);
    type Res = AccountProfile;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountProfileUpdateRoute;

impl ApiRouteRequest for AccountProfileUpdateRoute {
    const ROUTE: ApiRoute = ApiRoute::Account(ApiAccountRoute::ProfileUpdate);
    type Req = AccountProfileUpdateRequest;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ShippingAddress {
    pub line1: String,
    pub line2: String,
    pub city: String,
    pub state: String,
    pub postal_code: String,
    pub country: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountProfile {
    pub uid: UserId,
    pub email: String,
    pub username: AccountUsername,
    pub roles: Vec<UserRole>,
    pub full_name: String,
    pub shipping_address: ShippingAddress,
    pub receive_marketing: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountProfileUpdateRequest {
    pub full_name: String,
    pub shipping_address: ShippingAddress,
    pub receive_marketing: bool,
}
