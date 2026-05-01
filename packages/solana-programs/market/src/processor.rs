use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    AccountView, Address, ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::{
    instructions::{InitializeAccount3, Transfer},
    state::Account as TokenAccount,
    ID as TOKEN_PROGRAM_ID,
};

use crate::{
    errors::MarketError,
    instruction::{read_array, read_u32, read_u64, read_u8, MarketIx},
    pda::{assert_pda, PARTICIPATION_SEED, POOL_SEED, VAULT_SEED},
    state::{Participation, Pool, PoolStatus, STATE_VERSION},
};

pub fn process(program_id: &Address, accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let (tag, rest) = data
        .split_first()
        .ok_or(ProgramError::from(MarketError::InvalidInstruction))?;

    match MarketIx::from_tag(*tag)? {
        MarketIx::InitializePool => initialize_pool(program_id, accounts, rest),
        MarketIx::Deposit => deposit(program_id, accounts, rest),
        MarketIx::LockPool => lock_pool(program_id, accounts),
        MarketIx::Release => release(program_id, accounts),
        MarketIx::EnterRefundMode => enter_refund_mode(program_id, accounts),
        MarketIx::ClaimRefund => claim_refund(program_id, accounts, rest),
        MarketIx::SelfRefund => self_refund(program_id, accounts),
    }
}

// -------------------------------------------------------------------------
// InitializePool
// -------------------------------------------------------------------------

fn initialize_pool(
    program_id: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
) -> ProgramResult {
    // Parse: product_hash[32], pool_bump, vault_bump, batch_id (u32 LE), threshold (u32 LE)
    let mut cursor = 0;
    let product_hash: [u8; 32] = read_array(data, &mut cursor)?;
    let pool_bump = read_u8(data, &mut cursor)?;
    let vault_bump = read_u8(data, &mut cursor)?;
    let batch_id = read_u32(data, &mut cursor)?;
    let threshold = read_u32(data, &mut cursor)?;
    if threshold == 0 {
        return Err(MarketError::InvalidThreshold.into());
    }
    let batch_id_bytes = batch_id.to_le_bytes();

    let [authority, pool_ai, vault_ai, mint_ai, system_program_ai, token_program_ai] =
        split_accounts(accounts, 6)?;

    if !authority.is_signer() {
        return Err(MarketError::MissingAuthoritySignature.into());
    }

    assert_pda(
        pool_ai.address(),
        &[POOL_SEED, &product_hash, &batch_id_bytes, &[pool_bump]],
        program_id,
        MarketError::InvalidPoolPda,
    )?;
    assert_pda(
        vault_ai.address(),
        &[VAULT_SEED, &product_hash, &batch_id_bytes, &[vault_bump]],
        program_id,
        MarketError::InvalidVaultPda,
    )?;

    if pool_ai.data_len() != 0 || pool_ai.lamports() != 0 {
        return Err(MarketError::PoolAlreadyInitialized.into());
    }

    // Create pool account (owned by our program)
    CreateAccount {
        from: authority,
        to: pool_ai,
        lamports: pinocchio_rent_minimum_balance(Pool::LEN)?,
        space: Pool::LEN as u64,
        owner: program_id,
    }
    .invoke_signed(&[Signer::from(&[
        Seed::from(POOL_SEED),
        Seed::from(&product_hash[..]),
        Seed::from(&batch_id_bytes[..]),
        Seed::from(&[pool_bump][..]),
    ])])?;

    // Create vault as an SPL token account (owned by Token program, authority = vault PDA itself)
    CreateAccount {
        from: authority,
        to: vault_ai,
        lamports: pinocchio_rent_minimum_balance(TokenAccount::LEN)?,
        space: TokenAccount::LEN as u64,
        owner: &TOKEN_PROGRAM_ID,
    }
    .invoke_signed(&[Signer::from(&[
        Seed::from(VAULT_SEED),
        Seed::from(&product_hash[..]),
        Seed::from(&batch_id_bytes[..]),
        Seed::from(&[vault_bump][..]),
    ])])?;

    // Initialize the token account in-place. Authority = vault PDA (self-owned).
    InitializeAccount3 {
        account: vault_ai,
        mint: mint_ai,
        owner: vault_ai.address(),
    }
    .invoke()?;

    // Write initial pool state
    let now = Clock::get()?.unix_timestamp;
    let pool = Pool::from_account_mut(pool_ai)?;
    pool.version = STATE_VERSION;
    pool.bump = pool_bump;
    pool.vault_bump = vault_bump;
    pool.status = PoolStatus::Open as u8;
    pool._pad = [0; 4];
    pool.authority = *authority.address().as_array();
    pool.product_hash = product_hash;
    pool.usdc_mint = *mint_ai.address().as_array();
    pool.vault = *vault_ai.address().as_array();
    pool.product_total = 0;
    pool.shipping_total = 0;
    pool.refundable_outstanding = 0;
    pool.participant_count = 0;
    pool.threshold = threshold;
    pool.created_at = now;
    pool.updated_at = now;
    pool.batch_id = batch_id;
    pool._pad3 = [0; 4];
    pool.total_quantity = 0;

    // Suppress unused variable warnings for accounts we don't directly touch (they're
    // passed to CPIs by the runtime via the account array).
    let _ = (system_program_ai, token_program_ai);

    Ok(())
}

