mod api;
mod config;
mod legal;
mod route;
mod threshold;
mod wallet;

use std::sync::{Arc, Mutex, OnceLock};

use dominator::{append_dom, body, clone, events, html, svg, Dom};
use futures_signals::signal::{Mutable, SignalExt};
use groupshop_backend_shared::prelude::*;
use groupshop_frontend_shared::{
    document,
    theme::{self, chrome, typography},
    window,
};
use wasm_bindgen::{closure::Closure, JsCast};
use wasm_bindgen_futures::spawn_local;

use crate::{
    api::ApiCtx,
    legal::{help_page, privacy_policy, terms_of_service, PageContent},
    route::{Resolved, Route},
    wallet::render_deposit_panel,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum AuthMode {
    Signin,
    Register,
}

#[derive(Clone, PartialEq, Eq)]
enum UsernameStatus {
    Empty,
    Checking,
    Available,
    Taken,
    Invalid,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LegalModalKind {
    TermsOfService,
    PrivacyPolicy,
}

fn main() {
    theme::stylesheet::init();
    document().set_title("GROUPSHOP");

    let initialized = Mutable::new(false);
    let route = Mutable::new(Route::Home);

    append_dom(
        &body(),
        html!("div", {
            .future(clone!(initialized, route => async move {
                let current_route = Route::current();
                let _ = ApiCtx::init().await;
                let profile = ApiCtx::get().profile.get_cloned();
                match current_route.resolve(profile.as_ref()) {
                    Resolved::Redirect(target) => target.go_to_url(),
                    Resolved::Render(target) => {
                        route.set(target);
                        initialized.set(true);
                    }
                }
            }))
            .child_signal(initialized.signal().map(clone!(route => move |initialized| {
                if initialized {
                    Some(render(route.get_cloned()))
                } else {
                    Some(render_loading())
                }
            })))
        }),
    );
}

fn render_loading() -> Dom {
    html!("main", {
        .class(&*chrome::PAGE)
        .child(html!("div", {
            .class(&*chrome::SHELL)
            .child(html!("p", {
                .class(&*typography::LEAD_TEXT)
                .text("Loading Groupshop…")
            }))
        }))
    })
}

fn render(route: Route) -> Dom {
    html!("main", {
        .class(&*chrome::PAGE)
        .child(match route {
            Route::Home => render_home(),
            Route::Product { id } => render_product_detail(id),
            Route::Help => render_legal(
                help_page(),
                "How to set up your Phantom wallet with devnet SOL and USDC to place a group order.",
            ),
            Route::PrivacyPolicy => render_legal(
                privacy_policy(),
                "How Groupshop collects, uses, shares, and protects information in connection with the website and service.",
            ),
            Route::TermsOfService => render_legal(
                terms_of_service(),
                "The terms governing access to the Groupshop website, accounts, group deals, wallet interactions, orders, and related services.",
            ),
            Route::Signin => render_signin(),
            Route::Register => render_register(),
            Route::VerifyEmail => render_verify_email(),
            Route::VerifyEmailConfirm { token } => render_verify_email_confirm(token),
            Route::ResetPassword { token } => render_reset_password(token),
            Route::ChooseUsername => render_choose_username(),
            Route::OpenIdFinalize { token } => render_openid_finalize(token),
            Route::Profile => render_profile(),
            Route::Orders => render_orders(),
            Route::Error(err) => render_error(err),
            Route::NotFound => render_not_found(),
        })
    })
}

const HOME_PAGE_SIZE: u32 = 24;

fn render_home() -> Dom {
    let products: Mutable<Option<Vec<ProductSummary>>> = Mutable::new(None);
    let total: Mutable<u32> = Mutable::new(0);
    let categories: Mutable<Option<Vec<ProductCategorySummary>>> = Mutable::new(None);
    let selected_category: Mutable<Option<ProductCategoryId>> = Mutable::new(None);
    let current_page: Mutable<u32> = Mutable::new(1);
    // Filter: when on, only deals that already have at least one buyer
    // are returned. The backend does the filtering via a SQL EXISTS clause
    // so pagination stays accurate.
    let with_participants_only: Mutable<bool> = Mutable::new(false);

    // Refetch whenever the filter flips OR the polling tick advances.
    // Polling keeps participant counts fresh on the cards as other buyers
    // join — see `config::REFRESH_THRESHHOLD_VIEW_MS` for the cadence.
    let tick: Mutable<u32> = Mutable::new(0);
    spawn_local(clone!(tick => async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(config::REFRESH_THRESHHOLD_VIEW_MS).await;
            tick.set(tick.get().wrapping_add(1));
        }
    }));

    // One-shot category-tree load. Errors collapse to an empty list so
    // the sidebar gracefully shows "no categories" rather than blocking
    // product browsing.
    spawn_local(clone!(categories => async move {
        match ApiCtx::get().client.product_categories().await {
            Ok(res) => categories.set(Some(res.categories)),
            Err(_) => categories.set(Some(Vec::new())),
        }
    }));

    // Reset the page back to 1 whenever the traction filter flips —
    // otherwise a user toggling the filter on while on page 4 could land
    // on an out-of-range page. The initial emission is harmless since
    // the page is already 1.
    spawn_local(clone!(with_participants_only, current_page => async move {
        with_participants_only.signal().for_each(move |_| {
            current_page.set(1);
            async {}
        }).await;
    }));

    spawn_local(
        clone!(products, total, with_participants_only, selected_category, current_page, tick => async move {
            // Combined signal: re-emits whenever any of the inputs change.
            let signal = futures_signals::map_ref! {
                let only = with_participants_only.signal(),
                let cat = selected_category.signal_cloned(),
                let page = current_page.signal(),
                let _tick = tick.signal() => (*only, cat.clone(), *page)
            };
            signal.for_each(move |(only, cat, page)| {
                let products = products.clone();
                let total = total.clone();
                async move {
                    match ApiCtx::get().client.product_list(&ProductListRequest {
                        page,
                        per_page: HOME_PAGE_SIZE,
                        category_id: cat,
                        brand_id: None,
                        search: None,
                        with_participants_only: only,
                    }).await {
                        Ok(res) => {
                            total.set(res.total);
                            products.set(Some(res.products));
                        }
                        Err(_) => {
                            total.set(0);
                            products.set(Some(Vec::new()));
                        }
                    }
                }
            }).await;
        }),
    );

    html!("div", {
        .class(&*chrome::SHELL)
        .child(site_header())
        .child(hero_banner())
        .child(html!("section", {
            .attr("id", "products")
            .style("margin-top", "2.5rem")
            .style("display", "grid")
            .style("grid-template-columns", "minmax(0, 14rem) minmax(0, 1fr)")
            .style("gap", "1.5rem")
            .style("align-items", "start")
            .child(category_sidebar(categories.clone(), selected_category.clone(), current_page.clone()))
            .child(html!("div", {
                .style("min-width", "0")
                .child(html!("div", {
                    .class(&*chrome::SECTION_HEADER)
                    .children([
                        html!("div", {
                            .style("display", "flex")
                            .style("align-items", "center")
                            .style("gap", "0.9rem")
                            .style("flex-wrap", "wrap")
                            .children([
                                section_title("Products"),
                                traction_filter_toggle(with_participants_only.clone()),
                            ])
                        }),
                        html!("span", {
                            .class(&*typography::MICRO_LABEL)
                            .text_signal(total.signal().map(|t| {
                                if t > 0 { format!("{t} items") } else { String::new() }
                            }))
                        }),
                    ])
                }))
                .child(breadcrumbs(
                    categories.clone(),
                    selected_category.clone(),
                    current_page.clone(),
                ))
                .child(html!("div", {
                    .child_signal(products.signal_cloned().map(|maybe_products| {
                        Some(match maybe_products {
                            None => html!("p", {
                                .class(&*typography::BODY_MUTED)
                                .text("Loading products...")
                            }),
                            Some(products) if products.is_empty() => html!("div", {
                                .class(&*chrome::CARD)
                                .style("text-align", "center")
                                .style("padding", "3rem")
                                .children([
                                    html!("p", {
                                        .class(&*typography::LEAD_TEXT)
                                        .text("No products match this filter.")
                                    }),
                                    html!("p", {
                                        .class(&*typography::BODY_MUTED)
                                        .text("Try a different category or turn off the traction filter.")
                                    }),
                                ])
                            }),
                            Some(products) => html!("div", {
                                .class(&*chrome::DEAL_GRID)
                                .children(products.into_iter().map(product_card).collect::<Vec<_>>())
                            }),
                        })
                    }))
                }))
                .child(pagination_bar(current_page.clone(), total.clone()))
            }))
        }))
        .child(how_it_works_section())
        .child(site_footer())
    })
}

/// Sidebar with the full category tree. Selecting a category updates the
/// `selected_category` Mutable and resets the page back to 1 so the user
/// doesn't land on an empty page when switching to a smaller category.
fn category_sidebar(
    categories: Mutable<Option<Vec<ProductCategorySummary>>>,
    selected: Mutable<Option<ProductCategoryId>>,
    current_page: Mutable<u32>,
) -> Dom {
    html!("aside", {
        .style("position", "sticky")
        .style("top", "1rem")
        .style("align-self", "start")
        .style("max-height", "calc(100vh - 2rem)")
        .style("overflow-y", "auto")
        .style("border", &format!("1px solid {}", groupshop_frontend_shared::theme::color::LINE))
        .style("border-radius", "0.6rem")
        .style("background", "#ffffff")
        .style("padding", "0.75rem")
        .child(html!("div", {
            .class(&*typography::MICRO_LABEL)
            .style("margin-bottom", "0.5rem")
            .text("Categories")
        }))
        .child(category_link_button(
            "All products",
            None,
            0,
            selected.clone(),
            current_page.clone(),
        ))
        .child_signal(categories.signal_cloned().map(clone!(selected, current_page => move |maybe_cats| {
            Some(match maybe_cats {
                None => html!("p", {
                    .class(&*typography::BODY_MUTED)
                    .style("font-size", "0.8rem")
                    .style("padding", "0.4rem 0.5rem")
                    .text("Loading…")
                }),
                Some(cats) if cats.is_empty() => html!("p", {
                    .class(&*typography::BODY_MUTED)
                    .style("font-size", "0.8rem")
                    .style("padding", "0.4rem 0.5rem")
                    .text("No categories yet.")
                }),
                Some(cats) => html!("div", {
                    .style("display", "flex")
                    .style("flex-direction", "column")
                    .children(cats.into_iter().map(|c| category_link_button(
                        &c.name,
                        Some(c.id),
                        c.depth,
                        selected.clone(),
                        current_page.clone(),
                    )).collect::<Vec<_>>())
                }),
            })
        })))
    })
}

fn category_link_button(
    label: &str,
    target: Option<ProductCategoryId>,
    depth: u32,
    selected: Mutable<Option<ProductCategoryId>>,
    current_page: Mutable<u32>,
) -> Dom {
    let label = label.to_string();
    let target_for_signal = target.clone();
    let target_for_click = target.clone();
    html!("button", {
        .attr("type", "button")
        .style("text-align", "left")
        .style("background", "transparent")
        .style("border", "0")
        .style("cursor", "pointer")
        .style("padding", "0.4rem 0.5rem")
        .style("padding-left", &format!("{}rem", 0.5 + depth as f64 * 0.85))
        .style("border-radius", "0.35rem")
        .style("font-size", "0.88rem")
        .style_signal("background", selected.signal_cloned().map(move |sel| {
            if sel == target_for_signal { "rgba(13, 104, 246, 0.08)".to_string() } else { "transparent".to_string() }
        }))
        .style_signal("font-weight", selected.signal_cloned().map({
            let target_for_weight = target.clone();
            move |sel| if sel == target_for_weight { "600".to_string() } else { "400".to_string() }
        }))
        .style_signal("color", selected.signal_cloned().map({
            let target_for_color = target.clone();
            move |sel| if sel == target_for_color { "#0d68f6".to_string() } else { "#111827".to_string() }
        }))
        .text(&label)
        .event(clone!(selected, current_page => move |_: events::Click| {
            current_page.set(1);
            selected.set(target_for_click.clone());
        }))
    })
}

