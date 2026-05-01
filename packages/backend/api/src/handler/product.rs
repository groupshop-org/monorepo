use std::collections::HashMap;

use crate::{
    db::{
        account::participation::UserParticipationDb,
        product::{brand::ProductBrandDb, catalog::ProductCatalogDb, category::ProductCategoryDb},
    },
    prelude::*,
    utils::req_to_json,
};

pub async fn handle_product_list(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<ProductListResponse> {
    let req: ProductListRequest = req_to_json(req).await?;

    let page = req.page.max(1);
    let per_page = req.per_page.clamp(1, 100);
    let search = req.search.as_deref().filter(|s| !s.is_empty());

    let total = ProductCatalogDb::count(
        ctx,
        req.category_id.as_ref(),
        req.brand_id.as_ref(),
        search,
        true,
        req.with_participants_only,
    )
    .await?;

    let rows = ProductCatalogDb::load_page(
        ctx,
        page,
        per_page,
        req.category_id.as_ref(),
        req.brand_id.as_ref(),
        search,
        true,
        req.with_participants_only,
    )
    .await?;

    // Hydrate `committed_units` for the *active* batch of each product
    // in a single round-trip rather than N. Numbers are derived from
    // D1 (mirror of the on-chain `Pool.total_quantity` maintained by
    // the deposit-confirm endpoint) so no Solana RPC calls are required
    // for paginated browsing — important for hosted RPCs that throttle.
    let ids: Vec<&ProductId> = rows.iter().map(|row| &row.id).collect();
    let units = UserParticipationDb::units_for_active_batches(ctx, &ids).await?;
    let unit_map: HashMap<ProductId, u64> = units.into_iter().collect();

    let products = rows
        .into_iter()
        .map(|row| {
            let committed_units = unit_map.get(&row.id).copied().unwrap_or(0);
            row_to_summary(row, committed_units)
        })
        .collect();

    Ok(ProductListResponse {
        page,
        per_page,
        total,
        products,
    })
}

pub async fn handle_product_detail(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<ProductDetailResponse> {
    let req: ProductDetailRequest = req_to_json(req).await?;

    let row = ProductCatalogDb::load_by_id(ctx, &req.id).await?;

    if !row.is_active {
        return Err(ApiError::Validation("product not found".to_string()));
    }

    // Detail page wants the committed-units total for the *active*
    // batch. Look up that batch in D1; if no batch row exists yet the
    // product has never had a deposit, so committed_units is 0.
    let committed_units =
        match crate::db::product::batch::ProductBatchDb::load_active(ctx, &row.id).await? {
            Some(batch) if batch.pipeline_status == "open" => {
                UserParticipationDb::units_for_batch(ctx, &row.id, batch.batch_id).await?
            }
            _ => 0,
        };

    Ok(ProductDetailResponse {
        product: row_to_summary(row, committed_units),
    })
}

pub async fn handle_product_categories(
    ctx: &mut ApiContext,
    _req: HttpRequest,
) -> ApiResult<ProductCategoriesResponse> {
    let rows = ProductCategoryDb::load_all(ctx).await?;

    let categories = rows
        .into_iter()
        .map(|row| ProductCategorySummary {
            id: row.id,
            name: row.name,
            parent_id: row.parent_id,
            depth: row.depth,
        })
        .collect();

    Ok(ProductCategoriesResponse { categories })
}

pub async fn handle_product_brands(
    ctx: &mut ApiContext,
    _req: HttpRequest,
) -> ApiResult<ProductBrandsResponse> {
    let rows = ProductBrandDb::load_all(ctx).await?;

    let brands = rows
        .into_iter()
        .map(|row| ProductBrandSummary {
            id: row.id,
            name: row.name,
        })
        .collect();

    Ok(ProductBrandsResponse { brands })
}

fn row_to_summary(row: ProductCatalogDb, committed_units: u64) -> ProductSummary {
    ProductSummary {
        id: row.id,
        gtin: row.gtin,
        name: row.name,
        category_id: row.category_id,
        brand_id: row.brand_id,
        price_cents: row.price_cents,
        currency: row.currency,
        minimum_order_quantity: row.minimum_order_quantity,
        inventory: row.inventory,
        is_preorder: row.is_preorder,
        estimated_delivery_weeks: row.estimated_delivery_weeks,
        image_url: row.image_url,
        is_active: row.is_active,
        committed_units,
    }
}
