//! Rent Calculation Utilities
//! 
//! This module contains utilities for managing rent-exempt status and account balance validation.
//! These functions ensure that program-owned accounts maintain sufficient balance for rent exemption.

use crate::types::*;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::rent::Rent,
};
use borsh::{BorshDeserialize, BorshSerialize};

/// Checks if an account is rent-exempt. For program-owned accounts, uses rent tracking; otherwise, checks minimum balance.
///
/// # Arguments
/// * `account` - The account to check
/// * `program_id` - The program ID
/// * `rent` - The rent sysvar
/// * `current_slot` - The current slot
///
/// # Returns
/// * `ProgramResult` - Success or error code
pub fn check_rent_exempt(account: &AccountInfo, program_id: &Pubkey, rent: &Rent, current_slot: u64) -> ProgramResult {
    // Check if the account is owned by the program
    if account.owner == program_id {
        // For program-owned accounts, use the new rent tracking mechanism
        ensure_rent_exempt(account, rent, current_slot)
    } else {
        // For other accounts, use the simple check
        let minimum_balance = rent.minimum_balance(account.data_len());
        if account.lamports() < minimum_balance {
            msg!("Account {} below rent-exempt threshold. Required: {}, Current: {}", 
                 account.key, minimum_balance, account.lamports());
            return Err(ProgramError::InsufficientFunds);
        }
        Ok(())
    }
}

/// Ensures that a program-owned account maintains rent-exempt status with dynamic tracking.
///
/// This function provides enhanced rent management for program-owned accounts by tracking
/// rent requirements over time and updating them when necessary. It's designed to handle
/// rent rate changes and ensure consistent rent-exempt status.
///
/// # Arguments
/// * `pool_state` - The pool state account to check
/// * `rent` - Current rent sysvar
/// * `current_slot` - Current slot number
///
/// # Returns
/// * `ProgramResult` - Success or error code
fn ensure_rent_exempt(
    pool_state: &AccountInfo,
    rent: &Rent,
    current_slot: u64,
) -> ProgramResult {
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    
    // Update rent requirements if needed
    if pool_state_data.rent_requirements.update_if_needed(rent, current_slot) {
        // ========================================================================
        // SOLANA BUFFER SERIALIZATION WORKAROUND FOR PDA DATA CORRUPTION
        // ========================================================================
        // Apply the same workaround used in process_deposit to prevent data corruption
        // when the pool state PDA is used as both authority and data storage.
        
        // Step 1: Serialize the pool state data to a temporary buffer
        let mut serialized_data = Vec::new();
        pool_state_data.serialize(&mut serialized_data)?;
        
        // Step 2: Atomic copy to account data
        {
            let mut account_data = pool_state.data.borrow_mut();
            account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
        }
    }

    // Calculate total required rent
    let total_required_rent = pool_state_data.rent_requirements.get_total_required_rent();
    
    // Check if we have enough balance
    if pool_state.lamports() < total_required_rent {
        return Err(PoolError::RentExemptError {
            account: *pool_state.key,
            required: total_required_rent,
            available: pool_state.lamports(),
        }.into());
    }

    Ok(())
} 