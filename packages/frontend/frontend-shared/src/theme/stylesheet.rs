use dominator::stylesheet;

use crate::theme::color;

pub fn init() {
    stylesheet!("*", {
        .style("box-sizing", "border-box")
    });

    stylesheet!("html, body", {
        .style("margin", "0")
        .style("min-height", "100vh")
        .style("background", "#fafbfc")
        .style("color", color::INK)
        .style("font-family", "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, \"Helvetica Neue\", Arial, sans-serif")
        .style("-webkit-font-smoothing", "antialiased")
    });

    stylesheet!("body", {
        .style("padding", "0")
    });

    stylesheet!("img", {
        .style("max-width", "100%")
    });

    stylesheet!("a", {
        .style("color", "inherit")
    });
}
