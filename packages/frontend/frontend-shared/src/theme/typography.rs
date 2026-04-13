use std::sync::LazyLock;

use dominator::class;

use crate::theme::color;

pub static NAV_LABEL: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("font-size", "0.8rem")
        .style("font-weight", "600")
        .style("letter-spacing", "0.04em")
    }
});

pub static EYEBROW: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("margin", "0 0 0.25rem")
        .style("font-size", "0.72rem")
        .style("font-weight", "600")
        .style("letter-spacing", "0.12em")
        .style("text-transform", "uppercase")
        .style("opacity", "0.75")
    }
});

pub static BANNER_TITLE: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("margin", "0")
        .style("font-size", "clamp(1.6rem, 3.5vw, 2.6rem)")
        .style("font-weight", "800")
        .style("line-height", "1.1")
        .style("letter-spacing", "-0.02em")
        .style("text-transform", "uppercase")
    }
});

pub static STAT_LABEL: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("font-size", "0.72rem")
        .style("font-weight", "600")
        .style("letter-spacing", "0.12em")
        .style("text-transform", "uppercase")
        .style("opacity", "0.8")
    }
});

pub static STAT_VALUE: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("margin-top", "0.25rem")
        .style("font-size", "clamp(1.8rem, 3vw, 2.8rem)")
        .style("font-weight", "800")
        .style("letter-spacing", "-0.03em")
        .style("line-height", "1")
    }
});

pub static METRIC_VALUE: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("color", color::INK)
        .style("font-size", "1.15rem")
        .style("font-weight", "700")
        .style("letter-spacing", "-0.02em")
    }
});

pub static METRIC_LABEL: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("margin-top", "0.15rem")
        .style("color", color::MUTED)
        .style("font-size", "0.72rem")
        .style("font-weight", "500")
        .style("letter-spacing", "0.04em")
        .style("text-transform", "uppercase")
    }
});

pub static SECTION_TITLE: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("display", "flex")
        .style("align-items", "center")
        .style("gap", "0.6rem")
        .style("color", color::INK)
        .style("font-size", "0.85rem")
        .style("font-weight", "700")
        .style("letter-spacing", "0.10em")
        .style("text-transform", "uppercase")
    }
});

pub static LEAD_TEXT: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("max-width", "46rem")
        .style("margin", "0.75rem 0 0")
        .style("font-size", "1rem")
        .style("line-height", "1.7")
        .style("opacity", "0.85")
    }
});

pub static CARD_TITLE: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("color", color::INK)
        .style("font-size", "1rem")
        .style("font-weight", "600")
        .style("line-height", "1.35")
    }
});

pub static VISUAL_WORD: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("position", "absolute")
        .style("left", "0.75rem")
        .style("bottom", "0.5rem")
        .style("z-index", "1")
        .style("color", "rgba(255, 255, 255, 0.50)")
        .style("font-size", "2.8rem")
        .style("font-weight", "900")
        .style("letter-spacing", "0.04em")
        .style("text-transform", "uppercase")
        .style("pointer-events", "none")
    }
});

pub static VISUAL_CALLOUT: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("color", color::INK)
        .style("font-size", "0.78rem")
        .style("font-weight", "700")
    }
});

pub static PRICE_NOW: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("color", color::INK)
        .style("font-size", "1.5rem")
        .style("font-weight", "700")
        .style("letter-spacing", "-0.03em")
    }
});

pub static PRICE_THEN: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("color", color::SUBTLE)
        .style("font-size", "0.85rem")
        .style("font-weight", "500")
        .style("text-decoration", "line-through")
    }
});

pub static MICRO_LABEL: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("color", color::MUTED)
        .style("font-size", "0.7rem")
        .style("font-weight", "600")
        .style("letter-spacing", "0.06em")
        .style("text-transform", "uppercase")
    }
});

pub static MINI_HEADING: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("color", color::INK)
        .style("font-size", "1rem")
        .style("font-weight", "600")
        .style("letter-spacing", "-0.01em")
    }
});

pub static BODY: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("color", color::INK)
        .style("font-size", "1rem")
        .style("line-height", "1.7")
    }
});

pub static BODY_MUTED: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("color", color::MUTED)
        .style("font-size", "0.92rem")
        .style("line-height", "1.65")
    }
});

pub static FEED_NAME: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("color", color::INK)
        .style("font-size", "0.86rem")
        .style("font-weight", "600")
    }
});

pub static FEED_TEXT: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("margin", "0.2rem 0 0")
        .style("color", color::MUTED)
        .style("font-size", "0.78rem")
        .style("line-height", "1.5")
    }
});

pub static FOOTER_TEXT: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("color", color::MUTED)
        .style("font-size", "0.88rem")
    }
});

pub static LEGAL_TITLE: LazyLock<String> = LazyLock::new(|| {
    class! {
        .style("margin", "0")
        .style("font-size", "clamp(1.6rem, 3.5vw, 2.6rem)")
        .style("font-weight", "800")
        .style("line-height", "1.1")
        .style("letter-spacing", "-0.02em")
    }
});
