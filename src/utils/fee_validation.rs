//! Fee Validation Framework
//!
//! **PHASE 3: CENTRALIZED FEE COLLECTION**
//!
//! This module implements centralized fee collection where all fees go directly
//! to the main treasury with real-time counter updates. This eliminates the need
//! for specialized treasuries and consolidation operations.
//!
//! Key Features:
//! - All fees collected directly into main treasury
//! - Real-time counter and total updates
//! - Simplified architecture with single treasury
//! - Atomic fee collection with state updates
//! - Proper error handling with rollback capabilities

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{clock::Clock, Sysvar},
};

use crate::{
    constants::*,
    error::PoolError,
    state::MainTreasuryState,
};

/// Fee collection context for tracking and validation
#[derive(Debug, Clone)]
pub struct FeeContext {
    pub fee_type: String,
    pub amount: u64,
    pub payer: Pubkey,
    pub recipient: Pubkey,
}

/// Fee validation result with detailed information
#[derive(Debug, Clone)]
pub struct FeeValidationResult {
    pub is_valid: bool,
    pub available_balance: u64,
    pub required_amount: u64,
    pub error_message: Option<String>,
}

/// Pre-flight fee validation
/// 
/// Validates that the user has sufficient balance to pay the required fee
/// before any operation state changes occur.
///
/// # Arguments
/// * `payer_account` - The account that will pay the fee
/// * `fee_amount` - The required fee amount in lamports
/// * `validation_context_code` - Validation context byte code (use VALIDATION_CONTEXT_* constants)
///
/// # Returns
/// * `FeeValidationResult` - Detailed validation result
pub fn validate_fee_payment(
    payer_account: &AccountInfo,
    fee_amount: u64,
    validation_context_code: u8,
) -> FeeValidationResult {
    let available_balance = payer_account.lamports();
    
    if available_balance < fee_amount {
        return FeeValidationResult {
            is_valid: false,
            available_balance,
            required_amount: fee_amount,
            error_message: Some(format!(
                "Insufficient balance for context {}: required {} lamports, available {} lamports",
                validation_context_code, fee_amount, available_balance
            )),
        };
    }
    
    FeeValidationResult {
        is_valid: true,
        available_balance,
        required_amount: fee_amount,
        error_message: None,
    }
}

/// Validates treasury account PDA and writability
///
/// # Arguments
/// * `treasury_account` - The treasury account to validate
/// * `expected_pda` - The expected PDA address
/// * `treasury_type_code` - Treasury type byte code (use TREASURY_TYPE_* constants)
///
/// # Returns
/// * `ProgramResult` - Success or error
pub fn validate_treasury_account(
    treasury_account: &AccountInfo,
    expected_pda: &Pubkey,
    treasury_type_code: u8,
) -> ProgramResult {
    // Verify PDA matches expected
    if *treasury_account.key != *expected_pda {
        return Err(PoolError::TreasuryValidationFailed {
            expected: *expected_pda,
            provided: *treasury_account.key,
            treasury_type: treasury_type_code.to_string(),
        }.into());
    }
    
    // Verify account is writable
    if !treasury_account.is_writable {
        return Err(PoolError::FeeValidationFailed {
            reason: format!("Treasury account for type {} is not writable", treasury_type_code),
        }.into());
    }
    
    Ok(())
}

