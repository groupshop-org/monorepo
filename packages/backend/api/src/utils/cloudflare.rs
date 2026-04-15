use groupshop_backend_shared::prelude::{ApiError, ApiResult, DurableObjectError, GroupshopCodec};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use worker::{
    d1::{D1Database, D1PreparedStatement},
    Env, Method, Request, RequestInit,
};

use crate::config::Config;

pub fn get_d1(env: &Env) -> ApiResult<D1Database> {
    let cfg = Config::new(env);
    env.d1(&cfg.db_binding)
        .map_err(|err| ApiError::Db(err.to_string()))
}

pub fn db_prepare(
    d1: &D1Database,
    query: impl Into<String>,
    params: &[JsValue],
) -> ApiResult<D1PreparedStatement> {
    d1.prepare(query)
        .bind(params)
        .map_err(|err| ApiError::Db(err.to_string()))
}

pub async fn db_exists(stmt: D1PreparedStatement) -> ApiResult<bool> {
    let row = stmt
        .first::<u32>(Some("n"))
        .await
        .map_err(|err| ApiError::Db(err.to_string()))?;
    Ok(row.is_some())
}

pub async fn db_try_load<T: for<'a> Deserialize<'a>>(
    stmt: D1PreparedStatement,
) -> ApiResult<Option<T>> {
    stmt.first::<T>(None)
        .await
        .map_err(|err| ApiError::Db(err.to_string()))
}

pub async fn db_load<T: for<'a> Deserialize<'a>>(
    stmt: D1PreparedStatement,
    not_found_msg: impl Into<String>,
) -> ApiResult<T> {
    db_try_load(stmt)
        .await?
        .ok_or_else(|| ApiError::Db(not_found_msg.into()))
}

pub async fn db_load_all<T: for<'a> Deserialize<'a>>(
    stmt: D1PreparedStatement,
) -> ApiResult<Vec<T>> {
    let results = stmt
        .all()
        .await
        .map_err(|err| ApiError::Db(err.to_string()))?;
    results
        .results::<T>()
        .map_err(|err| ApiError::Db(err.to_string()))
}

pub async fn db_execute(stmt: D1PreparedStatement) -> ApiResult<()> {
    let res = stmt
        .run()
        .await
        .map_err(|err| ApiError::Db(err.to_string()))?;

    match res.error() {
        Some(err) => Err(ApiError::Db(err.to_string())),
        None => Ok(()),
    }
}

pub async fn db_batch(d1: &D1Database, stmts: Vec<D1PreparedStatement>) -> ApiResult<()> {
    d1.batch(stmts)
        .await
        .map_err(|err| ApiError::Db(err.to_string()))?;
    Ok(())
}

pub fn deserialize_d1_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum BoolOrNumber {
        Bool(bool),
        Float(f64),
        Int(i64),
    }

    match BoolOrNumber::deserialize(deserializer)? {
        BoolOrNumber::Bool(value) => Ok(value),
        BoolOrNumber::Float(value) => Ok(value != 0.0),
        BoolOrNumber::Int(value) => Ok(value != 0),
    }
}

pub fn do_json_response<T: Serialize>(resp: &T) -> worker::Result<worker::Response> {
    let bytes = serde_json::to_vec(resp)
        .map_err(|err| worker::Error::RustError(format!("failed to encode response: {err}")))?;
    let mut response = worker::Response::from_body(worker::ResponseBody::Body(bytes))?;
    response
        .headers_mut()
        .set("Content-Type", "application/json")?;
    Ok(response)
}

pub async fn do_send<T: GroupshopCodec>(
    env: &Env,
    internal_endpoint: &str,
    binding: &str,
    object_id: Option<impl Into<String>>,
    cmd: impl GroupshopCodec,
) -> ApiResult<(String, T)> {
    let namespace = env
        .durable_object(binding)
        .map_err(|err| DurableObjectError::Namespace(err.to_string()))?;

    let object_id = match object_id {
        Some(object_id) => object_id.into(),
        None => namespace
            .unique_id()
            .map_err(|err| DurableObjectError::UniqueId(err.to_string()))?
            .to_string(),
    };

    let stub = namespace
        .get_by_name(&object_id)
        .map_err(|err| DurableObjectError::FailedStub {
            object_id: object_id.clone(),
            err: err.to_string(),
        })?;

    let req_body = cmd.encode()?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post);
    init.with_body(Some(req_body.into()));

    let mut req = Request::new_with_init(internal_endpoint, &init).map_err(|err| {
        DurableObjectError::RequestBuilder(format!("failed to build request: {err}"))
    })?;

    req.headers_mut()
        .map_err(|err| {
            DurableObjectError::RequestBuilder(format!("failed to get request headers: {err}"))
        })?
        .set("Content-Type", "application/json")
        .map_err(|err| {
            DurableObjectError::RequestBuilder(format!("failed to set request headers: {err}"))
        })?;

    let mut resp = stub
        .fetch_with_request(req)
        .await
        .map_err(|err| DurableObjectError::Fetch(err.to_string()))?;

    let status = resp.status_code();
    let body = resp
        .bytes()
        .await
        .map_err(|err| DurableObjectError::Fetch(err.to_string()))?;

    if !(200..300).contains(&status) {
        return Err(DurableObjectError::Status {
            status,
            message: String::from_utf8(body.to_vec()).ok(),
        }
        .into());
    }

    Ok((object_id, T::decode(&body)?))
}
