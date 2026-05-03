use std::str::FromStr;

use base64::Engine;
use bs58;
use curve25519_dalek::edwards::CompressedEdwardsY;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use groupshop_backend_shared::prelude::{ApiError, ApiResult, ProductHash, ProductId, UserId};
use serde::de::DeserializeOwned;
use serde_json::json;
use sha2::{Digest, Sha256};
use worker::{Method, Request, RequestInit};

use crate::{config::SolanaConfig, context::ApiContext};

const ASSOCIATED_TOKEN_PROGRAM_ID_STR: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
const TOKEN_PROGRAM_ID_STR: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const PROGRAM_DERIVED_ADDRESS_MARKER: &[u8] = b"ProgramDerivedAddress";
const TAG_INITIALIZE_POOL: u8 = 0;
const TAG_DEPOSIT: u8 = 1;
const TAG_SELF_REFUND: u8 = 6;
const POOL_SEED: &[u8] = b"pool";
const VAULT_SEED: &[u8] = b"vault";
const PARTICIPATION_SEED: &[u8] = b"part";

/// Empty 64-byte placeholder for a signature slot that has not been filled
/// yet. Phantom recognizes the all-zero buffer as the wallet's pending
/// signature when deserializing a transaction.
pub const SIGNATURE_PLACEHOLDER: [u8; 64] = [0u8; 64];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Pubkey([u8; 32]);

impl Pubkey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_base58(self) -> String {
        bs58::encode(self.0).into_string()
    }

    pub fn from_base58(value: &str) -> ApiResult<Self> {
        let bytes = bs58::decode(value)
            .into_vec()
            .map_err(|err| ApiError::Validation(format!("invalid base58 pubkey: {err}")))?;
        let arr: [u8; 32] = bytes.try_into().map_err(|bytes: Vec<u8>| {
            ApiError::Validation(format!("pubkey must be 32 bytes, got {}", bytes.len()))
        })?;
        Ok(Self(arr))
    }
}

impl std::fmt::Display for Pubkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_base58())
    }
}

impl FromStr for Pubkey {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_base58(s)
    }
}

#[derive(Clone, Debug)]
pub struct DepositRecipe {
    pub buyer: Pubkey,
    pub authority: Pubkey,
    pub pool: Pubkey,
    pub participation: Pubkey,
    pub vault: Pubkey,
    pub buyer_ata: Pubkey,
    pub usdc_mint: Pubkey,
    pub program_id: Pubkey,
    pub user_id: UserId,
    pub participation_bump: u8,
    pub product_amount: u64,
    pub shipping_amount: u64,
    pub batch_id: u32,
    /// Number of product units this deposit commits to. The on-chain
    /// program adds this to `Pool.total_quantity` and auto-locks when
    /// the running total hits `Pool.threshold`.
    pub quantity: u64,
}

#[derive(Clone, Debug)]
pub struct InitializePoolRecipe {
    pub authority: Pubkey,
    pub pool: Pubkey,
    pub vault: Pubkey,
    pub usdc_mint: Pubkey,
    pub program_id: Pubkey,
    pub product_hash: ProductHash,
    pub pool_bump: u8,
    pub vault_bump: u8,
    pub batch_id: u32,
    pub threshold: u32,
}

#[derive(Clone, Debug)]
pub struct EscrowDerivedAddresses {
    pub pool: Pubkey,
    pub vault: Pubkey,
    pub participation: Pubkey,
    pub buyer_ata: Pubkey,
    pub participation_bump: u8,
}

/// All accounts needed to build a buyer-initiated `SelfRefund`
/// transaction. The buyer signs as the principal; the authority co-signs
/// (so the backend can apply policy gates that aren't on-chain).
#[derive(Clone, Debug)]
pub struct SelfRefundRecipe {
    pub buyer: Pubkey,
    pub authority: Pubkey,
    pub pool: Pubkey,
    pub participation: Pubkey,
    pub vault: Pubkey,
    pub buyer_ata: Pubkey,
    pub program_id: Pubkey,
}

pub fn signing_key_from_config(cfg: &SolanaConfig) -> ApiResult<SigningKey> {
    let bytes = cfg.authority_signing_key;
    Ok(SigningKey::from_bytes(&bytes))
}

pub fn authority_pubkey_from_config(cfg: &SolanaConfig) -> ApiResult<Pubkey> {
    let signing = signing_key_from_config(cfg)?;
    Ok(Pubkey::from_bytes(signing.verifying_key().to_bytes()))
}

pub fn product_amount_base_units(price_cents: u32, minimum_order_quantity: u32) -> ApiResult<u64> {
    let cents = u64::from(price_cents);
    let qty = u64::from(minimum_order_quantity);
    cents
        .checked_mul(qty)
        .and_then(|value| value.checked_mul(10_000))
        .ok_or_else(|| ApiError::Validation("deposit amount overflow".to_string()))
}

pub fn verify_wallet_signature(
    wallet: &Pubkey,
    message: &[u8],
    signature_base64: &str,
) -> ApiResult<()> {
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(signature_base64)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(signature_base64))
        .map_err(|err| ApiError::Validation(format!("invalid wallet signature base64: {err}")))?;
    let signature = Signature::from_slice(&sig_bytes)
        .map_err(|err| ApiError::Validation(format!("invalid wallet signature bytes: {err}")))?;
    let verifying = VerifyingKey::from_bytes(wallet.as_ref())
        .map_err(|err| ApiError::Validation(format!("invalid wallet public key: {err}")))?;
    verifying.verify(message, &signature).map_err(|err| {
        ApiError::Validation(format!("wallet signature verification failed: {err}"))
    })?;
    Ok(())
}

pub fn decode_blockhash(value: &str) -> ApiResult<[u8; 32]> {
    Pubkey::from_base58(value).map(|value| value.to_bytes())
}

