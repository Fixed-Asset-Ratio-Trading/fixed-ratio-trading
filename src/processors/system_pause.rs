//! System-wide pause functionality
//!
//! This module handles system-wide pause and unpause operations that affect
//! the entire contract. System pause takes precedence over all pool-specific
//! pause states and provides emergency controls for the contract authority.
//!
//! Note: The process_initialize_program function has been moved to 
//! src/processors/process_initialize.rs for better code organization.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use crate::{
    error::PoolError,
    state::SystemState,
    utils::validation::{validate_signer, validate_writable},
};

/// Processes the PauseSystem instruction with ultra-optimized account ordering.
/// 
/// Pauses the entire system, blocking all operations except unpause.
/// Only the system authority can execute this instruction. This provides
/// emergency controls for the contract authority with system-wide pause
/// taking precedence over all pool-specific pause states.
/// 
/// # Arguments
/// * `program_id` - The program ID
/// * `reason` - Human-readable reason for the system pause
/// * `accounts` - Array of accounts in ultra-optimized order (2 accounts minimum)
/// 
/// # Account Info
/// The accounts must be provided in the following order:
/// 0. **Authority/User Signer** (signer, writable) - System authority account
/// 1. **System State PDA** (writable) - System state account for pause
/// 
/// # Returns
/// * `ProgramResult` - Success or error
/// 
/// # Critical Notes
/// - **ACCOUNT OPTIMIZATION**: Reduced account count from 13 to 2 accounts (85% reduction)
/// - **PLACEHOLDER ELIMINATION**: All placeholder accounts (indices 1-12) removed
/// - **TRANSACTION EFFICIENCY**: Minimal transaction size and validation overhead
/// - **COMPUTE SAVINGS**: Estimated compute unit savings of 385-770 CUs per transaction
/// - **CLIENT INTEGRATION**: Extremely simplified client integration
/// - **EMERGENCY CONTROLS**: System pause takes precedence over all pool pause states
pub fn process_pause_system(
    _program_id: &Pubkey,
    reason: String,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("🛑 Processing system pause: {}", reason);
    
    // ✅ ACCOUNT VALIDATION: Ultra-minimal account count requirement
    if accounts.len() < 2 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // ✅ ACCOUNT EXTRACTION: Extract accounts using optimized indices
    let authority_account = &accounts[0];              // Index 0: Authority/User Signer
    let system_state_account = &accounts[1];           // Index 1: System State PDA
    
    // ✅ EXISTING VALIDATION LOGIC: Maintain all existing validations
    validate_signer(authority_account, "System authority")?;
    validate_writable(system_state_account, "System state account")?;
    
    // Deserialize system state
    let mut system_state = SystemState::try_from_slice(&system_state_account.data.borrow())?;
    
    // Verify authority
    if !system_state.validate_authority(authority_account.key) {
        msg!("Unauthorized: {} is not the system authority", authority_account.key);
        return Err(PoolError::UnauthorizedAccess.into());
    }
    
    // Check if already paused
    if system_state.is_paused {
        msg!("System is already paused since timestamp: {}", system_state.pause_timestamp);
        msg!("Current pause reason: {}", system_state.pause_reason);
        return Err(PoolError::SystemAlreadyPaused.into());
    }
    
    // Get current timestamp
    let clock = Clock::get()?;
    let current_timestamp = clock.unix_timestamp;
    
    // Pause the system
    system_state.pause(reason.clone(), current_timestamp);
    
    // Serialize updated state back to account
    let serialized_data = system_state.try_to_vec()?;
    system_state_account.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    // Log the system pause
    msg!("🛑 SYSTEM PAUSED: All operations blocked");
    msg!("Authority: {}", authority_account.key);
    msg!("Reason: {}", reason);
    msg!("Timestamp: {}", current_timestamp);
    msg!("System pause takes precedence over all pool pause states");
    
    Ok(())
}

/// Processes the UnpauseSystem instruction with ultra-optimized account ordering.
/// 
/// Unpauses the entire system, allowing all operations to resume.
/// Only the system authority can execute this instruction. This restores
/// normal system operations while maintaining any pool-specific pause states
/// that were previously set.
/// 
/// # Arguments
/// * `program_id` - The program ID
/// * `accounts` - Array of accounts in ultra-optimized order (2 accounts minimum)
/// 
/// # Account Info
/// The accounts must be provided in the following order:
/// 0. **Authority/User Signer** (signer, writable) - System authority account
/// 1. **System State PDA** (writable) - System state account for unpause
/// 
/// # Returns
/// * `ProgramResult` - Success or error
/// 
/// # Critical Notes
/// - **ACCOUNT OPTIMIZATION**: Reduced account count from 13 to 2 accounts (85% reduction)
/// - **PLACEHOLDER ELIMINATION**: All placeholder accounts (indices 1-12) removed
/// - **TRANSACTION EFFICIENCY**: Minimal transaction size and validation overhead
/// - **COMPUTE SAVINGS**: Estimated compute unit savings of 385-770 CUs per transaction
/// - **CLIENT INTEGRATION**: Extremely simplified client integration
/// - **POOL STATES**: Pool-specific pause states remain active if previously set
pub fn process_unpause_system(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("✅ Processing system unpause");
    
    // ✅ ACCOUNT VALIDATION: Ultra-minimal account count requirement
    if accounts.len() < 2 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // ✅ ACCOUNT EXTRACTION: Extract accounts using optimized indices
    let authority_account = &accounts[0];              // Index 0: Authority/User Signer
    let system_state_account = &accounts[1];           // Index 1: System State PDA
    
    // ✅ EXISTING VALIDATION LOGIC: Maintain all existing validations
    validate_signer(authority_account, "System authority")?;
    validate_writable(system_state_account, "System state account")?;
    
    // Deserialize system state
    let mut system_state = SystemState::try_from_slice(&system_state_account.data.borrow())?;
    
    // Verify authority
    if !system_state.validate_authority(authority_account.key) {
        msg!("Unauthorized: {} is not the system authority", authority_account.key);
        return Err(PoolError::UnauthorizedAccess.into());
    }
    
    // Check if already unpaused
    if !system_state.is_paused {
        msg!("System is not currently paused");
        return Err(PoolError::SystemNotPaused.into());
    }
    
    // Store pause info for logging before clearing
    let pause_duration = Clock::get()?.unix_timestamp - system_state.pause_timestamp;
    let pause_reason = system_state.pause_reason.clone();
    
    // Unpause the system
    system_state.unpause();
    
    // Serialize updated state back to account
    let serialized_data = system_state.try_to_vec()?;
    system_state_account.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    // Log the system unpause
    msg!("✅ SYSTEM UNPAUSED: All operations resumed");
    msg!("Authority: {}", authority_account.key);
    msg!("Previous pause reason: {}", pause_reason);
    msg!("Pause duration: {} seconds", pause_duration);
    msg!("Pool-specific pause states remain active if previously set");
    
    Ok(())
} 