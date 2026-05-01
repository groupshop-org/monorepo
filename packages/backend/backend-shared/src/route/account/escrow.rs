use crate::{
    id::{AuthTokenSignature, ProductId, UserId},
    route::{ApiRoute, ApiRouteRequestResponse},
};
use http::Method;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountEscrowDepositIntentRoute;

impl ApiRouteRequestResponse for AccountEscrowDepositIntentRoute {
    const ROUTE: ApiRoute = ApiRoute::Account(crate::route::ApiAccountRoute::EscrowDepositIntent);
    type Req = AccountEscrowDepositIntentRequest;
    type Res = AccountEscrowDepositIntentResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountEscrowDepositBuildRoute;

impl ApiRouteRequestResponse for AccountEscrowDepositBuildRoute {
    const ROUTE: ApiRoute = ApiRoute::Account(crate::route::ApiAccountRoute::EscrowDepositBuild);
    type Req = AccountEscrowDepositBuildRequest;
    type Res = AccountEscrowDepositBuildResponse;
    const METHOD: Method = Method::POST;
}

/// Called by the client after Phantom successfully submits the deposit
/// transaction. The server independently verifies the deposit by reading the
/// on-chain Participation PDA (the source of truth) and then mirrors the
/// snapshot into D1 so subsequent "my orders" / threshold queries don't have
/// to hit Solana RPC. Idempotent — calling it twice with the same signature
/// just refreshes the D1 row.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountEscrowDepositConfirmRoute;

impl ApiRouteRequestResponse for AccountEscrowDepositConfirmRoute {
    const ROUTE: ApiRoute = ApiRoute::Account(crate::route::ApiAccountRoute::EscrowDepositConfirm);
    type Req = AccountEscrowDepositConfirmRequest;
    type Res = AccountEscrowDepositConfirmResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountEscrowDepositConfirmRequest {
    pub product_id: ProductId,
    pub tx_signature: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountEscrowDepositConfirmResponse {
    pub product_id: ProductId,
    pub product_amount_base_units: u64,
    pub shipping_amount_base_units: u64,
    pub committed_units: u64,
    pub minimum_order_quantity: u32,
    pub batch_id: u32,
}

// -- Self-refund (withdraw while pool is still Open) ----------------------

/// First leg of the self-refund: returns the fully-built transaction
/// bytes (authority slot pre-signed, buyer slot empty) for Phantom to sign
/// and submit. Same shape as deposit-build so the client can reuse the
/// existing Transaction.from() round-trip.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountEscrowRefundBuildRoute;

impl ApiRouteRequestResponse for AccountEscrowRefundBuildRoute {
    const ROUTE: ApiRoute = ApiRoute::Account(crate::route::ApiAccountRoute::EscrowRefundBuild);
    type Req = AccountEscrowRefundBuildRequest;
    type Res = AccountEscrowRefundBuildResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountEscrowRefundBuildRequest {
    pub product_id: ProductId,
    /// Buyer's wallet — the one that made the original deposit. The
    /// server cross-checks against the on-chain Participation to refuse
    /// refund requests for the wrong wallet.
    pub wallet_address: String,
    pub batch_id: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountEscrowRefundBuildResponse {
    pub network: String,
    pub rpc_url: String,
    pub program_id: String,
    pub usdc_mint: String,
    pub authority_address: String,
    pub wallet_address: String,
    pub batch_id: u32,
    pub buyer_associated_token_account: String,
    pub product_amount_base_units: u64,
    pub shipping_amount_base_units: u64,
    pub total_amount_base_units: u64,
    /// Full Solana transaction bytes (base64) — buyer slot zero-filled,
    /// authority slot pre-signed. Client uses `Transaction.from()` to
    /// preserve the message bytes through Phantom signing.
    pub transaction_base64: String,
}

/// Called after Phantom returns success. Server verifies the on-chain
/// Participation has flipped to `refunded == 1`, then mirrors that into
/// D1 so the orders view updates.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountEscrowRefundConfirmRoute;

impl ApiRouteRequestResponse for AccountEscrowRefundConfirmRoute {
    const ROUTE: ApiRoute = ApiRoute::Account(crate::route::ApiAccountRoute::EscrowRefundConfirm);
    type Req = AccountEscrowRefundConfirmRequest;
    type Res = AccountEscrowRefundConfirmResponse;
    const METHOD: Method = Method::POST;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountEscrowRefundConfirmRequest {
    pub product_id: ProductId,
    pub batch_id: u32,
    pub tx_signature: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountEscrowRefundConfirmResponse {
    pub product_id: ProductId,
    pub batch_id: u32,
    pub committed_units: u64,
    pub minimum_order_quantity: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountEscrowDepositIntentRequest {
    pub product_id: ProductId,
    pub wallet_address: String,
    /// How many product units the buyer wants to commit to in this
    /// deposit. Must be >= 1. The product's `minimum_order_quantity`
    /// is the *batch-wide* threshold, not a per-buyer minimum, so an
    /// individual buyer can buy any positive number.
    pub quantity: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountEscrowDepositBuildRequest {
    pub proof_token: String,
    pub wallet_signature_base64: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountEscrowDepositIntentResponse {
    pub network: String,
    pub rpc_url: String,
    pub program_id: String,
    pub usdc_mint: String,
    pub authority_address: String,
    pub wallet_address: String,
    pub user_id: UserId,
    pub product_amount_base_units: u64,
    pub shipping_amount_base_units: u64,
    pub total_amount_base_units: u64,
    pub token_decimals: u8,
    pub product_name: String,
    pub challenge_message: String,
    pub challenge_expires_at_ms: u64,
    pub proof_token: String,
    pub batch_id: u32,
    pub quantity: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountEscrowDepositBuildResponse {
    pub network: String,
    pub rpc_url: String,
    pub program_id: String,
    pub usdc_mint: String,
    pub authority_address: String,
    pub wallet_address: String,
    pub recent_blockhash: String,
    pub product_amount_base_units: u64,
    pub shipping_amount_base_units: u64,
    pub total_amount_base_units: u64,
    pub product_name: String,
    pub user_id: UserId,
    pub buyer_associated_token_account: String,
    pub pool_address: String,
    pub vault_address: String,
    pub participation_address: String,
    pub participation_bump: u8,
    pub authority_signature_base64: String,
    /// The full Solana transaction bytes (base64 encoded) with the authority
    /// signature already populated and a zero-filled placeholder for the
    /// buyer signature. The client deserializes this directly via
    /// `Transaction.from(bytes)` so the message bytes the authority signed
    /// are preserved verbatim through Phantom signing and on-chain submission.
    pub transaction_base64: String,
    pub batch_id: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EscrowWalletProofToken {
    pub claims: EscrowWalletProofClaims,
    pub signature: AuthTokenSignature,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EscrowWalletProofClaims {
    pub uid: UserId,
    pub product_id: ProductId,
    pub product_name: String,
    pub wallet_address: String,
    pub network: String,
    pub rpc_url: String,
    pub program_id: String,
    pub usdc_mint: String,
    pub authority_address: String,
    pub product_amount_base_units: u64,
    pub shipping_amount_base_units: u64,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub nonce: String,
    /// Batch this proof is bound to. Resolved server-side at intent
    /// time and re-validated at build time so a stale proof can't be
    /// replayed against a newer batch.
    pub batch_id: u32,
    /// Number of product units this deposit will commit. Bound to the
    /// proof so a client can't escalate quantity at build time.
    pub quantity: u32,
}
