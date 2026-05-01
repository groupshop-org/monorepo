//! Market program subcommands.
//!
//! Mirrors the instructions defined in `packages/solana-programs/market`. See
//! `docs/solana-programs.md` for the design.

use std::{path::PathBuf, str::FromStr};

use anyhow::{Context, Result};
use clap::Subcommand;
use groupshop_backend_shared::prelude::{ProductId, UserId};
use sha2::{Digest, Sha256};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
    transaction::Transaction,
};

// System program id. Hardcoded rather than re-exported from a moving target in solana-sdk.
fn system_program_id() -> Pubkey {
    Pubkey::from_str("11111111111111111111111111111111").expect("system program id should parse")
}

use crate::context::CliCtx;

// Tag bytes — must match `packages/solana-programs/market/src/instruction.rs`.
const TAG_INITIALIZE_POOL: u8 = 0;
const TAG_DEPOSIT: u8 = 1;
const TAG_LOCK_POOL: u8 = 2;
const TAG_RELEASE: u8 = 3;
const TAG_ENTER_REFUND_MODE: u8 = 4;
const TAG_CLAIM_REFUND: u8 = 5;

const POOL_SEED: &[u8] = b"pool";
const VAULT_SEED: &[u8] = b"vault";
const PARTICIPATION_SEED: &[u8] = b"part";

// Token program — pinned to the standard SPL Token program (not Token-2022).
const TOKEN_PROGRAM_ID_STR: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

#[derive(Subcommand, Clone, Debug)]
pub enum MarketCmd {
    /// Show PDAs (pool, vault, participation) that a product slug resolves to.
    Pdas {
        #[arg(long)]
        program_id: String,
        #[arg(long)]
        product_slug: String,
        /// Optional: compute the participation PDA for this UserId too.
        #[arg(long)]
        user_id: Option<String>,
        /// Batch index — defaults to 0 (the very first batch of a
        /// product). Each subsequent group-buy round bumps the batch_id.
        #[arg(long, default_value_t = 0)]
        batch_id: u32,
    },

    /// Initialize a new product pool. Signed by the backend authority.
    InitializePool {
        #[arg(long)]
        program_id: String,
        #[arg(long)]
        product_slug: String,
        #[arg(long)]
        usdc_mint: String,
        /// Authority keypair (defaults to the CLI's --keypair).
        #[arg(long)]
        authority_keypair: Option<PathBuf>,
        #[arg(long, default_value_t = 0)]
        batch_id: u32,
        /// Group-deal threshold; the on-chain program auto-locks the pool
        /// once this many distinct participants have deposited.
        #[arg(long)]
        threshold: u32,
    },

    /// Deposit from a buyer's USDC account into the product pool. Signed by
    /// buyer + authority.
    Deposit {
        #[arg(long)]
        program_id: String,
        #[arg(long)]
        product_slug: String,
        #[arg(long)]
        user_id: String,
        #[arg(long)]
        buyer_keypair: PathBuf,
        /// Buyer's USDC ATA (use `spl-token accounts` to find).
        #[arg(long)]
        buyer_ata: String,
        #[arg(long)]
        product_amount: u64,
        #[arg(long)]
        shipping_amount: u64,
        /// Authority keypair (defaults to the CLI's --keypair).
        #[arg(long)]
        authority_keypair: Option<PathBuf>,
        #[arg(long, default_value_t = 0)]
        batch_id: u32,
        /// Number of product units this deposit commits to. Must be >= 1.
        /// The on-chain program adds this to `Pool.total_quantity` and
        /// auto-locks when the running total hits the pool's threshold.
        #[arg(long)]
        quantity: u64,
    },

    /// Lock the pool — no more deposits accepted.
    LockPool {
        #[arg(long)]
        program_id: String,
        #[arg(long)]
        product_slug: String,
        #[arg(long)]
        authority_keypair: Option<PathBuf>,
        #[arg(long, default_value_t = 0)]
        batch_id: u32,
    },

    /// Release the entire vault balance to a destination USDC ATA.
    Release {
        #[arg(long)]
        program_id: String,
        #[arg(long)]
        product_slug: String,
        #[arg(long)]
        destination_ata: String,
        #[arg(long)]
        authority_keypair: Option<PathBuf>,
        #[arg(long, default_value_t = 0)]
        batch_id: u32,
    },

