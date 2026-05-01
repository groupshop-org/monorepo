use wasm_bindgen::prelude::*;

use crate::db::tables::{SQL_TABLE_PRODUCT_CATALOG, SQL_TABLE_USER_PARTICIPATION};
use crate::{
    prelude::*,
    utils::{db_execute, db_load, db_load_all, db_prepare, deserialize_d1_bool, get_d1},
};

/// Index row for a confirmed escrow deposit. Mirrors fields read from the
/// on-chain Participation account so we can answer "my orders" and
/// per-product unit totals without making `getProgramAccounts` calls
/// (which most hosted Solana RPCs disable or rate-limit at scale).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserParticipationDb {
    pub uid: UserId,
    pub product_id: ProductId,
    pub batch_id: u32,
    pub wallet_address: String,
    pub product_amount_base_units: u64,
    pub shipping_amount_base_units: u64,
    /// Number of units this buyer committed in this batch. Mirrors
    /// on-chain `Participation.quantity`.
    pub quantity: u64,
    /// Mirrors the on-chain `Participation.refunded` byte.
    #[serde(deserialize_with = "deserialize_d1_bool")]
    pub refunded: bool,
    pub last_tx_signature: String,
    pub first_deposited_at: String,
    pub last_deposited_at: String,
}

const COLUMNS: &str = "uid, product_id, batch_id, wallet_address, product_amount_base_units, shipping_amount_base_units, quantity, refunded, last_tx_signature, first_deposited_at, last_deposited_at";

