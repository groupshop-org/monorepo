mod user;

use crate::error::ApiError;

pub use user::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ApiAdminRoute {
    ListUsers,
    UpdateUser,
    DeleteUser,
}

impl std::fmt::Display for ApiAdminRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::ListUsers => "list-users",
            Self::UpdateUser => "update-user",
            Self::DeleteUser => "delete-user",
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
            _ => Err(ApiError::UnknownRoute(uri.path().to_string())),
        }
    }
}
