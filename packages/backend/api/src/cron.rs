//! Periodic backend chores driven by Cloudflare cron triggers.
//!
//! The marquee responsibility is **batch recycling**: when a deposit
//! pushes a pool's `participant_count` to its threshold, the on-chain
//! program auto-locks the pool. This cron observes those locked pools,
//! advances the D1 `pipeline_status` (Open → Locked → ShippingToDistributor),
//! and opens the next batch by initializing a fresh on-chain pool with
//! `batch_id + 1`. The next deposit on that product then targets the
//! new batch automatically.
//!
//! Triggered both by the scheduled-event handler in `lib.rs` (production)
//! and the `/internal/cron-tick` HTTP route (development & manual
//! recovery). The work is idempotent — running it twice in the same
//! second is safe.

use crate::{
    db::product::batch::ProductBatchDb,
    prelude::*,
    solana::{derive_pool_pda, ensure_pool_initialized, rpc_get_account_data, PoolAccount},
};

#[derive(Debug, Default, serde::Serialize)]
pub struct CronReport {
    pub batches_inspected: u32,
    pub batches_locked: u32,
    pub batches_opened: u32,
    pub errors: Vec<String>,
}

pub async fn run(ctx: &mut ApiContext) -> ApiResult<CronReport> {
    let mut report = CronReport::default();
    let open_batches = ProductBatchDb::list_open_for_cron(ctx).await?;
    for batch in open_batches {
        report.batches_inspected += 1;
        if let Err(err) = process_batch(ctx, &batch, &mut report).await {
            // Don't let one product's RPC hiccup take down the whole
            // sweep. Log the failure into the report so callers can
            // see what happened in dev.
            report.errors.push(format!(
                "{} batch {}: {err}",
                batch.product_id, batch.batch_id
            ));
        }
    }
    Ok(report)
}

/// Inspect a single open batch row. If the on-chain pool has auto-locked
/// (status flipped to Locked), advance the D1 pipeline and open the
/// successor batch.
async fn process_batch(
    ctx: &mut ApiContext,
    batch: &ProductBatchDb,
    report: &mut CronReport,
) -> ApiResult<()> {
    let (pool_pda, _bump) = derive_pool_pda(&ctx.config.solana, &batch.product_id, batch.batch_id)?;
    let Some(bytes) = rpc_get_account_data(ctx, &pool_pda).await? else {
        // Pool not on-chain yet — the deposit-build flow hasn't run for
        // this product. Nothing to do.
        return Ok(());
    };
    let pool = PoolAccount::from_bytes(&bytes)?;

    // Pool status enum: 0=Open 1=Locked 2=Released 3=Refunding. We only
    // act when on-chain Locked surfaces — that's what auto-lock at
    // threshold sets.
    if pool.status != 1 {
        return Ok(());
    }

    // Mark the current batch as ShippingToDistributor in D1. Per the
    // product brief, we instantly transition past Locked into shipping
    // — Locked-on-chain plus ShippingToDistributor in D1 is the
    // canonical "deal triggered, fulfillment running" state. Real-world
    // shipping advancement (`shipping_to_distributor` →
    // `shipping_individually` → `released`) is a separate admin path.
    ProductBatchDb::set_pipeline_status(
        ctx,
        &batch.product_id,
        batch.batch_id,
        "shipping_to_distributor",
    )
    .await?;
    report.batches_locked += 1;

    // Open the successor on-chain pool so subsequent deposits have a
    // place to land. We pick batch_id = max(current+1, max_known+1) so
    // any racing init still settles on a unique batch. Threshold is
    // taken from the just-locked batch — products keep the same MOQ
    // unless an admin updates the catalog.
    let next_batch_id = match ProductBatchDb::max_batch_id(ctx, &batch.product_id).await? {
        Some(max_known) => max_known.max(batch.batch_id).saturating_add(1),
        None => batch.batch_id.saturating_add(1),
    };
    ProductBatchDb::insert_if_absent(ctx, &batch.product_id, next_batch_id, batch.threshold)
        .await?;
    let opened =
        ensure_pool_initialized(ctx, &batch.product_id, next_batch_id, batch.threshold).await?;
    if opened {
        report.batches_opened += 1;
    }
    Ok(())
}
