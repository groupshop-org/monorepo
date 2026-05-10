use dominator::{clone, events, html, Dom};
use futures_signals::signal::{Mutable, SignalExt};
use gloo_net::http::Request;
use groupshop_backend_shared::prelude::{
    AccountEscrowDepositBuildRequest, AccountEscrowDepositBuildResponse,
    AccountEscrowDepositConfirmRequest, AccountEscrowDepositIntentRequest,
    AccountEscrowDepositSubmitRequest, ProductSummary,
};
use groupshop_frontend_shared::{
    error::{FrontendError, FrontendResult},
    theme::{chrome, typography},
};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

use crate::{api::ApiCtx, config, route::Route};

#[derive(Clone, serde::Deserialize)]
struct SolanaManifest {
    networks: std::collections::BTreeMap<String, SolanaManifestNetwork>,
}

#[derive(Clone, PartialEq, Eq, serde::Deserialize)]
struct SolanaManifestNetwork {
    network: String,
    rpc_url: String,
    market_program_id: String,
    usdc_mint: String,
}

#[derive(Clone, PartialEq, Eq)]
enum ManifestState {
    Loading,
    Ready(SolanaManifestNetwork),
    Error(String),
}

enum DepositAction {
    Connected,
    Submitted(String),
}

#[derive(Clone)]
enum SuccessMessage {
    Text(String),
    Transaction { signature: String, url: String },
}

#[wasm_bindgen(module = "/src/wallet/phantom_bridge.generated.js")]
extern "C" {
    #[wasm_bindgen(js_name = phantomIsAvailable)]
    fn phantom_is_available() -> bool;

    #[wasm_bindgen(catch, js_name = phantomConnect)]
    async fn phantom_connect_js() -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, js_name = phantomSignMessage)]
    async fn phantom_sign_message_js(message: String) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, js_name = phantomSignEscrowDepositTransaction)]
    async fn phantom_sign_escrow_deposit_transaction_js(
        payload_json: String,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, js_name = phantomSignEscrowRefundTransaction)]
    async fn phantom_sign_escrow_refund_transaction_js(
        payload_json: String,
    ) -> Result<JsValue, JsValue>;
}

