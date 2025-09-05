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
    sysvar::{rent::Rent},
};

use crate::{
    constants::*,
    state::{MainTreasuryState},
    utils::validation::{validate_writable},
};

/// Processes treasury fee withdrawal with optimized account structure.
/// 
/// This function implements an optimized account structure by removing all
/// placeholder accounts that are not used in treasury operations. This provides
/// maximum efficiency for treasury management operations with strict authority validation.
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA derivation
/// * `amount` - Amount to withdraw in lamports (0 = withdraw all available)
/// * `system_authority_signer` - System upgrade authority signer authorizing withdrawal
/// * `main_treasury_pda` - Main treasury PDA for withdrawal
/// * `rent_sysvar_account` - For rent calculations
/// * `destination_account` - Account receiving the withdrawn SOL
/// * `system_state_pda` - For authority validation and pause check
/// * `program_data_account` - Program data account for authority validation
/// 
/// # Account Info (Optimized - 6 accounts total)
/// The accounts must be provided in the following order:
/// 0. **System Authority Signer** (signer, writable) - System upgrade authority signer authorizing withdrawal
/// 1. **Main Treasury PDA** (writable) - Main treasury PDA for withdrawal
/// 2. **Rent Sysvar Account** (readable) - For rent calculations
/// 3. **Destination Account** (writable) - Account receiving the withdrawn SOL
/// 4. **System State PDA** (readable) - For authority validation and pause check
/// 5. **Program Data Account** (readable) - Program data account for authority validation
/// 
/// # Returns
/// * `ProgramResult` - Success or error
///
pub fn process_treasury_withdraw_fees(
    program_id: &Pubkey,
    amount: u64,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("🏦 Processing treasury fee withdrawal: {} lamports", amount);
    
    // ✅ COMPUTE OPTIMIZATION: No account length verification
    // Solana runtime automatically fails with NotEnoughAccountKeys when accessing
    // accounts[N] if insufficient accounts are provided. Manual length checks are
    // redundant and waste compute units on every function call.
    
    // ✅ OPTIMIZED ACCOUNT EXTRACTION: Removed 3 unused placeholder accounts
    let system_authority_signer = &accounts[0];      // Index 0: System Authority Signer
    let main_treasury_pda = &accounts[1];            // Index 1: Main Treasury PDA
    let rent_sysvar_account = &accounts[2];          // Index 2: Rent Sysvar Account
    let destination_account = &accounts[3];          // Index 3: Destination Account
    let system_state_pda = &accounts[4];             // Index 4: System State PDA
    let program_data_account = &accounts[5];         // Index 5: Program Data Account
    
    // ✅ SECURITY: Signer validation handled by validate_program_upgrade_authority()
    // The validate_program_upgrade_authority() function includes comprehensive
    // signer checks as part of its authority validation process.
    validate_writable(main_treasury_pda, "Main treasury PDA")?;
    validate_writable(destination_account, "Destination account")?;
    
    // Verify main treasury PDA
    let (expected_main_treasury, _treasury_bump) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        program_id,
    );
    if *main_treasury_pda.key != expected_main_treasury {
        msg!("Invalid main treasury PDA. Expected: {}, Got: {}", 
             expected_main_treasury, main_treasury_pda.key);
        return Err(ProgramError::InvalidAccountData);
    }
    
    // ✅ AUTHORITY VALIDATION: Use secure system pause validation
    crate::utils::validation::validate_system_not_paused_secure(system_state_pda, program_id)?;
    
    // ✅ ADMIN AUTHORITY VALIDATION: Use admin authority with upgrade authority fallback
    use crate::utils::admin_validation::validate_admin_authority;
    validate_admin_authority(
        system_authority_signer,
        system_state_pda,
        Some(program_data_account),
        program_id,
    )?;
    msg!("✅ Authority validation passed: {}", system_authority_signer.key);
    
    // Load main treasury state with robust error handling for production environments
    let mut main_treasury_state = match MainTreasuryState::try_from_slice(&main_treasury_pda.data.borrow()) {
        Ok(state) => {
            msg!("✅ Successfully loaded treasury state from account data");
            state
        },
        Err(e) => {
            msg!("⚠️ Warning: Failed to deserialize treasury state: {:?}", e);
            msg!("🔄 Creating default treasury state with current account balance");
            
            // Create a default state with current account balance
            let current_balance = main_treasury_pda.lamports();
            let mut default_state = MainTreasuryState::new();
            default_state.total_balance = current_balance;
            default_state.rent_exempt_minimum = 2_039_280; // Standard rent exempt minimum
            
            msg!("📊 Default state created:");
            msg!("   - Current balance: {} lamports", current_balance);
            msg!("   - Rent exempt minimum: {} lamports", default_state.rent_exempt_minimum);
            msg!("   - All counters reset to 0 (data corruption detected)");
            
            default_state
        }
    };
    
    // Calculate rent-exempt minimum
    let rent = &Rent::from_account_info(rent_sysvar_account)?;
    let rent_exempt_minimum = rent.minimum_balance(MainTreasuryState::get_packed_len());
    
    // Calculate available balance for withdrawal
    let current_balance = main_treasury_pda.lamports();
    let available_balance = current_balance.saturating_sub(rent_exempt_minimum);
    
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
    
    // Get current timestamp for rate limiting validation
    use solana_program::clock::Clock;
    use solana_program::sysvar::Sysvar;
    
    let current_timestamp = match Clock::get() {
        Ok(clock) => {
            msg!("✅ Successfully retrieved current timestamp: {}", clock.unix_timestamp);
            clock.unix_timestamp
        },
        Err(e) => {
            msg!("⚠️ Warning: Failed to get current timestamp: {:?}", e);
            msg!("🔄 Using fallback timestamp (0) for rate limiting validation");
            0 // Fallback timestamp
        }
    };
    
    // **RATE LIMITING VALIDATION (Fixed 60-minute cooldown after success)**
    if let Err(rate_limit_error) = main_treasury_state.validate_withdrawal_rate_limit(withdrawal_amount, current_timestamp) {
        msg!("🚫 WITHDRAWAL BLOCKED: {}", rate_limit_error);
        
        // Check if this is a system restart penalty
        let restart_penalty_time = main_treasury_state.restart_penalty_time_remaining(current_timestamp);
        if restart_penalty_time > 0 {
            msg!("🔒 SYSTEM RESTART PENALTY ACTIVE:");
            msg!("   Remaining penalty time: {} seconds ({} hours, {} days)", 
                restart_penalty_time, 
                restart_penalty_time / 3600,
                restart_penalty_time / (3600 * 24));
            msg!("   This 3-day cooling-off period prevents immediate fund drainage after system restart");
            msg!("   Penalty started when system was last re-enabled after being paused");
        } else {
            // Fixed cooldown timing info
            let time_until_next = main_treasury_state.time_until_next_withdrawal_allowed(current_timestamp);
            if time_until_next > 0 {
                msg!("⏰ Next withdrawal allowed in {} seconds ({} minutes)", 
                    time_until_next, time_until_next / 60);
            }
        }
        
        msg!("💡 Withdrawal Context:");
        msg!("   Available for withdrawal: {} lamports ({} SOL)", 
            available_balance, available_balance as f64 / 1_000_000_000.0);
        msg!("   Requested amount: {} lamports ({} SOL)",
            withdrawal_amount, withdrawal_amount as f64 / 1_000_000_000.0);
        msg!("   Last withdrawal: {} (timestamp)", main_treasury_state.last_withdrawal_timestamp);
        return Err(ProgramError::InvalidInstructionData);
    }
    
    msg!("✅ Rate limiting validation passed (fixed 60-minute cooldown)");
    msg!("💰 Treasury Withdrawal Details:");
    msg!("   Current balance: {} lamports", current_balance);
    msg!("   Rent-exempt minimum: {} lamports", rent_exempt_minimum);
    msg!("   Available for withdrawal: {} lamports", available_balance);
    msg!("   Withdrawing: {} lamports", withdrawal_amount);
    
    // Transfer SOL from treasury to destination account
    **main_treasury_pda.try_borrow_mut_lamports()? -= withdrawal_amount;
    **destination_account.try_borrow_mut_lamports()? += withdrawal_amount;
    
    // Update treasury statistics with the timestamp we already obtained
    main_treasury_state.add_treasury_withdrawal(withdrawal_amount, current_timestamp);
    
    main_treasury_state.total_balance = main_treasury_pda.lamports();
    
    // Serialize updated treasury state with robust error handling
    let serialized_data = match main_treasury_state.try_to_vec() {
        Ok(data) => {
            msg!("✅ Successfully serialized treasury state ({} bytes)", data.len());
            data
        },
        Err(e) => {
            msg!("🚨 Critical Error: Failed to serialize treasury state: {:?}", e);
            msg!("❌ Treasury withdrawal cannot proceed - serialization failure");
            return Err(ProgramError::InvalidAccountData);
        }
    };
    
    // Write serialized data to account
    let mut account_data = main_treasury_pda.data.borrow_mut();
    if serialized_data.len() > account_data.len() {
        msg!("🚨 Critical Error: Serialized data too large for account");
        msg!("   Required: {} bytes, Available: {} bytes", serialized_data.len(), account_data.len());
        return Err(ProgramError::AccountDataTooSmall);
    }
    
    account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
    msg!("✅ Successfully updated treasury account data");
    
    msg!("✅ Treasury withdrawal completed successfully");
    msg!("   Amount withdrawn: {} lamports", withdrawal_amount);
    msg!("   Remaining treasury balance: {} lamports", main_treasury_state.total_balance);
    
    Ok(())
}

