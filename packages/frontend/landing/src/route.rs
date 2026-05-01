use std::sync::Arc;

use groupshop_backend_shared::{
    prelude::{ApiError, AuthError, AuthToken, ProductId, UserRole},
    GroupshopCodec,
};
use groupshop_frontend_shared::window;

#[derive(Debug, Clone)]
pub enum Route {
    Home,
    Product { id: ProductId },
    PrivacyPolicy,
    TermsOfService,
    Signin,
    Register,
    VerifyEmail,
    VerifyEmailConfirm { token: AuthToken },
    ResetPassword { token: AuthToken },
    ChooseUsername,
    OpenIdFinalize { token: AuthToken },
    Profile,
    Orders,
    Error(Arc<ApiError>),
    NotFound,
}

#[derive(Clone, Debug)]
pub enum Resolved {
    Redirect(Route),
    Render(Route),
}

impl Route {
    pub fn current() -> Self {
        let path = window()
            .location()
            .pathname()
            .unwrap_or_else(|_| "/".to_string());

        let parts = path
            .trim_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();

        match parts.as_slice() {
            [] => Self::Home,
            ["product", slug] => match ProductId::new(*slug) {
                Ok(id) => Self::Product { id },
                Err(_) => Self::NotFound,
            },
            ["privacy-policy"] => Self::PrivacyPolicy,
            ["terms-of-service"] => Self::TermsOfService,
            ["signin"] => Self::Signin,
            ["register"] => Self::Register,
            ["verify-email"] => Self::VerifyEmail,
            ["verify-email-confirm", token] => match AuthToken::decode_str(token) {
                Ok(token) => Self::VerifyEmailConfirm { token },
                Err(err) => Self::Error(Arc::new(ApiError::Auth(AuthError::VerifyEmail(format!(
                    "invalid token in URL: {err}"
                ))))),
            },
            ["reset-password", token] => match AuthToken::decode_str(token) {
                Ok(token) => Self::ResetPassword { token },
                Err(err) => Self::Error(Arc::new(ApiError::Auth(AuthError::ResetPassword(
                    format!("invalid token in URL: {err}"),
                )))),
            },
            ["choose-username"] => Self::ChooseUsername,
            ["openid-finalize", token] => match AuthToken::decode_str(token) {
                Ok(token) => Self::OpenIdFinalize { token },
                Err(err) => Self::Error(Arc::new(ApiError::Auth(AuthError::OpenId(format!(
                    "invalid token in URL: {err}"
                ))))),
            },
            ["profile"] => Self::Profile,
            ["orders"] => Self::Orders,
            ["error", error] => match ApiError::decode_str(error) {
                Ok(error) => Self::Error(Arc::new(error)),
                Err(err) => Self::Error(Arc::new(ApiError::Unknown(err.to_string()))),
            },
            _ => Self::NotFound,
        }
    }

    pub fn link(&self) -> String {
        match self {
            Self::Home => "/".to_string(),
            Self::Product { id } => format!("/product/{}", id.as_str()),
            Self::PrivacyPolicy => "/privacy-policy".to_string(),
            Self::TermsOfService => "/terms-of-service".to_string(),
            Self::Signin => "/signin".to_string(),
            Self::Register => "/register".to_string(),
            Self::VerifyEmail => "/verify-email".to_string(),
            Self::VerifyEmailConfirm { token } => {
                format!("/verify-email-confirm/{}", token.encode_str().unwrap())
            }
            Self::ResetPassword { token } => {
                format!("/reset-password/{}", token.encode_str().unwrap())
            }
            Self::ChooseUsername => "/choose-username".to_string(),
            Self::OpenIdFinalize { token } => {
                format!("/openid-finalize/{}", token.encode_str().unwrap())
            }
            Self::Profile => "/profile".to_string(),
            Self::Orders => "/orders".to_string(),
            Self::Error(err) => format!(
                "/error/{}",
                err.encode_str().unwrap_or_else(|_| "unknown".to_string())
            ),
            Self::NotFound => "/not-found".to_string(),
        }
    }

    pub fn go_to_url(&self) {
        let _ = window().location().set_href(&self.link());
    }

    pub fn resolve(
        self,
        profile: Option<&groupshop_backend_shared::prelude::AccountProfile>,
    ) -> Resolved {
        use Route::*;

        match &self {
            VerifyEmailConfirm { .. } | ResetPassword { .. } | OpenIdFinalize { .. } | Error(_) => {
                return Resolved::Render(self);
            }
            _ => {}
        }

        match profile {
            None => match self {
                Home
                | Product { .. }
                | PrivacyPolicy
                | TermsOfService
                | Signin
                | Register
                | NotFound => Resolved::Render(self),
                _ => Resolved::Redirect(Home),
            },
            Some(profile) => {
                let needs_verify = !profile.roles.contains(&UserRole::EmailVerified);
                let needs_username = !profile.roles.contains(&UserRole::UsernameChosen);

                if needs_verify {
                    if matches!(self, VerifyEmail) {
                        return Resolved::Render(self);
                    }
                    return Resolved::Redirect(VerifyEmail);
                }

                if needs_username {
                    if matches!(self, ChooseUsername) {
                        return Resolved::Render(self);
                    }
                    return Resolved::Redirect(ChooseUsername);
                }

                match self {
                    Signin | Register | VerifyEmail | ChooseUsername => Resolved::Redirect(Home),
                    _ => Resolved::Render(self),
                }
            }
        }
    }
}
