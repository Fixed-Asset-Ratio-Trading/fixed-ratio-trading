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
    state::{SystemState, MainTreasuryState, SwapTreasuryState, HftTreasuryState},
    utils::{serialization::serialize_to_account, validation::{validate_signer, validate_writable}},
    utils::account_builders::*,
};

/// Processes the InitializeProgram instruction with standardized account ordering.
/// 
/// This function implements the standardized account ordering policy for program initialization.
/// It creates all system-level PDAs and infrastructure using consistent account positioning.
/// 
/// # Standardized Account Order:
/// 0. **Authority/User Signer** (signer, writable) - System authority account
/// 1. **System Program** (readable) - Solana system program
/// 2. **Rent Sysvar** (readable) - For rent calculations
/// 3. **Clock Sysvar** (readable) - Not used in initialization (placeholder)
/// 4. **Pool State PDA** (writable) - Not used in initialization (placeholder)
/// 5. **Token A Mint** (readable) - Not used in initialization (placeholder)
/// 6. **Token B Mint** (readable) - Not used in initialization (placeholder)
/// 7. **Token A Vault PDA** (writable) - Not used in initialization (placeholder)
/// 8. **Token B Vault PDA** (writable) - Not used in initialization (placeholder)
/// 9. **SPL Token Program** (readable) - Not used in initialization (placeholder)
/// 10. **User Input Token Account** (writable) - Not used in initialization (placeholder)
/// 11. **User Output Token Account** (writable) - Not used in initialization (placeholder)
/// 12. **Main Treasury PDA** (writable) - Main treasury account to create
/// 13. **Swap Treasury PDA** (writable) - Swap treasury account to create
/// 14. **HFT Treasury PDA** (writable) - HFT treasury account to create
/// 15. **System State PDA** (writable) - System state account to create (function-specific)
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA derivation
/// * `accounts` - Array of accounts in standardized order (16 accounts minimum, system authority at index 0)
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_initialize_program(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("🚀 INITIALIZING PROGRAM: Creating system infrastructure");
    
    // ✅ STANDARDIZED ACCOUNT VALIDATION: Validate standard account positions where applicable
    validate_standard_accounts(accounts)?;
    // Note: Most pool/token accounts are placeholders for initialization
    validate_treasury_accounts(accounts)?;
    
    // Validate we have enough accounts for initialization
    if accounts.len() < 16 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // ✅ STANDARDIZED ACCOUNT EXTRACTION: Extract accounts using standardized indices
    let system_authority_account = &accounts[0];       // Index 0: Authority/User Signer
    let system_program_account = &accounts[1];         // Index 1: System Program
    let rent_sysvar_account = &accounts[2];            // Index 2: Rent Sysvar
    // Index 3: Clock Sysvar (unused placeholder)
    // Indices 4-11: Pool/token accounts (unused placeholders)
    let main_treasury_account = &accounts[12];         // Index 12: Main Treasury PDA
    let swap_treasury_account = &accounts[13];         // Index 13: Swap Treasury PDA
    let hft_treasury_account = &accounts[14];          // Index 14: HFT Treasury PDA
    
    // ✅ FUNCTION-SPECIFIC ACCOUNTS: Initialization-specific accounts at standardized positions 15+
    let system_state_account = &accounts[15];          // Index 15: System State PDA
    
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

    // 3. CREATE SWAP TREASURY PDA
    let swap_treasury_seeds = &[SWAP_TREASURY_SEED_PREFIX];
    let (expected_swap_treasury_pda, swap_treasury_bump) = Pubkey::find_program_address(swap_treasury_seeds, program_id);
    
    if *swap_treasury_account.key != expected_swap_treasury_pda {
        msg!("❌ Invalid SwapTreasury PDA");
        return Err(ProgramError::InvalidArgument);
    }

    let swap_treasury_rent = rent.minimum_balance(SwapTreasuryState::get_packed_len());
    let swap_treasury_seeds_with_bump = &[SWAP_TREASURY_SEED_PREFIX, &[swap_treasury_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            system_authority_account.key,
            swap_treasury_account.key,
            swap_treasury_rent,
            SwapTreasuryState::get_packed_len() as u64,
            program_id,
        ),
        &[
            system_authority_account.clone(),
            swap_treasury_account.clone(),
            system_program_account.clone(),
        ],
        &[swap_treasury_seeds_with_bump],
    )?;

    // Initialize SwapTreasury data
    let swap_treasury_data = SwapTreasuryState::new();
    serialize_to_account(&swap_treasury_data, swap_treasury_account)?;

    // 4. CREATE HFT TREASURY PDA
    let hft_treasury_seeds = &[HFT_TREASURY_SEED_PREFIX];
    let (expected_hft_treasury_pda, hft_treasury_bump) = Pubkey::find_program_address(hft_treasury_seeds, program_id);
    
    if *hft_treasury_account.key != expected_hft_treasury_pda {
        msg!("❌ Invalid HftTreasury PDA");
        return Err(ProgramError::InvalidArgument);
    }

    let hft_treasury_rent = rent.minimum_balance(HftTreasuryState::get_packed_len());
    let hft_treasury_seeds_with_bump = &[HFT_TREASURY_SEED_PREFIX, &[hft_treasury_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            system_authority_account.key,
            hft_treasury_account.key,
            hft_treasury_rent,
            HftTreasuryState::get_packed_len() as u64,
            program_id,
        ),
        &[
            system_authority_account.clone(),
            hft_treasury_account.clone(),
            system_program_account.clone(),
        ],
        &[hft_treasury_seeds_with_bump],
    )?;

    // Initialize HftTreasury data
    let hft_treasury_data = HftTreasuryState::new();
    serialize_to_account(&hft_treasury_data, hft_treasury_account)?;

    msg!("✅ PROGRAM INITIALIZED SUCCESSFULLY:");
    msg!("   • SystemState PDA: {}", system_state_account.key);
    msg!("   • MainTreasury PDA: {}", main_treasury_account.key);
    msg!("   • SwapTreasury PDA: {}", swap_treasury_account.key);
    msg!("   • HftTreasury PDA: {}", hft_treasury_account.key);
    msg!("   • System Authority: {}", system_authority_account.key);
    msg!("🎯 Pool creation and treasury operations now available!");

    Ok(())
}

