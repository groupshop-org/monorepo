mod config;
mod legal;

use dominator::{append_dom, body, html, Dom};
use groupshop_frontend_shared::{
    document, theme,
    theme::{chrome, typography},
    window,
};

use crate::legal::{privacy_policy, terms_of_service, PageContent};

enum Route {
    Home,
    PrivacyPolicy,
    TermsOfService,
    NotFound,
}

fn main() {
    theme::init();

    let route = current_route();
    let title = page_title(&route);
    document().set_title(title);

    append_dom(&body(), render(route));
}

fn current_route() -> Route {
    let path = window()
        .location()
        .pathname()
        .unwrap_or_else(|_| "/".to_string());

    match path.as_str() {
        "/" | "" => Route::Home,
        "/privacy-policy" | "/privacy-policy/" => Route::PrivacyPolicy,
        "/terms-of-service" | "/terms-of-service/" => Route::TermsOfService,
        _ => Route::NotFound,
    }
}

fn page_title(route: &Route) -> &'static str {
    match route {
        Route::Home => "GROUPSHOP",
        Route::PrivacyPolicy => "Privacy Policy | GROUPSHOP",
        Route::TermsOfService => "Terms of Service | GROUPSHOP",
        Route::NotFound => "Page Not Found | GROUPSHOP",
    }
}

fn render(route: Route) -> Dom {
    html!("main", {
        .class(&*chrome::PAGE)
        .child(match route {
            Route::Home => render_home(),
            Route::PrivacyPolicy => render_legal(
                privacy_policy(),
                "How Groupshop collects, uses, shares, and protects information in connection with the website and service.",
                Some("This page is a product-integrated legal draft based on the current Groupshop service model, including on-chain participation and physical-goods fulfillment workflows."),
            ),
            Route::TermsOfService => render_legal(
                terms_of_service(),
                "The terms governing access to the Groupshop website, accounts, group deals, wallet interactions, orders, and related services.",
                Some("These terms are written to match the current Groupshop product direction: on-chain deal coordination, off-chain purchasing, and real-world fulfillment."),
            ),
            Route::NotFound => render_legal(
                PageContent {
                    title: "Page not found.",
                    html: r#"
                        <p>The page you requested does not exist on the current landing site.</p>
                        <div>
                            <a href="/">Back to Home</a>
                        </div>
                    "#,
                },
                "The page you requested does not exist on the current landing site.",
                None,
            ),
        })
    })
}

fn render_home() -> Dom {
    html!("div", {
        .class(&*chrome::SHELL)
        .child(site_header())
        .child(hero_banner())
        .child(html!("div", {
            .class(&*chrome::MAIN_LAYOUT)
            .child(deals_column())
            .child(sidebar())
        }))
        .child(signal_strip())
        .child(story_section())
        .child(site_footer())
    })
}

fn render_legal(page: PageContent, lead: &'static str, note: Option<&'static str>) -> Dom {
    html!("div", {
        .class(&*chrome::LEGAL_SHELL)
        .child(site_header())
        .child(html!("section", {
            .class(&*chrome::LEGAL_HERO)
            .child(html!("h1", {
                .class(&*typography::LEGAL_TITLE)
                .text(page.title)
            }))
            .child(html!("p", {
                .class(&*typography::LEAD_TEXT)
                .text(lead)
            }))
        }))
        .child(html!("article", {
            .class(&*chrome::CARD)
            .child(html!("div", {
                .class(&*chrome::LEGAL_BODY)
                .after_inserted(move |element| {
                    element.set_inner_html(page.html);
                })
            }))
            .apply_if(note.is_some(), move |dom| {
                dom.child(html!("div", {
                    .class(&*chrome::NOTE)
                    .text(note.expect("note exists"))
                }))
            })
        }))
        .child(site_footer())
    })
}

