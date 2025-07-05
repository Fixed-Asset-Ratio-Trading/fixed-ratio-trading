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
};

/// **CRITICAL**: Initialize the entire program infrastructure
/// 
/// This function creates all system-level PDAs that the program depends on.
/// It MUST be called once before any other program operations.
/// 
/// # What it creates:
/// 1. SystemState PDA with system authority and global pause controls
/// 2. MainTreasury PDA for pool creation and liquidity fees
/// 3. SwapTreasury PDA for regular swap fees (high frequency)
/// 4. HftTreasury PDA for HFT swap fees (high frequency)
/// 
/// # Account Order (9 accounts required):
/// 0. **System Authority** (signer, writable) - Will control system operations
/// 1. **SystemState PDA** (writable) - Global system state to be created
/// 2. **MainTreasury PDA** (writable) - Main treasury to be created
/// 3. **SwapTreasury PDA** (writable) - Swap treasury to be created  
/// 4. **HftTreasury PDA** (writable) - HFT treasury to be created
/// 5. **System Program** (readable) - For account creation
/// 6. **Rent Sysvar** (readable) - For rent exemption calculations
/// 
/// # Security:
/// - Can only be called once (prevents re-initialization)
/// - Creates all PDAs with proper derivation validation
/// - Sets up rent exemption for all accounts
/// - System authority gains control over pause/treasury operations
pub fn process_initialize_program(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    system_authority: Pubkey,
) -> ProgramResult {
    msg!("🚀 INITIALIZING PROGRAM: Creating system infrastructure");
    
    let account_info_iter = &mut accounts.iter();
    let system_authority_account = next_account_info(account_info_iter)?;
    let system_state_account = next_account_info(account_info_iter)?;
    let main_treasury_account = next_account_info(account_info_iter)?;
    let swap_treasury_account = next_account_info(account_info_iter)?;
    let hft_treasury_account = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;
    let rent_sysvar_account = next_account_info(account_info_iter)?;

    let rent = &Rent::from_account_info(rent_sysvar_account)?;

    // Verify system authority is signer
    if !system_authority_account.is_signer {
        msg!("❌ System authority must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Verify system authority matches provided pubkey
    if *system_authority_account.key != system_authority {
        msg!("❌ System authority account mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

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
    let system_state_data = SystemState::new(system_authority);
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
    let main_treasury_data = MainTreasuryState::new(system_authority);
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
    msg!("   • System Authority: {}", system_authority);
    msg!("🎯 Pool creation and treasury operations now available!");

    Ok(())
}

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