/// Derive just the on-chain Participation PDA for a `(uid, product_id, batch_id)`.
/// The batch_id seed means a buyer can join successive batches of the
/// same product without colliding with their previous participation.
pub fn derive_participation_pda(
    cfg: &SolanaConfig,
    product_id: &ProductId,
    uid: &UserId,
    batch_id: u32,
) -> ApiResult<(Pubkey, u8)> {
    let program_id = Pubkey::from_base58(&cfg.market_program_id)?;
    let product_hash = product_id.hash();
    let batch_id_bytes = batch_id.to_le_bytes();
    find_program_address(
        &[
            PARTICIPATION_SEED,
            product_hash.as_ref(),
            &batch_id_bytes,
            uid.as_ref(),
        ],
        &program_id,
    )
}

/// Derive the on-chain Pool PDA for a `(product_id, batch_id)`.
pub fn derive_pool_pda(
    cfg: &SolanaConfig,
    product_id: &ProductId,
    batch_id: u32,
) -> ApiResult<(Pubkey, u8)> {
    let program_id = Pubkey::from_base58(&cfg.market_program_id)?;
    let product_hash = product_id.hash();
    let batch_id_bytes = batch_id.to_le_bytes();
    find_program_address(
        &[POOL_SEED, product_hash.as_ref(), &batch_id_bytes],
        &program_id,
    )
}

/// Derive the on-chain Vault PDA for a `(product_id, batch_id)`.
pub fn derive_vault_pda(
    cfg: &SolanaConfig,
    product_id: &ProductId,
    batch_id: u32,
) -> ApiResult<(Pubkey, u8)> {
    let program_id = Pubkey::from_base58(&cfg.market_program_id)?;
    let product_hash = product_id.hash();
    let batch_id_bytes = batch_id.to_le_bytes();
    find_program_address(
        &[VAULT_SEED, product_hash.as_ref(), &batch_id_bytes],
        &program_id,
    )
}

pub fn derive_addresses(
    cfg: &SolanaConfig,
    product_id: &ProductId,
    wallet: &Pubkey,
    uid: &UserId,
    batch_id: u32,
) -> ApiResult<EscrowDerivedAddresses> {
    let mint = Pubkey::from_base58(&cfg.usdc_mint)?;

    let (pool, _pool_bump) = derive_pool_pda(cfg, product_id, batch_id)?;
    let (vault, _vault_bump) = derive_vault_pda(cfg, product_id, batch_id)?;
    let (participation, participation_bump) =
        derive_participation_pda(cfg, product_id, uid, batch_id)?;
    let buyer_ata = associated_token_address(wallet, &mint)?;

    Ok(EscrowDerivedAddresses {
        pool,
        vault,
        participation,
        buyer_ata,
        participation_bump,
    })
}