/// Breadcrumbs: "All products / Parent / Selected". Walks parent_id up
/// the loaded category tree. Hidden when nothing is selected (the
/// products grid is "All products" by default).
fn breadcrumbs(
    categories: Mutable<Option<Vec<ProductCategorySummary>>>,
    selected: Mutable<Option<ProductCategoryId>>,
    current_page: Mutable<u32>,
) -> Dom {
    html!("nav", {
        .style("margin-top", "0.5rem")
        .style("margin-bottom", "0.75rem")
        .style("font-size", "0.85rem")
        .style("color", "#6b7280")
        .child_signal(
            futures_signals::map_ref! {
                let cats = categories.signal_cloned(),
                let sel = selected.signal_cloned() => (cats.clone(), sel.clone())
            }.map(clone!(selected, current_page => move |(cats, sel)| {
                let cats = cats.unwrap_or_default();
                let trail: Vec<ProductCategorySummary> = match sel {
                    None => Vec::new(),
                    Some(id) => walk_up_trail(&cats, &id),
                };
                if trail.is_empty() {
                    return None;
                }
                let mut items: Vec<Dom> = Vec::new();
                items.push(crumb_link(
                    "All products",
                    None,
                    selected.clone(),
                    current_page.clone(),
                ));
                let last_idx = trail.len() - 1;
                for (i, cat) in trail.into_iter().enumerate() {
                    items.push(html!("span", {
                        .style("margin", "0 0.4rem")
                        .text("›")
                    }));
                    if i == last_idx {
                        items.push(html!("span", {
                            .style("color", "#111827")
                            .style("font-weight", "600")
                            .text(&cat.name)
                        }));
                    } else {
                        items.push(crumb_link(
                            &cat.name,
                            Some(cat.id),
                            selected.clone(),
                            current_page.clone(),
                        ));
                    }
                }
                Some(html!("div", {
                    .style("display", "flex")
                    .style("flex-wrap", "wrap")
                    .style("align-items", "center")
                    .children(items)
                }))
            }))
        )
    })
}

fn crumb_link(
    label: &str,
    target: Option<ProductCategoryId>,
    selected: Mutable<Option<ProductCategoryId>>,
    current_page: Mutable<u32>,
) -> Dom {
    let label = label.to_string();
    html!("button", {
        .attr("type", "button")
        .style("background", "transparent")
        .style("border", "0")
        .style("padding", "0")
        .style("cursor", "pointer")
        .style("color", "#0d68f6")
        .style("font-size", "0.85rem")
        .text(&label)
        .event(clone!(selected, current_page => move |_: events::Click| {
            current_page.set(1);
            selected.set(target.clone());
        }))
    })
}

fn walk_up_trail(
    cats: &[ProductCategorySummary],
    leaf: &ProductCategoryId,
) -> Vec<ProductCategorySummary> {
    let mut out: Vec<ProductCategorySummary> = Vec::new();
    let mut current: Option<ProductCategoryId> = Some(leaf.clone());
    // Hard cap to defend against accidental cycles in seed data.
    for _ in 0..32 {
        let Some(id) = current.clone() else { break };
        let Some(found) = cats.iter().find(|c| c.id == id) else {
            break;
        };
        out.push(found.clone());
        current = found.parent_id.clone();
    }
    out.reverse();
    out
}

fn pagination_bar(current_page: Mutable<u32>, total: Mutable<u32>) -> Dom {
    html!("div", {
        .style("margin-top", "1.25rem")
        .style("display", "flex")
        .style("justify-content", "center")
        .style("align-items", "center")
        .style("gap", "0.75rem")
        .child_signal(
            futures_signals::map_ref! {
                let page = current_page.signal(),
                let total = total.signal() => (*page, *total)
            }.map(clone!(current_page => move |(page, total)| {
                let page_size = HOME_PAGE_SIZE;
                let page_count = if total == 0 { 1 } else { total.div_ceil(page_size) };
                if page_count <= 1 {
                    return None;
                }
                let prev_disabled = page <= 1;
                let next_disabled = page >= page_count;
                Some(html!("div", {
                    .style("display", "flex")
                    .style("align-items", "center")
                    .style("gap", "0.5rem")
                    .children([
                        page_button("‹ Prev", prev_disabled, clone!(current_page => move || {
                            let p = current_page.get();
                            if p > 1 { current_page.set(p - 1); }
                            scroll_to_top();
                        })),
                        html!("span", {
                            .style("font-size", "0.85rem")
                            .style("color", "#374151")
                            .text(&format!("Page {page} of {page_count}"))
                        }),
                        page_button("Next ›", next_disabled, clone!(current_page => move || {
                            let p = current_page.get();
                            current_page.set(p + 1);
                            scroll_to_top();
                        })),
                    ])
                }))
            }))
        )
    })
}

fn page_button(label: &str, disabled: bool, on_click: impl Fn() + 'static) -> Dom {
    let label = label.to_string();
    html!("button", {
        .attr("type", "button")
        .prop("disabled", disabled)
        .style("padding", "0.4rem 0.85rem")
        .style("border-radius", "0.45rem")
        .style("border", &format!("1px solid {}", groupshop_frontend_shared::theme::color::LINE))
        .style("background", if disabled { "#f3f4f6" } else { "#ffffff" })
        .style("color", if disabled { "#9ca3af" } else { "#111827" })
        .style("cursor", if disabled { "not-allowed" } else { "pointer" })
        .style("font-size", "0.85rem")
        .text(&label)
        .event(move |_: events::Click| {
            if !disabled {
                on_click();
            }
        })
    })
}

fn scroll_to_top() {
    if let Some(win) = web_sys::window() {
        win.scroll_to_with_x_and_y(0.0, 0.0);
    }
}

fn hero_banner() -> Dom {
    html!("section", {
        .class(&*chrome::BANNER)
        .child(html!("div", {
            .class(&*chrome::BANNER_COPY)
            .child(html!("p", {
                .class(&*typography::EYEBROW)
                .text("Group buying for wholesale prices")
            }))
            .child(html!("h1", {
                .class(&*typography::BANNER_TITLE)
                .text("Buy together. Pay less.")
            }))
            .child(html!("p", {
                .class(&*typography::LEAD_TEXT)
                .text("Join other buyers to meet minimum order quantities and unlock wholesale pricing on real products.")
            }))
        }))
    })
}

fn product_card(product: ProductSummary) -> Dom {
    let price = format!("${:.2}", product.price_cents as f64 / 100.0);
    let moq = format!("MOQ: {}", product.minimum_order_quantity);
    let link = Route::Product {
        id: product.id.clone(),
    }
    .link();

    html!("a", {
        .class(&*chrome::DEAL_CARD)
        .attr("href", &link)
        .style("text-decoration", "none")
        .style("color", "inherit")
        .style("display", "block")
        .child(html!("div", {
            .class(&*chrome::DEAL_VISUAL)
            .class(&*chrome::DEAL_VISUAL_SILVER)
            .child(html!("div", {
                .class(&*chrome::DEAL_BADGE)
                .text(product.category_id.as_str())
            }))
            .apply_if(!product.image_url.is_empty(), clone!(product => move |dom| {
                dom.child(html!("img", {
                    .attr("src", &product.image_url)
                    .attr("alt", &product.name)
                    .style("width", "100%")
                    .style("height", "100%")
                    .style("object-fit", "contain")
                    .style("position", "absolute")
                    .style("inset", "0")
                }))
            }))
        }))
        .child(html!("div", {
            .class(&*chrome::DEAL_BODY)
            .child(html!("div", {
                .class(&*typography::CARD_TITLE)
                .text(&product.name)
            }))
            .child(html!("div", {
                .class(&*chrome::PRICE_ROW)
                .children([
                    html!("div", {
                        .class(&*typography::PRICE_NOW)
                        .text(&price)
                    }),
                    html!("div", {
                        .style("font-size", "0.82rem")
                        .style("color", "#6b7280")
                        .text(&moq)
                    }),
                ])
            }))
            .child(html!("div", {
                .style("display", "flex")
                .style("gap", "0.5rem")
                .style("align-items", "center")
                .style("margin-top", "0.4rem")
                .children([
                    html!("span", {
                        .class(&*typography::MICRO_LABEL)
                        .text(product.brand_id.as_str())
                    }),
                    if product.is_preorder {
                        html!("span", {
                            .style("font-size", "0.7rem")
                            .style("padding", "0.15rem 0.5rem")
                            .style("border-radius", "999px")
                            .style("background", "rgba(245, 158, 11, 0.12)")
                            .style("color", "#b45309")
                            .text("Pre-order")
                        })
                    } else {
                        html!("span", {})
                    },
                ])
            }))
            // Threshold progress lives at the bottom of the card so users
            // can scan the grid for deals that are close to triggering.
            .child(html!("div", {
                .style("margin-top", "0.6rem")
                .child(threshold::progress(
                    product.committed_units,
                    product.minimum_order_quantity,
                    true,
                ))
            }))
        }))
    })
}

/// Checkbox toggle for the "deals gaining traction" filter. Sits next to
/// the section title so the filter is the first interactive thing in the
/// products section. The label is colored + bold so it reads as a
/// first-class affordance, not muted UI chrome.
fn traction_filter_toggle(state: Mutable<bool>) -> Dom {
    html!("label", {
        .style("display", "inline-flex")
        .style("align-items", "center")
        .style("gap", "0.5rem")
        .style("cursor", "pointer")
        .style("user-select", "none")
        .child(html!("input" => web_sys::HtmlInputElement, {
            .attr("type", "checkbox")
            // Sized up from the browser default so the checkbox is
            // visually balanced against the (now-bold) label text.
            .style("inline-size", "1.05rem")
            .style("block-size", "1.05rem")
            .style("accent-color", "#16a34a")
            .style("cursor", "pointer")
            .prop_signal("checked", state.signal())
            .event(clone!(state => move |evt: events::Change| {
                let checked = evt
                    .target()
                    .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                    .map(|input| input.checked())
                    .unwrap_or(false);
                state.set(checked);
            }))
        }))
        // Brand-green bold text when on; same color but a hair lighter
        // when off so the affordance stays readable in both states.
        .child(html!("span", {
            .style("font-size", "0.92rem")
            .style("font-weight", "700")
            .style_signal("color", state.signal().map(|on| {
                if on { "#0c6e5c" } else { "#16a34a" }
            }))
            .text("Only deals gaining traction")
        }))
    })
}

fn how_it_works_section() -> Dom {
    // Each card is a teaser for the full modal. Clicking anywhere on a
    // card pops the modal so users get the deep version (Solana, wallet,
    // escrow flow) rather than only the three short blurbs below.
    let teaser_card = |title: &'static str, body: &'static str| -> Dom {
        html!("article", {
            .class(&*chrome::SIGNAL_CARD)
            .style("cursor", "pointer")
            .event(|_: events::Click| {
                how_it_works_modal_state().set(true);
            })
            .children([
                html!("div", { .class(&*typography::MINI_HEADING) .text(title) }),
                html!("p", { .class(&*typography::BODY_MUTED) .text(body) }),
            ])
        })
    };

    html!("section", {
        .attr("id", "how-it-works")
        .class(&*chrome::SIGNAL_GRID)
        .style("margin-top", "3rem")
        .children([
            teaser_card(
                "Browse products",
                "Explore the catalog with real wholesale pricing. No account needed to browse.",
            ),
            teaser_card(
                "Commit together",
                "Join a group order to meet the minimum order quantity. More buyers means everyone saves.",
            ),
            teaser_card(
                "Get wholesale prices",
                "Once the group hits the threshold, orders are placed at bulk pricing and shipped to you.",
            ),
        ])
    })
}

fn section_title(text: &'static str) -> Dom {
    html!("div", {
        .class(&*typography::SECTION_TITLE)
        .class(&*chrome::SECTION_MARK)
        .text(text)
    })
}

