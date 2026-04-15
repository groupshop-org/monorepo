use bytes::BytesMut;
use futures_util::StreamExt;
use http::StatusCode;
use js_sys::{ArrayBuffer, Uint8Array};
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::prelude::*;
use wasm_streams::ReadableStream;
use web_sys::{CryptoKey, HmacImportParams, WorkerGlobalScope};
use worker::wasm_bindgen_futures::JsFuture;
use worker::{Body, HttpResponse};

use groupshop_backend_shared::prelude::{ApiError, ApiResult};

pub async fn body_to_text(body: Body) -> ApiResult<String> {
    let stream = body
        .into_inner()
        .ok_or_else(|| ApiError::MissingBody("none at all".to_string()))?;
    let stream = ReadableStream::from_raw(stream);
    let mut stream = stream.into_stream();

    let mut bytes = BytesMut::new();
    while let Some(Ok(chunk)) = stream.next().await {
        let chunk: JsValue = chunk;
        if chunk.is_undefined() || chunk.is_null() {
            continue;
        }
        bytes.extend_from_slice(&chunk.unchecked_into::<Uint8Array>().to_vec());
    }

    if bytes.is_empty() {
        return Err(ApiError::MissingBody("empty body".to_string()));
    }

    String::from_utf8(bytes.to_vec()).map_err(|err| ApiError::MissingBody(err.to_string()))
}

pub async fn req_to_json<T: DeserializeOwned>(req: worker::HttpRequest) -> ApiResult<T> {
    let text = body_to_text(req.into_body()).await?;
    serde_json::from_str(&text)
        .map_err(|err| ApiError::ParseBody(format!("failed to parse json: {err}. body: {text}")))
}

pub async fn json_response(
    res: &impl Serialize,
    status_code: Option<StatusCode>,
) -> ApiResult<HttpResponse> {
    let bytes = serde_json::to_vec(res).map_err(|err| ApiError::ParseBody(err.to_string()))?;
    let mut response: HttpResponse = worker::Response::from_bytes(bytes)
        .map_err(|err| ApiError::Unknown(err.to_string()))?
        .try_into()
        .map_err(|err: worker::Error| ApiError::Unknown(err.to_string()))?;
    response
        .headers_mut()
        .insert("Content-Type", "application/json".parse().unwrap());
    if let Some(status_code) = status_code {
        *response.status_mut() = status_code;
    }
    Ok(response)
}

pub fn empty_response(status_code: Option<StatusCode>) -> HttpResponse {
    let mut response = HttpResponse::new(Body::empty());
    if let Some(status_code) = status_code {
        *response.status_mut() = status_code;
    }
    response
}

pub fn redirect_response(location: &str) -> HttpResponse {
    let mut response = empty_response(Some(StatusCode::FOUND));
    response
        .headers_mut()
        .insert("Location", location.parse().unwrap());
    response
}

pub async fn sha256_hash(input: &[u8]) -> ApiResult<[u8; 32]> {
    let promise = worker_crypto()?
        .subtle()
        .digest_with_str_and_u8_array("SHA-256", input)
        .map_err(|err| ApiError::Crypto(format!("failed to start SHA-256 digest: {err:?}")))?;
    let buffer = promise_to_array_buffer(promise, "SHA-256 digest failed").await?;

    array_buffer_to_bytes::<32>(buffer, "SHA-256 digest")
}

pub async fn sign_bytes(
    signing_key: impl AsRef<[u8]>,
    input: impl AsRef<[u8]>,
) -> ApiResult<[u8; 32]> {
    let key = import_hmac_sha256_key(signing_key.as_ref(), &["sign"]).await?;
    let promise = worker_crypto()?
        .subtle()
        .sign_with_str_and_u8_array("HMAC", &key, input.as_ref())
        .map_err(|err| ApiError::SigningFailure(format!("{err:?}")))?;
    let buffer = promise_to_array_buffer(promise, "HMAC-SHA-256 signing failed").await?;

    array_buffer_to_bytes::<32>(buffer, "HMAC-SHA-256 signature")
}

pub async fn verify_bytes(
    signing_key: impl AsRef<[u8]>,
    input: impl AsRef<[u8]>,
    signature: impl AsRef<[u8]>,
) -> ApiResult<bool> {
    let key = import_hmac_sha256_key(signing_key.as_ref(), &["verify"]).await?;
    let promise = worker_crypto()?
        .subtle()
        .verify_with_str_and_u8_array_and_u8_array("HMAC", &key, signature.as_ref(), input.as_ref())
        .map_err(|err| ApiError::SigningFailure(format!("{err:?}")))?;

    JsFuture::from(promise)
        .await
        .map_err(|err| ApiError::SigningFailure(format!("{err:?}")))?
        .as_bool()
        .ok_or_else(|| ApiError::SigningFailure("verification returned non-bool".to_string()))
}

fn worker_global() -> WorkerGlobalScope {
    js_sys::global().unchecked_into()
}

fn worker_crypto() -> ApiResult<web_sys::Crypto> {
    worker_global()
        .crypto()
        .map_err(|err| ApiError::Crypto(format!("failed to access WebCrypto: {err:?}")))
}

async fn import_hmac_sha256_key(signing_key: &[u8], usages: &[&str]) -> ApiResult<CryptoKey> {
    if signing_key.len() != 32 {
        return Err(ApiError::SigningFailure(format!(
            "token signing key must be exactly 32 bytes, got {}",
            signing_key.len()
        )));
    }

    let key_data = Uint8Array::from(signing_key);
    let algorithm = HmacImportParams::new("HMAC", &JsValue::from_str("SHA-256"));
    let usages = js_sys::Array::from_iter(usages.iter().copied().map(JsValue::from_str));

    let promise = worker_crypto()?
        .subtle()
        .import_key_with_object(
            "raw",
            key_data.unchecked_ref(),
            algorithm.unchecked_ref(),
            false,
            usages.unchecked_ref(),
        )
        .map_err(|err| ApiError::SigningFailure(format!("{err:?}")))?;

    JsFuture::from(promise)
        .await
        .map_err(|err| ApiError::SigningFailure(format!("{err:?}")))?
        .dyn_into()
        .map_err(|err| ApiError::SigningFailure(format!("{err:?}")))
}

async fn promise_to_array_buffer(
    promise: js_sys::Promise,
    error_context: &str,
) -> ApiResult<ArrayBuffer> {
    let result = JsFuture::from(promise)
        .await
        .map_err(|err| ApiError::Crypto(format!("{error_context}: {err:?}")))?;

    result
        .dyn_into()
        .map_err(|err| ApiError::Crypto(format!("{error_context}: non-buffer result: {err:?}")))
}

fn array_buffer_to_bytes<const N: usize>(buffer: ArrayBuffer, label: &str) -> ApiResult<[u8; N]> {
    Uint8Array::new(&buffer)
        .to_vec()
        .try_into()
        .map_err(|bytes: Vec<u8>| {
            ApiError::Crypto(format!(
                "{label} returned {} bytes instead of {N}",
                bytes.len(),
            ))
        })
}
