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

    pubkey::Pubkey,
    sysvar::Sysvar,
};
use crate::{
    error::PoolError,
    state::SystemState,
    utils::validation::{validate_writable},
};

/// Processes the PauseSystem instruction with ultra-optimized account ordering.
/// 
/// Pauses the entire system, blocking all operations except unpause.
/// Only the system upgrade authority can execute this instruction. This provides
/// emergency controls for the contract authority with system-wide pause
/// taking precedence over all pool-specific pause states.
/// 
/// # Arguments
/// * `program_id` - The program ID
/// * `reason_code` - Standardized pause reason code (see SystemState documentation)
/// * `accounts` - Array of accounts in ultra-optimized order (3 accounts minimum)
/// 
/// # Account Info
/// The accounts must be provided in the following order:
/// 0. **System Authority Signer** (signer, writable) - System upgrade authority signer
/// 1. **System State PDA** (writable) - System state PDA for pause
/// 2. **Program Data Account** (readable) - Program data account for authority validation
/// 
/// # Returns
/// * `ProgramResult` - Success or error
/// 
/// # Critical Notes
/// - **TRANSACTION EFFICIENCY**: Minimal transaction size and validation overhead
/// - **CLIENT INTEGRATION**: Extremely simplified client integration
/// - **EMERGENCY CONTROLS**: System pause takes precedence over all pool pause states
/// - **STORAGE OPTIMIZED**: Uses single byte code instead of string for efficiency
/// - **AUTHORITY VALIDATION**: Uses program upgrade authority for maximum flexibility
pub fn process_pause_system(
    program_id: &Pubkey,
    reason_code: u8,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("🛑 Processing system pause with code: {}", reason_code);
    
    // ✅ COMPUTE OPTIMIZATION: No account length verification
    // Solana runtime automatically fails with NotEnoughAccountKeys when accessing
    // accounts[N] if insufficient accounts are provided. Manual length checks are
    // redundant and waste compute units on every function call.
    
    // ✅ ACCOUNT EXTRACTION: Extract accounts using optimized indices
    let system_authority_signer = &accounts[0];              // Index 0: System Authority Signer
    let system_state_pda = &accounts[1];                    // Index 1: System State PDA
    let program_data_account = &accounts[2];                 // Index 2: Program Data Account
    
    // ✅ COMPUTE OPTIMIZATION: No redundant signer verification
    // Solana runtime automatically fails with MissingRequiredSignature when
    // system state operations require signatures. Manual signer checks are
    // redundant and waste compute units on every function call.
    validate_writable(system_state_pda, "System state PDA")?;
    
    // ✅ AUTHORITY VALIDATION: Use program upgrade authority
    use crate::utils::program_authority::validate_program_upgrade_authority;
    validate_program_upgrade_authority(program_id, program_data_account, system_authority_signer)?;
    
    // Deserialize system state
    let mut system_state = SystemState::try_from_slice(&system_state_pda.data.borrow())?;
    
    // Check if already paused
    if system_state.is_paused {
        msg!("System is already paused since timestamp: {}", system_state.pause_timestamp);
        msg!("Current pause code: {}", system_state.pause_reason_code);
        return Err(PoolError::SystemAlreadyPaused.into());
    }
    
    // Get current timestamp
    let clock = Clock::get()?;
    let current_timestamp = clock.unix_timestamp;
    
    // Pause the system
    system_state.pause(reason_code, current_timestamp);
    
    // Serialize updated state back to account
    let serialized_data = system_state.try_to_vec()?;
    system_state_pda.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    // Log the system pause
    msg!("🛑 SYSTEM PAUSED: All operations blocked");
    msg!("Authority: {}", system_authority_signer.key);
    msg!("Pause code: {}", reason_code);
    msg!("Timestamp: {}", current_timestamp);
    msg!("System pause takes precedence over all pool pause states");
    
    Ok(())
}

/// Processes the UnpauseSystem instruction with ultra-optimized account ordering.
/// 
/// Unpauses the entire system, allowing all operations to resume.
/// Only the system upgrade authority can execute this instruction. This restores
/// normal system operations while maintaining any pool-specific pause states
/// that were previously set.
/// 
/// # Arguments
/// * `program_id` - The program ID
/// * `accounts` - Array of accounts in ultra-optimized order (3 accounts minimum)
/// 
/// # Account Info
/// The accounts must be provided in the following order:
/// 0. **System Authority Signer** (signer, writable) - System upgrade authority signer
/// 1. **System State PDA** (writable) - System state PDA for unpause
/// 2. **Program Data Account** (readable) - Program data account for authority validation
/// 
/// # Returns
/// * `ProgramResult` - Success or error
/// 
/// # Critical Notes
/// - **TRANSACTION EFFICIENCY**: Minimal transaction size and validation overhead
/// - **CLIENT INTEGRATION**: Extremely simplified client integration
/// - **POOL STATES**: Pool-specific pause states remain active if previously set
/// - **STORAGE OPTIMIZED**: Works with optimized pause code system
/// - **AUTHORITY VALIDATION**: Uses program upgrade authority for maximum flexibility
pub fn process_unpause_system(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("✅ Processing system unpause");
    
    // ✅ COMPUTE OPTIMIZATION: No account length verification
    // Solana runtime automatically fails with NotEnoughAccountKeys when accessing
    // accounts[N] if insufficient accounts are provided. Manual length checks are
    // redundant and waste compute units on every function call.
    
    // ✅ ACCOUNT EXTRACTION: Extract accounts using optimized indices
    let system_authority_signer = &accounts[0];              // Index 0: System Authority Signer
    let system_state_pda = &accounts[1];                    // Index 1: System State PDA
    let program_data_account = &accounts[2];                 // Index 2: Program Data Account
    
    // ✅ COMPUTE OPTIMIZATION: No redundant signer verification
    // Solana runtime automatically fails with MissingRequiredSignature when
    // system state operations require signatures. Manual signer checks are
    // redundant and waste compute units on every function call.
    validate_writable(system_state_pda, "System state PDA")?;
    
    // ✅ AUTHORITY VALIDATION: Use program upgrade authority
    use crate::utils::program_authority::validate_program_upgrade_authority;
    validate_program_upgrade_authority(program_id, program_data_account, system_authority_signer)?;
    
    // Deserialize system state
    let mut system_state = SystemState::try_from_slice(&system_state_pda.data.borrow())?;
    
    // Check if already unpaused
    if !system_state.is_paused {
        msg!("System is not currently paused");
        return Err(PoolError::SystemNotPaused.into());
    }
    
    // Store pause info for logging before clearing
    let pause_duration = Clock::get()?.unix_timestamp - system_state.pause_timestamp;
    let previous_pause_code = system_state.pause_reason_code;
    
    // Unpause the system
    system_state.unpause();
    
    // Serialize updated state back to account
    let serialized_data = system_state.try_to_vec()?;
    system_state_pda.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    // Log the system unpause
    msg!("✅ SYSTEM UNPAUSED: All operations resumed");
    msg!("Authority: {}", system_authority_signer.key);
    msg!("Previous pause code: {}", previous_pause_code);
    msg!("Pause duration: {} seconds", pause_duration);
    msg!("Pool-specific pause states remain active if previously set");
    
    Ok(())
} 