fn render_product_detail(id: ProductId) -> Dom {
    let product: Mutable<Option<Result<ProductSummary, String>>> = Mutable::new(None);
    // Live committed_units total is held in its own Mutable so the
    // polling loop can update it without forcing the surrounding DOM
    // (incl. the deposit panel) to re-render. Re-rendering the panel
    // would reset its local `wallet_address` / `success` state and
    // bounce the user back to "Phantom not connected" after every poll
    // tick.
    let committed_units: Mutable<u64> = Mutable::new(0);
    // The signed-in user's pending order in the active batch (`None`
    // when anonymous or no order yet). Polled alongside the product
    // detail so the cart banner reflects fresh deposits and refunds.
    let active_order: Mutable<Option<AccountOrderActive>> = Mutable::new(None);

    spawn_local(
        clone!(product, committed_units, active_order, id => async move {
            loop {
                match ApiCtx::get()
                    .client
                    .product_detail(&ProductDetailRequest { id: id.clone() })
                    .await
                {
                    Ok(res) => {
                        committed_units.set(res.product.committed_units);
                        // Only set the product Mutable on the first load —
                        // subsequent polls update committed_units above
                        // without churning the DOM.
                        if product.get_cloned().is_none() {
                            product.set(Some(Ok(res.product)));
                        }
                    }
                    Err(e) => {
                        if product.get_cloned().is_none() {
                            product.set(Some(Err(format!("{e:?}"))));
                        }
                    }
                }
                // Probe per-user cart state, but only when signed in.
                // Anonymous users can't have a pending order, so skip the
                // round-trip entirely.
                if ApiCtx::get().profile.get_cloned().is_some() {
                    if let Ok(res) = ApiCtx::get()
                        .client
                        .account_order_status(&AccountOrderStatusRequest { product_id: id.clone() })
                        .await
                    {
                        active_order.set(res.active_order);
                    }
                } else {
                    active_order.set(None);
                }
                gloo_timers::future::TimeoutFuture::new(config::REFRESH_THRESHHOLD_VIEW_MS).await;
            }
        }),
    );

    html!("div", {
        .class(&*chrome::SHELL)
        .child(site_header())
        .child(html!("div", {
            .child_signal(product.signal_cloned().map(clone!(committed_units, active_order => move |state| {
                Some(match state {
                    None => html!("div", {
                        .style("padding", "3rem 0")
                        .style("text-align", "center")
                        .child(html!("p", { .class(&*typography::BODY_MUTED) .text("Loading product...") }))
                    }),
                    Some(Err(msg)) => html!("div", {
                        .style("padding", "3rem 0")
                        .style("text-align", "center")
                        .children([
                            html!("p", { .style("color", "#ef4444") .text(&msg) }),
                            html!("a", {
                                .class(&*chrome::BUTTON)
                                .attr("href", "/")
                                .style("margin-top", "1rem")
                                .style("display", "inline-flex")
                                .text("Back to products")
                            }),
                        ])
                    }),
                    Some(Ok(p)) => render_product_detail_content(p, committed_units.clone(), active_order.clone()),
                })
            })))
        }))
        .child(site_footer())
    })
}

fn render_product_detail_content(
    product: ProductSummary,
    committed_units: Mutable<u64>,
    active_order: Mutable<Option<AccountOrderActive>>,
) -> Dom {
    let price = format!("${:.2}", product.price_cents as f64 / 100.0);

    html!("div", {
        .style("display", "grid")
        .style("grid-template-columns", "1fr 1fr")
        .style("gap", "2rem")
        .style("margin-top", "1.5rem")
        .style("align-items", "start")
        // Left: product image + info
        .child(html!("div", {
            // Image
            .child(html!("div", {
                .style("position", "relative")
                .style("width", "100%")
                .style("aspect-ratio", "1")
                .style("border-radius", "0.75rem")
                .style("overflow", "hidden")
                .style("background", "linear-gradient(135deg, #89f7fe 0%, #66a6ff 100%)")
                .apply_if(!product.image_url.is_empty(), clone!(product => move |dom| {
                    dom.child(html!("img", {
                        .attr("src", &product.image_url)
                        .attr("alt", &product.name)
                        .style("width", "100%")
                        .style("height", "100%")
                        .style("object-fit", "contain")
                    }))
                }))
            }))
            // Details below image
            .child(html!("div", {
                .style("margin-top", "1.5rem")
                .class(&*chrome::CARD)
                .children([
                    html!("h3", {
                        .class(&*typography::MINI_HEADING)
                        .text("Product details")
                    }),
                    detail_row("GTIN", &product.gtin),
                    detail_row("Category", product.category_id.as_str()),
                    detail_row("Brand", product.brand_id.as_str()),
                    detail_row("Inventory", &format!("{} units", product.inventory)),
                    detail_row("Currency", &product.currency),
                ])
                .apply_if(product.is_preorder, |dom| {
                    dom.child(detail_row("Delivery", &match product.estimated_delivery_weeks {
                        Some(w) => format!("Pre-order ({w} weeks)"),
                        None => "Pre-order".to_string(),
                    }))
                })
            }))
        }))
        // Right: payment panel
        .child(html!("div", {
            .style("position", "sticky")
            .style("top", "1.5rem")
            // Product name + price
            .child(html!("div", {
                .child(html!("h1", {
                    .class(&*typography::SECTION_TITLE)
                    .text(&product.name)
                }))
                .child(html!("div", {
                    .style("display", "flex")
                    .style("align-items", "baseline")
                    .style("gap", "0.75rem")
                    .style("margin-top", "0.5rem")
                    .children([
                        html!("span", {
                            .style("font-size", "2rem")
                            .style("font-weight", "700")
                            .style("color", "#111827")
                            .text(&price)
                        }),
                        html!("span", {
                            .class(&*typography::BODY_MUTED)
                            .text(&format!("per unit \u{00b7} MOQ: {}", product.minimum_order_quantity))
                        }),
                    ])
                }))
            }))
            // Threshold progress: anchored above the deposit panel so the
            // user sees recruitment status before committing. Driven by
            // a signal so the polling loop can refresh the count without
            // re-rendering (and resetting) the deposit panel below it.
            .child(html!("div", {
                .style("margin-top", "1rem")
                .child(threshold::progress_signal(
                    committed_units.signal(),
                    product.minimum_order_quantity,
                    false,
                ))
            }))
            // Cart-pending banner: shows up above the deposit panel
            // when the signed-in user already has a non-refunded
            // participation in this product's active batch. The
            // `child_signal` evaluates per polling tick so it appears
            // immediately after a successful deposit and disappears
            // after a self-refund.
            .child_signal(active_order.signal_cloned().map(clone!(product => move |maybe_order| {
                maybe_order.map(|order| cart_pending_banner(&product, &order))
            })))
            .child(render_deposit_panel(product.clone(), committed_units.clone()))
            // Back link
            .child(html!("a", {
                .attr("href", "/")
                .style("display", "inline-flex")
                .style("align-items", "center")
                .style("gap", "0.35rem")
                .style("margin-top", "1rem")
                .style("font-size", "0.85rem")
                .style("color", groupshop_frontend_shared::theme::color::BLUE)
                .text("\u{2190} Back to all products")
            }))
        }))
    })
}

/// Banner shown above the deposit panel when the signed-in user already
/// has a confirmed, non-refunded participation in the product's active
/// batch. Surfaces what they committed and links to the My Orders page
/// where they can manage / withdraw.
fn cart_pending_banner(product: &ProductSummary, order: &AccountOrderActive) -> Dom {
    let total_usdc = order.product_amount_base_units as f64 / 1_000_000.0;
    let line = format!(
        "You're in this group order — {} unit{} committed (${:.2})",
        order.quantity,
        if order.quantity == 1 { "" } else { "s" },
        total_usdc,
    );
    let _ = product; // reserved for future per-product deep-links

    html!("div", {
        .style("margin-top", "1rem")
        .style("padding", "0.85rem 1rem")
        .style("border-radius", "0.85rem")
        .style("border", "1px solid rgba(22, 163, 74, 0.45)")
        .style("background", "rgba(22, 163, 74, 0.08)")
        .style("display", "flex")
        .style("flex-wrap", "wrap")
        .style("align-items", "center")
        .style("justify-content", "space-between")
        .style("gap", "0.75rem")
        .child(html!("div", {
            .child(html!("div", {
                .style("font-size", "0.78rem")
                .style("font-weight", "600")
                .style("text-transform", "uppercase")
                .style("letter-spacing", "0.06em")
                .style("color", "#0c6e5c")
                .text(&format!("Pending order \u{00b7} Batch #{}", order.batch_id))
            }))
            .child(html!("div", {
                .style("margin-top", "0.2rem")
                .style("font-size", "0.92rem")
                .style("color", "#102120")
                .text(&line)
            }))
        }))
        .child(html!("a", {
            .attr("href", &Route::Orders.link())
            .style("display", "inline-flex")
            .style("align-items", "center")
            .style("padding", "0.5rem 0.95rem")
            .style("border-radius", "999px")
            .style("background", "#16a34a")
            .style("color", "#fff")
            .style("font-weight", "600")
            .style("font-size", "0.85rem")
            .style("text-decoration", "none")
            .style("white-space", "nowrap")
            .text("View in Cart")
        }))
    })
}

fn detail_row(label: &str, value: &str) -> Dom {
    html!("div", {
        .style("display", "flex")
        .style("justify-content", "space-between")
        .style("padding", "0.4rem 0")
        .style("border-bottom", &format!("1px solid {}", groupshop_frontend_shared::theme::color::BORDER_SOFT))
        .children([
            html!("span", {
                .class(&*typography::MICRO_LABEL)
                .text(label)
            }),
            html!("span", {
                .style("font-size", "0.85rem")
                .text(value)
            }),
        ])
    })
}

fn render_legal(page: PageContent, lead: &'static str) -> Dom {
    html!("div", {
        .class(&*chrome::LEGAL_SHELL)
        .child(site_header())
        .child(html!("section", {
            .class(&*chrome::LEGAL_HERO)
            .child(html!("h1", {
                .class(&*typography::LEGAL_TITLE)
                .text(page.title)
            }))
            .child(html!("p", {
                .class(&*typography::LEAD_TEXT)
                .text(lead)
            }))
        }))
        .child(html!("article", {
            .class(&*chrome::CARD)
            .child(html!("div", {
                .class(&*chrome::LEGAL_BODY)
                .after_inserted(move |element| {
                    element.set_inner_html(page.html);
                })
            }))
        }))
        .child(site_footer())
    })
}

fn render_signin() -> Dom {
    let email = Arc::new(Mutex::new(String::new()));
    let password = Arc::new(Mutex::new(String::new()));
    let error = Mutable::new(None::<String>);
    let notice = Mutable::new(None::<String>);

    render_auth_page(
        AuthMode::Signin,
        "Account access",
        "",
        vec![
            message_block(notice.clone(), "success"),
            message_block(error.clone(), "error"),
            auth_surface_row(vec![
                auth_surface(
                    "Email sign-in",
                    "",
                    vec![
                        input_field(
                            "Email",
                            "email",
                            "email",
                            "email",
                            clone!(email => move |value| {
                                *email.lock().unwrap() = value;
                            }),
                        ),
                        input_field(
                            "Password",
                            "password",
                            "password",
                            "current-password",
                            clone!(password => move |value| {
                                *password.lock().unwrap() = value;
                            }),
                        ),
                        auth_button(
                            "Email me a reset link",
                            true,
                            None,
                            clone!(email, notice, error => move || {
                                let email = email.lock().unwrap().clone();
                                if email.is_empty() {
                                    error.set(Some("Enter your email address first.".to_string()));
                                    return;
                                }
                                spawn_local(clone!(notice, error => async move {
                                    match ApiCtx::get().client.auth_email_password_send_reset(Some(email)).await {
                                        Ok(()) => notice.set(Some("If that email is registered, a reset link has been sent.".to_string())),
                                        Err(err) => error.set(Some(err.to_string())),
                                    }
                                }));
                            }),
                        ),
                        auth_button(
                            "Sign in",
                            false,
                            None,
                            clone!(email, password, error => move || {
                                let email = email.lock().unwrap().clone();
                                let password = password.lock().unwrap().clone();
                                spawn_local(clone!(error => async move {
                                    match ApiCtx::get().client.auth_email_password_signin(email, password).await {
                                        Ok(()) => {
                                            let _ = ApiCtx::refresh_profile().await;
                                            Route::Home.go_to_url();
                                        }
                                        Err(err) => error.set(Some(err.to_string())),
                                    }
                                }));
                            }),
                        ),
                    ],
                ),
                auth_surface(
                    "Google",
                    "",
                    vec![auth_button(
                        "Continue with Google",
                        true,
                        Some(google_icon),
                        clone!(error => move || {
                            spawn_local(clone!(error => async move {
                                match ApiCtx::get().client.auth_open_id_signin(OpenIdProvider::Google).await {
                                    Ok(resp) => {
                                        let _ = window().location().set_href(&resp.url);
                                    }
                                    Err(err) => error.set(Some(err.to_string())),
                                }
                            }));
                        }),
                    )],
                ),
            ]),
            auth_switch_note("Need an account?", Route::Register, "Create one"),
        ],
    )
}

