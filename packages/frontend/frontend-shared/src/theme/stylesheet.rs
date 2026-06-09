use dominator::{stylesheet, stylesheet_raw};

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

    // Responsive classes. Defined as raw CSS so we can attach @media
    // overrides — dominator's `class!` macro doesn't natively support
    // media queries. Class names are prefixed `gs-` to avoid collisions
    // with the random `class!` names.
    stylesheet_raw(
        r#"
/* Two-column grid (sidebar + content) on desktop, stacked on mobile. */
.gs-home-grid {
  display: grid;
  grid-template-columns: minmax(0, 14rem) minmax(0, 1fr);
  gap: 1.5rem;
  align-items: start;
}

/* Sticky sidebar on desktop, static on mobile so the sidebar doesn't
   eat scroll position above the products grid. */
.gs-sidebar {
  position: sticky;
  top: 1rem;
  align-self: start;
  max-height: calc(100vh - 2rem);
  overflow-y: auto;
}

/* Product detail: image-left + actions-right on desktop, single
   column on tablets and below. */
.gs-detail-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 2rem;
  margin-top: 1.5rem;
  align-items: start;
}
.gs-detail-sticky {
  position: sticky;
  top: 1.5rem;
}

/* Section header — title + meta. Wraps to two rows on narrow screens
   so the meta count never crowds the title. */
.gs-section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem 1rem;
  flex-wrap: wrap;
  margin-bottom: 1rem;
}

/* Order card row layout. Stacks the actions column below the body on
   narrow screens so the Withdraw button doesn't get squeezed off. */
.gs-order-card {
  display: grid;
  grid-template-columns: auto 1fr auto;
  gap: 1rem;
  align-items: center;
}

@media (max-width: 720px) {
  .gs-home-grid {
    grid-template-columns: 1fr;
    gap: 1rem;
  }
  .gs-sidebar {
    position: static;
    max-height: none;
    overflow-y: visible;
  }
  /* Swap tree (desktop) for native dropdown (mobile). The select is
     hidden by an inline display:none so the layout doesn't flash a
     duplicate before the stylesheet kicks in. */
  .gs-cat-tree { display: none; }
  .gs-cat-select { display: block !important; }
}

@media (max-width: 820px) {
  .gs-detail-grid {
    grid-template-columns: 1fr;
    gap: 1.25rem;
  }
  .gs-detail-sticky {
    position: static;
  }
}

@media (max-width: 560px) {
  .gs-order-card {
    grid-template-columns: auto 1fr;
    grid-template-areas:
      "thumb body"
      "actions actions";
  }
  .gs-order-card > :nth-child(1) { grid-area: thumb; }
  .gs-order-card > :nth-child(2) { grid-area: body; }
  .gs-order-card > :nth-child(3) {
    grid-area: actions;
    align-items: stretch;
  }
}
"#,
    );
}
