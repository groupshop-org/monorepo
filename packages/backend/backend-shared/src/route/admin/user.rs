use http::Method;

use crate::{
    id::UserId,
    route::{ApiAdminRoute, ApiRoute, ApiRouteRequest, ApiRouteRequestResponse, UserRole},
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminListUsersRoute;

impl ApiRouteRequestResponse for AdminListUsersRoute {
    const ROUTE: ApiRoute = ApiRoute::Admin(ApiAdminRoute::ListUsers);
    type Req = AdminListUsersRequest;
    type Res = AdminListUsersResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminListUsersRequest {
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminListUsersResponse {
    pub page: u32,
    pub per_page: u32,
    pub total_users: u32,
    pub users: Vec<AdminUserSummary>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminUpdateUserRoute;

impl ApiRouteRequestResponse for AdminUpdateUserRoute {
    const ROUTE: ApiRoute = ApiRoute::Admin(ApiAdminRoute::UpdateUser);
    type Req = AdminUpdateUserRequest;
    type Res = AdminUpdateUserResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminUpdateUserRequest {
    pub id: UserId,
    pub roles: Vec<UserRole>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminUpdateUserResponse {
    pub user: AdminUserSummary,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminDeleteUserRoute;

impl ApiRouteRequest for AdminDeleteUserRoute {
    const ROUTE: ApiRoute = ApiRoute::Admin(ApiAdminRoute::DeleteUser);
    type Req = AdminDeleteUserRequest;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminDeleteUserRequest {
    pub id: UserId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminUserSummary {
    pub id: UserId,
    pub username: String,
    pub email: Option<String>,
    pub roles: Vec<UserRole>,
    pub created_at: String,
}
