//! Fee Management Processors
//! 
//! This module contains all the processors for fee-related operations including
//! fee withdrawals, fee configuration, and fee collection management.
//!
//! ## Fee Types Overview
//! 
//! The Fixed Ratio Trading system implements two distinct fee types:
//!
//! ### 1. Contract Fees (Fixed SOL amounts)
//! - **Pool Creation**: 1.15 SOL per pool creation
//! - **Liquidity Operations**: 0.0013 SOL per deposit/withdrawal  
//! - **Swaps**: 0.00002715 SOL per swap transaction
//! - **Purpose**: Cover operational costs and prevent spam
//! - **Collection**: Automatically transferred to pool state PDA
//! - **Withdrawal**: Via `process_withdraw_fees()` by pool owner
//!
//! ### 2. Pool Fees (Percentage-based on tokens)
//! - **Rate**: 0% to 0.5% configurable by pool owner
//! - **Default**: 0% (free trading by default)
//! - **Application**: Deducted from input tokens during swaps
//! - **Purpose**: Revenue generation for pool operators
//! - **Collection**: Tracked in pool state (`collected_fees_token_a`, `collected_fees_token_b`)
//! - **Withdrawal**: Via `process_withdraw_pool_fees()` by pool owner

use crate::{
    types::*,
    utils::*,
    constants::{MAX_SWAP_FEE_BASIS_POINTS, POOL_STATE_SEED_PREFIX},
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar, clock::Clock},
    program::invoke_signed,
};
use borsh::{BorshDeserialize, BorshSerialize};
use spl_token;

/// Processes **Contract Fee** withdrawals by the pool owner.
///
/// This function allows the pool owner to withdraw accumulated **SOL fees** (contract fees) 
/// collected from pool operations. These are the fixed SOL amounts charged for:
/// - Pool creation (1.15 SOL)
/// - Deposits/withdrawals (0.0013 SOL each)  
/// - Swaps (0.00002715 SOL each)
///
/// The withdrawal maintains rent-exempt status by ensuring sufficient SOL remains in the 
/// pool state account. Only the designated pool owner can execute SOL fee withdrawals.
///
/// **Note**: This function handles SOL fees only. For SPL token fee withdrawals (pool fees),
/// use `process_withdraw_pool_fees()`.
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
/// This function only handles SOL fees. For SPL token fee withdrawals, use
/// `process_withdraw_pool_fees()`.
pub fn process_withdraw_fees(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing WithdrawFees");
    
    // ✅ SYSTEM PAUSE: Backward compatible validation
    crate::utils::validation::validate_system_not_paused_safe(accounts, 5)?; // Expected: 5 accounts minimum
    
    let account_info_iter = &mut accounts.iter();
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

    //=========================================================================
    // UPDATE CONTRACT FEE TRACKING
    //=========================================================================
    // Update withdrawal tracking for SOL fees - moved to central treasury
    // Note: SOL fees are now tracked centrally in TreasuryState, not per-pool
    msg!("✅ SOL fee tracking moved to central treasury system");
    msg!("   Per-pool SOL fee tracking no longer available");
    msg!("   Use GetTreasuryInfo instruction for system-wide SOL fee data");

    msg!("Fee withdrawal completed successfully. Amount: {} lamports", available_fees);

    Ok(())
}

/// Changes the swap fee rate (owner only).
///
/// This function allows the pool owner to modify the percentage-based trading fee 
/// charged on swap operations. The fee is deducted from the input token amount
/// and collected by the pool for revenue generation.
///
/// # Arguments
/// * `_program_id` - The program ID (currently unused but reserved for validation)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool owner account (must be signer and match pool state owner)
///   - `accounts[1]` - Pool state PDA account (writable)
/// * `new_fee_basis_points` - New fee rate in basis points (0-50 = 0%-0.5%)
///
/// # Security Features
/// - **Owner-only**: Only the pool owner can change fees
/// - **Rate limits**: Fee cannot exceed 0.5% (50 basis points)
/// - **Immediate effect**: New fee rate applies to all subsequent swaps
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Owner didn't sign transaction
/// - `ProgramError::InvalidAccountData` - Caller is not the pool owner or invalid fee rate
pub fn process_change_fee(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_fee_basis_points: u64,
) -> ProgramResult {
    msg!("Processing ChangeFee: {} basis points", new_fee_basis_points);
    
    // ✅ SYSTEM PAUSE: Backward compatible validation
    crate::utils::validation::validate_system_not_paused_safe(accounts, 2)?; // Expected: 2 accounts minimum
    
    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer to change fees");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate fee rate
    if new_fee_basis_points > MAX_SWAP_FEE_BASIS_POINTS {
        msg!("Fee rate {} exceeds maximum {}", new_fee_basis_points, MAX_SWAP_FEE_BASIS_POINTS);
        return Err(ProgramError::InvalidAccountData);
    }

    // Load and verify pool state
    let mut pool_state_data = PoolState::deserialize(&mut &pool_state.data.borrow()[..])?;
    if *owner.key != pool_state_data.owner {
        msg!("Only pool owner can change fees");
        return Err(ProgramError::InvalidAccountData);
    }

    // Update fee rate
    let old_fee = pool_state_data.swap_fee_basis_points;
    pool_state_data.swap_fee_basis_points = new_fee_basis_points;
    
    // Save updated state
    let mut serialized_data = Vec::new();
    pool_state_data.serialize(&mut serialized_data)?;
    let account_data_len = pool_state.data_len();
    if serialized_data.len() > account_data_len {
        return Err(ProgramError::AccountDataTooSmall);
    }
    {
        let mut account_data = pool_state.data.borrow_mut();
        account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
    }
    
    msg!("Fee rate changed successfully: {} → {} basis points", old_fee, new_fee_basis_points);

    Ok(())
}

