use wasm_bindgen::prelude::*;
use worker::d1::{D1Database, D1PreparedStatement};

use crate::db::tables::SQL_TABLE_USER_AUTH_ROLE;
use crate::{
    prelude::*,
    utils::{db_batch, db_exists, db_prepare, get_d1},
};

pub struct UserAuthRoleDb;

impl UserAuthRoleDb {
    pub fn prepare_insert_roles(
        d1: &D1Database,
        id: &UserId,
        roles: &[UserRole],
    ) -> ApiResult<Vec<D1PreparedStatement>> {
        roles
            .iter()
            .map(|role| {
                db_prepare(
                    d1,
                    format!(
                        "INSERT OR IGNORE INTO {SQL_TABLE_USER_AUTH_ROLE} (user_id, role_id) VALUES (?1, ?2)"
                    ),
                    &[
                        JsValue::from_str(&id.to_string()),
                        JsValue::from_f64(f64::from(u8::from(*role))),
                    ],
                )
            })
            .collect()
    }

    pub async fn insert_roles(ctx: &ApiContext, id: &UserId, roles: &[UserRole]) -> ApiResult<()> {
        let d1 = get_d1(&ctx.env)?;
        db_batch(&d1, Self::prepare_insert_roles(&d1, id, roles)?).await
    }

    pub async fn replace_roles(ctx: &ApiContext, id: &UserId, roles: &[UserRole]) -> ApiResult<()> {
        let d1 = get_d1(&ctx.env)?;
        let mut stmts = vec![db_prepare(
            &d1,
            format!("DELETE FROM {SQL_TABLE_USER_AUTH_ROLE} WHERE user_id = ?1"),
            &[JsValue::from_str(&id.to_string())],
        )?];
        stmts.extend(Self::prepare_insert_roles(&d1, id, roles)?);
        db_batch(&d1, stmts).await
    }

    pub async fn load_roles(ctx: &ApiContext, id: &UserId) -> ApiResult<Vec<UserRole>> {
        db_prepare(
            &get_d1(&ctx.env)?,
            format!("SELECT role_id FROM {SQL_TABLE_USER_AUTH_ROLE} WHERE user_id = ?1"),
            &[JsValue::from_str(&id.to_string())],
        )?
        .raw::<u8>()
        .await
        .map_err(|err| ApiError::Db(err.to_string()))
        .map(|rows| {
            rows.into_iter()
                .filter_map(|row| row.into_iter().next())
                .map(UserRole::from)
                .collect()
        })
    }

    pub async fn check_roles(ctx: &ApiContext, id: &UserId, roles: &[UserRole]) -> ApiResult<()> {
        for role in roles {
            if !Self::check_role(ctx, id, *role).await? {
                return Err(AuthError::MissingRole(*role).into());
            }
        }
        Ok(())
    }

    pub async fn check_role(ctx: &ApiContext, id: &UserId, role: UserRole) -> ApiResult<bool> {
        db_exists(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "SELECT 1 AS n FROM {SQL_TABLE_USER_AUTH_ROLE} WHERE user_id = ?1 AND role_id = ?2 LIMIT 1"
            ),
            &[
                JsValue::from_str(&id.to_string()),
                JsValue::from_f64(f64::from(u8::from(role))),
            ],
        )?)
        .await
    }
}
