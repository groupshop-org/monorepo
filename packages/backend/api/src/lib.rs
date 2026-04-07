mod config;

use worker::{event, Context, Env, HttpRequest, Response, Result};

use crate::config::Config;

#[event(fetch)]
async fn fetch(_req: HttpRequest, env: Env, _ctx: Context) -> Result<Response> {
    let config = Config::new(&env);

    let _db = env.d1(&config.db_binding)?;
    Response::ok("hello world")
}
