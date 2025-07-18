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



/// Validates that a pool state is properly initialized.
/// validate_pool_initialized removed as we now use the pool state PDA to check if the pool is initialized.

/// Validates that a pool is not paused (pool-specific pause check).
///
/// # Arguments
/// * `pool_state` - The pool state to validate
/// * `_current_timestamp` - Current timestamp (for future time-based pause logic)
///
/// # Returns


/// **SECURITY CRITICAL**: Validates and deserializes PoolState with PDA verification.
/// 
/// This function prevents malicious users from passing fake PoolState accounts by:
/// 1. Deriving the expected PoolState PDA from the pool's token mints and ratio
/// 2. Validating the provided account matches the expected PDA
/// 3. Only then deserializing the PoolState data
/// 
/// # Arguments
/// * `pool_state_account` - The pool state account to validate and deserialize
/// * `program_id` - The program ID for PDA derivation
/// 
/// # Returns
/// * `Result<PoolState, ProgramError>` - The validated and deserialized PoolState or error
pub fn validate_and_deserialize_pool_state_secure(
    pool_state_account: &AccountInfo,
    program_id: &Pubkey,
) -> Result<PoolState, ProgramError> {
    // First, deserialize to get the token mints and ratio for PDA derivation
    let pool_state_data = PoolState::deserialize(&mut &pool_state_account.data.borrow()[..])?;
    
    // Now validate this is the correct PDA for these parameters
    let (expected_pool_state_pda, _) = Pubkey::find_program_address(
        &[
            POOL_STATE_SEED_PREFIX,
            pool_state_data.token_a_mint.as_ref(),
            pool_state_data.token_b_mint.as_ref(),
            &pool_state_data.ratio_a_numerator.to_le_bytes(),
            &pool_state_data.ratio_b_denominator.to_le_bytes(),
        ],
        program_id,
    );
    
    if *pool_state_account.key != expected_pool_state_pda {
        msg!("🚨 SECURITY: Invalid PoolState PDA provided");
        msg!("Expected: {}, Provided: {}", expected_pool_state_pda, pool_state_account.key);
        msg!("Token A: {}, Token B: {}", pool_state_data.token_a_mint, pool_state_data.token_b_mint);
        msg!("Ratio: {}:{}", pool_state_data.ratio_a_numerator, pool_state_data.ratio_b_denominator);
        return Err(PoolError::TreasuryValidationFailed {
            expected: expected_pool_state_pda,
            provided: *pool_state_account.key,
            treasury_type: "PoolState".to_string(),
        }.into());
    }
    
    // PDA validation passed, return the deserialized data
    Ok(pool_state_data)
}



