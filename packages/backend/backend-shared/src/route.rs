mod account;
mod admin;
mod auth;

use http::Method;
use serde::{de::DeserializeOwned, Serialize};

pub use account::*;
pub use admin::*;
pub use auth::*;

use crate::error::ApiError;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum AuthRequirement {
    Session,
    Refresh,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ApiRoute {
    Auth(ApiAuthRoute),
    Admin(ApiAdminRoute),
    Account(ApiAccountRoute),
}

impl ApiRoute {
    pub fn auth_requirement(&self) -> Option<AuthRequirement> {
        match self {
            Self::Auth(route) => route.auth_requirement(),
            Self::Admin(_) => Some(AuthRequirement::Session),
            Self::Account(route) => route.auth_requirement(),
        }
    }

    pub fn role_requirement(&self) -> Option<Vec<UserRole>> {
        match self {
            Self::Auth(route) => route.role_requirement(),
            Self::Admin(_) => Some(UserRole::admin().to_vec()),
            Self::Account(route) => route.role_requirement(),
        }
    }
}

impl std::fmt::Display for ApiRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Auth(route) => format!("auth/{route}"),
            Self::Admin(route) => format!("admin/{route}"),
            Self::Account(route) => format!("account/{route}"),
        };

        write!(f, "{value}")
    }
}

impl TryFrom<&http::Uri> for ApiRoute {
    type Error = ApiError;

    fn try_from(uri: &http::Uri) -> Result<Self, Self::Error> {
        let first_part = uri
            .path()
            .split('/')
            .find(|segment| !segment.is_empty() && !segment.starts_with('/'))
            .unwrap_or_default();

        match first_part {
            "auth" => Ok(Self::Auth(uri.try_into()?)),
            "admin" => Ok(Self::Admin(uri.try_into()?)),
            "account" => Ok(Self::Account(uri.try_into()?)),
            _ => Err(ApiError::UnknownRoute(uri.path().to_string())),
        }
    }
}

pub fn query_param(query: &str, key: &str) -> Option<String> {
    for segment in query.split('&') {
        let (k, v) = segment.split_once('=')?;
        if k == key {
            return urlencoding::decode(v).ok().map(|s| s.to_string());
        }
    }
    None
}

pub trait ApiReq: DeserializeOwned + Serialize + 'static {}
impl<T> ApiReq for T where T: DeserializeOwned + Serialize + 'static {}

pub trait ApiRes: DeserializeOwned + Serialize + 'static {}
impl<T> ApiRes for T where T: DeserializeOwned + Serialize + 'static {}

pub trait ApiRouteRequestResponse {
    const ROUTE: ApiRoute;
    type Req: ApiReq;
    type Res: ApiRes;
    const METHOD: Method;
}

pub trait ApiRouteRequest {
    const ROUTE: ApiRoute;
    type Req: ApiReq;
    const METHOD: Method;
}

pub trait ApiRouteResponse {
    const ROUTE: ApiRoute;
    type Res: ApiRes;
    const METHOD: Method;
}

pub trait ApiRouteEmpty {
    const ROUTE: ApiRoute;
    const METHOD: Method;
}