fn site_header() -> Dom {
    html!("header", {
        .class(&*chrome::MASTHEAD)
        .child(html!("a", {
            .class(&*chrome::BRAND)
            .attr("href", "/")
            .attr("aria-label", "Groupshop home")
            .child(html!("img", {
                .class(&*chrome::LOGO_IMG)
                .attr("src", &config::media_link("logo-icon.png"))
                .attr("alt", "")
            }))
            .child(html!("span", {
                .class(&*chrome::BRAND_WORD)
                .text("Groupshop")
            }))
        }))
        .child(html!("nav", {
            .class(&*chrome::NAV)
            .attr("aria-label", "Primary")
            .children([
                nav_link("/#deals", "Explore Deals"),
                nav_link("/#escrow", "Escrow Flow"),
                nav_link("/#how-it-works", "How It Works"),
            ])
        }))
        .child(html!("div", {
            .class(&*chrome::NAV_RIGHT)
            .children([
                html!("div", {
                    .class(&*chrome::PILL)
                    .class(&*chrome::STATUS_PILL)
                    .text("Solana Ready")
                }),
                html!("div", {
                    .class(&*chrome::PILL)
                    .class(&*chrome::WALLET_PILL)
                    .text("Preview Only")
                }),
            ])
        }))
    })
}

fn nav_link(href: &'static str, text: &'static str) -> Dom {
    html!("a", {
        .class(&*chrome::NAV_LINK)
        .class(&*typography::NAV_LABEL)
        .attr("href", href)
        .text(text)
    })
}

fn hero_banner() -> Dom {
    html!("section", {
        .class(&*chrome::BANNER)
        .child(html!("div", {
            .class(&*chrome::BANNER_COPY)
            .child(html!("p", {
                .class(&*typography::EYEBROW)
                .text("Real-world group buying, secured on-chain")
            }))
            .child(html!("h1", {
                .class(&*typography::BANNER_TITLE)
                .text("Scale together. Pay wholesale.")
            }))
        }))
        .child(html!("div", {
            .class(&*chrome::BANNER_STAT)
            .children([
                html!("div", {
                    .class(&*typography::STAT_LABEL)
                    .text("Total Locked")
                }),
                html!("div", {
                    .class(&*typography::STAT_VALUE)
                    .text("$1,242,090")
                }),
            ])
        }))
    })
}

fn deals_column() -> Dom {
    html!("div", {
        .class(&*chrome::CONTENT_AREA)
        .attr("id", "deals")
        .child(html!("div", {
            .class(&*chrome::SECTION_HEADER)
            .children([
                section_title("Featured Deals"),
                html!("a", {
                    .class(&*chrome::SECTION_LINK)
                    .attr("href", "/#escrow")
                    .text("View Escrow Flow \u{2192}")
                }),
            ])
        }))
        .child(html!("div", {
            .class(&*chrome::DEAL_GRID)
            .children([
                deal_card("Footwear", &*chrome::DEAL_VISUAL_RED, "Runner", "44% off", "Hyperion Performance Runner", "$89.00", "$160.00", "82/100 joined", "82%", "Escrow converts only after the threshold clears."),
                deal_card("Wearables", &*chrome::DEAL_VISUAL_SILVER, "Timepiece", "51% off", "Nordic Minimalist Timepiece", "$145.00", "$295.00", "45/50 joined", "90%", "Near threshold with visible on-chain commitments."),
                deal_card("Audio", &*chrome::DEAL_VISUAL_YELLOW, "Headphones", "43% off", "Studio Master ANC Headphones", "$199.00", "$349.00", "122/500 joined", "24%", "Large-batch savings target with transparent progress."),
                deal_card("Workstation", &*chrome::DEAL_VISUAL_WHITE, "Keyboard", "41% off", "Mechanical Ergo Keyboard", "$110.00", "$185.00", "210/250 joined", "84%", "Crowd almost filled; pricing unlock is within reach."),
            ])
        }))
    })
}

fn sidebar() -> Dom {
    html!("aside", {
        .class(&*chrome::SIDEBAR)
        .child(feed_card())
        .child(html!("div", {
            .class(&*chrome::SIDEBAR_STATS)
            .children([
                sidebar_stat("4", "Live Deals"),
                sidebar_stat("44%", "Best Savings"),
                sidebar_stat("USDC", "Escrow Rail"),
            ])
        }))
    })
}

fn sidebar_stat(value: &'static str, label: &'static str) -> Dom {
    html!("div", {
        .class(&*chrome::SIDEBAR_STAT)
        .children([
            html!("div", {
                .class(&*typography::METRIC_VALUE)
                .text(value)
            }),
            html!("div", {
                .class(&*typography::METRIC_LABEL)
                .text(label)
            }),
        ])
    })
}