fn render_register() -> Dom {
    let email = Arc::new(Mutex::new(String::new()));
    let password = Arc::new(Mutex::new(String::new()));
    let accept_terms = Mutable::new(false);
    let accept_privacy = Mutable::new(false);
    let accept_updates = Mutable::new(false);
    let error = Mutable::new(None::<String>);
    let legal_modal = Mutable::new(None::<LegalModalKind>);

    with_legal_modal(
        render_auth_page(
            AuthMode::Register,
            "Create your account",
            "",
            vec![
                message_block(error.clone(), "error"),
                auth_surface_row(vec![
                    auth_surface(
                        "Email registration",
                        "",
                        vec![
                            input_field(
                                "Email",
                                "email",
                                "email",
                                "email",
                                clone!(email => move |value| {
                                    *email.lock().unwrap() = value;
                                }),
                            ),
                            input_field(
                                "Password",
                                "password",
                                "password",
                                "new-password",
                                clone!(password => move |value| {
                                    *password.lock().unwrap() = value;
                                }),
                            ),
                            legal_checkbox_row(
                                accept_terms.clone(),
                                "I accept the",
                                "Terms of Service",
                                legal_modal.clone(),
                                LegalModalKind::TermsOfService,
                            ),
                            legal_checkbox_row(
                                accept_privacy.clone(),
                                "I accept the",
                                "Privacy Policy",
                                legal_modal.clone(),
                                LegalModalKind::PrivacyPolicy,
                            ),
                            checkbox_row(
                                "Send me optional product updates",
                                accept_updates.clone(),
                            ),
                            auth_button(
                                "Create account",
                                false,
                                None,
                                clone!(email, password, accept_terms, accept_privacy, accept_updates, error => move || {
                                    if !accept_terms.get() || !accept_privacy.get() {
                                        error.set(Some("You must accept the Terms of Service and Privacy Policy.".to_string()));
                                        return;
                                    }
                                    let email = email.lock().unwrap().clone();
                                    let password = password.lock().unwrap().clone();
                                    let consent = AuthRegistrationConsent {
                                        accept_terms_of_service: accept_terms.get(),
                                        accept_privacy_policy: accept_privacy.get(),
                                        opt_in_marketing_emails: accept_updates.get(),
                                    };
                                    spawn_local(clone!(error => async move {
                                        match ApiCtx::get().client.auth_email_password_register(email, password, consent).await {
                                            Ok(()) => {
                                                let _ = ApiCtx::refresh_profile().await;
                                                Route::Home.go_to_url();
                                            }
                                            Err(err) => error.set(Some(err.to_string())),
                                        }
                                    }));
                                }),
                            ),
                        ],
                    ),
                    auth_surface(
                        "Quick start",
                        "",
                        vec![auth_button(
                            "Register with Google",
                            true,
                            Some(google_icon),
                            clone!(error => move || {
                                spawn_local(clone!(error => async move {
                                    match ApiCtx::get().client.auth_open_id_signin(OpenIdProvider::Google).await {
                                        Ok(resp) => {
                                            let _ = window().location().set_href(&resp.url);
                                        }
                                        Err(err) => error.set(Some(err.to_string())),
                                    }
                                }));
                            }),
                        )],
                    ),
                ]),
                auth_switch_note("Already have an account?", Route::Signin, "Sign in"),
            ],
        ),
        legal_modal,
    )
}

fn render_verify_email() -> Dom {
    let notice = Mutable::new(None::<String>);
    let error = Mutable::new(None::<String>);

    auth_shell(
        "Verify Your Email",
        vec![
            plain_text("We need a verified email address before you can continue."),
            action_button(
                "Send verification email",
                clone!(notice, error => move || {
                    spawn_local(clone!(notice, error => async move {
                        match ApiCtx::get().client.auth_email_send_verify().await {
                            Ok(()) => notice.set(Some("Verification email sent. Check your inbox.".to_string())),
                            Err(err) => error.set(Some(err.to_string())),
                        }
                    }));
                }),
            ),
            message_block(notice, "success"),
            message_block(error, "error"),
            action_button("Sign out", move || {
                ApiCtx::sign_out();
            }),
        ],
    )
}

fn render_verify_email_confirm(token: AuthToken) -> Dom {
    let status = Mutable::new("Verifying your email…".to_string());
    let status_future = status.clone();
    html!("div", {
        .future(async move {
            let result = ApiCtx::get().client.auth_email_confirm(token).await;
            match result {
                Ok(()) => {
                    let _ = ApiCtx::refresh_profile().await;
                    Route::Home.go_to_url();
                }
                Err(err) => status_future.set(format!("Failed to verify email: {err}")),
            }
        })
        .child(auth_shell("Verify Email", vec![plain_text_signal(status)]))
    })
}

fn render_reset_password(token: AuthToken) -> Dom {
    let password = Arc::new(Mutex::new(String::new()));
    let error = Mutable::new(None::<String>);

    auth_shell(
        "Reset Password",
        vec![
            input_field(
                "New password",
                "password",
                "password",
                "new-password",
                clone!(password => move |value| {
                    *password.lock().unwrap() = value;
                }),
            ),
            action_button(
                "Reset password",
                clone!(password, error => move || {
                    let password = password.lock().unwrap().clone();
                    let token = token.clone();
                    spawn_local(clone!(error => async move {
                        match ApiCtx::get().client.auth_email_password_confirm_reset(token.clone(), password).await {
                            Ok(()) => {
                                let _ = ApiCtx::refresh_profile().await;
                                Route::Home.go_to_url();
                            }
                            Err(err) => error.set(Some(err.to_string())),
                        }
                    }));
                }),
            ),
            message_block(error, "error"),
        ],
    )
}

fn render_choose_username() -> Dom {
    let username = Mutable::new(String::new());
    let status = Mutable::new(UsernameStatus::Empty);
    let error = Mutable::new(None::<String>);
    let generation = Mutable::new(0_u32);

    render_auth_page(
        AuthMode::Register,
        "Choose your username",
        "",
        vec![
            message_block(error.clone(), "error"),
            auth_surface_row(vec![auth_surface(
                "Username",
                "",
                vec![
                    input_field(
                        "Username",
                        "username",
                        "text",
                        "username",
                        clone!(username, status, generation => move |value| {
                            username.set(value.clone());
                            schedule_username_check(value, status.clone(), generation.clone());
                        }),
                    ),
                    username_status_block(status.clone()),
                    auth_button(
                        "Continue",
                        false,
                        None,
                        clone!(username, status, error => move || {
                            let username = username.get_cloned();
                            if username.is_empty() {
                                error.set(Some("Please enter a username.".to_string()));
                                return;
                            }
                            if status.get_cloned() != UsernameStatus::Available {
                                error.set(Some("Please choose an available username before continuing.".to_string()));
                                return;
                            }
                            spawn_local(clone!(error => async move {
                                match ApiCtx::get().client.account_username_update(username).await {
                                    Ok(_) => {
                                        let _ = ApiCtx::refresh_profile().await;
                                        Route::Home.go_to_url();
                                    }
                                    Err(err) => error.set(Some(err.to_string())),
                                }
                            }));
                        }),
                    ),
                ],
            )]),
        ],
    )
}

fn render_openid_finalize(token: AuthToken) -> Dom {
    let status = Mutable::new("Checking Google sign-in…".to_string());
    let error = Mutable::new(None::<String>);
    let accept_terms = Mutable::new(false);
    let accept_privacy = Mutable::new(false);
    let accept_updates = Mutable::new(false);
    let needs_registration = Mutable::new(false);
    let query_token = token.clone();
    let legal_modal = Mutable::new(None::<LegalModalKind>);

    with_legal_modal(
        html!("div", {
            .future(clone!(status, needs_registration, error => async move {
                match ApiCtx::get().client.auth_open_id_finalize_query(query_token.clone()).await {
                    Ok(AuthOpenIdFinalizeQueryResponse::UserExists { .. }) => {
                        match ApiCtx::get().client.auth_open_id_finalize_exec(query_token.clone(), None).await {
                            Ok(()) => {
                                let _ = ApiCtx::refresh_profile().await;
                                Route::Home.go_to_url();
                            }
                            Err(err) => error.set(Some(err.to_string())),
                        }
                    }
                    Ok(AuthOpenIdFinalizeQueryResponse::UserDoesNotExist) => {
                        needs_registration.set(true);
                        status.set("Complete registration to continue.".to_string());
                    }
                    Err(err) => error.set(Some(err.to_string())),
                }
            }))
            .child_signal(needs_registration.signal().map(clone!(status, accept_terms, accept_privacy, accept_updates, error, legal_modal => move |needs_registration| {
                Some(if needs_registration {
                    auth_shell(
                        "Complete Google Registration",
                        vec![
                            plain_text_signal(status.clone()),
                            legal_checkbox_row(
                                accept_terms.clone(),
                                "I accept the",
                                "Terms of Service",
                                legal_modal.clone(),
                                LegalModalKind::TermsOfService,
                            ),
                            legal_checkbox_row(
                                accept_privacy.clone(),
                                "I accept the",
                                "Privacy Policy",
                                legal_modal.clone(),
                                LegalModalKind::PrivacyPolicy,
                            ),
                            checkbox_row("Send me optional product updates", accept_updates.clone()),
                            action_button("Complete registration", clone!(accept_terms, accept_privacy, accept_updates, error => {
                                let token = token.clone();
                                move || {
                                if !accept_terms.get() || !accept_privacy.get() {
                                    error.set(Some("You must accept the Terms of Service and Privacy Policy.".to_string()));
                                    return;
                                }
                                let consent = AuthRegistrationConsent {
                                    accept_terms_of_service: accept_terms.get(),
                                    accept_privacy_policy: accept_privacy.get(),
                                    opt_in_marketing_emails: accept_updates.get(),
                                };
                                let token_for_exec = token.clone();
                                spawn_local(clone!(error => async move {
                                    match ApiCtx::get().client.auth_open_id_finalize_exec(token_for_exec.clone(), Some(consent)).await {
                                        Ok(()) => {
                                            let _ = ApiCtx::refresh_profile().await;
                                            Route::Home.go_to_url();
                                        }
                                        Err(err) => error.set(Some(err.to_string())),
                                    }
                                }));
                            }})),
                            message_block(error.clone(), "error"),
                        ],
                    )
                } else {
                    auth_shell("Google Sign-In", vec![plain_text_signal(status.clone()), message_block(error.clone(), "error")])
                })
            })))
        }),
        legal_modal,
    )
}

fn render_profile() -> Dom {
    let profile = ApiCtx::get()
        .profile
        .get_cloned()
        .expect("profile required");
    let username = Arc::new(Mutex::new(profile.username.to_string()));
    let full_name = Arc::new(Mutex::new(profile.full_name.clone()));
    let line1 = Arc::new(Mutex::new(profile.shipping_address.line1.clone()));
    let line2 = Arc::new(Mutex::new(profile.shipping_address.line2.clone()));
    let city = Arc::new(Mutex::new(profile.shipping_address.city.clone()));
    let state = Arc::new(Mutex::new(profile.shipping_address.state.clone()));
    let postal_code = Arc::new(Mutex::new(profile.shipping_address.postal_code.clone()));
    let country = Arc::new(Mutex::new(profile.shipping_address.country.clone()));
    let marketing = Mutable::new(profile.receive_marketing);
    let message = Mutable::new(None::<String>);
    let error = Mutable::new(None::<String>);

    auth_shell(
        "My Profile",
        vec![
            plain_text(&format!("Signed in as {}", profile.email)),
            input_field(
                "Username",
                "username",
                "text",
                "username",
                clone!(username => move |value| {
                    *username.lock().unwrap() = value;
                }),
            ),
            input_field(
                "Full name",
                "full_name",
                "text",
                "name",
                clone!(full_name => move |value| {
                    *full_name.lock().unwrap() = value;
                }),
            ),
            input_field(
                "Shipping line 1",
                "shipping_line1",
                "text",
                "address-line1",
                clone!(line1 => move |value| {
                    *line1.lock().unwrap() = value;
                }),
            ),
            input_field(
                "Shipping line 2",
                "shipping_line2",
                "text",
                "address-line2",
                clone!(line2 => move |value| {
                    *line2.lock().unwrap() = value;
                }),
            ),
            input_field(
                "City",
                "shipping_city",
                "text",
                "address-level2",
                clone!(city => move |value| {
                    *city.lock().unwrap() = value;
                }),
            ),
            input_field(
                "State / Region",
                "shipping_state",
                "text",
                "address-level1",
                clone!(state => move |value| {
                    *state.lock().unwrap() = value;
                }),
            ),
            input_field(
                "Postal code",
                "shipping_postal_code",
                "text",
                "postal-code",
                clone!(postal_code => move |value| {
                    *postal_code.lock().unwrap() = value;
                }),
            ),
            input_field(
                "Country",
                "shipping_country",
                "text",
                "country-name",
                clone!(country => move |value| {
                    *country.lock().unwrap() = value;
                }),
            ),
            checkbox_row("Receive optional product updates", marketing.clone()),
            action_button(
                "Save profile",
                clone!(username, full_name, line1, line2, city, state, postal_code, country, marketing, message, error => move || {
                    let current_username = username.lock().unwrap().clone();
                    let req = AccountProfileUpdateRequest {
                        full_name: full_name.lock().unwrap().clone(),
                        shipping_address: ShippingAddress {
                            line1: line1.lock().unwrap().clone(),
                            line2: line2.lock().unwrap().clone(),
                            city: city.lock().unwrap().clone(),
                            state: state.lock().unwrap().clone(),
                            postal_code: postal_code.lock().unwrap().clone(),
                            country: country.lock().unwrap().clone(),
                        },
                        receive_marketing: marketing.get(),
                    };
                    let old_username = profile.username.to_string();
                    spawn_local(clone!(message, error => async move {
                        if current_username != old_username {
                            if let Err(err) = ApiCtx::get().client.account_username_update(current_username).await {
                                error.set(Some(err.to_string()));
                                return;
                            }
                        }
                        match ApiCtx::get().client.account_profile_update(&req).await {
                            Ok(()) => {
                                let _ = ApiCtx::refresh_profile().await;
                                message.set(Some("Profile saved.".to_string()));
                            }
                            Err(err) => error.set(Some(err.to_string())),
                        }
                    }));
                }),
            ),
            message_block(message, "success"),
            message_block(error, "error"),
        ],
    )
}