pub fn render_deposit_panel(product: ProductSummary, committed_units: Mutable<u64>) -> Dom {
    let manifest = Mutable::new(ManifestState::Loading);
    let wallet_address = Mutable::new(None::<String>);
    let working = Mutable::new(false);
    let error = Mutable::new(None::<String>);
    let success = Mutable::new(None::<SuccessMessage>);
    // Buyer-chosen quantity. Defaults to 1 — the smallest commitment
    // that still helps the deal trigger. Users can crank it up to be
    // the entire batch's worth on their own if they want.
    let quantity: Mutable<u32> = Mutable::new(1);

    spawn_local(clone!(manifest => async move {
        manifest.set(match load_manifest_network().await {
            Ok(network) => ManifestState::Ready(network),
            Err(err) => ManifestState::Error(err.to_string()),
        });
    }));

    html!("div", {
        .class(&*chrome::CARD)
        .style("margin-top", "1.5rem")
        .children([
            html!("h3", {
                .class(&*typography::MINI_HEADING)
                .style("margin-bottom", "1rem")
                .text("Join this group order")
            }),
            html!("div", {
                .style("display", "flex")
                .style("flex-direction", "column")
                .style("gap", "0.75rem")
                .children([
                    summary_row("Unit price", &format!("${:.2}", product.price_cents as f64 / 100.0)),
                    summary_row("Group threshold", &format!("{} units", product.minimum_order_quantity)),
                    quantity_picker_row(quantity.clone()),
                ])
            }),
            // Live total based on the quantity input. `text_signal` keeps
            // it in lockstep with the input — no manual recompute.
            html!("div", {
                .style("display", "flex")
                .style("justify-content", "space-between")
                .style("margin-top", "1rem")
                .style("padding-top", "1rem")
                .style("border-top", &format!("1px solid {}", groupshop_frontend_shared::theme::color::LINE))
                .children([
                    html!("span", {
                        .style("font-weight", "700")
                        .text("Total")
                    }),
                    html!("span", {
                        .style("font-weight", "700")
                        .style("font-size", "1.15rem")
                        .text_signal(quantity.signal().map(clone!(product => move |q| {
                            format!(
                                "${:.2}",
                                product.price_cents as f64 / 100.0 * q.max(1) as f64
                            )
                        })))
                    }),
                ])
            }),
            html!("p", {
                .class(&*typography::BODY_MUTED)
                .style("margin-top", "0.75rem")
                .style("font-size", "0.8rem")
                .style("text-align", "center")
                .text_signal(manifest.signal_cloned().map(|state| match state {
                    ManifestState::Loading => "Loading Solana network config…".to_string(),
                    ManifestState::Ready(cfg) => format!(
                        "Deposits target {} via Phantom. Payment is held in escrow on Solana until the group order threshold is met.",
                        cfg.network
                    ),
                    ManifestState::Error(err) => format!("Solana config error: {err}"),
                }))
            }),
            html!("p", {
                .class(&*typography::BODY_MUTED)
                .style("margin-top", "0.35rem")
                .style("font-size", "0.78rem")
                .style("text-align", "center")
                .text_signal(wallet_address.signal_cloned().map(|wallet| match wallet {
                    Some(wallet) => format!("Connected wallet: {}", shorten_wallet(&wallet)),
                    None if phantom_is_available() => "Phantom not connected yet.".to_string(),
                    None => "Phantom extension not detected.".to_string(),
                }))
            }),
            html!("p", {
                .style("margin-top", "0.5rem")
                .style("font-size", "0.85rem")
                .style("color", "#8e1230")
                .style_signal("display", error.signal_cloned().map(|value| {
                    if value.is_some() { "block".to_string() } else { "none".to_string() }
                }))
                .text_signal(error.signal_cloned().map(|value| value.unwrap_or_default()))
            }),
            html!("p", {
                .style("margin-top", "0.5rem")
                .style("font-size", "0.85rem")
                .style("color", "#166534")
                .style_signal("display", success.signal_cloned().map(|value| {
                    if value.is_some() { "block".to_string() } else { "none".to_string() }
                }))
                .child_signal(success.signal_cloned().map(render_success_message))
            }),
            html!("button", {
                .class(&*chrome::BUTTON)
                .class(&*chrome::BUTTON_GRADIENT)
                .style("width", "100%")
                .style("margin-top", "1.25rem")
                .style("min-height", "3rem")
                .style("font-size", "1rem")
                .style_signal("opacity", working.signal().map(|busy| {
                    if busy { "0.72".to_string() } else { "1".to_string() }
                }))
                .style_signal("cursor", working.signal().map(|busy| {
                    if busy { "progress".to_string() } else { "pointer".to_string() }
                }))
                .prop_signal("disabled", working.signal())
                .text_signal(button_label(manifest.clone(), wallet_address.clone(), working.clone()))
                .event(clone!(manifest, wallet_address, working, error, success, product => move |_: events::Click| {
                    let profile = ApiCtx::get().profile.get_cloned();
                    if profile.is_none() {
                        Route::Signin.go_to_url();
                        return;
                    }

                    if !phantom_is_available() {
                        let _ = web_sys::window()
                            .unwrap()
                            .open_with_url("https://phantom.app/");
                        return;
                    }

                    if working.get() {
                        return;
                    }

                    error.set(None);
                    success.set(None);

                    let manifest_state = manifest.get_cloned();
                    let product = product.clone();

                    spawn_local(clone!(wallet_address, working, error, success, committed_units, quantity => async move {
                        working.set(true);

                        let run = async {
                            let manifest_cfg = match manifest_state {
                                ManifestState::Ready(cfg) => cfg,
                                ManifestState::Loading => {
                                    return Err(FrontendError::Other(
                                        "Solana config is still loading".to_string(),
                                    ));
                                }
                                ManifestState::Error(err) => {
                                    return Err(FrontendError::Other(err));
                                }
                            };

                            let wallet = match wallet_address.get_cloned() {
                                Some(wallet) => wallet,
                                None => {
                                    let wallet = phantom_connect().await?;
                                    wallet_address.set(Some(wallet.clone()));
                                    success.set(Some(SuccessMessage::Text(format!(
                                        "Connected wallet: {}. Click again to deposit.",
                                        shorten_wallet(&wallet)
                                    ))));
                                    return Ok::<DepositAction, FrontendError>(DepositAction::Connected);
                                }
                            };

                            let qty = quantity.get().max(1);
                            let intent = ApiCtx::get()
                                .client
                                .account_escrow_deposit_intent(&AccountEscrowDepositIntentRequest {
                                    product_id: product.id.clone(),
                                    wallet_address: wallet.clone(),
                                    quantity: qty,
                                })
                                .await?;

                            validate_intent_against_manifest(&intent, &manifest_cfg)?;

                            let proof_token = intent.proof_token.clone();
                            let wallet_signature = phantom_sign_message(intent.challenge_message).await?;
                            let build = ApiCtx::get()
                                .client
                                .account_escrow_deposit_build(&AccountEscrowDepositBuildRequest {
                                    proof_token: proof_token.clone(),
                                    wallet_signature_base64: wallet_signature,
                                })
                                .await?;

                            validate_build_against_manifest(&build, &manifest_cfg)?;

                            let signed_transaction_base64 = phantom_sign_deposit_transaction(build.clone()).await?;
                            let submit = ApiCtx::get()
                                .client
                                .account_escrow_deposit_submit(&AccountEscrowDepositSubmitRequest {
                                    proof_token,
                                    recent_blockhash: build.recent_blockhash,
                                    signed_transaction_base64,
                                })
                                .await?;
                            let tx_signature = submit.tx_signature;

                            // Tell the backend the deposit landed so it can
                            // mirror the on-chain Participation into D1.
                            // This is the source-of-truth write that powers
                            // "My Orders" and the threshold counters. We
                            // surface failures so a flaky confirm shows up
                            // in the UI rather than silently leaving the
                            // user's order absent from D1 reports.
                            let confirmed = ApiCtx::get()
                                .client
                                .account_escrow_deposit_confirm(
                                    &AccountEscrowDepositConfirmRequest {
                                        product_id: product.id.clone(),
                                        tx_signature: tx_signature.clone(),
                                    },
                                )
                                .await?;
                            // Bump the threshold count locally so the bar
                            // reflects this deposit immediately, without
                            // waiting for the next polling tick.
                            committed_units.set(confirmed.committed_units);

                            Ok::<DepositAction, FrontendError>(DepositAction::Submitted(
                                tx_signature,
                            ))
                        }
                        .await;

                        match run {
                            Ok(DepositAction::Connected) => {}
                            Ok(DepositAction::Submitted(signature)) => {
                                success.set(Some(SuccessMessage::Transaction {
                                    url: solscan_tx_url(config::solana_network(), &signature),
                                    signature,
                                }));
                            }
                            Err(err) => {
                                error.set(Some(err.to_string()));
                            }
                        }

                        working.set(false);
                    }));
                }))
            }),
        ])
    })
}

