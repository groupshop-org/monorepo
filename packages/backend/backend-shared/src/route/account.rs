mod escrow;
mod orders;
mod profile;
mod username;

use crate::{error::ApiError, route::AuthRequirement};

pub use escrow::*;
pub use orders::*;
pub use profile::*;
pub use username::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ApiAccountRoute {
    EscrowDepositIntent,
    EscrowDepositBuild,
    EscrowDepositSubmit,
    EscrowDepositConfirm,
    EscrowRefundBuild,
    EscrowRefundSubmit,
    EscrowRefundConfirm,
    Orders,
    OrderStatus,
    Profile,
    ProfileUpdate,
    UsernameCheck,
    UsernameUpdate,
}

impl ApiAccountRoute {
    pub fn auth_requirement(&self) -> Option<AuthRequirement> {
        Some(AuthRequirement::Session)
    }

    pub fn role_requirement(&self) -> Option<Vec<UserRole>> {
        Some(vec![UserRole::PartialRegistration])
    }
}

impl std::fmt::Display for ApiAccountRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::EscrowDepositIntent => "escrow-deposit-intent",
            Self::EscrowDepositBuild => "escrow-deposit-build",
            Self::EscrowDepositSubmit => "escrow-deposit-submit",
            Self::EscrowDepositConfirm => "escrow-deposit-confirm",
            Self::EscrowRefundBuild => "escrow-refund-build",
            Self::EscrowRefundSubmit => "escrow-refund-submit",
            Self::EscrowRefundConfirm => "escrow-refund-confirm",
            Self::Orders => "orders",
            Self::OrderStatus => "order-status",
            Self::Profile => "profile",
            Self::ProfileUpdate => "profile-update",
            Self::UsernameCheck => "username-check",
            Self::UsernameUpdate => "username-update",
        };

        write!(f, "{value}")
    }
}

impl TryFrom<&http::Uri> for ApiAccountRoute {
    type Error = ApiError;

    fn try_from(uri: &http::Uri) -> Result<Self, Self::Error> {
        let mut parts = uri
            .path()
            .split('/')
            .filter(|segment| !segment.is_empty() && !segment.starts_with('/'));

        if parts.next().unwrap_or_default() != "account" {
            return Err(ApiError::UnknownRoute(uri.path().to_string()));
        }

        let remaining = parts.collect::<Vec<_>>();

        match remaining.as_slice() {
            ["escrow-deposit-intent"] => Ok(Self::EscrowDepositIntent),
            ["escrow-deposit-build"] => Ok(Self::EscrowDepositBuild),
            ["escrow-deposit-submit"] => Ok(Self::EscrowDepositSubmit),
            ["escrow-deposit-confirm"] => Ok(Self::EscrowDepositConfirm),
            ["escrow-refund-build"] => Ok(Self::EscrowRefundBuild),
            ["escrow-refund-submit"] => Ok(Self::EscrowRefundSubmit),
            ["escrow-refund-confirm"] => Ok(Self::EscrowRefundConfirm),
            ["orders"] => Ok(Self::Orders),
            ["order-status"] => Ok(Self::OrderStatus),
            ["profile"] => Ok(Self::Profile),
            ["profile-update"] => Ok(Self::ProfileUpdate),
            ["username-check"] => Ok(Self::UsernameCheck),
            ["username-update"] => Ok(Self::UsernameUpdate),
            _ => Err(ApiError::UnknownRoute(uri.path().to_string())),
        }
    }
}

use crate::route::UserRole;
