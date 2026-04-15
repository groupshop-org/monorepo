use wasm_bindgen::prelude::*;
use worker::d1::{D1Database, D1PreparedStatement};

use crate::db::tables::SQL_TABLE_USER_AUTH_EMAIL;
use crate::{
    prelude::*,
    utils::{db_execute, db_load, db_prepare, db_try_load, get_d1},
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserAuthEmailDb {
    pub email: String,
    pub password_hash: String,
    pub user_id: UserId,
    pub created_at: String,
}

impl UserAuthEmailDb {
    pub fn prepare_insert(
        d1: &D1Database,
        email: &str,
        password_hash: &str,
        user_id: &UserId,
    ) -> ApiResult<D1PreparedStatement> {
        let email = normalize_email(email);
        db_prepare(
            d1,
            format!(
                "INSERT INTO {SQL_TABLE_USER_AUTH_EMAIL} (email, password_hash, user_id) VALUES (?1, ?2, ?3)"
            ),
            &[
                JsValue::from_str(&email),
                JsValue::from_str(password_hash),
                JsValue::from_str(&user_id.to_string()),
            ],
        )
    }

    pub async fn try_load(ctx: &ApiContext, email: &str) -> ApiResult<Option<Self>> {
        let email = normalize_email(email);
        db_try_load(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "SELECT * FROM {SQL_TABLE_USER_AUTH_EMAIL} WHERE lower(email) = lower(?1) LIMIT 1"
            ),
            &[JsValue::from_str(&email)],
        )?)
        .await
    }

    pub async fn load(ctx: &ApiContext, email: &str) -> ApiResult<Self> {
        let email = normalize_email(email);
        db_load(
            db_prepare(
                &get_d1(&ctx.env)?,
                format!(
                    "SELECT * FROM {SQL_TABLE_USER_AUTH_EMAIL} WHERE lower(email) = lower(?1) LIMIT 1"
                ),
                &[JsValue::from_str(&email)],
            )?,
            format!("Need to register (email {email})"),
        )
        .await
    }

    pub async fn update_password_hash(
        ctx: &ApiContext,
        user_id: &UserId,
        password_hash: &str,
    ) -> ApiResult<()> {
        db_execute(db_prepare(
            &get_d1(&ctx.env)?,
            format!("UPDATE {SQL_TABLE_USER_AUTH_EMAIL} SET password_hash = ?1 WHERE user_id = ?2"),
            &[
                JsValue::from_str(password_hash),
                JsValue::from_str(&user_id.to_string()),
            ],
        )?)
        .await
    }

    pub async fn load_by_user_id(ctx: &ApiContext, user_id: &UserId) -> ApiResult<Self> {
        db_load(
            db_prepare(
                &get_d1(&ctx.env)?,
                format!("SELECT * FROM {SQL_TABLE_USER_AUTH_EMAIL} WHERE user_id = ?1 LIMIT 1"),
                &[JsValue::from_str(&user_id.to_string())],
            )?,
            format!("Need email account mapping (user_id {user_id})"),
        )
        .await
    }
}

pub fn is_email_unique_violation(db_message: &str) -> bool {
    let lower = db_message.to_ascii_lowercase();
    lower.contains("unique") && lower.contains("user_auth_email.email")
}

pub fn normalize_email(email: &str) -> String {
    email.trim().to_ascii_lowercase()
}
