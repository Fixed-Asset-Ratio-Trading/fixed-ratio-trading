//! System-wide pause functionality
//!
//! This module handles system-wide pause and unpause operations that affect
//! the entire contract. System pause takes precedence over all pool-specific
//! pause states and provides emergency controls for the contract authority.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::{invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};
use crate::{
    constants::*,
    error::PoolError,
    state::{SystemState, MainTreasuryState},
    utils::{serialization::serialize_to_account, validation::{validate_signer, validate_writable}},
    utils::account_builders::*,
};

/// Processes the InitializeProgram instruction with ultra-optimized account ordering.
/// 
/// This function implements an ultra-optimized account structure for program initialization
/// by removing all placeholder accounts that are not used in system operations. This provides
/// maximum efficiency for system infrastructure creation.
/// 
/// **PHASE 8: ULTRA-OPTIMIZED SYSTEM ACCOUNT STRUCTURE**
/// After removing all placeholder pool/token accounts, this function now requires only 5 accounts
/// (down from 13), providing a 62% reduction in account overhead.
/// 
/// # Ultra-Optimized Account Order:
/// 0. **Authority/User Signer** (signer, writable) - System authority account
/// 1. **System Program** (readable) - Solana system program
/// 2. **Rent Sysvar** (readable) - For rent calculations
/// 3. **Main Treasury PDA** (writable) - Main treasury account to create
/// 4. **System State PDA** (writable) - System state account to create
/// 
/// **PHASE 8 OPTIMIZATION BENEFITS:**
/// - Reduced account count: 13 → 5 accounts (62% reduction)
/// - Eliminated all placeholder pool/token accounts (indices 4-11 removed)
/// - Reduced transaction size and validation overhead significantly
/// - Estimated compute unit savings: 280-560 CUs per transaction
/// - Simplified client integration with minimal account requirements
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA derivation
/// * `accounts` - Array of accounts in ultra-optimized order (5 accounts minimum)
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_initialize_program(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("🚀 INITIALIZING PROGRAM: Creating system infrastructure (Phase 8: Ultra-Optimized)");
    
    // ✅ PHASE 8 OPTIMIZATION: Ultra-reduced account count requirement
    if accounts.len() < 5 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // ✅ ULTRA-OPTIMIZED ACCOUNT EXTRACTION: Extract accounts using new ultra-optimized indices
    let system_authority_account = &accounts[0];       // Index 0: Authority/User Signer
    let system_program_account = &accounts[1];         // Index 1: System Program
    let rent_sysvar_account = &accounts[2];            // Index 2: Rent Sysvar
    let main_treasury_account = &accounts[3];          // Index 3: Main Treasury PDA (was 12)
    let system_state_account = &accounts[4];           // Index 4: System State PDA (was 13)
    
    let rent = &Rent::from_account_info(rent_sysvar_account)?;

    // ✅ EXISTING VALIDATION LOGIC: Maintain all existing validations
    // Verify system authority is signer
    if !system_authority_account.is_signer {
        msg!("❌ System authority must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // System authority is validated by signer check above
    // The account key itself is the authority

    // 1. CREATE SYSTEMSTATE PDA
    let system_state_seeds = &[SYSTEM_STATE_SEED_PREFIX];
    let (expected_system_state_pda, system_state_bump) = Pubkey::find_program_address(system_state_seeds, program_id);
    
    if *system_state_account.key != expected_system_state_pda {
        msg!("❌ Invalid SystemState PDA");
        return Err(ProgramError::InvalidArgument);
    }

    // Check if already initialized
    if system_state_account.data_len() > 0 && !system_state_account.data_is_empty() {
        msg!("❌ Program already initialized (SystemState exists)");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let system_state_rent = rent.minimum_balance(SystemState::LEN);
    let system_state_seeds_with_bump = &[SYSTEM_STATE_SEED_PREFIX, &[system_state_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            system_authority_account.key,
            system_state_account.key,
            system_state_rent,
            SystemState::LEN as u64,
            program_id,
        ),
        &[
            system_authority_account.clone(),
            system_state_account.clone(),
            system_program_account.clone(),
        ],
        &[system_state_seeds_with_bump],
    )?;

    // Initialize SystemState data
    let system_state_data = SystemState::new(*system_authority_account.key);
    serialize_to_account(&system_state_data, system_state_account)?;
    
    // 2. CREATE MAIN TREASURY PDA
    let main_treasury_seeds = &[MAIN_TREASURY_SEED_PREFIX];
    let (expected_main_treasury_pda, main_treasury_bump) = Pubkey::find_program_address(main_treasury_seeds, program_id);
    
    if *main_treasury_account.key != expected_main_treasury_pda {
        msg!("❌ Invalid MainTreasury PDA");
        return Err(ProgramError::InvalidArgument);
    }

    let main_treasury_rent = rent.minimum_balance(MainTreasuryState::get_packed_len());
    let main_treasury_seeds_with_bump = &[MAIN_TREASURY_SEED_PREFIX, &[main_treasury_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            system_authority_account.key,
            main_treasury_account.key,
            main_treasury_rent,
            MainTreasuryState::get_packed_len() as u64,
            program_id,
        ),
        &[
            system_authority_account.clone(),
            main_treasury_account.clone(),
            system_program_account.clone(),
        ],
        &[main_treasury_seeds_with_bump],
    )?;

    // Initialize MainTreasury data
    let main_treasury_data = MainTreasuryState::new(*system_authority_account.key);
    serialize_to_account(&main_treasury_data, main_treasury_account)?;

    // ✅ PHASE 3: SPECIALIZED TREASURIES REMOVED
    // No longer creating specialized treasuries - all fees go directly to main treasury
    // This eliminates consolidation race conditions and simplifies the architecture

    msg!("✅ PROGRAM INITIALIZED SUCCESSFULLY (PHASE 3: CENTRALIZED ARCHITECTURE):");
    msg!("   • SystemState PDA: {}", system_state_account.key);
    msg!("   • MainTreasury PDA: {}", main_treasury_account.key);
    msg!("   • System Authority: {}", system_authority_account.key);
    msg!("🎯 Phase 3 Benefits:");
    msg!("   • Single centralized treasury for all fees");
    msg!("   • Real-time fee tracking (no consolidation needed)");
    msg!("   • Simplified architecture with no race conditions");
    msg!("   • Pool creation and treasury operations now available!");

    Ok(())
}

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