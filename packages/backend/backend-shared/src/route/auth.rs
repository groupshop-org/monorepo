mod consent;
mod email;
mod openid;
mod refresh;
mod roles;
mod signout;
mod token;

use crate::{
    error::{ApiError, AuthError},
    GroupshopCodec,
};

pub use consent::*;
pub use email::*;
pub use openid::*;
pub use refresh::*;
pub use roles::*;
pub use signout::*;
pub use token::*;

use super::query_param;
use super::AuthRequirement;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ApiAuthRoute {
    EmailPasswordRegister,
    EmailPasswordSignin,
    EmailPasswordSendResetAny,
    EmailPasswordSendResetMe,
    EmailPasswordConfirmReset,
    EmailAddressSendVerification,
    EmailAddressConfirmVerification,
    Signout,
    Refresh,
    OpenIdConnect,
    OpenIdAccessTokenHook(OpenIdTokenAccessTokenHook),
    OpenIdFinalizeExec,
    OpenIdFinalizeQuery,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum OpenIdTokenAccessTokenHook {
    Google { code: String, state: AuthToken },
}

impl ApiAuthRoute {
    pub fn auth_requirement(&self) -> Option<AuthRequirement> {
        match self {
            Self::EmailPasswordRegister => None,
            Self::EmailPasswordSignin => None,
            Self::EmailPasswordSendResetAny => None,
            Self::EmailPasswordSendResetMe => Some(AuthRequirement::Session),
            Self::EmailPasswordConfirmReset => None,
            Self::EmailAddressSendVerification => Some(AuthRequirement::Session),
            Self::EmailAddressConfirmVerification => None,
            Self::Signout => Some(AuthRequirement::Session),
            Self::Refresh => Some(AuthRequirement::Refresh),
            Self::OpenIdConnect => None,
            Self::OpenIdAccessTokenHook(_) => None,
            Self::OpenIdFinalizeExec => None,
            Self::OpenIdFinalizeQuery => None,
        }
    }

    pub fn role_requirement(&self) -> Option<Vec<UserRole>> {
        match self {
            Self::EmailPasswordRegister => None,
            Self::EmailPasswordSignin => None,
            Self::EmailPasswordSendResetAny => None,
            Self::EmailPasswordSendResetMe => Some(vec![UserRole::PartialRegistration]),
            Self::EmailPasswordConfirmReset => None,
            Self::EmailAddressSendVerification => Some(vec![UserRole::PartialRegistration]),
            Self::EmailAddressConfirmVerification => None,
            Self::Signout => Some(vec![UserRole::PartialRegistration]),
            Self::Refresh => Some(vec![UserRole::PartialRegistration]),
            Self::OpenIdConnect => None,
            Self::OpenIdAccessTokenHook(_) => None,
            Self::OpenIdFinalizeExec => None,
            Self::OpenIdFinalizeQuery => None,
        }
    }
}

impl std::fmt::Display for ApiAuthRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::EmailPasswordRegister => "email-password/register".to_string(),
            Self::EmailPasswordSignin => "email-password/signin".to_string(),
            Self::EmailPasswordSendResetAny => "email-password/send-reset-any".to_string(),
            Self::EmailPasswordSendResetMe => "email-password/send-reset-me".to_string(),
            Self::EmailPasswordConfirmReset => "email-password/confirm-reset".to_string(),
            Self::EmailAddressSendVerification => "email-address/send-verification".to_string(),
            Self::EmailAddressConfirmVerification => {
                "email-address/confirm-verification".to_string()
            }
            Self::Signout => "signout".to_string(),
            Self::Refresh => "refresh".to_string(),
            Self::OpenIdConnect => "openid-connect".to_string(),
            Self::OpenIdAccessTokenHook(OpenIdTokenAccessTokenHook::Google { code, state }) => {
                let state = state
                    .encode_str()
                    .unwrap_or_else(|_| "invalid-state".to_string());
                format!("openid-access-token-hook/google?code={code}&state={state}")
            }
            Self::OpenIdFinalizeExec => "openid-finalize-exec".to_string(),
            Self::OpenIdFinalizeQuery => "openid-finalize-query".to_string(),
        };

        write!(f, "{value}")
    }
}

impl TryFrom<&http::Uri> for ApiAuthRoute {
    type Error = ApiError;

    fn try_from(uri: &http::Uri) -> Result<Self, Self::Error> {
        let mut parts = uri
            .path()
            .split('/')
            .filter(|segment| !segment.is_empty() && !segment.starts_with('/'));

        if parts.next().unwrap_or_default() != "auth" {
            return Err(ApiError::UnknownRoute(uri.path().to_string()));
        }

        let remaining = parts.collect::<Vec<_>>();

        match remaining.as_slice() {
            ["email-password", "register"] => Ok(Self::EmailPasswordRegister),
            ["email-password", "signin"] => Ok(Self::EmailPasswordSignin),
            ["email-password", "send-reset-any"] => Ok(Self::EmailPasswordSendResetAny),
            ["email-password", "send-reset-me"] => Ok(Self::EmailPasswordSendResetMe),
            ["email-password", "confirm-reset"] => Ok(Self::EmailPasswordConfirmReset),
            ["email-address", "send-verification"] => Ok(Self::EmailAddressSendVerification),
            ["email-address", "confirm-verification"] => Ok(Self::EmailAddressConfirmVerification),
            ["signout"] => Ok(Self::Signout),
            ["refresh"] => Ok(Self::Refresh),
            ["openid-connect"] => Ok(Self::OpenIdConnect),
            ["openid-access-token-hook", "google"] => {
                let query = uri.query().unwrap_or_default();
                if let Some(err) = query_param(query, "error") {
                    return Err(AuthError::OpenId(err).into());
                }

                let code = query_param(query, "code")
                    .filter(|value: &String| !value.is_empty())
                    .ok_or_else(|| AuthError::OpenId("missing `code`".to_string()))?;
                let state_raw = query_param(query, "state")
                    .filter(|value: &String| !value.is_empty())
                    .ok_or_else(|| AuthError::OpenId("missing `state`".to_string()))?;
                let state = AuthToken::decode_str(&state_raw)?;

                Ok(Self::OpenIdAccessTokenHook(
                    OpenIdTokenAccessTokenHook::Google { code, state },
                ))
            }
            ["openid-finalize-exec"] => Ok(Self::OpenIdFinalizeExec),
            ["openid-finalize-query"] => Ok(Self::OpenIdFinalizeQuery),
            _ => Err(ApiError::UnknownRoute(uri.path().to_string())),
        }
    }
}
