//! Input Validation Utilities
//! 
//! This module contains utilities for validating user inputs, account states, and program parameters.
//! These functions provide common validation logic used throughout the program.

use borsh::BorshDeserialize;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    error::PoolError,
    state::SystemState,
    PoolState,
};

use crate::constants::*;

/// Validates that an account is owned by the expected program.
///
/// # Arguments
/// * `account` - The account to validate
/// * `expected_owner` - The expected owner program ID
///
/// # Returns
/// * `ProgramResult` - Success if ownership is correct, error otherwise
pub fn validate_account_owner(account: &AccountInfo, expected_owner: &Pubkey) -> ProgramResult {
    if account.owner != expected_owner {
        msg!("Account {} has incorrect owner. Expected: {}, Actual: {}", 
             account.key, expected_owner, account.owner);
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Validates that an account is a signer.
///
/// # Arguments
/// * `account` - The account to validate
/// * `context` - Context string for error messages
///
/// # Returns
/// * `ProgramResult` - Success if account is signer, error otherwise
pub fn validate_signer(account: &AccountInfo, context: &str) -> ProgramResult {
    if !account.is_signer {
        msg!("{} must be a signer", context);
        return Err(ProgramError::MissingRequiredSignature);
    }
    Ok(())
}

/// Validates that an account is writable.
///
/// # Arguments
/// * `account` - The account to validate
/// * `context` - Context string for error messages
///
/// # Returns
/// * `ProgramResult` - Success if account is writable, error otherwise
pub fn validate_writable(account: &AccountInfo, context: &str) -> ProgramResult {
    if !account.is_writable {
        msg!("{} must be writable", context);
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Validates swap fee basis points are within allowed range.
///
/// # Arguments
/// * `fee_basis_points` - The fee in basis points to validate
///
/// # Returns
/// * `ProgramResult` - Success if fee is valid, error otherwise
pub fn validate_swap_fee(fee_basis_points: u16) -> ProgramResult {
    if u64::from(fee_basis_points) > MAX_SWAP_FEE_BASIS_POINTS {
        msg!("Swap fee {} basis points exceeds maximum of {}", 
             fee_basis_points, MAX_SWAP_FEE_BASIS_POINTS);
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Validates that a token amount is non-zero.
///
/// # Arguments
/// * `amount` - The amount to validate
/// * `context` - Context string for error messages
///
/// # Returns
/// * `ProgramResult` - Success if amount is valid, error otherwise
pub fn validate_non_zero_amount(amount: u64, context: &str) -> ProgramResult {
    if amount == 0 {
        msg!("{} amount cannot be zero", context);
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Validates that two token mints are different (prevents same-token pools).
///
/// # Arguments
/// * `token_a` - First token mint
/// * `token_b` - Second token mint
///
/// # Returns
/// * `ProgramResult` - Success if tokens are different, error otherwise
pub fn validate_different_tokens(token_a: &Pubkey, token_b: &Pubkey) -> ProgramResult {
    if token_a == token_b {
        msg!("Cannot create pool with identical tokens: {}", token_a);
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Validates that a pool state is properly initialized.
///
/// # Arguments
/// * `pool_state` - The pool state to validate
///
/// # Returns
/// * `ProgramResult` - Success if pool is initialized, error otherwise
pub fn validate_pool_initialized(pool_state: &PoolState) -> ProgramResult {
    if !pool_state.is_initialized {
        msg!("Pool is not yet initialized");
        return Err(ProgramError::UninitializedAccount);
    }
    Ok(())
}

/// Validates that a pool is not paused (for user operations).
/// 
/// **NEW PAUSE SYSTEM**: Simple pool-level pause validation without auto-unpause.
/// Pool pause persists indefinitely until manually unpaused by owner action.
///
/// # Arguments
/// * `pool_state` - The pool state to validate
/// * `_current_timestamp` - Timestamp (unused in new system, kept for compatibility)
///
/// # Returns
/// * `ProgramResult` - Success if pool is not paused, error if paused
pub fn validate_pool_not_paused(pool_state: &mut PoolState, _current_timestamp: i64) -> ProgramResult {
    if pool_state.paused {
        msg!("Pool operations are currently paused (indefinite until manual unpause)");
        msg!("Use owner action to unpause pool operations");
        return Err(PoolError::PoolPaused.into());
    }
    Ok(())
}

/// Validates that the system is not paused for user operations.
/// This check takes precedence over pool-specific pause checks.
///
/// # Arguments
/// * `system_state_account` - The system state account to check
///
/// # Returns
/// * `ProgramResult` - Success if system is not paused, error if paused
pub fn validate_system_not_paused(system_state_account: &AccountInfo) -> ProgramResult {
    // Deserialize system state
    let system_state = SystemState::try_from_slice(&system_state_account.data.borrow())?;
    
    if system_state.is_paused {
        msg!("🛑 SYSTEM PAUSED: All operations blocked (overrides pool pause state)");
        msg!("Pause code: {}", system_state.pause_reason_code);
        msg!("Paused at: {}", system_state.pause_timestamp);
        msg!("Only system unpause is allowed");
        return Err(PoolError::SystemPaused.into());
    }
    
    Ok(())
}

/// Validates ratio values and returns pool ID string for PDA derivation.
///
/// # Arguments
/// * `ratio_a_numerator` - Token A base units
/// * `ratio_b_denominator` - Token B base units
///
/// # Returns
/// * `ProgramResult` - Success if ratios are valid, error otherwise
pub fn validate_ratio_values(ratio_a_numerator: u64, ratio_b_denominator: u64) -> ProgramResult {
    if ratio_a_numerator == 0 {
        msg!("Ratio A numerator cannot be zero");
        return Err(ProgramError::InvalidArgument);
    }
    
    if ratio_b_denominator == 0 {
        msg!("Ratio B denominator cannot be zero");
        return Err(ProgramError::InvalidArgument);
    }
    
    Ok(())
}

/// Determines if a pool has a clean one-to-many ratio based on the provided ratios and token decimals.
/// 
/// A pool is considered one-to-many if:
/// - Both ratios represent whole numbers (no fractional parts when converted to display units)
/// - One of the tokens has exactly 1.0 ratio in display units
/// - Both ratios are positive (greater than zero)
///
/// # Arguments
/// * `ratio_a_numerator` - Token A base units
/// * `ratio_b_denominator` - Token B base units
/// * `token_a_decimals` - Number of decimal places for token A
/// * `token_b_decimals` - Number of decimal places for token B
///
/// # Returns
/// * `bool` - true if the pool qualifies as one-to-many, false otherwise
///
/// # Examples
/// ```
/// use fixed_ratio_trading::utils::validation::check_one_to_many_ratio;
/// 
/// // 1 SOL = 2 USDC (SOL: 9 decimals, USDC: 6 decimals)
/// let is_one_to_many = check_one_to_many_ratio(
///     1_000_000_000,  // 1.0 SOL in base units
///     2_000_000,      // 2.0 USDC in base units
///     9,              // SOL decimals
///     6               // USDC decimals
/// ); // Returns true
/// assert!(is_one_to_many);
/// 
/// // 1 BTC = 1.01 USDT (BTC: 8 decimals, USDT: 6 decimals)
/// let is_one_to_many = check_one_to_many_ratio(
///     100_000_000,    // 1.0 BTC in base units
///     1_010_000,      // 1.01 USDT in base units
///     8,              // BTC decimals
///     6               // USDT decimals
/// ); // Returns false (1.01 is not a whole number)
/// assert!(!is_one_to_many);
/// ```
pub fn check_one_to_many_ratio(
    ratio_a_numerator: u64,
    ratio_b_denominator: u64, 
    token_a_decimals: u8,
    token_b_decimals: u8
) -> bool {
    let token_a_decimal_factor = 10_u64.pow(token_a_decimals as u32);
    let token_b_decimal_factor = 10_u64.pow(token_b_decimals as u32);
    
    // Check if both ratios represent whole numbers (no fractional parts)
    let a_is_whole = (ratio_a_numerator % token_a_decimal_factor) == 0;
    let b_is_whole = (ratio_b_denominator % token_b_decimal_factor) == 0;
    
    // Convert to display units
    let display_ratio_a = ratio_a_numerator / token_a_decimal_factor;
    let display_ratio_b = ratio_b_denominator / token_b_decimal_factor;
    
    // Check if both are greater than zero, whole numbers, and one equals exactly 1
    let both_positive = display_ratio_a > 0 && display_ratio_b > 0;
    let one_equals_one = display_ratio_a == 1 || display_ratio_b == 1;
    
    a_is_whole && b_is_whole && both_positive && one_equals_one
} 