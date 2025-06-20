//! Fee Management Processors
//! 
//! This module contains all the processors for fee-related operations including
//! fee withdrawals, fee configuration, and fee collection management.

use crate::types::*;
use crate::utils::*;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar, clock::Clock},
};
use borsh::BorshDeserialize;

/// Processes fee withdrawals by the pool owner.
///
/// This function allows the pool owner to withdraw accumulated SOL fees collected from
/// pool operations (deposits, withdrawals, swaps, registration). The withdrawal maintains
/// rent-exempt status by ensuring sufficient SOL remains in the pool state account.
/// Only the designated pool owner can execute fee withdrawals.
///
/// # Purpose
/// - Enables pool owner to collect accumulated SOL fees for operational costs
/// - Maintains pool rent-exempt status during fee collection
/// - Provides transparent fee withdrawal mechanism with logging
/// - Supports sustainable pool operations through fee collection
/// - Ensures only authorized pool owner can access collected fees
///
/// # How it works
/// 1. Validates the caller is the designated pool owner and signed the transaction
/// 2. Loads current pool state data to verify ownership and calculate available fees
/// 3. Calculates available fees by subtracting rent-exempt minimum from pool balance
/// 4. Transfers available SOL fees directly from pool state PDA to owner account
/// 5. Uses direct lamport transfer for PDA accounts
/// 6. Logs fee withdrawal amount for transparency and audit compliance
///
/// # Arguments
/// * `_program_id` - The program ID (currently unused but reserved for validation)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool owner account (must be signer and match pool state owner)
///   - `accounts[1]` - Pool state PDA account (source of fees, must have sufficient balance)
///   - `accounts[2]` - System program (required for SOL transfers)
///   - `accounts[3]` - Rent sysvar (for rent-exempt calculations)
///   - `accounts[4]` - Clock sysvar (for rent tracking and timestamps)
///
/// # Account Requirements
/// - Owner: Must be signer and match the owner field in pool state data
/// - Pool state: Must be the valid pool state PDA with sufficient SOL balance
/// - System program: Standard Solana system program for SOL transfers
///
/// # Fee Calculation
/// - Available fees = Total pool state balance - Rent-exempt minimum
/// - Rent-exempt minimum calculated using current rent rates and account size
/// - Zero fees available indicates all SOL is reserved for rent exemption
///
/// # Security Features
/// - **Ownership validation**: Only the designated pool owner can withdraw fees
/// - **Rent protection**: Always maintains minimum balance for rent exemption
/// - **Direct lamport transfer**: Uses safe lamport transfer for PDA accounts
/// - **Transparency**: Logs all fee withdrawals for audit trail
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Owner didn't sign transaction
/// - `ProgramError::InvalidAccountData` - Caller is not the pool owner
/// - `ProgramError::ArithmeticOverflow` - Mathematical calculation errors
///
/// # Example Usage
/// ```ignore
/// // Pool owner withdraws accumulated SOL fees
/// let instruction = PoolInstruction::WithdrawFees;
/// // Transfers: pool_balance - rent_minimum → owner_account
/// ```
///
/// # Note
/// This function only handles SOL fees. For SPL token fee withdrawals, use the
/// delegate withdrawal system through `WithdrawFeesToDelegate`.
pub fn process_withdraw_fees(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing WithdrawFees");
    let account_info_iter = &mut accounts.iter();

    // ✅ CRITICAL: System pause validation (takes precedence over pool pause)
    let system_state_account = next_account_info(account_info_iter)?;
    crate::utils::validation::validate_system_not_paused(system_state_account)?;

    let owner = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;
    let _system_program = next_account_info(account_info_iter)?;
    let rent_sysvar = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer for fee withdrawal");
        return Err(ProgramError::MissingRequiredSignature);
    }
    msg!("✅ Owner is signer check passed");

    // Load and verify pool state
    msg!("📖 Loading pool state data...");
    let pool_state_data = PoolState::deserialize(&mut &pool_state.data.borrow()[..])?;
    msg!("✅ Pool state loaded successfully");
    
    msg!("🔍 Checking owner authorization...");
    msg!("   Owner provided: {}", owner.key);
    msg!("   Pool owner: {}", pool_state_data.owner);
    
    if *owner.key != pool_state_data.owner {
        msg!("❌ Only pool owner can withdraw fees");
        return Err(ProgramError::InvalidAccountData);
    }
    msg!("✅ Owner authorization verified");

    // Calculate available fees (total balance minus rent exempt requirement)
    msg!("💰 Calculating available fees...");
    let rent = &Rent::from_account_info(rent_sysvar)?;
    let clock = &Clock::from_account_info(clock_sysvar)?;
    msg!("✅ Rent and clock sysvars loaded");
    
    // Ensure rent exempt status before withdrawal
    msg!("🔒 Checking rent exempt status...");
    check_rent_exempt(pool_state, _program_id, rent, clock.slot)?;
    msg!("✅ Rent exempt status verified");

    let minimum_balance = rent.minimum_balance(pool_state.data_len());
    let current_balance = pool_state.lamports();
    
    if current_balance <= minimum_balance {
        msg!("No fees available for withdrawal. Current: {}, Required: {}", 
             current_balance, minimum_balance);
        return Ok(()); // No error, just no fees to withdraw
    }

    let available_fees = current_balance - minimum_balance;
    
    msg!("Withdrawing {} lamports in fees", available_fees);

    // Transfer lamports from pool state to owner
    **pool_state.try_borrow_mut_lamports()? -= available_fees;
    **owner.try_borrow_mut_lamports()? += available_fees;

    msg!("Fee withdrawal completed successfully. Amount: {} lamports", available_fees);

    Ok(())
} 