//! Treasury Management Processors
//!
//! This module handles all treasury-related operations including:
//! - Contract fee withdrawals by system authority
//! - Consolidation of specialized treasuries into main treasury
//! - Treasury information queries and analytics

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};

use crate::{
    constants::*,
    error::PoolError,
    state::{MainTreasuryState, SwapTreasuryState, HftTreasuryState, SystemState},
    utils::validation::{validate_signer, validate_writable},
};

/// Processes treasury fee withdrawal by system authority.
/// 
/// This function allows the system authority to withdraw accumulated contract fees
/// from the main treasury. This is the primary mechanism for extracting protocol revenue.
/// 
/// # Required Accounts (in order):
/// 0. `[signer]` System authority account
/// 1. `[writable]` Main treasury PDA account  
/// 2. `[writable]` Destination account (receives the SOL)
/// 3. `[]` System program
/// 4. `[]` Rent sysvar
/// 5. `[]` System state account (for authority validation)
/// 
/// # Arguments:
/// * `program_id` - The program ID for PDA derivation
/// * `accounts` - Array of required accounts
/// * `amount` - Amount to withdraw in lamports (0 = withdraw all available)
/// 
/// # Security:
/// - Only system authority can execute this instruction
/// - Maintains rent exemption for treasury account
/// - Validates all account ownership and signatures
/// 
/// # Errors:
/// - `UnauthorizedAccess` - Caller is not system authority
/// - `InsufficientFunds` - Requested amount exceeds available balance
/// - `InvalidAccountData` - Account validation failures
pub fn process_withdraw_treasury_fees(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    msg!("🏦 Processing treasury fee withdrawal: {} lamports", amount);
    
    let account_info_iter = &mut accounts.iter();
    let authority_account = next_account_info(account_info_iter)?;
    let main_treasury_account = next_account_info(account_info_iter)?;
    let destination_account = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let rent_sysvar = next_account_info(account_info_iter)?;
    let system_state_account = next_account_info(account_info_iter)?;
    
    // Validate account requirements
    validate_signer(authority_account, "System authority")?;
    validate_writable(main_treasury_account, "Main treasury")?;
    validate_writable(destination_account, "Destination account")?;
    
    // Verify main treasury PDA
    let (expected_main_treasury, treasury_bump) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        program_id,
    );
    if *main_treasury_account.key != expected_main_treasury {
        msg!("Invalid main treasury PDA. Expected: {}, Got: {}", 
             expected_main_treasury, main_treasury_account.key);
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Load and validate system state to verify authority
    let system_state = SystemState::try_from_slice(&system_state_account.data.borrow())?;
    if !system_state.validate_authority(authority_account.key) {
        msg!("Unauthorized: {} is not the system authority", authority_account.key);
        return Err(PoolError::UnauthorizedAccess.into());
    }
    msg!("✅ Authority validation passed: {}", authority_account.key);
    
    // Load main treasury state
    let mut main_treasury = MainTreasuryState::try_from_slice(&main_treasury_account.data.borrow())?;
    
    // Calculate rent-exempt minimum
    let rent = &Rent::from_account_info(rent_sysvar)?;
    let minimum_balance = rent.minimum_balance(main_treasury_account.data_len());
    let current_balance = main_treasury_account.lamports();
    
    // Determine withdrawal amount
    let available_balance = if current_balance > minimum_balance {
        current_balance - minimum_balance
    } else {
        0
    };
    
    let withdrawal_amount = if amount == 0 {
        // Withdraw all available
        available_balance
    } else {
        // Withdraw requested amount
        if amount > available_balance {
            msg!("Insufficient funds. Requested: {}, Available: {}", amount, available_balance);
            return Err(ProgramError::InsufficientFunds);
        }
        amount
    };
    
    if withdrawal_amount == 0 {
        msg!("No funds available for withdrawal. Current: {}, Required for rent: {}", 
             current_balance, minimum_balance);
        return Ok(());
    }
    
    msg!("💰 Withdrawing {} lamports from treasury", withdrawal_amount);
    msg!("   Current balance: {}", current_balance);
    msg!("   Minimum balance: {}", minimum_balance);
    msg!("   Available: {}", available_balance);
    
    // Transfer SOL from treasury to destination
    **main_treasury_account.try_borrow_mut_lamports()? -= withdrawal_amount;
    **destination_account.try_borrow_mut_lamports()? += withdrawal_amount;
    
    // Update treasury state
    main_treasury.record_withdrawal(withdrawal_amount)
        .map_err(|_| ProgramError::InsufficientFunds)?;
    
    // Save updated treasury state
    let serialized_data = main_treasury.try_to_vec()?;
    main_treasury_account.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    msg!("✅ Treasury withdrawal completed successfully");
    msg!("   Amount withdrawn: {} lamports", withdrawal_amount);
    msg!("   Total withdrawn to date: {} lamports", main_treasury.total_withdrawn);
    msg!("   Remaining treasury balance: {} lamports", main_treasury.total_balance);
    
    Ok(())
}

