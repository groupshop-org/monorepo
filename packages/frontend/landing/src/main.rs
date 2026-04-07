use groupshop_frontend_shared::document;

fn main() {
    let document = document();
    let app = document
        .get_element_by_id("app")
        .expect("app container should exist");

    app.set_inner_html("<h1>Coming Soon</h1>");
}
