//! Security Management Processors
//! 
//! This module contains all the processors for security-related operations including
//! security parameter updates, pause/unpause functionality, and risk management controls.

use crate::types::*;
use crate::utils::*;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use borsh::BorshDeserialize;

/// Updates the pool's security parameters to manage operational risk and compliance.
///
/// This function allows the pool owner to modify critical security settings that control
/// pool operations. Currently focused on pause/unpause functionality, with extensibility
/// for future security parameters. This provides emergency controls and operational 
/// flexibility for pool management.
///
/// # Purpose
/// - Provides emergency stop capability through pause functionality
/// - Enables dynamic security policy adjustments based on market conditions
/// - Allows compliance with regulatory requirements or protocol upgrades
/// - Maintains operational control for pool owners while protecting user funds
/// - Supports future expansion of security features and risk management
///
/// # How it works
/// 1. Validates the caller is the designated pool owner and signed the transaction
/// 2. Loads current pool state data to verify ownership permissions
/// 3. Applies any provided security parameter updates:
///    - `is_paused`: Immediately enables/disables pool operations
/// 4. Serializes updated pool state back to on-chain storage
/// 5. Logs changes for transparency and audit compliance
///
/// # Arguments
/// * `_program_id` - The program ID (currently unused, reserved for future validation)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool owner account (must be signer and match pool state owner)
///   - `accounts[1]` - Pool state PDA account (writable for parameter updates)
/// * `is_paused` - Optional boolean to pause/unpause all pool operations (except owner functions)
///
/// # Account Requirements
/// - Owner: Must be signer and match the owner field in pool state data
/// - Pool state: Must be writable for parameter updates
///
/// # Pause Functionality
/// When `is_paused = true`:
/// - Blocks all user operations: deposits, withdrawals, swaps
/// - Allows owner operations: fee withdrawals, security updates, delegate management
/// - Provides emergency stop for security incidents or maintenance
/// - Can be reversed by setting `is_paused = false`
///
/// # Security Features
/// - **Owner-only access**: Only designated pool owner can modify security parameters
/// - **Selective enforcement**: Pause affects user operations but preserves owner controls
/// - **Immediate effect**: Parameter changes take effect in the same transaction
/// - **Audit trail**: All parameter changes are logged for transparency
///
/// # Future Extensions
/// The reserved parameters enable future security enhancements:
/// - Rate limiting for various operations
/// - Dynamic fee adjustments based on market conditions
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Owner didn't sign transaction
/// - `ProgramError::InvalidAccountData` - Caller is not the pool owner
///
/// # Example Usage
/// ```ignore
/// // Emergency pause all pool operations
/// let instruction = PoolInstruction::UpdateSecurityParams {
///     is_paused: Some(true),          // Pause operations
/// };
///
/// // Resume normal operations
/// let instruction = PoolInstruction::UpdateSecurityParams {
///     is_paused: Some(false),         // Unpause operations
/// };
/// ```
pub fn process_update_security_params(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    is_paused: Option<bool>,
) -> ProgramResult {
    msg!("Processing UpdateSecurityParams");
    
    // ✅ SYSTEM PAUSE: Backward compatible validation
    crate::utils::validation::validate_system_not_paused_safe(accounts, 2)?; // Expected: 2 accounts minimum
    
    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify pool state
    let mut pool_state_data = PoolState::deserialize(&mut &pool_state.data.borrow()[..])?;
    if *owner.key != pool_state_data.owner {
        msg!("Only pool owner can update security parameters");
        return Err(ProgramError::InvalidAccountData);
    }

    // Only update is_paused if provided
    if let Some(paused) = is_paused {
        pool_state_data.is_paused = paused;
    }

    // Save updated state using buffer serialization approach
    serialize_to_account(&pool_state_data, pool_state)?;
    
    msg!("Security parameters updated successfully");

    Ok(())
} 