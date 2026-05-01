use crate::{
    id::ProductId,
    route::{ApiRoute, ApiRouteRequestResponse, ApiRouteResponse},
};
use http::Method;

/// Returns every order the signed-in user has placed, plus the live
/// threshold progress for each. The frontend splits the response into
/// "current" and "history" buckets based on `pool_status` — see
/// `OrderPoolStatus` for the mapping.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountOrdersRoute;

impl ApiRouteResponse for AccountOrdersRoute {
    const ROUTE: ApiRoute = ApiRoute::Account(crate::route::ApiAccountRoute::Orders);
    type Res = AccountOrdersResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountOrdersResponse {
    pub orders: Vec<AccountOrderSummary>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountOrderSummary {
    pub product_id: ProductId,
    pub product_name: String,
    pub product_image_url: String,
    pub price_cents: u32,
    pub minimum_order_quantity: u32,
    /// Which group-buy batch this order belongs to. The same buyer can
    /// have multiple historical orders for the same product, one per
    /// batch they joined.
    pub batch_id: u32,
    /// Per-user committed amount (USDC base units, 6 decimals).
    pub product_amount_base_units: u64,
    pub shipping_amount_base_units: u64,
    pub last_tx_signature: String,
    pub first_deposited_at: String,
    pub last_deposited_at: String,
    /// Number of units this buyer committed in the batch. Surfaced so
    /// the orders page can show "you committed 3 units" without
    /// reverse-engineering it from `product_amount_base_units`.
    pub quantity: u64,
    /// `true` after a successful self-refund. Refunded orders show up in
    /// History with a "Refunded" label and don't count toward the
    /// progress bar.
    pub refunded: bool,
    /// Sum of non-refunded `quantity` across all buyers in this batch.
    /// Compare to `minimum_order_quantity` to render the threshold
    /// progress bar.
    pub committed_units: u64,
    /// Internal pipeline status, off-chain. The on-chain Pool only goes
    /// Open → Locked → Released/Refunding; the pipeline is a richer
    /// fulfillment funnel we manage in D1.
    pub pipeline_status: BatchPipelineStatus,
}

/// Reflects our internal fulfillment pipeline. The on-chain pool drives
/// `Open ↔ Locked` (auto-locked when threshold hits); the rest are
/// admin-driven steps tracked in D1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BatchPipelineStatus {
    /// Pool is still recruiting. Withdraw allowed.
    Open,
    /// Threshold met; deposits closed; awaiting fulfillment kickoff.
    Locked,
    /// Bulk shipment to the distributor in progress.
    ShippingToDistributor,
    /// Distributor is shipping each unit to individual buyers.
    ShippingIndividually,
    /// Funds released to the supplier and order considered complete.
    Released,
    /// Pool flipped to refund mode (admin action). All buyers can
    /// claim a refund.
    Refunding,
}

/// Per-product check for "does the signed-in user have a non-refunded
/// participation in this product's currently-active batch?". The
/// product detail page hits this when signed in to render the
/// "You've committed N units" badge + "View in Cart" CTA.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountOrderStatusRoute;

impl ApiRouteRequestResponse for AccountOrderStatusRoute {
    const ROUTE: ApiRoute = ApiRoute::Account(crate::route::ApiAccountRoute::OrderStatus);
    type Req = AccountOrderStatusRequest;
    type Res = AccountOrderStatusResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountOrderStatusRequest {
    pub product_id: ProductId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountOrderStatusResponse {
    /// `Some` when the buyer has a confirmed, non-refunded participation
    /// in the product's active batch. The frontend uses this to render
    /// the cart-pending banner.
    pub active_order: Option<AccountOrderActive>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountOrderActive {
    pub batch_id: u32,
    pub quantity: u64,
    pub product_amount_base_units: u64,
    pub pipeline_status: BatchPipelineStatus,
}

impl BatchPipelineStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Locked => "locked",
            Self::ShippingToDistributor => "shipping_to_distributor",
            Self::ShippingIndividually => "shipping_individually",
            Self::Released => "released",
            Self::Refunding => "refunding",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "locked" => Self::Locked,
            "shipping_to_distributor" => Self::ShippingToDistributor,
            "shipping_individually" => Self::ShippingIndividually,
            "released" => Self::Released,
            "refunding" => Self::Refunding,
            _ => Self::Open,
        }
    }
}