fn render_orders() -> Dom {
    let orders: Mutable<Option<Vec<AccountOrderSummary>>> = Mutable::new(None);

    // Polling loop: fetch immediately, then refresh on the configured
    // cadence so the threshold bars and pool status stay live without
    // user interaction.
    spawn_local(clone!(orders => async move {
        loop {
            match ApiCtx::get().client.account_orders().await {
                Ok(res) => orders.set(Some(res.orders)),
                Err(_) => {
                    if orders.get_cloned().is_none() {
                        orders.set(Some(Vec::new()));
                    }
                }
            }
            gloo_timers::future::TimeoutFuture::new(config::REFRESH_THRESHHOLD_VIEW_MS).await;
        }
    }));

    html!("div", {
        .class(&*chrome::SHELL)
        .child(site_header())
        .child(html!("section", {
            .style("margin-top", "2.5rem")
            .child(html!("div", {
                .class(&*chrome::SECTION_HEADER)
                .children([
                    section_title("My Orders"),
                ])
            }))
            .child_signal(orders.signal_cloned().map(|state| {
                Some(match state {
                    None => html!("p", {
                        .class(&*typography::BODY_MUTED)
                        .text("Loading your orders...")
                    }),
                    Some(orders) if orders.is_empty() => html!("div", {
                        .class(&*chrome::CARD)
                        .style("text-align", "center")
                        .style("padding", "3rem")
                        .children([
                            html!("p", {
                                .class(&*typography::LEAD_TEXT)
                                .text("No orders yet.")
                            }),
                            html!("p", {
                                .class(&*typography::BODY_MUTED)
                                .text("Browse the deals on the home page and join one to get started.")
                            }),
                        ])
                    }),
                    Some(orders) => render_orders_split(orders),
                })
            }))
        }))
        .child(site_footer())
    })
}

/// Splits orders into "Current" (`pipeline_status == Open` and not
/// refunded) vs "History" (everything else). The on-chain pool's
/// auto-lock advances the cron-driven pipeline status, so a row moves
/// from Current → History within a minute of the threshold being met.
fn render_orders_split(orders: Vec<AccountOrderSummary>) -> Dom {
    let mut current: Vec<AccountOrderSummary> = Vec::new();
    let mut history: Vec<AccountOrderSummary> = Vec::new();
    for o in orders {
        let active = matches!(o.pipeline_status, BatchPipelineStatus::Open) && !o.refunded;
        if active {
            current.push(o);
        } else {
            history.push(o);
        }
    }

    html!("div", {
        .child(orders_section("Current orders", &current, true))
        .child(orders_section("Order history", &history, false))
    })
}

fn pipeline_status_label(status: BatchPipelineStatus, refunded: bool) -> &'static str {
    if refunded {
        return "Refunded";
    }
    match status {
        BatchPipelineStatus::Open => "Recruiting",
        BatchPipelineStatus::Locked => "Locked",
        BatchPipelineStatus::ShippingToDistributor => "Shipping to distributor",
        BatchPipelineStatus::ShippingIndividually => "Shipping to you",
        BatchPipelineStatus::Released => "Released",
        BatchPipelineStatus::Refunding => "Refund mode",
    }
}

fn orders_section(title: &str, items: &[AccountOrderSummary], current_section: bool) -> Dom {
    html!("div", {
        .style("margin-top", "1.5rem")
        .child(html!("h3", {
            .class(&*typography::MINI_HEADING)
            .style("margin-bottom", "0.75rem")
            .text(title)
        }))
        .apply_if(items.is_empty(), |dom| {
            dom.child(html!("p", {
                .class(&*typography::BODY_MUTED)
                .style("font-size", "0.85rem")
                .text(if current_section { "No active group deals." } else { "No completed orders yet." })
            }))
        })
        .apply_if(!items.is_empty(), |dom| {
            let cards: Vec<Dom> = items.iter().cloned().map(|o| order_card(o, current_section)).collect();
            dom.child(html!("div", {
                .style("display", "grid")
                .style("grid-template-columns", "1fr")
                .style("gap", "0.75rem")
                .children(cards)
            }))
        })
    })
}

fn order_card(order: AccountOrderSummary, current_section: bool) -> Dom {
    let unit_price = format!("${:.2}", order.price_cents as f64 / 100.0);
    // product_amount_base_units uses USDC decimals (6); divide by 1e6 for
    // a human-readable USDC figure.
    let total = format!(
        "${:.2}",
        order.product_amount_base_units as f64 / 1_000_000.0
    );
    let status_label = pipeline_status_label(order.pipeline_status, order.refunded);
    let link = Route::Product {
        id: order.product_id.clone(),
    }
    .link();
    let batch_label = format!("Batch #{}", order.batch_id);
    // Withdraw is only offered while the deal is still recruiting AND
    // the user hasn't already refunded — once the on-chain pool locks
    // (auto at threshold), `SelfRefund` is rejected.
    let can_withdraw = current_section && !order.refunded;
    let order_for_summary = order.clone();
    let order_for_progress = order.clone();
    let order_for_button = order;

    html!("div", {
        .class(&*chrome::CARD)
        .style("display", "grid")
        .style("grid-template-columns", "auto 1fr auto")
        .style("gap", "1rem")
        .style("align-items", "center")
        .child(html!("a", {
            .attr("href", &link)
            .style("text-decoration", "none")
            .style("color", "inherit")
            .style("width", "60px")
            .style("height", "60px")
            .style("border-radius", "0.5rem")
            .style("background", "#f3f4f6")
            .style("overflow", "hidden")
            .style("display", "block")
            .apply_if(!order_for_summary.product_image_url.is_empty(), {
                let url = order_for_summary.product_image_url.clone();
                let name = order_for_summary.product_name.clone();
                move |dom| {
                    dom.child(html!("img", {
                        .attr("src", &url)
                        .attr("alt", &name)
                        .style("width", "100%")
                        .style("height", "100%")
                        .style("object-fit", "contain")
                    }))
                }
            })
        }))
        .child(html!("div", {
            .child(html!("a", {
                .class(&*typography::CARD_TITLE)
                .attr("href", &link)
                .style("text-decoration", "none")
                .style("color", "inherit")
                .text(&order_for_summary.product_name)
            }))
            .child(html!("div", {
                .style("font-size", "0.78rem")
                .style("color", "#6b7280")
                .style("margin-top", "0.2rem")
                .text(&format!("{batch_label} \u{00b7} Unit {unit_price} \u{00b7} You committed {total}"))
            }))
            .apply_if(current_section, move |dom| {
                dom.child(html!("div", {
                    .style("margin-top", "0.6rem")
                    .child(threshold::progress(
                        order_for_progress.committed_units,
                        order_for_progress.minimum_order_quantity,
                        true,
                    ))
                }))
            })
        }))
        .child(html!("div", {
            .style("display", "flex")
            .style("flex-direction", "column")
            .style("align-items", "flex-end")
            .style("gap", "0.5rem")
            .style("white-space", "nowrap")
            .child(html!("div", {
                .style("font-size", "0.72rem")
                .style("color", if order_for_button.refunded { "#b45309" } else { "#6b7280" })
                .style("font-weight", if order_for_button.refunded { "600" } else { "500" })
                .text(status_label)
            }))
            .apply_if(can_withdraw, move |dom| {
                dom.child(withdraw_button(
                    order_for_button.product_id.clone(),
                    order_for_button.batch_id,
                ))
            })
        }))
    })
}

/// Buyer-initiated refund affordance. Disabled while the request is in
/// flight; on success the page reloads so the row moves over to History
/// with the Refunded label.
fn withdraw_button(product_id: groupshop_backend_shared::prelude::ProductId, batch_id: u32) -> Dom {
    let working = Mutable::new(false);
    let error = Mutable::new(None::<String>);
    html!("div", {
        .style("display", "flex")
        .style("flex-direction", "column")
        .style("align-items", "flex-end")
        .style("gap", "0.3rem")
        .child(html!("button", {
            .attr("type", "button")
            .style("font-size", "0.78rem")
            .style("padding", "0.4rem 0.8rem")
            .style("border-radius", "0.5rem")
            .style("border", "1px solid #b91c1c")
            .style("background", "#fff")
            .style("color", "#b91c1c")
            .style("cursor", "pointer")
            .style_signal("opacity", working.signal().map(|busy| {
                if busy { "0.6".to_string() } else { "1".to_string() }
            }))
            .prop_signal("disabled", working.signal())
            .text_signal(working.signal().map(|busy| {
                if busy { "Withdrawing\u{2026}".to_string() } else { "Withdraw".to_string() }
            }))
            .event(clone!(working, error, product_id => move |_: events::Click| {
                if working.get() { return; }
                working.set(true);
                error.set(None);
                spawn_local(clone!(working, error, product_id => async move {
                    match wallet::run_self_refund(product_id, batch_id).await {
                        Ok(_) => {
                            // Reload so the orders list re-fetches and the
                            // row moves over to History with the Refunded
                            // label. Covers both fresh refunds and the
                            // self-healed "already refunded" reconcile case.
                            let _ = groupshop_frontend_shared::window().location().reload();
                        }
                        Err(err) => {
                            error.set(Some(err.to_string()));
                            working.set(false);
                        }
                    }
                }));
            }))
        }))
        .child(html!("p", {
            .style("font-size", "0.7rem")
            .style("color", "#b91c1c")
            .style("max-width", "16rem")
            .style("text-align", "right")
            .style_signal("display", error.signal_cloned().map(|v| {
                if v.is_some() { "block".to_string() } else { "none".to_string() }
            }))
            .text_signal(error.signal_cloned().map(|v| v.unwrap_or_default()))
        }))
    })
}

fn render_error(err: Arc<ApiError>) -> Dom {
    auth_shell(
        "Error",
        vec![
            plain_text(&err.to_string()),
            action_link(Route::Home, "Back to home"),
        ],
    )
}

fn render_not_found() -> Dom {
    auth_shell(
        "Page Not Found",
        vec![
            plain_text("The page you requested does not exist."),
            action_link(Route::Home, "Back to home"),
        ],
    )
}

/// Singleton state for the "How It Works" modal. The modal can be triggered
/// from the nav on any page, but its DOM lives once at the document level so
/// only one instance is ever active.
fn how_it_works_modal_state() -> &'static Mutable<bool> {
    static STATE: OnceLock<Mutable<bool>> = OnceLock::new();
    STATE.get_or_init(|| Mutable::new(false))
}

fn beta_banner() -> Dom {
    html!("div", {
        .style("background", "#0d3832")
        .style("color", "#e8f5f3")
        .style("text-align", "center")
        .style("padding", "0.5rem 1rem")
        .style("font-size", "0.82rem")
        .style("font-weight", "600")
        .style("letter-spacing", "0.03em")
        .text("Currently in beta for Colosseum Frontier Hackathon \u{2014} devnet only")
    })
}

fn site_header() -> Dom {
    let profile = ApiCtx::get().profile.get_cloned();
    html!("div", {
        // Wrapping div so we can render the How It Works modal as a sibling
        // of the actual <header>. The modal is positioned: fixed, so its
        // physical place in the tree doesn't matter visually.
        .child(beta_banner())
        .child(site_header_inner(profile))
        .child_signal(how_it_works_modal_state().signal().map(|open| {
            if open { Some(render_how_it_works_modal()) } else { None }
        }))
    })
}

