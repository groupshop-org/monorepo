pub const USERNAME_MAX_GENERATION_ATTEMPTS: usize = 24;

use crate::{
    db::{
        account::profile::{is_username_unique_violation, UserAccountProfileDb},
        auth::{
            email::{is_email_unique_violation, UserAuthEmailDb},
            openid::UserAuthOpenIdDb,
            role::UserAuthRoleDb,
        },
    },
    handler::auth::email_password::hash_password,
    notification::email::EmailNotification,
    prelude::*,
    utils::{db_batch, get_d1, random_bytes},
};

pub(super) async fn register_user(
    ctx: &ApiContext,
    email: &str,
    email_verified: bool,
    password: &str,
    consent: &AuthRegistrationConsent,
    open_id: Option<(OpenIdProvider, String)>,
) -> ApiResult<UserId> {
    consent.validate()?;

    let password_hash = hash_password(password)?;
    let mut roles = vec![UserRole::PartialRegistration];

    if ctx.config.is_default_admin_email(email) {
        roles.push(UserRole::Admin);
    }
    if email_verified {
        roles.push(UserRole::EmailVerified);
    }

    let uid = UserId::from(random_bytes::<32>());
    let d1 = get_d1(&ctx.env)?;
    let mut inserted = false;

    for _ in 0..USERNAME_MAX_GENERATION_ATTEMPTS {
        let username = generate_default_username()?;

        let mut stmts = vec![
            UserAccountProfileDb::prepare_insert(
                &d1,
                &uid,
                &username,
                consent.opt_in_marketing_emails,
            )?,
            UserAuthEmailDb::prepare_insert(&d1, email, &password_hash, &uid)?,
        ];

        if let Some((provider, subject)) = open_id.as_ref() {
            stmts.push(UserAuthOpenIdDb::prepare_insert_or_update_email(
                &d1, provider, subject, email, &uid,
            )?);
        }

        stmts.extend(UserAuthRoleDb::prepare_insert_roles(&d1, &uid, &roles)?);

        match db_batch(&d1, stmts).await {
            Ok(()) => {
                inserted = true;
                break;
            }
            Err(ApiError::Db(msg)) if is_username_unique_violation(&msg) => continue,
            Err(ApiError::Db(msg)) if is_email_unique_violation(&msg) => {
                return Err(ApiError::Auth(AuthError::EmailAlreadyRegistered));
            }
            Err(err) => return Err(err),
        }
    }

    if !email_verified {
        EmailNotification::VerifyEmail { uid: uid.clone() }
            .send(ctx)
            .await?;
    }

    if inserted {
        Ok(uid)
    } else {
        Err(ApiError::Auth(AuthError::UsernameTaken))
    }
}

const RANDOM_SUFFIX_LEN: usize = 5;
const ADJECTIVES: &[&str] = &[
    "quiet", "brisk", "mossy", "lunar", "gentle", "witty", "amber", "silver", "rapid", "sunny",
    "candid", "nimble", "sturdy", "bold", "clever", "calm", "fresh", "bright", "plain", "keen",
];
const NOUNS: &[&str] = &[
    "otter", "lantern", "comet", "harbor", "cactus", "river", "signal", "meadow", "forest",
    "canyon", "falcon", "engine", "planet", "valley", "turtle", "shadow", "summit", "anchor",
];
const ALPHABET: &[u8] = b"0123456789abcdefghjkmnpqrstvwxyz";

fn generate_default_username() -> ApiResult<AccountUsername> {
    AccountUsername::validated(format!(
        "{}-{}-{}",
        pick(ADJECTIVES),
        pick(NOUNS),
        random_token(RANDOM_SUFFIX_LEN)
    ))
}

fn pick<'a>(list: &'a [&'a str]) -> &'a str {
    let bytes = random_bytes::<4>();
    let index = (u32::from_le_bytes(bytes) as usize) % list.len();
    list[index]
}

fn random_token(len: usize) -> String {
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        let byte = random_bytes::<1>()[0] as usize;
        out.push(ALPHABET[byte % ALPHABET.len()] as char);
    }
    out
}
