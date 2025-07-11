//! Program Initialization Processor
//!
//! This module handles the program initialization process that sets up
//! the core system infrastructure including system state and treasury.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
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
    state::{SystemState, MainTreasuryState},
    utils::{serialization::serialize_to_account},
};

/// Processes the InitializeProgram instruction with maximum security and efficiency.
/// 
/// **CRITICAL SECURITY FIX: PROGRAM AUTHORITY VALIDATION**
/// This function now enforces strict program authority validation to prevent unauthorized
/// program initialization. Only the hardcoded PROGRAM_AUTHORITY can initialize the program.
/// 
/// **PHASE 12: PROGRAM UPGRADE AUTHORITY PATTERN**
/// After implementing program upgrade authority validation, this function now uses
/// Solana's built-in program upgrade authority mechanism for maximum flexibility.
/// The upgrade authority can be transferred to PDAs, multisigs, or governance systems.
/// 
/// # Program Upgrade Authority Account Order:
/// 0. **Program Authority** (signer, writable) - MUST match program upgrade authority
/// 1. **System Program** (readable) - Solana system program
/// 2. **Rent Sysvar** (readable) - For rent calculations
/// 3. **System State PDA** (writable) - MUST match derived PDA (validated internally)
/// 4. **Main Treasury PDA** (writable) - MUST match derived PDA (validated internally)
/// 5. **Program Data Account** (readable) - Contains the program upgrade authority
/// 
/// **PHASE 12 SECURITY BENEFITS:**
/// - SECURITY FIX: Only program upgrade authority can initialize the program
/// - SECURITY FIX: All PDAs strictly validated against derived addresses (no fake PDAs possible)
/// - SECURITY FIX: Prevents unauthorized program initialization attacks
/// - FLEXIBILITY: Authority can be transferred to PDAs, multisigs, or governance systems
/// - Complete smart contract control over system infrastructure creation
/// - Eliminated risk of users providing fake PDA accounts
/// - Program upgrade authority validation prevents malicious initialization
/// - Maintains account count at 6 but with maximum security validation
/// 
/// **DEPLOYMENT SECURITY:**
/// - Program upgrade authority is set during deployment and can be transferred
/// - Only the program upgrade authority can initialize the program
/// - Prevents malicious actors from creating fake program instances
/// - Authority can be handed over to governance systems for decentralization
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA derivation
/// * `accounts` - Array of accounts in program upgrade authority order (6 accounts minimum)
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_initialize_program(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("🚀 INITIALIZING PROGRAM: Creating system infrastructure (Phase 12: Program Upgrade Authority)");
    
    // ✅ PHASE 12 SECURITY: Program upgrade authority account count requirement
    if accounts.len() < 6 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // ✅ PROGRAM UPGRADE AUTHORITY ACCOUNT EXTRACTION: Extract accounts using new upgrade authority indices
    let program_authority_account = &accounts[0];      // Index 0: Program Authority (MUST match upgrade authority)
    let system_program_account = &accounts[1];         // Index 1: System Program
    let rent_sysvar_account = &accounts[2];            // Index 2: Rent Sysvar
    let system_state_account = &accounts[3];           // Index 3: System State PDA (MUST match derived PDA)
    let main_treasury_account = &accounts[4];          // Index 4: Main Treasury PDA (MUST match derived PDA)
    let program_data_account = &accounts[5];           // Index 5: Program Data Account (contains upgrade authority)
    
    let rent = &Rent::from_account_info(rent_sysvar_account)?;

    // ✅ CRITICAL SECURITY: Validate program upgrade authority
    use crate::utils::program_authority::validate_program_upgrade_authority;
    
    msg!("🔍 Program Upgrade Authority Validation:");
    msg!("   Provided Authority: {}", program_authority_account.key);
    msg!("   Program Data Account: {}", program_data_account.key);
    
    // Validate that the provided authority matches the program upgrade authority
    validate_program_upgrade_authority(program_id, program_data_account, program_authority_account)?;

    // ✅ PHASE 11 SECURITY: Derive System State PDA and validate provided account matches
    let system_state_seeds = &[SYSTEM_STATE_SEED_PREFIX];
    let (expected_system_state_pda, system_state_bump) = Pubkey::find_program_address(system_state_seeds, program_id);
    
    if *system_state_account.key != expected_system_state_pda {
        msg!("❌ SECURITY VIOLATION: System State PDA does not match expected derived PDA");
        msg!("   Expected: {}", expected_system_state_pda);
        msg!("   Provided: {}", system_state_account.key);
        return Err(ProgramError::InvalidAccountData);
    }

    // ✅ PHASE 11 SECURITY: Check if program is already initialized
    if system_state_account.data_len() > 0 && !system_state_account.data_is_empty() {
        msg!("❌ Program already initialized (SystemState exists)");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // ✅ PHASE 11 SECURITY: Derive Main Treasury PDA and validate provided account matches
    let main_treasury_seeds = &[MAIN_TREASURY_SEED_PREFIX];
    let (expected_main_treasury_pda, main_treasury_bump) = Pubkey::find_program_address(main_treasury_seeds, program_id);
    
    if *main_treasury_account.key != expected_main_treasury_pda {
        msg!("❌ SECURITY VIOLATION: Main Treasury PDA does not match expected derived PDA");
        msg!("   Expected: {}", expected_main_treasury_pda);
        msg!("   Provided: {}", main_treasury_account.key);
        return Err(ProgramError::InvalidAccountData);
    }

    msg!("✅ All PDAs validated against derived addresses");

    // Create System State PDA account
    let system_state_rent = rent.minimum_balance(SystemState::LEN);
    let system_state_seeds_with_bump = &[SYSTEM_STATE_SEED_PREFIX, &[system_state_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            program_authority_account.key,
            system_state_account.key,
            system_state_rent,
            SystemState::LEN as u64,
            program_id,
        ),
        &[
            program_authority_account.clone(),
            system_state_account.clone(),
            system_program_account.clone(),
        ],
        &[system_state_seeds_with_bump],
    )?;

    // Initialize SystemState data
    let system_state_data = SystemState::new(*program_authority_account.key);
    serialize_to_account(&system_state_data, system_state_account)?;
    
    // Create Main Treasury PDA account
    let main_treasury_rent = rent.minimum_balance(MainTreasuryState::get_packed_len());
    let main_treasury_seeds_with_bump = &[MAIN_TREASURY_SEED_PREFIX, &[main_treasury_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            program_authority_account.key,
            main_treasury_account.key,
            main_treasury_rent,
            MainTreasuryState::get_packed_len() as u64,
            program_id,
        ),
        &[
            program_authority_account.clone(),
            main_treasury_account.clone(),
            system_program_account.clone(),
        ],
        &[main_treasury_seeds_with_bump],
    )?;

    // Initialize MainTreasury data
    let main_treasury_data = MainTreasuryState::new(*program_authority_account.key);
    serialize_to_account(&main_treasury_data, main_treasury_account)?;

    // ✅ PHASE 12: PROGRAM UPGRADE AUTHORITY INITIALIZATION COMPLETE
    msg!("✅ PROGRAM INITIALIZED SUCCESSFULLY (PHASE 12: PROGRAM UPGRADE AUTHORITY):");
    msg!("   • SystemState PDA: {} (validated against derived PDA)", system_state_account.key);
    msg!("   • MainTreasury PDA: {} (validated against derived PDA)", main_treasury_account.key);
    msg!("   • Program Authority: {} (validated against upgrade authority)", program_authority_account.key);
    msg!("🔐 Phase 12 Security Benefits:");
    msg!("   • Only program upgrade authority can initialize");
    msg!("   • All PDAs strictly validated against derived addresses");
    msg!("   • Prevents unauthorized program initialization attacks");
    msg!("   • Authority can be transferred to PDAs/governance systems");
    msg!("   • Complete smart contract control over system infrastructure");
    msg!("   • Pool creation and treasury operations now available!");

    Ok(())
} 