fn site_header_inner(profile: Option<AccountProfile>) -> Dom {
    html!("header", {
        .class(&*chrome::MASTHEAD)
        .child(html!("a", {
            .class(&*chrome::BRAND)
            .attr("href", "/")
            .attr("aria-label", "Groupshop home")
            .child(html!("img", {
                .class(&*chrome::LOGO_IMG)
                .attr("src", &config::media_link("logo-icon.png"))
                .attr("alt", "")
            }))
            .child(html!("span", {
                .class(&*chrome::BRAND_WORD)
                .text("Groupshop")
            }))
        }))
        .child(html!("nav", {
            .class(&*chrome::NAV)
            .children([
                // "How It Works" opens an explainer modal — see
                // `how_it_works_modal_state()`. Rendered as a button so
                // the click handler fires without a page navigation; the
                // <a href> form was a section anchor that no longer
                // matches the modal-driven content.
                html!("button", {
                    .class(&*chrome::NAV_LINK)
                    .class(&*typography::NAV_LABEL)
                    .style("background", "transparent")
                    .style("border", "0")
                    .style("cursor", "pointer")
                    .text("How It Works")
                    .event(|_: events::Click| {
                        how_it_works_modal_state().set(true);
                    })
                }),
                html!("a", {
                    .class(&*chrome::NAV_LINK)
                    .class(&*typography::NAV_LABEL)
                    .attr("href", "/help")
                    .text("Getting Started")
                }),
            ])
        }))
        .child(html!("div", {
            .class(&*chrome::NAV_RIGHT)
            .child(match profile {
                Some(profile) => account_menu(profile),
                None => signed_out_cta(),
            })
        }))
    })
}

fn site_footer() -> Dom {
    html!("footer", {
        .class(&*chrome::FOOTER)
        .child(html!("p", {
            .class(&*typography::FOOTER_TEXT)
            .children([
                html!("span", {
                    .text("Real-world group buying, secured on-chain. ")
                }),
                html!("a", {
                    .class(&*chrome::FOOTER_LINK)
                    .attr("href", "/help")
                    .text("Getting Started")
                }),
                html!("span", {
                    .text(" · ")
                }),
                html!("a", {
                    .class(&*chrome::FOOTER_LINK)
                    .attr("href", "/privacy-policy")
                    .text("Privacy Policy")
                }),
                html!("span", {
                    .text(" · ")
                }),
                html!("a", {
                    .class(&*chrome::FOOTER_LINK)
                    .attr("href", "/terms-of-service")
                    .text("Terms of Service")
                }),
            ])
        }))
    })
}

fn account_menu(profile: AccountProfile) -> Dom {
    html!("details" => web_sys::HtmlElement, {
        .style("position", "relative")
        .style("z-index", "40")
        .after_inserted(|details| {
            let details = details.clone();
            let closure = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(
                move |event: web_sys::Event| {
                    let Some(target_node) = event
                        .target()
                        .and_then(|target| target.dyn_into::<web_sys::Node>().ok())
                    else {
                        return;
                    };
                    if !details.contains(Some(&target_node)) {
                        let _ = details.remove_attribute("open");
                    }
                },
            ));
            let _ = document().add_event_listener_with_callback(
                "click",
                closure.as_ref().unchecked_ref(),
            );
            closure.forget();
        })
        .child(html!("summary", {
            .style("display", "inline-flex")
            .style("align-items", "center")
            .style("justify-content", "center")
            .style("gap", "0.55rem")
            .style("min-height", "2.9rem")
            .style("padding", "0.72rem 1rem")
            .style("border-radius", "999px")
            .style("border", "1px solid rgba(13,56,50,0.14)")
            .style("background", "rgba(255,255,255,0.98)")
            .style("box-shadow", "0 10px 24px rgba(9, 35, 31, 0.06)")
            .style("cursor", "pointer")
            .style("user-select", "none")
            .style("list-style", "none")
            .child(menu_icon())
            .child(html!("span", {
                .style("font-size", "0.78rem")
                .style("font-weight", "700")
                .style("letter-spacing", "0.06em")
                .style("text-transform", "uppercase")
                .style("color", "#12312d")
                .text("Menu")
            }))
        }))
        .child({
            let mut items = vec![
                menu_link_item(Route::Profile, "Account"),
                menu_link_item(Route::Orders, "My Orders"),
            ];
            if profile.roles.contains(&UserRole::Admin) {
                items.push(menu_external_link_item(config::admin_url(), "Admin"));
            }
            items.push(menu_action_item("Sign out", || {
                ApiCtx::sign_out();
            }));

            html!("div", {
                .style("position", "absolute")
                .style("top", "calc(100% + 0.6rem)")
                .style("right", "0")
                .style("display", "grid")
                .style("gap", "0.18rem")
                .style("min-width", "10.5rem")
                .style("padding", "0.45rem")
                .style("border-radius", "1rem")
                .style("border", "1px solid rgba(13,56,50,0.08)")
                .style("background", "rgba(255,255,255,0.98)")
                .style("box-shadow", "0 22px 44px rgba(11, 31, 28, 0.12)")
                .children(items)
            })
        })
    })
}

fn menu_icon() -> Dom {
    svg!("svg", {
        .attr("viewBox", "0 0 20 20")
        .attr("width", "18")
        .attr("height", "18")
        .attr("aria-hidden", "true")
        .children([
            svg!("path", {
                .attr("d", "M4 5.5H16M4 10H16M4 14.5H16")
                .attr("fill", "none")
                .attr("stroke", "#17443f")
                .attr("stroke-width", "1.8")
                .attr("stroke-linecap", "round")
            }),
        ])
    })
}

fn menu_link_item(route: Route, label: &'static str) -> Dom {
    html!("a", {
        .class(&*chrome::BUTTON)
        .class(&*chrome::BUTTON_SOFT)
        .attr("href", &route.link())
        .style("width", "100%")
        .style("min-height", "2.6rem")
        .style("padding", "0.72rem 1rem")
        .style("border-radius", "0.72rem")
        .style("font-size", "1rem")
        .text(label)
    })
}

fn menu_external_link_item(href: &str, label: &'static str) -> Dom {
    let href = href.to_string();
    html!("a", {
        .class(&*chrome::BUTTON)
        .class(&*chrome::BUTTON_SOFT)
        .attr("href", &href)
        .style("width", "100%")
        .style("min-height", "2.6rem")
        .style("padding", "0.72rem 1rem")
        .style("border-radius", "0.72rem")
        .style("font-size", "1rem")
        .text(label)
    })
}

fn menu_action_item(label: &'static str, on_click: impl Fn() + 'static) -> Dom {
    html!("button", {
        .class(&*chrome::BUTTON)
        .class(&*chrome::BUTTON_SOFT)
        .style("width", "100%")
        .style("min-height", "2.6rem")
        .style("padding", "0.72rem 1rem")
        .style("border-radius", "0.72rem")
        .style("font-size", "1rem")
        .text(label)
        .event(move |_: events::Click| on_click())
    })
}

fn action_link(route: Route, text: &'static str) -> Dom {
    html!("a", {
        .class(&*chrome::BUTTON)
        .class(&*chrome::BUTTON_SOFT)
        .attr("href", &route.link())
        .style("min-height", "3rem")
        .style("padding", "0.82rem 1.2rem")
        .style("border-radius", "999px")
        .text(text)
    })
}

fn signed_out_cta() -> Dom {
    html!("a", {
        .class(&*chrome::BUTTON)
        .attr("href", &Route::Signin.link())
        .style("min-height", "3rem")
        .style("padding", "0.82rem 1.2rem")
        .style("border-radius", "999px")
        .style("font-weight", "700")
        .style("border", "1px solid rgba(13,56,50,0.18)")
        .style("background", "rgba(255,255,255,0.98)")
        .style("color", "#12312d")
        .text("Sign in / Register")
    })
}

fn action_button(label: &'static str, on_click: impl Fn() + 'static) -> Dom {
    html!("button", {
        .class(&*chrome::BUTTON)
        .class(&*chrome::BUTTON_SOFT)
        .style("min-height", "3rem")
        .style("padding", "0.82rem 1.2rem")
        .style("border-radius", "999px")
        .text(label)
        .event(move |_: events::Click| on_click())
    })
}

fn eye_icon(visible: bool) -> Dom {
    if visible {
        svg!("svg", {
            .attr("viewBox", "0 0 24 24")
            .attr("width", "18")
            .attr("height", "18")
            .attr("fill", "none")
            .attr("aria-hidden", "true")
            .children([
                svg!("path", {
                    .attr("stroke", "currentColor")
                    .attr("stroke-width", "2")
                    .attr("stroke-linecap", "round")
                    .attr("d", "M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24")
                }),
                svg!("line", {
                    .attr("x1", "1")
                    .attr("y1", "1")
                    .attr("x2", "23")
                    .attr("y2", "23")
                    .attr("stroke", "currentColor")
                    .attr("stroke-width", "2")
                    .attr("stroke-linecap", "round")
                }),
            ])
        })
    } else {
        svg!("svg", {
            .attr("viewBox", "0 0 24 24")
            .attr("width", "18")
            .attr("height", "18")
            .attr("fill", "none")
            .attr("aria-hidden", "true")
            .children([
                svg!("path", {
                    .attr("stroke", "currentColor")
                    .attr("stroke-width", "2")
                    .attr("stroke-linecap", "round")
                    .attr("stroke-linejoin", "round")
                    .attr("d", "M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z")
                }),
                svg!("circle", {
                    .attr("cx", "12")
                    .attr("cy", "12")
                    .attr("r", "3")
                    .attr("stroke", "currentColor")
                    .attr("stroke-width", "2")
                }),
            ])
        })
    }
}

fn input_field(
    label: &'static str,
    name: &'static str,
    input_type: &'static str,
    autocomplete: &'static str,
    on_input: impl Fn(String) + 'static,
) -> Dom {
    let label_span = html!("span", {
        .style("font-size", "0.82rem")
        .style("font-weight", "700")
        .style("letter-spacing", "0.03em")
        .style("text-transform", "uppercase")
        .style("color", "rgba(10, 18, 17, 0.74)")
        .text(label)
    });

    let input_child = if input_type == "password" {
        let show = Mutable::new(false);
        html!("div", {
            .style("position", "relative")
            .child(html!("input" => web_sys::HtmlInputElement, {
                .attr("name", name)
                .attr_signal("type", show.signal().map(|s| if s { "text" } else { "password" }))
                .attr("autocomplete", autocomplete)
                .style("width", "100%")
                .style("padding", "0.98rem 3rem 0.98rem 1rem")
                .style("border-radius", "1rem")
                .style("border", "1px solid rgba(13, 56, 50, 0.2)")
                .style("background", "rgba(255,255,255,0.96)")
                .style("color", "#102120")
                .style("box-sizing", "border-box")
                .style("outline", "none")
                .style("font-size", "1rem")
                .style("box-shadow", "0 1px 0 rgba(255,255,255,0.55) inset")
                .event(move |evt: events::Input| {
                    let value = evt
                        .target()
                        .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .map(|input| input.value())
                        .unwrap_or_default();
                    on_input(value);
                })
            }))
            .child(html!("button", {
                .attr("type", "button")
                .attr("aria-label", "Toggle password visibility")
                .style("position", "absolute")
                .style("right", "0.75rem")
                .style("top", "50%")
                .style("transform", "translateY(-50%)")
                .style("background", "none")
                .style("border", "none")
                .style("cursor", "pointer")
                .style("padding", "0.25rem")
                .style("display", "flex")
                .style("align-items", "center")
                .style("color", "rgba(13, 56, 50, 0.5)")
                .child_signal(show.signal().map(|s| Some(eye_icon(s))))
                .event(clone!(show => move |_: events::Click| {
                    show.set(!show.get());
                }))
            }))
        })
    } else {
        html!("input" => web_sys::HtmlInputElement, {
            .attr("name", name)
            .attr("type", input_type)
            .attr("autocomplete", autocomplete)
            .style("width", "100%")
            .style("padding", "0.98rem 1rem")
            .style("border-radius", "1rem")
            .style("border", "1px solid rgba(13, 56, 50, 0.2)")
            .style("background", "rgba(255,255,255,0.96)")
            .style("color", "#102120")
            .style("box-sizing", "border-box")
            .style("outline", "none")
            .style("font-size", "1rem")
            .style("box-shadow", "0 1px 0 rgba(255,255,255,0.55) inset")
            .event(move |evt: events::Input| {
                let value = evt
                    .target()
                    .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                    .map(|input| input.value())
                    .unwrap_or_default();
                on_input(value);
            })
        })
    };

    html!("label", {
        .style("display", "grid")
        .style("gap", "0.45rem")
        .child(label_span)
        .child(input_child)
    })
}

fn checkbox_row(label: &'static str, value: Mutable<bool>) -> Dom {
    checkbox_children_row(
        value,
        vec![html!("span", {
            .style("font-size", "0.98rem")
            .style("line-height", "1.45")
            .style("color", "rgba(13, 25, 24, 0.84)")
            .text(label)
        })],
    )
}

