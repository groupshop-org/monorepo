mod config;

use std::sync::Arc;

use dominator::{append_dom, body, class, clone, html, Dom};
use futures_signals::signal::{Mutable, SignalExt};
use groupshop_frontend_shared::{
    document,
    theme::{self, chrome, color, typography},
};
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};

// ---------------------------------------------------------------------------
// Data types (mirror the health backend response)
// ---------------------------------------------------------------------------

#[derive(Clone, Deserialize)]
struct HealthReport {
    all_healthy: bool,
    services: Vec<ServiceHealth>,
}

#[derive(Clone, Deserialize)]
struct ServiceHealth {
    name: String,
    healthy: bool,
    status_code: Option<u16>,
    latency_ms: Option<u64>,
    error: Option<String>,
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct App {
    report: Mutable<Option<HealthReport>>,
}

impl App {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            report: Mutable::new(None),
        })
    }
}

// ---------------------------------------------------------------------------
// Async helpers
// ---------------------------------------------------------------------------

async fn sleep_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .unwrap();
    });
    let _ = JsFuture::from(promise).await;
}

async fn fetch_health(url: &str) -> Option<HealthReport> {
    let window = web_sys::window()?;
    let resp_value = JsFuture::from(window.fetch_with_str(url)).await.ok()?;
    let resp: web_sys::Response = resp_value.dyn_into().ok()?;
    let text_value = JsFuture::from(resp.text().ok()?).await.ok()?;
    let text = text_value.as_string()?;
    serde_json::from_str(&text).ok()
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    theme::stylesheet::init();
    document().set_title("System Health | GROUPSHOP");

    let app = App::new();

    // Polling loop
    spawn_local(clone!(app => async move {
        loop {
            let url = format!("{}/status", config::CONFIG.health_api_url);
            if let Some(report) = fetch_health(&url).await {
                app.report.set(Some(report));
            }
            sleep_ms(5_000).await;
        }
    }));

    append_dom(&body(), render(app));
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn render(app: Arc<App>) -> Dom {
    html!("main", {
        .class(&*chrome::PAGE)
        .child(html!("div", {
            .class(&*HEALTH_SHELL)
            .child(render_header())
            .child_signal(app.report.signal_cloned().map(|report| {
                Some(match report {
                    None => render_loading(),
                    Some(report) => render_dashboard(&report),
                })
            }))
            .child(render_footer())
        }))
    })
}

fn render_header() -> Dom {
    html!("header", {
        .class(&*chrome::MASTHEAD)
        .child(html!("div", {
            .class(&*chrome::BRAND)
            .child(html!("span", {
                .class(&*chrome::BRAND_WORD)
                .text("Groupshop")
            }))
        }))
        .child(html!("div", {
            .class(&*typography::SECTION_TITLE)
            .text("System Health")
        }))
    })
}

fn render_loading() -> Dom {
    html!("div", {
        .class(&*STATUS_BANNER_PENDING)
        .child(html!("div", {
            .class(&*typography::BANNER_TITLE)
            .text("CHECKING SERVICES...")
        }))
    })
}

fn render_dashboard(report: &HealthReport) -> Dom {
    let healthy_count = report.services.iter().filter(|s| s.healthy).count();
    let total = report.services.len();

    html!("div", {
        .children([
            // Status banner
            html!("section", {
                .class(if report.all_healthy { &*STATUS_BANNER_OK } else { &*STATUS_BANNER_FAIL })
                .child(html!("div", {
                    .class(&*BANNER_INNER)
                    .child(html!("div", {
                        .class(&*typography::BANNER_TITLE)
                        .text(if report.all_healthy { "ALL SYSTEMS OPERATIONAL" } else { "SERVICE ISSUES DETECTED" })
                    }))
                    .child(html!("div", {
                        .class(&*BANNER_SUBTITLE)
                        .text(&format!("{healthy_count}/{total} services healthy"))
                    }))
                }))
            }),
            // Service cards grid
            html!("div", {
                .class(&*SERVICE_GRID)
                .children(report.services.iter().map(render_service_card).collect::<Vec<_>>())
            }),
        ])
    })
}

fn render_service_card(service: &ServiceHealth) -> Dom {
    html!("article", {
        .class(&*SERVICE_CARD)
        .children([
            // Header row: status dot + name
            html!("div", {
                .class(&*SERVICE_HEADER)
                .child(html!("div", {
                    .class(if service.healthy { &*DOT_OK } else { &*DOT_FAIL })
                }))
                .child(html!("div", {
                    .class(&*typography::MINI_HEADING)
                    .text(&service.name)
                }))
            }),
            // Badge
            html!("div", {
                .class(if service.healthy { &*BADGE_OK } else { &*BADGE_FAIL })
                .text(if service.healthy { "PASSING" } else { "FAILING" })
            }),
            // Details
            html!("div", {
                .class(&*SERVICE_DETAILS)
                .apply_if(service.status_code.is_some(), |dom| {
                    dom.child(detail_row("Status", &format!("{}", service.status_code.unwrap())))
                })
                .apply_if(service.latency_ms.is_some(), |dom| {
                    dom.child(detail_row("Latency", &format!("{} ms", service.latency_ms.unwrap())))
                })
                .apply_if(service.error.is_some(), |dom| {
                    dom.child(html!("div", {
                        .class(&*ERROR_TEXT)
                        .text(service.error.as_deref().unwrap())
                    }))
                })
            }),
        ])
    })
}

fn detail_row(label: &str, value: &str) -> Dom {
    html!("div", {
        .class(&*DETAIL_ROW)
        .child(html!("span", {
            .class(&*typography::MICRO_LABEL)
            .text(label)
        }))
        .child(html!("span", {
            .class(&*DETAIL_VALUE)
            .text(value)
        }))
    })
}

fn render_footer() -> Dom {
    html!("footer", {
        .class(&*chrome::FOOTER)
        .child(html!("p", {
            .class(&*typography::FOOTER_TEXT)
            .text("Auto-refreshing every 5 seconds")
        }))
    })
}

// ---------------------------------------------------------------------------
// Health-dashboard-local styles (page composition stays app-local)
// ---------------------------------------------------------------------------

use std::sync::LazyLock;

static HEALTH_SHELL: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("width", "100%")
        .style("max-width", "56rem")
        .style("margin", "0 auto")
    }
});

