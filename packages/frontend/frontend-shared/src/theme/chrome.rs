use std::sync::LazyLock;

use dominator::{class, pseudo};

use crate::theme::color;

// ---------------------------------------------------------------------------
// Page shells
// ---------------------------------------------------------------------------

pub static PAGE: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("min-height", "100vh")
        .style("padding", "0.75rem clamp(1rem, 2.5vw, 2rem)")
    }
});

pub static SHELL: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("width", "100%")
        .style("max-width", "82rem")
        .style("margin", "0 auto")
    }
});

pub static LEGAL_SHELL: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("width", "min(100%, 64rem)")
        .style("margin", "0 auto")
    }
});

// ---------------------------------------------------------------------------
// Header / masthead
// ---------------------------------------------------------------------------

pub static MASTHEAD: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "flex")
        .style("flex-wrap", "wrap")
        .style("align-items", "center")
        .style("justify-content", "space-between")
        .style("gap", "0.75rem 1.25rem")
        .style("padding", "0.75rem 0")
        .style("border-bottom", format!("1px solid {}", color::LINE))
    }
});

pub static BRAND: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "inline-flex")
        .style("align-items", "center")
        .style("gap", "0.5rem")
        .style("text-decoration", "none")
    }
});

pub static BRAND_BADGE: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "inline-flex")
        .style("align-items", "center")
    }
});

pub static LOGO_IMG: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "block")
        .style("height", "1.75rem")
        .style("width", "auto")
        .style("object-fit", "contain")
    }
});

pub static BRAND_WORD: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("color", "#1e9b5a")
        .style("font-size", "1.25rem")
        .style("font-weight", "700")
        .style("letter-spacing", "-0.03em")
    }
});

pub static NAV: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "flex")
        .style("flex-wrap", "wrap")
        .style("align-items", "center")
        .style("gap", "1.5rem")
    }
});

pub static NAV_LINK: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("color", color::MUTED)
        .style("text-decoration", "none")
        .style("user-select", "none")
        .style("cursor", "pointer")
        .style("transition", "color 150ms ease")
        .pseudo!(":hover", {
            .style("color", color::INK)
        })
    }
});

pub static NAV_RIGHT: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "flex")
        .style("flex-wrap", "wrap")
        .style("gap", "0.5rem")
        .style("align-items", "center")
    }
});

pub static PILL: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "inline-flex")
        .style("align-items", "center")
        .style("justify-content", "center")
        .style("padding", "0.22rem 0.5rem")
        .style("border", format!("1px solid {}", color::LINE))
        .style("border-radius", "999px")
        .style("background", "#ffffff")
        .style("color", color::MUTED)
        .style("font-size", "0.6rem")
        .style("font-weight", "600")
        .style("letter-spacing", "0.04em")
        .style("text-transform", "uppercase")
        .style("user-select", "none")
    }
});

pub static STATUS_PILL: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("position", "relative")
        .style("padding-left", "1.1rem")
        .pseudo!("::before", {
            .style("content", "\"\"")
            .style("position", "absolute")
            .style("left", "0.45rem")
            .style("top", "50%")
            .style("width", "0.32rem")
            .style("height", "0.32rem")
            .style("margin-top", "-0.16rem")
            .style("border-radius", "999px")
            .style("background", color::GREEN)
        })
    }
});

pub static WALLET_PILL: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("border-color", color::LINE_STRONG)
    }
});

// ---------------------------------------------------------------------------
// Banner (compact hero)
// ---------------------------------------------------------------------------

pub static BANNER: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "flex")
        .style("flex-wrap", "wrap")
        .style("align-items", "center")
        .style("justify-content", "space-between")
        .style("gap", "1rem 2rem")
        .style("margin-top", "1rem")
        .style("padding", "clamp(1.25rem, 2.5vw, 2rem) clamp(1.5rem, 3vw, 2.5rem)")
        .style("border-radius", "0.75rem")
        .style("background", color::GRADIENT_BRAND)
        .style("color", "#ffffff")
    }
});

pub static BANNER_COPY: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "flex")
        .style("flex-direction", "column")
        .style("gap", "0.25rem")
    }
});

