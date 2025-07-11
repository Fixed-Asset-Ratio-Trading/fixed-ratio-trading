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
/// Only the system authority can execute this instruction.
/// 
/// **PHASE 8: ULTRA-OPTIMIZED SYSTEM PAUSE ACCOUNT STRUCTURE**
/// After removing all placeholder accounts, this function now requires only 2 accounts
/// (down from 13), providing a 85% reduction in account overhead.
/// 
/// # Ultra-Optimized Account Order:
/// 0. **Authority/User Signer** (signer, writable) - System authority account
/// 1. **System State PDA** (writable) - System state account for pause
/// 
/// **PHASE 8 OPTIMIZATION BENEFITS:**
/// - Reduced account count: 13 → 2 accounts (85% reduction)
/// - Eliminated all placeholder accounts (indices 1-12 removed)
/// - Minimal transaction size and validation overhead
/// - Estimated compute unit savings: 385-770 CUs per transaction
/// - Extremely simplified client integration
/// 
/// # Arguments
/// * `program_id` - The program ID
/// * `reason` - Human-readable reason for the system pause
/// * `accounts` - Array of accounts in ultra-optimized order (2 accounts minimum)
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_pause_system(
    _program_id: &Pubkey,
    reason: String,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("🛑 Processing system pause: {} (Phase 8: Ultra-Optimized)", reason);
    
    // ✅ PHASE 8 OPTIMIZATION: Ultra-minimal account count requirement
    if accounts.len() < 2 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // ✅ ULTRA-OPTIMIZED ACCOUNT EXTRACTION: Extract accounts using new ultra-optimized indices
    let authority_account = &accounts[0];              // Index 0: Authority/User Signer
    let system_state_account = &accounts[1];           // Index 1: System State PDA (was 13)
    
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
/// Only the system authority can execute this instruction.
/// 
/// **PHASE 8: ULTRA-OPTIMIZED SYSTEM UNPAUSE ACCOUNT STRUCTURE**
/// After removing all placeholder accounts, this function now requires only 2 accounts
/// (down from 13), providing a 85% reduction in account overhead.
/// 
/// # Ultra-Optimized Account Order:
/// 0. **Authority/User Signer** (signer, writable) - System authority account
/// 1. **System State PDA** (writable) - System state account for unpause
/// 
/// **PHASE 8 OPTIMIZATION BENEFITS:**
/// - Reduced account count: 13 → 2 accounts (85% reduction)
/// - Eliminated all placeholder accounts (indices 1-12 removed)
/// - Minimal transaction size and validation overhead
/// - Estimated compute unit savings: 385-770 CUs per transaction
/// - Extremely simplified client integration
/// 
/// # Arguments
/// * `program_id` - The program ID
/// * `accounts` - Array of accounts in ultra-optimized order (2 accounts minimum)
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_unpause_system(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("✅ Processing system unpause (Phase 8: Ultra-Optimized)");
    
    // ✅ PHASE 8 OPTIMIZATION: Ultra-minimal account count requirement
    if accounts.len() < 2 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // ✅ ULTRA-OPTIMIZED ACCOUNT EXTRACTION: Extract accounts using new ultra-optimized indices
    let authority_account = &accounts[0];              // Index 0: Authority/User Signer
    let system_state_account = &accounts[1];           // Index 1: System State PDA (was 13)
    
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