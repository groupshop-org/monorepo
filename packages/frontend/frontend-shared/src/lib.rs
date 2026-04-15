#[macro_export]
macro_rules! required_build_env {
    ($key:literal $(,)?) => {
        match option_env!($key) {
            Some(value) => value,
            None => panic!(concat!("required build env var not set: ", $key)),
        }
    };
}

pub mod api;
pub mod constants;
pub mod error;
pub mod theme;
pub mod util;

pub fn document() -> web_sys::Document {
    web_sys::window()
        .expect("window should exist")
        .document()
        .expect("document should exist")
}

pub fn window() -> web_sys::Window {
    web_sys::window().expect("window should exist")
}
