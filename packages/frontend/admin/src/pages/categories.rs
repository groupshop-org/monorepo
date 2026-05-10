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
    let categories: MutableVec<AdminCategorySummary> = MutableVec::new();
    let error: Mutable<Option<String>> = Mutable::new(None);
    let notice: Mutable<Option<String>> = Mutable::new(None);
    let pending: Mutable<bool> = Mutable::new(false);

    let form_id = Arc::new(Mutex::new(String::new()));
    let form_name = Arc::new(Mutex::new(String::new()));
    let form_parent_id = Arc::new(Mutex::new(String::new()));

    load_categories(categories.clone(), error.clone());

    html!("div", {
                .children([
            html!("h1", {
                .class(&*typography::SECTION_TITLE)
                .text("Categories")
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
                    html!("h3", { .text("Create Category") }),
                    html!("div", {
                        .style("display", "grid")
                        .style("grid-template-columns", "1fr 1fr 1fr")
                        .style("gap", "0.75rem")
                        .children([
                            input_field("Slug ID", "e.g. shampoo", form_id.clone(), ""),
                            input_field("Name", "Display name", form_name.clone(), ""),
                            input_field("Parent ID", "(optional) parent slug", form_parent_id.clone(), ""),
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
                        .event(clone!(form_id, form_name, form_parent_id, categories, error, notice, pending => move |_: events::Click| {
                            if pending.get() { return; }
                            error.set(None);
                            notice.set(None);
                            pending.set(true);

                            let id_val = form_id.lock().unwrap().clone();
                            let name_val = form_name.lock().unwrap().clone();
                            let parent_val = form_parent_id.lock().unwrap().clone();

                            let categories = categories.clone();
                            let error = error.clone();
                            let notice = notice.clone();
                            let pending = pending.clone();

                            spawn_local(async move {
                                let id = match ProductCategoryId::new(&id_val) {
                                    Ok(id) => id,
                                    Err(e) => { error.set(Some(format!("Invalid slug: {e}"))); pending.set(false); return; }
                                };

                                let parent_id = if parent_val.trim().is_empty() {
                                    None
                                } else {
                                    match ProductCategoryId::new(parent_val.trim()) {
                                        Ok(id) => Some(id),
                                        Err(e) => { error.set(Some(format!("Invalid parent: {e}"))); pending.set(false); return; }
                                    }
                                };

                                match ApiCtx::get().client.admin_create_category(&AdminCreateCategoryRequest {
                                    id,
                                    name: name_val,
                                    parent_id,
                                }).await {
                                    Ok(_) => {
                                        notice.set(Some("Category created".to_string()));
                                        load_categories(categories, error);
                                    }
                                    Err(e) => error.set(Some(format!("{e:?}"))),
                                }
                                pending.set(false);
                            });
                        }))
                    }),
                ])
            }),

            // Category tree
            html!("div", {
                .children_signal_vec(categories.signal_vec_cloned().map(clone!(error, notice => move |cat| {
                    render_category_row(cat, error.clone(), notice.clone())
                })))
            }),
        ])
    })
}

fn load_categories(categories: MutableVec<AdminCategorySummary>, error: Mutable<Option<String>>) {
    spawn_local(async move {
        match ApiCtx::get().client.admin_list_categories().await {
            Ok(res) => {
                categories.lock_mut().replace_cloned(res.categories);
            }
            Err(e) => error.set(Some(format!("{e:?}"))),
        }
    });
}

fn render_category_row(
    cat: AdminCategorySummary,
    error: Mutable<Option<String>>,
    notice: Mutable<Option<String>>,
) -> Dom {
    let indent = format!("{}rem", cat.depth as f64 * 1.5);
    let cat_id = cat.id.clone();
    let editing: Mutable<bool> = Mutable::new(false);
    let current_name: Mutable<String> = Mutable::new(cat.name.clone());
    let current_parent: Mutable<Option<ProductCategoryId>> = Mutable::new(cat.parent_id.clone());
    let name_buf = Arc::new(Mutex::new(cat.name.clone()));
    let parent_buf = Arc::new(Mutex::new(
        cat.parent_id
            .as_ref()
            .map(|p| p.as_str().to_string())
            .unwrap_or_default(),
    ));

    html!("div", {
        .style("padding", "0.4rem 0.75rem")
        .style("padding-left", &indent)
        .style("border-bottom", &format!("1px solid {}", color::LINE))
        .child_signal(editing.signal().map(clone!(cat_id, name_buf, parent_buf, current_name, current_parent, editing, error, notice => move |is_editing| {
            if is_editing {
                Some(render_edit(cat_id.clone(), name_buf.clone(), parent_buf.clone(), current_name.clone(), current_parent.clone(), editing.clone(), error.clone(), notice.clone()))
            } else {
                Some(render_view(cat_id.clone(), current_name.clone(), current_parent.clone(), name_buf.clone(), parent_buf.clone(), editing.clone(), error.clone()))
            }
        })))
    })
}

