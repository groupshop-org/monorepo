use crate::{db::product::catalog::ProductCatalogDb, prelude::*, utils::req_to_json};

pub async fn handle_admin_list_products(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AdminListProductsResponse> {
    let req: AdminListProductsRequest = req_to_json(req).await?;

    let page = req.page.max(1);
    let per_page = req.per_page.clamp(1, 100);
    let search = req.search.as_deref().filter(|s| !s.is_empty());

    let total = ProductCatalogDb::count(
        ctx,
        req.category_id.as_ref(),
        req.brand_id.as_ref(),
        search,
        false,
        false,
    )
    .await?;

    let rows = ProductCatalogDb::load_page(
        ctx,
        page,
        per_page,
        req.category_id.as_ref(),
        req.brand_id.as_ref(),
        search,
        false,
        false,
    )
    .await?;

    let products = rows.into_iter().map(row_to_admin_summary).collect();

    Ok(AdminListProductsResponse {
        page,
        per_page,
        total,
        products,
    })
}

pub async fn handle_admin_create_product(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AdminCreateProductResponse> {
    let req: AdminCreateProductRequest = req_to_json(req).await?;

    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::Validation("product name required".to_string()));
    }

    ProductCatalogDb::insert(
        ctx,
        &req.id,
        &req.gtin,
        &name,
        &req.category_id,
        &req.brand_id,
        req.price_cents,
        &req.currency,
        req.minimum_order_quantity,
        req.inventory,
        req.is_preorder,
        req.estimated_delivery_weeks,
        &req.supplier_url,
        &req.image_url,
    )
    .await?;

    let row = ProductCatalogDb::load_by_id(ctx, &req.id).await?;

    Ok(AdminCreateProductResponse {
        product: row_to_admin_summary(row),
    })
}

pub async fn handle_admin_update_product(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AdminUpdateProductResponse> {
    let req: AdminUpdateProductRequest = req_to_json(req).await?;

    if let Some(ref name) = req.name {
        if name.trim().is_empty() {
            return Err(ApiError::Validation("product name required".to_string()));
        }
    }

    ProductCatalogDb::update(
        ctx,
        &req.id,
        req.name.as_deref(),
        req.price_cents,
        req.currency.as_deref(),
        req.minimum_order_quantity,
        req.inventory,
        req.is_preorder,
        req.estimated_delivery_weeks,
        req.supplier_url.as_deref(),
        req.image_url.as_deref(),
        req.is_active,
    )
    .await?;

    let row = ProductCatalogDb::load_by_id(ctx, &req.id).await?;

    Ok(AdminUpdateProductResponse {
        product: row_to_admin_summary(row),
    })
}

pub async fn handle_admin_delete_product(ctx: &mut ApiContext, req: HttpRequest) -> ApiResult<()> {
    let req: AdminDeleteProductRequest = req_to_json(req).await?;
    ProductCatalogDb::delete(ctx, &req.id).await
}

pub async fn handle_admin_wipe_catalog(ctx: &mut ApiContext) -> ApiResult<()> {
    ProductCatalogDb::delete_all(ctx).await
}

fn row_to_admin_summary(row: ProductCatalogDb) -> AdminProductSummary {
    AdminProductSummary {
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
        supplier_url: row.supplier_url,
        image_url: row.image_url,
        is_active: row.is_active,
        created_at: row.created_at,
    }
}
