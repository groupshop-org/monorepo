use http::Method;

use crate::{
    id::{ProductBrandId, ProductCategoryId, ProductId},
    route::{ApiAdminRoute, ApiRoute, ApiRouteRequest, ApiRouteRequestResponse},
};

// --- List Products ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminListProductsRoute;

impl ApiRouteRequestResponse for AdminListProductsRoute {
    const ROUTE: ApiRoute = ApiRoute::Admin(ApiAdminRoute::ListProducts);
    type Req = AdminListProductsRequest;
    type Res = AdminListProductsResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminListProductsRequest {
    pub page: u32,
    pub per_page: u32,
    pub category_id: Option<ProductCategoryId>,
    pub brand_id: Option<ProductBrandId>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminListProductsResponse {
    pub page: u32,
    pub per_page: u32,
    pub total: u32,
    pub products: Vec<AdminProductSummary>,
}

// --- Create Product ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminCreateProductRoute;

impl ApiRouteRequestResponse for AdminCreateProductRoute {
    const ROUTE: ApiRoute = ApiRoute::Admin(ApiAdminRoute::CreateProduct);
    type Req = AdminCreateProductRequest;
    type Res = AdminCreateProductResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminCreateProductRequest {
    pub id: ProductId,
    pub gtin: String,
    pub name: String,
    pub category_id: ProductCategoryId,
    pub brand_id: ProductBrandId,
    pub price_cents: u32,
    pub currency: String,
    pub minimum_order_quantity: u32,
    pub inventory: u32,
    pub is_preorder: bool,
    pub estimated_delivery_weeks: Option<u32>,
    pub supplier_url: String,
    pub image_url: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminCreateProductResponse {
    pub product: AdminProductSummary,
}

// --- Update Product ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminUpdateProductRoute;

impl ApiRouteRequestResponse for AdminUpdateProductRoute {
    const ROUTE: ApiRoute = ApiRoute::Admin(ApiAdminRoute::UpdateProduct);
    type Req = AdminUpdateProductRequest;
    type Res = AdminUpdateProductResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminUpdateProductRequest {
    pub id: ProductId,
    pub name: Option<String>,
    pub price_cents: Option<u32>,
    pub currency: Option<String>,
    pub minimum_order_quantity: Option<u32>,
    pub inventory: Option<u32>,
    pub is_preorder: Option<bool>,
    pub estimated_delivery_weeks: Option<Option<u32>>,
    pub supplier_url: Option<String>,
    pub image_url: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminUpdateProductResponse {
    pub product: AdminProductSummary,
}

// --- Delete Product ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminDeleteProductRoute;

impl ApiRouteRequest for AdminDeleteProductRoute {
    const ROUTE: ApiRoute = ApiRoute::Admin(ApiAdminRoute::DeleteProduct);
    type Req = AdminDeleteProductRequest;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminDeleteProductRequest {
    pub id: ProductId,
}

// --- Shared Summary ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminProductSummary {
    pub id: ProductId,
    pub gtin: String,
    pub name: String,
    pub category_id: ProductCategoryId,
    pub brand_id: ProductBrandId,
    pub price_cents: u32,
    pub currency: String,
    pub minimum_order_quantity: u32,
    pub inventory: u32,
    pub is_preorder: bool,
    pub estimated_delivery_weeks: Option<u32>,
    pub supplier_url: String,
    pub image_url: String,
    pub is_active: bool,
    pub created_at: String,
}
