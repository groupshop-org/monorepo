use std::sync::{Arc, Mutex};

use dominator::{clone, events, html, Dom};
use futures_signals::signal::{Mutable, SignalExt};
use futures_signals::signal_vec::{MutableVec, SignalVecExt};
use groupshop_backend_shared::prelude::*;
use groupshop_frontend_shared::theme::{color, typography};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

use crate::api::ApiCtx;

pub fn render() -> Dom {
    let products: MutableVec<AdminProductSummary> = MutableVec::new();
    let total: Mutable<u32> = Mutable::new(0);
    let page: Mutable<u32> = Mutable::new(1);
    let error: Mutable<Option<String>> = Mutable::new(None);
    let notice: Mutable<Option<String>> = Mutable::new(None);
    let pending: Mutable<bool> = Mutable::new(false);

    let form_id = Arc::new(Mutex::new(String::new()));
    let form_gtin = Arc::new(Mutex::new(String::new()));
    let form_name = Arc::new(Mutex::new(String::new()));
    let form_category_id = Arc::new(Mutex::new(String::new()));
    let form_brand_id = Arc::new(Mutex::new(String::new()));
    let form_price = Arc::new(Mutex::new(String::new()));
    let form_moq = Arc::new(Mutex::new(String::new()));

    load_products(products.clone(), total.clone(), page.clone(), error.clone());

    html!("div", {
                .children([
            html!("h1", {
                .class(&*typography::SECTION_TITLE)
                .text("Products")
            }),

            // Messages
            html!("div", {
                .child_signal(error.signal_cloned().map(|e| e.map(|msg| html!("p", {
                    .style("color", color::RED)
                    .style("padding", "0.5rem")
                    .text(&msg)
                }))))
            }),
            html!("div", {
                .child_signal(notice.signal_cloned().map(|n| n.map(|msg| html!("p", {
                    .style("color", color::GREEN)
                    .style("padding", "0.5rem")
                    .text(&msg)
                }))))
            }),

            // Create form
            html!("div", {
                .style("border", &format!("1px solid {}", color::LINE))
                .style("border-radius", "0.5rem")
                .style("padding", "1rem")
                .style("margin-bottom", "1.5rem")
                .style("background", color::PAPER)
                .children([
                    html!("h3", { .text("Create Product") }),
                    html!("div", {
                        .style("display", "grid")
                        .style("grid-template-columns", "1fr 1fr")
                        .style("gap", "0.75rem")
                        .children([
                            input_field("Slug ID", "e.g. my-product", form_id.clone()),
                            input_field("GTIN", "Barcode", form_gtin.clone()),
                            input_field("Name", "Display name", form_name.clone()),
                            input_field("Category ID", "Category slug", form_category_id.clone()),
                            input_field("Brand ID", "Brand slug", form_brand_id.clone()),
                            input_field("Price (cents)", "e.g. 102", form_price.clone()),
                            input_field("Min Order Qty", "e.g. 100", form_moq.clone()),
                        ])
                    }),
                    html!("button", {
                        .style("margin-top", "0.75rem")
                        .style("cursor", "pointer")
                        .style("padding", "0.5rem 1.5rem")
                        .style("border-radius", "0.375rem")
                        .style("border", &format!("1px solid {}", color::LINE))
                        .style("background", color::PAPER)
                        .style("color", color::INK)
                        .text("Create")
                        .event(clone!(form_id, form_gtin, form_name, form_category_id, form_brand_id, form_price, form_moq, products, total, page, error, notice, pending => move |_: events::Click| {
                            if pending.get() { return; }
                            error.set(None);
                            notice.set(None);
                            pending.set(true);

                            let id_val = form_id.lock().unwrap().clone();
                            let gtin_val = form_gtin.lock().unwrap().clone();
                            let name_val = form_name.lock().unwrap().clone();
                            let cat_val = form_category_id.lock().unwrap().clone();
                            let brand_val = form_brand_id.lock().unwrap().clone();
                            let price_val = form_price.lock().unwrap().clone();
                            let moq_val = form_moq.lock().unwrap().clone();

                            let products = products.clone();
                            let total = total.clone();
                            let page = page.clone();
                            let error = error.clone();
                            let notice = notice.clone();
                            let pending = pending.clone();

                            spawn_local(async move {
                                let id = match ProductId::new(&id_val) {
                                    Ok(id) => id,
                                    Err(e) => { error.set(Some(format!("Invalid slug: {e}"))); pending.set(false); return; }
                                };
                                let category_id = match ProductCategoryId::new(&cat_val) {
                                    Ok(id) => id,
                                    Err(e) => { error.set(Some(format!("Invalid category: {e}"))); pending.set(false); return; }
                                };
                                let brand_id = match ProductBrandId::new(&brand_val) {
                                    Ok(id) => id,
                                    Err(e) => { error.set(Some(format!("Invalid brand: {e}"))); pending.set(false); return; }
                                };
                                let price_cents = match price_val.parse::<u32>() {
                                    Ok(v) => v,
                                    Err(_) => { error.set(Some("Invalid price".to_string())); pending.set(false); return; }
                                };
                                let moq = match moq_val.parse::<u32>() {
                                    Ok(v) => v,
                                    Err(_) => { error.set(Some("Invalid MOQ".to_string())); pending.set(false); return; }
                                };

                                match ApiCtx::get().client.admin_create_product(&AdminCreateProductRequest {
                                    id,
                                    gtin: gtin_val,
                                    name: name_val,
                                    category_id,
                                    brand_id,
                                    price_cents,
                                    currency: "USD".to_string(),
                                    minimum_order_quantity: moq,
                                    inventory: 0,
                                    is_preorder: false,
                                    estimated_delivery_weeks: None,
                                    supplier_url: String::new(),
                                    image_url: String::new(),
                                }).await {
                                    Ok(_) => {
                                        notice.set(Some("Product created".to_string()));
                                        load_products(products, total, page, error);
                                    }
                                    Err(e) => error.set(Some(format!("{e:?}"))),
                                }
                                pending.set(false);
                            });
                        }))
                    }),
                ])
            }),

            // Product list
            html!("div", {
                .children_signal_vec(products.signal_vec_cloned().map(clone!(error => move |product| {
                    render_product_row(product, error.clone())
                })))
            }),

            // Total count
            html!("p", {
                .class(&*typography::BODY_MUTED)
                .style("margin-top", "1rem")
                .text_signal(total.signal().map(|t| format!("Total: {t}")))
            }),
        ])
    })
}

