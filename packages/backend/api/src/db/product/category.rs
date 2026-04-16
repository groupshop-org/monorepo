use wasm_bindgen::prelude::*;

use crate::db::tables::SQL_TABLE_PRODUCT_CATEGORY;
use crate::{
    prelude::*,
    utils::{db_execute, db_load, db_load_all, db_prepare, get_d1},
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProductCategoryDb {
    pub id: ProductCategoryId,
    pub name: String,
    pub parent_id: Option<ProductCategoryId>,
    pub depth: u32,
    pub created_at: String,
    pub updated_at: String,
}

const COLUMNS: &str = "id, name, parent_id, depth, created_at, updated_at";

impl ProductCategoryDb {
    pub async fn load_by_id(ctx: &ApiContext, id: &ProductCategoryId) -> ApiResult<Self> {
        db_load(
            db_prepare(
                &get_d1(&ctx.env)?,
                format!("SELECT {COLUMNS} FROM {SQL_TABLE_PRODUCT_CATEGORY} WHERE id = ?1"),
                &[JsValue::from_str(id.as_str())],
            )?,
            "category not found",
        )
        .await
    }

    pub async fn load_all(ctx: &ApiContext) -> ApiResult<Vec<Self>> {
        db_load_all(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "SELECT {COLUMNS} FROM {SQL_TABLE_PRODUCT_CATEGORY} ORDER BY depth ASC, name ASC"
            ),
            &[],
        )?)
        .await
    }

    pub async fn load_children(
        ctx: &ApiContext,
        parent_id: &ProductCategoryId,
    ) -> ApiResult<Vec<Self>> {
        db_load_all(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "SELECT {COLUMNS} FROM {SQL_TABLE_PRODUCT_CATEGORY} WHERE parent_id = ?1 ORDER BY name ASC"
            ),
            &[JsValue::from_str(parent_id.as_str())],
        )?)
        .await
    }

    pub async fn count(ctx: &ApiContext) -> ApiResult<u32> {
        #[derive(serde::Deserialize)]
        struct CountRow {
            n: f64,
        }

        let row: CountRow = db_load(
            db_prepare(
                &get_d1(&ctx.env)?,
                format!("SELECT count(*) AS n FROM {SQL_TABLE_PRODUCT_CATEGORY}"),
                &[],
            )?,
            "count query failed",
        )
        .await?;

        Ok(row.n as u32)
    }

    pub async fn insert(
        ctx: &ApiContext,
        id: &ProductCategoryId,
        name: &str,
        parent_id: Option<&ProductCategoryId>,
        depth: u32,
    ) -> ApiResult<()> {
        db_execute(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "INSERT INTO {SQL_TABLE_PRODUCT_CATEGORY} (id, name, parent_id, depth) VALUES (?1, ?2, ?3, ?4)"
            ),
            &[
                JsValue::from_str(id.as_str()),
                JsValue::from_str(name),
                match parent_id {
                    Some(pid) => JsValue::from_str(pid.as_str()),
                    None => JsValue::NULL,
                },
                JsValue::from_f64(depth as f64),
            ],
        )?)
        .await
    }

    pub async fn update(
        ctx: &ApiContext,
        id: &ProductCategoryId,
        name: Option<&str>,
        parent_id: Option<Option<&ProductCategoryId>>,
        depth: Option<u32>,
    ) -> ApiResult<()> {
        let mut sets = Vec::new();
        let mut bindings: Vec<JsValue> = Vec::new();
        let mut idx = 1u32;

        if let Some(name) = name {
            sets.push(format!("name = ?{idx}"));
            bindings.push(JsValue::from_str(name));
            idx += 1;
        }
        if let Some(parent_id) = parent_id {
            sets.push(format!("parent_id = ?{idx}"));
            bindings.push(match parent_id {
                Some(pid) => JsValue::from_str(pid.as_str()),
                None => JsValue::NULL,
            });
            idx += 1;
        }
        if let Some(depth) = depth {
            sets.push(format!("depth = ?{idx}"));
            bindings.push(JsValue::from_f64(depth as f64));
            idx += 1;
        }

        if sets.is_empty() {
            return Ok(());
        }

        sets.push(format!("updated_at = CURRENT_TIMESTAMP"));
        let set_clause = sets.join(", ");
        bindings.push(JsValue::from_str(id.as_str()));

        db_execute(db_prepare(
            &get_d1(&ctx.env)?,
            format!("UPDATE {SQL_TABLE_PRODUCT_CATEGORY} SET {set_clause} WHERE id = ?{idx}"),
            &bindings,
        )?)
        .await
    }

    pub async fn delete(ctx: &ApiContext, id: &ProductCategoryId) -> ApiResult<()> {
        db_execute(db_prepare(
            &get_d1(&ctx.env)?,
            format!("DELETE FROM {SQL_TABLE_PRODUCT_CATEGORY} WHERE id = ?1"),
            &[JsValue::from_str(id.as_str())],
        )?)
        .await
    }
}

pub fn is_name_unique_violation(db_message: &str) -> bool {
    let lower = db_message.to_ascii_lowercase();
    lower.contains("unique")
        && (lower.contains("product_category.name") || lower.contains("idx_product_category_name"))
}

pub fn is_id_unique_violation(db_message: &str) -> bool {
    let lower = db_message.to_ascii_lowercase();
    lower.contains("unique") && lower.contains("product_category.id")
}
