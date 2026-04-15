use crate::{
    db::{
        account::profile::UserAccountProfileDb,
        auth::{email::UserAuthEmailDb, role::UserAuthRoleDb},
    },
    prelude::*,
    utils::req_to_json,
};

pub async fn handle_profile(ctx: &mut ApiContext, _req: HttpRequest) -> ApiResult<AccountProfile> {
    let uid = ctx.unchecked_uid();
    let email = UserAuthEmailDb::load_by_user_id(ctx, uid).await?.email;
    let roles = UserAuthRoleDb::load_roles(ctx, uid).await?;
    let profile = UserAccountProfileDb::load_by_user_id(ctx, uid).await?;

    Ok(AccountProfile {
        uid: uid.clone(),
        email,
        roles,
        username: AccountUsername::unchecked_new(profile.username),
        full_name: profile.full_name,
        shipping_address: ShippingAddress {
            line1: profile.shipping_line1,
            line2: profile.shipping_line2,
            city: profile.shipping_city,
            state: profile.shipping_state,
            postal_code: profile.shipping_postal_code,
            country: profile.shipping_country,
        },
        receive_marketing: profile.receive_marketing,
    })
}

pub async fn handle_profile_update(ctx: &mut ApiContext, req: HttpRequest) -> ApiResult<()> {
    let uid = ctx.unchecked_uid().clone();
    let req: AccountProfileUpdateRequest = req_to_json(req).await?;
    UserAccountProfileDb::update_profile(ctx, &uid, &req).await
}
