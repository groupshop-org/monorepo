use crate::{
    db::{admin::user::AdminUserRowDb, auth::role::UserAuthRoleDb},
    prelude::*,
    utils::req_to_json,
};

pub async fn handle_list_users(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AdminListUsersResponse> {
    let req: AdminListUsersRequest = req_to_json(req).await?;

    let page = req.page.max(1);
    let per_page = req.per_page.clamp(1, 100);
    let total_users = AdminUserRowDb::count(ctx).await?;
    let rows = AdminUserRowDb::load_page(ctx, page, per_page).await?;
    let user_ids = rows.iter().map(|row| row.id.clone()).collect::<Vec<_>>();
    let roles_by_user_id = AdminUserRowDb::load_roles_by_user_id(ctx, &user_ids).await?;

    let users = rows
        .into_iter()
        .map(|row| AdminUserSummary {
            roles: roles_by_user_id
                .get(&row.id.to_string())
                .cloned()
                .unwrap_or_default(),
            id: row.id,
            username: row.username,
            email: row.email,
            created_at: row.created_at,
        })
        .collect();

    Ok(AdminListUsersResponse {
        page,
        per_page,
        total_users,
        users,
    })
}

pub async fn handle_delete_user(ctx: &mut ApiContext, req: HttpRequest) -> ApiResult<()> {
    let req: AdminDeleteUserRequest = req_to_json(req).await?;
    AdminUserRowDb::delete_all_user_data(ctx, &req.id).await
}

pub async fn handle_update_user(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AdminUpdateUserResponse> {
    let req: AdminUpdateUserRequest = req_to_json(req).await?;
    UserAuthRoleDb::replace_roles(ctx, &req.id, &req.roles).await?;

    let row = AdminUserRowDb::load_by_user_id(ctx, &req.id).await?;
    let roles = UserAuthRoleDb::load_roles(ctx, &req.id).await?;

    Ok(AdminUpdateUserResponse {
        user: AdminUserSummary {
            id: row.id,
            username: row.username,
            email: row.email,
            roles,
            created_at: row.created_at,
        },
    })
}
