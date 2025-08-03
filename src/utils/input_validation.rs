//! Input validation utilities for consistent validation across all entry points
//!
//! This module provides centralized validation functions to ensure consistent
//! input validation across all instruction handlers, preventing DoS attacks
//! and ensuring proper error handling.

use solana_program::{
    account_info::AccountInfo,
    program_error::ProgramError,
    msg,
};

/// Expected account counts for each instruction type
pub const INITIALIZE_PROGRAM_ACCOUNTS: usize = 6;
pub const INITIALIZE_POOL_ACCOUNTS: usize = 13;
pub const DEPOSIT_ACCOUNTS: usize = 11;
pub const WITHDRAW_ACCOUNTS: usize = 11;
pub const SWAP_ACCOUNTS: usize = 11;  // 9 base + 2 mint accounts
pub const SET_SWAP_OWNER_ONLY_ACCOUNTS: usize = 4;
pub const UPDATE_POOL_FEES_ACCOUNTS: usize = 4;
pub const PAUSE_SYSTEM_ACCOUNTS: usize = 3;
pub const UNPAUSE_SYSTEM_ACCOUNTS: usize = 3;
pub const PAUSE_POOL_ACCOUNTS: usize = 4;
pub const UNPAUSE_POOL_ACCOUNTS: usize = 4;
pub const WITHDRAW_TREASURY_FEES_ACCOUNTS: usize = 6;
pub const CONSOLIDATE_POOL_FEES_MIN_ACCOUNTS: usize = 2; // Plus pool count
pub const GET_POOL_INFO_ACCOUNTS: usize = 4;
pub const GET_TREASURY_INFO_ACCOUNTS: usize = 1;
pub const GET_VERSION_ACCOUNTS: usize = 0;

/// Minimum instruction data sizes (in bytes) for each instruction type
/// These are conservative estimates based on Borsh serialization
pub const MIN_INSTRUCTION_DATA_SIZE: usize = 1; // At least instruction discriminant
pub const INITIALIZE_POOL_DATA_SIZE: usize = 17; // 1 (discriminant) + 8 (u64) + 8 (u64)
pub const DEPOSIT_DATA_SIZE: usize = 41; // 1 + 32 (Pubkey) + 8 (u64)
pub const WITHDRAW_DATA_SIZE: usize = 41; // 1 + 32 (Pubkey) + 8 (u64)
pub const SWAP_DATA_SIZE: usize = 49; // 1 + 32 (Pubkey) + 8 (u64) + 8 (u64)
pub const SET_SWAP_OWNER_ONLY_DATA_SIZE: usize = 34; // 1 + 1 (bool) + 32 (Pubkey)
pub const UPDATE_POOL_FEES_DATA_SIZE: usize = 25; // 1 + 8 (u64) + 8 (u64) + 8 (u64)
pub const CONSOLIDATE_POOL_FEES_DATA_SIZE: usize = 2; // 1 + 1 (u8 pool count)

/// Validates that the accounts array has the expected length
///
/// # Arguments
/// * `accounts` - The accounts array to validate
/// * `expected_count` - The expected number of accounts
/// * `instruction_name` - Name of the instruction for error messages
///
/// # Returns
/// * `Result<(), ProgramError>` - Ok if valid, error otherwise
pub fn validate_account_count(
    accounts: &[AccountInfo],
    expected_count: usize,
    instruction_name: &str,
) -> Result<(), ProgramError> {
    if accounts.len() != expected_count {
        msg!("❌ INVALID ACCOUNT COUNT for {}", instruction_name);
        msg!("   • Expected: {} accounts", expected_count);
        msg!("   • Received: {} accounts", accounts.len());
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Validates that the accounts array has at least the minimum required length
///
/// # Arguments
/// * `accounts` - The accounts array to validate
/// * `min_count` - The minimum number of accounts required
/// * `instruction_name` - Name of the instruction for error messages
///
/// # Returns
/// * `Result<(), ProgramError>` - Ok if valid, error otherwise
pub fn validate_min_account_count(
    accounts: &[AccountInfo],
    min_count: usize,
    instruction_name: &str,
) -> Result<(), ProgramError> {
    if accounts.len() < min_count {
        msg!("❌ INSUFFICIENT ACCOUNTS for {}", instruction_name);
        msg!("   • Minimum required: {} accounts", min_count);
        msg!("   • Received: {} accounts", accounts.len());
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Validates that the instruction data has at least the minimum required size
///
/// # Arguments
/// * `instruction_data` - The instruction data to validate
/// * `min_size` - The minimum required size in bytes
/// * `instruction_name` - Name of the instruction for error messages
///
/// # Returns
/// * `Result<(), ProgramError>` - Ok if valid, error otherwise
pub fn validate_instruction_data_size(
    instruction_data: &[u8],
    min_size: usize,
    instruction_name: &str,
) -> Result<(), ProgramError> {
    if instruction_data.len() < min_size {
        msg!("❌ INVALID INSTRUCTION DATA SIZE for {}", instruction_name);
        msg!("   • Minimum required: {} bytes", min_size);
        msg!("   • Received: {} bytes", instruction_data.len());
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(())
}

/// Validates consolidation instruction with dynamic pool count
///
/// # Arguments
/// * `accounts` - The accounts array to validate
/// * `pool_count` - The number of pools to consolidate
/// * `instruction_name` - Name of the instruction for error messages
///
/// # Returns
/// * `Result<(), ProgramError>` - Ok if valid, error otherwise
pub fn validate_consolidation_accounts(
    accounts: &[AccountInfo],
    pool_count: u8,
    instruction_name: &str,
) -> Result<(), ProgramError> {
    let expected_count = CONSOLIDATE_POOL_FEES_MIN_ACCOUNTS + pool_count as usize;
    validate_account_count(accounts, expected_count, instruction_name)
}