pub static BANNER_STAT: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("text-align", "right")
    }
});

// ---------------------------------------------------------------------------
// Main layout (deals + sidebar)
// ---------------------------------------------------------------------------

pub static MAIN_LAYOUT: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "flex")
        .style("flex-wrap", "wrap")
        .style("gap", "1.5rem")
        .style("margin-top", "2rem")
    }
});

pub static CONTENT_AREA: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("flex", "1 1 32rem")
        .style("min-width", "0")
    }
});

pub static SIDEBAR: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("flex", "0 1 20rem")
        .style("min-width", "0")
    }
});

pub static SIDEBAR_STATS: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "grid")
        .style("gap", "0.75rem")
        .style("margin-top", "1rem")
    }
});

pub static SIDEBAR_STAT: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("padding", "0.85rem 1rem")
        .style("border", format!("1px solid {}", color::LINE))
        .style("border-radius", "0.5rem")
        .style("background", "#ffffff")
    }
});

// ---------------------------------------------------------------------------
// Section headers
// ---------------------------------------------------------------------------

pub static SECTION_HEADER: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "flex")
        .style("align-items", "center")
        .style("justify-content", "space-between")
        .style("flex-wrap", "wrap")
        .style("gap", "0.5rem 1rem")
        .style("margin-bottom", "1rem")
    }
});

pub static SECTION_MARK: LazyLock<String> = LazyLock::new(|| {
    class! {
        .pseudo!("::before", {
            .style("content", "\"\"")
            .style("display", "block")
            .style("width", "0.22rem")
            .style("height", "1rem")
            .style("border-radius", "999px")
            .style("background", color::GRADIENT_BRAND)
        })
    }
});

pub static SECTION_LINK: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("color", color::BLUE)
        .style("font-size", "0.78rem")
        .style("font-weight", "600")
        .style("letter-spacing", "0.04em")
        .style("text-decoration", "none")
        .pseudo!(":hover", {
            .style("text-decoration", "underline")
        })
    }
});

// ---------------------------------------------------------------------------
// Deal cards
// ---------------------------------------------------------------------------

pub static DEAL_GRID: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "grid")
        .style("grid-template-columns", "repeat(auto-fill, minmax(min(14rem, 100%), 1fr))")
        .style("gap", "1rem")
    }
});

pub static DEAL_CARD: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("border", format!("1px solid {}", color::LINE))
        .style("border-radius", "0.6rem")
        .style("background", "#ffffff")
        .style("overflow", "hidden")
        .style("box-shadow", "0 1px 3px rgba(0,0,0,0.06)")
        .style("transition", "box-shadow 180ms ease, transform 180ms ease")
        .pseudo!(":hover", {
            .style("box-shadow", "0 6px 20px rgba(0,0,0,0.10)")
            .style("transform", "translateY(-2px)")
        })
    }
});

pub static DEAL_VISUAL: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("position", "relative")
        .style("height", "10rem")
        .style("overflow", "hidden")
    }
});

pub static DEAL_VISUAL_RED: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("background", "linear-gradient(135deg, #ff6b6b 0%, #ee5a24 100%)")
    }
});

pub static DEAL_VISUAL_SILVER: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("background", "linear-gradient(135deg, #89f7fe 0%, #66a6ff 100%)")
    }
});

pub static DEAL_VISUAL_YELLOW: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("background", "linear-gradient(135deg, #ffd200 0%, #f7971e 100%)")
    }
});

pub static DEAL_VISUAL_WHITE: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("background", "linear-gradient(135deg, #6c5ce7 0%, #a29bfe 100%)")
    }
});

pub static DEAL_BADGE: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("position", "absolute")
        .style("top", "0.65rem")
        .style("left", "0.65rem")
        .style("z-index", "1")
        .style("padding", "0.3rem 0.55rem")
        .style("border-radius", "0.3rem")
        .style("background", "rgba(0, 0, 0, 0.32)")
        .style("color", "#ffffff")
        .style("font-size", "0.6rem")
        .style("font-weight", "700")
        .style("letter-spacing", "0.06em")
        .style("text-transform", "uppercase")
    }
});

