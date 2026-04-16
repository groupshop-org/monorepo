mod api;
mod config;
mod pages;
mod route;

use crate::{api::ApiCtx, route::Route};
use dominator::{append_dom, body, clone, events, html, Dom};
use futures_signals::signal::{Mutable, SignalExt};
use groupshop_backend_shared::prelude::*;
use groupshop_frontend_shared::theme::color;
use groupshop_frontend_shared::{document, theme};

fn main() {
    theme::stylesheet::init();
    document().set_title("GROUPSHOP Admin");

    let initialized = Mutable::new(false);
    let route = Mutable::new(Route::Index);

    append_dom(
        &body(),
        html!("div", {
            .future(clone!(initialized, route => async move {
                let current_route = Route::current();
                let _ = ApiCtx::init().await;
                let profile = ApiCtx::get().profile.get_cloned();

                match profile {
                    Some(ref p) if p.roles.contains(&UserRole::Admin) => {
                        route.set(current_route);
                        initialized.set(true);
                    }
                    _ => {
                        let _ = web_sys::window()
                            .unwrap()
                            .location()
                            .set_href(config::landing_url());
                    }
                }
            }))
            .child_signal(initialized.signal().map(clone!(route => move |ready| {
                if ready {
                    Some(render_layout(route.get_cloned()))
                } else {
                    Some(render_loading())
                }
            })))
        }),
    );
}

fn render_loading() -> Dom {
    html!("div", {
        .style("display", "flex")
        .style("align-items", "center")
        .style("justify-content", "center")
        .style("min-height", "100vh")
        .style("color", color::MUTED)
        .style("font-size", "1rem")
        .text("Loading admin...")
    })
}

fn render_layout(route: Route) -> Dom {
    html!("div", {
        .style("display", "flex")
        .style("min-height", "100vh")
        .children([
            render_sidebar(&route),
            html!("div", {
                .style("flex", "1")
                .style("min-width", "0")
                .style("padding", "1.5rem 2rem")
                .child(match route {
                    Route::Index => pages::index::render(),
                    Route::Users => pages::users::render(),
                    Route::Products => pages::products::render(),
                    Route::Categories => pages::categories::render(),
                    Route::Brands => pages::brands::render(),
                })
            }),
        ])
    })
}

fn render_sidebar(current: &Route) -> Dom {
    let routes = [
        Route::Index,
        Route::Products,
        Route::Categories,
        Route::Brands,
        Route::Users,
    ];

    html!("nav", {
        .style("width", "13rem")
        .style("flex-shrink", "0")
        .style("padding", "1.5rem 1rem")
        .style("background", color::BG_DEEP)
        .style("border-right", &format!("1px solid {}", color::LINE))
        .style("display", "flex")
        .style("flex-direction", "column")
        .style("gap", "0.25rem")
        .children([
            html!("div", {
                .style("font-size", "1.1rem")
                .style("font-weight", "700")
                .style("margin-bottom", "1.5rem")
                .style("padding", "0 0.5rem")
                .style("color", "#1e9b5a")
                .text("Groupshop Admin")
            }),
        ])
        .children(routes.into_iter().map(|route| {
            let is_active = *current == route;
            nav_link(route, is_active)
        }).collect::<Vec<_>>())
        .child(html!("div", {
            .style("margin-top", "auto")
            .style("padding-top", "1rem")
            .style("border-top", &format!("1px solid {}", color::LINE))
            .children([
                html!("a", {
                    .style("display", "block")
                    .style("padding", "0.5rem")
                    .style("font-size", "0.85rem")
                    .style("color", color::MUTED)
                    .attr("href", config::landing_url())
                    .text("Back to site")
                }),
                html!("button", {
                    .style("display", "block")
                    .style("width", "100%")
                    .style("padding", "0.5rem")
                    .style("font-size", "0.85rem")
                    .style("color", color::RED)
                    .style("background", "transparent")
                    .style("border", "0")
                    .style("cursor", "pointer")
                    .style("text-align", "left")
                    .text("Sign out")
                    .event(|_: events::Click| {
                        ApiCtx::sign_out();
                    })
                }),
            ])
        }))
    })
}

fn nav_link(route: Route, active: bool) -> Dom {
    let bg = if active {
        "rgba(13, 104, 246, 0.08)"
    } else {
        "transparent"
    };
    let color = if active { color::BLUE } else { color::MUTED };
    let label = route.label();
    let href = route.link();

    html!("a", {
        .style("display", "block")
        .style("padding", "0.55rem 0.75rem")
        .style("border-radius", "0.375rem")
        .style("font-size", "0.9rem")
        .style("color", color)
        .style("background", bg)
        .style("font-weight", if active { "600" } else { "400" })
        .attr("href", &href)
        .text(label)
    })
}