fn legal_checkbox_row(
    value: Mutable<bool>,
    prefix: &'static str,
    link_label: &'static str,
    legal_modal: Mutable<Option<LegalModalKind>>,
    kind: LegalModalKind,
) -> Dom {
    html!("div", {
        .style("display", "flex")
        .style("gap", "0.6rem")
        .style("align-items", "flex-start")
        .style("padding", "0.85rem 0.95rem")
        .style("border-radius", "1rem")
        .style("border", "1px solid rgba(13, 56, 50, 0.12)")
        .style("background", "rgba(255,255,255,0.78)")
        .style("cursor", "pointer")
        .style("user-select", "none")
        .event(clone!(value => move |_: events::Click| {
            value.set(!value.get());
        }))
        .child(html!("input" => web_sys::HtmlInputElement, {
            .attr("type", "checkbox")
            .style("margin-top", "0.2rem")
            .style("inline-size", "1rem")
            .style("block-size", "1rem")
            .prop_signal("checked", value.signal())
            .event(|evt: events::Click| {
                evt.stop_propagation();
            })
            .event(clone!(value => move |evt: events::Change| {
                let checked = evt
                    .target()
                    .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                    .map(|input| input.checked())
                    .unwrap_or(false);
                value.set(checked);
            }))
        }))
        .child(html!("div", {
            .style("display", "flex")
            .style("flex-wrap", "wrap")
            .style("gap", "0.35rem")
            .style("align-items", "center")
            .children([
                html!("span", {
                    .style("font-size", "0.98rem")
                    .style("line-height", "1.45")
                    .style("color", "rgba(13, 25, 24, 0.84)")
                    .text(prefix)
                }),
                html!("span", {
                    .style("font-size", "0.98rem")
                    .style("font-weight", "700")
                    .style("color", "#0c6e5c")
                    .style("cursor", "pointer")
                    .style("user-select", "text")
                    .text(link_label)
                    .event(move |evt: events::Click| {
                        evt.stop_propagation();
                        legal_modal.set(Some(kind));
                    })
                }),
            ])
        }))
    })
}

fn with_legal_modal(content: Dom, legal_modal: Mutable<Option<LegalModalKind>>) -> Dom {
    html!("div", {
        .child(content)
        .child_signal(legal_modal.signal_cloned().map(clone!(legal_modal => move |kind| {
            kind.map(|kind| render_legal_modal(kind, legal_modal.clone()))
        })))
    })
}

fn render_legal_modal(kind: LegalModalKind, legal_modal: Mutable<Option<LegalModalKind>>) -> Dom {
    let page = match kind {
        LegalModalKind::TermsOfService => terms_of_service(),
        LegalModalKind::PrivacyPolicy => privacy_policy(),
    };

    html!("div", {
        .style("position", "fixed")
        .style("inset", "0")
        .style("z-index", "1000")
        .style("display", "flex")
        .style("align-items", "center")
        .style("justify-content", "center")
        .style("padding", "1.25rem")
        .child(html!("div", {
            .style("position", "absolute")
            .style("inset", "0")
            .style("background", "rgba(7, 18, 17, 0.6)")
            .event(clone!(legal_modal => move |_: events::Click| {
                legal_modal.set(None);
            }))
        }))
        .child(html!("div", {
            .style("position", "relative")
            .style("z-index", "1")
            .style("display", "grid")
            .style("grid-template-rows", "auto minmax(0, 1fr)")
            .style("width", "min(56rem, 100%)")
            .style("max-height", "min(86vh, 54rem)")
            .style("overflow", "hidden")
            .style("padding", "1.2rem 1.2rem 1rem 1.2rem")
            .style("border-radius", "1.35rem")
            .style("background", "rgba(255,255,255,0.98)")
            .style("box-shadow", "0 32px 80px rgba(0, 0, 0, 0.22)")
            .children([
                html!("div", {
                    .style("display", "flex")
                    .style("justify-content", "space-between")
                    .style("align-items", "flex-start")
                    .style("gap", "1rem")
                    .style("padding-bottom", "0.9rem")
                    .style("border-bottom", "1px solid rgba(13,56,50,0.08)")
                    .children([
                        html!("div", {
                            .child(html!("h2", {
                                .style("margin", "0")
                                .style("font-size", "clamp(1.6rem, 2.1vw, 2.05rem)")
                                .style("line-height", "1.05")
                                .style("color", "#0c1f1c")
                                .text(page.title)
                            }))
                        }),
                        html!("button", {
                            .attr("type", "button")
                            .style("display", "inline-flex")
                            .style("align-items", "center")
                            .style("justify-content", "center")
                            .style("inline-size", "2.6rem")
                            .style("block-size", "2.6rem")
                            .style("border-radius", "999px")
                            .style("border", "1px solid rgba(13,56,50,0.14)")
                            .style("background", "rgba(247, 248, 247, 0.96)")
                            .style("cursor", "pointer")
                            .child(close_circle_icon())
                            .event(move |_: events::Click| {
                                legal_modal.set(None);
                            })
                        }),
                    ])
                }),
                html!("div", {
                    .style("min-height", "0")
                    .style("overflow", "auto")
                    .style("padding", "1rem 0.15rem 0.15rem 0.15rem")
                    .style("color", "#102120")
                    .style("line-height", "1.6")
                    .style("font-size", "0.98rem")
                    .after_inserted(move |element| {
                        element.set_inner_html(page.html);
                    })
                }),
            ])
        }))
    })
}

fn render_how_it_works_modal() -> Dom {
    let close = || {
        how_it_works_modal_state().set(false);
    };

    html!("div", {
        .style("position", "fixed")
        .style("inset", "0")
        .style("z-index", "1000")
        .style("display", "flex")
        .style("align-items", "center")
        .style("justify-content", "center")
        .style("padding", "1.25rem")
        .child(html!("div", {
            .style("position", "absolute")
            .style("inset", "0")
            .style("background", "rgba(7, 18, 17, 0.6)")
            .event(move |_: events::Click| { close(); })
        }))
        .child(html!("div", {
            .style("position", "relative")
            .style("z-index", "1")
            .style("display", "grid")
            .style("grid-template-rows", "auto minmax(0, 1fr)")
            .style("width", "min(48rem, 100%)")
            .style("max-height", "min(86vh, 48rem)")
            .style("overflow", "hidden")
            .style("padding", "1.4rem 1.5rem 1.2rem 1.5rem")
            .style("border-radius", "1.35rem")
            .style("background", "rgba(255,255,255,0.98)")
            .style("box-shadow", "0 32px 80px rgba(0, 0, 0, 0.22)")
            .children([
                html!("div", {
                    .style("display", "flex")
                    .style("justify-content", "space-between")
                    .style("align-items", "flex-start")
                    .style("gap", "1rem")
                    .style("padding-bottom", "0.9rem")
                    .style("border-bottom", "1px solid rgba(13,56,50,0.08)")
                    .children([
                        html!("h2", {
                            .style("margin", "0")
                            .style("font-size", "clamp(1.5rem, 2vw, 1.85rem)")
                            .style("color", "#0c1f1c")
                            .text("How Groupshop works")
                        }),
                        html!("button", {
                            .attr("type", "button")
                            .style("display", "inline-flex")
                            .style("align-items", "center")
                            .style("justify-content", "center")
                            .style("inline-size", "2.6rem")
                            .style("block-size", "2.6rem")
                            .style("border-radius", "999px")
                            .style("border", "1px solid rgba(13,56,50,0.14)")
                            .style("background", "rgba(247, 248, 247, 0.96)")
                            .style("cursor", "pointer")
                            .child(close_circle_icon())
                            .event(move |_: events::Click| { close(); })
                        }),
                    ])
                }),
                html!("div", {
                    .style("min-height", "0")
                    .style("overflow", "auto")
                    .style("padding", "1rem 0.15rem 0.15rem 0.15rem")
                    .style("color", "#102120")
                    .style("line-height", "1.6")
                    .style("font-size", "0.98rem")
                    .children([
                        how_it_works_step(
                            "1",
                            "Browse open deals",
                            "The home page lists active group orders. Each card shows the unit price, how many buyers have committed, and how many more are needed before the deal triggers.",
                        ),
                        how_it_works_step(
                            "2",
                            "Connect a Solana wallet",
                            "Groupshop uses Solana for escrow. When you click Deposit on a product, you'll be asked to connect your Phantom wallet. We never see your private key — Phantom signs on your behalf in the browser. If you don't have Phantom yet, install it from phantom.app.",
                        ),
                        how_it_works_step(
                            "3",
                            "Commit funds to escrow",
                            "Your USDC deposit is held in an on-chain escrow account, not in our database. The transaction is co-signed by Groupshop (the authority wallet) and you, so neither party can move the funds alone. You'll see the on-chain transaction signature when the deposit lands.",
                        ),
                        how_it_works_step(
                            "4",
                            "Watch the threshold fill",
                            "Each deal has a minimum number of buyers. Your product card and \"My Orders\" page both show a live progress bar. Counts refresh automatically as new buyers join — no need to refresh.",
                        ),
                        how_it_works_step(
                            "5",
                            "Deal triggers, you get wholesale pricing",
                            "Once the threshold is reached, the pool is locked and the order is placed at bulk pricing. If the threshold isn't met, your deposit can be refunded — the on-chain program is the only thing that can move the funds.",
                        ),
                        html!("p", {
                            .style("margin-top", "1.5rem")
                            .style("padding-top", "1rem")
                            .style("border-top", "1px solid rgba(13,56,50,0.08)")
                            .style("font-size", "0.9rem")
                            .style("color", "#4b5563")
                            .text("All on-chain activity is on the Solana network you see in the deposit panel (devnet during early access, mainnet at launch). Need help? Reach out via the address in the footer.")
                        }),
                    ])
                }),
            ])
        }))
    })
}

fn how_it_works_step(num: &str, title: &str, body: &str) -> Dom {
    html!("div", {
        .style("display", "grid")
        .style("grid-template-columns", "2.4rem 1fr")
        .style("gap", "1rem")
        .style("padding", "0.9rem 0")
        .style("border-bottom", "1px solid rgba(13,56,50,0.06)")
        .child(html!("div", {
            .style("display", "flex")
            .style("align-items", "center")
            .style("justify-content", "center")
            .style("inline-size", "2.4rem")
            .style("block-size", "2.4rem")
            .style("border-radius", "999px")
            .style("background", "linear-gradient(135deg, #2563eb 0%, #16a34a 100%)")
            .style("color", "#fff")
            .style("font-weight", "700")
            .style("font-size", "0.95rem")
            .text(num)
        }))
        .child(html!("div", {
            .child(html!("div", {
                .style("font-weight", "600")
                .style("color", "#0c1f1c")
                .style("font-size", "1.05rem")
                .text(title)
            }))
            .child(html!("p", {
                .style("margin", "0.35rem 0 0 0")
                .style("color", "#374151")
                .style("font-size", "0.95rem")
                .style("line-height", "1.55")
                .text(body)
            }))
        }))
    })
}

fn close_circle_icon() -> Dom {
    svg!("svg", {
        .attr("viewBox", "0 0 24 24")
        .attr("width", "18")
        .attr("height", "18")
        .attr("aria-hidden", "true")
        .children([
            svg!("circle", {
                .attr("cx", "12")
                .attr("cy", "12")
                .attr("r", "8.5")
                .attr("fill", "none")
                .attr("stroke", "#113d38")
                .attr("stroke-width", "1.6")
            }),
            svg!("path", {
                .attr("d", "M9.2 9.2L14.8 14.8M14.8 9.2L9.2 14.8")
                .attr("fill", "none")
                .attr("stroke", "#113d38")
                .attr("stroke-width", "1.8")
                .attr("stroke-linecap", "round")
            }),
        ])
    })
}

fn checkbox_children_row(value: Mutable<bool>, children: Vec<Dom>) -> Dom {
    html!("div", {
        .style("display", "flex")
        .style("gap", "0.6rem")
        .style("align-items", "flex-start")
        .style("padding", "0.85rem 0.95rem")
        .style("border-radius", "1rem")
        .style("border", "1px solid rgba(13, 56, 50, 0.12)")
        .style("background", "rgba(255,255,255,0.78)")
        .style("cursor", "pointer")
        .style("user-select", "none")
        .event(clone!(value => move |_: events::Click| {
            value.set(!value.get());
        }))
        .child(html!("input" => web_sys::HtmlInputElement, {
            .attr("type", "checkbox")
            .style("margin-top", "0.2rem")
            .style("inline-size", "1rem")
            .style("block-size", "1rem")
            .prop_signal("checked", value.signal())
            .event(|evt: events::Click| {
                evt.stop_propagation();
            })
            .event(clone!(value => move |evt: events::Change| {
                let checked = evt
                    .target()
                    .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                    .map(|input| input.checked())
                    .unwrap_or(false);
                value.set(checked);
            }))
        }))
        .child(html!("div", {
            .style("display", "flex")
            .style("flex-wrap", "wrap")
            .style("gap", "0.35rem")
            .children(children)
        }))
    })
}

