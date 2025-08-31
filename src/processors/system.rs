//! System Management Processor
//!
//! This module handles all system-wide management functions including
//! initialization, pause/unpause operations, and version information.

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
    utils::{
        serialization::serialize_to_account,
        validation::validate_writable,
    },
};

/// Processes the InitializeProgram instruction with maximum security and efficiency.
/// 
/// This function handles the program initialization process that sets up the core system 
/// infrastructure including system state and treasury. It enforces strict program authority 
/// validation to prevent unauthorized program initialization using Solana's built-in program 
/// upgrade authority mechanism for maximum flexibility.
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA derivation
/// * `admin_authority` - The pubkey that will become the admin authority
/// * `accounts` - Array of accounts in program upgrade authority order (6 accounts minimum)
/// 
/// # Account Info
/// The accounts must be provided in the following order:
/// 0. **Program Authority Signer** (signer, writable) - MUST match program upgrade authority
/// 1. **System Program Account** (readable) - Solana system program
/// 2. **Rent Sysvar Account** (readable) - For rent calculations
/// 3. **System State PDA** (writable) - MUST match derived PDA (validated internally)
/// 4. **Main Treasury PDA** (writable) - MUST match derived PDA (validated internally)
/// 5. **Program Data Account** (readable) - Contains the program upgrade authority
/// 
/// # Returns
/// * `ProgramResult` - Success or error
/// 
/// # Critical Notes
/// - **AUTHORITY VALIDATION**: Only the program upgrade authority can initialize the program
/// - **PDA VALIDATION**: All PDAs are strictly validated against derived addresses (no fake PDAs possible)
/// - **INITIALIZATION PROTECTION**: Prevents unauthorized program initialization attacks
/// - **AUTHORITY TRANSFER**: Authority can be transferred to PDAs, multisigs, or governance systems
/// - **SMART CONTRACT CONTROL**: Complete smart contract control over system infrastructure creation
/// - **DEPLOYMENT SECURITY**: Program upgrade authority is set during deployment and can be transferred
/// - **GOVERNANCE READY**: Authority can be handed over to governance systems for decentralization
pub fn process_system_initialize(
    program_id: &Pubkey,
    admin_authority: Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("🚀 INITIALIZING PROGRAM: Creating system infrastructure");
    
    // ✅ COMPUTE OPTIMIZATION: No account length verification
    // Solana runtime automatically fails with NotEnoughAccountKeys when accessing
    // accounts[N] if insufficient accounts are provided. Manual length checks are
    // redundant and waste compute units on every function call.
    
    // ✅ PROGRAM UPGRADE AUTHORITY ACCOUNT EXTRACTION: Extract accounts using upgrade authority indices
    let program_authority_signer = &accounts[0];      // Index 0: Program Authority Signer (MUST match upgrade authority)
    let system_program_account = &accounts[1];         // Index 1: System Program Account
    let rent_sysvar_account = &accounts[2];            // Index 2: Rent Sysvar Account
    let system_state_pda = &accounts[3];           // Index 3: System State PDA (MUST match derived PDA)
    let main_treasury_pda = &accounts[4];          // Index 4: Main Treasury PDA (MUST match derived PDA)
    let program_data_account = &accounts[5];           // Index 5: Program Data Account (contains upgrade authority)
    
    let rent = &Rent::from_account_info(rent_sysvar_account)?;

    // ✅ CRITICAL SECURITY: Validate program upgrade authority
    use crate::utils::program_authority::validate_program_upgrade_authority;
    
    msg!("🔍 Program Upgrade Authority Validation:");
    msg!("   Provided Authority: {}", program_authority_signer.key);
    msg!("   Program Data Account: {}", program_data_account.key);
    
    // Validate that the provided authority matches the program upgrade authority
    validate_program_upgrade_authority(program_id, program_data_account, program_authority_signer)?;

    // ✅ SECURITY: Derive System State PDA and validate provided account matches
    let system_state_seeds = &[SYSTEM_STATE_SEED_PREFIX];
    let (expected_system_state_pda, system_state_bump) = Pubkey::find_program_address(system_state_seeds, program_id);
    
    if *system_state_pda.key != expected_system_state_pda {
        msg!("❌ SECURITY VIOLATION: System State PDA does not match expected derived PDA");
        msg!("   Expected: {}", expected_system_state_pda);
        msg!("   Provided: {}", system_state_pda.key);
        return Err(ProgramError::InvalidAccountData);
    }

    // ✅ SECURITY: Check if program is already initialized
    if system_state_pda.data_len() > 0 && !system_state_pda.data_is_empty() {
        msg!("❌ Program already initialized (SystemState exists)");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // ✅ SECURITY: Derive Main Treasury PDA and validate provided account matches
    let main_treasury_seeds = &[MAIN_TREASURY_SEED_PREFIX];
    let (expected_main_treasury_pda, main_treasury_bump) = Pubkey::find_program_address(main_treasury_seeds, program_id);
    
    if *main_treasury_pda.key != expected_main_treasury_pda {
        msg!("❌ SECURITY VIOLATION: Main Treasury PDA does not match expected derived PDA");
        msg!("   Expected: {}", expected_main_treasury_pda);
        msg!("   Provided: {}", main_treasury_pda.key);
        return Err(ProgramError::InvalidAccountData);
    }

    msg!("✅ All PDAs validated against derived addresses");

    // Create System State PDA account
    let system_state_rent = rent.minimum_balance(SystemState::LEN);
    let system_state_seeds_with_bump = &[SYSTEM_STATE_SEED_PREFIX, &[system_state_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            program_authority_signer.key,
            system_state_pda.key,
            system_state_rent,
            SystemState::LEN as u64,
            program_id,
        ),
        &[
            program_authority_signer.clone(),
            system_state_pda.clone(),
            system_program_account.clone(),
        ],
        &[system_state_seeds_with_bump],
    )?;

    // Create system state data with the provided admin authority
    // This allows configurable admin authority at initialization time
    let system_state_data = SystemState::new(admin_authority);
    
    // Serialize system state to account with robust error handling
    let serialized_system_state = system_state_data.try_to_vec()?;
    msg!("🔍 SystemState serialization: {} bytes to write into {} byte account", 
         serialized_system_state.len(), system_state_pda.data_len());
    
    // Ensure account is large enough
    if serialized_system_state.len() > system_state_pda.data_len() {
        msg!("❌ SystemState data too large for account: {} > {}", 
             serialized_system_state.len(), system_state_pda.data_len());
        return Err(ProgramError::AccountDataTooSmall);
    }
    
    // Clear the account first, then write data
    {
        let mut account_data = system_state_pda.data.borrow_mut();
        // Zero out the entire account first
        account_data.fill(0);
        // Then write the actual data
        account_data[..serialized_system_state.len()].copy_from_slice(&serialized_system_state);
    }
    
    msg!("✅ SystemState written to account: {} bytes", serialized_system_state.len());
    
    // 🏦 Create main treasury PDA and account (Phase 3: Centralized Treasury)
    let main_treasury_rent = rent.minimum_balance(MainTreasuryState::get_packed_len());
    let main_treasury_seeds_with_bump = &[MAIN_TREASURY_SEED_PREFIX, &[main_treasury_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            program_authority_signer.key,
            main_treasury_pda.key,
            main_treasury_rent,
            MainTreasuryState::get_packed_len() as u64,
            program_id,
        ),
        &[
            program_authority_signer.clone(),
            main_treasury_pda.clone(),
            system_program_account.clone(),
        ],
        &[main_treasury_seeds_with_bump],
    )?;

    // Create main treasury state data
    let main_treasury_data = MainTreasuryState::new();
    serialize_to_account(&main_treasury_data, main_treasury_pda)?;

    // ✅ PROGRAM INITIALIZATION COMPLETE
    msg!("✅ PROGRAM INITIALIZED SUCCESSFULLY:");
    msg!("   • SystemState PDA: {} (validated against derived PDA)", system_state_pda.key);
    msg!("   • MainTreasury PDA: {} (validated against derived PDA)", main_treasury_pda.key);
    msg!("   • Program Authority: {} (validated against upgrade authority)", program_authority_signer.key);
    msg!("🔐 Security Benefits:");
    msg!("   • Only program upgrade authority can initialize");
    msg!("   • All PDAs strictly validated against derived addresses");
    msg!("   • Prevents unauthorized program initialization attacks");
    msg!("   • Authority can be transferred to PDAs/governance systems");
    msg!("   • Complete smart contract control over system infrastructure");
    msg!("   • Pool creation and treasury operations now available!");

    Ok(())
}

/// Processes the PauseSystem instruction with ultra-optimized account ordering.
/// 
/// Pauses the entire system, blocking all operations except unpause.
/// Only the admin authority can execute this instruction. This provides
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
/// 0. **System Authority Signer** (signer, writable) - Admin authority signer
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
/// - **AUTHORITY VALIDATION**: Uses admin authority with upgrade authority fallback for maximum flexibility
pub fn process_system_pause(
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
    
    // ✅ SECURITY: Validate writable accounts
    validate_writable(system_state_pda, "System state PDA")?;
    
    // ✅ AUTHORITY VALIDATION: Use admin authority with upgrade authority fallback
    use crate::utils::admin_validation::validate_admin_authority;
    validate_admin_authority(
        system_authority_signer,
        system_state_pda,
        Some(program_data_account),
        program_id,
    )?;
    
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
/// Only the admin authority can execute this instruction. This restores
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
/// 0. **System Authority Signer** (signer, writable) - Admin authority signer
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
/// - **AUTHORITY VALIDATION**: Uses admin authority with upgrade authority fallback for maximum flexibility
pub fn process_system_unpause(
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
    
    // ✅ SECURITY: Validate writable accounts
    validate_writable(system_state_pda, "System state PDA")?;
    validate_writable(main_treasury_pda, "Main treasury PDA")?;
    
    // ✅ AUTHORITY VALIDATION: Use admin authority with upgrade authority fallback
    use crate::utils::admin_validation::validate_admin_authority;
    validate_admin_authority(
        system_authority_signer,
        system_state_pda,
        Some(program_data_account),
        program_id,
    )?;
    
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

/// **VIEW INSTRUCTION**: Returns smart contract version information.
/// 
/// This function provides version information for the smart contract including
/// the main contract version from Cargo.toml and the schema version for data structures.
/// 
/// # Purpose
/// - Frontend/client version compatibility checking
/// - Deployment verification and audit trails
/// - API compatibility detection
/// - Development and debugging support
/// 
/// # Returns
/// * `ProgramResult` - Logs comprehensive version information
pub fn process_system_get_version(_accounts: &[AccountInfo]) -> ProgramResult {
    msg!("=== SMART CONTRACT VERSION ===");
    msg!("Contract Name: {}", env!("CARGO_PKG_NAME"));
    msg!("Contract Version: {}", env!("CARGO_PKG_VERSION"));
    msg!("Contract Description: {}", env!("CARGO_PKG_DESCRIPTION"));
    msg!("Schema Version: v2"); // From POOL_STATE_SEED_PREFIX
    msg!("Solana Program: Yes");
    msg!("License: {}", env!("CARGO_PKG_LICENSE"));
    msg!("Program ID: 4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn");
    msg!("===============================");
    
    Ok(())
}

/// **ADMIN AUTHORITY MANAGEMENT**: Process admin authority change with automatic completion
/// 
/// This unified function handles both initiation and completion of admin changes:
/// 1. If no change is pending or different admin proposed: starts 72-hour timer
/// 2. If 72+ hours have passed and pending admin differs from current: completes change
/// 3. If same admin as current is proposed: acts as cancellation (clears pending)
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA validation
/// * `new_admin` - The proposed new admin authority pubkey
/// * `accounts` - Array of accounts in the following order:
///   - [0] Current Admin Authority (signer) - Must be current admin
///   - [1] System State PDA (writable) - To store/update admin state
///   - [2] Program Data Account (readable) - For upgrade authority fallback during migration
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_admin_change(
    program_id: &Pubkey,
    new_admin: Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("🔄 PROCESSING ADMIN AUTHORITY CHANGE");
    msg!("====================================");
    msg!("Proposed admin: {}", new_admin);
    
    // Extract accounts
    let current_admin_signer = &accounts[0];
    let system_state_pda = &accounts[1];
    let program_data_account = &accounts[2];
    
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
    let mut system_state = SystemState::try_from_slice(&system_state_pda.data.borrow())?;
    
    // Validate current admin authority (with fallback to upgrade authority during migration)
    let is_current_admin = system_state.is_admin(current_admin_signer.key);
    let is_upgrade_authority = if !is_current_admin {
        // Fallback to upgrade authority validation for migration period
        use crate::utils::program_authority::validate_program_upgrade_authority;
        validate_program_upgrade_authority(program_id, program_data_account, current_admin_signer).is_ok()
    } else {
        false
    };
    
    if !is_current_admin && !is_upgrade_authority {
        msg!("❌ UNAUTHORIZED: Caller is not the current admin authority or upgrade authority");
        msg!("   Current admin: {}", system_state.admin_authority);
        msg!("   Provided signer: {}", current_admin_signer.key);
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Require signer
    if !current_admin_signer.is_signer {
        msg!("❌ Current admin must sign the transaction");
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    // Get current timestamp
    let current_timestamp = Clock::get()?.unix_timestamp;
    
    // Process the admin change
    match system_state.process_admin_change(new_admin, current_timestamp) {
        Ok(result) => {
            // Save updated system state
            let serialized_data = system_state.try_to_vec()?;
            system_state_pda.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);
            
            // Log the result
            match result {
                crate::state::AdminChangeResult::Initiated { new_admin, previous_pending } => {
                    msg!("✅ ADMIN CHANGE INITIATED");
                    msg!("   New pending admin: {}", new_admin);
                    msg!("   Timelock duration: {} hours", SystemState::ADMIN_CHANGE_TIMELOCK / 3600);
                    msg!("   Completion available after: {} (timestamp)", current_timestamp + SystemState::ADMIN_CHANGE_TIMELOCK);
                    if let Some(prev) = previous_pending {
                        msg!("   Previous pending admin replaced: {}", prev);
                        msg!("   ⚠️  Timer reset due to different admin proposed");
                    }
                },
                crate::state::AdminChangeResult::Completed { old_admin, new_admin } => {
                    msg!("🎉 ADMIN CHANGE COMPLETED!");
                    msg!("   Previous admin: {}", old_admin);
                    msg!("   New admin: {}", new_admin);
                    msg!("   All admin operations now require new admin signature");
                },
                crate::state::AdminChangeResult::Cancelled => {
                    msg!("🚫 ADMIN CHANGE CANCELLED");
                    msg!("   Pending change cleared (same admin as current proposed)");
                    msg!("   Current admin remains: {}", system_state.admin_authority);
                },
                crate::state::AdminChangeResult::NoChange => {
                    msg!("ℹ️ NO CHANGE NEEDED");
                    msg!("   Proposed admin same as current admin: {}", system_state.admin_authority);
                },
                crate::state::AdminChangeResult::Pending { pending_admin, remaining_seconds } => {
                    msg!("⏰ ADMIN CHANGE PENDING");
                    msg!("   Pending admin: {}", pending_admin);
                    msg!("   Time remaining: {} seconds ({} hours)", remaining_seconds, remaining_seconds / 3600);
                    msg!("   Same admin proposed again - no timer reset");
                },
            }
            
            Ok(())
        },
        Err(error_msg) => {
            msg!("❌ Admin change processing failed: {}", error_msg);
            Err(ProgramError::InvalidInstructionData)
        }
    }
}

