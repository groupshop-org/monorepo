use std::collections::HashMap;

use wasm_bindgen::prelude::*;

use crate::db::tables::{
    SQL_TABLE_USER_ACCOUNT_PROFILE, SQL_TABLE_USER_AUTH_EMAIL, SQL_TABLE_USER_AUTH_OPENID,
    SQL_TABLE_USER_AUTH_ROLE,
};
use crate::{
    prelude::*,
    utils::{db_batch, db_load, db_load_all, db_prepare, get_d1},
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminUserRowDb {
    pub id: UserId,
    pub username: String,
    pub email: Option<String>,
    pub created_at: String,
}

impl AdminUserRowDb {
    pub async fn count(ctx: &ApiContext) -> ApiResult<u32> {
        #[derive(serde::Deserialize)]
        struct CountRow {
            n: f64,
        }

        let row: CountRow = db_load(
            db_prepare(
                &get_d1(&ctx.env)?,
                format!("SELECT count(*) AS n FROM {SQL_TABLE_USER_ACCOUNT_PROFILE}"),
                &[],
            )?,
            "count query failed",
        )
        .await?;

        Ok(row.n as u32)
    }

    pub async fn load_page(ctx: &ApiContext, page: u32, per_page: u32) -> ApiResult<Vec<Self>> {
        let offset = (page.saturating_sub(1)) * per_page;

        db_load_all(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "SELECT
                    p.user_id AS id,
                    p.username AS username,
                    COALESCE(
                        (SELECT e.email FROM {SQL_TABLE_USER_AUTH_EMAIL} e WHERE e.user_id = p.user_id ORDER BY e.created_at ASC LIMIT 1),
                        (SELECT o.email FROM {SQL_TABLE_USER_AUTH_OPENID} o WHERE o.user_id = p.user_id ORDER BY o.created_at ASC LIMIT 1)
                    ) AS email,
                    p.created_at AS created_at
                 FROM {SQL_TABLE_USER_ACCOUNT_PROFILE} p
                 ORDER BY p.created_at DESC, p.user_id DESC
                 LIMIT ?1 OFFSET ?2"
            ),
            &[
                JsValue::from_f64(per_page as f64),
                JsValue::from_f64(offset as f64),
            ],
        )?)
        .await
    }

    pub async fn load_by_user_id(ctx: &ApiContext, user_id: &UserId) -> ApiResult<Self> {
        db_load(
            db_prepare(
                &get_d1(&ctx.env)?,
                format!(
                    "SELECT
                        p.user_id AS id,
                        p.username AS username,
                        COALESCE(
                            (SELECT e.email FROM {SQL_TABLE_USER_AUTH_EMAIL} e WHERE e.user_id = p.user_id ORDER BY e.created_at ASC LIMIT 1),
                            (SELECT o.email FROM {SQL_TABLE_USER_AUTH_OPENID} o WHERE o.user_id = p.user_id ORDER BY o.created_at ASC LIMIT 1)
                        ) AS email,
                        p.created_at AS created_at
                    FROM {SQL_TABLE_USER_ACCOUNT_PROFILE} p
                    WHERE p.user_id = ?1"
                ),
                &[JsValue::from_str(&user_id.to_string())],
            )?,
            "admin user not found",
        )
        .await
    }

    pub async fn load_roles_by_user_id(
        ctx: &ApiContext,
        user_ids: &[UserId],
    ) -> ApiResult<HashMap<String, Vec<UserRole>>> {
        #[derive(serde::Deserialize)]
        struct RoleRow {
            role_id: u8,
        }

        let mut by_user = HashMap::new();

        for user_id in user_ids {
            let rows: Vec<RoleRow> = db_load_all(db_prepare(
                &get_d1(&ctx.env)?,
                format!(
                    "SELECT role_id FROM {SQL_TABLE_USER_AUTH_ROLE} WHERE user_id = ?1 ORDER BY role_id ASC"
                ),
                &[JsValue::from_str(&user_id.to_string())],
            )?)
            .await?;

            by_user.insert(
                user_id.to_string(),
                rows.into_iter()
                    .map(|row| UserRole::from(row.role_id))
                    .collect(),
            );
        }

        Ok(by_user)
    }

    pub async fn delete_all_user_data(ctx: &ApiContext, user_id: &UserId) -> ApiResult<()> {
        let d1 = get_d1(&ctx.env)?;
        let bindings = &[JsValue::from_str(&user_id.to_string())];
        let stmts = vec![
            db_prepare(
                &d1,
                format!("DELETE FROM {SQL_TABLE_USER_AUTH_ROLE} WHERE user_id = ?1"),
                bindings,
            )?,
            db_prepare(
                &d1,
                format!("DELETE FROM {SQL_TABLE_USER_AUTH_EMAIL} WHERE user_id = ?1"),
                bindings,
            )?,
            db_prepare(
                &d1,
                format!("DELETE FROM {SQL_TABLE_USER_AUTH_OPENID} WHERE user_id = ?1"),
                bindings,
            )?,
            db_prepare(
                &d1,
                format!("DELETE FROM {SQL_TABLE_USER_ACCOUNT_PROFILE} WHERE user_id = ?1"),
                bindings,
            )?,
        ];

        db_batch(&d1, stmts).await
    }
}