fn plain_text(text: &str) -> Dom {
    html!("p", {
        .class(&*typography::LEAD_TEXT)
        .text(text)
    })
}

fn plain_text_signal(text: Mutable<String>) -> Dom {
    html!("p", {
        .class(&*typography::LEAD_TEXT)
        .text_signal(text.signal_cloned())
    })
}

fn message_block(message: Mutable<Option<String>>, kind: &'static str) -> Dom {
    let border = if kind == "error" {
        "1px solid rgba(157, 32, 54, 0.18)"
    } else {
        "1px solid rgba(21, 128, 61, 0.18)"
    };
    let background = if kind == "error" {
        "rgba(255, 236, 239, 0.96)"
    } else {
        "rgba(236, 253, 245, 0.96)"
    };
    let color = if kind == "error" {
        "#8e1230"
    } else {
        "#166534"
    };

    html!("div", {
        .child_signal(message.signal_cloned().map(move |message| {
            message.map(|message| html!("div", {
                .style("padding", "0.9rem 1rem")
                .style("border-radius", "1rem")
                .style("border", border)
                .style("background", background)
                .style("color", color)
                .style("font-size", "0.98rem")
                .style("line-height", "1.5")
                .text(&message)
            }))
        }))
    })
}

fn auth_shell(title: &'static str, content: Vec<Dom>) -> Dom {
    html!("div", {
        .class(&*chrome::SHELL)
        .child(site_header())
        .child(html!("section", {
            .class(&*chrome::CARD)
            .style("max-width", "58rem")
            .style("margin", "0 auto")
            .style("width", "100%")
            .style("display", "grid")
            .style("gap", "1rem")
            .style("padding", "clamp(1.1rem, 2vw, 1.6rem)")
            .child(html!("h1", {
                .class(&*typography::LEGAL_TITLE)
                .text(title)
            }))
            .children(content)
        }))
        .child(site_footer())
    })
}

fn render_auth_page(
    mode: AuthMode,
    title: &'static str,
    body: &'static str,
    content: Vec<Dom>,
) -> Dom {
    html!("div", {
        .class(&*chrome::SHELL)
        .child(site_header())
        .child(html!("section", {
            .style("max-width", "62rem")
            .style("margin", "0 auto")
            .style("width", "100%")
            .child(html!("div", {
                .style("display", "grid")
                .style("gap", "1rem")
                .style("padding", "clamp(1rem, 1.8vw, 1.5rem)")
                .style("border-radius", "1.7rem")
                .style("background", "linear-gradient(180deg, rgba(255,255,255,0.95), rgba(244,241,233,0.92))")
                .style("box-shadow", "0 24px 50px rgba(14, 31, 28, 0.12)")
                .style("border", "1px solid rgba(13,56,50,0.08)")
                .child(auth_tabs(mode))
                .child(html!("div", {
                    .style("display", "grid")
                    .style("gap", "1rem")
                    .style("max-width", "56rem")
                    .child(html!("div", {
                        .style("display", "grid")
                        .style("gap", "0.4rem")
                        .child(html!("div", {
                            .style("font-size", "0.78rem")
                            .style("font-weight", "700")
                            .style("letter-spacing", "0.16em")
                            .style("text-transform", "uppercase")
                            .style("color", "#0d5b50")
                            .text(match mode {
                                AuthMode::Signin => "Sign in",
                                AuthMode::Register => "Register",
                            })
                        }))
                        .child(html!("h2", {
                            .style("margin", "0")
                            .style("font-size", "clamp(1.75rem, 2.3vw, 2.35rem)")
                            .style("line-height", "1.08")
                            .style("font-weight", "800")
                            .style("color", "#0c1f1c")
                            .text(title)
                        }))
                        .apply_if(!body.is_empty(), |dom| {
                            dom.child(html!("p", {
                                .style("margin", "0")
                                .style("font-size", "1rem")
                                .style("line-height", "1.65")
                                .style("color", "rgba(12,31,28,0.76)")
                                .text(body)
                            }))
                        })
                    }))
                    .children(content)
                }))
            }))
        }))
        .child(site_footer())
    })
}

fn auth_tabs(mode: AuthMode) -> Dom {
    html!("div", {
        .style("display", "grid")
        .style("grid-template-columns", "repeat(2, minmax(0, 1fr))")
        .style("gap", "0.55rem")
        .children([
            auth_tab(Route::Signin, "Sign in", mode == AuthMode::Signin),
            auth_tab(Route::Register, "Register", mode == AuthMode::Register),
        ])
    })
}

fn auth_tab(route: Route, label: &'static str, active: bool) -> Dom {
    let background = if active {
        "linear-gradient(135deg, #179c92 0%, #39be89 100%)"
    } else {
        "linear-gradient(180deg, rgba(255,255,255,0.98), rgba(242,245,243,0.95))"
    };
    let color = if active { "#f7fffd" } else { "#16322f" };
    let border = if active {
        "1px solid rgba(28, 150, 127, 0.26)"
    } else {
        "1px solid rgba(13,56,50,0.12)"
    };

    html!("a", {
        .attr("href", &route.link())
        .style("display", "flex")
        .style("justify-content", "center")
        .style("align-items", "center")
        .style("min-height", "3rem")
        .style("padding", "0.8rem 1rem")
        .style("border-radius", "999px")
        .style("text-decoration", "none")
        .style("font-weight", "700")
        .style("letter-spacing", "0.01em")
        .style("background", background)
        .style("color", color)
        .style("border", border)
        .text(label)
    })
}

fn auth_surface_row(children: Vec<Dom>) -> Dom {
    html!("div", {
        .style("display", "flex")
        .style("flex-wrap", "wrap")
        .style("gap", "1rem")
        .children(children)
    })
}

fn auth_surface(title: &'static str, body: &'static str, children: Vec<Dom>) -> Dom {
    html!("section", {
        .style("flex", "1 1 18rem")
        .style("display", "grid")
        .style("gap", "0.75rem")
        .style("align-content", "start")
        .style("padding", "1.15rem")
        .style("border-radius", "1.35rem")
        .style("background", "rgba(255,255,255,0.9)")
        .style("border", "1px solid rgba(13,56,50,0.12)")
        .style("box-shadow", "0 10px 25px rgba(10,20,19,0.05)")
        .children([
            html!("div", {
                .style("display", "grid")
                .style("gap", "0.4rem")
                .child(html!("div", {
                    .style("font-size", "0.8rem")
                    .style("font-weight", "700")
                    .style("letter-spacing", "0.14em")
                    .style("text-transform", "uppercase")
                    .style("color", "#0f6a5b")
                    .text(title)
                }))
                .apply_if(!body.is_empty(), |dom| {
                    dom.child(html!("p", {
                        .style("margin", "0")
                        .style("font-size", "0.96rem")
                        .style("line-height", "1.55")
                        .style("color", "rgba(12,31,28,0.74)")
                        .text(body)
                    }))
                })
            }),
            html!("div", {
                .style("display", "grid")
                .style("gap", "0.85rem")
                .children(children)
            }),
        ])
    })
}

fn auth_button(
    label: &'static str,
    light: bool,
    icon: Option<fn() -> Dom>,
    on_click: impl Fn() + 'static,
) -> Dom {
    let border = if light {
        "1px solid rgba(13,56,50,0.14)"
    } else {
        "1px solid rgba(28, 150, 127, 0.28)"
    };
    let background = if light {
        "linear-gradient(180deg, rgba(255,255,255,0.98), rgba(245,248,246,0.95))"
    } else {
        "linear-gradient(135deg, #1aa9a0 0%, #3cc58f 100%)"
    };
    let color = if light { "#102120" } else { "#f9fffd" };

    html!("button", {
        .attr("type", "button")
        .style("display", "flex")
        .style("align-items", "center")
        .style("justify-content", "center")
        .style("gap", "0.7rem")
        .style("width", "100%")
        .style("min-height", "3.45rem")
        .style("padding", "0.95rem 1.1rem")
        .style("border-radius", "1rem")
        .style("border", border)
        .style("background", background)
        .style("color", color)
        .style("cursor", "pointer")
        .style("font-size", "1rem")
        .style("font-weight", "700")
        .style("letter-spacing", "0.01em")
        .style("user-select", "none")
        .children(icon.map(|icon| icon()).into_iter())
        .child(html!("span", {
            .text(label)
        }))
        .event(move |_: events::Click| on_click())
    })
}

fn auth_switch_note(prefix: &'static str, route: Route, label: &'static str) -> Dom {
    html!("div", {
        .style("display", "flex")
        .style("flex-wrap", "wrap")
        .style("gap", "0.45rem")
        .style("justify-content", "center")
        .style("align-items", "center")
        .style("padding-top", "0.2rem")
        .children([
            html!("span", {
                .style("font-size", "0.96rem")
                .style("color", "rgba(12,31,28,0.72)")
                .text(prefix)
            }),
            html!("a", {
                .style("font-size", "0.96rem")
                .style("font-weight", "700")
                .style("color", "#0e6d5a")
                .style("text-decoration", "none")
                .attr("href", &route.link())
                .text(label)
            }),
        ])
    })
}

fn username_status_block(status: Mutable<UsernameStatus>) -> Dom {
    html!("div", {
        .style("min-height", "1.6rem")
        .child_signal(status.signal_cloned().map(|status| {
            let (text, color) = match status {
                UsernameStatus::Empty => return None,
                UsernameStatus::Checking => ("Checking availability…", "#5b6470"),
                UsernameStatus::Available => ("Username is available.", "#166534"),
                UsernameStatus::Taken => ("That username is already taken.", "#8e1230"),
                UsernameStatus::Invalid => ("Use a lowercase slug with 3–32 characters.", "#8e1230"),
            };

            Some(html!("div", {
                .style("font-size", "0.95rem")
                .style("font-weight", "600")
                .style("color", color)
                .text(text)
            }))
        }))
    })
}

fn schedule_username_check(
    username: String,
    status: Mutable<UsernameStatus>,
    generation: Mutable<u32>,
) {
    if username.is_empty() {
        status.set(UsernameStatus::Empty);
        return;
    }

    let current_generation = generation.replace_with(|value| *value + 1) + 1;
    status.set(UsernameStatus::Checking);

    spawn_local(async move {
        gloo_timers::future::TimeoutFuture::new(350).await;
        if generation.get() != current_generation {
            return;
        }

        match ApiCtx::get().client.account_username_check(username).await {
            Ok(resp) => {
                if generation.get() != current_generation {
                    return;
                }
                if !resp.valid {
                    status.set(UsernameStatus::Invalid);
                } else if resp.available {
                    status.set(UsernameStatus::Available);
                } else {
                    status.set(UsernameStatus::Taken);
                }
            }
            Err(_) => {
                if generation.get() == current_generation {
                    status.set(UsernameStatus::Invalid);
                }
            }
        }
    });
}

fn google_icon() -> Dom {
    svg!("svg", {
        .attr("viewBox", "0 0 24 24")
        .attr("width", "18")
        .attr("height", "18")
        .attr("aria-hidden", "true")
        .children([
            svg!("path", {
                .attr("fill", "#EA4335")
                .attr("d", "M12.24 10.285v3.821h5.445c-.24 1.54-1.799 4.515-5.445 4.515-3.275 0-5.943-2.71-5.943-6.05s2.668-6.05 5.943-6.05c1.864 0 3.11.796 3.822 1.482l2.607-2.523C17.007 3.938 14.868 3 12.24 3 7.545 3 3.75 6.805 3.75 11.5S7.545 20 12.24 20c6.073 0 8.49-4.241 8.49-6.434 0-.433-.047-.765-.104-1.096H12.24z")
            }),
            svg!("path", {
                .attr("fill", "#34A853")
                .attr("d", "M3.75 7.928l3.144 2.306c.85-1.685 2.595-2.856 5.346-2.856 1.864 0 3.11.796 3.822 1.482l2.607-2.523C17.007 3.938 14.868 3 12.24 3 8.166 3 4.63 5.338 3.75 7.928z")
            }),
            svg!("path", {
                .attr("fill", "#FBBC05")
                .attr("d", "M3.75 11.5c0 1.37.328 2.665.91 3.806l3.383-2.61a6.07 6.07 0 0 1-.31-1.196c0-.414.043-.818.12-1.208L4.67 7.99A8.45 8.45 0 0 0 3.75 11.5z")
            }),
            svg!("path", {
                .attr("fill", "#4285F4")
                .attr("d", "M12.24 20c2.628 0 4.836-.866 6.448-2.351l-3.004-2.326c-.804.562-1.875.956-3.444.956-2.731 0-5.047-1.844-5.873-4.329l-3.24 2.5C4.62 17.617 8.15 20 12.24 20z")
            }),
        ])
    })
}
