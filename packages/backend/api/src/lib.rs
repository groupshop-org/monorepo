mod config;
mod context;
mod cors;
mod db;
mod durable;
mod handler;
mod notification;
mod prelude;
mod token_signing;
mod utils;

use worker::{event, Context, Env, HttpRequest, HttpResponse};

use crate::{
    context::ApiContext,
    cors::{apply_cors, CorsDeps},
    handler::handle_request,
};

#[event(fetch, respond_with_errors)]
async fn fetch(req: HttpRequest, env: Env, ctx: Context) -> worker::Result<HttpResponse> {
    let ctx = ApiContext::new(env, ctx);
    let cors_deps = CorsDeps::new(&req);
    let config = ctx.config.clone();

    Ok(apply_cors(
        handle_request(ctx, req).await?,
        cors_deps,
        &config,
    ))
}