static STATUS_BANNER_OK: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("margin-top", "1rem")
        .style("padding", "clamp(1.25rem, 2.5vw, 2rem) clamp(1.5rem, 3vw, 2.5rem)")
        .style("border-radius", "0.75rem")
        .style("background", color::GRADIENT_BRAND)
        .style("color", "#ffffff")
    }
});

static STATUS_BANNER_FAIL: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("margin-top", "1rem")
        .style("padding", "clamp(1.25rem, 2.5vw, 2rem) clamp(1.5rem, 3vw, 2.5rem)")
        .style("border-radius", "0.75rem")
        .style("background", format!("linear-gradient(135deg, {} 0%, #dc2626 100%)", color::RED))
        .style("color", "#ffffff")
    }
});

static STATUS_BANNER_PENDING: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("margin-top", "1rem")
        .style("padding", "clamp(1.25rem, 2.5vw, 2rem) clamp(1.5rem, 3vw, 2.5rem)")
        .style("border-radius", "0.75rem")
        .style("background", format!("linear-gradient(135deg, {} 0%, {} 100%)", color::MUTED, color::SUBTLE))
        .style("color", "#ffffff")
    }
});

static BANNER_INNER: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "flex")
        .style("flex-wrap", "wrap")
        .style("align-items", "center")
        .style("justify-content", "space-between")
        .style("gap", "0.5rem 2rem")
    }
});

static BANNER_SUBTITLE: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("font-size", "1rem")
        .style("font-weight", "600")
        .style("opacity", "0.85")
    }
});

static SERVICE_GRID: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "grid")
        .style("grid-template-columns", "repeat(auto-fill, minmax(min(13rem, 100%), 1fr))")
        .style("gap", "1rem")
        .style("margin-top", "1.5rem")
    }
});

static SERVICE_CARD: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("padding", "1rem")
        .style("border", format!("1px solid {}", color::LINE))
        .style("border-radius", "0.6rem")
        .style("background", "#ffffff")
        .style("box-shadow", "0 1px 3px rgba(0,0,0,0.06)")
    }
});

static SERVICE_HEADER: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "flex")
        .style("align-items", "center")
        .style("gap", "0.5rem")
        .style("margin-bottom", "0.65rem")
    }
});

static DOT_OK: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("width", "0.55rem")
        .style("height", "0.55rem")
        .style("border-radius", "999px")
        .style("background", color::GREEN)
        .style("flex-shrink", "0")
    }
});

static DOT_FAIL: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("width", "0.55rem")
        .style("height", "0.55rem")
        .style("border-radius", "999px")
        .style("background", color::RED)
        .style("flex-shrink", "0")
    }
});

static BADGE_OK: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "inline-block")
        .style("padding", "0.2rem 0.5rem")
        .style("border-radius", "999px")
        .style("background", "rgba(37, 172, 105, 0.10)")
        .style("color", color::GREEN)
        .style("font-size", "0.6rem")
        .style("font-weight", "700")
        .style("letter-spacing", "0.06em")
    }
});

static BADGE_FAIL: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "inline-block")
        .style("padding", "0.2rem 0.5rem")
        .style("border-radius", "999px")
        .style("background", "rgba(239, 68, 68, 0.10)")
        .style("color", color::RED)
        .style("font-size", "0.6rem")
        .style("font-weight", "700")
        .style("letter-spacing", "0.06em")
    }
});

static SERVICE_DETAILS: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("margin-top", "0.65rem")
        .style("display", "grid")
        .style("gap", "0.35rem")
    }
});

static DETAIL_ROW: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "flex")
        .style("justify-content", "space-between")
        .style("align-items", "center")
        .style("gap", "0.5rem")
    }
});

static DETAIL_VALUE: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("color", color::INK)
        .style("font-size", "0.82rem")
        .style("font-weight", "600")
    }
});

static ERROR_TEXT: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("margin-top", "0.25rem")
        .style("padding", "0.5rem")
        .style("border-radius", "0.3rem")
        .style("background", "rgba(239, 68, 68, 0.06)")
        .style("color", color::RED)
        .style("font-size", "0.75rem")
        .style("line-height", "1.5")
        .style("word-break", "break-word")
    }
});
