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
    types::{PoolState, pool_state::PoolPauseReason},
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

/// Validates delegate wait time is within allowed bounds.
///
/// # Arguments
/// * `wait_time` - The wait time in seconds to validate
///
/// # Returns
/// * `ProgramResult` - Success if wait time is valid, error otherwise
pub fn validate_wait_time(wait_time: u64) -> ProgramResult {
    if wait_time < MIN_WITHDRAWAL_WAIT_TIME || wait_time > MAX_WITHDRAWAL_WAIT_TIME {
        msg!("Wait time {} seconds is outside allowed range [{}, {}]", 
             wait_time, MIN_WITHDRAWAL_WAIT_TIME, MAX_WITHDRAWAL_WAIT_TIME);
        return Err(PoolError::InvalidWaitTime { wait_time }.into());
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
/// Also handles automatic unpause if the pause duration has elapsed.
///
/// # Arguments
/// * `pool_state` - The pool state to validate
/// * `current_timestamp` - Current Unix timestamp
///
/// # Returns
/// * `ProgramResult` - Success if pool is not paused or pause has elapsed
pub fn validate_pool_not_paused(pool_state: &mut PoolState, current_timestamp: i64) -> ProgramResult {
    // Check if pause has elapsed and handle automatic unpause
    if pool_state.is_paused && pool_state.pause_end_timestamp > 0 && current_timestamp >= pool_state.pause_end_timestamp {
        pool_state.is_paused = false;
        pool_state.pause_end_timestamp = 0;
        pool_state.pause_reason = PoolPauseReason::default();
        msg!("Pool automatically unpaused as pause duration has elapsed");
        return Ok(());
    }

    if pool_state.is_paused {
        msg!("Pool operations are currently paused");
        msg!("Pause reason: {:?}", pool_state.pause_reason);
        msg!("Pause ends at timestamp: {}", pool_state.pause_end_timestamp);
        return Err(PoolError::PoolPaused.into());
    }
    Ok(())
}

/// Validates that the system is not paused for user operations.
/// This must be called by ALL operations except unpause.
/// This check takes precedence over pool-specific pause checks.
///
/// # Arguments
/// * `system_state_account` - The system state account to check
///
/// # Returns
/// * `ProgramResult` - Success if system is not paused, error if paused
pub fn validate_system_not_paused(system_state_account: &AccountInfo) -> ProgramResult {
    let system_state = SystemState::try_from_slice(&system_state_account.data.borrow())?;
    
    if system_state.is_paused {
        msg!("🛑 SYSTEM PAUSED: All operations blocked (overrides pool pause state)");
        msg!("Pause reason: {}", system_state.pause_reason);
        msg!("Paused at: {}", system_state.pause_timestamp);
        msg!("Only system unpause is allowed");
        return Err(PoolError::SystemPaused.into());
    }
    
    Ok(())
} 