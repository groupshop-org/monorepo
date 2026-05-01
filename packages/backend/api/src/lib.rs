mod config;
mod context;
mod cors;
mod cron;
mod db;
mod durable;
mod handler;
mod notification;
mod prelude;
mod solana;
mod token_signing;
mod utils;

use worker::{event, Context, Env, HttpRequest, HttpResponse, ScheduleContext, ScheduledEvent};

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

/// Scheduled-event handler. Wrangler triggers this on the cron schedule
/// in `wrangler.jsonc::triggers.crons`. The actual work lives in
/// `crate::cron::run` — same code path that the `/internal/cron-tick`
/// HTTP route hits, so dev iteration doesn't require scheduled events.
#[event(scheduled)]
async fn scheduled(_event: ScheduledEvent, env: Env, _sched: ScheduleContext) {
    // ScheduledEvent doesn't expose a `Context`-shaped object the way
    // `fetch` does, but ApiContext only needs `env`-derived state for
    // cron purposes. Build a minimal one and run.
    let mut api_ctx = ApiContext::new_without_worker_ctx(env);
    if let Err(err) = cron::run(&mut api_ctx).await {
        worker::console_error!("cron run failed: {err}");
    }
}
