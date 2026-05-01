//! Shared UI helpers for "X of N orders placed" threshold progress.
//!
//! Each group deal has a threshold (`minimum_order_quantity`); we render a
//! short label plus a horizontal progress bar that fills as more buyers
//! join. Used on the product detail page, the My Orders page, and on the
//! product cards in the landing-page grid.

use dominator::{clone, html, Dom};
use futures_signals::signal::{Mutable, Signal, SignalExt};
use groupshop_frontend_shared::theme::color;

/// Compact "5 / 6 units" label + progress bar. `compact = true` shrinks
/// the bar height for use inside dense product cards.
///
/// Static variant — call sites that already get a fresh
/// `committed_units` on every render can use this. Anything that needs
/// live polling without re-rendering its surroundings (e.g. the product
/// detail page, where the deposit panel holds wallet state) should use
/// `progress_signal` instead.
pub fn progress(committed_units: u64, threshold: u32, compact: bool) -> Dom {
    let count = Mutable::new(committed_units);
    progress_signal(count.signal(), threshold, compact)
}

/// Reactive variant of `progress`: subscribes to a signal so updates flow
/// through without re-creating the surrounding DOM. Important for views
/// where neighboring components hold local state that mustn't be reset
/// on each poll tick.
pub fn progress_signal<S>(count: S, threshold: u32, compact: bool) -> Dom
where
    S: Signal<Item = u64> + 'static,
{
    let bar_height = if compact { "0.35rem" } else { "0.5rem" };
    // Mirror the signal into a Mutable so several DOM nodes can read it
    // without tying up the original signal.
    let count = {
        let mirror = Mutable::new(0u64);
        let mirror_clone = mirror.clone();
        wasm_bindgen_futures::spawn_local(async move {
            count
                .for_each(move |v| {
                    mirror_clone.set(v);
                    async {}
                })
                .await;
        });
        mirror
    };

    let threshold_u64 = threshold as u64;
    html!("div", {
        .style("width", "100%")
        .child(html!("div", {
            .style("display", "flex")
            .style("justify-content", "space-between")
            .style("align-items", "baseline")
            .style("margin-bottom", "0.25rem")
            .children([
                html!("span", {
                    .style("font-size", if compact { "0.72rem" } else { "0.82rem" })
                    .style_signal("color", count.signal().map(move |c| {
                        if c >= threshold_u64 && threshold_u64 > 0 { "#15803d" } else { "#374151" }
                    }))
                    .style_signal("font-weight", count.signal().map(move |c| {
                        if c >= threshold_u64 && threshold_u64 > 0 { "600" } else { "500" }
                    }))
                    .text_signal(count.signal().map(move |c| format!("{c} of {threshold} units")))
                }),
                html!("span", {
                    .style("font-size", if compact { "0.7rem" } else { "0.78rem" })
                    .style("color", "#6b7280")
                    .text_signal(count.signal().map(move |c| {
                        if c >= threshold_u64 && threshold_u64 > 0 {
                            "Threshold met".to_string()
                        } else {
                            format!("{}%", pct_of(c, threshold_u64))
                        }
                    }))
                }),
            ])
        }))
        .child(html!("div", {
            .style("width", "100%")
            .style("height", bar_height)
            .style("background", "#e5e7eb")
            .style("border-radius", "9999px")
            .style("overflow", "hidden")
            .child(html!("div", {
                .style("height", "100%")
                .style_signal("width", count.signal().map(move |c| format!("{}%", pct_of(c, threshold_u64))))
                // Green once the threshold is reached, gradient blue→green
                // while still recruiting.
                .style_signal("background", clone!(count => count.signal().map(move |c| {
                    if c >= threshold_u64 && threshold_u64 > 0 {
                        "#22c55e".to_string()
                    } else {
                        format!("linear-gradient(90deg, {} 0%, #22c55e 100%)", color::BLUE)
                    }
                })))
                .style("transition", "width 0.6s ease")
            }))
        }))
    })
}

fn pct_of(numer: u64, denom: u64) -> u32 {
    if denom == 0 {
        return 0;
    }
    let ratio = (numer.saturating_mul(100)) / denom;
    ratio.min(100) as u32
}
