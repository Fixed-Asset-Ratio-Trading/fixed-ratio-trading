//! System-wide pause functionality
//!
//! This module handles system-wide pause and unpause operations that affect
//! the entire contract. System pause takes precedence over all pool-specific
//! pause states and provides emergency controls for the contract authority.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::{
    error::PoolError,
    state::SystemState,
    utils::validation::{validate_signer, validate_writable},
};

/// Processes the PauseSystem instruction.
/// 
/// Pauses the entire system, blocking all operations except unpause.
/// Only the system authority can execute this instruction.
/// 
/// # System Pause Behavior
/// When the system is paused:
/// - All user operations are blocked (swaps, liquidity, etc.)
/// - Only system unpause operations are allowed
/// - Takes precedence over pool-specific pause states
/// - Provides emergency control for security incidents
/// 
/// # Required Accounts
/// 0. `[signer]` System authority account
/// 1. `[writable]` System state account
/// 
/// # Arguments
/// * `program_id` - The program ID
/// * `accounts` - The accounts required for the instruction
/// * `reason` - Human-readable reason for the system pause
/// 
/// # Returns
/// * `ProgramResult` - Success or failure of the operation
pub fn process_pause_system(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    reason: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Parse accounts
    let authority_account = next_account_info(account_info_iter)?;
    let system_state_account = next_account_info(account_info_iter)?;
    
    // Validate account requirements
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

/// Processes the UnpauseSystem instruction.
/// 
/// Unpauses the entire system, allowing all operations to resume.
/// Only the system authority can execute this instruction.
/// 
/// # System Unpause Behavior
/// When the system is unpaused:
/// - All operations are allowed to resume
/// - Pool-specific pause states remain intact and continue to function
/// - Clears the system pause state completely
/// - Provides emergency recovery from system pause
/// 
/// # Required Accounts
/// 0. `[signer]` System authority account
/// 1. `[writable]` System state account
/// 
/// # Arguments
/// * `program_id` - The program ID
/// * `accounts` - The accounts required for the instruction
/// 
/// # Returns
/// * `ProgramResult` - Success or failure of the operation
pub fn process_unpause_system(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Parse accounts
    let authority_account = next_account_info(account_info_iter)?;
    let system_state_account = next_account_info(account_info_iter)?;
    
    // Validate account requirements
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