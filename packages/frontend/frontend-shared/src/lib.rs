pub fn document() -> web_sys::Document {
    web_sys::window()
        .expect("window should exist")
        .document()
        .expect("document should exist")
}
