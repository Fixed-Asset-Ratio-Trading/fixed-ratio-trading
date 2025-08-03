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
    
    /// Pool operations are currently paused
    PoolPaused,
    
    /// Pool swaps are currently paused by owner
    PoolSwapsPaused,
    
    /// Pool swaps are already paused
    PoolSwapsAlreadyPaused,
    
    /// Pool swaps are not currently paused
    PoolSwapsNotPaused,
    
    /// System is paused - all operations blocked except unpause
    SystemPaused,
    
    /// System is already paused
    SystemAlreadyPaused,
    
    /// System is not paused
    SystemNotPaused,
    
    /// Unauthorized access to system controls
    UnauthorizedAccess,
    
    /// Arithmetic overflow
    ArithmeticOverflow,
    
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
            PoolError::PoolPaused => write!(f, "Pool operations are currently paused"),
            PoolError::PoolSwapsPaused => write!(f, "Pool swaps are currently paused by owner"),
            PoolError::PoolSwapsAlreadyPaused => write!(f, "Pool swaps are already paused"),
            PoolError::PoolSwapsNotPaused => write!(f, "Pool swaps are not currently paused"),
            PoolError::SystemPaused => write!(f, "System is paused - all operations blocked except unpause"),
            PoolError::SystemAlreadyPaused => write!(f, "System is already paused"),
            PoolError::SystemNotPaused => write!(f, "System is not paused"),
            PoolError::UnauthorizedAccess => write!(f, "Unauthorized access to system controls"),
            PoolError::ArithmeticOverflow => write!(f, "Arithmetic overflow"),
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
            PoolError::PoolPaused => 1007,
            PoolError::PoolSwapsPaused => 1027,
            PoolError::PoolSwapsAlreadyPaused => 1029,
            PoolError::PoolSwapsNotPaused => 1030,
            PoolError::SystemPaused => 1023,
            PoolError::SystemAlreadyPaused => 1024,
            PoolError::SystemNotPaused => 1025,
            PoolError::UnauthorizedAccess => 1026,
            PoolError::ArithmeticOverflow => 1019,
            PoolError::Unauthorized => 1012,
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