pub fn build_initialize_pool_recipe(
    cfg: &SolanaConfig,
    product_id: &ProductId,
    batch_id: u32,
    threshold: u32,
) -> ApiResult<InitializePoolRecipe> {
    let authority = authority_pubkey_from_config(cfg)?;
    let product_hash = product_id.hash();
    let program_id = Pubkey::from_base58(&cfg.market_program_id)?;
    let usdc_mint = Pubkey::from_base58(&cfg.usdc_mint)?;
    let (pool, pool_bump) = derive_pool_pda(cfg, product_id, batch_id)?;
    let (vault, vault_bump) = derive_vault_pda(cfg, product_id, batch_id)?;

    Ok(InitializePoolRecipe {
        authority,
        pool,
        vault,
        usdc_mint,
        program_id,
        product_hash,
        pool_bump,
        vault_bump,
        batch_id,
        threshold,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn build_deposit_recipe(
    cfg: &SolanaConfig,
    product_id: &ProductId,
    wallet: &Pubkey,
    uid: &UserId,
    product_amount: u64,
    shipping_amount: u64,
    batch_id: u32,
    quantity: u64,
) -> ApiResult<DepositRecipe> {
    let authority = authority_pubkey_from_config(cfg)?;
    let derived = derive_addresses(cfg, product_id, wallet, uid, batch_id)?;

    Ok(DepositRecipe {
        buyer: *wallet,
        authority,
        pool: derived.pool,
        participation: derived.participation,
        vault: derived.vault,
        buyer_ata: derived.buyer_ata,
        usdc_mint: Pubkey::from_base58(&cfg.usdc_mint)?,
        program_id: Pubkey::from_base58(&cfg.market_program_id)?,
        user_id: uid.clone(),
        participation_bump: derived.participation_bump,
        product_amount,
        shipping_amount,
        batch_id,
        quantity,
    })
}

pub fn build_self_refund_recipe(
    cfg: &SolanaConfig,
    product_id: &ProductId,
    wallet: &Pubkey,
    uid: &UserId,
    batch_id: u32,
) -> ApiResult<SelfRefundRecipe> {
    let authority = authority_pubkey_from_config(cfg)?;
    let derived = derive_addresses(cfg, product_id, wallet, uid, batch_id)?;

    Ok(SelfRefundRecipe {
        buyer: *wallet,
        authority,
        pool: derived.pool,
        participation: derived.participation,
        vault: derived.vault,
        buyer_ata: derived.buyer_ata,
        program_id: Pubkey::from_base58(&cfg.market_program_id)?,
    })
}

pub fn initialize_pool_message(
    recipe: &InitializePoolRecipe,
    recent_blockhash: [u8; 32],
) -> Vec<u8> {
    // product_hash[32] + pool_bump[1] + vault_bump[1] + batch_id_le[4] + threshold_le[4]
    let mut data = Vec::with_capacity(1 + 32 + 1 + 1 + 4 + 4);
    data.push(TAG_INITIALIZE_POOL);
    data.extend_from_slice(recipe.product_hash.as_ref());
    data.push(recipe.pool_bump);
    data.push(recipe.vault_bump);
    data.extend_from_slice(&recipe.batch_id.to_le_bytes());
    data.extend_from_slice(&recipe.threshold.to_le_bytes());

    let account_keys = vec![
        recipe.authority,
        recipe.vault,
        recipe.pool,
        system_program_id(),
        token_program_id(),
        recipe.usdc_mint,
        recipe.program_id,
    ];

    let mut message = Vec::new();
    message.push(1);
    message.push(0);
    message.push(4);
    encode_shortvec(account_keys.len(), &mut message);
    for key in &account_keys {
        message.extend_from_slice(key.as_ref());
    }
    message.extend_from_slice(&recent_blockhash);
    encode_shortvec(1, &mut message);
    message.push(6);
    encode_shortvec(6, &mut message);
    message.extend_from_slice(&[0, 2, 1, 5, 3, 4]);
    encode_shortvec(data.len(), &mut message);
    message.extend_from_slice(&data);
    message
}

/// Builds the wire-format message for a buyer-initiated `SelfRefund`. Same
/// signer ordering as the deposit message (buyer at index 0, authority at
/// index 1) so the same client-side flow — backend pre-attaches authority
/// signature, Phantom fills buyer slot via `Transaction.from()` round-trip
/// — works without modification.
pub fn self_refund_message(recipe: &SelfRefundRecipe, recent_blockhash: [u8; 32]) -> Vec<u8> {
    let data = vec![TAG_SELF_REFUND];

    // Account ordering for the on-chain instruction: buyer, authority,
    // pool, participation, vault, buyer_ata, token_program. Layout in the
    // serialized message follows the same slot conventions as deposit.
    let account_keys = vec![
        recipe.buyer,
        recipe.authority,
        recipe.pool,
        recipe.participation,
        recipe.vault,
        recipe.buyer_ata,
        recipe.program_id,
        token_program_id(),
    ];

    let mut message = Vec::new();
    // header: numRequiredSignatures=2 (buyer, authority),
    //         numReadonlySignedAccounts=1 (authority readonly),
    //         numReadonlyUnsignedAccounts=2 (program_id, token_program)
    message.push(2);
    message.push(1);
    message.push(2);
    encode_shortvec(account_keys.len(), &mut message);
    for key in &account_keys {
        message.extend_from_slice(key.as_ref());
    }
    message.extend_from_slice(&recent_blockhash);
    encode_shortvec(1, &mut message);
    message.push(6); // programIdIndex (recipe.program_id is at slot 6)
    encode_shortvec(7, &mut message);
    // instruction-account indices: buyer(0), authority(1), pool(2),
    // participation(3), vault(4), buyer_ata(5), token_program(7)
    message.extend_from_slice(&[0, 1, 2, 3, 4, 5, 7]);
    encode_shortvec(data.len(), &mut message);
    message.extend_from_slice(&data);
    message
}

/// Builds the wire-format Solana legacy message bytes for a deposit
/// instruction. We hand-roll the bytes (instead of going through `solana-sdk`)
/// so the worker doesn't need the full SDK in its WASM build, and so the
/// authority signature we attach is bound to a known byte sequence we control.
///
/// The signer order in the resulting message is fixed: index 0 is the buyer
/// (fee payer), index 1 is the authority. The client deserializes the full
/// transaction bytes and lets Phantom sign the buyer slot without recompiling
/// the message — see `handle_escrow_deposit_build` for the rationale.
pub fn deposit_message(recipe: &DepositRecipe, recent_blockhash: [u8; 32]) -> Vec<u8> {
    let mut data = Vec::with_capacity(1 + 32 + 1 + 8 + 8 + 8);
    data.push(TAG_DEPOSIT);
    data.extend_from_slice(&recipe.user_id.inner());
    data.push(recipe.participation_bump);
    data.extend_from_slice(&recipe.product_amount.to_le_bytes());
    data.extend_from_slice(&recipe.shipping_amount.to_le_bytes());
    data.extend_from_slice(&recipe.quantity.to_le_bytes());

    let account_keys = vec![
        recipe.buyer,
        recipe.authority,
        recipe.pool,
        recipe.participation,
        recipe.vault,
        recipe.buyer_ata,
        system_program_id(),
        recipe.usdc_mint,
        recipe.program_id,
        token_program_id(),
    ];

    let mut message = Vec::new();
    message.push(2);
    message.push(1);
    message.push(4);
    encode_shortvec(account_keys.len(), &mut message);
    for key in &account_keys {
        message.extend_from_slice(key.as_ref());
    }
    message.extend_from_slice(&recent_blockhash);
    encode_shortvec(1, &mut message);
    message.push(8);
    encode_shortvec(9, &mut message);
    message.extend_from_slice(&[0, 1, 2, 3, 4, 5, 7, 6, 9]);
    encode_shortvec(data.len(), &mut message);
    message.extend_from_slice(&data);
    message
}

pub fn transaction_bytes(signatures: &[Vec<u8>], message: &[u8]) -> Vec<u8> {
    let mut tx = Vec::new();
    encode_shortvec(signatures.len(), &mut tx);
    for sig in signatures {
        tx.extend_from_slice(sig);
    }
    tx.extend_from_slice(message);
    tx
}

pub fn authority_sign_deposit_transaction(
    cfg: &SolanaConfig,
    signed_tx_bytes: &[u8],
    recipe: &DepositRecipe,
) -> ApiResult<Vec<u8>> {
    let parsed = parse_transaction_bytes(signed_tx_bytes)?;
    let message = parse_legacy_message(&parsed.message)?;
    validate_deposit_message(&message, recipe)?;
    authority_sign_parsed_transaction(cfg, parsed, &recipe.buyer, &recipe.authority)
}

pub fn authority_sign_self_refund_transaction(
    cfg: &SolanaConfig,
    signed_tx_bytes: &[u8],
    recipe: &SelfRefundRecipe,
) -> ApiResult<Vec<u8>> {
    let parsed = parse_transaction_bytes(signed_tx_bytes)?;
    let message = parse_legacy_message(&parsed.message)?;
    validate_self_refund_message(&message, recipe)?;
    authority_sign_parsed_transaction(cfg, parsed, &recipe.buyer, &recipe.authority)
}

fn authority_sign_parsed_transaction(
    cfg: &SolanaConfig,
    parsed: ParsedTransaction,
    expected_buyer: &Pubkey,
    expected_authority: &Pubkey,
) -> ApiResult<Vec<u8>> {
    let message = parse_legacy_message(&parsed.message)?;
    if message.num_required_signatures != 2 {
        return Err(ApiError::Validation(format!(
            "expected 2 required signatures, got {}",
            message.num_required_signatures
        )));
    }
    if message.account_keys.first() != Some(expected_buyer) {
        return Err(ApiError::Validation(
            "signed transaction fee payer is not the approved buyer".to_string(),
        ));
    }
    if message.account_keys.get(1) != Some(expected_authority) {
        return Err(ApiError::Validation(
            "signed transaction authority signer is not approved".to_string(),
        ));
    }
    if parsed.signatures.len() != usize::from(message.num_required_signatures) {
        return Err(ApiError::Validation(format!(
            "expected {} transaction signatures, got {}",
            message.num_required_signatures,
            parsed.signatures.len()
        )));
    }
    if parsed.signatures[0] == SIGNATURE_PLACEHOLDER {
        return Err(ApiError::Validation(
            "buyer signature is missing from transaction".to_string(),
        ));
    }

    let buyer_verifying_key = VerifyingKey::from_bytes(expected_buyer.as_ref())
        .map_err(|err| ApiError::Validation(format!("invalid buyer public key: {err}")))?;
    let buyer_signature = Signature::from_slice(&parsed.signatures[0])
        .map_err(|err| ApiError::Validation(format!("invalid buyer signature bytes: {err}")))?;
    buyer_verifying_key
        .verify(&parsed.message, &buyer_signature)
        .map_err(|err| {
            ApiError::Validation(format!("buyer transaction signature failed: {err}"))
        })?;

    let authority_signature = signing_key_from_config(cfg)?.sign(&parsed.message);
    Ok(transaction_bytes(
        &[
            parsed.signatures[0].to_vec(),
            authority_signature.to_bytes().to_vec(),
        ],
        &parsed.message,
    ))
}

fn validate_deposit_message(
    message: &ParsedLegacyMessage,
    recipe: &DepositRecipe,
) -> ApiResult<()> {
    let mut data = Vec::with_capacity(1 + 32 + 1 + 8 + 8 + 8);
    data.push(TAG_DEPOSIT);
    data.extend_from_slice(&recipe.user_id.inner());
    data.push(recipe.participation_bump);
    data.extend_from_slice(&recipe.product_amount.to_le_bytes());
    data.extend_from_slice(&recipe.shipping_amount.to_le_bytes());
    data.extend_from_slice(&recipe.quantity.to_le_bytes());

    validate_single_program_instruction(
        message,
        &recipe.program_id,
        &[
            recipe.buyer,
            recipe.authority,
            recipe.pool,
            recipe.participation,
            recipe.vault,
            recipe.buyer_ata,
            recipe.usdc_mint,
            system_program_id(),
            token_program_id(),
        ],
        &data,
    )
}

fn validate_self_refund_message(
    message: &ParsedLegacyMessage,
    recipe: &SelfRefundRecipe,
) -> ApiResult<()> {
    validate_single_program_instruction(
        message,
        &recipe.program_id,
        &[
            recipe.buyer,
            recipe.authority,
            recipe.pool,
            recipe.participation,
            recipe.vault,
            recipe.buyer_ata,
            token_program_id(),
        ],
        &[TAG_SELF_REFUND],
    )
}

fn validate_single_program_instruction(
    message: &ParsedLegacyMessage,
    program_id: &Pubkey,
    expected_accounts: &[Pubkey],
    expected_data: &[u8],
) -> ApiResult<()> {
    // Phantom injects compute-budget instructions at signing time, so the
    // total instruction count may be > 1. Find the one targeting our program.
    let mut found: Option<&ParsedCompiledInstruction> = None;
    for ix in &message.instructions {
        let ix_program = message
            .account_keys
            .get(usize::from(ix.program_id_index))
            .ok_or_else(|| {
                ApiError::Validation("instruction program index out of bounds".to_string())
            })?;
        if ix_program == program_id {
            if found.is_some() {
                return Err(ApiError::Validation(
                    "transaction contains multiple escrow instructions".to_string(),
                ));
            }
            found = Some(ix);
        }
    }
    let ix = found.ok_or_else(|| {
        ApiError::Validation("signed transaction targets the wrong escrow program".to_string())
    })?;
    if ix.data != expected_data {
        return Err(ApiError::Validation(
            "signed transaction escrow instruction data was modified".to_string(),
        ));
    }
    if ix.account_indices.len() != expected_accounts.len() {
        return Err(ApiError::Validation(format!(
            "expected {} escrow accounts, got {}",
            expected_accounts.len(),
            ix.account_indices.len()
        )));
    }

    for (position, (actual_index, expected_key)) in ix
        .account_indices
        .iter()
        .zip(expected_accounts.iter())
        .enumerate()
    {
        let actual_key = message
            .account_keys
            .get(usize::from(*actual_index))
            .ok_or_else(|| {
                ApiError::Validation("instruction account index out of bounds".to_string())
            })?;
        if actual_key != expected_key {
            return Err(ApiError::Validation(format!(
                "escrow instruction account {position} does not match the approved transaction"
            )));
        }
    }

    Ok(())
}

struct ParsedTransaction {
    signatures: Vec<[u8; 64]>,
    message: Vec<u8>,
}

struct ParsedLegacyMessage {
    num_required_signatures: u8,
    account_keys: Vec<Pubkey>,
    instructions: Vec<ParsedCompiledInstruction>,
}

struct ParsedCompiledInstruction {
    program_id_index: u8,
    account_indices: Vec<u8>,
    data: Vec<u8>,
}

fn parse_transaction_bytes(bytes: &[u8]) -> ApiResult<ParsedTransaction> {
    let mut cursor = 0usize;
    let signature_count = decode_shortvec(bytes, &mut cursor)?;
    let signatures_len = signature_count
        .checked_mul(64)
        .ok_or_else(|| ApiError::Validation("transaction signature length overflow".to_string()))?;
    let end = cursor
        .checked_add(signatures_len)
        .ok_or_else(|| ApiError::Validation("transaction signature length overflow".to_string()))?;
    if end > bytes.len() {
        return Err(ApiError::Validation(
            "transaction is shorter than its signature section".to_string(),
        ));
    }

    let mut signatures = Vec::with_capacity(signature_count);
    for chunk in bytes[cursor..end].chunks_exact(64) {
        let mut signature = [0u8; 64];
        signature.copy_from_slice(chunk);
        signatures.push(signature);
    }

    Ok(ParsedTransaction {
        signatures,
        message: bytes[end..].to_vec(),
    })
}

fn parse_legacy_message(bytes: &[u8]) -> ApiResult<ParsedLegacyMessage> {
    let mut cursor = 0usize;
    let num_required_signatures = read_u8_from_bytes(bytes, &mut cursor)?;
    let _num_readonly_signed_accounts = read_u8_from_bytes(bytes, &mut cursor)?;
    let _num_readonly_unsigned_accounts = read_u8_from_bytes(bytes, &mut cursor)?;

    let account_key_count = decode_shortvec(bytes, &mut cursor)?;
    let mut account_keys = Vec::with_capacity(account_key_count);
    for _ in 0..account_key_count {
        account_keys.push(Pubkey::from_bytes(read_32_from_bytes(bytes, &mut cursor)?));
    }

    let _recent_blockhash = read_32_from_bytes(bytes, &mut cursor)?;
    let instruction_count = decode_shortvec(bytes, &mut cursor)?;
    let mut instructions = Vec::with_capacity(instruction_count);

    for _ in 0..instruction_count {
        let program_id_index = read_u8_from_bytes(bytes, &mut cursor)?;
        let account_count = decode_shortvec(bytes, &mut cursor)?;
        let account_indices = read_vec_from_bytes(bytes, &mut cursor, account_count)?;
        let data_len = decode_shortvec(bytes, &mut cursor)?;
        let data = read_vec_from_bytes(bytes, &mut cursor, data_len)?;
        instructions.push(ParsedCompiledInstruction {
            program_id_index,
            account_indices,
            data,
        });
    }

    if cursor != bytes.len() {
        return Err(ApiError::Validation(
            "legacy transaction message has trailing bytes".to_string(),
        ));
    }

    Ok(ParsedLegacyMessage {
        num_required_signatures,
        account_keys,
        instructions,
    })
}

fn read_u8_from_bytes(bytes: &[u8], cursor: &mut usize) -> ApiResult<u8> {
    let Some(value) = bytes.get(*cursor).copied() else {
        return Err(ApiError::Validation(
            "transaction message ended unexpectedly".to_string(),
        ));
    };
    *cursor += 1;
    Ok(value)
}

fn read_32_from_bytes(bytes: &[u8], cursor: &mut usize) -> ApiResult<[u8; 32]> {
    let end = cursor
        .checked_add(32)
        .ok_or_else(|| ApiError::Validation("transaction message offset overflow".to_string()))?;
    let slice = bytes.get(*cursor..end).ok_or_else(|| {
        ApiError::Validation("transaction message ended unexpectedly".to_string())
    })?;
    let mut out = [0u8; 32];
    out.copy_from_slice(slice);
    *cursor = end;
    Ok(out)
}

fn read_vec_from_bytes(bytes: &[u8], cursor: &mut usize, len: usize) -> ApiResult<Vec<u8>> {
    let end = cursor
        .checked_add(len)
        .ok_or_else(|| ApiError::Validation("transaction message offset overflow".to_string()))?;
    let slice = bytes.get(*cursor..end).ok_or_else(|| {
        ApiError::Validation("transaction message ended unexpectedly".to_string())
    })?;
    *cursor = end;
    Ok(slice.to_vec())
}

fn decode_shortvec(bytes: &[u8], cursor: &mut usize) -> ApiResult<usize> {
    let mut result = 0usize;
    let mut shift = 0usize;

    loop {
        let Some(byte) = bytes.get(*cursor).copied() else {
            return Err(ApiError::Validation(
                "shortvec ended before value completed".to_string(),
            ));
        };
        *cursor += 1;

        let value = usize::from(byte & 0x7f);
        result |= value
            .checked_shl(shift as u32)
            .ok_or_else(|| ApiError::Validation("shortvec value overflow".to_string()))?;

        if byte & 0x80 == 0 {
            return Ok(result);
        }

        shift += 7;
        if shift >= usize::BITS as usize {
            return Err(ApiError::Validation("shortvec value overflow".to_string()));
        }
    }
}

pub async fn rpc_get_latest_blockhash(ctx: &ApiContext) -> ApiResult<String> {
    #[derive(serde::Deserialize)]
    struct BlockhashValue {
        blockhash: String,
    }
    #[derive(serde::Deserialize)]
    struct RpcResult {
        value: BlockhashValue,
    }

    let res: RpcResult = rpc_call(
        &ctx.config.solana.rpc_url,
        "getLatestBlockhash",
        json!([{"commitment":"confirmed"}]),
    )
    .await?;
    Ok(res.value.blockhash)
}

pub async fn rpc_account_exists(ctx: &ApiContext, address: &Pubkey) -> ApiResult<bool> {
    Ok(rpc_get_account_data(ctx, address).await?.is_some())
}

/// Fetches the raw bytes of an on-chain account, or `None` if it doesn't
/// exist. Used by the deposit/refund confirm flows to verify that the
/// on-chain state is real before mirroring it into D1.
///
/// Reads at `confirmed` commitment, matching the JS bridge's
/// `connection.confirmTransaction(sig, "confirmed")`. Without this, the
/// confirm endpoint sees stale `finalized` state and returns "account not
/// found yet" even though the user's tx already landed.
pub async fn rpc_get_account_data(
    ctx: &ApiContext,
    address: &Pubkey,
) -> ApiResult<Option<Vec<u8>>> {
    #[derive(serde::Deserialize)]
    struct AccountInfo {
        // `[base64_data, "base64"]`
        data: Option<(String, String)>,
    }
    #[derive(serde::Deserialize)]
    struct RpcResult {
        value: Option<AccountInfo>,
    }

    let res: RpcResult = rpc_call(
        &ctx.config.solana.rpc_url,
        "getAccountInfo",
        json!([
            address.to_base58(),
            {"encoding":"base64", "commitment":"confirmed"}
        ]),
    )
    .await?;

    let Some(account) = res.value else {
        return Ok(None);
    };
    let Some((data_b64, _encoding)) = account.data else {
        return Ok(Some(Vec::new()));
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_b64)
        .map_err(|err| ApiError::Base64Decode(err.to_string()))?;
    Ok(Some(bytes))
}

/// Decoded view of an on-chain `Participation` account. Mirrors the layout
/// in `packages/solana-programs/market/src/state.rs` — keep these in sync if
/// the on-chain struct changes (adding fields requires bumping the version
/// byte and updating both sides). Some fields aren't surfaced through the
/// API yet but are kept here so the parser stays a faithful mirror.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ParticipationAccount {
    pub version: u8,
    pub bump: u8,
    pub refunded: bool,
    pub pool: Pubkey,
    pub user_id: [u8; 32],
    pub wallet: Pubkey,
    pub product_amount: u64,
    pub shipping_amount: u64,
    pub deposited_at: i64,
    pub quantity: u64,
}

impl ParticipationAccount {
    /// On-chain `Participation::LEN` — `repr(C)` size of the struct including
    /// padding. Hardcoded so the WASM build doesn't depend on the program crate.
    /// Layout: version(1) bump(1) refunded(1) pad0(1) pad1(4) pool(32)
    /// user_id(32) wallet(32) product_amount(8) shipping_amount(8)
    /// deposited_at(8) quantity(8) = 136
    pub const LEN: usize = 136;

    pub fn from_bytes(bytes: &[u8]) -> ApiResult<Self> {
        if bytes.len() != Self::LEN {
            return Err(ApiError::Validation(format!(
                "participation account is {} bytes, expected {}",
                bytes.len(),
                Self::LEN,
            )));
        }
        let mut pool = [0u8; 32];
        pool.copy_from_slice(&bytes[8..40]);
        let mut user_id = [0u8; 32];
        user_id.copy_from_slice(&bytes[40..72]);
        let mut wallet = [0u8; 32];
        wallet.copy_from_slice(&bytes[72..104]);
        let product_amount = u64::from_le_bytes(bytes[104..112].try_into().unwrap());
        let shipping_amount = u64::from_le_bytes(bytes[112..120].try_into().unwrap());
        let deposited_at = i64::from_le_bytes(bytes[120..128].try_into().unwrap());
        let quantity = u64::from_le_bytes(bytes[128..136].try_into().unwrap());
        Ok(Self {
            version: bytes[0],
            bump: bytes[1],
            refunded: bytes[2] != 0,
            pool: Pubkey::from_bytes(pool),
            user_id,
            wallet: Pubkey::from_bytes(wallet),
            product_amount,
            shipping_amount,
            deposited_at,
            quantity,
        })
    }
}

/// Decoded view of an on-chain `Pool` account — only the fields we surface
/// in the threshold UI. Keep field offsets in sync with the on-chain
/// `Pool` struct in `market/src/state.rs`.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PoolAccount {
    pub status: u8,
    pub product_total: u64,
    pub shipping_total: u64,
    pub participant_count: u32,
    pub threshold: u32,
    pub batch_id: u32,
    pub total_quantity: u64,
}

impl PoolAccount {
    /// On-chain `Pool::LEN`. Layout (in order):
    ///   version(1) bump(1) vault_bump(1) status(1) pad(4)
    ///   authority(32) product_hash(32) usdc_mint(32) vault(32)
    ///   product_total(8) shipping_total(8) refundable_outstanding(8)
    ///   participant_count(4) threshold(4)
    ///   created_at(8) updated_at(8) batch_id(4) pad3(4)
    ///   total_quantity(8) = 200
    pub const LEN: usize = 200;

    pub fn from_bytes(bytes: &[u8]) -> ApiResult<Self> {
        if bytes.len() != Self::LEN {
            return Err(ApiError::Validation(format!(
                "pool account is {} bytes, expected {}",
                bytes.len(),
                Self::LEN,
            )));
        }
        Ok(Self {
            status: bytes[3],
            product_total: u64::from_le_bytes(bytes[136..144].try_into().unwrap()),
            shipping_total: u64::from_le_bytes(bytes[144..152].try_into().unwrap()),
            participant_count: u32::from_le_bytes(bytes[160..164].try_into().unwrap()),
            threshold: u32::from_le_bytes(bytes[164..168].try_into().unwrap()),
            batch_id: u32::from_le_bytes(bytes[184..188].try_into().unwrap()),
            total_quantity: u64::from_le_bytes(bytes[192..200].try_into().unwrap()),
        })
    }
}

pub async fn rpc_send_transaction(ctx: &ApiContext, tx_bytes: &[u8]) -> ApiResult<String> {
    let encoded = base64::engine::general_purpose::STANDARD.encode(tx_bytes);
    rpc_call(
        &ctx.config.solana.rpc_url,
        "sendTransaction",
        json!([
            encoded,
            {
                "encoding":"base64",
                "preflightCommitment":"confirmed"
            }
        ]),
    )
    .await
}

/// Idempotently creates the on-chain Pool + Vault for a given
/// `(product_id, batch_id, threshold)`. Returns `Ok(true)` if a new pool
/// was created, `Ok(false)` if it already existed. Used by both the
/// deposit-build flow (for the very first batch) and the cron job that
/// opens the next batch after the previous one auto-locks.
pub async fn ensure_pool_initialized(
    ctx: &ApiContext,
    product_id: &ProductId,
    batch_id: u32,
    threshold: u32,
) -> ApiResult<bool> {
    let recipe = build_initialize_pool_recipe(&ctx.config.solana, product_id, batch_id, threshold)?;
    if rpc_account_exists(ctx, &recipe.pool).await? {
        return Ok(false);
    }

    let blockhash = rpc_get_latest_blockhash(ctx).await?;
    let blockhash_bytes = decode_blockhash(&blockhash)?;
    let message = initialize_pool_message(&recipe, blockhash_bytes);
    let signing = signing_key_from_config(&ctx.config.solana)?;
    let signature = signing.sign(&message).to_bytes().to_vec();
    let tx = transaction_bytes(&[signature], &message);

    match rpc_send_transaction(ctx, &tx).await {
        Ok(_) => Ok(true),
        Err(err) => {
            if rpc_account_exists(ctx, &recipe.pool).await.unwrap_or(false) {
                Ok(false)
            } else {
                Err(err)
            }
        }
    }
}

fn system_program_id() -> Pubkey {
    Pubkey::from_base58("11111111111111111111111111111111").expect("system program id should parse")
}

fn token_program_id() -> Pubkey {
    Pubkey::from_base58(TOKEN_PROGRAM_ID_STR).expect("token program id should parse")
}

fn associated_token_program_id() -> Pubkey {
    Pubkey::from_base58(ASSOCIATED_TOKEN_PROGRAM_ID_STR)
        .expect("associated token program id should parse")
}

fn associated_token_address(wallet: &Pubkey, mint: &Pubkey) -> ApiResult<Pubkey> {
    find_program_address(
        &[wallet.as_ref(), token_program_id().as_ref(), mint.as_ref()],
        &associated_token_program_id(),
    )
    .map(|(address, _)| address)
}

fn find_program_address(seeds: &[&[u8]], program_id: &Pubkey) -> ApiResult<(Pubkey, u8)> {
    for bump in (0u8..=255).rev() {
        let bump_seed = [bump];
        let mut all = seeds.to_vec();
        all.push(&bump_seed);
        if let Ok(address) = create_program_address(&all, program_id) {
            return Ok((address, bump));
        }
    }
    Err(ApiError::Validation(
        "failed to derive program address".to_string(),
    ))
}

fn create_program_address(seeds: &[&[u8]], program_id: &Pubkey) -> ApiResult<Pubkey> {
    let mut hasher = Sha256::new();
    for seed in seeds {
        if seed.len() > 32 {
            return Err(ApiError::Validation(format!(
                "program address seed too long: {}",
                seed.len()
            )));
        }
        hasher.update(seed);
    }
    hasher.update(program_id.as_ref());
    hasher.update(PROGRAM_DERIVED_ADDRESS_MARKER);
    let digest: [u8; 32] = hasher.finalize().into();

    if CompressedEdwardsY(digest).decompress().is_some() {
        return Err(ApiError::Validation(
            "derived address must be off-curve".to_string(),
        ));
    }

    Ok(Pubkey::from_bytes(digest))
}

fn encode_shortvec(mut value: usize, out: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value > 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

async fn rpc_call<T: DeserializeOwned>(
    rpc_url: &str,
    method: &str,
    params: serde_json::Value,
) -> ApiResult<T> {
    #[derive(serde::Deserialize)]
    struct RpcEnvelope<T> {
        result: Option<T>,
        error: Option<RpcError>,
    }

    #[derive(serde::Deserialize)]
    struct RpcError {
        code: i64,
        message: String,
    }

    let body = serde_json::to_vec(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    }))
    .map_err(|err| ApiError::Request(format!("failed to encode rpc request: {err}")))?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post);
    init.with_body(Some(body.into()));

    let mut req = Request::new_with_init(rpc_url, &init)
        .map_err(|err| ApiError::Request(format!("failed to build rpc request: {err}")))?;
    req.headers_mut()
        .map_err(|err| ApiError::Request(format!("failed to access rpc headers: {err}")))?
        .set("Content-Type", "application/json")
        .map_err(|err| ApiError::Request(format!("failed to set rpc headers: {err}")))?;

    let mut resp = worker::Fetch::Request(req)
        .send()
        .await
        .map_err(|err| ApiError::Request(format!("rpc request failed: {err}")))?;
    let text = resp
        .text()
        .await
        .map_err(|err| ApiError::Request(format!("failed to read rpc response: {err}")))?;

    let envelope: RpcEnvelope<T> = serde_json::from_str(&text).map_err(|err| {
        ApiError::Request(format!("failed to parse rpc response: {err}. body: {text}"))
    })?;

    match (envelope.result, envelope.error) {
        (Some(result), None) => Ok(result),
        (_, Some(err)) => Err(ApiError::Request(format!(
            "rpc {method} failed ({}): {}",
            err.code, err.message
        ))),
        _ => Err(ApiError::Request(format!(
            "rpc {method} returned neither result nor error"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer as _;
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey as SolanaPubkey,
        signature::{Keypair, Signer},
        transaction::Transaction,
    };

    fn test_config(program_id: &str, mint: &str) -> SolanaConfig {
        SolanaConfig {
            network: "local".to_string(),
            rpc_url: "http://localhost:9086".to_string(),
            market_program_id: program_id.to_string(),
            usdc_mint: mint.to_string(),
            authority_signing_key: [7u8; 32],
        }
    }

    #[test]
    fn derive_addresses_matches_solana_sdk() {
        let program_id = "EiSozRFZHRueq39P6kt1NVtX8JXQqcgprN6w9mJZy1ft";
        let mint = "AA78vZXVzYHhrB5rNZmmectaVyAA1qJ7VfjdThmewa3V";
        let cfg = test_config(program_id, mint);
        let product_id = ProductId::new("test-product").expect("valid slug");
        let user_id = UserId::from([3u8; 32]);
        let wallet = Pubkey::from_base58("B16PwUZzeJzUzkPfRLBYdXznyupjJX1pPVpdFWNhr9Kp")
            .expect("valid wallet");

        let batch_id: u32 = 7;
        let derived =
            derive_addresses(&cfg, &product_id, &wallet, &user_id, batch_id).expect("derive");

        let program = SolanaPubkey::from_str(program_id).expect("program");
        let mint = SolanaPubkey::from_str(mint).expect("mint");
        let wallet =
            SolanaPubkey::from_str("B16PwUZzeJzUzkPfRLBYdXznyupjJX1pPVpdFWNhr9Kp").expect("wallet");
        let product_hash = product_id.hash();
        let batch_id_bytes = batch_id.to_le_bytes();

        let (expected_pool, _) = SolanaPubkey::find_program_address(
            &[POOL_SEED, product_hash.as_ref(), &batch_id_bytes],
            &program,
        );
        let (expected_vault, _) = SolanaPubkey::find_program_address(
            &[VAULT_SEED, product_hash.as_ref(), &batch_id_bytes],
            &program,
        );
        let (expected_participation, expected_bump) = SolanaPubkey::find_program_address(
            &[
                PARTICIPATION_SEED,
                product_hash.as_ref(),
                &batch_id_bytes,
                user_id.as_ref(),
            ],
            &program,
        );
        let expected_ata = SolanaPubkey::find_program_address(
            &[wallet.as_ref(), token_program_id().as_ref(), mint.as_ref()],
            &SolanaPubkey::from_str(ASSOCIATED_TOKEN_PROGRAM_ID_STR).expect("ata program"),
        )
        .0;

        assert_eq!(derived.pool.to_string(), expected_pool.to_string());
        assert_eq!(derived.vault.to_string(), expected_vault.to_string());
        assert_eq!(
            derived.participation.to_string(),
            expected_participation.to_string()
        );
        assert_eq!(derived.participation_bump, expected_bump);
        assert_eq!(derived.buyer_ata.to_string(), expected_ata.to_string());
    }

    #[test]
    fn initialize_pool_transaction_bytes_match_solana_sdk() {
        let authority_secret = [11u8; 32];
        let authority = Keypair::try_from(&{
            let mut bytes = [0u8; 64];
            bytes[..32].copy_from_slice(&authority_secret);
            let public = Keypair::new_from_array(authority_secret)
                .pubkey()
                .to_bytes();
            bytes[32..].copy_from_slice(&public);
            bytes
        })
        .expect("keypair");
        let cfg = SolanaConfig {
            network: "local".to_string(),
            rpc_url: "http://localhost:9086".to_string(),
            market_program_id: "EiSozRFZHRueq39P6kt1NVtX8JXQqcgprN6w9mJZy1ft".to_string(),
            usdc_mint: "AA78vZXVzYHhrB5rNZmmectaVyAA1qJ7VfjdThmewa3V".to_string(),
            authority_signing_key: authority_secret,
        };
        let product_id = ProductId::new("test-product").expect("valid slug");
        let batch_id: u32 = 0;
        let threshold: u32 = 6;
        let recipe =
            build_initialize_pool_recipe(&cfg, &product_id, batch_id, threshold).expect("recipe");
        let recent_blockhash = [9u8; 32];

        let backend_message = initialize_pool_message(&recipe, recent_blockhash);
        let backend_signature = signing_key_from_config(&cfg)
            .expect("signing key")
            .sign(&backend_message)
            .to_bytes()
            .to_vec();
        let backend_tx = transaction_bytes(&[backend_signature], &backend_message);

        let ix = Instruction {
            program_id: SolanaPubkey::from_str(&cfg.market_program_id).expect("program"),
            accounts: vec![
                AccountMeta::new(authority.pubkey(), true),
                AccountMeta::new(
                    SolanaPubkey::from_str(&recipe.pool.to_string()).expect("pool"),
                    false,
                ),
                AccountMeta::new(
                    SolanaPubkey::from_str(&recipe.vault.to_string()).expect("vault"),
                    false,
                ),
                AccountMeta::new_readonly(
                    SolanaPubkey::from_str(&cfg.usdc_mint).expect("mint"),
                    false,
                ),
                AccountMeta::new_readonly(
                    SolanaPubkey::from_str("11111111111111111111111111111111").expect("system"),
                    false,
                ),
                AccountMeta::new_readonly(
                    SolanaPubkey::from_str(TOKEN_PROGRAM_ID_STR).expect("token"),
                    false,
                ),
            ],
            data: {
                let mut data = Vec::with_capacity(1 + 32 + 1 + 1 + 4 + 4);
                data.push(TAG_INITIALIZE_POOL);
                data.extend_from_slice(recipe.product_hash.as_ref());
                data.push(recipe.pool_bump);
                data.push(recipe.vault_bump);
                data.extend_from_slice(&recipe.batch_id.to_le_bytes());
                data.extend_from_slice(&recipe.threshold.to_le_bytes());
                data
            },
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&authority.pubkey()),
            &[&authority],
            solana_sdk::hash::Hash::new_from_array(recent_blockhash),
        );
        let mut sdk_tx = Vec::new();
        encode_shortvec(tx.signatures.len(), &mut sdk_tx);
        for signature in &tx.signatures {
            sdk_tx.extend_from_slice(signature.as_ref());
        }
        sdk_tx.extend_from_slice(&tx.message_data());

        assert_eq!(backend_tx, sdk_tx);
    }
}
