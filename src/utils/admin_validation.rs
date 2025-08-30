//! Admin Authority Validation Utilities
//!
//! This module provides centralized admin authority validation for all admin operations.
//! It handles the transition from upgrade authority to configurable admin authority.

use borsh::BorshDeserialize;
use solana_program::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    msg,
};

use crate::state::SystemState;

/// Validates that the provided signer is the current admin authority
/// 
/// This function checks the SystemState to determine the current admin authority
/// and validates that the provided signer matches and has signed the transaction.
/// 
/// During migration period, it also falls back to upgrade authority validation
/// if the admin authority is not properly set.
/// 
/// # Arguments
/// * `admin_signer` - The account claiming to be the admin authority
/// * `system_state_pda` - The system state PDA containing admin authority info
/// * `program_data_account` - Program data account for upgrade authority fallback
/// * `program_id` - The program ID for validation
/// 
/// # Returns
/// * `Ok(())` - If the signer is a valid admin authority
/// * `Err(ProgramError)` - If validation fails
pub fn validate_admin_authority(
    admin_signer: &AccountInfo,
    system_state_pda: &AccountInfo,
    program_data_account: Option<&AccountInfo>,
    program_id: &Pubkey,
) -> Result<(), ProgramError> {
    // Require signer
    if !admin_signer.is_signer {
        msg!("❌ Admin authority must sign the transaction");
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    // Validate system state PDA
    let (expected_system_state, _) = Pubkey::find_program_address(
        &[crate::constants::SYSTEM_STATE_SEED_PREFIX],
        program_id,
    );
    if *system_state_pda.key != expected_system_state {
        msg!("❌ Invalid system state PDA. Expected: {}, Got: {}", 
             expected_system_state, system_state_pda.key);
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Load system state
    let system_state = SystemState::try_from_slice(&system_state_pda.data.borrow())?;
    
    // Check if signer matches current admin authority
    if system_state.is_admin(admin_signer.key) {
        msg!("✅ Admin authority validation passed: {}", admin_signer.key);
        return Ok(());
    }
    
    // Fallback to upgrade authority validation during migration period
    if let Some(program_data_account) = program_data_account {
        msg!("ℹ️ Admin authority mismatch, checking upgrade authority as fallback");
        msg!("   Current admin: {}", system_state.admin_authority);
        msg!("   Provided signer: {}", admin_signer.key);
        
        use crate::utils::program_authority::validate_program_upgrade_authority;
        if validate_program_upgrade_authority(program_id, program_data_account, admin_signer).is_ok() {
            msg!("✅ Upgrade authority validation passed (migration fallback): {}", admin_signer.key);
            msg!("⚠️  Consider using InitiateAdminChange to set proper admin authority");
            return Ok(());
        }
    }
    
    // Both admin and upgrade authority validation failed
    msg!("❌ UNAUTHORIZED: Caller is not the admin authority or upgrade authority");
    msg!("   Current admin: {}", system_state.admin_authority);
    msg!("   Provided signer: {}", admin_signer.key);
    Err(ProgramError::InvalidAccountData)
}

/// Simplified admin validation for cases where we already have SystemState loaded
/// 
/// # Arguments
/// * `admin_signer` - The account claiming to be the admin authority
/// * `system_state` - Already loaded system state
/// * `program_data_account` - Program data account for upgrade authority fallback
/// * `program_id` - The program ID for validation
/// 
/// # Returns
/// * `Ok(())` - If the signer is a valid admin authority
/// * `Err(ProgramError)` - If validation fails
pub fn validate_admin_authority_with_state(
    admin_signer: &AccountInfo,
    system_state: &SystemState,
    program_data_account: Option<&AccountInfo>,
    program_id: &Pubkey,
) -> Result<(), ProgramError> {
    // Require signer
    if !admin_signer.is_signer {
        msg!("❌ Admin authority must sign the transaction");
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    // Check if signer matches current admin authority
    if system_state.is_admin(admin_signer.key) {
        msg!("✅ Admin authority validation passed: {}", admin_signer.key);
        return Ok(());
    }
    
    // Fallback to upgrade authority validation during migration period
    if let Some(program_data_account) = program_data_account {
        msg!("ℹ️ Admin authority mismatch, checking upgrade authority as fallback");
        msg!("   Current admin: {}", system_state.admin_authority);
        msg!("   Provided signer: {}", admin_signer.key);
        
        use crate::utils::program_authority::validate_program_upgrade_authority;
        if validate_program_upgrade_authority(program_id, program_data_account, admin_signer).is_ok() {
            msg!("✅ Upgrade authority validation passed (migration fallback): {}", admin_signer.key);
            msg!("⚠️  Consider using InitiateAdminChange to set proper admin authority");
            return Ok(());
        }
    }
    
    // Both admin and upgrade authority validation failed
    msg!("❌ UNAUTHORIZED: Caller is not the admin authority or upgrade authority");
    msg!("   Current admin: {}", system_state.admin_authority);
    msg!("   Provided signer: {}", admin_signer.key);
    Err(ProgramError::InvalidAccountData)
}
