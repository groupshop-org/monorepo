use crate::{db::product::category::ProductCategoryDb, prelude::*, utils::req_to_json};

pub async fn handle_admin_list_categories(
    ctx: &mut ApiContext,
    _req: HttpRequest,
) -> ApiResult<AdminListCategoriesResponse> {
    let rows = ProductCategoryDb::load_all(ctx).await?;

    let categories = rows
        .into_iter()
        .map(|row| AdminCategorySummary {
            id: row.id,
            name: row.name,
            parent_id: row.parent_id,
            depth: row.depth,
        })
        .collect();

    Ok(AdminListCategoriesResponse { categories })
}

pub async fn handle_admin_create_category(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AdminCreateCategoryResponse> {
    let req: AdminCreateCategoryRequest = req_to_json(req).await?;

    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::Validation("category name required".to_string()));
    }

    let depth = if let Some(ref parent_id) = req.parent_id {
        let parent = ProductCategoryDb::load_by_id(ctx, parent_id).await?;
        parent.depth + 1
    } else {
        0
    };

    ProductCategoryDb::insert(ctx, &req.id, &name, req.parent_id.as_ref(), depth).await?;

    let row = ProductCategoryDb::load_by_id(ctx, &req.id).await?;

    Ok(AdminCreateCategoryResponse {
        category: AdminCategorySummary {
            id: row.id,
            name: row.name,
            parent_id: row.parent_id,
            depth: row.depth,
        },
    })
}

pub async fn handle_admin_update_category(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AdminUpdateCategoryResponse> {
    let req: AdminUpdateCategoryRequest = req_to_json(req).await?;

    let name = req.name.as_deref().map(|n| n.trim());
    if let Some(n) = name {
        if n.is_empty() {
            return Err(ApiError::Validation("category name required".to_string()));
        }
    }

    let depth = if let Some(ref parent_id_opt) = req.parent_id {
        if let Some(parent_id) = parent_id_opt {
            let parent = ProductCategoryDb::load_by_id(ctx, parent_id).await?;
            Some(parent.depth + 1)
        } else {
            Some(0)
        }
    } else {
        None
    };

    ProductCategoryDb::update(
        ctx,
        &req.id,
        name,
        req.parent_id.as_ref().map(|o| o.as_ref()),
        depth,
    )
    .await?;

    let row = ProductCategoryDb::load_by_id(ctx, &req.id).await?;

    Ok(AdminUpdateCategoryResponse {
        category: AdminCategorySummary {
            id: row.id,
            name: row.name,
            parent_id: row.parent_id,
            depth: row.depth,
        },
    })
}

pub async fn handle_admin_delete_category(ctx: &mut ApiContext, req: HttpRequest) -> ApiResult<()> {
    let req: AdminDeleteCategoryRequest = req_to_json(req).await?;
    ProductCategoryDb::delete(ctx, &req.id).await
}
