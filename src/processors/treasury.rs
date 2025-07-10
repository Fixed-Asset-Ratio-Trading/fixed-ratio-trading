//! Treasury Management Processors
//!
//! This module handles all treasury-related operations including:
//! - Contract fee withdrawals by system authority
//! - Consolidation of specialized treasuries into main treasury
//! - Treasury information queries and analytics

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
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
    utils::account_builders::*,
};

/// Processes treasury fee withdrawal with standardized account ordering.
/// 
/// This function implements the standardized account ordering policy defined in
/// ACCOUNT_ORDERING_POLICY.md for treasury management operations. It maintains the same
/// functionality as the original process_withdraw_treasury_fees but uses consistent account positioning.
/// 
/// # Standardized Account Order:
/// 0. **Authority/User Signer** (signer, writable) - System authority authorizing withdrawal
/// 1. **System Program** (readable) - Solana system program
/// 2. **Rent Sysvar** (readable) - For rent calculations
/// 3. **Clock Sysvar** (readable) - For timestamps (placeholder)
/// 4. **Pool State PDA** (writable) - Not used in treasury ops (placeholder)
/// 5. **Token A Mint** (readable) - Not used in treasury ops (placeholder)
/// 6. **Token B Mint** (readable) - Not used in treasury ops (placeholder)
/// 7. **Token A Vault PDA** (writable) - Not used in treasury ops (placeholder)
/// 8. **Token B Vault PDA** (writable) - Not used in treasury ops (placeholder)
/// 9. **SPL Token Program** (readable) - Not used in treasury ops (placeholder)
/// 10. **User Input Token Account** (writable) - Not used in treasury ops (placeholder)
/// 11. **User Output Token Account** (writable) - Not used in treasury ops (placeholder)
/// 12. **Main Treasury PDA** (writable) - Main treasury account for withdrawal
/// 13. **Swap Treasury PDA** (writable) - Not used in main treasury withdrawal (placeholder)
/// 14. **HFT Treasury PDA** (writable) - Not used in main treasury withdrawal (placeholder)
/// 15. **Destination Account** (writable) - Account receiving the withdrawn SOL (function-specific)
/// 16. **System State Account** (readable) - For authority validation (function-specific)
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA derivation
/// * `amount` - Amount to withdraw in lamports (0 = withdraw all available)
/// * `accounts` - Array of accounts in standardized order (17 accounts minimum)
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_withdraw_treasury_fees(
    program_id: &Pubkey,
    amount: u64,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("🏦 Processing treasury fee withdrawal: {} lamports", amount);
    
    // ✅ STANDARDIZED ACCOUNT VALIDATION: Validate standard account positions where applicable
    validate_standard_accounts(accounts)?;
    // Note: Most pool/token accounts are placeholders for treasury operations
    validate_treasury_accounts(accounts)?;
    
    // Validate we have enough accounts for treasury-specific operations
    if accounts.len() < 17 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // ✅ STANDARDIZED ACCOUNT EXTRACTION: Extract accounts using standardized indices
    let authority_account = &accounts[0];              // Index 0: Authority/User Signer
    let _system_program = &accounts[1];                // Index 1: System Program
    let rent_sysvar = &accounts[2];                    // Index 2: Rent Sysvar
    let _clock_sysvar = &accounts[3];                  // Index 3: Clock Sysvar (unused)
    // Indices 4-11: Pool/token accounts (unused placeholders)
    let main_treasury_account = &accounts[12];         // Index 12: Main Treasury PDA
    let _swap_treasury_account = &accounts[13];        // Index 13: Swap Treasury PDA (unused)
    let _hft_treasury_account = &accounts[14];         // Index 14: HFT Treasury PDA (unused)
    
    // ✅ FUNCTION-SPECIFIC ACCOUNTS: Treasury-specific accounts at standardized positions 15+
    let destination_account = &accounts[15];           // Index 15: Destination Account
    let system_state_account = &accounts[16];          // Index 16: System State Account
    
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

/// Processes treasury consolidation with standardized account ordering.
/// 
/// **🔒 PHASE 2 SECURITY**: This function now REQUIRES the system to be paused before
/// consolidation can proceed. This prevents race conditions by ensuring all user
/// operations (swaps, pool creation, liquidity) are blocked during consolidation.
/// 
/// This function consolidates specialized treasuries into the main treasury.
/// It empties the specialized swap and HFT treasuries, transferring their
/// balances and statistics to the main treasury.
/// 
/// # Security Requirements
/// - **System MUST be paused** - consolidation will fail if system is not paused
/// - **Authority validation** - only contract creator can consolidate (via system state)
/// - **Race condition prevention** - paused system blocks all fee-generating operations
/// 
/// # Standardized Account Order:
/// 0. **Authority/User Signer** (signer, writable) - Contract creator authorizing consolidation
/// 1. **System Program** (readable) - Not used in consolidation (placeholder)
/// 2. **Rent Sysvar** (readable) - Not used in consolidation (placeholder)
/// 3. **Clock Sysvar** (readable) - For timestamp operations
/// 4. **Pool State PDA** (writable) - Not used in treasury ops (placeholder)
/// 5. **Token A Mint** (readable) - Not used in treasury ops (placeholder)
/// 6. **Token B Mint** (readable) - Not used in treasury ops (placeholder)
/// 7. **Token A Vault PDA** (writable) - Not used in treasury ops (placeholder)
/// 8. **Token B Vault PDA** (writable) - Not used in treasury ops (placeholder)
/// 9. **SPL Token Program** (readable) - Not used in treasury ops (placeholder)
/// 10. **User Input Token Account** (writable) - Not used in treasury ops (placeholder)
/// 11. **User Output Token Account** (writable) - Not used in treasury ops (placeholder)
/// 12. **Main Treasury PDA** (writable) - Main treasury account for consolidation
/// 13. **Swap Treasury PDA** (writable) - Swap treasury to consolidate from
/// 14. **HFT Treasury PDA** (writable) - HFT treasury to consolidate from
/// 15. **System State PDA** (readable) - For pause validation and authority control (function-specific)
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA derivation
/// * `accounts` - Array of accounts in standardized order (16 accounts minimum)
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_consolidate_treasuries(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("🔄 Processing treasury consolidation with Phase 2 security");
    
    // ✅ STANDARDIZED ACCOUNT VALIDATION: Validate standard account positions where applicable
    validate_standard_accounts(accounts)?;
    // Note: Most pool/token accounts are placeholders for treasury operations
    validate_treasury_accounts(accounts)?;
    
    // Validate we have enough accounts for treasury consolidation (now requires system state)
    if accounts.len() < 16 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // ✅ STANDARDIZED ACCOUNT EXTRACTION: Extract accounts using standardized indices
    let authority_account = &accounts[0];              // Index 0: Authority/User Signer
    // Indices 1-2: System accounts (unused placeholders for consolidation)
    let clock_sysvar = &accounts[3];                   // Index 3: Clock Sysvar
    // Indices 4-11: Pool/token accounts (unused placeholders)
    let main_treasury_account = &accounts[12];         // Index 12: Main Treasury PDA
    let swap_treasury_account = &accounts[13];         // Index 13: Swap Treasury PDA
    let hft_treasury_account = &accounts[14];          // Index 14: HFT Treasury PDA
    
    // ✅ FUNCTION-SPECIFIC ACCOUNTS: Phase 2 security accounts at standardized positions 15+
    let system_state_account = &accounts[15];          // Index 15: System State PDA
    
    // ✅ PHASE 2 SECURITY VALIDATION: System pause and authority requirements
    validate_signer(authority_account, "Contract creator")?;
    
    // 1. Load and validate system state
    let system_state = SystemState::try_from_slice(&system_state_account.data.borrow())?;
    
    // 2. Validate contract creator authority
    if !system_state.validate_authority(authority_account.key) {
        msg!("🚨 Unauthorized consolidation attempt");
        msg!("Provided authority: {}", authority_account.key);
        msg!("Expected authority: {}", system_state.authority);
        return Err(PoolError::UnauthorizedAccess.into());
    }
    
    // 3. CRITICAL: Require system to be paused before consolidation
    if !system_state.is_paused {
        msg!("🚨 CONSOLIDATION BLOCKED: System must be paused before consolidation");
        msg!("Current state: is_paused = false");
        msg!("Required: System must be paused to prevent race conditions");
        msg!("💡 Solution: Use process_pause_system() first, then consolidate");
        return Err(PoolError::SystemNotPaused.into());
    }
    
    msg!("✅ Phase 2 Security Validation Passed:");
    msg!("   • System is paused (race condition prevention active)");
    msg!("   • Authority validated: {}", authority_account.key);
    msg!("   • Pause reason: {}", system_state.pause_reason);
    msg!("   • All user operations blocked during consolidation");
    
    // ✅ EXISTING VALIDATION LOGIC: Maintain all existing validations
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
    
    msg!("💰 Consolidation Details:");
    msg!("   Swap treasury balance: {} lamports ({} SOL)", 
         swap_balance, swap_balance as f64 / 1_000_000_000.0);
    msg!("   HFT treasury balance: {} lamports ({} SOL)", 
         hft_balance, hft_balance as f64 / 1_000_000_000.0);
    msg!("   Total to consolidate: {} lamports ({} SOL)", 
         total_to_transfer, total_to_transfer as f64 / 1_000_000_000.0);
    
    if total_to_transfer > 0 {
        // Transfer SOL from specialized treasuries to main treasury
        **swap_treasury_account.try_borrow_mut_lamports()? = 0;
        **hft_treasury_account.try_borrow_mut_lamports()? = 0;
        **main_treasury_account.try_borrow_mut_lamports()? += total_to_transfer;
        
        // Update main treasury statistics
        main_treasury.total_balance = main_treasury_account.lamports();
        main_treasury.regular_swap_count += swap_treasury.swap_count;
        main_treasury.hft_swap_count += hft_treasury.hft_swap_count;
        main_treasury.total_regular_swap_fees += swap_treasury.total_collected;
        main_treasury.total_hft_swap_fees += hft_treasury.total_collected;
        main_treasury.last_consolidation_timestamp = current_timestamp;
        
        msg!("✅ Consolidated {} lamports from specialized treasuries", total_to_transfer);
        
        // Reset specialized treasury statistics
        swap_treasury.swap_count = 0;
        swap_treasury.total_collected = 0;
        swap_treasury.last_consolidation = current_timestamp;
        
        hft_treasury.hft_swap_count = 0;
        hft_treasury.total_collected = 0;
        hft_treasury.last_consolidation = current_timestamp;
    } else {
        msg!("⚠️ No funds to consolidate");
    }
    
    // Serialize updated treasury states
    let main_serialized = main_treasury.try_to_vec()?;
    main_treasury_account.data.borrow_mut()[..main_serialized.len()].copy_from_slice(&main_serialized);
    
    let swap_serialized = swap_treasury.try_to_vec()?;
    swap_treasury_account.data.borrow_mut()[..swap_serialized.len()].copy_from_slice(&swap_serialized);
    
    let hft_serialized = hft_treasury.try_to_vec()?;
    hft_treasury_account.data.borrow_mut()[..hft_serialized.len()].copy_from_slice(&hft_serialized);
    
    msg!("✅ Treasury consolidation completed successfully");
    msg!("   Main treasury balance: {} lamports", main_treasury.total_balance);
    
    Ok(())
}

/// Processes treasury information queries with standardized account ordering.
/// 
/// **⚠️ PHASE 2 CHANGE**: This function NO LONGER automatically consolidates treasuries.
/// Due to Phase 2 security requirements, consolidation now requires system pause and
/// authority validation. This function now returns current treasury information without
/// automatic consolidation.
/// 
/// For consolidated data, use the dedicated consolidation workflow:
/// 1. Pause system via `process_pause_system()`
/// 2. Consolidate via `process_consolidate_treasuries()`
/// 3. Query info via this function
/// 4. Unpause system via `process_unpause_system()`
/// 
/// # Standardized Account Order:
/// 0. **Authority/User Signer** (signer, writable) - Not required for info query (placeholder)
/// 1. **System Program** (readable) - Not used in info query (placeholder)
/// 2. **Rent Sysvar** (readable) - Not used in info query (placeholder)
/// 3. **Clock Sysvar** (readable) - Not used in info query (placeholder)
/// 4. **Pool State PDA** (writable) - Not used in treasury ops (placeholder)
/// 5. **Token A Mint** (readable) - Not used in treasury ops (placeholder)
/// 6. **Token B Mint** (readable) - Not used in treasury ops (placeholder)
/// 7. **Token A Vault PDA** (writable) - Not used in treasury ops (placeholder)
/// 8. **Token B Vault PDA** (writable) - Not used in treasury ops (placeholder)
/// 9. **SPL Token Program** (readable) - Not used in treasury ops (placeholder)
/// 10. **User Input Token Account** (writable) - Not used in treasury ops (placeholder)
/// 11. **User Output Token Account** (writable) - Not used in treasury ops (placeholder)
/// 12. **Main Treasury PDA** (writable) - Main treasury account for info query
/// 13. **Swap Treasury PDA** (writable) - Not used in info query (placeholder)
/// 14. **HFT Treasury PDA** (writable) - Not used in info query (placeholder)
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA derivation
/// * `accounts` - Array of accounts in standardized order (15 accounts minimum)
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_get_treasury_info(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("📊 Getting treasury information (Phase 2: no automatic consolidation)");
    
    // ✅ STANDARDIZED ACCOUNT VALIDATION: Validate standard account positions where applicable
    validate_standard_accounts(accounts)?;
    // Note: Most pool/token accounts are placeholders for treasury operations
    validate_treasury_accounts(accounts)?;
    
    // Validate we have enough accounts for treasury info query
    if accounts.len() < 15 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // Load main treasury data (no automatic consolidation due to Phase 2 security)
    let main_treasury_account = &accounts[12]; // Index 12: Main Treasury PDA
    let main_treasury = MainTreasuryState::try_from_slice(&main_treasury_account.data.borrow())?;
    
    msg!("⚠️ NOTE: This shows current main treasury data without automatic consolidation");
    msg!("For consolidated data, use: pause → consolidate → query → unpause workflow");
    
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
    msg!("⏰ TIMING INFORMATION:");
    msg!("   Last Consolidation: {}", main_treasury.last_consolidation_timestamp);
    
    Ok(())
}

/// Processes specialized treasury balance queries with standardized account ordering.
/// 
/// This function returns current specialized treasury balances without consolidation.
/// It provides a pre-consolidation view of the specialized treasury accounts,
/// useful for monitoring fee flow.
/// 
/// # Standardized Account Order:
/// 0. **Authority/User Signer** (signer, writable) - Not required for balance query (placeholder)
/// 1. **System Program** (readable) - Not used in balance query (placeholder)
/// 2. **Rent Sysvar** (readable) - Not used in balance query (placeholder)
/// 3. **Clock Sysvar** (readable) - Not used in balance query (placeholder)
/// 4. **Pool State PDA** (writable) - Not used in treasury ops (placeholder)
/// 5. **Token A Mint** (readable) - Not used in treasury ops (placeholder)
/// 6. **Token B Mint** (readable) - Not used in treasury ops (placeholder)
/// 7. **Token A Vault PDA** (writable) - Not used in treasury ops (placeholder)
/// 8. **Token B Vault PDA** (writable) - Not used in treasury ops (placeholder)
/// 9. **SPL Token Program** (readable) - Not used in treasury ops (placeholder)
/// 10. **User Input Token Account** (writable) - Not used in treasury ops (placeholder)
/// 11. **User Output Token Account** (writable) - Not used in treasury ops (placeholder)
/// 12. **Main Treasury PDA** (writable) - Not used in specialized balance query (placeholder)
/// 13. **Swap Treasury PDA** (writable) - Swap treasury for balance query
/// 14. **HFT Treasury PDA** (writable) - HFT treasury for balance query
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA derivation
/// * `accounts` - Array of accounts in standardized order (15 accounts minimum)
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_get_specialized_treasury_balances(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("📊 Getting specialized treasury balances (pre-consolidation)");
    
    // ✅ STANDARDIZED ACCOUNT VALIDATION: Validate standard account positions where applicable
    validate_standard_accounts(accounts)?;
    // Note: Most pool/token accounts are placeholders for treasury operations
    validate_treasury_accounts(accounts)?;
    
    // Validate we have enough accounts for treasury balance query
    if accounts.len() < 15 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // ✅ STANDARDIZED ACCOUNT EXTRACTION: Extract accounts using standardized indices
    // Indices 0-12: System/pool/main treasury accounts (unused placeholders)
    let swap_treasury_account = &accounts[13];         // Index 13: Swap Treasury PDA
    let hft_treasury_account = &accounts[14];          // Index 14: HFT Treasury PDA
    
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
    msg!("📊 TOTAL UNCONSOLIDATED: {} lamports ({} SOL)", 
         swap_balance + hft_balance, 
         (swap_balance + hft_balance) as f64 / 1_000_000_000.0);
    
    Ok(())
} 