// -------------------------------------------------------------------------
// Deposit
// -------------------------------------------------------------------------

fn deposit(program_id: &Address, accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let mut cursor = 0;
    let user_id: [u8; 32] = read_array(data, &mut cursor)?;
    let participation_bump = read_u8(data, &mut cursor)?;
    let product_amount = read_u64(data, &mut cursor)?;
    let shipping_amount = read_u64(data, &mut cursor)?;
    let quantity = read_u64(data, &mut cursor)?;

    let total = product_amount
        .checked_add(shipping_amount)
        .ok_or(ProgramError::from(MarketError::AmountOverflow))?;
    if total == 0 {
        return Err(MarketError::ZeroDepositAmount.into());
    }
    if quantity == 0 {
        return Err(MarketError::ZeroDepositAmount.into());
    }

    let [buyer, authority, pool_ai, participation_ai, vault_ai, buyer_ata_ai, mint_ai, system_program_ai, token_program_ai] =
        split_accounts(accounts, 9)?;

    if !buyer.is_signer() {
        return Err(MarketError::MissingBuyerSignature.into());
    }
    if !authority.is_signer() {
        return Err(MarketError::MissingAuthoritySignature.into());
    }

    // Load pool, check authority + status + mint + vault.
    let (product_hash, batch_id_bytes) = {
        let pool = Pool::from_account(pool_ai)?;
        if pool.status()? != PoolStatus::Open {
            return Err(MarketError::PoolNotOpen.into());
        }
        if &pool.authority_address() != authority.address() {
            return Err(MarketError::UnauthorizedAuthority.into());
        }
        if &pool.usdc_mint_address() != mint_ai.address() {
            return Err(MarketError::InvalidMint.into());
        }
        if &pool.vault_address() != vault_ai.address() {
            return Err(MarketError::InvalidVaultAccount.into());
        }
        let batch_id_bytes = pool.batch_id.to_le_bytes();
        // Verify pool PDA using stored bump and batch_id.
        assert_pda(
            pool_ai.address(),
            &[POOL_SEED, &pool.product_hash, &batch_id_bytes, &[pool.bump]],
            program_id,
            MarketError::InvalidPoolPda,
        )?;
        (pool.product_hash, batch_id_bytes)
    };

    // Verify participation PDA. The seeds include batch_id so each batch
    // gets a fresh participation account, allowing the same buyer to
    // participate in successive batches of the same product.
    assert_pda(
        participation_ai.address(),
        &[
            PARTICIPATION_SEED,
            &product_hash,
            &batch_id_bytes,
            &user_id,
            &[participation_bump],
        ],
        program_id,
        MarketError::InvalidParticipationPda,
    )?;

    // Create participation PDA on first deposit.
    let is_new = participation_ai.data_len() == 0;
    if is_new {
        CreateAccount {
            from: buyer,
            to: participation_ai,
            lamports: pinocchio_rent_minimum_balance(Participation::LEN)?,
            space: Participation::LEN as u64,
            owner: program_id,
        }
        .invoke_signed(&[Signer::from(&[
            Seed::from(PARTICIPATION_SEED),
            Seed::from(&product_hash[..]),
            Seed::from(&batch_id_bytes[..]),
            Seed::from(&user_id[..]),
            Seed::from(&[participation_bump][..]),
        ])])?;
    } else {
        // Re-deposit: same wallet, not yet refunded.
        let p = Participation::from_account(participation_ai)?;
        if p.wallet != *buyer.address().as_array() {
            return Err(MarketError::ParticipationWalletMismatch.into());
        }
        if p.refunded != 0 {
            return Err(MarketError::ParticipationAlreadyRefunded.into());
        }
    }

    // Move USDC from buyer ATA to vault. Buyer is the SPL token authority here;
    // no PDA seeds needed.
    Transfer {
        from: buyer_ata_ai,
        to: vault_ai,
        authority: buyer,
        multisig_signers: &[] as &[&AccountView],
        amount: total,
    }
    .invoke()?;

    let now = Clock::get()?.unix_timestamp;

    // Update participation.
    {
        let p = Participation::from_account_mut(participation_ai)?;
        if is_new {
            p.version = STATE_VERSION;
            p.bump = participation_bump;
            p.refunded = 0;
            p._pad0 = 0;
            p._pad1 = [0; 4];
            p.pool = *pool_ai.address().as_array();
            p.user_id = user_id;
            p.wallet = *buyer.address().as_array();
            p.product_amount = 0;
            p.shipping_amount = 0;
            p.deposited_at = now;
            p.quantity = 0;
        }
        p.product_amount = p
            .product_amount
            .checked_add(product_amount)
            .ok_or(ProgramError::from(MarketError::AmountOverflow))?;
        p.shipping_amount = p
            .shipping_amount
            .checked_add(shipping_amount)
            .ok_or(ProgramError::from(MarketError::AmountOverflow))?;
        p.quantity = p
            .quantity
            .checked_add(quantity)
            .ok_or(ProgramError::from(MarketError::AmountOverflow))?;
        p.deposited_at = now;
    }

    // Update pool totals.
    {
        let pool = Pool::from_account_mut(pool_ai)?;
        pool.product_total = pool
            .product_total
            .checked_add(product_amount)
            .ok_or(ProgramError::from(MarketError::AmountOverflow))?;
        pool.shipping_total = pool
            .shipping_total
            .checked_add(shipping_amount)
            .ok_or(ProgramError::from(MarketError::AmountOverflow))?;
        pool.refundable_outstanding = pool
            .refundable_outstanding
            .checked_add(total)
            .ok_or(ProgramError::from(MarketError::AmountOverflow))?;
        if is_new {
            pool.participant_count = pool
                .participant_count
                .checked_add(1)
                .ok_or(ProgramError::from(MarketError::AmountOverflow))?;
        }
        pool.total_quantity = pool
            .total_quantity
            .checked_add(quantity)
            .ok_or(ProgramError::from(MarketError::AmountOverflow))?;
        // Auto-lock the moment the cumulative units committed reach the
        // group-deal threshold. Triggers on any deposit (new buyer or
        // top-up) so a single big buyer can clear the threshold alone
        // — that's the natural wholesale group-buy semantic.
        if pool.total_quantity >= pool.threshold as u64 {
            pool.status = PoolStatus::Locked as u8;
        }
        pool.updated_at = now;
    }

    let _ = (mint_ai, system_program_ai, token_program_ai);
    Ok(())
}

