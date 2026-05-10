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
                            input_field("Slug ID", "e.g. my-product", form_id.clone(), ""),
                            input_field("GTIN", "Barcode", form_gtin.clone(), ""),
                            input_field("Name", "Display name", form_name.clone(), ""),
                            input_field("Category ID", "Category slug", form_category_id.clone(), ""),
                            input_field("Brand ID", "Brand slug", form_brand_id.clone(), ""),
                            input_field("Price (cents)", "e.g. 102", form_price.clone(), ""),
                            input_field("Min Order Qty", "e.g. 100", form_moq.clone(), ""),
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
                .children_signal_vec(products.signal_vec_cloned().map(clone!(error, notice => move |product| {
                    render_product_row(product, error.clone(), notice.clone())
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

#[derive(Clone)]
struct ProductState {
    name: Mutable<String>,
    price_cents: Mutable<u32>,
    currency: Mutable<String>,
    minimum_order_quantity: Mutable<u32>,
    inventory: Mutable<u32>,
    is_preorder: Mutable<bool>,
    estimated_delivery_weeks: Mutable<Option<u32>>,
    supplier_url: Mutable<String>,
    image_url: Mutable<String>,
    is_active: Mutable<bool>,
}

impl ProductState {
    fn from_summary(p: &AdminProductSummary) -> Self {
        Self {
            name: Mutable::new(p.name.clone()),
            price_cents: Mutable::new(p.price_cents),
            currency: Mutable::new(p.currency.clone()),
            minimum_order_quantity: Mutable::new(p.minimum_order_quantity),
            inventory: Mutable::new(p.inventory),
            is_preorder: Mutable::new(p.is_preorder),
            estimated_delivery_weeks: Mutable::new(p.estimated_delivery_weeks),
            supplier_url: Mutable::new(p.supplier_url.clone()),
            image_url: Mutable::new(p.image_url.clone()),
            is_active: Mutable::new(p.is_active),
        }
    }

    fn apply(&self, p: &AdminProductSummary) {
        self.name.set(p.name.clone());
        self.price_cents.set(p.price_cents);
        self.currency.set(p.currency.clone());
        self.minimum_order_quantity.set(p.minimum_order_quantity);
        self.inventory.set(p.inventory);
        self.is_preorder.set(p.is_preorder);
        self.estimated_delivery_weeks
            .set(p.estimated_delivery_weeks);
        self.supplier_url.set(p.supplier_url.clone());
        self.image_url.set(p.image_url.clone());
        self.is_active.set(p.is_active);
    }
}

fn render_product_row(
    product: AdminProductSummary,
    error: Mutable<Option<String>>,
    notice: Mutable<Option<String>>,
) -> Dom {
    let product_id = product.id.clone();
    let category_id = product.category_id.clone();
    let brand_id = product.brand_id.clone();
    let state = ProductState::from_summary(&product);
    let editing: Mutable<bool> = Mutable::new(false);

    html!("div", {
        .style("padding", "0.5rem 0.75rem")
        .style("border-bottom", &format!("1px solid {}", color::LINE))
        .child_signal(editing.signal().map(clone!(product_id, category_id, brand_id, state, editing, error, notice => move |is_editing| {
            if is_editing {
                Some(render_edit(product_id.clone(), state.clone(), editing.clone(), error.clone(), notice.clone()))
            } else {
                Some(render_view(product_id.clone(), category_id.clone(), brand_id.clone(), state.clone(), editing.clone(), error.clone(), notice.clone()))
            }
        })))
    })
}

#[allow(clippy::too_many_arguments)]
fn render_view(
    product_id: ProductId,
    category_id: ProductCategoryId,
    brand_id: ProductBrandId,
    state: ProductState,
    editing: Mutable<bool>,
    error: Mutable<Option<String>>,
    notice: Mutable<Option<String>>,
) -> Dom {
    html!("div", {
        .style("display", "flex")
        .style("align-items", "center")
        .style("gap", "1rem")
        .children([
            html!("div", {
                .style("flex", "1")
                .children([
                    html!("strong", { .text_signal(state.name.signal_cloned()) }),
                    html!("span", {
                        .style("color", color::MUTED)
                        .style("margin-left", "0.5rem")
                        .style("font-size", "0.8rem")
                        .text(&format!("{} / {}", category_id.as_str(), brand_id.as_str()))
                    }),
                ])
            }),
            html!("span", {
                .style("font-size", "0.85rem")
                .text_signal(state.price_cents.signal().map(|p| format!("${:.2}", p as f64 / 100.0)))
            }),
            html!("span", {
                .style("font-size", "0.85rem")
                .style("color", color::MUTED)
                .text_signal(state.minimum_order_quantity.signal().map(|m| format!("MOQ: {}", m)))
            }),
            html!("span", {
                .style("font-size", "0.8rem")
                .style_signal("color", state.is_active.signal().map(|a| if a { color::GREEN } else { color::RED }))
                .text_signal(state.is_active.signal().map(|a| if a { "Active" } else { "Inactive" }))
            }),
            html!("button", {
                .style("cursor", "pointer")
                .style("color", color::BLUE)
                .style("font-size", "0.8rem")
                .style("background", "transparent")
                .style("border", "0")
                .text("Edit")
                .event(clone!(editing => move |_: events::Click| {
                    editing.set(true);
                }))
            }),
            html!("button", {
                .style("cursor", "pointer")
                .style("font-size", "0.8rem")
                .style("background", "transparent")
                .style("border", "0")
                .style_signal("color", state.is_active.signal().map(|a| if a { color::YELLOW } else { color::GREEN }))
                .text_signal(state.is_active.signal().map(|a| if a { "Deactivate" } else { "Activate" }))
                .event(clone!(product_id, state, error, notice => move |_: events::Click| {
                    let id = product_id.clone();
                    let new_active = !state.is_active.get();
                    let state = state.clone();
                    let error = error.clone();
                    let notice = notice.clone();
                    spawn_local(async move {
                        match ApiCtx::get().client.admin_update_product(&AdminUpdateProductRequest {
                            id,
                            name: None,
                            price_cents: None,
                            currency: None,
                            minimum_order_quantity: None,
                            inventory: None,
                            is_preorder: None,
                            estimated_delivery_weeks: None,
                            supplier_url: None,
                            image_url: None,
                            is_active: Some(new_active),
                        }).await {
                            Ok(res) => {
                                state.apply(&res.product);
                                notice.set(Some(format!("Product {}", if new_active { "activated" } else { "deactivated" })));
                            }
                            Err(e) => error.set(Some(format!("{e:?}"))),
                        }
                    });
                }))
            }),
            html!("button", {
                .style("cursor", "pointer")
                .style("color", color::RED)
                .style("font-size", "0.8rem")
                .style("background", "transparent")
                .style("border", "0")
                .text("Delete")
                .event(clone!(product_id, error => move |_: events::Click| {
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

fn render_edit(
    product_id: ProductId,
    state: ProductState,
    editing: Mutable<bool>,
    error: Mutable<Option<String>>,
    notice: Mutable<Option<String>>,
) -> Dom {
    let name_buf = Arc::new(Mutex::new(state.name.get_cloned()));
    let price_buf = Arc::new(Mutex::new(state.price_cents.get().to_string()));
    let currency_buf = Arc::new(Mutex::new(state.currency.get_cloned()));
    let moq_buf = Arc::new(Mutex::new(state.minimum_order_quantity.get().to_string()));
    let inventory_buf = Arc::new(Mutex::new(state.inventory.get().to_string()));
    let supplier_url_buf = Arc::new(Mutex::new(state.supplier_url.get_cloned()));
    let image_url_buf = Arc::new(Mutex::new(state.image_url.get_cloned()));
    let edw_buf = Arc::new(Mutex::new(
        state
            .estimated_delivery_weeks
            .get()
            .map(|v| v.to_string())
            .unwrap_or_default(),
    ));
    let is_preorder = Mutable::new(state.is_preorder.get());
    let is_active = Mutable::new(state.is_active.get());

    let n0 = name_buf.lock().unwrap().clone();
    let p0 = price_buf.lock().unwrap().clone();
    let c0 = currency_buf.lock().unwrap().clone();
    let m0 = moq_buf.lock().unwrap().clone();
    let i0 = inventory_buf.lock().unwrap().clone();
    let s0 = supplier_url_buf.lock().unwrap().clone();
    let img0 = image_url_buf.lock().unwrap().clone();
    let e0 = edw_buf.lock().unwrap().clone();

    html!("div", {
        .style("background", color::PAPER_DARK)
        .style("border-radius", "0.375rem")
        .style("padding", "0.75rem")
        .children([
            html!("div", {
                .style("font-size", "0.85rem")
                .style("color", color::MUTED)
                .style("margin-bottom", "0.5rem")
                .text(&format!("Editing: {}", product_id.as_str()))
            }),
            html!("div", {
                .style("display", "grid")
                .style("grid-template-columns", "1fr 1fr 1fr")
                .style("gap", "0.5rem")
                .children([
                    input_field("Name", "Display name", name_buf.clone(), &n0),
                    input_field("Price (cents)", "e.g. 1099", price_buf.clone(), &p0),
                    input_field("Currency", "USD", currency_buf.clone(), &c0),
                    input_field("Min Order Qty", "e.g. 100", moq_buf.clone(), &m0),
                    input_field("Inventory", "e.g. 0", inventory_buf.clone(), &i0),
                    input_field("Est. Delivery (weeks)", "(blank for none)", edw_buf.clone(), &e0),
                    input_field("Supplier URL", "https://...", supplier_url_buf.clone(), &s0),
                    input_field("Image URL", "https://...", image_url_buf.clone(), &img0),
                    checkbox_field("Preorder", is_preorder.clone()),
                    checkbox_field("Active", is_active.clone()),
                ])
            }),
            html!("div", {
                .style("display", "flex")
                .style("gap", "0.75rem")
                .style("margin-top", "0.75rem")
                .children([
                    html!("button", {
                        .style("cursor", "pointer")
                        .style("padding", "0.4rem 1rem")
                        .style("border-radius", "0.375rem")
                        .style("border", &format!("1px solid {}", color::LINE))
                        .style("background", color::GREEN)
                        .style("color", "#ffffff")
                        .style("font-size", "0.85rem")
                        .text("Save")
                        .event(clone!(product_id, state, editing, error, notice,
                                      name_buf, price_buf, currency_buf, moq_buf, inventory_buf,
                                      supplier_url_buf, image_url_buf, edw_buf, is_preorder, is_active => move |_: events::Click| {
                            let id = product_id.clone();
                            let state = state.clone();
                            let editing = editing.clone();
                            let error = error.clone();
                            let notice = notice.clone();

                            let name = name_buf.lock().unwrap().clone();
                            let price_str = price_buf.lock().unwrap().clone();
                            let currency = currency_buf.lock().unwrap().clone();
                            let moq_str = moq_buf.lock().unwrap().clone();
                            let inventory_str = inventory_buf.lock().unwrap().clone();
                            let supplier_url = supplier_url_buf.lock().unwrap().clone();
                            let image_url = image_url_buf.lock().unwrap().clone();
                            let edw_str = edw_buf.lock().unwrap().clone();
                            let preorder_val = is_preorder.get();
                            let active_val = is_active.get();

                            spawn_local(async move {
                                let price_cents = match price_str.parse::<u32>() {
                                    Ok(v) => v,
                                    Err(_) => { error.set(Some("Invalid price".to_string())); return; }
                                };
                                let moq = match moq_str.parse::<u32>() {
                                    Ok(v) => v,
                                    Err(_) => { error.set(Some("Invalid MOQ".to_string())); return; }
                                };
                                let inventory = match inventory_str.parse::<u32>() {
                                    Ok(v) => v,
                                    Err(_) => { error.set(Some("Invalid inventory".to_string())); return; }
                                };
                                let edw_inner: Option<u32> = if edw_str.trim().is_empty() {
                                    None
                                } else {
                                    match edw_str.trim().parse::<u32>() {
                                        Ok(v) => Some(v),
                                        Err(_) => { error.set(Some("Invalid delivery weeks".to_string())); return; }
                                    }
                                };

                                match ApiCtx::get().client.admin_update_product(&AdminUpdateProductRequest {
                                    id,
                                    name: Some(name),
                                    price_cents: Some(price_cents),
                                    currency: Some(currency),
                                    minimum_order_quantity: Some(moq),
                                    inventory: Some(inventory),
                                    is_preorder: Some(preorder_val),
                                    estimated_delivery_weeks: Some(edw_inner),
                                    supplier_url: Some(supplier_url),
                                    image_url: Some(image_url),
                                    is_active: Some(active_val),
                                }).await {
                                    Ok(res) => {
                                        state.apply(&res.product);
                                        editing.set(false);
                                        notice.set(Some("Product updated".to_string()));
                                    }
                                    Err(e) => error.set(Some(format!("{e:?}"))),
                                }
                            });
                        }))
                    }),
                    html!("button", {
                        .style("cursor", "pointer")
                        .style("padding", "0.4rem 1rem")
                        .style("border-radius", "0.375rem")
                        .style("border", &format!("1px solid {}", color::LINE))
                        .style("background", color::PAPER)
                        .style("color", color::INK)
                        .style("font-size", "0.85rem")
                        .text("Cancel")
                        .event(clone!(editing => move |_: events::Click| {
                            editing.set(false);
                        }))
                    }),
                ])
            }),
        ])
    })
}

fn input_field(label: &str, placeholder: &str, value: Arc<Mutex<String>>, initial: &str) -> Dom {
    let label = label.to_string();
    let placeholder = placeholder.to_string();
    let initial = initial.to_string();
    *value.lock().unwrap() = initial.clone();

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
                .prop("value", &initial)
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

fn checkbox_field(label: &str, value: Mutable<bool>) -> Dom {
    let label = label.to_string();

    html!("label", {
        .style("display", "flex")
        .style("align-items", "center")
        .style("gap", "0.4rem")
        .style("font-size", "0.85rem")
        .style("padding-top", "1.1rem")
        .children([
            html!("input" => web_sys::HtmlInputElement, {
                .attr("type", "checkbox")
                .prop_signal("checked", value.signal())
                .event(clone!(value => move |_: events::Change| {
                    value.set(!value.get());
                }))
            }),
            html!("span", { .text(&label) }),
        ])
    })
}
