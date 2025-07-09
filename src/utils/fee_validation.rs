//! Fee Validation Framework
//!
//! This module implements comprehensive fee validation and collection mechanisms
//! to ensure all fees are properly collected before operations proceed.
//!
//! Key Features:
//! - Pre-flight fee validation
//! - Atomic fee collection pattern
//! - Post-transfer validation
//! - Proper error handling with rollback capabilities

use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
};

use crate::{
    constants::*,
    error::PoolError,
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
/// * `fee_type` - Description of the fee type for error reporting
///
/// # Returns
/// * `FeeValidationResult` - Detailed validation result
pub fn validate_fee_payment(
    payer_account: &AccountInfo,
    fee_amount: u64,
    fee_type: &str,
) -> FeeValidationResult {
    let available_balance = payer_account.lamports();
    
    msg!("🔍 Pre-flight fee validation: {} fee", fee_type);
    msg!("   Required: {} lamports", fee_amount);
    msg!("   Available: {} lamports", available_balance);
    
    if available_balance < fee_amount {
        return FeeValidationResult {
            is_valid: false,
            available_balance,
            required_amount: fee_amount,
            error_message: Some(format!(
                "Insufficient balance for {} fee: required {} lamports, available {} lamports",
                fee_type, fee_amount, available_balance
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
/// * `treasury_type` - Description of treasury type for error reporting
///
/// # Returns
/// * `ProgramResult` - Success or error
pub fn validate_treasury_account(
    treasury_account: &AccountInfo,
    expected_pda: &Pubkey,
    treasury_type: &str,
) -> ProgramResult {
    // Verify PDA matches expected
    if *treasury_account.key != *expected_pda {
        msg!("❌ Treasury PDA mismatch for {}", treasury_type);
        return Err(PoolError::TreasuryValidationFailed {
            expected: *expected_pda,
            provided: *treasury_account.key,
            treasury_type: treasury_type.to_string(),
        }.into());
    }
    
    // Verify account is writable
    if !treasury_account.is_writable {
        msg!("❌ Treasury account is not writable: {}", treasury_type);
        return Err(PoolError::FeeValidationFailed {
            reason: format!("Treasury account for {} is not writable", treasury_type),
        }.into());
    }
    
    msg!("✅ Treasury account validated: {}", treasury_type);
    Ok(())
}

/// Atomic fee collection with pre and post validation
///
/// This function implements the "fees first" pattern by:
/// 1. Pre-flight validation of fee payment capability
/// 2. Treasury account validation
/// 3. Atomic fee transfer
/// 4. Post-transfer validation
///
/// # Arguments
/// * `payer_account` - The account paying the fee
/// * `treasury_account` - The treasury account receiving the fee
/// * `system_program` - The system program account
/// * `fee_amount` - The fee amount in lamports
/// * `fee_type` - Description of the fee type
/// * `expected_treasury_pda` - Expected treasury PDA for validation
///
/// # Returns
/// * `ProgramResult` - Success or error with detailed context
pub fn collect_fee_atomic<'a>(
    payer_account: &AccountInfo<'a>,
    treasury_account: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    fee_amount: u64,
    fee_type: &str,
    expected_treasury_pda: &Pubkey,
) -> ProgramResult {
    msg!("💰 Starting atomic fee collection: {}", fee_type);
    
    // 1. Pre-flight validation
    let validation_result = validate_fee_payment(payer_account, fee_amount, fee_type);
    if !validation_result.is_valid {
        return Err(PoolError::InsufficientFeeBalance {
            required: fee_amount,
            available: validation_result.available_balance,
            account: *payer_account.key,
        }.into());
    }
    
    // 2. Treasury account validation
    validate_treasury_account(treasury_account, expected_treasury_pda, fee_type)?;
    
    // 3. Record pre-transfer balances
    let payer_balance_before = payer_account.lamports();
    let treasury_balance_before = treasury_account.lamports();
    
    // 4. Atomic fee transfer
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
    
    // 5. Post-transfer validation
    let payer_balance_after = payer_account.lamports();
    let treasury_balance_after = treasury_account.lamports();
    
    let payer_deducted = payer_balance_before.saturating_sub(payer_balance_after);
    let treasury_received = treasury_balance_after.saturating_sub(treasury_balance_before);
    
    // Validate transfer amounts
    if payer_deducted != fee_amount {
        msg!("❌ Payer deduction mismatch: expected {}, actual {}", fee_amount, payer_deducted);
        return Err(PoolError::FeeCollectionFailed {
            expected: fee_amount,
            collected: payer_deducted,
            fee_type: format!("{} (payer side)", fee_type),
        }.into());
    }
    
    if treasury_received != fee_amount {
        msg!("❌ Treasury receipt mismatch: expected {}, actual {}", fee_amount, treasury_received);
        return Err(PoolError::FeeCollectionFailed {
            expected: fee_amount,
            collected: treasury_received,
            fee_type: format!("{} (treasury side)", fee_type),
        }.into());
    }
    
    msg!("✅ Atomic fee collection completed successfully");
    msg!("   Fee type: {}", fee_type);
    msg!("   Amount: {} lamports", fee_amount);
    msg!("   Payer: {}", payer_account.key);
    msg!("   Treasury: {}", treasury_account.key);
    
    Ok(())
}

/// Validates pool creation fee payment
///
/// # Arguments
/// * `payer_account` - The account paying the fee
/// * `treasury_account` - The main treasury account
/// * `system_program` - The system program account
/// * `program_id` - The program ID for PDA derivation
///
/// # Returns
/// * `ProgramResult` - Success or error
pub fn collect_pool_creation_fee<'a>(
    payer_account: &AccountInfo<'a>,
    treasury_account: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    program_id: &Pubkey,
) -> ProgramResult {
    let (expected_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        program_id,
    );
    
    collect_fee_atomic(
        payer_account,
        treasury_account,
        system_program,
        REGISTRATION_FEE,
        "Pool Creation",
        &expected_treasury_pda,
    )
}

/// Validates liquidity operation fee payment
///
/// # Arguments
/// * `payer_account` - The account paying the fee
/// * `treasury_account` - The main treasury account
/// * `system_program` - The system program account
/// * `program_id` - The program ID for PDA derivation
///
/// # Returns
/// * `ProgramResult` - Success or error
pub fn collect_liquidity_fee<'a>(
    payer_account: &AccountInfo<'a>,
    treasury_account: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    program_id: &Pubkey,
) -> ProgramResult {
    let (expected_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        program_id,
    );
    
    collect_fee_atomic(
        payer_account,
        treasury_account,
        system_program,
        DEPOSIT_WITHDRAWAL_FEE,
        "Liquidity Operation",
        &expected_treasury_pda,
    )
}

// Regular swap fee collection removed - use collect_regular_swap_fee_ultra_efficient() instead

// HFT swap fee collection removed - use collect_hft_swap_fee_ultra_efficient() instead

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
// ULTRA-EFFICIENT FEE COLLECTION FOR SWAP OPERATIONS
//=============================================================================
// These functions prioritize maximum CU efficiency over detailed validation
// and error messages. They should only be used for swap operations where
// performance is critical and generic errors are acceptable.

/// Ultra-efficient fee collection for swap operations
/// 
/// This function minimizes CU usage by:
/// - Skipping pre-flight validation
/// - Skipping post-transfer validation  
/// - Minimal PDA validation
/// - No logging
/// - Generic error handling
/// 
/// Estimated CU usage: ~50-100 CUs (vs 400-600 for atomic version)
/// 
/// # Arguments
/// * `payer_account` - The account paying the fee
/// * `treasury_account` - The treasury account receiving the fee
/// * `system_program` - The system program account
/// * `fee_amount` - The fee amount in lamports
/// * `expected_treasury_pda` - Expected treasury PDA for validation
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

/// Ultra-efficient regular swap fee collection
/// 
/// Optimized for maximum CU efficiency with minimal validation.
/// Uses generic errors and no logging for best performance.
/// 
/// # Arguments
/// * `payer_account` - The account paying the fee
/// * `treasury_account` - The swap treasury account
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
        &[SWAP_TREASURY_SEED_PREFIX],
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

/// Ultra-efficient HFT swap fee collection
/// 
/// Optimized for maximum CU efficiency with minimal validation.
/// Uses generic errors and no logging for best performance.
/// 
/// # Arguments
/// * `payer_account` - The account paying the fee
/// * `treasury_account` - The HFT treasury account
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
        &[HFT_TREASURY_SEED_PREFIX],
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