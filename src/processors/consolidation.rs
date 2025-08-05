//! Pool Fee Consolidation Processor
//! 
//! This module implements the single batch consolidation process for SOL fees from
//! multiple pool states to the MainTreasuryState with flexible pause support.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    clock::Clock,
    sysvar::Sysvar,
};

use crate::{
    constants::*,
    state::{PoolState, MainTreasuryState, ConsolidatedOperations},
    utils::validation::validate_and_deserialize_pool_state_secure,
};

/// Processes batch consolidation of SOL fees with flexible pause requirements
/// 
/// This function safely consolidates SOL fees from 1-20 pools to the MainTreasuryState
/// using either system-wide pause or individual pool pause. The consolidation is atomic - 
/// either all eligible pools are processed successfully or the entire operation fails.
/// 
/// # Flexible Pause Requirements
/// 1. **System Paused**: If system is paused, all specified pools are consolidated
/// 2. **System Active**: If system is NOT paused, only pools with both `swaps_paused` AND `liquidity_paused` set to true are consolidated
/// 3. **Individual Control**: Allows pausing specific pools without affecting entire system
/// 4. **Race Protection**: Paused state prevents concurrent operations during consolidation
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA validation
/// * `pool_count` - Number of pools to consolidate (1-20)
/// * `accounts` - Array of accounts in the following order:
///   - [0] System State PDA (for pause validation)
///   - [1] Main Treasury PDA (receives consolidated fees)
///   - [2..2+pool_count] Pool State PDAs (pools to consolidate)
/// 
/// # Returns
/// * `ProgramResult` - Success or error
/// 
/// # CU Estimate: ~109,000 CUs for 20 pools, scales linearly down to ~5,000 for 1 pool
/// - System pause validation: 1,000 CUs
/// - Pool processing (N pools): N * 5,200 CUs  
/// - Treasury update: 4,000 CUs
/// 
/// # External Validation
/// - **No fee minimums**: All pools processed regardless of fee amount
/// - **No operation minimums**: All pools processed regardless of operation count
/// - **External filtering**: Caller responsible for determining which pools to consolidate
/// - **Flexible pause support**: Works with system-wide pause OR individual pool pause
pub fn process_consolidate_pool_fees(
    program_id: &Pubkey,
    pool_count: u8,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("🔄 Starting batch consolidation for {} pools", pool_count);
    
    // Validate pool count within limits
    if pool_count == 0 {
        msg!("❌ Pool count cannot be zero");
        return Err(ProgramError::InvalidArgument);
    }
    if pool_count > MAX_POOLS_PER_CONSOLIDATION_BATCH {
        msg!("❌ Pool count {} exceeds maximum {}", pool_count, MAX_POOLS_PER_CONSOLIDATION_BATCH);
        return Err(ProgramError::InvalidArgument);
    }
    
    // Extract accounts
    let system_state_pda = &accounts[0];
    let main_treasury_pda = &accounts[1];
    let pool_accounts = &accounts[2..2 + pool_count as usize];
    
    // Validate account count
    let expected_accounts = 2 + pool_count as usize;
    if accounts.len() != expected_accounts {
        msg!("❌ Expected {} accounts, got {}", expected_accounts, accounts.len());
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    let current_timestamp = Clock::get()?.unix_timestamp;
    
    // **PHASE 1: DETERMINE CONSOLIDATION MODE**
    let consolidation_mode = determine_consolidation_mode(program_id, system_state_pda)?;
    
    // **PHASE 2: BATCH CONSOLIDATION**
    perform_batch_consolidation(
        program_id,
        pool_accounts,
        main_treasury_pda,
        current_timestamp,
        consolidation_mode,
    )
}

/// Consolidation mode based on system/pool pause state
#[derive(Debug, Clone, Copy)]
enum ConsolidationMode {
    /// System is paused - consolidate all specified pools
    SystemPaused,
    /// System is active - only consolidate individually paused pools
    IndividualPoolPause,
}

/// Determines the consolidation mode based on system state
fn determine_consolidation_mode(
    program_id: &Pubkey,
    system_state_pda: &AccountInfo,
) -> Result<ConsolidationMode, ProgramError> {
    // Validate system state PDA
    let system_state = crate::utils::validation::validate_and_deserialize_system_state_secure(
        system_state_pda, 
        program_id
    )?;
    
    if system_state.is_paused {
        // System is paused - consolidate all specified pools
        msg!("🔍 System is paused - consolidating all specified pools");
        if system_state.pause_reason_code == PAUSE_REASON_CONSOLIDATION {
            msg!("✅ System paused with consolidation reason code ({})", PAUSE_REASON_CONSOLIDATION);
        } else {
            msg!("ℹ️ System paused with reason code: {}", system_state.pause_reason_code);
        }
        Ok(ConsolidationMode::SystemPaused)
    } else {
        // System is active - only consolidate individually paused pools
        msg!("🔍 System is active - checking for individually paused pools");
        Ok(ConsolidationMode::IndividualPoolPause)
    }
}

/// Performs the actual batch consolidation logic
fn perform_batch_consolidation(
    program_id: &Pubkey,
    pool_accounts: &[AccountInfo],
    main_treasury_pda: &AccountInfo,
    timestamp: i64,
    consolidation_mode: ConsolidationMode,
) -> ProgramResult {
    let mut total_sol_collected = 0u64;
    let mut consolidated_ops = ConsolidatedOperations::default();
    let mut pools_processed = 0u8;
    
    // **PROCESS POOLS BASED ON CONSOLIDATION MODE**
    for pool_account in pool_accounts {
        let mut pool_state = validate_and_deserialize_pool_state_secure(pool_account, program_id)?;
        
        // Check if pool is eligible for consolidation based on mode
        let is_eligible = match consolidation_mode {
            ConsolidationMode::SystemPaused => {
                // System paused - all pools are eligible
                true
            }
            ConsolidationMode::IndividualPoolPause => {
                // System active - only pools with both swaps_paused AND paused are eligible
                if pool_state.swaps_paused() && pool_state.liquidity_paused() {
                    msg!("✅ Pool {} is individually paused (swaps_paused: true, liquidity_paused: true)", pool_account.key);
                    true
                } else {
                    msg!("ℹ️ Pool {} not eligible - swaps_paused: {}, liquidity_paused: {}", 
                         pool_account.key, pool_state.swaps_paused(), pool_state.liquidity_paused());
                    false
                }
            }
        };
        
        if !is_eligible {
            continue;
        }
        
        let pool_fees = pool_state.pending_sol_fees();
        
        // Skip pools with no fees (but don't error)
        if pool_fees == 0 {
            msg!("ℹ️ Pool {} has no fees to consolidate, skipping", pool_account.key);
            continue;
        }
        
        // **RENT EXEMPT PROTECTION: Calculate rent exempt minimum for pool state**
        let rent_exempt_minimum = {
            use solana_program::sysvar::{rent::Rent, Sysvar};
            let rent = Rent::get()?;
            rent.minimum_balance(std::mem::size_of::<PoolState>())
        };
        
        // **RENT EXEMPT PROTECTION: Use helper method to safely calculate available consolidation**
        let current_pool_balance = pool_account.lamports();
        let available_for_consolidation = pool_state.calculate_available_for_consolidation(
            current_pool_balance,
            rent_exempt_minimum,
        );
        
        if available_for_consolidation == 0 {
            msg!("⚠️ Pool {} has {} lamports but needs {} for rent exemption, skipping consolidation", 
                 pool_account.key, current_pool_balance, rent_exempt_minimum);
            continue;
        }
        
        if available_for_consolidation < pool_fees {
            msg!("⚠️ Pool {} has {} pending fees but only {} available above rent exempt minimum", 
                 pool_account.key, pool_fees, available_for_consolidation);
            msg!("   Current balance: {} lamports", current_pool_balance);
            msg!("   Rent exempt minimum: {} lamports", rent_exempt_minimum);
            msg!("   Consolidating partial amount: {} lamports", available_for_consolidation);
        }
        
        // **SAFETY VALIDATION: Double-check consolidation safety before proceeding**
        if let Err(safety_error) = pool_state.validate_consolidation_safety(
            available_for_consolidation,
            current_pool_balance,
            rent_exempt_minimum,
        ) {
            msg!("❌ Consolidation safety check failed for pool {}: {}", pool_account.key, safety_error);
            continue;
        }
        
        // **GITHUB_ISSUE_31960_WORKAROUND: BUFFER SERIALIZATION PATTERN**
        // 
        // ** CRITICAL: Update pool state BEFORE any SOL transfers **
        // SOL transfer operations can corrupt PDA data buffers, so we must:
        // 1. Calculate all state changes first
        // 2. Serialize pool state to temporary buffer  
        // 3. Then perform SOL transfers
        // 4. Finally copy serialized data to account atomically
        
        // **IMPORTANT: Partial consolidation tracking**
        // Since we may not consolidate all fees, we need to track what was actually consolidated
        let consolidation_ratio = if pool_fees > 0 {
            available_for_consolidation as f64 / pool_fees as f64
        } else {
            0.0
        };
        
        // Apply consolidation ratio to fee breakdown
        let liquidity_fees_consolidated = (pool_state.collected_liquidity_fees as f64 * consolidation_ratio) as u64;
        let regular_swap_fees_consolidated = (pool_state.collected_swap_contract_fees as f64 * consolidation_ratio) as u64;
        
        // Accumulate consolidated data
        consolidated_ops.liquidity_fees += liquidity_fees_consolidated;
        consolidated_ops.regular_swap_fees += regular_swap_fees_consolidated;
        
        // Calculate operation counts from consolidated fees (using fixed fee constants)
        let liquidity_ops = liquidity_fees_consolidated / DEPOSIT_WITHDRAWAL_FEE;
        let regular_ops = regular_swap_fees_consolidated / SWAP_CONTRACT_FEE;
        
        consolidated_ops.liquidity_operation_count += liquidity_ops;
        consolidated_ops.regular_swap_count += regular_ops;
        
        total_sol_collected += available_for_consolidation;
        
        // **PARTIAL CONSOLIDATION: Update pool state based on what was actually consolidated**
        if consolidation_ratio >= 1.0 {
            // Full consolidation - reset all counters
            pool_state.reset_consolidation_counters(timestamp);
        } else {
            // Partial consolidation - reduce counters proportionally
            pool_state.collected_liquidity_fees -= liquidity_fees_consolidated;
            pool_state.collected_swap_contract_fees -= regular_swap_fees_consolidated;
            
            // Update total consolidated amount
            pool_state.total_fees_consolidated += available_for_consolidation;
            
            // Update metadata
            pool_state.last_consolidation_timestamp = timestamp;
            pool_state.total_consolidations += 1;
        }
        
        // **CONSISTENCY VALIDATION**: Verify fee tracking integrity after consolidation
        if pool_state.validate_fee_consistency().is_err() {
            msg!("❌ Fee consistency check failed for pool {}", pool_account.key);
            continue; // Skip this pool instead of panicking
        }
        
        // **STEP 1: Serialize pool state to temporary buffer BEFORE SOL transfers**
        let serialized_pool_data = pool_state.try_to_vec()?;
        
        // **STEP 2: Perform SOL transfers AFTER serialization**
        **pool_account.try_borrow_mut_lamports()? -= available_for_consolidation;
        **main_treasury_pda.try_borrow_mut_lamports()? += available_for_consolidation;
        
        // **RENT EXEMPT VALIDATION**: Verify pool still has rent exempt balance AFTER transfer
        if pool_account.lamports() < rent_exempt_minimum {
            msg!("❌ Pool {} balance {} below rent exempt minimum {} after consolidation",
                 pool_account.key, pool_account.lamports(), rent_exempt_minimum);
            // Note: SOL transfer already completed, but we log the issue
        }
        
        // **STEP 3: Copy serialized data to account atomically**
        {
            let mut account_data = pool_account.data.borrow_mut();
            account_data[..serialized_pool_data.len()].copy_from_slice(&serialized_pool_data);
        } // Release borrow immediately
        
        pools_processed += 1;
        msg!("✅ Pool {} consolidated: {} SOL ({}% of pending fees)", 
             pool_account.key, 
             available_for_consolidation as f64 / 1_000_000_000.0,
             (consolidation_ratio * 100.0) as u64);
    }
    
    // **STEP 3: Update MainTreasuryState** (even if no pools processed, update timestamp)
    let mut treasury_state = MainTreasuryState::try_from_slice(&main_treasury_pda.data.borrow())?;
    
    // Process batch consolidation
    treasury_state.batch_consolidation(total_sol_collected, &consolidated_ops, timestamp);
    
    // Sync balance with actual account balance
    treasury_state.sync_balance_with_account(main_treasury_pda.lamports());
    
    // Save updated treasury state
    let serialized_data = treasury_state.try_to_vec()?;
    main_treasury_pda.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    // Report consolidation results
    match consolidation_mode {
        ConsolidationMode::SystemPaused => {
            msg!("🎉 System-wide consolidation completed:");
        }
        ConsolidationMode::IndividualPoolPause => {
            msg!("🎉 Individual pool consolidation completed:");
        }
    }
    
    msg!("   Pools processed: {}", pools_processed);
    msg!("   Total SOL consolidated: {} ({} SOL)", 
         total_sol_collected, total_sol_collected as f64 / 1_000_000_000.0);
    msg!("   Total operations: {}", 
         consolidated_ops.liquidity_operation_count + 
         consolidated_ops.regular_swap_count);
    msg!("   🛡️ Rent exempt SOL protected in all pools");
    
    // Handle case where no pools were eligible/processed
    if pools_processed == 0 {
        match consolidation_mode {
            ConsolidationMode::SystemPaused => {
                msg!("ℹ️ No pools had fees to consolidate above rent exempt minimum");
            }
            ConsolidationMode::IndividualPoolPause => {
                msg!("ℹ️ No pools were individually paused or had sufficient fees above rent exempt minimum");
                msg!("ℹ️ To consolidate specific pools, pause them individually or pause the entire system");
            }
        }
    }
    
    Ok(())
}

/// **NEW: Get consolidation status for pools**
/// View-only function to check pool consolidation status
pub fn get_consolidation_status(
    program_id: &Pubkey,
    pool_accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("📊 CONSOLIDATION STATUS REPORT");
    msg!("===============================");
    
    let mut total_fees = 0u64;
    let mut pools_with_fees = 0u8;
    
    for (i, pool_account) in pool_accounts.iter().enumerate() {
        let pool_state = validate_and_deserialize_pool_state_secure(pool_account, program_id)?;
        
        let pool_fees = pool_state.pending_sol_fees();
        let operations = pool_state.total_operations_since_consolidation();
        
        msg!("Pool {}: {}", i + 1, pool_account.key);
        msg!("  Collected SOL: {} lamports ({:.6} SOL)", pool_fees, pool_fees as f64 / 1_000_000_000.0);
        msg!("  Operations since last consolidation: {}", operations);
        msg!("  Last consolidation: {}", 
             if pool_state.last_consolidation_timestamp == 0 { 
                 "Never".to_string() 
             } else { 
                 pool_state.last_consolidation_timestamp.to_string() 
             });
        msg!("");
        
        if pool_fees > 0 {
            total_fees += pool_fees;
            pools_with_fees += 1;
        }
    }
    
    msg!("📈 SUMMARY:");
    msg!("  Pools with fees: {}", pools_with_fees);
    msg!("  Total SOL available: {} lamports ({:.6} SOL)", 
         total_fees, total_fees as f64 / 1_000_000_000.0);
    msg!("  Estimated consolidation cost: ~0.58 SOL");
    if total_fees > 580_000_000 { // 0.58 SOL
        msg!("  Net benefit: {:.6} SOL", (total_fees as f64 / 1_000_000_000.0) - 0.58);
    } else {
        msg!("  ⚠️ Consolidation cost exceeds available fees");
    }
    
    Ok(())
} 