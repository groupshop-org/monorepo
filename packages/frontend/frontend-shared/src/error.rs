use http::Method;
use thiserror::Error;

use groupshop_backend_shared::prelude::ApiError;

pub type FrontendResult<T> = Result<T, FrontendError>;

#[derive(Debug, Error)]
pub enum FrontendError {
    #[error("{0}")]
    Api(#[from] ApiError),

    #[error("gloo-net: {0}")]
    GlooNet(#[from] gloo_net::Error),

    #[error("request serialization failed: {err}")]
    ParseRequest { err: serde_json::Error },

    #[error("response deserialization failed: {err}. body: {text}")]
    ParseResponse {
        err: serde_json::Error,
        text: String,
    },

    #[error("unsupported method: {0}")]
    UnsupportedHttpMethod(Method),

    #[error("storage unavailable")]
    StorageUnavailable,

    #[error("{0}")]
    Other(String),
}
