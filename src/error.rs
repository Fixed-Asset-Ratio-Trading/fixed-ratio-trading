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
    
    /// Withdrawal amount exceeds maximum allowed percentage
    #[error("Withdrawal amount exceeds maximum allowed percentage")]
    WithdrawalTooLarge,
    
    /// Withdrawal is currently in cooldown period
    #[error("Withdrawal is currently in cooldown period")]
    WithdrawalCooldown,
    
    /// Pool operations are currently paused
    #[error("Pool operations are currently paused")]
    PoolPaused,
    
    /// Maximum number of delegates exceeded
    #[error("Delegate limit exceeded")]
    DelegateLimitExceeded,
    
    /// Delegate already exists in the pool
    #[error("Delegate already exists: {delegate}")]
    DelegateAlreadyExists { delegate: Pubkey },
    
    /// Delegate not found in the pool
    #[error("Delegate not found: {delegate}")]
    DelegateNotFound { delegate: Pubkey },
    
    /// Invalid wait time specified
    #[error("Invalid wait time: {wait_time} seconds")]
    InvalidWaitTime { wait_time: u64 },
    
    /// Unauthorized operation
    #[error("Unauthorized")]
    Unauthorized,
    
    /// Unauthorized delegate operation
    #[error("Unauthorized delegate")]
    UnauthorizedDelegate,
    
    /// Invalid action parameters
    #[error("Invalid action parameters")]
    InvalidActionParameters,
    
    /// Invalid action type
    #[error("Invalid action type")]
    InvalidActionType,
    
    /// Action not ready for execution
    #[error("Action not ready for execution")]
    ActionNotReady,
    
    /// Action not found
    #[error("Action not found")]
    ActionNotFound,
    
    /// Too many pending actions
    #[error("Too many pending actions")]
    TooManyPendingActions,
    
    /// Arithmetic overflow
    #[error("Arithmetic overflow")]
    ArithmeticOverflow,
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
            PoolError::WithdrawalTooLarge => 1007,
            PoolError::WithdrawalCooldown => 1008,
            PoolError::PoolPaused => 1009,
            PoolError::DelegateLimitExceeded => 1010,
            PoolError::DelegateAlreadyExists { .. } => 1011,
            PoolError::DelegateNotFound { .. } => 1012,
            PoolError::InvalidWaitTime { .. } => 1013,
            PoolError::Unauthorized => 1014,
            PoolError::UnauthorizedDelegate => 1015,
            PoolError::InvalidActionParameters => 1016,
            PoolError::InvalidActionType => 1017,
            PoolError::ActionNotReady => 1018,
            PoolError::ActionNotFound => 1019,
            PoolError::TooManyPendingActions => 1020,
            PoolError::ArithmeticOverflow => 1021,
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