/// Processes the PauseSystem instruction with standardized account ordering.
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
/// # Standardized Account Order:
/// 0. **Authority/User Signer** (signer, writable) - System authority account
/// 1. **System Program** (readable) - Not used in pause (placeholder)
/// 2. **Rent Sysvar** (readable) - Not used in pause (placeholder)
/// 3. **Clock Sysvar** (readable) - Not used in pause (placeholder)
/// 4. **Pool State PDA** (writable) - Not used in pause (placeholder)
/// 5. **Token A Mint** (readable) - Not used in pause (placeholder)
/// 6. **Token B Mint** (readable) - Not used in pause (placeholder)
/// 7. **Token A Vault PDA** (writable) - Not used in pause (placeholder)
/// 8. **Token B Vault PDA** (writable) - Not used in pause (placeholder)
/// 9. **SPL Token Program** (readable) - Not used in pause (placeholder)
/// 10. **User Input Token Account** (writable) - Not used in pause (placeholder)
/// 11. **User Output Token Account** (writable) - Not used in pause (placeholder)
/// 12. **Main Treasury PDA** (writable) - Not used in pause (placeholder)
/// 13. **Swap Treasury PDA** (writable) - Not used in pause (placeholder)
/// 14. **HFT Treasury PDA** (writable) - Not used in pause (placeholder)
/// 15. **System State PDA** (writable) - System state account for pause (function-specific)
/// 
/// # Arguments
/// * `program_id` - The program ID
/// * `reason` - Human-readable reason for the system pause
/// * `accounts` - Array of accounts in standardized order (16 accounts minimum)
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_pause_system(
    _program_id: &Pubkey,
    reason: String,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("🛑 Processing system pause: {}", reason);
    
    // ✅ STANDARDIZED ACCOUNT VALIDATION: Validate standard account positions where applicable
    validate_standard_accounts(accounts)?;
    // Note: Most pool/token/treasury accounts are placeholders for pause operations
    
    // Validate we have enough accounts for pause operation
    if accounts.len() < 16 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // ✅ STANDARDIZED ACCOUNT EXTRACTION: Extract accounts using standardized indices
    let authority_account = &accounts[0];              // Index 0: Authority/User Signer
    // Indices 1-14: System/pool/token/treasury accounts (unused placeholders)
    
    // ✅ FUNCTION-SPECIFIC ACCOUNTS: Pause-specific accounts at standardized positions 15+
    let system_state_account = &accounts[15];          // Index 15: System State PDA
    
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

/// Processes the UnpauseSystem instruction with standardized account ordering.
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
/// # Standardized Account Order:
/// 0. **Authority/User Signer** (signer, writable) - System authority account
/// 1. **System Program** (readable) - Not used in unpause (placeholder)
/// 2. **Rent Sysvar** (readable) - Not used in unpause (placeholder)
/// 3. **Clock Sysvar** (readable) - Not used in unpause (placeholder)
/// 4. **Pool State PDA** (writable) - Not used in unpause (placeholder)
/// 5. **Token A Mint** (readable) - Not used in unpause (placeholder)
/// 6. **Token B Mint** (readable) - Not used in unpause (placeholder)
/// 7. **Token A Vault PDA** (writable) - Not used in unpause (placeholder)
/// 8. **Token B Vault PDA** (writable) - Not used in unpause (placeholder)
/// 9. **SPL Token Program** (readable) - Not used in unpause (placeholder)
/// 10. **User Input Token Account** (writable) - Not used in unpause (placeholder)
/// 11. **User Output Token Account** (writable) - Not used in unpause (placeholder)
/// 12. **Main Treasury PDA** (writable) - Not used in unpause (placeholder)
/// 13. **Swap Treasury PDA** (writable) - Not used in unpause (placeholder)
/// 14. **HFT Treasury PDA** (writable) - Not used in unpause (placeholder)
/// 15. **System State PDA** (writable) - System state account for unpause (function-specific)
/// 
/// # Arguments
/// * `program_id` - The program ID
/// * `accounts` - Array of accounts in standardized order (16 accounts minimum)
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_unpause_system(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("✅ Processing system unpause");
    
    // ✅ STANDARDIZED ACCOUNT VALIDATION: Validate standard account positions where applicable
    validate_standard_accounts(accounts)?;
    // Note: Most pool/token/treasury accounts are placeholders for unpause operations
    
    // Validate we have enough accounts for unpause operation
    if accounts.len() < 16 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // ✅ STANDARDIZED ACCOUNT EXTRACTION: Extract accounts using standardized indices
    let authority_account = &accounts[0];              // Index 0: Authority/User Signer
    // Indices 1-14: System/pool/token/treasury accounts (unused placeholders)
    
    // ✅ FUNCTION-SPECIFIC ACCOUNTS: Unpause-specific accounts at standardized positions 15+
    let system_state_account = &accounts[15];          // Index 15: System State PDA
    
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