/// Consolidates specialized treasuries into the main treasury.
/// 
/// This function empties the specialized swap and HFT treasuries, transferring their
/// balances and statistics to the main treasury. This provides accurate fee reporting
/// while maintaining performance during operations.
/// 
/// # Required Accounts (in order):
/// 0. `[writable]` Main treasury PDA account
/// 1. `[writable]` Swap treasury PDA account  
/// 2. `[writable]` HFT treasury PDA account
/// 3. `[]` Clock sysvar
/// 
/// # Arguments:
/// * `program_id` - The program ID for PDA derivation
/// * `accounts` - Array of required accounts
/// 
/// # Returns:
/// * `ProgramResult` - Success or error
pub fn process_consolidate_treasuries(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("🔄 Processing treasury consolidation");
    
    let account_info_iter = &mut accounts.iter();
    let main_treasury_account = next_account_info(account_info_iter)?;
    let swap_treasury_account = next_account_info(account_info_iter)?;
    let hft_treasury_account = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;
    
    // Validate accounts are writable
    validate_writable(main_treasury_account, "Main treasury")?;
    validate_writable(swap_treasury_account, "Swap treasury")?;
    validate_writable(hft_treasury_account, "HFT treasury")?;
    
    // Verify PDA addresses
    let (expected_main_treasury, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX], program_id);
    let (expected_swap_treasury, _) = Pubkey::find_program_address(
        &[SWAP_TREASURY_SEED_PREFIX], program_id);
    let (expected_hft_treasury, _) = Pubkey::find_program_address(
        &[HFT_TREASURY_SEED_PREFIX], program_id);
    
    if *main_treasury_account.key != expected_main_treasury {
        return Err(ProgramError::InvalidAccountData);
    }
    if *swap_treasury_account.key != expected_swap_treasury {
        return Err(ProgramError::InvalidAccountData);
    }
    if *hft_treasury_account.key != expected_hft_treasury {
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Load treasury states
    let mut main_treasury = MainTreasuryState::try_from_slice(&main_treasury_account.data.borrow())?;
    let mut swap_treasury = SwapTreasuryState::try_from_slice(&swap_treasury_account.data.borrow())?;
    let mut hft_treasury = HftTreasuryState::try_from_slice(&hft_treasury_account.data.borrow())?;
    
    // Get current timestamp
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_timestamp = clock.unix_timestamp;
    
    // Calculate total SOL to transfer
    let swap_balance = swap_treasury_account.lamports();
    let hft_balance = hft_treasury_account.lamports();
    let total_to_transfer = swap_balance + hft_balance;
    
    msg!("💰 Consolidating treasuries:");
    msg!("   Swap treasury: {} lamports, {} swaps", swap_balance, swap_treasury.swap_count);
    msg!("   HFT treasury: {} lamports, {} HFT swaps", hft_balance, hft_treasury.hft_swap_count);
    msg!("   Total to transfer: {} lamports", total_to_transfer);
    
    if total_to_transfer > 0 {
        // Transfer SOL from specialized treasuries to main treasury
        **swap_treasury_account.try_borrow_mut_lamports()? -= swap_balance;
        **hft_treasury_account.try_borrow_mut_lamports()? -= hft_balance;
        **main_treasury_account.try_borrow_mut_lamports()? += total_to_transfer;
    }
    
    // Drain specialized treasury data
    let swap_data = swap_treasury.drain();
    let hft_data = hft_treasury.drain();
    
    // Update timestamps
    swap_treasury.last_consolidation = current_timestamp;
    hft_treasury.last_consolidation = current_timestamp;
    
    // Consolidate into main treasury
    main_treasury.consolidate_from_specialized_treasuries(
        swap_data,
        hft_data,
        current_timestamp
    );
    
    // Save updated states
    let main_data = main_treasury.try_to_vec()?;
    let swap_data = swap_treasury.try_to_vec()?;
    let hft_data = hft_treasury.try_to_vec()?;
    
    main_treasury_account.data.borrow_mut()[..main_data.len()].copy_from_slice(&main_data);
    swap_treasury_account.data.borrow_mut()[..swap_data.len()].copy_from_slice(&swap_data);
    hft_treasury_account.data.borrow_mut()[..hft_data.len()].copy_from_slice(&hft_data);
    
    msg!("✅ Treasury consolidation completed");
    msg!("   Main treasury balance: {} lamports", main_treasury.total_balance);
    msg!("   Total regular swaps: {}", main_treasury.regular_swap_count);
    msg!("   Total HFT swaps: {}", main_treasury.hft_swap_count);
    
    Ok(())
}

