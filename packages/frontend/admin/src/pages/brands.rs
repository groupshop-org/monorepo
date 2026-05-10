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
    let brands: MutableVec<AdminBrandSummary> = MutableVec::new();
    let total: Mutable<u32> = Mutable::new(0);
    let error: Mutable<Option<String>> = Mutable::new(None);
    let notice: Mutable<Option<String>> = Mutable::new(None);
    let pending: Mutable<bool> = Mutable::new(false);

    let form_id = Arc::new(Mutex::new(String::new()));
    let form_name = Arc::new(Mutex::new(String::new()));

    load_brands(brands.clone(), total.clone(), error.clone());

    html!("div", {
                .children([
            html!("h1", {
                .class(&*typography::SECTION_TITLE)
                .text("Brands")
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
                    html!("h3", { .text("Create Brand") }),
                    html!("div", {
                        .style("display", "grid")
                        .style("grid-template-columns", "1fr 1fr")
                        .style("gap", "0.75rem")
                        .children([
                            input_field("Slug ID", "e.g. loreal", form_id.clone(), ""),
                            input_field("Name", "Display name", form_name.clone(), ""),
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
                        .event(clone!(form_id, form_name, brands, total, error, notice, pending => move |_: events::Click| {
                            if pending.get() { return; }
                            error.set(None);
                            notice.set(None);
                            pending.set(true);

                            let id_val = form_id.lock().unwrap().clone();
                            let name_val = form_name.lock().unwrap().clone();

                            let brands = brands.clone();
                            let total = total.clone();
                            let error = error.clone();
                            let notice = notice.clone();
                            let pending = pending.clone();

                            spawn_local(async move {
                                let id = match ProductBrandId::new(&id_val) {
                                    Ok(id) => id,
                                    Err(e) => { error.set(Some(format!("Invalid slug: {e}"))); pending.set(false); return; }
                                };

                                match ApiCtx::get().client.admin_create_brand(&AdminCreateBrandRequest {
                                    id,
                                    name: name_val,
                                }).await {
                                    Ok(_) => {
                                        notice.set(Some("Brand created".to_string()));
                                        load_brands(brands, total, error);
                                    }
                                    Err(e) => error.set(Some(format!("{e:?}"))),
                                }
                                pending.set(false);
                            });
                        }))
                    }),
                ])
            }),

            // Brand list
            html!("div", {
                .children_signal_vec(brands.signal_vec_cloned().map(clone!(error, notice => move |brand| {
                    render_brand_row(brand, error.clone(), notice.clone())
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

fn load_brands(
    brands: MutableVec<AdminBrandSummary>,
    total: Mutable<u32>,
    error: Mutable<Option<String>>,
) {
    spawn_local(async move {
        match ApiCtx::get()
            .client
            .admin_list_brands(&AdminListBrandsRequest {
                page: 1,
                per_page: 50,
                search: None,
            })
            .await
        {
            Ok(res) => {
                total.set(res.total);
                brands.lock_mut().replace_cloned(res.brands);
            }
            Err(e) => error.set(Some(format!("{e:?}"))),
        }
    });
}

fn render_brand_row(
    brand: AdminBrandSummary,
    error: Mutable<Option<String>>,
    notice: Mutable<Option<String>>,
) -> Dom {
    let editing: Mutable<bool> = Mutable::new(false);
    let name_buf = Arc::new(Mutex::new(brand.name.clone()));
    let current_name: Mutable<String> = Mutable::new(brand.name.clone());
    let brand_id = brand.id.clone();

    html!("div", {
        .style("padding", "0.4rem 0.75rem")
        .style("border-bottom", &format!("1px solid {}", color::LINE))
        .child_signal(editing.signal().map(clone!(brand_id, name_buf, current_name, editing, error, notice => move |is_editing| {
            if is_editing {
                Some(render_edit(brand_id.clone(), name_buf.clone(), current_name.clone(), editing.clone(), error.clone(), notice.clone()))
            } else {
                Some(render_view(brand_id.clone(), current_name.clone(), name_buf.clone(), editing.clone(), error.clone()))
            }
        })))
    })
}

fn render_view(
    brand_id: ProductBrandId,
    current_name: Mutable<String>,
    name_buf: Arc<Mutex<String>>,
    editing: Mutable<bool>,
    error: Mutable<Option<String>>,
) -> Dom {
    html!("div", {
        .style("display", "flex")
        .style("align-items", "center")
        .style("gap", "1rem")
        .children([
            html!("div", {
                .style("flex", "1")
                .children([
                    html!("strong", { .text_signal(current_name.signal_cloned()) }),
                    html!("span", {
                        .style("color", color::SUBTLE)
                        .style("margin-left", "0.5rem")
                        .style("font-size", "0.8rem")
                        .text(brand_id.as_str())
                    }),
                ])
            }),
            html!("button", {
                .style("cursor", "pointer")
                .style("color", color::BLUE)
                .style("font-size", "0.8rem")
                .style("background", "transparent")
                .style("border", "0")
                .text("Edit")
                .event(clone!(current_name, name_buf, editing => move |_: events::Click| {
                    *name_buf.lock().unwrap() = current_name.get_cloned();
                    editing.set(true);
                }))
            }),
            html!("button", {
                .style("cursor", "pointer")
                .style("color", color::RED)
                .style("font-size", "0.8rem")
                .style("background", "transparent")
                .style("border", "0")
                .text("Delete")
                .event(clone!(error => move |_: events::Click| {
                    let id = brand_id.clone();
                    let error = error.clone();
                    spawn_local(async move {
                        match ApiCtx::get().client.admin_delete_brand(&AdminDeleteBrandRequest { id }).await {
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
    brand_id: ProductBrandId,
    name_buf: Arc<Mutex<String>>,
    current_name: Mutable<String>,
    editing: Mutable<bool>,
    error: Mutable<Option<String>>,
    notice: Mutable<Option<String>>,
) -> Dom {
    let initial = name_buf.lock().unwrap().clone();
    html!("div", {
        .style("display", "flex")
        .style("align-items", "center")
        .style("gap", "0.75rem")
        .children([
            html!("span", {
                .style("color", color::SUBTLE)
                .style("font-size", "0.8rem")
                .text(brand_id.as_str())
            }),
            html!("div", {
                .style("flex", "1")
                .child(input_field("", "Name", name_buf.clone(), &initial))
            }),
            html!("button", {
                .style("cursor", "pointer")
                .style("color", color::GREEN)
                .style("font-size", "0.8rem")
                .style("background", "transparent")
                .style("border", "0")
                .text("Save")
                .event(clone!(brand_id, name_buf, current_name, editing, error, notice => move |_: events::Click| {
                    let id = brand_id.clone();
                    let new_name = name_buf.lock().unwrap().clone();
                    let current_name = current_name.clone();
                    let editing = editing.clone();
                    let error = error.clone();
                    let notice = notice.clone();
                    spawn_local(async move {
                        match ApiCtx::get().client.admin_update_brand(&AdminUpdateBrandRequest {
                            id,
                            name: Some(new_name.clone()),
                        }).await {
                            Ok(res) => {
                                current_name.set(res.brand.name);
                                editing.set(false);
                                notice.set(Some("Brand updated".to_string()));
                            }
                            Err(e) => error.set(Some(format!("{e:?}"))),
                        }
                    });
                }))
            }),
            html!("button", {
                .style("cursor", "pointer")
                .style("color", color::MUTED)
                .style("font-size", "0.8rem")
                .style("background", "transparent")
                .style("border", "0")
                .text("Cancel")
                .event(clone!(editing => move |_: events::Click| {
                    editing.set(false);
                }))
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
