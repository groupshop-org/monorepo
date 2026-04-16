mod brand;
mod category;
mod product;
mod user;

use crate::error::ApiError;

pub use brand::*;
pub use category::*;
pub use product::*;
pub use user::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ApiAdminRoute {
    ListUsers,
    UpdateUser,
    DeleteUser,
    ListProducts,
    CreateProduct,
    UpdateProduct,
    DeleteProduct,
    ListCategories,
    CreateCategory,
    UpdateCategory,
    DeleteCategory,
    ListBrands,
    CreateBrand,
    UpdateBrand,
    DeleteBrand,
}

impl std::fmt::Display for ApiAdminRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::ListUsers => "list-users",
            Self::UpdateUser => "update-user",
            Self::DeleteUser => "delete-user",
            Self::ListProducts => "list-products",
            Self::CreateProduct => "create-product",
            Self::UpdateProduct => "update-product",
            Self::DeleteProduct => "delete-product",
            Self::ListCategories => "list-categories",
            Self::CreateCategory => "create-category",
            Self::UpdateCategory => "update-category",
            Self::DeleteCategory => "delete-category",
            Self::ListBrands => "list-brands",
            Self::CreateBrand => "create-brand",
            Self::UpdateBrand => "update-brand",
            Self::DeleteBrand => "delete-brand",
        };

        write!(f, "{value}")
    }
}

impl TryFrom<&http::Uri> for ApiAdminRoute {
    type Error = ApiError;

    fn try_from(uri: &http::Uri) -> Result<Self, Self::Error> {
        let mut parts = uri
            .path()
            .split('/')
            .filter(|segment| !segment.is_empty() && !segment.starts_with('/'));

        if parts.next().unwrap_or_default() != "admin" {
            return Err(ApiError::UnknownRoute(uri.path().to_string()));
        }

        let remaining = parts.collect::<Vec<_>>();

        match remaining.as_slice() {
            ["list-users"] => Ok(Self::ListUsers),
            ["update-user"] => Ok(Self::UpdateUser),
            ["delete-user"] => Ok(Self::DeleteUser),
            ["list-products"] => Ok(Self::ListProducts),
            ["create-product"] => Ok(Self::CreateProduct),
            ["update-product"] => Ok(Self::UpdateProduct),
            ["delete-product"] => Ok(Self::DeleteProduct),
            ["list-categories"] => Ok(Self::ListCategories),
            ["create-category"] => Ok(Self::CreateCategory),
            ["update-category"] => Ok(Self::UpdateCategory),
            ["delete-category"] => Ok(Self::DeleteCategory),
            ["list-brands"] => Ok(Self::ListBrands),
            ["create-brand"] => Ok(Self::CreateBrand),
            ["update-brand"] => Ok(Self::UpdateBrand),
            ["delete-brand"] => Ok(Self::DeleteBrand),
            _ => Err(ApiError::UnknownRoute(uri.path().to_string())),
        }
    }
}