fn button_label(
    manifest: Mutable<ManifestState>,
    wallet_address: Mutable<Option<String>>,
    working: Mutable<bool>,
) -> impl futures_signals::signal::Signal<Item = String> {
    futures_signals::map_ref! {
        let manifest = manifest.signal_cloned(),
        let wallet = wallet_address.signal_cloned(),
        let working = working.signal(),
        let profile = ApiCtx::get().profile.signal_cloned() => {
            if *working {
                "Preparing deposit…".to_string()
            } else if profile.is_none() {
                "Sign in to deposit".to_string()
            } else if !phantom_is_available() {
                "Install Phantom".to_string()
            } else if matches!(&*manifest, ManifestState::Loading) {
                "Loading network…".to_string()
            } else if matches!(&*manifest, ManifestState::Error(_)) {
                "Solana config unavailable".to_string()
            } else if wallet.is_none() {
                "Connect Phantom".to_string()
            } else {
                "Deposit to escrow".to_string()
            }
        }
    }
}

async fn load_manifest_network() -> FrontendResult<SolanaManifestNetwork> {
    let manifest: SolanaManifest = Request::get(config::solana_deployments_url())
        .send()
        .await?
        .json()
        .await?;

    manifest
        .networks
        .get(config::solana_network())
        .cloned()
        .ok_or_else(|| {
            FrontendError::Other(format!(
                "network `{}` missing from {}",
                config::solana_network(),
                config::solana_deployments_url()
            ))
        })
}

async fn phantom_connect() -> FrontendResult<String> {
    let value = phantom_connect_js().await.map_err(js_error)?;
    value.as_string().ok_or_else(|| {
        FrontendError::Other("Phantom connect returned a non-string wallet".to_string())
    })
}

async fn phantom_sign_message(message: String) -> FrontendResult<String> {
    let value = phantom_sign_message_js(message).await.map_err(js_error)?;
    value.as_string().ok_or_else(|| {
        FrontendError::Other("Phantom signMessage returned a non-string signature".to_string())
    })
}

async fn phantom_sign_deposit_transaction(
    build: AccountEscrowDepositBuildResponse,
) -> FrontendResult<String> {
    let payload = serde_json::to_string(&build)
        .map_err(|err| FrontendError::Other(format!("failed to encode wallet payload: {err}")))?;
    let value = phantom_sign_escrow_deposit_transaction_js(payload)
        .await
        .map_err(js_error)?;
    value.as_string().ok_or_else(|| {
        FrontendError::Other(
            "Phantom signTransaction returned a non-string transaction".to_string(),
        )
    })
}

