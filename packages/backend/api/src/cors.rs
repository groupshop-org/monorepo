use crate::prelude::*;
use http::{HeaderValue, Method};

pub struct CorsDeps {
    pub origin: Option<HeaderValue>,
    pub request_headers: String,
}

impl CorsDeps {
    pub fn new(req: &HttpRequest) -> Self {
        let origin = request_origin(req);
        let request_headers = req
            .headers()
            .get("Access-Control-Request-Headers")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();

        Self {
            origin,
            request_headers,
        }
    }
}

pub fn apply_cors(
    mut res: HttpResponse,
    CorsDeps {
        origin,
        request_headers,
    }: CorsDeps,
    config: &Config,
) -> HttpResponse {
    let headers = res.headers_mut();

    if is_request_origin_allowed(origin.as_ref(), config) {
        if let Some(origin) = origin {
            headers.insert("Access-Control-Allow-Origin", origin);
        }
    }

    headers.insert("Access-Control-Allow-Credentials", "true".parse().unwrap());
    headers.insert("Access-Control-Max-Age", "86400".parse().unwrap());
    headers.insert(
        "Access-Control-Allow-Methods",
        "GET, HEAD, POST, OPTIONS".parse().unwrap(),
    );
    headers.insert(
        "Access-Control-Allow-Headers",
        format!("Content-Type, Authorization, {request_headers}")
            .parse()
            .unwrap(),
    );
    headers.insert(
        "Access-Control-Expose-Headers",
        HEADER_AUTH_SESSION_TOKEN_UPDATE.parse().unwrap(),
    );
    headers.insert("Vary", "Origin".parse().unwrap());

    res
}

pub fn valid_cors_request(req: &HttpRequest, config: &Config) -> bool {
    let method = req.method();

    if !(method == Method::POST || method == Method::PUT || method == Method::DELETE) {
        true
    } else {
        is_request_origin_allowed(request_origin(req).as_ref(), config)
    }
}

fn is_request_origin_allowed(origin: Option<&HeaderValue>, config: &Config) -> bool {
    let Some(origin) = origin else {
        // No Origin header means this is not a browser request (e.g. CLI, curl).
        // Allow it — CORS is a browser-enforced mechanism.
        return true;
    };

    fn normalize_origin(origin: &str) -> &str {
        origin.trim_end_matches('/')
    }

    fn is_local_origin(origin: &str) -> bool {
        origin.starts_with("http://localhost:")
            || origin.starts_with("http://127.0.0.1:")
            || origin.starts_with("https://localhost:")
            || origin.starts_with("https://127.0.0.1:")
    }

    let origin_str = normalize_origin(origin.to_str().unwrap_or_default());
    let strict_match = config
        .allowed_origins
        .iter()
        .map(|allowed| normalize_origin(allowed))
        .any(|allowed| allowed == origin_str);

    strict_match || (config.allow_local_cors_bypass && is_local_origin(origin_str))
}

fn request_origin(req: &HttpRequest) -> Option<HeaderValue> {
    let headers = req.headers();
    headers
        .get("origin")
        .cloned()
        .or_else(|| headers.get("Origin").cloned())
        .or_else(|| {
            headers
                .get("referer")
                .or_else(|| headers.get("referrer"))
                .and_then(|value| value.to_str().ok())
                .and_then(|referer| {
                    let (scheme, rest) = referer.split_once("://")?;
                    let authority = rest.split('/').next()?;
                    Some(format!("{scheme}://{authority}"))
                })
                .and_then(|origin| HeaderValue::from_str(&origin).ok())
        })
}
