use crate::{
    db::{account::profile::UserAccountProfileDb, auth::role::UserAuthRoleDb},
    prelude::*,
    utils::req_to_json,
};

pub async fn handle_username_check(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AccountUsernameCheckResponse> {
    let req: AccountUsernameCheckRequest = req_to_json(req).await?;
    let valid = AccountUsername::validated(&req.username).is_ok();
    let available = if valid {
        let username = AccountUsername::validated(&req.username)?;
        !UserAccountProfileDb::username_exists(ctx, &username).await?
    } else {
        false
    };

    Ok(AccountUsernameCheckResponse { available, valid })
}

pub async fn handle_username_update(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AccountUsernameUpdateResponse> {
    let uid = ctx.unchecked_uid().clone();
    let req: AccountUsernameUpdateRequest = req_to_json(req).await?;
    let username = AccountUsername::validated(&req.username)?;

    if UserAccountProfileDb::username_exists(ctx, &username).await? {
        return Err(ApiError::Auth(AuthError::UsernameTaken));
    }

    UserAccountProfileDb::update_username(ctx, &uid, &username).await?;
    UserAuthRoleDb::insert_roles(ctx, &uid, &[UserRole::UsernameChosen]).await?;

    Ok(AccountUsernameUpdateResponse {})
}
