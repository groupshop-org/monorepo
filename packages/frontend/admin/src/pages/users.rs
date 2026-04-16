use dominator::{clone, events, html, Dom};
use futures_signals::signal::{Mutable, SignalExt};
use groupshop_backend_shared::prelude::*;
use groupshop_frontend_shared::theme::{color, typography};
use wasm_bindgen_futures::spawn_local;

use crate::api::ApiCtx;

pub fn render() -> Dom {
    let users: Mutable<Option<Vec<AdminUserSummary>>> = Mutable::new(None);
    let error: Mutable<Option<String>> = Mutable::new(None);

    spawn_local(clone!(users, error => async move {
        match ApiCtx::get().client.admin_list_users(&AdminListUsersRequest {
            page: 1,
            per_page: 50,
        }).await {
            Ok(res) => users.set(Some(res.users)),
            Err(e) => error.set(Some(format!("{e:?}"))),
        }
    }));

    html!("div", {
                .children([
            html!("h1", {
                .class(&*typography::SECTION_TITLE)
                .text("Users")
            }),
            html!("div", {
                .child_signal(error.signal_cloned().map(|err| {
                    err.map(|msg| html!("p", {
                        .style("color", color::RED)
                        .text(&msg)
                    }))
                }))
            }),
            html!("div", {
                .child_signal(users.signal_cloned().map(clone!(error => move |maybe_users| {
                    match maybe_users {
                        None => Some(html!("p", { .class(&*typography::BODY_MUTED) .text("Loading users...") })),
                        Some(users) => Some(html!("div", {
                            .style("display", "flex")
                            .style("flex-direction", "column")
                            .style("gap", "0.75rem")
                            .children(users.into_iter().map(|user| {
                                render_user_row(user, error.clone())
                            }).collect::<Vec<_>>())
                        })),
                    }
                })))
            }),
        ])
    })
}

fn render_user_row(user: AdminUserSummary, error: Mutable<Option<String>>) -> Dom {
    let is_admin = Mutable::new(user.roles.contains(&UserRole::Admin));
    let user_id = user.id.clone();

    html!("div", {
        .style("display", "flex")
        .style("align-items", "center")
        .style("gap", "1rem")
        .style("padding", "0.75rem")
        .style("border", &format!("1px solid {}", color::LINE))
        .style("border-radius", "0.375rem")
        .style("background", color::PAPER)
        .children([
            html!("div", {
                .style("flex", "1")
                .children([
                    html!("strong", { .text(&user.username) }),
                    html!("span", {
                        .style("color", color::MUTED)
                        .style("margin-left", "0.5rem")
                        .text(&user.email.unwrap_or_default())
                    }),
                    html!("div", {
                        .style("font-size", "0.8rem")
                        .style("color", color::SUBTLE)
                        .text(&format!("Roles: {:?}", user.roles))
                    }),
                ])
            }),
            html!("label", {
                .style("display", "flex")
                .style("align-items", "center")
                .style("gap", "0.25rem")
                .style("cursor", "pointer")
                .children([
                    html!("input" => web_sys::HtmlInputElement, {
                        .attr("type", "checkbox")
                        .prop_signal("checked", is_admin.signal())
                        .event(clone!(is_admin => move |_: events::Change| {
                            is_admin.set(!is_admin.get());
                        }))
                    }),
                    html!("span", { .text("Admin") }),
                ])
            }),
            html!("button", {
                .style("cursor", "pointer")
                .style("padding", "0.4rem 1rem")
                .style("border-radius", "0.375rem")
                .style("border", &format!("1px solid {}", color::LINE))
                .style("background", color::PAPER)
                .style("color", color::INK)
                .style("font-size", "0.85rem")
                .text("Save")
                .event(clone!(user_id, is_admin, error => move |_: events::Click| {
                    let user_id = user_id.clone();
                    let is_admin = is_admin.clone();
                    let error = error.clone();
                    spawn_local(async move {
                        let mut roles = vec![
                            UserRole::PartialRegistration,
                            UserRole::EmailVerified,
                            UserRole::UsernameChosen,
                        ];
                        if is_admin.get() {
                            roles.push(UserRole::Admin);
                        }
                        match ApiCtx::get().client.admin_update_user(&AdminUpdateUserRequest {
                            id: user_id,
                            roles,
                        }).await {
                            Ok(_) => { let _ = web_sys::window().unwrap().location().reload(); }
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
                .style("color", color::RED)
                .style("font-size", "0.85rem")
                .text("Delete")
                .event(clone!(error => move |_: events::Click| {
                    let user_id = user.id.clone();
                    let error = error.clone();
                    spawn_local(async move {
                        match ApiCtx::get().client.admin_delete_user(&AdminDeleteUserRequest {
                            id: user_id,
                        }).await {
                            Ok(_) => { let _ = web_sys::window().unwrap().location().reload(); }
                            Err(e) => error.set(Some(format!("{e:?}"))),
                        }
                    });
                }))
            }),
        ])
    })
}