/// Withdraws accumulated pool fees (owner only).
///
/// This function allows the pool owner to withdraw SPL token fees that have been
/// collected from swap operations. These fees are tracked separately from the
/// main pool liquidity.
///
/// # Arguments
/// * `_program_id` - The program ID (currently unused but reserved for validation)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool owner account (must be signer and match pool state owner)
///   - `accounts[1]` - Pool state PDA account (writable)
///   - `accounts[2]` - Owner's token account (for receiving fees)
///   - `accounts[3]` - Token program
///   - `accounts[4]` - Pool vault account (for the token being withdrawn)
/// * `token_mint` - The token mint to withdraw fees for
/// * `amount` - Amount of tokens to withdraw
///
/// # Security Features
/// - **Owner-only**: Only the pool owner can withdraw fees
/// - **Amount validation**: Cannot withdraw more than collected
/// - **Rent protection**: Maintains vault rent exemption
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Owner didn't sign transaction
/// - `ProgramError::InvalidAccountData` - Caller is not the pool owner or insufficient fees
pub fn process_withdraw_pool_fees(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    token_mint: Pubkey,
    amount: u64,
) -> ProgramResult {
    msg!("Processing WithdrawPoolFees: {} tokens of mint {}", amount, token_mint);
    
    // ✅ SYSTEM PAUSE: Backward compatible validation
    crate::utils::validation::validate_system_not_paused_safe(accounts, 5)?; // Expected: 5 accounts minimum
    
    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let pool_state_account = next_account_info(account_info_iter)?;
    let owner_token_account = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let vault_account_info = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer to withdraw pool fees");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify pool state
    let mut pool_state = PoolState::deserialize(&mut &pool_state_account.data.borrow()[..])?;
    if *owner.key != pool_state.owner {
        msg!("Only pool owner can withdraw pool fees");
        return Err(ProgramError::InvalidAccountData);
    }

    // Determine which token and fees we're dealing with
    let (collected_fees, vault_account) = if token_mint == pool_state.token_a_mint {
        (pool_state.collected_fees_token_a, pool_state.token_a_vault)
    } else if token_mint == pool_state.token_b_mint {
        (pool_state.collected_fees_token_b, pool_state.token_b_vault)
    } else {
        msg!("Invalid token mint for this pool");
        return Err(ProgramError::InvalidAccountData);
    };

    // Verify vault account matches
    if *vault_account_info.key != vault_account {
        msg!("Invalid vault account provided");
        return Err(ProgramError::InvalidAccountData);
    }

    // Check if sufficient fees are available
    if amount > collected_fees {
        msg!("Insufficient fees available. Requested: {}, Available: {}", amount, collected_fees);
        return Err(ProgramError::InvalidAccountData);
    }

    // Transfer fees from vault to owner
    let pool_state_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state.token_a_mint.as_ref(),
        pool_state.token_b_mint.as_ref(),
        &pool_state.ratio_a_numerator.to_le_bytes(),
        &pool_state.ratio_b_denominator.to_le_bytes(),
        &[pool_state.pool_authority_bump_seed],
    ];

    invoke_signed(
        &spl_token::instruction::transfer(
            token_program.key,
            vault_account_info.key,
            owner_token_account.key,
            pool_state_account.key,
            &[],
            amount,
        )?,
        &[
            vault_account_info.clone(),
            owner_token_account.clone(),
            pool_state_account.clone(),
            token_program.clone(),
        ],
        &[pool_state_seeds],
    )?;

    // Update fee tracking
    if token_mint == pool_state.token_a_mint {
        pool_state.collected_fees_token_a = pool_state.collected_fees_token_a
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        pool_state.total_fees_withdrawn_token_a = pool_state.total_fees_withdrawn_token_a
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    } else {
        pool_state.collected_fees_token_b = pool_state.collected_fees_token_b
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        pool_state.total_fees_withdrawn_token_b = pool_state.total_fees_withdrawn_token_b
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }

    // Save updated state
    let mut serialized_data = Vec::new();
    pool_state.serialize(&mut serialized_data)?;
    pool_state_account.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);

    msg!("Pool fee withdrawal completed successfully. Amount: {} tokens", amount);

    Ok(())
}