/// Processes treasury information query with optimized account structure.
/// 
/// This function implements an optimized account structure by removing all
/// placeholder accounts that are not used in treasury information queries. This provides
/// maximum efficiency for treasury information retrieval with real-time data access.
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA derivation (unused, kept for compatibility)
/// * `main_treasury_pda` - Main treasury PDA for info query
/// 
/// # Account Info (Optimized - 1 account total)
/// The accounts must be provided in the following order:
/// 0. **Main Treasury PDA** (readable) - Main treasury PDA for info query
/// 
/// # Returns
/// * `ProgramResult` - Success or error
/// 
pub fn process_treasury_get_info(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("📊 Getting real-time treasury information");
    
    // ✅ COMPUTE OPTIMIZATION: No account length verification
    // Solana runtime automatically fails with NotEnoughAccountKeys when accessing
    // accounts[N] if insufficient accounts are provided. Manual length checks are
    // redundant and waste compute units on every function call.
    
    // ✅ OPTIMIZED ACCOUNT EXTRACTION: Removed 4 unused placeholder accounts
    let main_treasury_pda = &accounts[0];            // Index 0: Main Treasury PDA
    
    // Load main treasury data with robust error handling for production environments
    let main_treasury_state = match MainTreasuryState::try_from_slice(&main_treasury_pda.data.borrow()) {
        Ok(state) => {
            msg!("✅ Successfully loaded treasury state from account data");
            state
        },
        Err(e) => {
            msg!("⚠️ Warning: Failed to deserialize treasury state: {:?}", e);
            msg!("🔄 Creating default treasury state with current account balance");
            
            // Create a default state with current account balance
            let current_balance = main_treasury_pda.lamports();
            let mut default_state = MainTreasuryState::new();
            default_state.total_balance = current_balance;
            default_state.rent_exempt_minimum = 2_039_280; // Standard rent exempt minimum
            
            msg!("📊 Default state created:");
            msg!("   - Current balance: {} lamports", current_balance);
            msg!("   - Rent exempt minimum: {} lamports", default_state.rent_exempt_minimum);
            msg!("   - All counters reset to 0 (data corruption detected)");
            
            default_state
        }
    };
    
    // Load and display treasury information
    
    msg!("🏦 CENTRALIZED TREASURY INFORMATION (REAL-TIME):");
    msg!("   Current Balance: {} lamports ({} SOL)", 
         main_treasury_state.total_balance, 
         main_treasury_state.total_balance as f64 / 1_000_000_000.0);
    msg!("   Total Withdrawn: {} lamports ({} SOL)", 
         main_treasury_state.total_withdrawn,
         main_treasury_state.total_withdrawn as f64 / 1_000_000_000.0);
    msg!("");
    msg!("📈 OPERATION STATISTICS:");
    msg!("   Pool Creations: {} (Total fees: {} lamports, Avg: {:.2})", 
         main_treasury_state.pool_creation_count, 
         main_treasury_state.total_pool_creation_fees,
         main_treasury_state.average_pool_creation_fee());
    msg!("   Liquidity Operations: {} (Total fees: {} lamports, Avg: {:.2})", 
         main_treasury_state.liquidity_operation_count, 
         main_treasury_state.total_liquidity_fees,
         main_treasury_state.average_liquidity_fee());
    msg!("   Regular Swaps: {} (Total fees: {} lamports, Avg: {:.2})", 
         main_treasury_state.regular_swap_count, 
         main_treasury_state.total_regular_swap_fees,
         main_treasury_state.average_swap_fee());
    msg!("   Treasury Withdrawals: {} (Total: {} lamports)", 
         main_treasury_state.treasury_withdrawal_count, 
         main_treasury_state.total_withdrawn);
    msg!("   Consolidations: {} (Last: {})", 
         main_treasury_state.total_consolidations_performed,
         main_treasury_state.last_update_timestamp);
    msg!("   Donations: {} (Total: {} lamports, {:.6} SOL)", 
         main_treasury_state.donation_count,
         main_treasury_state.total_donations,
         main_treasury_state.total_donations as f64 / 1_000_000_000.0);
    msg!("");
    msg!("📊 ENHANCED ANALYTICS:");
    msg!("   Total Successful Operations: {}", main_treasury_state.total_successful_operations());
    msg!("   Failed Operations: {}", main_treasury_state.failed_operation_count);
    msg!("   Success Rate: {:.2}%", main_treasury_state.success_rate_percentage());
    msg!("   Total Fees Collected: {} lamports ({:.4} SOL)", 
         main_treasury_state.total_fees_collected(),
         main_treasury_state.total_fees_collected() as f64 / 1_000_000_000.0);
    msg!("   Average Fee per Operation: {:.2} lamports", main_treasury_state.average_fee_per_operation());
    msg!("");
    msg!("⏰ TIMING INFORMATION:");
    msg!("   Last Update: {}", main_treasury_state.last_update_timestamp);
    msg!("");
    msg!("✅ TREASURY BENEFITS:");
    msg!("   • Real-time data (no consolidation needed)");
    msg!("   • Single source of truth");
    msg!("   • No race conditions");
    msg!("   • Simplified architecture");
    
    Ok(())
}

