use crate::error::{ApiResult, AuthError};

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AuthRegistrationConsent {
    pub accept_terms_of_service: bool,
    pub accept_privacy_policy: bool,
    pub opt_in_marketing_emails: bool,
}

impl AuthRegistrationConsent {
    pub fn validate(&self) -> ApiResult<()> {
        if self.accept_terms_of_service && self.accept_privacy_policy {
            Ok(())
        } else {
            Err(AuthError::RegistrationConsentRequired.into())
        }
    }
}
