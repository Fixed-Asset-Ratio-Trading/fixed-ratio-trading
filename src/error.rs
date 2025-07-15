use solana_program::{
    program_error::ProgramError,
    pubkey::Pubkey,
};
use thiserror::Error;

/// Comprehensive error types for the Solana Trading Pool Program.
/// 
/// This enum defines all possible error conditions that can occur during
/// pool operations, providing structured error handling with detailed
/// context information for debugging and user feedback.
#[derive(Error, Debug, Clone)]
pub enum PoolError {
    /// Invalid token pair configuration
    #[error("Invalid token pair: {token_a} and {token_b}. Reason: {reason}")]
    InvalidTokenPair {
        token_a: Pubkey,
        token_b: Pubkey,
        reason: String,
    },
    
    /// Invalid ratio configuration
    #[error("Invalid ratio: {ratio}. Must be between {min_ratio} and {max_ratio}")]
    InvalidRatio {
        ratio: u64,
        min_ratio: u64,
        max_ratio: u64,
    },
    
    /// Insufficient funds for the operation
    #[error("Insufficient funds: Required {required}, Available {available}, Account {account}")]
    InsufficientFunds {
        required: u64,
        available: u64,
        account: Pubkey,
    },
    
    /// Invalid token account state or configuration
    #[error("Invalid token account: Account {account}. Reason: {reason}")]
    InvalidTokenAccount {
        account: Pubkey,
        reason: String,
    },
    
    /// Invalid swap amount (outside allowed bounds)
    #[error("Invalid swap amount: {amount} is not between {min_amount} and {max_amount}")]
    InvalidSwapAmount {
        amount: u64,
        min_amount: u64,
        max_amount: u64,
    },
    
    /// Rent exemption error
    #[error("Insufficient funds: Required {required}, Available {available}, Account {account}")]
    RentExemptError {
        account: Pubkey,
        required: u64,
        available: u64,
    },
    
    /// Pool operations are currently paused
    #[error("Pool is paused")]
    PoolPaused,
    
    /// Unauthorized operation
    #[error("Unauthorized")]
    Unauthorized,
    
    /// Arithmetic overflow
    #[error("Arithmetic overflow")]
    ArithmeticOverflow,
    
    /// System is paused - all operations blocked except unpause
    #[error("System is paused - all operations blocked except unpause")]
    SystemPaused,
    
    /// System is already paused
    #[error("System is already paused")]
    SystemAlreadyPaused,
    
    /// System is not paused
    #[error("System is not paused")]
    SystemNotPaused,
    
    /// Unauthorized access to system controls
    #[error("Unauthorized access to system controls")]
    UnauthorizedAccess,
    
    /// Pool swaps are currently paused by owner
    #[error("Pool swaps are currently paused by owner")]
    PoolSwapsPaused,
    
    /// Pool swaps are already paused
    #[error("Pool swaps are already paused")]
    PoolSwapsAlreadyPaused,
    
    /// Pool swaps are not currently paused
    #[error("Pool swaps are not currently paused")]
    PoolSwapsNotPaused,
    
    /// Insufficient balance for fee payment
    #[error("Insufficient balance for fee payment: Required {required} lamports, Available {available} lamports, Account {account}")]
    InsufficientFeeBalance {
        required: u64,
        available: u64,
        account: Pubkey,
    },
    
    /// Fee collection failed during transfer
    #[error("Fee collection failed: Expected {expected} lamports, Collected {collected} lamports, Fee type: {fee_type}")]
    FeeCollectionFailed {
        expected: u64,
        collected: u64,
        fee_type: String,
    },
    
    /// Fee validation failed during pre-flight checks
    #[error("Fee validation failed: {reason}")]
    FeeValidationFailed {
        reason: String,
    },
    
    /// Treasury account validation failed
    #[error("Treasury account validation failed: Expected {expected}, Provided {provided}, Treasury type: {treasury_type}")]
    TreasuryValidationFailed {
        expected: Pubkey,
        provided: Pubkey,
        treasury_type: String,
    },
    
    /// Invalid SystemState PDA provided
    #[error("Invalid SystemState PDA provided")]
    InvalidSystemStatePDA,
    
    /// SystemState deserialization failed
    #[error("SystemState deserialization failed")]
    InvalidSystemStateDeserialization,
    
    /// **NEW: Consolidation-related errors**
    #[error("Consolidation failed: {reason}")]
    ConsolidationFailed { reason: String },
    
    #[error("Invalid consolidation batch: expected {expected} pools, got {actual}")]
    InvalidConsolidationBatch { expected: u8, actual: u8 },
    
    #[error("Pool not eligible for consolidation: {reason}")]
    PoolNotEligibleForConsolidation { reason: String },
    
    #[error("Consolidation race condition detected")]
    ConsolidationRaceCondition,
    
    #[error("No pools eligible for consolidation")]
    NoPoolsEligibleForConsolidation,
}

impl PoolError {
    /// Returns a unique error code for each error variant.
    /// 
    /// Error codes are used for programmatic error handling and
    /// provide a stable interface for client applications.
    pub fn error_code(&self) -> u32 {
        match self {
            PoolError::InvalidTokenPair { .. } => 1001,
            PoolError::InvalidRatio { .. } => 1002,
            PoolError::InsufficientFunds { .. } => 1003,
            PoolError::InvalidTokenAccount { .. } => 1004,
            PoolError::InvalidSwapAmount { .. } => 1005,
            PoolError::RentExemptError { .. } => 1006,
            PoolError::PoolPaused => 1007,
            PoolError::Unauthorized => 1012,
            PoolError::ArithmeticOverflow => 1019,
            PoolError::SystemPaused => 1023,
            PoolError::SystemAlreadyPaused => 1024,
            PoolError::SystemNotPaused => 1025,
            PoolError::UnauthorizedAccess => 1026,
            PoolError::PoolSwapsPaused => 1027,
            PoolError::PoolSwapsAlreadyPaused => 1028,
            PoolError::PoolSwapsNotPaused => 1029,
            PoolError::InsufficientFeeBalance { .. } => 1030,
            PoolError::FeeCollectionFailed { .. } => 1031,
            PoolError::FeeValidationFailed { .. } => 1032,
            PoolError::TreasuryValidationFailed { .. } => 1033,
            PoolError::InvalidSystemStatePDA => 1034,
            PoolError::InvalidSystemStateDeserialization => 1035,
            PoolError::ConsolidationFailed { .. } => 1036,
            PoolError::InvalidConsolidationBatch { .. } => 1037,
            PoolError::PoolNotEligibleForConsolidation { .. } => 1038,
            PoolError::ConsolidationRaceCondition => 1039,
            PoolError::NoPoolsEligibleForConsolidation => 1040,
        }
    }
}

impl From<PoolError> for ProgramError {
    /// Converts a PoolError into a ProgramError for Solana program compatibility.
    /// 
    /// This enables seamless integration with Solana's error handling system
    /// while preserving detailed error information through custom error codes.
    fn from(e: PoolError) -> Self {
        ProgramError::Custom(e.error_code())
    }
} 