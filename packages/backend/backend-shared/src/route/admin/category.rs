use http::Method;

use crate::{
    id::ProductCategoryId,
    route::{ApiAdminRoute, ApiRoute, ApiRouteRequest, ApiRouteRequestResponse, ApiRouteResponse},
};

// --- List Categories ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminListCategoriesRoute;

impl ApiRouteResponse for AdminListCategoriesRoute {
    const ROUTE: ApiRoute = ApiRoute::Admin(ApiAdminRoute::ListCategories);
    type Res = AdminListCategoriesResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminListCategoriesResponse {
    pub categories: Vec<AdminCategorySummary>,
}

// --- Create Category ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminCreateCategoryRoute;

impl ApiRouteRequestResponse for AdminCreateCategoryRoute {
    const ROUTE: ApiRoute = ApiRoute::Admin(ApiAdminRoute::CreateCategory);
    type Req = AdminCreateCategoryRequest;
    type Res = AdminCreateCategoryResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminCreateCategoryRequest {
    pub id: ProductCategoryId,
    pub name: String,
    pub parent_id: Option<ProductCategoryId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminCreateCategoryResponse {
    pub category: AdminCategorySummary,
}

// --- Update Category ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminUpdateCategoryRoute;

impl ApiRouteRequestResponse for AdminUpdateCategoryRoute {
    const ROUTE: ApiRoute = ApiRoute::Admin(ApiAdminRoute::UpdateCategory);
    type Req = AdminUpdateCategoryRequest;
    type Res = AdminUpdateCategoryResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminUpdateCategoryRequest {
    pub id: ProductCategoryId,
    pub name: Option<String>,
    pub parent_id: Option<Option<ProductCategoryId>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminUpdateCategoryResponse {
    pub category: AdminCategorySummary,
}

// --- Delete Category ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminDeleteCategoryRoute;

impl ApiRouteRequest for AdminDeleteCategoryRoute {
    const ROUTE: ApiRoute = ApiRoute::Admin(ApiAdminRoute::DeleteCategory);
    type Req = AdminDeleteCategoryRequest;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminDeleteCategoryRequest {
    pub id: ProductCategoryId,
}

// --- Shared Summary ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminCategorySummary {
    pub id: ProductCategoryId,
    pub name: String,
    pub parent_id: Option<ProductCategoryId>,
    pub depth: u32,
}