pub static DEAL_CALLOUT: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("position", "absolute")
        .style("right", "0.65rem")
        .style("top", "0.65rem")
        .style("z-index", "1")
        .style("padding", "0.3rem 0.55rem")
        .style("border-radius", "999px")
        .style("background", "rgba(255, 255, 255, 0.92)")
        .style("box-shadow", "0 2px 6px rgba(0, 0, 0, 0.10)")
    }
});

pub static DEAL_BODY: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("padding", "0.85rem")
    }
});

pub static PRICE_ROW: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "flex")
        .style("align-items", "baseline")
        .style("gap", "0.5rem")
        .style("margin-top", "0.4rem")
    }
});

pub static PROGRESS_HEAD: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "flex")
        .style("justify-content", "space-between")
        .style("gap", "0.5rem")
        .style("margin-top", "0.75rem")
    }
});

pub static PROGRESS_BAR: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("height", "0.35rem")
        .style("margin-top", "0.4rem")
        .style("border-radius", "999px")
        .style("background", "#e5e7eb")
        .style("overflow", "hidden")
    }
});

pub static PROGRESS_FILL: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("height", "100%")
        .style("border-radius", "999px")
        .style("background", color::GRADIENT_BRAND)
    }
});

// ---------------------------------------------------------------------------
// Buttons
// ---------------------------------------------------------------------------

pub static BUTTON: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "inline-flex")
        .style("align-items", "center")
        .style("justify-content", "center")
        .style("min-height", "2.4rem")
        .style("padding", "0.55rem 1.1rem")
        .style("border", format!("1px solid {}", color::LINE))
        .style("border-radius", "0.4rem")
        .style("background", "#ffffff")
        .style("color", color::INK)
        .style("font-size", "0.82rem")
        .style("font-weight", "600")
        .style("text-decoration", "none")
        .style("user-select", "none")
        .style("cursor", "pointer")
        .style("transition", "background 150ms ease, box-shadow 150ms ease, transform 150ms ease")
        .pseudo!(":hover", {
            .style("transform", "translateY(-1px)")
            .style("box-shadow", "0 3px 10px rgba(0,0,0,0.08)")
        })
    }
});

pub static BUTTON_PRIMARY: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("background", color::GRADIENT_BRAND)
        .style("color", "#ffffff")
        .style("border-color", "transparent")
    }
});

pub static BUTTON_SOFT: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("background", "rgba(0, 0, 0, 0.03)")
        .style("color", color::INK)
    }
});

pub static BUTTON_DARK: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("width", "100%")
        .style("margin-top", "0.65rem")
        .style("background", color::GRADIENT_BRAND)
        .style("color", "#ffffff")
        .style("border-color", "transparent")
    }
});

pub static BUTTON_GRADIENT: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("background", color::GRADIENT_BRAND)
        .style("color", "#ffffff")
        .style("border-color", "transparent")
    }
});

pub static DEAL_META: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("margin-top", "0.5rem")
        .style("color", color::MUTED)
        .style("font-size", "0.75rem")
        .style("line-height", "1.5")
    }
});

// ---------------------------------------------------------------------------
// Feed card (sidebar)
// ---------------------------------------------------------------------------

pub static FEED_CARD: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("border", format!("1px solid {}", color::LINE))
        .style("border-radius", "0.6rem")
        .style("background", "#ffffff")
        .style("overflow", "hidden")
        .style("box-shadow", "0 1px 3px rgba(0,0,0,0.06)")
    }
});

pub static FEED_HEADER: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("padding", "0.85rem 1rem")
        .style("border-bottom", format!("1px solid {}", color::BORDER_SOFT))
    }
});

pub static FEED_TITLE: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "flex")
        .style("align-items", "center")
        .style("gap", "0.5rem")
        .style("color", color::INK)
        .style("font-size", "0.88rem")
        .style("font-weight", "700")
        .pseudo!("::before", {
            .style("content", "\"\"")
            .style("display", "block")
            .style("width", "0.2rem")
            .style("height", "1.1rem")
            .style("border-radius", "999px")
            .style("background", color::GRADIENT_BRAND)
        })
    }
});