// -------------------------------------------------------------------------
// LockPool
// -------------------------------------------------------------------------

fn lock_pool(program_id: &Address, accounts: &mut [AccountView]) -> ProgramResult {
    let [authority, pool_ai] = split_accounts(accounts, 2)?;
    ensure_authority(authority, pool_ai, program_id)?;

    let pool = Pool::from_account_mut(pool_ai)?;
    if pool.status()? != PoolStatus::Open {
        return Err(MarketError::PoolNotOpen.into());
    }
    pool.status = PoolStatus::Locked as u8;
    pool.updated_at = Clock::get()?.unix_timestamp;
    Ok(())
}

// -------------------------------------------------------------------------
// Release
// -------------------------------------------------------------------------

fn release(program_id: &Address, accounts: &mut [AccountView]) -> ProgramResult {
    let [authority, pool_ai, vault_ai, destination_ai, token_program_ai] =
        split_accounts(accounts, 5)?;
    ensure_authority(authority, pool_ai, program_id)?;

    // Snapshot pool fields (product_hash, vault_bump, batch_id, status)
    // without holding a borrow across the CPI.
    let (product_hash, vault_bump, vault_addr, batch_id_bytes) = {
        let pool = Pool::from_account(pool_ai)?;
        if pool.status()? != PoolStatus::Locked {
            return Err(MarketError::PoolNotLocked.into());
        }
        if &pool.vault_address() != vault_ai.address() {
            return Err(MarketError::InvalidVaultAccount.into());
        }
        (
            pool.product_hash,
            pool.vault_bump,
            pool.vault_address(),
            pool.batch_id.to_le_bytes(),
        )
    };

    // Validate vault PDA.
    assert_pda(
        &vault_addr,
        &[VAULT_SEED, &product_hash, &batch_id_bytes, &[vault_bump]],
        program_id,
        MarketError::InvalidVaultPda,
    )?;

    // Read the vault's current balance and send it all to the destination. The
    // vault's authority is the vault PDA itself — sign with its seeds.
    let amount = {
        let data = vault_ai.try_borrow()?;
        TokenAccount::from_bytes_unchecked_from_data(&data)?.amount()
    };

    if amount > 0 {
        Transfer {
            from: vault_ai,
            to: destination_ai,
            authority: vault_ai,
            multisig_signers: &[] as &[&AccountView],
            amount,
        }
        .invoke_signed(&[Signer::from(&[
            Seed::from(VAULT_SEED),
            Seed::from(&product_hash[..]),
            Seed::from(&batch_id_bytes[..]),
            Seed::from(&[vault_bump][..]),
        ])])?;
    }

    let pool = Pool::from_account_mut(pool_ai)?;
    pool.status = PoolStatus::Released as u8;
    pool.updated_at = Clock::get()?.unix_timestamp;

    let _ = token_program_ai;
    Ok(())
}

