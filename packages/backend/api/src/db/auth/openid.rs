use wasm_bindgen::prelude::*;
use worker::d1::{D1Database, D1PreparedStatement};

use crate::db::tables::SQL_TABLE_USER_AUTH_OPENID;
use crate::{
    prelude::*,
    utils::{db_execute, db_load, db_prepare, get_d1},
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserAuthOpenIdDb {
    pub provider: String,
    pub subject: String,
    pub email: String,
    pub user_id: UserId,
    pub created_at: String,
}

impl UserAuthOpenIdDb {
    pub async fn load(
        ctx: &ApiContext,
        provider: OpenIdProvider,
        subject: &str,
    ) -> ApiResult<Self> {
        db_load(
            db_prepare(
                &get_d1(&ctx.env)?,
                format!(
                    "SELECT * FROM {SQL_TABLE_USER_AUTH_OPENID} WHERE provider = ?1 AND subject = ?2 LIMIT 1"
                ),
                &[JsValue::from_str(provider.as_str()), JsValue::from_str(subject)],
            )?,
            "openid account mapping not found",
        )
        .await
    }

    pub fn prepare_insert_or_update_email(
        d1: &D1Database,
        provider: &OpenIdProvider,
        subject: &str,
        email: &str,
        user_id: &UserId,
    ) -> ApiResult<D1PreparedStatement> {
        db_prepare(
            d1,
            format!(
                "INSERT INTO {SQL_TABLE_USER_AUTH_OPENID} (provider, subject, email, user_id) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(provider, subject) DO UPDATE SET email = excluded.email, user_id = excluded.user_id"
            ),
            &[
                JsValue::from_str(provider.as_str()),
                JsValue::from_str(subject),
                JsValue::from_str(email),
                JsValue::from_str(&user_id.to_string()),
            ],
        )
    }

    pub async fn insert_or_update_email(
        ctx: &ApiContext,
        provider: &OpenIdProvider,
        subject: &str,
        email: &str,
        user_id: &UserId,
    ) -> ApiResult<()> {
        db_execute(Self::prepare_insert_or_update_email(
            &get_d1(&ctx.env)?,
            provider,
            subject,
            email,
            user_id,
        )?)
        .await
    }

    pub async fn load_any_by_user_id(ctx: &ApiContext, user_id: &UserId) -> ApiResult<Self> {
        db_load(
            db_prepare(
                &get_d1(&ctx.env)?,
                format!(
                    "SELECT * FROM {SQL_TABLE_USER_AUTH_OPENID} WHERE user_id = ?1 ORDER BY created_at ASC LIMIT 1"
                ),
                &[JsValue::from_str(&user_id.to_string())],
            )?,
            format!("Need openid account mapping (user_id {user_id})"),
        )
        .await
    }
}