    /// Flip the pool into Refunding state. Backend must re-seed the vault
    /// first if it is already Released.
    EnterRefundMode {
        #[arg(long)]
        program_id: String,
        #[arg(long)]
        product_slug: String,
        #[arg(long)]
        authority_keypair: Option<PathBuf>,
        #[arg(long, default_value_t = 0)]
        batch_id: u32,
    },

    /// Refund a participant in the currently-Refunding pool.
    ClaimRefund {
        #[arg(long)]
        program_id: String,
        #[arg(long)]
        product_slug: String,
        #[arg(long)]
        user_id: String,
        /// Destination USDC ATA (must be the wallet recorded on the participation).
        #[arg(long)]
        destination_ata: String,
        #[arg(long)]
        authority_keypair: Option<PathBuf>,
        #[arg(long, default_value_t = 0)]
        batch_id: u32,
    },
}

pub fn run(ctx: &mut CliCtx, cmd: MarketCmd) -> Result<()> {
    match cmd {
        MarketCmd::Pdas {
            program_id,
            product_slug,
            user_id,
            batch_id,
        } => run_pdas(&program_id, &product_slug, user_id.as_deref(), batch_id),
        MarketCmd::InitializePool {
            program_id,
            product_slug,
            usdc_mint,
            authority_keypair,
            batch_id,
            threshold,
        } => run_initialize_pool(
            ctx,
            &program_id,
            &product_slug,
            &usdc_mint,
            authority_keypair,
            batch_id,
            threshold,
        ),
        MarketCmd::Deposit {
            program_id,
            product_slug,
            user_id,
            buyer_keypair,
            buyer_ata,
            product_amount,
            shipping_amount,
            authority_keypair,
            batch_id,
            quantity,
        } => run_deposit(
            ctx,
            &program_id,
            &product_slug,
            &user_id,
            &buyer_keypair,
            &buyer_ata,
            product_amount,
            shipping_amount,
            authority_keypair,
            batch_id,
            quantity,
        ),
        MarketCmd::LockPool {
            program_id,
            product_slug,
            authority_keypair,
            batch_id,
        } => run_lock_pool(ctx, &program_id, &product_slug, authority_keypair, batch_id),
        MarketCmd::Release {
            program_id,
            product_slug,
            destination_ata,
            authority_keypair,
            batch_id,
        } => run_release(
            ctx,
            &program_id,
            &product_slug,
            &destination_ata,
            authority_keypair,
            batch_id,
        ),
        MarketCmd::EnterRefundMode {
            program_id,
            product_slug,
            authority_keypair,
            batch_id,
        } => run_enter_refund(ctx, &program_id, &product_slug, authority_keypair, batch_id),
        MarketCmd::ClaimRefund {
            program_id,
            product_slug,
            user_id,
            destination_ata,
            authority_keypair,
            batch_id,
        } => run_claim_refund(
            ctx,
            &program_id,
            &product_slug,
            &user_id,
            &destination_ata,
            authority_keypair,
            batch_id,
        ),
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

fn run_pdas(
    program_id_str: &str,
    product_slug: &str,
    user_id_str: Option<&str>,
    batch_id: u32,
) -> Result<()> {
    let program_id = Pubkey::from_str(program_id_str).context("invalid program ID")?;
    let hash = product_hash_from_slug(product_slug)?;
    let (pool, pool_bump) = derive_pool(&program_id, &hash, batch_id);
    let (vault, vault_bump) = derive_vault(&program_id, &hash, batch_id);
    println!("product_slug     = {product_slug}");
    println!("product_hash     = {}", hex32(&hash));
    println!("batch_id         = {batch_id}");
    println!("pool             = {pool}  (bump {pool_bump})");
    println!("vault            = {vault}  (bump {vault_bump})");
    if let Some(user_id_str) = user_id_str {
        let user_id = UserId::from_encoded_str(user_id_str)
            .map_err(|e| anyhow::anyhow!("invalid --user-id: {e}"))?;
        let (participation, bump) =
            derive_participation(&program_id, &hash, batch_id, &user_id.inner());
        println!("user_id          = {user_id}");
        println!("participation    = {participation}  (bump {bump})");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_initialize_pool(
    ctx: &mut CliCtx,
    program_id_str: &str,
    product_slug: &str,
    usdc_mint_str: &str,
    authority_keypair: Option<PathBuf>,
    batch_id: u32,
    threshold: u32,
) -> Result<()> {
    let program_id = Pubkey::from_str(program_id_str).context("invalid program ID")?;
    let usdc_mint = Pubkey::from_str(usdc_mint_str).context("invalid usdc mint")?;
    let token_program = token_program_id()?;
    let hash = product_hash_from_slug(product_slug)?;
    let (pool, pool_bump) = derive_pool(&program_id, &hash, batch_id);
    let (vault, vault_bump) = derive_vault(&program_id, &hash, batch_id);

    let authority = load_authority(ctx, authority_keypair)?;

    let mut data = Vec::with_capacity(1 + 32 + 1 + 1 + 4 + 4);
    data.push(TAG_INITIALIZE_POOL);
    data.extend_from_slice(&hash);
    data.push(pool_bump);
    data.push(vault_bump);
    data.extend_from_slice(&batch_id.to_le_bytes());
    data.extend_from_slice(&threshold.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(authority.pubkey(), true),
        AccountMeta::new(pool, false),
        AccountMeta::new(vault, false),
        AccountMeta::new_readonly(usdc_mint, false),
        AccountMeta::new_readonly(system_program_id(), false),
        AccountMeta::new_readonly(token_program, false),
    ];
    let ix = Instruction {
        program_id,
        accounts,
        data,
    };

    send_tx(ctx, &[ix], &[&authority], &authority.pubkey())?;
    println!("pool = {pool}");
    println!("vault = {vault}");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_deposit(
    ctx: &mut CliCtx,
    program_id_str: &str,
    product_slug: &str,
    user_id_str: &str,
    buyer_keypair_path: &PathBuf,
    buyer_ata_str: &str,
    product_amount: u64,
    shipping_amount: u64,
    authority_keypair: Option<PathBuf>,
    batch_id: u32,
    quantity: u64,
) -> Result<()> {
    let program_id = Pubkey::from_str(program_id_str).context("invalid program ID")?;
    let buyer_ata = Pubkey::from_str(buyer_ata_str).context("invalid buyer ata")?;
    let token_program = token_program_id()?;

    let hash = product_hash_from_slug(product_slug)?;
    let user_id = UserId::from_encoded_str(user_id_str)
        .map_err(|e| anyhow::anyhow!("invalid --user-id: {e}"))?;
    let user_id_bytes = user_id.inner();

    let (pool, _) = derive_pool(&program_id, &hash, batch_id);
    let (vault, _) = derive_vault(&program_id, &hash, batch_id);
    let (participation, part_bump) =
        derive_participation(&program_id, &hash, batch_id, &user_id_bytes);

    let buyer = read_keypair_file(buyer_keypair_path)
        .map_err(|e| anyhow::anyhow!("failed to read buyer keypair {buyer_keypair_path:?}: {e}"))?;
    let authority = load_authority(ctx, authority_keypair)?;

    let usdc_mint = fetch_mint_from_pool(ctx, &pool)?;

    let mut data = Vec::with_capacity(1 + 32 + 1 + 8 + 8 + 8);
    data.push(TAG_DEPOSIT);
    data.extend_from_slice(&user_id_bytes);
    data.push(part_bump);
    data.extend_from_slice(&product_amount.to_le_bytes());
    data.extend_from_slice(&shipping_amount.to_le_bytes());
    data.extend_from_slice(&quantity.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(buyer.pubkey(), true),
        AccountMeta::new_readonly(authority.pubkey(), true),
        AccountMeta::new(pool, false),
        AccountMeta::new(participation, false),
        AccountMeta::new(vault, false),
        AccountMeta::new(buyer_ata, false),
        AccountMeta::new_readonly(usdc_mint, false),
        AccountMeta::new_readonly(system_program_id(), false),
        AccountMeta::new_readonly(token_program, false),
    ];
    let ix = Instruction {
        program_id,
        accounts,
        data,
    };

    send_tx(ctx, &[ix], &[&buyer, &authority], &buyer.pubkey())?;
    println!("participation = {participation}");
    Ok(())
}

fn run_lock_pool(
    ctx: &mut CliCtx,
    program_id_str: &str,
    product_slug: &str,
    authority_keypair: Option<PathBuf>,
    batch_id: u32,
) -> Result<()> {
    let program_id = Pubkey::from_str(program_id_str).context("invalid program ID")?;
    let hash = product_hash_from_slug(product_slug)?;
    let (pool, _) = derive_pool(&program_id, &hash, batch_id);
    let authority = load_authority(ctx, authority_keypair)?;

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(authority.pubkey(), true),
            AccountMeta::new(pool, false),
        ],
        data: vec![TAG_LOCK_POOL],
    };
    send_tx(ctx, &[ix], &[&authority], &authority.pubkey())
}

#[allow(clippy::too_many_arguments)]
fn run_release(
    ctx: &mut CliCtx,
    program_id_str: &str,
    product_slug: &str,
    destination_ata_str: &str,
    authority_keypair: Option<PathBuf>,
    batch_id: u32,
) -> Result<()> {
    let program_id = Pubkey::from_str(program_id_str).context("invalid program ID")?;
    let destination = Pubkey::from_str(destination_ata_str).context("invalid destination ata")?;
    let token_program = token_program_id()?;
    let hash = product_hash_from_slug(product_slug)?;
    let (pool, _) = derive_pool(&program_id, &hash, batch_id);
    let (vault, _) = derive_vault(&program_id, &hash, batch_id);
    let authority = load_authority(ctx, authority_keypair)?;

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(authority.pubkey(), true),
            AccountMeta::new(pool, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(destination, false),
            AccountMeta::new_readonly(token_program, false),
        ],
        data: vec![TAG_RELEASE],
    };
    send_tx(ctx, &[ix], &[&authority], &authority.pubkey())
}

fn run_enter_refund(
    ctx: &mut CliCtx,
    program_id_str: &str,
    product_slug: &str,
    authority_keypair: Option<PathBuf>,
    batch_id: u32,
) -> Result<()> {
    let program_id = Pubkey::from_str(program_id_str).context("invalid program ID")?;
    let hash = product_hash_from_slug(product_slug)?;
    let (pool, _) = derive_pool(&program_id, &hash, batch_id);
    let (vault, _) = derive_vault(&program_id, &hash, batch_id);
    let authority = load_authority(ctx, authority_keypair)?;

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(authority.pubkey(), true),
            AccountMeta::new(pool, false),
            AccountMeta::new_readonly(vault, false),
        ],
        data: vec![TAG_ENTER_REFUND_MODE],
    };
    send_tx(ctx, &[ix], &[&authority], &authority.pubkey())
}