fn render_view(
    cat_id: ProductCategoryId,
    current_name: Mutable<String>,
    current_parent: Mutable<Option<ProductCategoryId>>,
    name_buf: Arc<Mutex<String>>,
    parent_buf: Arc<Mutex<String>>,
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
                        .text(cat_id.as_str())
                    }),
                    html!("span", {
                        .style("color", color::SUBTLE)
                        .style("margin-left", "0.5rem")
                        .style("font-size", "0.75rem")
                        .text_signal(current_parent.signal_cloned().map(|p| match p {
                            Some(pid) => format!("(parent: {})", pid.as_str()),
                            None => String::new(),
                        }))
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
                .event(clone!(current_name, current_parent, name_buf, parent_buf, editing => move |_: events::Click| {
                    *name_buf.lock().unwrap() = current_name.get_cloned();
                    *parent_buf.lock().unwrap() = current_parent
                        .get_cloned()
                        .map(|p| p.as_str().to_string())
                        .unwrap_or_default();
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
                    let id = cat_id.clone();
                    let error = error.clone();
                    spawn_local(async move {
                        match ApiCtx::get().client.admin_delete_category(&AdminDeleteCategoryRequest { id }).await {
                            Ok(_) => { let _ = web_sys::window().unwrap().location().reload(); }
                            Err(e) => error.set(Some(format!("{e:?}"))),
                        }
                    });
                }))
            }),
        ])
    })
}

#[allow(clippy::too_many_arguments)]
fn render_edit(
    cat_id: ProductCategoryId,
    name_buf: Arc<Mutex<String>>,
    parent_buf: Arc<Mutex<String>>,
    current_name: Mutable<String>,
    current_parent: Mutable<Option<ProductCategoryId>>,
    editing: Mutable<bool>,
    error: Mutable<Option<String>>,
    notice: Mutable<Option<String>>,
) -> Dom {
    let initial_name = name_buf.lock().unwrap().clone();
    let initial_parent = parent_buf.lock().unwrap().clone();
    html!("div", {
        .style("display", "grid")
        .style("grid-template-columns", "auto 1fr 1fr auto auto")
        .style("align-items", "end")
        .style("gap", "0.75rem")
        .children([
            html!("span", {
                .style("color", color::SUBTLE)
                .style("font-size", "0.8rem")
                .style("padding-bottom", "0.4rem")
                .text(cat_id.as_str())
            }),
            input_field("Name", "Display name", name_buf.clone(), &initial_name),
            input_field("Parent ID", "(optional) parent slug", parent_buf.clone(), &initial_parent),
            html!("button", {
                .style("cursor", "pointer")
                .style("color", color::GREEN)
                .style("font-size", "0.8rem")
                .style("background", "transparent")
                .style("border", "0")
                .text("Save")
                .event(clone!(cat_id, name_buf, parent_buf, current_name, current_parent, editing, error, notice => move |_: events::Click| {
                    let id = cat_id.clone();
                    let new_name = name_buf.lock().unwrap().clone();
                    let parent_val = parent_buf.lock().unwrap().clone();
                    let current_name = current_name.clone();
                    let current_parent = current_parent.clone();
                    let editing = editing.clone();
                    let error = error.clone();
                    let notice = notice.clone();
                    spawn_local(async move {
                        let parent_id_opt: Option<Option<ProductCategoryId>> = if parent_val.trim().is_empty() {
                            Some(None)
                        } else {
                            match ProductCategoryId::new(parent_val.trim()) {
                                Ok(p) => Some(Some(p)),
                                Err(e) => { error.set(Some(format!("Invalid parent: {e}"))); return; }
                            }
                        };
                        match ApiCtx::get().client.admin_update_category(&AdminUpdateCategoryRequest {
                            id,
                            name: Some(new_name),
                            parent_id: parent_id_opt,
                        }).await {
                            Ok(res) => {
                                current_name.set(res.category.name);
                                current_parent.set(res.category.parent_id);
                                editing.set(false);
                                notice.set(Some("Category updated".to_string()));
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
