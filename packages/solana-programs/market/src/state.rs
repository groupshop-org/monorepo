use pinocchio::{error::ProgramError, AccountView, Address};

use crate::errors::MarketError;

pub const STATE_VERSION: u8 = 1;

/// Lifecycle state of a product pool.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PoolStatus {
    Open = 0,
    Locked = 1,
    Released = 2,
    Refunding = 3,
}

impl PoolStatus {
    #[inline]
    pub fn from_u8(v: u8) -> Result<Self, ProgramError> {
        match v {
            0 => Ok(PoolStatus::Open),
            1 => Ok(PoolStatus::Locked),
            2 => Ok(PoolStatus::Released),
            3 => Ok(PoolStatus::Refunding),
            _ => Err(MarketError::InvalidInstructionData.into()),
        }
    }
}

/// On-chain Pool account. `#[repr(C)]` with explicit padding so layout is
/// deterministic and matches `Pool::LEN`.
///
/// A given `(product_hash, batch_id)` pair has at most one Pool. When a
/// pool's `participant_count` crosses `threshold` the program auto-locks
/// it; the backend then opens the next batch (`batch_id + 1`) so the
/// product keeps recycling without admin intervention.
#[repr(C)]
pub struct Pool {
    pub version: u8,
    pub bump: u8,
    pub vault_bump: u8,
    pub status: u8,
    pub _pad: [u8; 4],
    pub authority: [u8; 32],
    pub product_hash: [u8; 32],
    pub usdc_mint: [u8; 32],
    pub vault: [u8; 32],
    pub product_total: u64,
    pub shipping_total: u64,
    pub refundable_outstanding: u64,
    pub participant_count: u32,
    /// Group-deal threshold, measured in **units of the product**. Set at
    /// init time; the program auto-locks when `total_quantity` reaches
    /// this value. (Was: participant count. Renamed-in-place rather than
    /// adding a new field — the on-chain layout is still ABI-compatible
    /// with the wider Pool struct, just with different semantics.)
    pub threshold: u32,
    pub created_at: i64,
    pub updated_at: i64,
    /// Sequential batch number for this product. Bumped by the backend
    /// when it opens a successor pool after the previous one auto-locks.
    pub batch_id: u32,
    pub _pad3: [u8; 4],
    /// Sum of every active participation's `quantity`. Refunds
    /// decrement this. Compared against `threshold` to decide auto-lock.
    pub total_quantity: u64,
}

impl Pool {
    pub const LEN: usize = core::mem::size_of::<Pool>();

    #[inline]
    pub fn from_account(account: &AccountView) -> Result<&Self, ProgramError> {
        if account.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        // SAFETY: length matches and `Pool` is `repr(C)` with all-plain-data fields;
        // `AccountView`'s data region is 8-byte aligned.
        Ok(unsafe { &*(account.data_ptr() as *const Pool) })
    }

    #[inline]
    pub fn from_account_mut(account: &mut AccountView) -> Result<&mut Self, ProgramError> {
        if account.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(account.data_mut_ptr() as *mut Pool) })
    }

    #[inline]
    pub fn status(&self) -> Result<PoolStatus, ProgramError> {
        PoolStatus::from_u8(self.status)
    }

    #[inline]
    pub fn authority_address(&self) -> Address {
        Address::new_from_array(self.authority)
    }

    #[inline]
    pub fn usdc_mint_address(&self) -> Address {
        Address::new_from_array(self.usdc_mint)
    }

    #[inline]
    pub fn vault_address(&self) -> Address {
        Address::new_from_array(self.vault)
    }
}

/// On-chain Participation account.
#[repr(C)]
pub struct Participation {
    pub version: u8,
    pub bump: u8,
    pub refunded: u8,
    pub _pad0: u8,
    pub _pad1: [u8; 4],
    pub pool: [u8; 32],
    pub user_id: [u8; 32],
    pub wallet: [u8; 32],
    pub product_amount: u64,
    pub shipping_amount: u64,
    pub deposited_at: i64,
    /// Number of product units the buyer committed across all their
    /// deposits to this batch. The on-chain `Pool.total_quantity`
    /// invariant is the sum of this field across non-refunded
    /// participations.
    pub quantity: u64,
}

impl Participation {
    pub const LEN: usize = core::mem::size_of::<Participation>();

    #[inline]
    pub fn from_account(account: &AccountView) -> Result<&Self, ProgramError> {
        if account.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &*(account.data_ptr() as *const Participation) })
    }

    #[inline]
    pub fn from_account_mut(account: &mut AccountView) -> Result<&mut Self, ProgramError> {
        if account.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(account.data_mut_ptr() as *mut Participation) })
    }

    #[inline]
    pub fn total(&self) -> Result<u64, ProgramError> {
        self.product_amount
            .checked_add(self.shipping_amount)
            .ok_or_else(|| MarketError::AmountOverflow.into())
    }
}
