use dominator::{events, html, Dom};
use groupshop_frontend_shared::theme::{chrome, color, typography};

use crate::route::Route;

pub fn render() -> Dom {
    html!("div", {
        .children([
            html!("h1", {
                .class(&*typography::SECTION_TITLE)
                .text("Admin Dashboard")
            }),
            html!("p", {
                .class(&*typography::BODY_MUTED)
                .style("margin", "0 0 2rem")
                .text("Manage products, categories, brands, and users.")
            }),
            html!("div", {
                .style("display", "grid")
                .style("grid-template-columns", "repeat(auto-fill, minmax(14rem, 1fr))")
                .style("gap", "1.5rem")
                .children([
                    tile("Products", "Manage the product catalog, pricing, and inventory.", Route::Products),
                    tile("Categories", "Organize products into hierarchical categories.", Route::Categories),
                    tile("Brands", "Manage brand listings and metadata.", Route::Brands),
                    tile("Users", "Manage user accounts, roles, and access.", Route::Users),
                ])
            }),
        ])
    })
}

fn tile(title: &str, description: &str, route: Route) -> Dom {
    let title = title.to_string();
    let description = description.to_string();

    html!("div", {
        .class(&*chrome::CARD)
        .style("margin-top", "0")
        .style("cursor", "pointer")
        .style("user-select", "none")
        .style("transition", "box-shadow 180ms ease, transform 180ms ease")
        .event(move |_: events::Click| {
            route.go_to_url();
        })
        .children([
            html!("h3", {
                .style("margin", "0 0 0.5rem")
                .style("font-size", "1.1rem")
                .style("color", color::INK)
                .text(&title)
            }),
            html!("p", {
                .style("margin", "0")
                .class(&*typography::BODY_MUTED)
                .text(&description)
            }),
        ])
    })
}