/// Outcome of `run_self_refund`. Both variants tell the caller "the
/// participation is now refunded — reload the orders list."
pub enum RefundOutcome {
    /// We submitted a fresh on-chain refund this round. Carries the tx
    /// signature so future call sites can render a Solscan link.
    #[allow(dead_code)]
    Submitted(String),
    /// Backend reported the participation was already refunded on-chain
    /// (a previous attempt landed). The build/submit endpoints self-heal
    /// the D1 mirror in that case, so a reload now shows the row in
    /// History.
    AlreadyRefunded,
}

/// Run the buyer-initiated refund end-to-end. Used by the My Orders
/// "Withdraw" button. Steps:
///   1. Backend builds the SelfRefund transaction with empty signer slots.
///   2. Phantom signs as buyer.
///   3. Backend adds the authority signature and submits.
///   4. Backend confirms by reading on-chain `Participation.refunded`
///      and mirrors the flag into D1.
/// If a prior attempt already landed the on-chain refund but the D1
/// mirror is stale, the build/submit endpoints self-heal D1 and surface
/// `AlreadyRefunded` here so the caller can reload without showing an
/// error.
pub async fn run_self_refund(
    product_id: groupshop_backend_shared::prelude::ProductId,
    batch_id: u32,
) -> FrontendResult<RefundOutcome> {
    if !phantom_is_available() {
        return Err(FrontendError::Other(
            "Phantom wallet not available — install it from phantom.app to refund".to_string(),
        ));
    }
    let wallet = phantom_connect().await?;

    let build = match ApiCtx::get()
        .client
        .account_escrow_refund_build(
            &groupshop_backend_shared::prelude::AccountEscrowRefundBuildRequest {
                product_id: product_id.clone(),
                wallet_address: wallet.clone(),
                batch_id,
            },
        )
        .await
    {
        Ok(build) => build,
        Err(err) if is_already_refunded(&err) => return Ok(RefundOutcome::AlreadyRefunded),
        Err(err) => return Err(err),
    };

    let payload = serde_json::to_string(&build)
        .map_err(|err| FrontendError::Other(format!("failed to encode refund payload: {err}")))?;
    let signed_transaction_base64 = phantom_sign_escrow_refund_transaction_js(payload)
        .await
        .map_err(js_error)?
        .as_string()
        .ok_or_else(|| {
            FrontendError::Other("Phantom refund returned a non-string transaction".to_string())
        })?;
    let submit = match ApiCtx::get()
        .client
        .account_escrow_refund_submit(
            &groupshop_backend_shared::prelude::AccountEscrowRefundSubmitRequest {
                product_id: product_id.clone(),
                wallet_address: wallet,
                batch_id,
                recent_blockhash: build.recent_blockhash,
                signed_transaction_base64,
            },
        )
        .await
    {
        Ok(submit) => submit,
        Err(err) if is_already_refunded(&err) => return Ok(RefundOutcome::AlreadyRefunded),
        Err(err) => return Err(err),
    };
    let signature = submit.tx_signature;

    ApiCtx::get()
        .client
        .account_escrow_refund_confirm(
            &groupshop_backend_shared::prelude::AccountEscrowRefundConfirmRequest {
                product_id,
                batch_id,
                tx_signature: signature.clone(),
            },
        )
        .await?;
    Ok(RefundOutcome::Submitted(signature))
}

fn is_already_refunded(err: &FrontendError) -> bool {
    err.to_string().contains("already been refunded")
}

fn validate_intent_against_manifest(
    intent: &groupshop_backend_shared::prelude::AccountEscrowDepositIntentResponse,
    manifest: &SolanaManifestNetwork,
) -> FrontendResult<()> {
    if intent.network != manifest.network
        || intent.rpc_url != manifest.rpc_url
        || intent.program_id != manifest.market_program_id
        || intent.usdc_mint != manifest.usdc_mint
    {
        return Err(FrontendError::Other(
            "backend Solana config does not match the deployed frontend manifest".to_string(),
        ));
    }
    Ok(())
}

fn validate_build_against_manifest(
    build: &AccountEscrowDepositBuildResponse,
    manifest: &SolanaManifestNetwork,
) -> FrontendResult<()> {
    if build.network != manifest.network
        || build.rpc_url != manifest.rpc_url
        || build.program_id != manifest.market_program_id
        || build.usdc_mint != manifest.usdc_mint
    {
        return Err(FrontendError::Other(
            "backend deposit build does not match the deployed frontend manifest".to_string(),
        ));
    }
    Ok(())
}

