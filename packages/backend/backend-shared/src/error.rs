use thiserror::Error;

use crate::route::UserRole;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Error)]
pub enum ApiError {
    #[error("base64 decode: {0}")]
    Base64Decode(String),

    #[error("database: {0}")]
    Db(String),

    #[error("missing body: {0}")]
    MissingBody(String),

    #[error("parse body: {0}")]
    ParseBody(String),

    #[error("stream creation: {0}")]
    StreamCreation(String),

    #[error("crypto: {0}")]
    Crypto(String),

    #[error("unknown open-id provider: {0}")]
    UnknownOpenIdProvider(String),

    #[error("unknown route: {0}")]
    UnknownRoute(String),

    #[error("unimplemented: {0}")]
    Unimplemented(String),

    #[error("invalid header value: {0}")]
    InvalidHeaderValue(String),

    #[error("auth: {0}")]
    Auth(#[from] AuthError),

    #[error("durable object: {0}")]
    DurableObject(#[from] DurableObjectError),

    #[error("signing failure: {0}")]
    SigningFailure(String),

    #[error("request failure: {0}")]
    Request(String),

    #[error("validation: {0}")]
    Validation(String),

    #[error("api returned status {status}: {body}")]
    HttpStatus { status: u16, body: String },

    #[error("{0}")]
    Unknown(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Error)]
pub enum DurableObjectError {
    #[error("namespace: {0}")]
    Namespace(String),

    #[error("request builder: {0}")]
    RequestBuilder(String),

    #[error("fetch handler: {0}")]
    Fetch(String),

    #[error("storage get: {0}")]
    StorageGet(String),

    #[error("storage put: {0}")]
    StoragePut(String),

    #[error("storage delete: {0}")]
    StorageDelete(String),

    #[error("failed to get stub for {object_id}: {err}")]
    FailedStub { object_id: String, err: String },

    #[error("status code: {status}, message: {message:?}")]
    Status {
        status: u16,
        message: Option<String>,
    },

    #[error("encode response: {0}")]
    EncodeResponse(String),

    #[error("failed to set alarm: {0}")]
    Alarm(String),

    #[error("failed to get unique id: {0}")]
    UniqueId(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Error)]
pub enum AuthError {
    #[error("refresh token mismatch for user {uid}")]
    RefreshTokenMismatch { uid: String },

    #[error("once token mismatch")]
    OnceTokenMismatch,

    #[error("missing session token header")]
    MissingSessionTokenHeader,

    #[error("missing session token cookie")]
    MissingSessionTokenCookie,

    #[error("missing refresh token")]
    MissingRefreshToken,

    #[error("expired")]
    Expired,

    #[error("wrong claims")]
    WrongClaims,

    #[error("invalid signature: {0}")]
    InvalidSignature(String),

    #[error("openid: {0}")]
    OpenId(String),

    #[error("registration consent required")]
    RegistrationConsentRequired,

    #[error("unable to create salt: {0}")]
    Salt(String),

    #[error("password: {0}")]
    Password(String),

    #[error("username is empty")]
    UsernameEmpty,

    #[error("username is invalid")]
    UsernameInvalid,

    #[error("username is taken")]
    UsernameTaken,

    #[error("missing role: {0:?}")]
    MissingRole(UserRole),

    #[error("invalid email")]
    InvalidEmail,

    #[error("email address already registered")]
    EmailAlreadyRegistered,

    #[error("invalid email or password")]
    InvalidCredentials,

    #[error("password too short")]
    PasswordTooShort,

    #[error("verify email: {0}")]
    VerifyEmail(String),

    #[error("reset password: {0}")]
    ResetPassword(String),
}
