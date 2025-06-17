//! Pool Error Types
//! 
//! This module contains all the error definitions for the Solana Trading Pool Program.
//! Error types provide structured error handling and user-friendly error messages.

use solana_program::{program_error::ProgramError, pubkey::Pubkey};
use std::fmt;

/// Comprehensive error types for the Solana Trading Pool Program.
/// 
/// This enum defines all possible error conditions that can occur during
/// pool operations, providing structured error handling with detailed
/// context information for debugging and user feedback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolError {
    /// Invalid token pair configuration
    InvalidTokenPair {
        token_a: Pubkey,
        token_b: Pubkey,
        reason: String,
    },
    
    /// Invalid ratio configuration
    InvalidRatio {
        ratio: u64,
        min_ratio: u64,
        max_ratio: u64,
    },
    
    /// Insufficient funds for the operation
    InsufficientFunds {
        required: u64,
        available: u64,
        account: Pubkey,
    },
    
    /// Invalid token account state or configuration
    InvalidTokenAccount {
        account: Pubkey,
        reason: String,
    },
    
    /// Invalid swap amount (outside allowed bounds)
    InvalidSwapAmount {
        amount: u64,
        min_amount: u64,
        max_amount: u64,
    },
    
    /// Rent exemption error
    RentExemptError {
        account: Pubkey,
        required: u64,
        available: u64,
    },
    
    /// Withdrawal amount exceeds maximum allowed percentage
    WithdrawalTooLarge,
    
    /// Pool operations are currently paused
    PoolPaused,
    
    /// Maximum number of delegates exceeded
    DelegateLimitExceeded,
    
    /// Delegate already exists in the pool
    DelegateAlreadyExists { delegate: Pubkey },
    
    /// Delegate not found in the pool
    DelegateNotFound { delegate: Pubkey },
    
    /// Invalid wait time specified
    InvalidWaitTime { wait_time: u64 },
    
    /// A pending withdrawal request already exists
    PendingWithdrawalExists,
    
    /// No pending withdrawal request found
    NoPendingWithdrawal,
    
    /// Unauthorized delegate operation
    UnauthorizedDelegate,
    
    /// Insufficient fees for the operation
    InsufficientFees,
    
    /// Invalid withdrawal request
    InvalidWithdrawalRequest,
    
    /// Withdrawal request is not ready yet
    WithdrawalNotReady,
    
    /// Unauthorized operation
    Unauthorized,
}

impl fmt::Display for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PoolError::InvalidTokenPair { token_a, token_b, reason } => {
                write!(f, "Invalid token pair: {} and {}. Reason: {}", token_a, token_b, reason)
            },
            PoolError::InvalidRatio { ratio, min_ratio, max_ratio } => {
                write!(f, "Invalid ratio: {}. Must be between {} and {}", ratio, min_ratio, max_ratio)
            },
            PoolError::InsufficientFunds { required, available, account } => {
                write!(f, "Insufficient funds: Required {}, Available {}, Account {}", required, available, account)
            },
            PoolError::InvalidTokenAccount { account, reason } => {
                write!(f, "Invalid token account: Account {}, Reason: {}", account, reason)
            },
            PoolError::InvalidSwapAmount { amount, min_amount, max_amount } => {
                write!(f, "Invalid swap amount: {} is not between {} and {}", amount, min_amount, max_amount)
            },
            PoolError::RentExemptError { account, required, available } => {
                write!(f, "Insufficient funds: Required {}, Available {}, Account {}", required, available, account)
            },
            PoolError::WithdrawalTooLarge => write!(f, "Withdrawal amount exceeds maximum allowed percentage"),
            PoolError::PoolPaused => write!(f, "Pool operations are currently paused"),
            PoolError::DelegateLimitExceeded => write!(f, "Delegate limit exceeded"),
            PoolError::DelegateAlreadyExists { delegate } => write!(f, "Delegate already exists: {}", delegate),
            PoolError::DelegateNotFound { delegate } => write!(f, "Delegate not found: {}", delegate),
            PoolError::InvalidWaitTime { wait_time } => write!(f, "Invalid wait time: {} seconds", wait_time),
            PoolError::PendingWithdrawalExists => write!(f, "Pending withdrawal request exists"),
            PoolError::NoPendingWithdrawal => write!(f, "No pending withdrawal request"),
            PoolError::UnauthorizedDelegate => write!(f, "Unauthorized delegate"),
            PoolError::InsufficientFees => write!(f, "Insufficient fees"),
            PoolError::InvalidWithdrawalRequest => write!(f, "Invalid withdrawal request"),
            PoolError::WithdrawalNotReady => write!(f, "Withdrawal not ready"),
            PoolError::Unauthorized => write!(f, "Unauthorized"),
        }
    }
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
            PoolError::PoolPaused => 1008,
            PoolError::DelegateLimitExceeded => 1009,
            PoolError::DelegateAlreadyExists { .. } => 1010,
            PoolError::DelegateNotFound { .. } => 1011,
            PoolError::InvalidWaitTime { .. } => 1012,
            PoolError::PendingWithdrawalExists => 1013,
            PoolError::NoPendingWithdrawal => 1014,
            PoolError::UnauthorizedDelegate => 1015,
            PoolError::InsufficientFees => 1016,
            PoolError::InvalidWithdrawalRequest => 1017,
            PoolError::WithdrawalNotReady => 1018,
            PoolError::Unauthorized => 1019,
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