// -------------------------------------------------------------------------
// EnterRefundMode
// -------------------------------------------------------------------------

fn enter_refund_mode(program_id: &Address, accounts: &mut [AccountView]) -> ProgramResult {
    let [authority, pool_ai, vault_ai] = split_accounts(accounts, 3)?;
    ensure_authority(authority, pool_ai, program_id)?;

    let (refundable, vault_addr) = {
        let pool = Pool::from_account(pool_ai)?;
        let st = pool.status()?;
        if !matches!(
            st,
            PoolStatus::Open | PoolStatus::Locked | PoolStatus::Released
        ) {
            return Err(MarketError::PoolNotEligibleForRefund.into());
        }
        (pool.refundable_outstanding, pool.vault_address())
    };

    if &vault_addr != vault_ai.address() {
        return Err(MarketError::InvalidVaultAccount.into());
    }

    let vault_balance = {
        let data = vault_ai.try_borrow()?;
        TokenAccount::from_bytes_unchecked_from_data(&data)?.amount()
    };
    if vault_balance < refundable {
        return Err(MarketError::VaultUnderfunded.into());
    }

    let pool = Pool::from_account_mut(pool_ai)?;
    pool.status = PoolStatus::Refunding as u8;
    pool.updated_at = Clock::get()?.unix_timestamp;
    Ok(())
}

// -------------------------------------------------------------------------
// ClaimRefund
// -------------------------------------------------------------------------