fn feed_card() -> Dom {
    html!("div", {
        .class(&*chrome::FEED_CARD)
        .attr("id", "escrow")
        .child(html!("div", {
            .class(&*chrome::FEED_HEADER)
            .child(html!("div", {
                .class(&*chrome::FEED_TITLE)
                .text("Live Escrow Feed")
            }))
        }))
        .children([
            feed_item("jake_sol\u{2026}4x9", "committed 89.00 USDC", "2 mins ago"),
            feed_item("anon\u{2026}k21", "committed 145.00 USDC", "5 mins ago"),
            feed_item("degen_king\u{2026}f2", "committed 199.00 USDC", "8 mins ago"),
            feed_item("builder_dev", "unlocked the runner threshold", "15 mins ago"),
        ])
        .child(html!("div", {
            .class(&*chrome::FEED_FOOTER)
            .child(html!("div", {
                .class(&*chrome::NOTE)
                .text("Preview only. Wallet and backend flows are intentionally not wired on this page yet.")
            }))
        }))
    })
}

fn signal_strip() -> Dom {
    html!("section", {
        .class(&*chrome::SIGNAL_GRID)
        .children([
            signal_card(
                "Threshold unlocks",
                "Supplier pricing only turns on after enough buyers fill the deal together.",
            ),
            signal_card(
                "Visible commitments",
                "Escrow state lives on-chain instead of in screenshots, spreadsheets, and chat threads.",
            ),
            signal_card(
                "Commerce-first",
                "The crypto layer handles trust while discovery, product choice, and fulfillment stay readable.",
            ),
        ])
    })
}

fn signal_card(title: &'static str, body: &'static str) -> Dom {
    html!("article", {
        .class(&*chrome::SIGNAL_CARD)
        .children([
            html!("div", {
                .class(&*typography::MINI_HEADING)
                .text(title)
            }),
            html!("p", {
                .class(&*typography::BODY_MUTED)
                .text(body)
            }),
        ])
    })
}

fn story_section() -> Dom {
    html!("section", {
        .class(&*chrome::STORY_GRID)
        .children([
            how_it_works_panel(),
            why_groupshop_panel(),
        ])
    })
}

fn how_it_works_panel() -> Dom {
    html!("article", {
        .class(&*chrome::PANEL)
        .attr("id", "how-it-works")
        .children([
            section_title("How It Works"),
            html!("p", {
                .class(&*typography::BODY_MUTED)
                .text("The point is simple: use on-chain commitments to coordinate demand, then keep the shopping flow legible.")
            }),
            html!("div", {
                .class(&*chrome::STEP_GRID)
                .children([
                    step_card("01", "Join a live deal", "See the product, the threshold, and the unlocked price before committing."),
                    step_card("02", "Commit to escrow", "Funds move into a visible Solana-native flow instead of a group admin wallet."),
                    step_card("03", "Clear the threshold", "Once the deal fills, Groupshop handles the off-chain purchase and fulfillment path."),
                ])
            }),
        ])
    })
}

fn why_groupshop_panel() -> Dom {
    html!("article", {
        .class(&*chrome::PANEL)
        .children([
            section_title("Why Groupshop"),
            html!("p", {
                .class(&*typography::BODY_MUTED)
                .text("The crypto layer should make trust better, not make shopping harder. That is the design constraint.")
            }),
            html!("div", {
                .class(&*chrome::INFO_LIST)
                .children([
                    info_item("Real products", "Built for physical goods, shipping, supplier thresholds, and real fulfillment."),
                    info_item("Readable state", "Deal progress, savings, and escrow visibility stay obvious at a glance."),
                    info_item("Solana-native trust", "Buyers share a transparent commitment rail instead of trusting whoever runs the chat."),
                ])
            }),
        ])
    })
}

fn section_title(text: &'static str) -> Dom {
    html!("div", {
        .class(&*typography::SECTION_TITLE)
        .class(&*chrome::SECTION_MARK)
        .text(text)
    })
}