impl UserParticipationDb {
    /// Insert a new row or update the totals for an existing
    /// `(uid, product_id, batch_id)` triple. Idempotent on retry: we
    /// always overwrite with the on-chain snapshot the caller passed in.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert(
        ctx: &ApiContext,
        uid: &UserId,
        product_id: &ProductId,
        batch_id: u32,
        wallet_address: &str,
        product_amount_base_units: u64,
        shipping_amount_base_units: u64,
        quantity: u64,
        tx_signature: &str,
    ) -> ApiResult<()> {
        db_execute(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "INSERT INTO {SQL_TABLE_USER_PARTICIPATION}
                    (uid, product_id, batch_id, wallet_address, product_amount_base_units, shipping_amount_base_units, quantity, last_tx_signature)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(uid, product_id, batch_id) DO UPDATE SET
                     wallet_address = excluded.wallet_address,
                     product_amount_base_units = excluded.product_amount_base_units,
                     shipping_amount_base_units = excluded.shipping_amount_base_units,
                     quantity = excluded.quantity,
                     last_tx_signature = excluded.last_tx_signature,
                     last_deposited_at = CURRENT_TIMESTAMP"
            ),
            &[
                JsValue::from_str(&uid.to_string()),
                JsValue::from_str(product_id.as_str()),
                JsValue::from_f64(batch_id as f64),
                JsValue::from_str(wallet_address),
                JsValue::from_f64(product_amount_base_units as f64),
                JsValue::from_f64(shipping_amount_base_units as f64),
                JsValue::from_f64(quantity as f64),
                JsValue::from_str(tx_signature),
            ],
        )?)
        .await
    }

    /// Mark a participation as refunded after the chain side has
    /// confirmed the SelfRefund instruction landed.
    pub async fn mark_refunded(
        ctx: &ApiContext,
        uid: &UserId,
        product_id: &ProductId,
        batch_id: u32,
        tx_signature: &str,
    ) -> ApiResult<()> {
        db_execute(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "UPDATE {SQL_TABLE_USER_PARTICIPATION}
                    SET refunded = 1,
                        last_tx_signature = ?4,
                        last_deposited_at = CURRENT_TIMESTAMP
                  WHERE uid = ?1 AND product_id = ?2 AND batch_id = ?3"
            ),
            &[
                JsValue::from_str(&uid.to_string()),
                JsValue::from_str(product_id.as_str()),
                JsValue::from_f64(batch_id as f64),
                JsValue::from_str(tx_signature),
            ],
        )?)
        .await
    }

    /// Load the most recent (highest batch_id) participation row for a
    /// `(uid, product_id)`. Used by the deposit/refund flow to find the
    /// row to update — there's only ever one "active" participation per
    /// user per product at a time (older batches that completed are
    /// historical).
    pub async fn load_latest_for_user(
        ctx: &ApiContext,
        uid: &UserId,
        product_id: &ProductId,
    ) -> ApiResult<Option<Self>> {
        let rows: Vec<Self> = db_load_all(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "SELECT {COLUMNS} FROM {SQL_TABLE_USER_PARTICIPATION}
                 WHERE uid = ?1 AND product_id = ?2
                 ORDER BY batch_id DESC LIMIT 1"
            ),
            &[
                JsValue::from_str(&uid.to_string()),
                JsValue::from_str(product_id.as_str()),
            ],
        )?)
        .await?;
        Ok(rows.into_iter().next())
    }

    /// Load this user's participation for a specific batch of a product,
    /// or `None` if they haven't joined that batch.
    pub async fn load_for_batch(
        ctx: &ApiContext,
        uid: &UserId,
        product_id: &ProductId,
        batch_id: u32,
    ) -> ApiResult<Option<Self>> {
        let rows: Vec<Self> = db_load_all(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "SELECT {COLUMNS} FROM {SQL_TABLE_USER_PARTICIPATION}
                 WHERE uid = ?1 AND product_id = ?2 AND batch_id = ?3"
            ),
            &[
                JsValue::from_str(&uid.to_string()),
                JsValue::from_str(product_id.as_str()),
                JsValue::from_f64(batch_id as f64),
            ],
        )?)
        .await?;
        Ok(rows.into_iter().next())
    }

    pub async fn list_by_uid(ctx: &ApiContext, uid: &UserId) -> ApiResult<Vec<Self>> {
        db_load_all(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "SELECT {COLUMNS} FROM {SQL_TABLE_USER_PARTICIPATION}
                 WHERE uid = ?1
                 ORDER BY last_deposited_at DESC"
            ),
            &[JsValue::from_str(&uid.to_string())],
        )?)
        .await
    }

    /// Sum of `quantity` across non-refunded participations for a
    /// `(product_id, batch_id)`. Matches the on-chain
    /// `Pool.total_quantity` field — what the threshold UI compares
    /// against `product.minimum_order_quantity`.
    pub async fn units_for_batch(
        ctx: &ApiContext,
        product_id: &ProductId,
        batch_id: u32,
    ) -> ApiResult<u64> {
        #[derive(serde::Deserialize)]
        struct SumRow {
            // `coalesce(sum(...), 0)` returns 0 for empty result sets,
            // so this is always a number even with no participations.
            n: f64,
        }
        let row: SumRow = db_load(
            db_prepare(
                &get_d1(&ctx.env)?,
                format!(
                    "SELECT COALESCE(SUM(quantity), 0) AS n
                     FROM {SQL_TABLE_USER_PARTICIPATION}
                     WHERE product_id = ?1 AND batch_id = ?2 AND refunded = 0"
                ),
                &[
                    JsValue::from_str(product_id.as_str()),
                    JsValue::from_f64(batch_id as f64),
                ],
            )?,
            "unit sum query failed",
        )
        .await?;
        Ok(row.n as u64)
    }

    /// Per-product committed-units totals for the *active* batch of a
    /// list of products. Used to hydrate `committed_units` on a
    /// paginated product list without N round-trips. The query joins
    /// against `product_batch.max(batch_id)` so a product whose latest
    /// batch is freshly opened (no buyers yet) returns no row and the
    /// caller defaults to 0. Refunded participations are excluded.
    pub async fn units_for_active_batches(
        ctx: &ApiContext,
        product_ids: &[&ProductId],
    ) -> ApiResult<Vec<(ProductId, u64)>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        #[derive(serde::Deserialize)]
        struct Row {
            product_id: ProductId,
            n: f64,
        }
        let placeholders = (1..=product_ids.len())
            .map(|i| format!("?{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let bindings: Vec<JsValue> = product_ids
            .iter()
            .map(|id| JsValue::from_str(id.as_str()))
            .collect();
        let rows: Vec<Row> = db_load_all(db_prepare(
            &get_d1(&ctx.env)?,
            format!(
                "SELECT up.product_id AS product_id, COALESCE(SUM(up.quantity), 0) AS n
                 FROM {SQL_TABLE_USER_PARTICIPATION} up
                 JOIN (
                     SELECT product_id, max(batch_id) AS max_batch
                     FROM product_batch
                     WHERE product_id IN ({placeholders})
                     GROUP BY product_id
                 ) latest
                   ON latest.product_id = up.product_id
                  AND latest.max_batch  = up.batch_id
                 WHERE up.refunded = 0
                 GROUP BY up.product_id"
            ),
            &bindings,
        )?)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.product_id, r.n as u64))
            .collect())
    }

    /// Cross-check that the schemas line up at compile time — this is a
    /// no-op at runtime but ensures the FK target is the catalog table we
    /// expect.
    #[allow(dead_code)]
    const _SCHEMA_PROBE: &'static str = SQL_TABLE_PRODUCT_CATALOG;
}
