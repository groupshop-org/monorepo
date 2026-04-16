use http::Method;

use crate::{
    id::ProductBrandId,
    route::{ApiAdminRoute, ApiRoute, ApiRouteRequest, ApiRouteRequestResponse},
};

// --- List Brands ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminListBrandsRoute;

impl ApiRouteRequestResponse for AdminListBrandsRoute {
    const ROUTE: ApiRoute = ApiRoute::Admin(ApiAdminRoute::ListBrands);
    type Req = AdminListBrandsRequest;
    type Res = AdminListBrandsResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminListBrandsRequest {
    pub page: u32,
    pub per_page: u32,
    pub search: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminListBrandsResponse {
    pub page: u32,
    pub per_page: u32,
    pub total: u32,
    pub brands: Vec<AdminBrandSummary>,
}

// --- Create Brand ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminCreateBrandRoute;

impl ApiRouteRequestResponse for AdminCreateBrandRoute {
    const ROUTE: ApiRoute = ApiRoute::Admin(ApiAdminRoute::CreateBrand);
    type Req = AdminCreateBrandRequest;
    type Res = AdminCreateBrandResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminCreateBrandRequest {
    pub id: ProductBrandId,
    pub name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminCreateBrandResponse {
    pub brand: AdminBrandSummary,
}

// --- Update Brand ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminUpdateBrandRoute;

impl ApiRouteRequestResponse for AdminUpdateBrandRoute {
    const ROUTE: ApiRoute = ApiRoute::Admin(ApiAdminRoute::UpdateBrand);
    type Req = AdminUpdateBrandRequest;
    type Res = AdminUpdateBrandResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminUpdateBrandRequest {
    pub id: ProductBrandId,
    pub name: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminUpdateBrandResponse {
    pub brand: AdminBrandSummary,
}

// --- Delete Brand ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminDeleteBrandRoute;

impl ApiRouteRequest for AdminDeleteBrandRoute {
    const ROUTE: ApiRoute = ApiRoute::Admin(ApiAdminRoute::DeleteBrand);
    type Req = AdminDeleteBrandRequest;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminDeleteBrandRequest {
    pub id: ProductBrandId,
}

// --- Shared Summary ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminBrandSummary {
    pub id: ProductBrandId,
    pub name: String,
}