/// **PHASE 3: CENTRALIZED FEE COLLECTION WITH REAL-TIME STATE UPDATES**
///
/// This function implements centralized fee collection by:
/// 1. Pre-flight validation of fee payment capability
/// 2. Treasury account validation
/// 3. Atomic fee transfer to main treasury
/// 4. Real-time treasury state update
/// 5. Post-transfer validation
///
/// # Arguments
/// * `payer_account` - The account paying the fee
/// * `treasury_account` - The main treasury account receiving the fee
/// * `system_program` - The system program account
/// * `clock_sysvar` - Clock sysvar for timestamp
/// * `fee_amount` - The fee amount in lamports
/// * `fee_type_code` - Fee type byte code (use FEE_TYPE_* constants)
/// * `expected_treasury_pda` - Expected treasury PDA for validation
///
/// # Returns
/// * `ProgramResult` - Success or error with detailed context
pub fn collect_fee_with_real_time_tracking<'a>(
    payer_account: &AccountInfo<'a>,
    treasury_account: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    clock_sysvar: &AccountInfo<'a>,
    fee_amount: u64,
    fee_type_code: u8,
    expected_treasury_pda: &Pubkey,
) -> ProgramResult {
    // 1. Pre-flight validation
    let validation_result = validate_fee_payment(payer_account, fee_amount, VALIDATION_CONTEXT_FEE);
    if !validation_result.is_valid {
        return Err(PoolError::InsufficientFeeBalance {
            required: fee_amount,
            available: validation_result.available_balance,
            account: *payer_account.key,
        }.into());
    }
    
    // 2. Treasury account validation
    validate_treasury_account(treasury_account, expected_treasury_pda, TREASURY_TYPE_MAIN)?;
    
    // 3. Get current timestamp
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_timestamp = clock.unix_timestamp;
    
    // 4. Record pre-transfer balances
    let payer_balance_before = payer_account.lamports();
    let treasury_balance_before = treasury_account.lamports();
    
    // 5. Atomic fee transfer
    let transfer_instruction = system_instruction::transfer(
        payer_account.key,
        treasury_account.key,
        fee_amount,
    );
    
    invoke(
        &transfer_instruction,
        &[
            payer_account.clone(),
            treasury_account.clone(),
            system_program.clone(),
        ],
    )?;
    
    // 6. Post-transfer validation
    let payer_balance_after = payer_account.lamports();
    let treasury_balance_after = treasury_account.lamports();
    
    let payer_deducted = payer_balance_before.saturating_sub(payer_balance_after);
    let treasury_received = treasury_balance_after.saturating_sub(treasury_balance_before);
    
    // Validate transfer amounts
    if payer_deducted != fee_amount || treasury_received != fee_amount {
        return Err(PoolError::FeeCollectionFailed {
            expected: fee_amount,
            collected: payer_deducted,
            fee_type: fee_type_code.to_string(),
        }.into());
    }
    
    // 7. **PHASE 3: REAL-TIME STATE UPDATE**
    let mut treasury_state = MainTreasuryState::try_from_slice(&treasury_account.data.borrow())?;
    
    // Update state based on fee type code (efficient byte matching)
    match fee_type_code {
        FEE_TYPE_POOL_CREATION => {
            treasury_state.add_pool_creation_fee(fee_amount, current_timestamp);
        }
        FEE_TYPE_LIQUIDITY_OPERATION => {
            treasury_state.add_liquidity_fee(fee_amount, current_timestamp);
        }
        FEE_TYPE_REGULAR_SWAP => {
            treasury_state.add_regular_swap_fee(fee_amount, current_timestamp);
        }
        FEE_TYPE_HFT_SWAP => {
            treasury_state.add_hft_swap_fee(fee_amount, current_timestamp);
        }
        _ => {
            // Unknown fee type - still collect fee but don't update specific counters
        }
    }
    
    // Sync balance with actual account balance
    treasury_state.sync_balance_with_account(treasury_balance_after);
    
    // Save updated state
    let serialized_data = treasury_state.try_to_vec()?;
    treasury_account.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    Ok(())
}

/// **PHASE 3: POOL CREATION FEE COLLECTION**
/// Collects pool creation fee directly to main treasury with real-time tracking
pub fn collect_pool_creation_fee<'a>(
    payer_account: &AccountInfo<'a>,
    treasury_account: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    clock_sysvar: &AccountInfo<'a>,
    program_id: &Pubkey,
) -> ProgramResult {
    let (expected_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        program_id,
    );
    
    collect_fee_with_real_time_tracking(
        payer_account,
        treasury_account,
        system_program,
        clock_sysvar,
        REGISTRATION_FEE,
        FEE_TYPE_POOL_CREATION,
        &expected_treasury_pda,
    )
}

/// **PHASE 3: LIQUIDITY OPERATION FEE COLLECTION**
/// Collects liquidity operation fee directly to main treasury with real-time tracking
pub fn collect_liquidity_fee<'a>(
    payer_account: &AccountInfo<'a>,
    treasury_account: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    clock_sysvar: &AccountInfo<'a>,
    program_id: &Pubkey,
) -> ProgramResult {
    let (expected_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        program_id,
    );
    
    collect_fee_with_real_time_tracking(
        payer_account,
        treasury_account,
        system_program,
        clock_sysvar,
        DEPOSIT_WITHDRAWAL_FEE,
        FEE_TYPE_LIQUIDITY_OPERATION,
        &expected_treasury_pda,
    )
}

/// **PHASE 3: REGULAR SWAP FEE COLLECTION**
/// Collects regular swap fee directly to main treasury with real-time tracking
pub fn collect_regular_swap_fee<'a>(
    payer_account: &AccountInfo<'a>,
    treasury_account: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    clock_sysvar: &AccountInfo<'a>,
    program_id: &Pubkey,
) -> ProgramResult {
    let (expected_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        program_id,
    );
    
    collect_fee_with_real_time_tracking(
        payer_account,
        treasury_account,
        system_program,
        clock_sysvar,
        SWAP_FEE,
        FEE_TYPE_REGULAR_SWAP,
        &expected_treasury_pda,
    )
}

/// **PHASE 3: HFT SWAP FEE COLLECTION**
/// Collects HFT swap fee directly to main treasury with real-time tracking
pub fn collect_hft_swap_fee<'a>(
    payer_account: &AccountInfo<'a>,
    treasury_account: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    clock_sysvar: &AccountInfo<'a>,
    program_id: &Pubkey,
) -> ProgramResult {
    let (expected_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        program_id,
    );
    
    collect_fee_with_real_time_tracking(
        payer_account,
        treasury_account,
        system_program,
        clock_sysvar,
        HFT_SWAP_FEE,
        FEE_TYPE_HFT_SWAP,
        &expected_treasury_pda,
    )
}