fn render_success_message(message: Option<SuccessMessage>) -> Option<Dom> {
    Some(match message? {
        SuccessMessage::Text(text) => html!("span", {
            .text(&text)
        }),
        SuccessMessage::Transaction { signature, url } => html!("span", {
            .text("Deposit submitted. ")
            .child(html!("a", {
                .attr("href", &url)
                .attr("target", "_blank")
                .attr("rel", "noopener noreferrer")
                .style("color", "#166534")
                .style("font-weight", "700")
                .style("text-decoration", "underline")
                .text("View transaction on Solscan")
            }))
            .child(html!("span", {
                .text(&format!(" ({})", shorten_signature(&signature)))
            }))
        }),
    })
}

fn solscan_tx_url(network: &str, signature: &str) -> String {
    let cluster = match network {
        "mainnet" | "mainnet-beta" => "",
        "devnet" => "?cluster=devnet",
        "testnet" => "?cluster=testnet",
        "local" | "localhost" | "localnet" => "?cluster=custom",
        _ => "",
    };
    format!("https://solscan.io/tx/{signature}{cluster}")
}

fn shorten_wallet(value: &str) -> String {
    if value.len() <= 10 {
        value.to_string()
    } else {
        format!("{}…{}", &value[..4], &value[value.len() - 4..])
    }
}

fn shorten_signature(value: &str) -> String {
    if value.len() <= 16 {
        value.to_string()
    } else {
        format!("{}…{}", &value[..8], &value[value.len() - 8..])
    }
}

fn js_error(err: JsValue) -> FrontendError {
    if let Some(text) = err.as_string() {
        return FrontendError::Other(text);
    }

    if err.is_object() {
        let object = js_sys::Object::from(err.clone());
        let message = js_sys::Reflect::get(&object, &JsValue::from_str("message"))
            .ok()
            .and_then(|value| value.as_string());
        let code = js_sys::Reflect::get(&object, &JsValue::from_str("code"))
            .ok()
            .and_then(|value| {
                value
                    .as_f64()
                    .map(|number| number.to_string())
                    .or_else(|| value.as_string())
            });

        if let Some(message) = message {
            return FrontendError::Other(match code {
                Some(code) => format!("Phantom error {code}: {message}"),
                None => message,
            });
        }
    }

    FrontendError::Other(
        js_sys::JSON::stringify(&err)
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "unknown Phantom error".to_string()),
    )
}

/// Number-input row for the buyer-chosen quantity. Min 1, no upper
/// cap from the UI (the on-chain program rejects insufficient USDC
/// balance, so over-commits fail loudly at sign time). Bound to
/// the deposit panel's `quantity` Mutable so the live total updates
/// as the user types.
fn quantity_picker_row(quantity: Mutable<u32>) -> Dom {
    html!("div", {
        .style("display", "flex")
        .style("justify-content", "space-between")
        .style("align-items", "center")
        .children([
            html!("span", {
                .class(&*typography::BODY_MUTED)
                .text("Your quantity")
            }),
            html!("input" => web_sys::HtmlInputElement, {
                .attr("type", "number")
                .attr("min", "1")
                .attr("step", "1")
                .attr("inputmode", "numeric")
                .style("width", "5rem")
                .style("text-align", "right")
                .style("padding", "0.35rem 0.5rem")
                .style("border", "1px solid #d1d5db")
                .style("border-radius", "0.45rem")
                .style("font-size", "0.95rem")
                .style("font-weight", "600")
                .prop_signal("value", quantity.signal().map(|q| q.to_string()))
                .event(clone!(quantity => move |evt: events::Input| {
                    if let Some(input) = evt
                        .target()
                        .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                    {
                        // Parse defensively: empty + non-numeric both
                        // collapse to 1 so the deposit math never sees
                        // a zero quantity (which the backend rejects).
                        let parsed = input.value().trim().parse::<u32>().unwrap_or(0).max(1);
                        quantity.set(parsed);
                    }
                }))
            }),
        ])
    })
}

fn summary_row(label: &str, value: &str) -> Dom {
    html!("div", {
        .style("display", "flex")
        .style("justify-content", "space-between")
        .children([
            html!("span", {
                .class(&*typography::BODY_MUTED)
                .text(label)
            }),
            html!("span", {
                .style("font-weight", "500")
                .text(value)
            }),
        ])
    })
}