#[allow(clippy::too_many_arguments)]
fn run_claim_refund(
    ctx: &mut CliCtx,
    program_id_str: &str,
    product_slug: &str,
    user_id_str: &str,
    destination_ata_str: &str,
    authority_keypair: Option<PathBuf>,
    batch_id: u32,
) -> Result<()> {
    let program_id = Pubkey::from_str(program_id_str).context("invalid program ID")?;
    let destination = Pubkey::from_str(destination_ata_str).context("invalid destination ata")?;
    let token_program = token_program_id()?;

    let hash = product_hash_from_slug(product_slug)?;
    let user_id = UserId::from_encoded_str(user_id_str)
        .map_err(|e| anyhow::anyhow!("invalid --user-id: {e}"))?;
    let (pool, _) = derive_pool(&program_id, &hash, batch_id);
    let (vault, _) = derive_vault(&program_id, &hash, batch_id);
    let (participation, _) = derive_participation(&program_id, &hash, batch_id, &user_id.inner());

    let authority = load_authority(ctx, authority_keypair)?;

    let mut data = Vec::with_capacity(1 + 32);
    data.push(TAG_CLAIM_REFUND);
    data.extend_from_slice(&user_id.inner());

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(authority.pubkey(), true),
            AccountMeta::new(pool, false),
            AccountMeta::new(participation, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(destination, false),
            AccountMeta::new_readonly(token_program, false),
        ],
        data,
    };
    send_tx(ctx, &[ix], &[&authority], &authority.pubkey())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn product_hash_from_slug(slug: &str) -> Result<[u8; 32]> {
    let product_id =
        ProductId::from_str(slug).map_err(|e| anyhow::anyhow!("invalid product slug: {e}"))?;
    Ok(product_id.hash().inner())
}

