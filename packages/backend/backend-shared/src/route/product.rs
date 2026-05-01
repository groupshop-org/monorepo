use crate::{
    error::ApiError,
    id::{ProductBrandId, ProductCategoryId, ProductId},
    route::{ApiRoute, ApiRouteRequestResponse, ApiRouteResponse, AuthRequirement, UserRole},
};
use http::Method;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ApiProductRoute {
    List,
    Detail,
    Categories,
    Brands,
}

impl ApiProductRoute {
    pub fn auth_requirement(&self) -> Option<AuthRequirement> {
        None
    }

    pub fn role_requirement(&self) -> Option<Vec<UserRole>> {
        None
    }
}

impl std::fmt::Display for ApiProductRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::List => "list",
            Self::Detail => "detail",
            Self::Categories => "categories",
            Self::Brands => "brands",
        };

        write!(f, "{value}")
    }
}

impl TryFrom<&http::Uri> for ApiProductRoute {
    type Error = ApiError;

    fn try_from(uri: &http::Uri) -> Result<Self, Self::Error> {
        let mut parts = uri
            .path()
            .split('/')
            .filter(|segment| !segment.is_empty() && !segment.starts_with('/'));

        if parts.next().unwrap_or_default() != "product" {
            return Err(ApiError::UnknownRoute(uri.path().to_string()));
        }

        let remaining = parts.collect::<Vec<_>>();

        match remaining.as_slice() {
            ["list"] => Ok(Self::List),
            ["detail"] => Ok(Self::Detail),
            ["categories"] => Ok(Self::Categories),
            ["brands"] => Ok(Self::Brands),
            _ => Err(ApiError::UnknownRoute(uri.path().to_string())),
        }
    }
}

// --- List Products (public, filtered, paginated) ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProductListRoute;

impl ApiRouteRequestResponse for ProductListRoute {
    const ROUTE: ApiRoute = ApiRoute::Product(ApiProductRoute::List);
    type Req = ProductListRequest;
    type Res = ProductListResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProductListRequest {
    pub page: u32,
    pub per_page: u32,
    pub category_id: Option<ProductCategoryId>,
    pub brand_id: Option<ProductBrandId>,
    pub search: Option<String>,
    /// When `true`, only products that have at least one confirmed
    /// participation are returned. Backs the "deals gaining traction"
    /// landing-page filter.
    #[serde(default)]
    pub with_participants_only: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProductListResponse {
    pub page: u32,
    pub per_page: u32,
    pub total: u32,
    pub products: Vec<ProductSummary>,
}

// --- Product Detail ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProductDetailRoute;

impl ApiRouteRequestResponse for ProductDetailRoute {
    const ROUTE: ApiRoute = ApiRoute::Product(ApiProductRoute::Detail);
    type Req = ProductDetailRequest;
    type Res = ProductDetailResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProductDetailRequest {
    pub id: ProductId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProductDetailResponse {
    pub product: ProductSummary,
}

// --- Categories (full tree) ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProductCategoriesRoute;

impl ApiRouteResponse for ProductCategoriesRoute {
    const ROUTE: ApiRoute = ApiRoute::Product(ApiProductRoute::Categories);
    type Res = ProductCategoriesResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProductCategoriesResponse {
    pub categories: Vec<ProductCategorySummary>,
}

// --- Brands (all) ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProductBrandsRoute;

impl ApiRouteResponse for ProductBrandsRoute {
    const ROUTE: ApiRoute = ApiRoute::Product(ApiProductRoute::Brands);
    type Res = ProductBrandsResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProductBrandsResponse {
    pub brands: Vec<ProductBrandSummary>,
}

// --- Shared Summaries ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProductSummary {
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
    pub image_url: String,
    pub is_active: bool,
    /// Sum of `quantity` across non-refunded participations on the
    /// product's active batch. Compare against `minimum_order_quantity`
    /// (which is the unit threshold) to render the progress bar.
    pub committed_units: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProductCategorySummary {
    pub id: ProductCategoryId,
    pub name: String,
    pub parent_id: Option<ProductCategoryId>,
    pub depth: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProductBrandSummary {
    pub id: ProductBrandId,
    pub name: String,
}
