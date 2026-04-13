use serde::Serialize;
use worker::{
    event, Context, Env, Fetch, HttpRequest, Method, Request as WorkerRequest, Response, Result,
};

#[derive(Serialize)]
struct HealthReport {
    all_healthy: bool,
    services: Vec<ServiceHealth>,
}

#[derive(Serialize)]
struct ServiceHealth {
    name: String,
    healthy: bool,
    status_code: Option<u16>,
    latency_ms: Option<u64>,
    error: Option<String>,
}

struct Config {
    url_api: String,
    url_landing: String,
    url_landing_media: String,
    url_solana_rpc: String,
    allowed_origin: String,
}

impl Config {
    fn new(env: &Env) -> Self {
        let var = |key: &str| env.var(key).map(|v| v.to_string()).unwrap_or_default();
        Self {
            url_api: var("URL_API"),
            url_landing: var("URL_LANDING"),
            url_landing_media: var("URL_LANDING_MEDIA"),
            url_solana_rpc: var("URL_SOLANA_RPC"),
            allowed_origin: var("ALLOWED_ORIGIN"),
        }
    }
}

async fn check_service(name: &str, url: &str) -> ServiceHealth {
    let start = js_sys::Date::now();

    let result = (|| async {
        let request = WorkerRequest::new(url, Method::Get)?;
        let response = Fetch::Request(request).send().await?;
        Ok::<_, worker::Error>(response.status_code())
    })()
    .await;

    let latency = (js_sys::Date::now() - start) as u64;

    match result {
        Ok(status) => ServiceHealth {
            name: name.to_string(),
            healthy: status < 500,
            status_code: Some(status),
            latency_ms: Some(latency),
            error: None,
        },
        Err(e) => ServiceHealth {
            name: name.to_string(),
            healthy: false,
            status_code: None,
            latency_ms: Some(latency),
            error: Some(e.to_string()),
        },
    }
}

fn cors_headers(origin: &str) -> worker::Headers {
    let headers = worker::Headers::new();
    let _ = headers.set("Access-Control-Allow-Origin", origin);
    let _ = headers.set("Access-Control-Allow-Methods", "GET, OPTIONS");
    let _ = headers.set("Access-Control-Allow-Headers", "Content-Type");
    headers
}

#[event(fetch)]
async fn fetch(req: HttpRequest, env: Env, _ctx: Context) -> Result<Response> {
    let config = Config::new(&env);

    if req.method().as_str() == "OPTIONS" {
        return Ok(Response::empty()?.with_headers(cors_headers(&config.allowed_origin)));
    }

    if req.uri().path() != "/status" {
        return Response::error("Not Found", 404);
    }

    let services = vec![
        check_service("api", &config.url_api).await,
        check_service("landing", &config.url_landing).await,
        check_service("landing-media", &config.url_landing_media).await,
        check_service("solana-rpc", &format!("{}/health", config.url_solana_rpc)).await,
    ];

    let all_healthy = services.iter().all(|s| s.healthy);
    let report = HealthReport {
        all_healthy,
        services,
    };

    let json =
        serde_json::to_string(&report).map_err(|e| worker::Error::RustError(e.to_string()))?;
    let headers = cors_headers(&config.allowed_origin);
    let _ = headers.set("Content-Type", "application/json");
    Ok(Response::ok(json)?.with_headers(headers))
}