/// Pauses swap operations for this specific pool (owner only).
///
/// This function allows the pool owner to immediately pause swap operations
/// while keeping deposits and withdrawals functional.
///
/// # Arguments
/// * `_program_id` - The program ID (currently unused but reserved for validation)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool owner account (must be signer and match pool state owner)
///   - `accounts[1]` - Pool state PDA account (writable)
///   - `accounts[2]` - Clock sysvar (for timestamp)
///
/// # Security Features
/// - **Owner-only**: Only the pool owner can pause swaps
/// - **Immediate effect**: Swap operations are blocked immediately
/// - **Selective pause**: Only swaps are paused, not deposits/withdrawals
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Owner didn't sign transaction
/// - `ProgramError::InvalidAccountData` - Caller is not the pool owner
/// - `PoolError::PoolSwapsAlreadyPaused` - Swaps are already paused
pub fn process_pause_pool_swaps(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing PausePoolSwaps");
    
    // ✅ SYSTEM PAUSE: Backward compatible validation
    crate::utils::validation::validate_system_not_paused_safe(accounts, 3)?; // Expected: 3 accounts minimum
    
    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let pool_state_account = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer to pause pool swaps");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify pool state
    let mut pool_state = PoolState::deserialize(&mut &pool_state_account.data.borrow()[..])?;
    if *owner.key != pool_state.owner {
        msg!("Only pool owner can pause pool swaps");
        return Err(ProgramError::InvalidAccountData);
    }

    // Check if already paused
    if pool_state.swaps_paused {
        msg!("Pool swaps are already paused");
        return Err(PoolError::PoolSwapsAlreadyPaused.into());
    }

    // Get current timestamp
    let clock = &Clock::from_account_info(clock_sysvar)?;

    // Pause swaps
        pool_state.swaps_paused = true;

    // Save updated state
    let mut serialized_data = Vec::new();
    pool_state.serialize(&mut serialized_data)?;
    pool_state_account.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);

    msg!("Pool swaps paused successfully by owner at timestamp {}", clock.unix_timestamp);

    Ok(())
}

/// Unpauses swap operations for this specific pool (owner only).
///
/// This function allows the pool owner to resume swap operations
/// that were previously paused.
///
/// # Arguments
/// * `_program_id` - The program ID (currently unused but reserved for validation)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool owner account (must be signer and match pool state owner)
///   - `accounts[1]` - Pool state PDA account (writable)
///
/// # Security Features
/// - **Owner-only**: Only the pool owner can unpause swaps
/// - **Immediate effect**: Swap operations resume immediately
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Owner didn't sign transaction
/// - `ProgramError::InvalidAccountData` - Caller is not the pool owner
/// - `PoolError::PoolSwapsNotPaused` - Swaps are not currently paused
pub fn process_unpause_pool_swaps(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing UnpausePoolSwaps");
    
    // ✅ SYSTEM PAUSE: Backward compatible validation
    crate::utils::validation::validate_system_not_paused_safe(accounts, 2)?; // Expected: 2 accounts minimum
    
    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let pool_state_account = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer to unpause pool swaps");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify pool state
    let mut pool_state = PoolState::deserialize(&mut &pool_state_account.data.borrow()[..])?;
    if *owner.key != pool_state.owner {
        msg!("Only pool owner can unpause pool swaps");
        return Err(ProgramError::InvalidAccountData);
    }

    // Check if actually paused
    if !pool_state.swaps_paused {
        msg!("Pool swaps are not currently paused");
        return Err(PoolError::PoolSwapsNotPaused.into());
    }

    // Unpause swaps
    pool_state.swaps_paused = false;

    // Save updated state
    let mut serialized_data = Vec::new();
    pool_state.serialize(&mut serialized_data)?;
    pool_state_account.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);

    msg!("Pool swaps unpaused successfully by owner");

    Ok(())
} 