//! Serialization Utilities
//! 
//! This module contains utilities for safe serialization of program data.
//! It provides buffer serialization patterns that ensure data integrity and persistence.

use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
};
use borsh::BorshSerialize;

/// Safe buffer serialization utility that ensures data integrity.
///
/// This function implements a two-step serialization process:
/// 1. Serialize data to a temporary buffer to verify success
/// 2. Copy the buffer to the account data atomically
///
/// This approach prevents issues where serialization reports "OK" but data doesn't persist,
/// which can occur with direct serialization on some Solana runtime versions.
///
/// # Arguments
/// * `data` - The data to serialize (must implement BorshSerialize)
/// * `account` - The account to write the data to
///
/// # Returns
/// * `ProgramResult` - Success or error code
pub fn serialize_to_account<T: BorshSerialize>(data: &T, account: &AccountInfo) -> ProgramResult {
    // Step 1: Serialize to a temporary buffer
    let mut serialized_data = Vec::new();
    match data.serialize(&mut serialized_data) {
        Ok(_) => {
            msg!("DEBUG: serialize_to_account: Serialization to buffer successful. Buffer len: {}", serialized_data.len());
        }
        Err(e) => {
            msg!("DEBUG: serialize_to_account: Serialization to buffer FAILED: {:?}", e);
            return Err(e.into());
        }
    }
    
    // Step 2: Verify buffer size fits in account
    let account_data_len = account.data_len();
    if serialized_data.len() > account_data_len {
        msg!("DEBUG: serialize_to_account: Serialized data too large for account. Need: {}, Have: {}", 
             serialized_data.len(), account_data_len);
        return Err(ProgramError::AccountDataTooSmall);
    }
    
    // Step 3: Copy the serialized data to the account data atomically
    {
        let mut account_data = account.data.borrow_mut();
        account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
        msg!("DEBUG: serialize_to_account: Data copied to account successfully");
    }
    
    msg!("DEBUG: serialize_to_account: Account data len after copy: {}", account.data.borrow().len());
    Ok(())
}

/// **CRITICAL WORKAROUND**: Prepare account data for GitHub Issue #31960
///
/// **DO NOT REMOVE - REQUIRED FOR CORRECT ACCOUNT SIZE CALCULATION**
///
/// This function creates a properly sized account based on actual serialized data size
/// rather than calculated packed length, preventing size mismatches that cause test failures.
///
/// # Arguments
/// * `data` - The data that will be stored in the account
///
/// # Returns
/// * `Result<(Vec<u8>, usize), ProgramError>` - (serialized_data, actual_size)
pub fn prepare_account_data<T: BorshSerialize>(data: &T) -> Result<(Vec<u8>, usize), ProgramError> {
    let mut serialized_data = Vec::new();
    data.serialize(&mut serialized_data).map_err(|_| ProgramError::InvalidAccountData)?;
    let actual_size = serialized_data.len();
    Ok((serialized_data, actual_size))
}

/// **CRITICAL WORKAROUND**: Get actual serialized size for GitHub Issue #31960
///
/// **DO NOT REMOVE - REQUIRED FOR CORRECT ACCOUNT SIZE CALCULATION**
///
/// This function calculates the actual serialized size of data, which may differ from
/// calculated packed lengths due to Borsh's variable-length encoding optimizations.
///
/// # Arguments
/// * `data` - The data to measure
///
/// # Returns
/// * `Result<usize, ProgramError>` - Actual serialized size or error
pub fn get_actual_serialized_size<T: BorshSerialize>(data: &T) -> Result<usize, ProgramError> {
    let mut buffer = Vec::new();
    data.serialize(&mut buffer).map_err(|_| ProgramError::InvalidAccountData)?;
    Ok(buffer.len())
}

/// Validates that serialized data will fit in the target account.
///
/// # Arguments
/// * `data` - The data to check
/// * `account_size` - The size of the target account
///
/// # Returns
/// * `ProgramResult` - Success if data fits, error otherwise
pub fn validate_serialized_size<T: BorshSerialize>(data: &T, account_size: usize) -> ProgramResult {
    let mut buffer = Vec::new();
    data.serialize(&mut buffer)?;
    
    if buffer.len() > account_size {
        msg!("Serialized data size {} exceeds account size {}", buffer.len(), account_size);
        return Err(ProgramError::AccountDataTooSmall);
    }
    
    Ok(())
} 