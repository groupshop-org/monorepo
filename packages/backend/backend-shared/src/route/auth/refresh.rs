use http::Method;

use crate::route::{ApiAuthRoute, ApiRoute, ApiRouteEmpty};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthRefreshRoute;

impl ApiRouteEmpty for AuthRefreshRoute {
    const ROUTE: ApiRoute = ApiRoute::Auth(ApiAuthRoute::Refresh);
    const METHOD: Method = Method::POST;
}
