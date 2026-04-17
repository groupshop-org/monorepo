use pinocchio::{error::ProgramError, Address};

use crate::errors::MarketError;

pub const POOL_SEED: &[u8] = b"pool";
pub const VAULT_SEED: &[u8] = b"vault";
pub const PARTICIPATION_SEED: &[u8] = b"part";

/// Verify that `actual` matches `create_program_address(seeds, program_id)`.
/// Returns `err` on mismatch.
#[inline]
pub fn assert_pda(
    actual: &Address,
    seeds: &[&[u8]],
    program_id: &Address,
    err: MarketError,
) -> Result<(), ProgramError> {
    let derived =
        Address::create_program_address(seeds, program_id).map_err(|_| ProgramError::from(err))?;
    if derived != *actual {
        return Err(err.into());
    }
    Ok(())
}
