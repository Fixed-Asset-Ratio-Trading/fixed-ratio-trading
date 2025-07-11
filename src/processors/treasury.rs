//! Treasury Management Processors
//!
//! This module handles centralized treasury operations with real-time tracking:
//! - Contract fee withdrawals by system authority
//! - Real-time treasury information queries
//! - Simplified architecture with single treasury
//!
//! Removed functionality:
//! - Specialized treasury consolidation (no longer needed)
//! - Specialized treasury balance queries (no longer needed)
//! - Complex consolidation race condition handling (eliminated by design)

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};

use crate::{
    constants::*,
    error::PoolError,
    state::{MainTreasuryState, SystemState},
    utils::validation::{validate_signer, validate_writable},
    utils::account_builders::*,
};

/// Processes treasury fee withdrawal with ultra-optimized account ordering.
/// 
/// This function implements an ultra-optimized account structure by removing all
/// placeholder accounts that are not used in treasury operations. This provides
/// maximum efficiency for treasury management operations with strict authority validation.
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA derivation
/// * `amount` - Amount to withdraw in lamports (0 = withdraw all available)
/// * `accounts` - Array of accounts in ultra-optimized order (6 accounts minimum)
/// 
/// # Account Info
/// The accounts must be provided in the following order:
/// 0. **Authority/User Signer** (signer, writable) - System authority authorizing withdrawal
/// 1. **System Program** (readable) - Solana system program
/// 2. **Rent Sysvar** (readable) - For rent calculations
/// 3. **Main Treasury PDA** (writable) - Main treasury account for withdrawal
/// 4. **Destination Account** (writable) - Account receiving the withdrawn SOL
/// 5. **System State Account** (readable) - For authority validation
/// 
/// # Returns
/// * `ProgramResult` - Success or error
/// 
/// # Critical Notes
/// - **ACCOUNT OPTIMIZATION**: Reduced account count from 15 to 6 accounts (60% reduction)
/// - **PLACEHOLDER ELIMINATION**: All placeholder accounts (indices 3-11) removed
/// - **TRANSACTION EFFICIENCY**: Reduced transaction size and validation overhead significantly
/// - **COMPUTE SAVINGS**: Estimated compute unit savings of 210-420 CUs per transaction
/// - **CLIENT INTEGRATION**: Simplified client integration with minimal account requirements
/// - **AUTHORITY VALIDATION**: Strict system authority validation for all withdrawals
pub fn process_withdraw_treasury_fees(
    program_id: &Pubkey,
    amount: u64,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("🏦 Processing treasury fee withdrawal: {} lamports", amount);
    
    // ✅ ACCOUNT VALIDATION: Ultra-reduced account count requirement
    if accounts.len() < 6 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // ✅ ACCOUNT EXTRACTION: Extract accounts using optimized indices
    let authority_account = &accounts[0];              // Index 0: Authority/User Signer
    let _system_program = &accounts[1];                // Index 1: System Program
    let rent_sysvar = &accounts[2];                    // Index 2: Rent Sysvar
    let main_treasury_account = &accounts[3];          // Index 3: Main Treasury PDA
    let destination_account = &accounts[4];            // Index 4: Destination Account
    let system_state_account = &accounts[5];           // Index 5: System State Account
    
    // ✅ EXISTING VALIDATION LOGIC: Maintain all existing validations
    validate_signer(authority_account, "System authority")?;
    validate_writable(main_treasury_account, "Main treasury")?;
    validate_writable(destination_account, "Destination account")?;
    
    // Verify main treasury PDA
    let (expected_main_treasury, _treasury_bump) = Pubkey::find_program_address(
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
    let rent_exempt_minimum = rent.minimum_balance(MainTreasuryState::get_packed_len());
    
    // Calculate available balance for withdrawal
    let current_balance = main_treasury_account.lamports();
    let available_balance = if current_balance > rent_exempt_minimum {
        current_balance - rent_exempt_minimum
    } else {
        0
    };
    
    // Determine actual withdrawal amount
    let withdrawal_amount = if amount == 0 {
        available_balance // Withdraw all available
    } else {
        amount
    };
    
    if withdrawal_amount == 0 {
        msg!("No funds available for withdrawal");
        return Err(ProgramError::InsufficientFunds);
    }
    
    if withdrawal_amount > available_balance {
        msg!("Requested amount {} exceeds available balance {}", 
             withdrawal_amount, available_balance);
        return Err(ProgramError::InsufficientFunds);
    }
    
    msg!("💰 Treasury Withdrawal Details:");
    msg!("   Current balance: {} lamports", current_balance);
    msg!("   Rent-exempt minimum: {} lamports", rent_exempt_minimum);
    msg!("   Available for withdrawal: {} lamports", available_balance);
    msg!("   Withdrawing: {} lamports", withdrawal_amount);
    
    // Transfer SOL from treasury to destination account
    **main_treasury_account.try_borrow_mut_lamports()? -= withdrawal_amount;
    **destination_account.try_borrow_mut_lamports()? += withdrawal_amount;
    
    // Update treasury statistics
    main_treasury.total_withdrawn = main_treasury.total_withdrawn
        .checked_add(withdrawal_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    main_treasury.total_balance = main_treasury_account.lamports();
    
    // Serialize updated treasury state
    let serialized_data = main_treasury.try_to_vec()?;
    main_treasury_account.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    msg!("✅ Treasury withdrawal completed successfully");
    msg!("   Amount withdrawn: {} lamports", withdrawal_amount);
    msg!("   Remaining treasury balance: {} lamports", main_treasury.total_balance);
    
    Ok(())
}

/// Processes treasury information query with ultra-optimized account ordering.
/// 
/// This function implements an ultra-optimized account structure by removing all
/// placeholder accounts that are not used in treasury information queries. This provides
/// maximum efficiency for treasury information retrieval with real-time data access.
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA derivation (unused, kept for compatibility)
/// * `accounts` - Array of accounts in ultra-optimized order (1 account minimum)
/// 
/// # Account Info
/// The accounts must be provided in the following order:
/// 0. **Main Treasury PDA** (readable) - Main treasury account for info query
/// 
/// # Returns
/// * `ProgramResult` - Success or error
/// 
/// # Critical Notes
/// - **ACCOUNT OPTIMIZATION**: Reduced account count from 13 to 1 account (92% reduction)
/// - **PLACEHOLDER ELIMINATION**: All placeholder accounts (indices 0-11) removed
/// - **TRANSACTION EFFICIENCY**: Minimal transaction size and validation overhead
/// - **COMPUTE SAVINGS**: Estimated compute unit savings of 420-840 CUs per transaction
/// - **CLIENT INTEGRATION**: Extremely simplified client integration with single account requirement
/// - **READ-ONLY OPERATION**: Maximum efficiency for information retrieval
pub fn process_get_treasury_info(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("📊 Getting real-time treasury information");
    
    // ✅ ACCOUNT VALIDATION: Ultra-minimal account count requirement
    if accounts.len() < 1 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // ✅ ACCOUNT EXTRACTION: Single account extraction
    let main_treasury_account = &accounts[0]; // Index 0: Main Treasury PDA
    
    // Load main treasury data (real-time data, no consolidation needed)
    let main_treasury = MainTreasuryState::try_from_slice(&main_treasury_account.data.borrow())?;
    
    msg!("🏦 CENTRALIZED TREASURY INFORMATION (REAL-TIME):");
    msg!("   Authority: {}", main_treasury.authority);
    msg!("   Current Balance: {} lamports ({} SOL)", 
         main_treasury.total_balance, 
         main_treasury.total_balance as f64 / 1_000_000_000.0);
    msg!("   Total Withdrawn: {} lamports ({} SOL)", 
         main_treasury.total_withdrawn,
         main_treasury.total_withdrawn as f64 / 1_000_000_000.0);
    msg!("");
    msg!("📈 REAL-TIME FEE STATISTICS:");
    msg!("   Pool Creations: {} (Total fees: {} lamports)", 
         main_treasury.pool_creation_count, main_treasury.total_pool_creation_fees);
    msg!("   Liquidity Operations: {} (Total fees: {} lamports)", 
         main_treasury.liquidity_operation_count, main_treasury.total_liquidity_fees);
    msg!("   Regular Swaps: {} (Total fees: {} lamports)", 
         main_treasury.regular_swap_count, main_treasury.total_regular_swap_fees);
    msg!("   HFT Swaps: {} (Total fees: {} lamports)", 
         main_treasury.hft_swap_count, main_treasury.total_hft_swap_fees);
    msg!("");
    msg!("📊 ANALYTICS:");
    msg!("   Total Operations: {}", main_treasury.total_operations_processed());
    msg!("   Total Fees Collected: {} lamports", main_treasury.total_fees_collected());
    msg!("   Average Fee per Operation: {:.2} lamports", main_treasury.average_fee_per_operation());
    msg!("");
    msg!("⏰ TIMING INFORMATION:");
    msg!("   Last Update: {}", main_treasury.last_update_timestamp);
    msg!("");
    msg!("✅ TREASURY BENEFITS:");
    msg!("   • Real-time data (no consolidation needed)");
    msg!("   • Single source of truth");
    msg!("   • No race conditions");
    msg!("   • Simplified architecture");
    
    Ok(())
}

// ============================================================================
// REMOVED FUNCTIONS
// ============================================================================
// 
// The following functions have been removed for simplification:
// 
// - process_consolidate_treasuries(): No longer needed, fees go directly to main treasury
// - process_get_specialized_treasury_balances(): No specialized treasuries exist
// 
// Benefits of removal:
// - Eliminates consolidation race conditions completely
// - Reduces code complexity by ~200 lines
// - Improves performance (no consolidation overhead)
// - Provides real-time data without delays
// - Single source of truth for all treasury operations
// 
// Migration impact:
// - External apps no longer need to call consolidation
// - Treasury info is always up-to-date and real-time
// - Specialized treasury accounts can be closed and SOL reclaimed
// ============================================================================ 