/// Validates that the system is not paused for user operations.
/// This check takes precedence over pool-specific pause checks.
///
/// **SECURITY FIX**: Now validates PDA to prevent fake SystemState accounts.
///
/// # Arguments
/// * `system_state_account` - The system state account to check
/// * `program_id` - The program ID for PDA derivation
///
/// # Returns
/// * `ProgramResult` - Success if system is not paused, error if paused
pub fn validate_system_not_paused_secure(
    system_state_account: &AccountInfo,
    program_id: &Pubkey,
) -> ProgramResult {
    // 🔒 SECURITY: First validate this is the correct SystemState PDA
    let (expected_system_state_pda, _) = Pubkey::find_program_address(
        &[crate::constants::SYSTEM_STATE_SEED_PREFIX], // b"system_state"
        program_id,
    );
    
    if *system_state_account.key != expected_system_state_pda {
        msg!("🚨 SECURITY: Invalid SystemState PDA provided");
        msg!("Expected: {}, Provided: {}", expected_system_state_pda, system_state_account.key);
        return Err(PoolError::TreasuryValidationFailed {
            expected: expected_system_state_pda,
            provided: *system_state_account.key,
            treasury_type: "SystemState".to_string(),
        }.into());
    }
    
    // Now safely deserialize and validate pause state
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

/// **POOL_FLAG_ONE_TO_MANY_RATIO Validation Function**
/// 
/// Determines if a pool qualifies for the POOL_FLAG_ONE_TO_MANY_RATIO flag based on
/// specific whole-number ratio patterns. This function analyzes token ratios in display
/// units to identify pools suitable for applications targeting these specific patterns.
///
/// **Flag Logic Definition**: The flag should be set when the pool has a token ratio where:
/// * One or both tokens have a ratio value of exactly 1 (representing 1 whole token, not fractional)
/// * The corresponding token(s) must have whole number values only (no fractional amounts)
/// * Both ratios must be positive (greater than zero)
///
/// **Technical Implementation**:
/// 1. Converts base units to display units using token decimal factors
/// 2. Validates both ratios represent whole numbers (no fractional parts)
/// 3. Ensures both ratios are positive
/// 4. Checks if one of the ratios equals exactly 1.0 in display units
///
/// **Valid Examples** (returns true):
/// * ✅ 1 SOL = 160 USDT (one token equals exactly 1, other is whole number)
/// * ✅ 1000 DOGE = 1 USDC (one token equals exactly 1, other is whole number)
/// * ✅ 1 BTC = 50000 USDT (one token equals exactly 1, other is whole number)
///
/// **Invalid Examples** (returns false):
/// * ❌ 1 SOL = 160.55 USDT (fractional value violates whole-number requirement)
/// * ❌ 0.5 BTC = 1 ETH (fractional value violates whole-number requirement)
/// * ❌ 2 TokenA = 3 TokenB (neither token equals exactly 1)
/// * ❌ 2.5 TokenA = 3.7 TokenB (fractional values violate whole-number requirement)
///
/// **Application Purpose**: This flag serves as a filtering mechanism for applications
/// that specifically target pools with these whole-number ratios. Other applications
/// remain free to implement different ratio types as needed.
///
/// **Usage in Pool Creation**: This function is called during pool creation in
/// `process_initialize_pool()` to automatically set the POOL_FLAG_ONE_TO_MANY_RATIO
/// flag based on the provided token ratios and their decimal configurations.
///
/// # Arguments
/// * `ratio_a_numerator` - Token A base units in the ratio
/// * `ratio_b_denominator` - Token B base units in the ratio
/// * `token_a_decimals` - Number of decimal places for token A (used for display conversion)
/// * `token_b_decimals` - Number of decimal places for token B (used for display conversion)
///
/// # Returns
/// * `bool` - true if the pool qualifies for POOL_FLAG_ONE_TO_MANY_RATIO, false otherwise
///
/// # Examples
/// ```
/// use fixed_ratio_trading::utils::validation::check_one_to_many_ratio;
/// 
/// // ✅ Valid: 1 SOL = 2 USDC (SOL: 9 decimals, USDC: 6 decimals)
/// let is_one_to_many = check_one_to_many_ratio(
///     1_000_000_000,  // 1.0 SOL in base units
///     2_000_000,      // 2.0 USDC in base units
///     9,              // SOL decimals
///     6               // USDC decimals
/// ); // Returns true - one token equals 1, both are whole numbers
/// assert!(is_one_to_many);
/// 
/// // ❌ Invalid: 1 BTC = 1.01 USDT (BTC: 8 decimals, USDT: 6 decimals)
/// let is_one_to_many = check_one_to_many_ratio(
///     100_000_000,    // 1.0 BTC in base units
///     1_010_000,      // 1.01 USDT in base units
///     8,              // BTC decimals
///     6               // USDT decimals
/// ); // Returns false - 1.01 is not a whole number
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

/// **NEW: Secure system state validation**
/// Validates that the account is the correct SystemState PDA and deserializes it
pub fn validate_and_deserialize_system_state_secure(
    system_state_account: &AccountInfo,
    program_id: &Pubkey,
) -> Result<SystemState, ProgramError> {
    // Validate this is the correct SystemState PDA
    let (expected_system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        program_id,
    );
    
    if *system_state_account.key != expected_system_state_pda {
        msg!("❌ Invalid SystemState PDA provided");
        msg!("❌ Expected: {}", expected_system_state_pda);
        msg!("❌ Got: {}", system_state_account.key);
        return Err(PoolError::InvalidSystemStatePDA.into());
    }
    
    // Deserialize and return system state
    SystemState::try_from_slice(&system_state_account.data.borrow())
        .map_err(|_| PoolError::InvalidSystemStateDeserialization.into())
} 