pub use crate::config::Config;
pub use crate::context::ApiContext;
pub use groupshop_backend_shared::prelude::*;
pub use worker::{HttpRequest, HttpResponse};

pub enum ApiHandlerResponse {
    Json(serde_json::Value),
    Raw(HttpResponse),
}

impl ApiHandlerResponse {
    pub fn raw(response: HttpResponse) -> Self {
        Self::Raw(response)
    }
}

pub trait IntoApiHandlerResponse: serde::Serialize + 'static {
    fn boxed(self) -> ApiHandlerResponse
    where
        Self: Sized,
    {
        ApiHandlerResponse::Json(serde_json::to_value(self).expect("response should serialize"))
    }
}

impl<T> IntoApiHandlerResponse for T where T: serde::Serialize + 'static {}