fn load_products(
    products: MutableVec<AdminProductSummary>,
    total: Mutable<u32>,
    page: Mutable<u32>,
    error: Mutable<Option<String>>,
) {
    let current_page = page.get();
    spawn_local(async move {
        match ApiCtx::get()
            .client
            .admin_list_products(&AdminListProductsRequest {
                page: current_page,
                per_page: 50,
                category_id: None,
                brand_id: None,
                search: None,
            })
            .await
        {
            Ok(res) => {
                total.set(res.total);
                products.lock_mut().replace_cloned(res.products);
            }
            Err(e) => error.set(Some(format!("{e:?}"))),
        }
    });
}

fn render_product_row(product: AdminProductSummary, error: Mutable<Option<String>>) -> Dom {
    let product_id = product.id.clone();

    html!("div", {
        .style("display", "flex")
        .style("align-items", "center")
        .style("gap", "1rem")
        .style("padding", "0.5rem 0.75rem")
        .style("border-bottom", &format!("1px solid {}", color::LINE))
        .children([
            html!("div", {
                .style("flex", "1")
                .children([
                    html!("strong", { .text(&product.name) }),
                    html!("span", {
                        .style("color", color::MUTED)
                        .style("margin-left", "0.5rem")
                        .style("font-size", "0.8rem")
                        .text(&format!("{} / {}", product.category_id.as_str(), product.brand_id.as_str()))
                    }),
                ])
            }),
            html!("span", {
                .style("font-size", "0.85rem")
                .text(&format!("${:.2}", product.price_cents as f64 / 100.0))
            }),
            html!("span", {
                .style("font-size", "0.85rem")
                .style("color", color::MUTED)
                .text(&format!("MOQ: {}", product.minimum_order_quantity))
            }),
            html!("span", {
                .style("font-size", "0.8rem")
                .style("color", if product.is_active { color::GREEN } else { color::RED })
                .text(if product.is_active { "Active" } else { "Inactive" })
            }),
            html!("button", {
                .style("cursor", "pointer")
                .style("color", color::RED)
                .style("font-size", "0.8rem")
                .style("background", "transparent")
                .style("border", "0")
                .text("Delete")
                .event(clone!(error => move |_: events::Click| {
                    let id = product_id.clone();
                    let error = error.clone();
                    spawn_local(async move {
                        match ApiCtx::get().client.admin_delete_product(&AdminDeleteProductRequest { id }).await {
                            Ok(_) => { let _ = web_sys::window().unwrap().location().reload(); }
                            Err(e) => error.set(Some(format!("{e:?}"))),
                        }
                    });
                }))
            }),
        ])
    })
}

fn input_field(label: &str, placeholder: &str, value: Arc<Mutex<String>>) -> Dom {
    let label = label.to_string();
    let placeholder = placeholder.to_string();

    html!("div", {
        .children([
            html!("label", {
                .style("font-size", "0.8rem")
                .style("color", color::MUTED)
                .text(&label)
            }),
            html!("input" => web_sys::HtmlInputElement, {
                .attr("type", "text")
                .attr("placeholder", &placeholder)
                .style("display", "block")
                .style("width", "100%")
                .style("padding", "0.375rem 0.5rem")
                .style("border", &format!("1px solid {}", color::LINE_STRONG))
                .style("border-radius", "0.25rem")
                .style("background", color::BG)
                .style("color", color::INK)
                .event(move |e: events::Input| {
                    if let Some(target) = e.target() {
                        if let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() {
                            *value.lock().unwrap() = input.value();
                        }
                    }
                })
            }),
        ])
    })
}
