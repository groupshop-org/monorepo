use std::collections::HashMap;

use crate::{
    db::{
        account::participation::UserParticipationDb,
        product::{batch::ProductBatchDb, catalog::ProductCatalogDb},
    },
    prelude::*,
    utils::req_to_json,
};

/// Returns every order the signed-in user has placed (one row per
/// `(product_id, batch_id)`), enriched with the per-batch participant
/// count and our internal pipeline status. The frontend uses
/// `pipeline_status` to bucket into Current / History.
///
/// Everything is read from D1 — no Solana RPC calls — so this scales to
/// users with many orders without burning RPC credits. The cron job is
/// what keeps `product_batch.pipeline_status` in sync with the on-chain
/// Pool.
pub async fn handle_account_orders(
    ctx: &mut ApiContext,
    _req: HttpRequest,
) -> ApiResult<AccountOrdersResponse> {
    let uid = ctx.unchecked_uid().clone();
    let participations = UserParticipationDb::list_by_uid(ctx, &uid).await?;

    if participations.is_empty() {
        return Ok(AccountOrdersResponse { orders: Vec::new() });
    }

    // Hydrate product info once per distinct product.
    let mut product_map: HashMap<ProductId, ProductCatalogDb> = HashMap::new();
    for p in &participations {
        if !product_map.contains_key(&p.product_id) {
            let product = ProductCatalogDb::load_by_id(ctx, &p.product_id).await?;
            product_map.insert(p.product_id.clone(), product);
        }
    }

    // Per-batch unit totals: query each `(product_id, batch_id)`
    // individually. N is bounded by how many distinct batches the user
    // has joined, which stays small.
    let mut unit_map: HashMap<(ProductId, u32), u64> = HashMap::new();
    let mut pipeline_map: HashMap<(ProductId, u32), BatchPipelineStatus> = HashMap::new();
    for p in &participations {
        let key = (p.product_id.clone(), p.batch_id);
        if !unit_map.contains_key(&key) {
            let units =
                UserParticipationDb::units_for_batch(ctx, &p.product_id, p.batch_id).await?;
            unit_map.insert(key.clone(), units);
        }
        if !pipeline_map.contains_key(&key) {
            let status = ProductBatchDb::load(ctx, &p.product_id, p.batch_id)
                .await?
                .map(|row| BatchPipelineStatus::from_str(&row.pipeline_status))
                .unwrap_or(BatchPipelineStatus::Open);
            pipeline_map.insert(key, status);
        }
    }

    let orders = participations
        .into_iter()
        .map(|p| {
            let product = product_map
                .get(&p.product_id)
                .expect("product was just loaded");
            let key = (p.product_id.clone(), p.batch_id);
            AccountOrderSummary {
                product_id: p.product_id.clone(),
                product_name: product.name.clone(),
                product_image_url: product.image_url.clone(),
                price_cents: product.price_cents,
                minimum_order_quantity: product.minimum_order_quantity,
                batch_id: p.batch_id,
                product_amount_base_units: p.product_amount_base_units,
                shipping_amount_base_units: p.shipping_amount_base_units,
                last_tx_signature: p.last_tx_signature,
                first_deposited_at: p.first_deposited_at,
                last_deposited_at: p.last_deposited_at,
                quantity: p.quantity,
                refunded: p.refunded,
                committed_units: unit_map.get(&key).copied().unwrap_or(0),
                pipeline_status: pipeline_map
                    .get(&key)
                    .copied()
                    .unwrap_or(BatchPipelineStatus::Open),
            }
        })
        .collect();

    Ok(AccountOrdersResponse { orders })
}

/// Per-product cart-pending probe. Returns the user's active
/// participation for a single product, scoped to the *active* batch
/// (the one the deposit-build flow would target right now). `None` for
/// anonymous-side calls is impossible — auth is enforced by the route
/// definition — but `None` for "no active batch" or "no participation
/// yet" is the common case and the frontend just hides the cart
/// banner.
pub async fn handle_account_order_status(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AccountOrderStatusResponse> {
    let req: AccountOrderStatusRequest = req_to_json(req).await?;
    let uid = ctx.unchecked_uid().clone();

    // No active batch means no possible cart-pending state. (E.g. the
    // product has never had a deposit, or its previous batch already
    // released and the cron hasn't opened the next one yet.)
    let Some(active_batch) = ProductBatchDb::load_active(ctx, &req.product_id).await? else {
        return Ok(AccountOrderStatusResponse { active_order: None });
    };

    let Some(participation) =
        UserParticipationDb::load_for_batch(ctx, &uid, &req.product_id, active_batch.batch_id)
            .await?
    else {
        return Ok(AccountOrderStatusResponse { active_order: None });
    };

    if participation.refunded {
        // Refunded participations don't count as cart-pending. The
        // frontend treats `None` as "go ahead and place an order"; a
        // refunded row in History is not blocking.
        return Ok(AccountOrderStatusResponse { active_order: None });
    }

    Ok(AccountOrderStatusResponse {
        active_order: Some(AccountOrderActive {
            batch_id: active_batch.batch_id,
            quantity: participation.quantity,
            product_amount_base_units: participation.product_amount_base_units,
            pipeline_status: BatchPipelineStatus::from_str(&active_batch.pipeline_status),
        }),
    })
}
