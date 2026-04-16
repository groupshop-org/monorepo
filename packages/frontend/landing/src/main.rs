mod api;
mod config;
mod legal;
mod route;

use std::sync::{Arc, Mutex};

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
    legal::{privacy_policy, terms_of_service, PageContent},
    route::{Resolved, Route},
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
            Route::Error(err) => render_error(err),
            Route::NotFound => render_not_found(),
        })
    })
}

fn render_home() -> Dom {
    let products: Mutable<Option<Vec<ProductSummary>>> = Mutable::new(None);
    let total: Mutable<u32> = Mutable::new(0);

    spawn_local(clone!(products, total => async move {
        match ApiCtx::get().client.product_list(&ProductListRequest {
            page: 1,
            per_page: 24,
            category_id: None,
            brand_id: None,
            search: None,
        }).await {
            Ok(res) => {
                total.set(res.total);
                products.set(Some(res.products));
            }
            Err(_) => {
                products.set(Some(Vec::new()));
            }
        }
    }));

    html!("div", {
        .class(&*chrome::SHELL)
        .child(site_header())
        .child(hero_banner())
        .child(html!("section", {
            .attr("id", "products")
            .style("margin-top", "2.5rem")
            .child(html!("div", {
                .class(&*chrome::SECTION_HEADER)
                .children([
                    section_title("Products"),
                    html!("span", {
                        .class(&*typography::MICRO_LABEL)
                        .text_signal(total.signal().map(|t| {
                            if t > 0 { format!("{t} items") } else { String::new() }
                        }))
                    }),
                ])
            }))
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
                                    .text("No products available yet.")
                                }),
                                html!("p", {
                                    .class(&*typography::BODY_MUTED)
                                    .text("Check back soon — new products are being added regularly.")
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
        }))
        .child(how_it_works_section())
        .child(site_footer())
    })
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
        }))
    })
}

fn how_it_works_section() -> Dom {
    html!("section", {
        .attr("id", "how-it-works")
        .class(&*chrome::SIGNAL_GRID)
        .style("margin-top", "3rem")
        .children([
            html!("article", {
                .class(&*chrome::SIGNAL_CARD)
                .children([
                    html!("div", { .class(&*typography::MINI_HEADING) .text("Browse products") }),
                    html!("p", { .class(&*typography::BODY_MUTED) .text("Explore the catalog with real wholesale pricing. No account needed to browse.") }),
                ])
            }),
            html!("article", {
                .class(&*chrome::SIGNAL_CARD)
                .children([
                    html!("div", { .class(&*typography::MINI_HEADING) .text("Commit together") }),
                    html!("p", { .class(&*typography::BODY_MUTED) .text("Join a group order to meet the minimum order quantity. More buyers means everyone saves.") }),
                ])
            }),
            html!("article", {
                .class(&*chrome::SIGNAL_CARD)
                .children([
                    html!("div", { .class(&*typography::MINI_HEADING) .text("Get wholesale prices") }),
                    html!("p", { .class(&*typography::BODY_MUTED) .text("Once the group hits the threshold, orders are placed at bulk pricing and shipped to you.") }),
                ])
            }),
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

    spawn_local(clone!(product => async move {
        match ApiCtx::get().client.product_detail(&ProductDetailRequest { id }).await {
            Ok(res) => product.set(Some(Ok(res.product))),
            Err(e) => product.set(Some(Err(format!("{e:?}")))),
        }
    }));

    html!("div", {
        .class(&*chrome::SHELL)
        .child(site_header())
        .child(html!("div", {
            .child_signal(product.signal_cloned().map(|state| {
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
                    Some(Ok(p)) => render_product_detail_content(p),
                })
            }))
        }))
        .child(site_footer())
    })
}

fn render_product_detail_content(product: ProductSummary) -> Dom {
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
            // Payment card
            .child(html!("div", {
                .class(&*chrome::CARD)
                .style("margin-top", "1.5rem")
                .children([
                    html!("h3", {
                        .class(&*typography::MINI_HEADING)
                        .style("margin-bottom", "1rem")
                        .text("Join this group order")
                    }),
                    // Order summary
                    html!("div", {
                        .style("display", "flex")
                        .style("flex-direction", "column")
                        .style("gap", "0.75rem")
                        .children([
                            summary_row("Unit price", &price),
                            summary_row("Min. order quantity", &format!("{}", product.minimum_order_quantity)),
                            summary_row("Your commitment", &format!("{} units", product.minimum_order_quantity)),
                        ])
                    }),
                    // Total
                    html!("div", {
                        .style("display", "flex")
                        .style("justify-content", "space-between")
                        .style("margin-top", "1rem")
                        .style("padding-top", "1rem")
                        .style("border-top", &format!("1px solid {}", groupshop_frontend_shared::theme::color::LINE))
                        .children([
                            html!("span", {
                                .style("font-weight", "700")
                                .text("Total")
                            }),
                            html!("span", {
                                .style("font-weight", "700")
                                .style("font-size", "1.15rem")
                                .text(&format!("${:.2}", product.price_cents as f64 / 100.0 * product.minimum_order_quantity as f64))
                            }),
                        ])
                    }),
                    // Wallet connect button
                    html!("button", {
                        .class(&*chrome::BUTTON)
                        .class(&*chrome::BUTTON_GRADIENT)
                        .style("width", "100%")
                        .style("margin-top", "1.5rem")
                        .style("min-height", "3rem")
                        .style("font-size", "1rem")
                        .text("Connect wallet & pay")
                        .event(|_: events::Click| {
                            let _ = web_sys::window()
                                .unwrap()
                                .alert_with_message("Wallet connection coming soon!");
                        })
                    }),
                    // Info note
                    html!("p", {
                        .class(&*typography::BODY_MUTED)
                        .style("margin-top", "0.75rem")
                        .style("font-size", "0.8rem")
                        .style("text-align", "center")
                        .text("Payment is held in escrow on Solana until the group order threshold is met.")
                    }),
                ])
            }))
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

fn summary_row(label: &str, value: &str) -> Dom {
    html!("div", {
        .style("display", "flex")
        .style("justify-content", "space-between")
        .children([
            html!("span", {
                .class(&*typography::BODY_MUTED)
                .text(label)
            }),
            html!("span", {
                .style("font-weight", "500")
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

fn site_header() -> Dom {
    let profile = ApiCtx::get().profile.get_cloned();
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
                section_nav_link("/#products", "Products"),
                section_nav_link("/#how-it-works", "How It Works"),
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

fn section_nav_link(href: &'static str, text: &'static str) -> Dom {
    html!("a", {
        .class(&*chrome::NAV_LINK)
        .class(&*typography::NAV_LABEL)
        .attr("href", href)
        .text(text)
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
            let mut items = vec![menu_link_item(Route::Profile, "Account")];
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

fn input_field(
    label: &'static str,
    name: &'static str,
    input_type: &'static str,
    autocomplete: &'static str,
    on_input: impl Fn(String) + 'static,
) -> Dom {
    html!("label", {
        .style("display", "grid")
        .style("gap", "0.45rem")
        .child(html!("span", {
            .style("font-size", "0.82rem")
            .style("font-weight", "700")
            .style("letter-spacing", "0.03em")
            .style("text-transform", "uppercase")
            .style("color", "rgba(10, 18, 17, 0.74)")
            .text(label)
        }))
        .child(html!("input" => web_sys::HtmlInputElement, {
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
        }))
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
