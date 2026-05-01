use pinocchio::error::ProgramError;

/// Custom error codes for the market program. Converted into
/// `ProgramError::Custom(code)` on the way out.
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum MarketError {
    InvalidInstruction = 1,
    InvalidInstructionData = 2,
    InvalidAccountCount = 3,
    UnauthorizedAuthority = 4,
    MissingBuyerSignature = 5,
    MissingAuthoritySignature = 6,
    InvalidPoolPda = 7,
    InvalidVaultPda = 8,
    InvalidParticipationPda = 9,
    InvalidMint = 10,
    InvalidVaultAccount = 11,
    InvalidDestination = 12,
    PoolAlreadyInitialized = 13,
    PoolNotOpen = 14,
    PoolNotLocked = 15,
    PoolNotEligibleForRefund = 16,
    PoolNotRefunding = 17,
    ParticipationWalletMismatch = 18,
    ParticipationAlreadyRefunded = 19,
    VaultUnderfunded = 20,
    AmountOverflow = 21,
    ZeroDepositAmount = 22,
    InvalidThreshold = 23,
}

impl From<MarketError> for ProgramError {
    fn from(value: MarketError) -> Self {
        ProgramError::Custom(value as u32)
    }
}