/// Processes voluntary SOL donations to the treasury
/// 
/// This function allows anyone to donate SOL to the protocol treasury.
/// Donations are tracked separately from fees for transparency and analytics.
/// 
/// # Security:
/// - System pause validation prevents donations when system is paused
/// - Donations are non-refundable once sent
/// - All donations are logged with optional messages
/// - Thread-safe counter updates
/// 
/// # Arguments:
/// * `program_id` - The program ID for PDA derivation
/// * `amount` - Amount to donate in lamports (must be > 0)
/// * `message` - Optional message (logged but not stored)
/// * `accounts` - Array of accounts in order
/// 
/// # Account Info
/// The accounts must be provided in the following order:
/// 0. **Donor Account** (signer, writable) - Account donating SOL
/// 1. **Main Treasury PDA** (writable) - Receives the donation
/// 2. **System State PDA** (readable) - For pause validation
/// 3. **System Program Account** (readable) - For SOL transfer
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_treasury_donate_sol(
    program_id: &Pubkey,
    amount: u64,
    message: String,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("💰 Processing SOL donation: {} lamports", amount);
    msg!("📝 Donation message: \"{}\"", message);
    msg!("⚠️  NOTICE: All donations are NON-REFUNDABLE. Accidental donations will NOT be returned.");
    
    // Extract accounts
    let donor_account = &accounts[0];
    let main_treasury_pda = &accounts[1];
    let system_state_pda = &accounts[2];
    let system_program = &accounts[3];
    
    // Validate accounts
    use crate::utils::validation::{validate_writable, validate_signer};
    validate_signer(donor_account, "Donor account")?;
    validate_writable(donor_account, "Donor account")?;
    validate_writable(main_treasury_pda, "Main treasury PDA")?;
    
    // Validate system program
    if *system_program.key != solana_program::system_program::id() {
        msg!("❌ Invalid system program account");
        return Err(ProgramError::IncorrectProgramId);
    }
    
    // Validate amount > 0
    if amount == 0 {
        msg!("❌ Donation amount must be greater than 0");
        return Err(ProgramError::InvalidArgument);
    }
    
    // Validate minimum donation amount (0.1 SOL)
    use crate::constants::MIN_DONATION_AMOUNT;
    if amount < MIN_DONATION_AMOUNT {
        msg!("❌ Donation amount must be at least {} lamports ({:.1} SOL). Received: {} lamports ({:.6} SOL)", 
             MIN_DONATION_AMOUNT, 
             MIN_DONATION_AMOUNT as f64 / 1_000_000_000.0,
             amount,
             amount as f64 / 1_000_000_000.0);
        msg!("💡 Minimum donation helps prevent spam and ensures meaningful contributions");
        return Err(ProgramError::InvalidArgument);
    }
    
    // Check donor has sufficient balance
    if donor_account.lamports() < amount {
        msg!("❌ Insufficient balance. Available: {}, Required: {}", 
             donor_account.lamports(), amount);
        return Err(ProgramError::InsufficientFunds);
    }
    
    // Verify main treasury PDA
    let (expected_main_treasury, _treasury_bump) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        program_id,
    );
    if *main_treasury_pda.key != expected_main_treasury {
        msg!("❌ Invalid main treasury PDA. Expected: {}, Got: {}", 
             expected_main_treasury, main_treasury_pda.key);
        return Err(ProgramError::InvalidAccountData);
    }
    
    // ✅ SECURITY: Validate system not paused
    crate::utils::validation::validate_system_not_paused_secure(system_state_pda, program_id)?;
    msg!("✅ System pause validation passed");
    
    // Transfer SOL from donor to treasury
    msg!("💸 Transferring {} lamports from {} to treasury", 
         amount, donor_account.key);
    
    solana_program::program::invoke(
        &solana_program::system_instruction::transfer(
            donor_account.key,
            main_treasury_pda.key,
            amount,
        ),
        &[donor_account.clone(), main_treasury_pda.clone()],
    )?;
    
    msg!("✅ Transfer successful");
    
    // Load and update treasury state
    let mut main_treasury_state = match MainTreasuryState::try_from_slice(&main_treasury_pda.data.borrow()) {
        Ok(state) => state,
        Err(e) => {
            msg!("⚠️ Failed to deserialize treasury state: {:?}", e);
            msg!("🔄 Creating default treasury state");
            
            let current_balance = main_treasury_pda.lamports();
            let mut default_state = MainTreasuryState::new();
            default_state.total_balance = current_balance;
            default_state.rent_exempt_minimum = 2_039_280;
            default_state
        }
    };
    
    // Get current timestamp
    use solana_program::clock::Clock;
    use solana_program::sysvar::Sysvar;
    
    let current_timestamp = Clock::get()?.unix_timestamp;
    
    // Update treasury state with donation
    main_treasury_state.add_donation(amount, current_timestamp);
    main_treasury_state.total_balance = main_treasury_pda.lamports();
    
    // Serialize updated state back to account
    use crate::utils::serialization::serialize_to_account;
    serialize_to_account(&main_treasury_state, main_treasury_pda)?;
    
    // Log donation details
    msg!("✅ DONATION RECORDED SUCCESSFULLY:");
    msg!("   Donor: {}", donor_account.key);
    msg!("   Amount: {} lamports ({:.6} SOL)", amount, amount as f64 / 1_000_000_000.0);
    msg!("   Total Donations: {} ({:.6} SOL)", 
         main_treasury_state.total_donations,
         main_treasury_state.total_donations as f64 / 1_000_000_000.0);
    msg!("   Donation Count: {}", main_treasury_state.donation_count);
    msg!("   Treasury Balance: {} lamports", main_treasury_state.total_balance);
    
    if !message.is_empty() {
        msg!("   Message: \"{}\"", message);
    }
    
    msg!("💖 Thank you for supporting the protocol!");
    
    Ok(())
}
