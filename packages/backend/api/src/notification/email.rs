use serde::Serialize;
use worker::{Fetch, Headers, Request, RequestInit};

use crate::{
    config::LandingRoute, db::auth::email::UserAuthEmailDb, durable::auth::once::AuthTokenOnceKind,
    prelude::*, token_signing::sign_once_token,
};

#[derive(Clone)]
pub enum EmailNotification {
    VerifyEmail { uid: UserId },
    ResetPassword { uid: UserId },
}

impl From<EmailNotification> for AuthTokenOnceKind {
    fn from(value: EmailNotification) -> Self {
        match value {
            EmailNotification::VerifyEmail { uid } => AuthTokenOnceKind::VerifyEmail { uid },
            EmailNotification::ResetPassword { uid } => AuthTokenOnceKind::ResetPassword { uid },
        }
    }
}

impl EmailNotification {
    pub async fn send(self, ctx: &ApiContext) -> ApiResult<()> {
        let uid = match &self {
            Self::VerifyEmail { uid } => uid,
            Self::ResetPassword { uid } => uid,
        };

        let email_address = UserAuthEmailDb::load_by_user_id(ctx, uid).await?.email;
        let token = sign_once_token(ctx, self.clone().into()).await?;
        let token = token.encode_str()?;

        let url = match self {
            Self::VerifyEmail { .. } => ctx.config.landing_url(LandingRoute::VerifyEmailConfirm {
                token: token.clone(),
            }),
            Self::ResetPassword { .. } => ctx.config.landing_url(LandingRoute::ResetPassword {
                token: token.clone(),
            }),
        };

        if ctx.config.debug_auth_links_in_console {
            worker::console_warn!("auth action url: {url}");
        }

        let Some(server_token) = ctx.config.postmark_server_token.as_ref() else {
            return Ok(());
        };
        let Some(from_email) = ctx.config.postmark_from_email.as_ref() else {
            return Ok(());
        };
        let Some(message_stream) = ctx.config.postmark_message_stream.as_ref() else {
            return Ok(());
        };

        let (subject, html) = match &self {
            Self::VerifyEmail { .. } => (
                "Verify your Groupshop email",
                format!("<p>Verify your email: <a href=\"{url}\">{url}</a></p>"),
            ),
            Self::ResetPassword { .. } => (
                "Reset your Groupshop password",
                format!("<p>Reset your password: <a href=\"{url}\">{url}</a></p>"),
            ),
        };

        #[derive(Serialize)]
        struct PostmarkBody<'a> {
            #[serde(rename = "From")]
            from: &'a str,
            #[serde(rename = "To")]
            to: &'a str,
            #[serde(rename = "Subject")]
            subject: &'a str,
            #[serde(rename = "HtmlBody")]
            html_body: &'a str,
            #[serde(rename = "MessageStream")]
            message_stream: &'a str,
        }

        let body = serde_json::to_string(&PostmarkBody {
            from: from_email,
            to: &email_address,
            subject,
            html_body: &html,
            message_stream,
        })
        .map_err(|err| ApiError::Unknown(err.to_string()))?;

        let headers = Headers::new();
        headers
            .set("Accept", "application/json")
            .map_err(|err| ApiError::Unknown(err.to_string()))?;
        headers
            .set("Content-Type", "application/json")
            .map_err(|err| ApiError::Unknown(err.to_string()))?;
        headers
            .set("X-Postmark-Server-Token", server_token)
            .map_err(|err| ApiError::Unknown(err.to_string()))?;

        let mut init = RequestInit::new();
        init.with_method(worker::Method::Post);
        init.with_headers(headers);
        init.with_body(Some(body.into()));

        let req = Request::new_with_init("https://api.postmarkapp.com/email", &init)
            .map_err(|err| ApiError::Unknown(err.to_string()))?;
        let mut resp = Fetch::Request(req)
            .send()
            .await
            .map_err(|err| ApiError::Unknown(err.to_string()))?;

        if !(200..300).contains(&resp.status_code()) {
            let body = resp
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable body>".to_string());
            return Err(ApiError::Unknown(format!(
                "postmark returned status {}: {body}",
                resp.status_code()
            )));
        }

        Ok(())
    }
}