fn derive_pool(program_id: &Pubkey, product_hash: &[u8; 32], batch_id: u32) -> (Pubkey, u8) {
    let batch_id_bytes = batch_id.to_le_bytes();
    Pubkey::find_program_address(&[POOL_SEED, product_hash, &batch_id_bytes], program_id)
}

fn derive_vault(program_id: &Pubkey, product_hash: &[u8; 32], batch_id: u32) -> (Pubkey, u8) {
    let batch_id_bytes = batch_id.to_le_bytes();
    Pubkey::find_program_address(&[VAULT_SEED, product_hash, &batch_id_bytes], program_id)
}

fn derive_participation(
    program_id: &Pubkey,
    product_hash: &[u8; 32],
    batch_id: u32,
    user_id: &[u8; 32],
) -> (Pubkey, u8) {
    let batch_id_bytes = batch_id.to_le_bytes();
    Pubkey::find_program_address(
        &[PARTICIPATION_SEED, product_hash, &batch_id_bytes, user_id],
        program_id,
    )
}

fn token_program_id() -> Result<Pubkey> {
    Pubkey::from_str(TOKEN_PROGRAM_ID_STR).context("failed to parse token program id")
}

fn load_authority(ctx: &mut CliCtx, authority_keypair: Option<PathBuf>) -> Result<Keypair> {
    match authority_keypair {
        Some(path) => read_keypair_file(&path)
            .map_err(|e| anyhow::anyhow!("failed to read authority keypair {path:?}: {e}")),
        None => {
            // Fall back to the CLI's default keypair. We clone from the context's keypair
            // so the caller can still use `ctx.keypair()` elsewhere.
            let kp = ctx.keypair()?;
            Keypair::try_from(kp.to_bytes().as_slice()).context("failed to copy default keypair")
        }
    }
}