fn claim_refund(program_id: &Address, accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let mut cursor = 0;
    let _user_id: [u8; 32] = read_array(data, &mut cursor)?;
    // user_id is used only to help the client locate the participation account;
    // we then verify the participation's own stored `user_id` below.

    let [authority, pool_ai, participation_ai, vault_ai, destination_ai, token_program_ai] =
        split_accounts(accounts, 6)?;
    ensure_authority(authority, pool_ai, program_id)?;

    let (product_hash, vault_bump, vault_addr, status, batch_id_bytes) = {
        let pool = Pool::from_account(pool_ai)?;
        (
            pool.product_hash,
            pool.vault_bump,
            pool.vault_address(),
            pool.status()?,
            pool.batch_id.to_le_bytes(),
        )
    };
    if status != PoolStatus::Refunding {
        return Err(MarketError::PoolNotRefunding.into());
    }
    if &vault_addr != vault_ai.address() {
        return Err(MarketError::InvalidVaultAccount.into());
    }

    // Validate the participation belongs to this pool and is unrefunded.
    let (refund_amount, participation_user_id) = {
        let p = Participation::from_account(participation_ai)?;
        if p.refunded != 0 {
            return Err(MarketError::ParticipationAlreadyRefunded.into());
        }
        if p.pool != *pool_ai.address().as_array() {
            return Err(MarketError::InvalidParticipationPda.into());
        }
        // Verify PDA using the stored bump + user_id + batch_id.
        assert_pda(
            participation_ai.address(),
            &[
                PARTICIPATION_SEED,
                &product_hash,
                &batch_id_bytes,
                &p.user_id,
                &[p.bump],
            ],
            program_id,
            MarketError::InvalidParticipationPda,
        )?;
        (p.total()?, p.user_id)
    };
    let _ = participation_user_id;

    // Vault PDA signs the SPL transfer to the destination.
    Transfer {
        from: vault_ai,
        to: destination_ai,
        authority: vault_ai,
        multisig_signers: &[] as &[&AccountView],
        amount: refund_amount,
    }
    .invoke_signed(&[Signer::from(&[
        Seed::from(VAULT_SEED),
        Seed::from(&product_hash[..]),
        Seed::from(&batch_id_bytes[..]),
        Seed::from(&[vault_bump][..]),
    ])])?;

    // Mark refunded & decrement outstanding.
    {
        let p = Participation::from_account_mut(participation_ai)?;
        p.refunded = 1;
    }
    {
        let pool = Pool::from_account_mut(pool_ai)?;
        pool.refundable_outstanding = pool
            .refundable_outstanding
            .checked_sub(refund_amount)
            .ok_or(ProgramError::from(MarketError::AmountOverflow))?;
        pool.updated_at = Clock::get()?.unix_timestamp;
    }

    let _ = token_program_ai;
    Ok(())
}

// -------------------------------------------------------------------------
// SelfRefund — buyer-initiated refund while the pool is still Open.
// -------------------------------------------------------------------------

fn self_refund(program_id: &Address, accounts: &mut [AccountView]) -> ProgramResult {
    let [buyer, authority, pool_ai, participation_ai, vault_ai, buyer_ata_ai, token_program_ai] =
        split_accounts(accounts, 7)?;

    if !buyer.is_signer() {
        return Err(MarketError::MissingBuyerSignature.into());
    }
    if !authority.is_signer() {
        // Authority co-signs. This isn't strictly required for safety
        // (the participation belongs to the buyer), but it lets the
        // backend gate the action on rules that aren't representable
        // on-chain — e.g. fraud holds.
        return Err(MarketError::MissingAuthoritySignature.into());
    }

    // Snapshot pool fields without holding a borrow across CPIs.
    let (product_hash, vault_bump, vault_addr, status, batch_id_bytes) = {
        let pool = Pool::from_account(pool_ai)?;
        (
            pool.product_hash,
            pool.vault_bump,
            pool.vault_address(),
            pool.status()?,
            pool.batch_id.to_le_bytes(),
        )
    };
    if status != PoolStatus::Open {
        // Once a pool auto-locks (threshold reached) or is otherwise out
        // of the recruiting phase, only the admin-mediated ClaimRefund
        // path is allowed.
        return Err(MarketError::PoolNotOpen.into());
    }
    if &vault_addr != vault_ai.address() {
        return Err(MarketError::InvalidVaultAccount.into());
    }
    assert_pda(
        &vault_addr,
        &[VAULT_SEED, &product_hash, &batch_id_bytes, &[vault_bump]],
        program_id,
        MarketError::InvalidVaultPda,
    )?;

    // Validate the participation: belongs to this buyer + this pool, not
    // already refunded.
    let (
        refund_amount,
        product_amount,
        shipping_amount,
        quantity,
        participation_bump,
        participation_user_id,
    ) = {
        let p = Participation::from_account(participation_ai)?;
        if p.refunded != 0 {
            return Err(MarketError::ParticipationAlreadyRefunded.into());
        }
        if p.pool != *pool_ai.address().as_array() {
            return Err(MarketError::InvalidParticipationPda.into());
        }
        if p.wallet != *buyer.address().as_array() {
            return Err(MarketError::ParticipationWalletMismatch.into());
        }
        (
            p.total()?,
            p.product_amount,
            p.shipping_amount,
            p.quantity,
            p.bump,
            p.user_id,
        )
    };
    assert_pda(
        participation_ai.address(),
        &[
            PARTICIPATION_SEED,
            &product_hash,
            &batch_id_bytes,
            &participation_user_id,
            &[participation_bump],
        ],
        program_id,
        MarketError::InvalidParticipationPda,
    )?;

    if refund_amount > 0 {
        Transfer {
            from: vault_ai,
            to: buyer_ata_ai,
            authority: vault_ai,
            multisig_signers: &[] as &[&AccountView],
            amount: refund_amount,
        }
        .invoke_signed(&[Signer::from(&[
            Seed::from(VAULT_SEED),
            Seed::from(&product_hash[..]),
            Seed::from(&batch_id_bytes[..]),
            Seed::from(&[vault_bump][..]),
        ])])?;
    }

    // Mark the participation refunded and roll back pool aggregates.
    {
        let p = Participation::from_account_mut(participation_ai)?;
        p.refunded = 1;
    }
    {
        let pool = Pool::from_account_mut(pool_ai)?;
        pool.refundable_outstanding = pool
            .refundable_outstanding
            .checked_sub(refund_amount)
            .ok_or(ProgramError::from(MarketError::AmountOverflow))?;
        pool.product_total = pool
            .product_total
            .checked_sub(product_amount)
            .ok_or(ProgramError::from(MarketError::AmountOverflow))?;
        pool.shipping_total = pool
            .shipping_total
            .checked_sub(shipping_amount)
            .ok_or(ProgramError::from(MarketError::AmountOverflow))?;
        // Refunded buyers don't count toward the threshold anymore —
        // both the participant tally and the unit total roll back.
        pool.participant_count = pool.participant_count.saturating_sub(1);
        pool.total_quantity = pool.total_quantity.saturating_sub(quantity);
        pool.updated_at = Clock::get()?.unix_timestamp;
    }

    let _ = token_program_ai;
    Ok(())
}

