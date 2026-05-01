use wasm_bindgen::prelude::*;

use crate::db::tables::SQL_TABLE_PRODUCT_BATCH;
use crate::{
    prelude::*,
    utils::{db_execute, db_load, db_load_all, db_prepare, get_d1},
};

/// One row per `(product_id, batch_id)` — mirrors the on-chain Pool keyed
/// by the same pair. New rows are inserted by:
///   1. The deposit-build handler when no batch exists yet (first deposit
///      on a product creates batch 0).
///   2. The cron job that opens the next batch after the previous one
///      auto-locks at threshold.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProductBatchDb {
    pub product_id: ProductId,
    pub batch_id: u32,
    pub threshold: u32,
    pub pipeline_status: String,
    pub opened_at: String,
    pub locked_at: Option<String>,
    pub closed_at: Option<String>,
}

const COLUMNS: &str =
    "product_id, batch_id, threshold, pipeline_status, opened_at, locked_at, closed_at";

impl ProductBatchDb {
    /// Look up the active (non-terminal) batch for a product. Returns the
    /// row with the highest batch_id whose pipeline_status hasn't reached
    /// a final state. Used by deposit-intent to know which Pool the next
    /// deposit should target.
    pub async fn load_active(ctx: &ApiContext, product_id: &ProductId) -> ApiResult<Option<Self>> {
        // "Active" = anything not yet released or fully refunded. We
        // include locked/shipping states because those still represent
        // the buyer's commitment for that batch even though new deposits
        // shouldn't go there.
        let rows: Vec<Self> = db_load_all(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "SELECT {COLUMNS} FROM {SQL_TABLE_PRODUCT_BATCH}
                 WHERE product_id = ?1
                 ORDER BY batch_id DESC
                 LIMIT 1"
            ),
            &[JsValue::from_str(product_id.as_str())],
        )?)
        .await?;
        Ok(rows.into_iter().next())
    }

    pub async fn load(
        ctx: &ApiContext,
        product_id: &ProductId,
        batch_id: u32,
    ) -> ApiResult<Option<Self>> {
        let rows: Vec<Self> = db_load_all(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "SELECT {COLUMNS} FROM {SQL_TABLE_PRODUCT_BATCH}
                 WHERE product_id = ?1 AND batch_id = ?2"
            ),
            &[
                JsValue::from_str(product_id.as_str()),
                JsValue::from_f64(batch_id as f64),
            ],
        )?)
        .await?;
        Ok(rows.into_iter().next())
    }

    /// Insert a new batch row for a product. Idempotent on `(product_id,
    /// batch_id)` so retried calls (e.g. cron + deposit-build racing)
    /// don't duplicate or fail.
    pub async fn insert_if_absent(
        ctx: &ApiContext,
        product_id: &ProductId,
        batch_id: u32,
        threshold: u32,
    ) -> ApiResult<()> {
        db_execute(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "INSERT OR IGNORE INTO {SQL_TABLE_PRODUCT_BATCH}
                    (product_id, batch_id, threshold, pipeline_status)
                 VALUES (?1, ?2, ?3, 'open')"
            ),
            &[
                JsValue::from_str(product_id.as_str()),
                JsValue::from_f64(batch_id as f64),
                JsValue::from_f64(threshold as f64),
            ],
        )?)
        .await
    }

    /// Set the pipeline status for an existing batch. Caller is responsible
    /// for any cross-state validation (e.g. don't go from `released` back
    /// to `open`); we keep the DB write simple and let the application
    /// layer enforce ordering.
    pub async fn set_pipeline_status(
        ctx: &ApiContext,
        product_id: &ProductId,
        batch_id: u32,
        pipeline_status: &str,
    ) -> ApiResult<()> {
        // Maintain `locked_at` automatically when crossing the locked
        // threshold so downstream views can show "locked X minutes ago"
        // without a second field write.
        db_execute(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "UPDATE {SQL_TABLE_PRODUCT_BATCH}
                    SET pipeline_status = ?3,
                        locked_at = CASE
                            WHEN locked_at IS NULL AND ?3 != 'open' THEN CURRENT_TIMESTAMP
                            ELSE locked_at
                        END
                  WHERE product_id = ?1 AND batch_id = ?2"
            ),
            &[
                JsValue::from_str(product_id.as_str()),
                JsValue::from_f64(batch_id as f64),
                JsValue::from_str(pipeline_status),
            ],
        )?)
        .await
    }

    /// Cron helper: returns every batch whose pipeline_status is `open`,
    /// across all products. The cron then reads each on-chain pool to
    /// see whether it has auto-locked, and if so, advances the row + opens
    /// the successor batch.
    pub async fn list_open_for_cron(ctx: &ApiContext) -> ApiResult<Vec<Self>> {
        db_load_all(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "SELECT {COLUMNS} FROM {SQL_TABLE_PRODUCT_BATCH}
                 WHERE pipeline_status = 'open'"
            ),
            &[],
        )?)
        .await
    }

    /// Returns the highest batch_id we know about for a product (or
    /// `None` if no batches exist yet). Used to choose `next_batch_id`
    /// when opening the successor.
    pub async fn max_batch_id(ctx: &ApiContext, product_id: &ProductId) -> ApiResult<Option<u32>> {
        #[derive(serde::Deserialize)]
        struct Row {
            max_batch: Option<f64>,
        }
        let row: Row = db_load(
            db_prepare(
                &get_d1(&ctx.env)?,
                format!(
                    "SELECT max(batch_id) AS max_batch FROM {SQL_TABLE_PRODUCT_BATCH} WHERE product_id = ?1"
                ),
                &[JsValue::from_str(product_id.as_str())],
            )?,
            "max batch id query failed",
        )
        .await?;
        Ok(row.max_batch.map(|v| v as u32))
    }
}
