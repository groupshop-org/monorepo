use http::Method;

use crate::route::{ApiAuthRoute, ApiRoute, ApiRouteEmpty};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthSignoutRoute;

impl ApiRouteEmpty for AuthSignoutRoute {
    const ROUTE: ApiRoute = ApiRoute::Auth(ApiAuthRoute::Signout);
    const METHOD: Method = Method::POST;
}
