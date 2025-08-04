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
    
    // ✅ SECURITY: Signer validation handled by validate_program_upgrade_authority()
    // The validate_program_upgrade_authority() function includes comprehensive
    // signer checks as part of its authority validation process.
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
/// **NEW: SYSTEM RESTART PENALTY**: Applies a 3-day treasury withdrawal penalty
/// when system is re-enabled to prevent immediate fund drainage after system restart.
/// 
/// # Arguments
/// * `program_id` - The program ID
/// * `accounts` - Array of accounts in ultra-optimized order (4 accounts minimum)
/// 
/// # Account Info
/// The accounts must be provided in the following order:
/// 0. **System Authority Signer** (signer, writable) - System upgrade authority signer
/// 1. **System State PDA** (writable) - System state PDA for unpause
/// 2. **Main Treasury PDA** (writable) - Main treasury PDA for restart penalty application
/// 3. **Program Data Account** (readable) - Program data account for authority validation
/// 
/// # Returns
/// * `ProgramResult` - Success or error
/// 
/// # Critical Notes
/// - **TRANSACTION EFFICIENCY**: Minimal transaction size and validation overhead
/// - **CLIENT INTEGRATION**: Extremely simplified client integration
/// - **POOL STATES**: Pool-specific pause states remain active if previously set
/// - **RESTART PENALTY**: Treasury withdrawals blocked for 3 days after system restart
/// - **STORAGE OPTIMIZED**: Works with optimized pause code system
/// - **AUTHORITY VALIDATION**: Uses program upgrade authority for maximum flexibility
pub fn process_unpause_system(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("✅ Processing system unpause");
    
    // ✅ ACCOUNT VALIDATION: Ensure we have the required number of accounts
    // While Solana runtime normally handles this, explicit validation prevents
    // index out of bounds panics in edge cases and provides clearer error messages
    if accounts.len() < 4 {
        msg!("❌ Insufficient accounts provided: expected 4, got {}", accounts.len());
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // ✅ ACCOUNT EXTRACTION: Extract accounts using optimized indices
    let system_authority_signer = &accounts[0];              // Index 0: System Authority Signer
    let system_state_pda = &accounts[1];                    // Index 1: System State PDA
    let main_treasury_pda = &accounts[2];                   // Index 2: Main Treasury PDA
    let program_data_account = &accounts[3];                 // Index 3: Program Data Account
    
    // ✅ SECURITY: Signer validation handled by validate_program_upgrade_authority()
    // The validate_program_upgrade_authority() function includes comprehensive
    // signer checks as part of its authority validation process.
    validate_writable(system_state_pda, "System state PDA")?;
    validate_writable(main_treasury_pda, "Main treasury PDA")?;
    
    // ✅ AUTHORITY VALIDATION: Use program upgrade authority
    use crate::utils::program_authority::validate_program_upgrade_authority;
    validate_program_upgrade_authority(program_id, program_data_account, system_authority_signer)?;
    
    // ✅ TREASURY PDA VALIDATION: Verify main treasury PDA
    let (expected_main_treasury, _treasury_bump) = Pubkey::find_program_address(
        &[crate::constants::MAIN_TREASURY_SEED_PREFIX],
        program_id,
    );
    if *main_treasury_pda.key != expected_main_treasury {
        msg!("Invalid main treasury PDA. Expected: {}, Got: {}",
            expected_main_treasury, main_treasury_pda.key);
        return Err(ProgramError::InvalidAccountData);
    }
    
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
    
    // Get current timestamp for restart penalty
    let current_timestamp = Clock::get()?.unix_timestamp;
    
    // Unpause the system
    system_state.unpause();
    
    // Serialize updated state back to account
    let serialized_data = system_state.try_to_vec()?;
    system_state_pda.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    // **APPLY SYSTEM RESTART PENALTY**: Block treasury withdrawals for 3 days
    // Load and update main treasury state with restart penalty
    use crate::state::MainTreasuryState;
    let mut main_treasury_state = MainTreasuryState::try_from_slice(&main_treasury_pda.data.borrow())?;
    
    // Apply the 71-hour restart penalty
    main_treasury_state.apply_system_restart_penalty(current_timestamp);
    
    // Serialize updated treasury state back to account
    let treasury_serialized_data = main_treasury_state.try_to_vec()?;
    if treasury_serialized_data.len() > main_treasury_pda.data.borrow().len() {
        msg!("🚨 Critical Error: Treasury serialized data too large for account");
        return Err(ProgramError::AccountDataTooSmall);
    }
    main_treasury_pda.data.borrow_mut()[..treasury_serialized_data.len()].copy_from_slice(&treasury_serialized_data);
    
    // Log the system unpause with restart penalty information
    msg!("✅ SYSTEM UNPAUSED: All operations resumed");
    msg!("🔒 RESTART PENALTY APPLIED: Treasury withdrawals blocked for 3 days");
    msg!("Authority: {}", system_authority_signer.key);
    msg!("Previous pause code: {}", previous_pause_code);
    msg!("Pause duration: {} seconds", pause_duration);
    msg!("Treasury penalty expires at: {} (timestamp)", main_treasury_state.last_withdrawal_timestamp);
    msg!("Pool-specific pause states remain active if previously set");
    
    Ok(())
} 