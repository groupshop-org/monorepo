use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

use crate::{
    db::auth::{email::UserAuthEmailDb, role::UserAuthRoleDb},
    durable::auth::once::AuthTokenOnceKind,
    handler::auth::{register::register_user, validation::Validation},
    notification::email::EmailNotification,
    prelude::*,
    utils::{random_bytes, req_to_json},
};

pub async fn handle_email_password_register(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<()> {
    let AuthEmailPasswordRegisterRequest {
        email,
        password,
        consent,
    } = req_to_json(req).await?;

    validate_email_password(&email, &password)?;
    let uid = register_user(ctx, &email, false, &password, &consent, None).await?;
    ctx.updated_tokens = Validation::sign_user_in(ctx, uid).await?;
    Ok(())
}

pub async fn handle_email_password_signin(ctx: &mut ApiContext, req: HttpRequest) -> ApiResult<()> {
    let AuthEmailPasswordSigninRequest { email, password } = req_to_json(req).await?;

    let record = match UserAuthEmailDb::try_load(ctx, &email).await? {
        Some(record) => record,
        None => {
            return Err(if ctx.config.is_default_admin_email(&email) {
                AuthError::InvalidCredentials.into()
            } else {
                AuthError::InvalidCredentials.into()
            })
        }
    };

    if !verify_password(&password, &record.password_hash)? {
        return Err(AuthError::InvalidCredentials.into());
    }

    if ctx.config.is_default_admin_email(&record.email) {
        UserAuthRoleDb::insert_roles(ctx, &record.user_id, &[UserRole::Admin]).await?;
    }

    ctx.updated_tokens = Validation::sign_user_in(ctx, record.user_id).await?;
    Ok(())
}

pub async fn handle_email_password_send_reset_any(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<()> {
    let AuthEmailPasswordSendResetAnyRequest { email } = req_to_json(req).await?;

    if let Some(record) = UserAuthEmailDb::try_load(ctx, &email).await? {
        EmailNotification::ResetPassword {
            uid: record.user_id,
        }
        .send(ctx)
        .await?;
    }

    Ok(())
}

pub async fn handle_email_password_send_reset_me(
    ctx: &mut ApiContext,
    _req: HttpRequest,
) -> ApiResult<()> {
    let uid = ctx.unchecked_uid().clone();
    EmailNotification::ResetPassword { uid }.send(ctx).await
}

pub async fn handle_email_address_send_verification(
    ctx: &mut ApiContext,
    _req: HttpRequest,
) -> ApiResult<()> {
    let uid = ctx.unchecked_uid().clone();
    EmailNotification::VerifyEmail { uid }.send(ctx).await
}

pub async fn handle_email_address_confirm_verification(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<()> {
    let AuthEmailAddressConfirmVerificationRequest { token } = req_to_json(req).await?;

    match Validation::once_token_consume(ctx, &token).await? {
        AuthTokenOnceKind::VerifyEmail { uid } => {
            UserAuthRoleDb::insert_roles(ctx, &uid, &[UserRole::EmailVerified]).await
        }
        _ => Err(AuthError::OnceTokenMismatch.into()),
    }
}

pub async fn handle_email_password_confirm_reset(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<()> {
    let AuthEmailPasswordConfirmResetRequest {
        token,
        new_password,
    } = req_to_json(req).await?;

    validate_password(&new_password)?;

    match Validation::once_token_consume(ctx, &token).await? {
        AuthTokenOnceKind::ResetPassword { uid } => {
            let password_hash = hash_password(&new_password)?;
            UserAuthEmailDb::update_password_hash(ctx, &uid, &password_hash).await?;
            ctx.updated_tokens = Validation::sign_user_in(ctx, uid).await?;
            Ok(())
        }
        _ => Err(AuthError::OnceTokenMismatch.into()),
    }
}

const MIN_PASSWORD_LENGTH: usize = 8;

pub fn validate_password(password: &str) -> ApiResult<()> {
    if password.len() < MIN_PASSWORD_LENGTH {
        Err(AuthError::PasswordTooShort.into())
    } else {
        Ok(())
    }
}

fn validate_email_password(email: &str, password: &str) -> ApiResult<()> {
    let email = email.trim();
    if email.is_empty() || !email.contains('@') || !email.contains('.') {
        return Err(AuthError::InvalidEmail.into());
    }
    validate_password(password)
}

pub fn hash_password(password: &str) -> ApiResult<String> {
    let salt = SaltString::encode_b64(&random_bytes::<20>())
        .map_err(|err| AuthError::Salt(err.to_string()))?;

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| ApiError::Auth(AuthError::Password(err.to_string())))
}

pub fn verify_password(password: &str, password_hash: &str) -> ApiResult<bool> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|err| ApiError::Auth(AuthError::Password(err.to_string())))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