fn step_card(number: &'static str, title: &'static str, body: &'static str) -> Dom {
    html!("div", {
        .class(&*chrome::STEP_CARD)
        .children([
            html!("div", {
                .class(&*typography::MICRO_LABEL)
                .text(number)
            }),
            html!("div", {
                .class(&*typography::MINI_HEADING)
                .text(title)
            }),
            html!("p", {
                .class(&*typography::BODY_MUTED)
                .text(body)
            }),
        ])
    })
}

fn info_item(title: &'static str, body: &'static str) -> Dom {
    html!("div", {
        .class(&*chrome::INFO_ITEM)
        .children([
            html!("div", {
                .class(&*typography::MINI_HEADING)
                .text(title)
            }),
            html!("p", {
                .class(&*typography::BODY_MUTED)
                .text(body)
            }),
        ])
    })
}

fn deal_card(
    badge: &'static str,
    visual_class: &str,
    visual_word: &'static str,
    callout: &'static str,
    title: &'static str,
    price_now: &'static str,
    price_then: &'static str,
    progress: &'static str,
    progress_width: &'static str,
    meta: &'static str,
) -> Dom {
    html!("article", {
        .class(&*chrome::DEAL_CARD)
        .child(html!("div", {
            .class(&*chrome::DEAL_VISUAL)
            .class(visual_class)
            .child(html!("div", {
                .class(&*chrome::DEAL_BADGE)
                .text(badge)
            }))
            .child(html!("div", {
                .class(&*chrome::DEAL_CALLOUT)
                .child(html!("div", {
                    .class(&*typography::VISUAL_CALLOUT)
                    .text(callout)
                }))
            }))
            .child(html!("div", {
                .class(&*typography::VISUAL_WORD)
                .text(visual_word)
            }))
        }))
        .child(html!("div", {
            .class(&*chrome::DEAL_BODY)
            .child(html!("div", {
                .class(&*typography::CARD_TITLE)
                .text(title)
            }))
            .child(html!("div", {
                .class(&*chrome::PRICE_ROW)
                .children([
                    html!("div", {
                        .class(&*typography::PRICE_NOW)
                        .text(price_now)
                    }),
                    html!("div", {
                        .class(&*typography::PRICE_THEN)
                        .text(price_then)
                    }),
                ])
            }))
            .child(html!("div", {
                .class(&*chrome::PROGRESS_HEAD)
                .children([
                    html!("span", {
                        .class(&*typography::MICRO_LABEL)
                        .text("Group Progress")
                    }),
                    html!("span", {
                        .class(&*typography::MICRO_LABEL)
                        .text(progress)
                    }),
                ])
            }))
            .child(html!("div", {
                .class(&*chrome::PROGRESS_BAR)
                .child(html!("div", {
                    .class(&*chrome::PROGRESS_FILL)
                    .style("width", progress_width)
                }))
            }))
            .child(html!("button", {
                .class(&*chrome::BUTTON)
                .class(&*chrome::BUTTON_DARK)
                .attr("type", "button")
                .text("Preview Deal")
            }))
            .child(html!("div", {
                .class(&*chrome::DEAL_META)
                .text(meta)
            }))
        }))
    })
}

fn feed_item(who: &'static str, what: &'static str, when: &'static str) -> Dom {
    html!("div", {
        .class(&*chrome::FEED_ITEM)
        .children([
            html!("strong", {
                .class(&*typography::FEED_NAME)
                .text(who)
            }),
            html!("p", {
                .class(&*typography::FEED_TEXT)
                .text(what)
            }),
            html!("p", {
                .class(&*typography::FEED_TEXT)
                .text(when)
            }),
        ])
    })
}

fn site_footer() -> Dom {
    html!("footer", {
        .class(&*chrome::FOOTER)
        .child(html!("p", {
            .class(&*typography::FOOTER_TEXT)
            .children([
                html!("span", {
                    .text("Real-world group buying, secured on-chain. ")
                }),
                html!("a", {
                    .class(&*chrome::FOOTER_LINK)
                    .attr("href", "/privacy-policy")
                    .text("Privacy Policy")
                }),
                html!("span", {
                    .text(" \u{00b7} ")
                }),
                html!("a", {
                    .class(&*chrome::FOOTER_LINK)
                    .attr("href", "/terms-of-service")
                    .text("Terms of Service")
                }),
            ])
        }))
    })
}