// -------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------

/// Slice `accounts` into a fixed-size array of mutable references. Fails if
/// the slice is shorter than `count`.
fn split_accounts<const N: usize>(
    accounts: &mut [AccountView],
    count: usize,
) -> Result<[&mut AccountView; N], ProgramError> {
    if accounts.len() < count {
        return Err(MarketError::InvalidAccountCount.into());
    }
    // SAFETY: we only use the first `N` entries, each pointing to a distinct
    // `AccountView` in the slice; the runtime guarantees they are independent.
    let ptr = accounts.as_mut_ptr();
    let mut out: [core::mem::MaybeUninit<&mut AccountView>; N] =
        unsafe { core::mem::MaybeUninit::uninit().assume_init() };
    for i in 0..N {
        out[i].write(unsafe { &mut *ptr.add(i) });
    }
    Ok(out.map(|slot| unsafe { slot.assume_init() }))
}

fn ensure_authority(
    authority: &AccountView,
    pool_ai: &AccountView,
    program_id: &Address,
) -> ProgramResult {
    if !authority.is_signer() {
        return Err(MarketError::MissingAuthoritySignature.into());
    }
    let pool = Pool::from_account(pool_ai)?;
    if &pool.authority_address() != authority.address() {
        return Err(MarketError::UnauthorizedAuthority.into());
    }
    let batch_id_bytes = pool.batch_id.to_le_bytes();
    // Re-derive the pool PDA to make sure it's ours.
    assert_pda(
        pool_ai.address(),
        &[POOL_SEED, &pool.product_hash, &batch_id_bytes, &[pool.bump]],
        program_id,
        MarketError::InvalidPoolPda,
    )?;
    Ok(())
}

#[inline]
fn pinocchio_rent_minimum_balance(len: usize) -> Result<u64, ProgramError> {
    pinocchio::sysvars::rent::Rent::get()?.try_minimum_balance(len)
}

// Small helper: `TokenAccount::from_bytes_unchecked` wants a full `&[u8]` of exactly
// TokenAccount::LEN. Provide a wrapper that asserts length and delegates.
trait TokenAccountFromData: Sized {
    fn from_bytes_unchecked_from_data(data: &[u8]) -> Result<&TokenAccount, ProgramError>;
}
impl TokenAccountFromData for TokenAccount {
    #[inline]
    fn from_bytes_unchecked_from_data(data: &[u8]) -> Result<&TokenAccount, ProgramError> {
        if data.len() != TokenAccount::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { TokenAccount::from_bytes_unchecked(data) })
    }
}
