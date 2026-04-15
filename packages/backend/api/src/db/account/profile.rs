use wasm_bindgen::prelude::*;
use worker::d1::{D1Database, D1PreparedStatement};

use crate::db::tables::SQL_TABLE_USER_ACCOUNT_PROFILE;
use crate::{
    prelude::*,
    utils::{db_execute, db_exists, db_load, db_prepare, deserialize_d1_bool, get_d1},
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserAccountProfileDb {
    pub user_id: UserId,
    pub username: String,
    pub full_name: String,
    pub shipping_line1: String,
    pub shipping_line2: String,
    pub shipping_city: String,
    pub shipping_state: String,
    pub shipping_postal_code: String,
    pub shipping_country: String,
    #[serde(deserialize_with = "deserialize_d1_bool")]
    pub receive_marketing: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl UserAccountProfileDb {
    pub fn prepare_insert(
        d1: &D1Database,
        uid: &UserId,
        username: &AccountUsername,
        receive_marketing: bool,
    ) -> ApiResult<D1PreparedStatement> {
        db_prepare(
            d1,
            format!(
                "INSERT INTO {SQL_TABLE_USER_ACCOUNT_PROFILE}
                (user_id, username, full_name, shipping_line1, shipping_line2, shipping_city, shipping_state, shipping_postal_code, shipping_country, receive_marketing)
                VALUES (?1, ?2, '', '', '', '', '', '', '', ?3)"
            ),
            &[
                JsValue::from_str(&uid.to_string()),
                JsValue::from_str(username.as_str()),
                JsValue::from_bool(receive_marketing),
            ],
        )
    }

    pub async fn load_by_user_id(ctx: &ApiContext, user_id: &UserId) -> ApiResult<Self> {
        db_load(
            db_prepare(
                &get_d1(&ctx.env)?,
                format!("SELECT * FROM {SQL_TABLE_USER_ACCOUNT_PROFILE} WHERE user_id = ?1"),
                &[JsValue::from_str(&user_id.to_string())],
            )?,
            "profile not found for user",
        )
        .await
    }

    pub async fn update_username(
        ctx: &ApiContext,
        user_id: &UserId,
        username: &AccountUsername,
    ) -> ApiResult<()> {
        db_execute(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "UPDATE {SQL_TABLE_USER_ACCOUNT_PROFILE} SET username = ?1, updated_at = CURRENT_TIMESTAMP WHERE user_id = ?2"
            ),
            &[
                JsValue::from_str(username.as_str()),
                JsValue::from_str(&user_id.to_string()),
            ],
        )?)
        .await
    }

    pub async fn update_profile(
        ctx: &ApiContext,
        user_id: &UserId,
        req: &AccountProfileUpdateRequest,
    ) -> ApiResult<()> {
        let shipping = &req.shipping_address;
        db_execute(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "UPDATE {SQL_TABLE_USER_ACCOUNT_PROFILE}
                 SET full_name = ?1,
                     shipping_line1 = ?2,
                     shipping_line2 = ?3,
                     shipping_city = ?4,
                     shipping_state = ?5,
                     shipping_postal_code = ?6,
                     shipping_country = ?7,
                     receive_marketing = ?8,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE user_id = ?9"
            ),
            &[
                JsValue::from_str(req.full_name.trim()),
                JsValue::from_str(shipping.line1.trim()),
                JsValue::from_str(shipping.line2.trim()),
                JsValue::from_str(shipping.city.trim()),
                JsValue::from_str(shipping.state.trim()),
                JsValue::from_str(shipping.postal_code.trim()),
                JsValue::from_str(shipping.country.trim()),
                JsValue::from_bool(req.receive_marketing),
                JsValue::from_str(&user_id.to_string()),
            ],
        )?)
        .await
    }

    pub async fn username_exists(ctx: &ApiContext, username: &AccountUsername) -> ApiResult<bool> {
        db_exists(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "SELECT 1 AS n FROM {SQL_TABLE_USER_ACCOUNT_PROFILE} WHERE username = ?1 LIMIT 1"
            ),
            &[JsValue::from_str(username.as_str())],
        )?)
        .await
    }
}

pub fn is_username_unique_violation(db_message: &str) -> bool {
    let lower = db_message.to_ascii_lowercase();
    lower.contains("unique")
        && (lower.contains("user_account_profile.username")
            || lower.contains("idx_user_account_profile_username"))
}