fn send_tx(
    ctx: &mut CliCtx,
    instructions: &[Instruction],
    signers: &[&Keypair],
    payer: &Pubkey,
) -> Result<()> {
    let blockhash = ctx
        .rpc_client()?
        .get_latest_blockhash()
        .context("failed to get blockhash")?;
    let tx = Transaction::new_signed_with_payer(instructions, Some(payer), signers, blockhash);

    println!("Simulating...");
    let sim = ctx
        .rpc_client()?
        .simulate_transaction(&tx)
        .context("simulation failed")?;
    if let Some(logs) = &sim.value.logs {
        for log in logs {
            println!("  {log}");
        }
    }
    if let Some(err) = &sim.value.err {
        anyhow::bail!("simulation error: {err:?}");
    }

    println!("Sending...");
    let sig = ctx
        .rpc_client()?
        .send_and_confirm_transaction(&tx)
        .context("transaction failed")?;
    println!("Confirmed: {sig}");
    Ok(())
}

fn fetch_mint_from_pool(ctx: &mut CliCtx, pool: &Pubkey) -> Result<Pubkey> {
    // Pool account layout: u8 version, u8 bump, u8 vault_bump, u8 status, [u8;4] _pad,
    //                      [u8;32] authority, [u8;32] product_hash, [u8;32] usdc_mint, ...
    // Offset of usdc_mint = 8 + 32 + 32 = 72.
    let data = ctx
        .rpc_client()?
        .get_account_data(pool)
        .context("failed to fetch pool account")?;
    if data.len() < 72 + 32 {
        anyhow::bail!("pool account data too short: {}", data.len());
    }
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&data[72..72 + 32]);
    Ok(Pubkey::new_from_array(bytes))
}

fn hex32(bytes: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}

// Make clippy / rust-analyzer see the sha2 import is used when the product_hash
// helper above is inlined from groupshop-backend-shared (which does its own
// hashing). Retained here so the CLI can also hash raw slug strings if ever
// needed outside the ProductId type.
#[allow(dead_code)]
fn raw_sha256(bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().into()
}