/// Returns comprehensive treasury information with automatic consolidation.
/// 
/// This view function consolidates treasuries before returning data to ensure
/// the most accurate and up-to-date information.
/// 
/// # Required Accounts (in order):
/// 0. `[writable]` Main treasury PDA account
/// 1. `[writable]` Swap treasury PDA account
/// 2. `[writable]` HFT treasury PDA account  
/// 3. `[]` Clock sysvar
/// 
/// # Arguments:
/// * `program_id` - The program ID for PDA derivation
/// * `accounts` - Array of required accounts
/// 
/// # Returns:
/// * `ProgramResult` - Success or error
pub fn process_get_treasury_info(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("📊 Getting treasury information with consolidation");
    
    // First, consolidate treasuries to get accurate data
    process_consolidate_treasuries(program_id, accounts)?;
    
    // Then load the consolidated main treasury data
    let main_treasury_account = &accounts[0];
    let main_treasury = MainTreasuryState::try_from_slice(&main_treasury_account.data.borrow())?;
    
    msg!("🏦 MAIN TREASURY INFORMATION:");
    msg!("   Authority: {}", main_treasury.authority);
    msg!("   Current Balance: {} lamports ({} SOL)", 
         main_treasury.total_balance, 
         main_treasury.total_balance as f64 / 1_000_000_000.0);
    msg!("   Total Withdrawn: {} lamports ({} SOL)", 
         main_treasury.total_withdrawn,
         main_treasury.total_withdrawn as f64 / 1_000_000_000.0);
    msg!("");
    msg!("📈 FEE STATISTICS:");
    msg!("   Pool Creations: {} (Total fees: {} lamports)", 
         main_treasury.pool_creation_count, main_treasury.total_pool_creation_fees);
    msg!("   Liquidity Operations: {} (Total fees: {} lamports)", 
         main_treasury.liquidity_operation_count, main_treasury.total_liquidity_fees);
    msg!("   Regular Swaps: {} (Total fees: {} lamports)", 
         main_treasury.regular_swap_count, main_treasury.total_regular_swap_fees);
    msg!("   HFT Swaps: {} (Total fees: {} lamports)", 
         main_treasury.hft_swap_count, main_treasury.total_hft_swap_fees);
    msg!("");
    msg!("⏰ CONSOLIDATION:");
    msg!("   Last Consolidation: {}", main_treasury.last_consolidation_timestamp);
    
    Ok(())
}

/// Returns current specialized treasury balances without consolidation.
/// 
/// This view function shows real-time balances in the specialized treasuries
/// without triggering consolidation, useful for monitoring fee flow.
/// 
/// # Required Accounts (in order):
/// 0. `[]` Swap treasury PDA account
/// 1. `[]` HFT treasury PDA account
/// 
/// # Arguments:
/// * `program_id` - The program ID for PDA derivation  
/// * `accounts` - Array of required accounts
/// 
/// # Returns:
/// * `ProgramResult` - Success or error
pub fn process_get_specialized_treasury_balances(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("📊 Getting specialized treasury balances (pre-consolidation)");
    
    let account_info_iter = &mut accounts.iter();
    let swap_treasury_account = next_account_info(account_info_iter)?;
    let hft_treasury_account = next_account_info(account_info_iter)?;
    
    // Verify PDA addresses
    let (expected_swap_treasury, _) = Pubkey::find_program_address(
        &[SWAP_TREASURY_SEED_PREFIX], program_id);
    let (expected_hft_treasury, _) = Pubkey::find_program_address(
        &[HFT_TREASURY_SEED_PREFIX], program_id);
    
    if *swap_treasury_account.key != expected_swap_treasury {
        return Err(ProgramError::InvalidAccountData);
    }
    if *hft_treasury_account.key != expected_hft_treasury {
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Load treasury states
    let swap_treasury = SwapTreasuryState::try_from_slice(&swap_treasury_account.data.borrow())?;
    let hft_treasury = HftTreasuryState::try_from_slice(&hft_treasury_account.data.borrow())?;
    
    // Get current balances
    let swap_balance = swap_treasury_account.lamports();
    let hft_balance = hft_treasury_account.lamports();
    
    msg!("💰 SPECIALIZED TREASURY BALANCES:");
    msg!("");
    msg!("🔄 SWAP TREASURY:");
    msg!("   Current Balance: {} lamports ({} SOL)", 
         swap_balance, swap_balance as f64 / 1_000_000_000.0);
    msg!("   Regular Swaps Processed: {}", swap_treasury.swap_count);
    msg!("   Total Collected: {} lamports", swap_treasury.total_collected);
    msg!("   Last Consolidation: {}", swap_treasury.last_consolidation);
    msg!("");
    msg!("⚡ HFT TREASURY:");
    msg!("   Current Balance: {} lamports ({} SOL)", 
         hft_balance, hft_balance as f64 / 1_000_000_000.0);
    msg!("   HFT Swaps Processed: {}", hft_treasury.hft_swap_count);
    msg!("   Total Collected: {} lamports", hft_treasury.total_collected);
    msg!("   Last Consolidation: {}", hft_treasury.last_consolidation);
    msg!("");
    msg!("📊 TOTALS:");
    msg!("   Combined Balance: {} lamports ({} SOL)", 
         swap_balance + hft_balance, 
         (swap_balance + hft_balance) as f64 / 1_000_000_000.0);
    msg!("   Total Operations: {}", swap_treasury.swap_count + hft_treasury.hft_swap_count);
    
    Ok(())
} 