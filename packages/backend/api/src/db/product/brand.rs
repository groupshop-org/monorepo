use wasm_bindgen::prelude::*;

use crate::db::tables::SQL_TABLE_PRODUCT_BRAND;
use crate::{
    prelude::*,
    utils::{db_execute, db_load, db_load_all, db_prepare, get_d1},
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProductBrandDb {
    pub id: ProductBrandId,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

const COLUMNS: &str = "id, name, created_at, updated_at";

impl ProductBrandDb {
    pub async fn load_by_id(ctx: &ApiContext, id: &ProductBrandId) -> ApiResult<Self> {
        db_load(
            db_prepare(
                &get_d1(&ctx.env)?,
                format!("SELECT {COLUMNS} FROM {SQL_TABLE_PRODUCT_BRAND} WHERE id = ?1"),
                &[JsValue::from_str(id.as_str())],
            )?,
            "brand not found",
        )
        .await
    }

    pub async fn load_all(ctx: &ApiContext) -> ApiResult<Vec<Self>> {
        db_load_all(db_prepare(
            &get_d1(&ctx.env)?,
            format!("SELECT {COLUMNS} FROM {SQL_TABLE_PRODUCT_BRAND} ORDER BY name ASC"),
            &[],
        )?)
        .await
    }

    pub async fn load_page(
        ctx: &ApiContext,
        page: u32,
        per_page: u32,
        search: Option<&str>,
    ) -> ApiResult<Vec<Self>> {
        let offset = (page.saturating_sub(1)) * per_page;

        let (where_clause, bindings) = if let Some(search) = search {
            (
                "WHERE name LIKE ?1",
                vec![
                    JsValue::from_str(&format!("%{search}%")),
                    JsValue::from_f64(per_page as f64),
                    JsValue::from_f64(offset as f64),
                ],
            )
        } else {
            (
                "",
                vec![
                    JsValue::from_f64(per_page as f64),
                    JsValue::from_f64(offset as f64),
                ],
            )
        };

        let (limit_idx, offset_idx) = if search.is_some() {
            ("?2", "?3")
        } else {
            ("?1", "?2")
        };

        db_load_all(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "SELECT {COLUMNS} FROM {SQL_TABLE_PRODUCT_BRAND} {where_clause} ORDER BY name ASC LIMIT {limit_idx} OFFSET {offset_idx}"
            ),
            &bindings,
        )?)
        .await
    }

    pub async fn count(ctx: &ApiContext, search: Option<&str>) -> ApiResult<u32> {
        #[derive(serde::Deserialize)]
        struct CountRow {
            n: f64,
        }

        let (where_clause, bindings) = if let Some(search) = search {
            (
                "WHERE name LIKE ?1",
                vec![JsValue::from_str(&format!("%{search}%"))],
            )
        } else {
            ("", vec![])
        };

        let row: CountRow = db_load(
            db_prepare(
                &get_d1(&ctx.env)?,
                format!("SELECT count(*) AS n FROM {SQL_TABLE_PRODUCT_BRAND} {where_clause}"),
                &bindings,
            )?,
            "count query failed",
        )
        .await?;

        Ok(row.n as u32)
    }

    pub async fn insert(ctx: &ApiContext, id: &ProductBrandId, name: &str) -> ApiResult<()> {
        db_execute(db_prepare(
            &get_d1(&ctx.env)?,
            format!("INSERT INTO {SQL_TABLE_PRODUCT_BRAND} (id, name) VALUES (?1, ?2)"),
            &[JsValue::from_str(id.as_str()), JsValue::from_str(name)],
        )?)
        .await
    }

    pub async fn update(ctx: &ApiContext, id: &ProductBrandId, name: &str) -> ApiResult<()> {
        db_execute(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "UPDATE {SQL_TABLE_PRODUCT_BRAND} SET name = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2"
            ),
            &[
                JsValue::from_str(name),
                JsValue::from_str(id.as_str()),
            ],
        )?)
        .await
    }

    pub async fn delete(ctx: &ApiContext, id: &ProductBrandId) -> ApiResult<()> {
        db_execute(db_prepare(
            &get_d1(&ctx.env)?,
            format!("DELETE FROM {SQL_TABLE_PRODUCT_BRAND} WHERE id = ?1"),
            &[JsValue::from_str(id.as_str())],
        )?)
        .await
    }
}

pub fn is_name_unique_violation(db_message: &str) -> bool {
    let lower = db_message.to_ascii_lowercase();
    lower.contains("unique")
        && (lower.contains("product_brand.name") || lower.contains("idx_product_brand_name"))
}

pub fn is_id_unique_violation(db_message: &str) -> bool {
    let lower = db_message.to_ascii_lowercase();
    lower.contains("unique") && lower.contains("product_brand.id")
}
