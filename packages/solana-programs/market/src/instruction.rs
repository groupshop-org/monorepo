use pinocchio::error::ProgramError;

use crate::errors::MarketError;

/// Tag byte at offset 0 of each instruction's data buffer.
#[repr(u8)]
pub enum MarketIx {
    /// Data: `[u8; 32] product_hash, u8 pool_bump, u8 vault_bump`
    /// Accounts: authority, pool, vault, usdc_mint, system_program, token_program
    InitializePool = 0,

    /// Data: `[u8; 32] user_id, u8 participation_bump, u64 product_amount, u64 shipping_amount`
    /// Accounts: buyer, authority, pool, participation, vault, buyer_usdc_ata, usdc_mint,
    ///           system_program, token_program
    Deposit = 1,

    /// Data: empty
    /// Accounts: authority, pool
    LockPool = 2,

    /// Data: empty
    /// Accounts: authority, pool, vault, destination_ata, token_program
    Release = 3,

    /// Data: empty
    /// Accounts: authority, pool, vault
    EnterRefundMode = 4,

    /// Data: `[u8; 32] user_id`
    /// Accounts: authority, pool, participation, vault, destination_ata, token_program
    ClaimRefund = 5,
}

impl MarketIx {
    #[inline]
    pub fn from_tag(tag: u8) -> Result<Self, ProgramError> {
        match tag {
            0 => Ok(Self::InitializePool),
            1 => Ok(Self::Deposit),
            2 => Ok(Self::LockPool),
            3 => Ok(Self::Release),
            4 => Ok(Self::EnterRefundMode),
            5 => Ok(Self::ClaimRefund),
            _ => Err(MarketError::InvalidInstruction.into()),
        }
    }
}

/// Read a fixed-size slice `[A; N]` starting at offset `*cursor`, advance cursor.
#[inline]
pub fn read_array<const N: usize>(
    data: &[u8],
    cursor: &mut usize,
) -> Result<[u8; N], ProgramError> {
    let end = cursor
        .checked_add(N)
        .ok_or(ProgramError::from(MarketError::InvalidInstructionData))?;
    if end > data.len() {
        return Err(MarketError::InvalidInstructionData.into());
    }
    let mut out = [0u8; N];
    out.copy_from_slice(&data[*cursor..end]);
    *cursor = end;
    Ok(out)
}

#[inline]
pub fn read_u8(data: &[u8], cursor: &mut usize) -> Result<u8, ProgramError> {
    let arr: [u8; 1] = read_array(data, cursor)?;
    Ok(arr[0])
}

#[inline]
pub fn read_u64(data: &[u8], cursor: &mut usize) -> Result<u64, ProgramError> {
    let arr: [u8; 8] = read_array(data, cursor)?;
    Ok(u64::from_le_bytes(arr))
}
