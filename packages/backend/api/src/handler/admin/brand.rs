use crate::{db::product::brand::ProductBrandDb, prelude::*, utils::req_to_json};

pub async fn handle_admin_list_brands(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AdminListBrandsResponse> {
    let req: AdminListBrandsRequest = req_to_json(req).await?;

    let page = req.page.max(1);
    let per_page = req.per_page.clamp(1, 100);
    let search = req.search.as_deref().filter(|s| !s.is_empty());
    let total = ProductBrandDb::count(ctx, search).await?;
    let rows = ProductBrandDb::load_page(ctx, page, per_page, search).await?;

    let brands = rows
        .into_iter()
        .map(|row| AdminBrandSummary {
            id: row.id,
            name: row.name,
        })
        .collect();

    Ok(AdminListBrandsResponse {
        page,
        per_page,
        total,
        brands,
    })
}

pub async fn handle_admin_create_brand(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AdminCreateBrandResponse> {
    let req: AdminCreateBrandRequest = req_to_json(req).await?;

    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::Validation("brand name required".to_string()));
    }

    ProductBrandDb::insert(ctx, &req.id, &name).await?;

    let row = ProductBrandDb::load_by_id(ctx, &req.id).await?;

    Ok(AdminCreateBrandResponse {
        brand: AdminBrandSummary {
            id: row.id,
            name: row.name,
        },
    })
}

pub async fn handle_admin_update_brand(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AdminUpdateBrandResponse> {
    let req: AdminUpdateBrandRequest = req_to_json(req).await?;

    if let Some(ref name) = req.name {
        let name = name.trim();
        if name.is_empty() {
            return Err(ApiError::Validation("brand name required".to_string()));
        }
        ProductBrandDb::update(ctx, &req.id, name).await?;
    }

    let row = ProductBrandDb::load_by_id(ctx, &req.id).await?;

    Ok(AdminUpdateBrandResponse {
        brand: AdminBrandSummary {
            id: row.id,
            name: row.name,
        },
    })
}

pub async fn handle_admin_delete_brand(ctx: &mut ApiContext, req: HttpRequest) -> ApiResult<()> {
    let req: AdminDeleteBrandRequest = req_to_json(req).await?;
    ProductBrandDb::delete(ctx, &req.id).await
}