pub static FEED_ITEM: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("padding", "0.75rem 1rem")
        .style("border-bottom", format!("1px solid {}", color::BORDER_SOFT))
    }
});

pub static FEED_FOOTER: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("padding", "0.75rem 1rem")
    }
});

// ---------------------------------------------------------------------------
// Signal strip (value props)
// ---------------------------------------------------------------------------

pub static SIGNAL_GRID: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "grid")
        .style("grid-template-columns", "repeat(auto-fit, minmax(16rem, 1fr))")
        .style("gap", "1rem")
        .style("margin-top", "2rem")
    }
});

pub static SIGNAL_CARD: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("padding", "1.15rem")
        .style("border", format!("1px solid {}", color::LINE))
        .style("border-radius", "0.6rem")
        .style("background", "#ffffff")
        .style("box-shadow", "0 1px 3px rgba(0,0,0,0.06)")
    }
});

// ---------------------------------------------------------------------------
// Story section (how it works / why)
// ---------------------------------------------------------------------------

pub static STORY_GRID: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "grid")
        .style("grid-template-columns", "repeat(auto-fit, minmax(20rem, 1fr))")
        .style("gap", "1rem")
        .style("margin-top", "2rem")
    }
});

pub static PANEL: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("padding", "1.25rem")
        .style("border", format!("1px solid {}", color::LINE))
        .style("border-radius", "0.6rem")
        .style("background", "#ffffff")
        .style("box-shadow", "0 1px 3px rgba(0,0,0,0.06)")
    }
});

pub static STEP_GRID: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "grid")
        .style("grid-template-columns", "repeat(auto-fit, minmax(12rem, 1fr))")
        .style("gap", "0.75rem")
        .style("margin-top", "0.85rem")
    }
});

pub static STEP_CARD: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("padding", "0.85rem")
        .style("border", format!("1px solid {}", color::LINE))
        .style("border-radius", "0.5rem")
        .style("background", color::BG_DEEP)
    }
});

pub static INFO_LIST: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "grid")
        .style("gap", "0.75rem")
        .style("margin-top", "0.85rem")
    }
});

pub static INFO_ITEM: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("padding", "0.85rem")
        .style("border", format!("1px solid {}", color::LINE))
        .style("border-radius", "0.5rem")
        .style("background", color::BG_DEEP)
    }
});

// ---------------------------------------------------------------------------
// Notes
// ---------------------------------------------------------------------------

pub static NOTE: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("margin-top", "0")
        .style("padding", "0.75rem 0.85rem")
        .style("border", format!("1px solid {}", color::LINE))
        .style("border-radius", "0.4rem")
        .style("background", color::NOTE_BG)
        .style("color", color::NOTE_TEXT)
        .style("font-size", "0.82rem")
        .style("line-height", "1.55")
    }
});

// ---------------------------------------------------------------------------
// Footer
// ---------------------------------------------------------------------------

pub static FOOTER: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("margin-top", "2rem")
        .style("padding", "1.25rem 0")
        .style("border-top", format!("1px solid {}", color::LINE))
    }
});

pub static FOOTER_LINK: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("color", color::BLUE)
        .style("text-decoration", "none")
        .pseudo!(":hover", {
            .style("text-decoration", "underline")
        })
    }
});

// ---------------------------------------------------------------------------
// Legal pages
// ---------------------------------------------------------------------------

pub static LEGAL_HERO: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("margin-top", "1rem")
        .style("padding", "clamp(1.25rem, 2.5vw, 2rem) clamp(1.5rem, 3vw, 2.5rem)")
        .style("border-radius", "0.75rem")
        .style("background", color::GRADIENT_BRAND)
        .style("color", "#ffffff")
    }
});

pub static CARD: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("margin-top", "1rem")
        .style("padding", "clamp(1.25rem, 2vw, 2rem)")
        .style("border", format!("1px solid {}", color::LINE))
        .style("border-radius", "0.6rem")
        .style("background", "#ffffff")
        .style("box-shadow", "0 1px 3px rgba(0,0,0,0.06)")
    }
});

pub static LEGAL_BODY: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("color", color::INK)
        .style("font-size", "1rem")
        .style("line-height", "1.75")
    }
});