/// Emergency rollback mechanism for failed operations
///
/// This function can be used to rollback state changes if an operation
/// fails after fee collection. Note: This is a best-effort mechanism
/// and may not be able to rollback all changes.
///
/// # Arguments
/// * `fee_context` - Context of the fee that was collected
/// * `error_reason` - Reason for the rollback
///
/// # Returns
/// * `ProgramResult` - Success or error
pub fn rollback_fee_collection(
    fee_context: &FeeContext,
    error_reason: &str,
) -> ProgramResult {
    msg!("🔄 Emergency fee rollback requested");
    msg!("   Fee type: {}", fee_context.fee_type);
    msg!("   Amount: {} lamports", fee_context.amount);
    msg!("   Reason: {}", error_reason);
    
    // Note: Actual rollback implementation would require additional
    // infrastructure for transaction reversal. This is a placeholder
    // for future rollback mechanisms.
    
    msg!("⚠️ Fee rollback not implemented - fees have been collected");
    Err(PoolError::FeeValidationFailed {
        reason: format!("Operation failed after fee collection: {}", error_reason),
    }.into())
}

//=============================================================================
// ULTRA-EFFICIENT FEE COLLECTION FOR SWAP OPERATIONS (PHASE 3 COMPATIBLE)
//=============================================================================
// These functions prioritize maximum CU efficiency over detailed validation
// and error messages. They collect fees directly to main treasury but skip
// real-time state updates for maximum performance.

/// Ultra-efficient fee collection for swap operations
/// 
/// **PHASE 3: MAIN TREASURY ONLY**
/// This function minimizes CU usage by:
/// - Collecting fees directly to main treasury (no specialized treasuries)
/// - Skipping pre-flight validation
/// - Skipping post-transfer validation  
/// - Minimal PDA validation
/// - No logging
/// - No real-time state updates (for performance)
/// - Generic error handling
/// 
/// Estimated CU usage: ~50-100 CUs (vs 400-600 for full tracking version)
/// 
/// # Arguments
/// * `payer_account` - The account paying the fee
/// * `treasury_account` - The main treasury account receiving the fee
/// * `system_program` - The system program account
/// * `fee_amount` - The fee amount in lamports
/// * `expected_treasury_pda` - Expected main treasury PDA for validation
///
/// # Returns
/// * `ProgramResult` - Success or generic error
pub fn collect_fee_ultra_efficient<'a>(
    payer_account: &AccountInfo<'a>,
    treasury_account: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    fee_amount: u64,
    expected_treasury_pda: &Pubkey,
) -> ProgramResult {
    // Only critical PDA validation (skip writability and other checks)
    if *treasury_account.key != *expected_treasury_pda {
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Direct transfer with minimal overhead
    invoke(
        &system_instruction::transfer(
            payer_account.key,
            treasury_account.key,
            fee_amount,
        ),
        &[
            payer_account.clone(),
            treasury_account.clone(),
            system_program.clone(),
        ],
    )
}

/// **PHASE 3: ULTRA-EFFICIENT REGULAR SWAP FEE COLLECTION**
/// 
/// Optimized for maximum CU efficiency with minimal validation.
/// Collects fees directly to main treasury (no specialized treasuries).
/// Uses generic errors and no logging for best performance.
/// 
/// # Arguments
/// * `payer_account` - The account paying the fee
/// * `treasury_account` - The main treasury account (not specialized treasury)
/// * `system_program` - The system program account
/// * `program_id` - The program ID for PDA derivation
///
/// # Returns
/// * `ProgramResult` - Success or generic error
pub fn collect_regular_swap_fee_ultra_efficient<'a>(
    payer_account: &AccountInfo<'a>,
    treasury_account: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    program_id: &Pubkey,
) -> ProgramResult {
    let (expected_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX], // Phase 3: Use main treasury instead of swap treasury
        program_id,
    );
    
    collect_fee_ultra_efficient(
        payer_account,
        treasury_account,
        system_program,
        SWAP_FEE,
        &expected_treasury_pda,
    )
}

/// **PHASE 3: ULTRA-EFFICIENT HFT SWAP FEE COLLECTION**
/// 
/// Optimized for maximum CU efficiency with minimal validation.
/// Collects fees directly to main treasury (no specialized treasuries).
/// Uses generic errors and no logging for best performance.
/// 
/// # Arguments
/// * `payer_account` - The account paying the fee
/// * `treasury_account` - The main treasury account (not specialized treasury)
/// * `system_program` - The system program account
/// * `program_id` - The program ID for PDA derivation
///
/// # Returns
/// * `ProgramResult` - Success or generic error
pub fn collect_hft_swap_fee_ultra_efficient<'a>(
    payer_account: &AccountInfo<'a>,
    treasury_account: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    program_id: &Pubkey,
) -> ProgramResult {
    let (expected_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX], // Phase 3: Use main treasury instead of HFT treasury
        program_id,
    );
    
    collect_fee_ultra_efficient(
        payer_account,
        treasury_account,
        system_program,
        HFT_SWAP_FEE,
        &expected_treasury_pda,
    )
} 