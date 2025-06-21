//! Utility Processors
//! 
//! This module contains utility processors for helper functions, view operations,
//! PDA derivation, and debugging/testing support functions.

use crate::constants::*;
use crate::types::*;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    account_info::next_account_info,
};
use borsh::BorshDeserialize;
use crate::error::PoolError;

// ================================================================================================
// PDA HELPER UTILITIES
// ================================================================================================

/// **PDA HELPER**: Computes and returns the Pool State PDA address for given tokens and ratio.
/// 
/// This utility function helps clients derive the Pool State PDA address without requiring
/// account creation or on-chain calls. Essential for preparing transaction account lists.
/// 
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `primary_token_mint` - Primary token mint pubkey
/// * `base_token_mint` - Base token mint pubkey  
/// * `ratio_primary_per_base` - Exchange ratio between tokens
/// 
/// # Returns
/// * `ProgramResult` - Logs the derived PDA address and bump seed
pub fn get_pool_state_pda(
    program_id: &Pubkey,
    primary_token_mint: Pubkey,
    base_token_mint: Pubkey,
    ratio_primary_per_base: u64,
) -> ProgramResult {
    msg!("DEBUG: get_pool_state_pda: Computing Pool State PDA");
    
    // Enhanced normalization to prevent economic duplicates (same logic as pool creation)
    // Step 1: Lexicographic token ordering
    let (token_a_mint_key, token_b_mint_key) = 
        if primary_token_mint < base_token_mint {
            (primary_token_mint, base_token_mint)
        } else {
            (base_token_mint, primary_token_mint)
        };
    
    // Step 2: Canonical ratio mapping to prevent liquidity fragmentation
    let (ratio_a_numerator, ratio_b_denominator): (u64, u64) = 
        if primary_token_mint < base_token_mint {
            (ratio_primary_per_base, 1u64)
        } else {
            // Use canonical form - both pools with same token pair get same ratio
            (ratio_primary_per_base, 1u64)
        };
    
    // Find PDA with canonical bump seed
    let (pool_state_pda, bump_seed) = Pubkey::find_program_address(
        &[
            POOL_STATE_SEED_PREFIX,
            token_a_mint_key.as_ref(),
            token_b_mint_key.as_ref(),
            &ratio_a_numerator.to_le_bytes(),
            &ratio_b_denominator.to_le_bytes(),
        ],
        program_id,
    );
    
    msg!("Pool State PDA: {}", pool_state_pda);
    msg!("Pool State PDA Bump Seed: {}", bump_seed);
    msg!("Normalized Token A: {}", token_a_mint_key);
    msg!("Normalized Token B: {}", token_b_mint_key);
    msg!("Normalized Ratio A: {}", ratio_a_numerator);
    msg!("Normalized Ratio B: {}", ratio_b_denominator);
    
    Ok(())
}

/// **PDA HELPER**: Computes and returns Token Vault PDA addresses for a given pool.
/// 
/// This utility helps clients derive the token vault addresses for pool operations.
/// Useful for preparing deposit, withdraw, and swap transaction account lists.
/// 
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `pool_state_pda` - The Pool State PDA address
/// 
/// # Returns
/// * `ProgramResult` - Logs the derived vault PDA addresses and bump seeds
pub fn get_token_vault_pdas(
    program_id: &Pubkey,
    pool_state_pda: Pubkey,
) -> ProgramResult {
    msg!("DEBUG: get_token_vault_pdas: Computing Token Vault PDAs for pool: {}", pool_state_pda);
    
    // Find Token A Vault PDA
    let (token_a_vault_pda, token_a_bump) = Pubkey::find_program_address(
        &[
            TOKEN_A_VAULT_SEED_PREFIX,
            pool_state_pda.as_ref(),
        ],
        program_id,
    );
    
    // Find Token B Vault PDA
    let (token_b_vault_pda, token_b_bump) = Pubkey::find_program_address(
        &[
            TOKEN_B_VAULT_SEED_PREFIX,
            pool_state_pda.as_ref(),
        ],
        program_id,
    );
    
    msg!("Token A Vault PDA: {}", token_a_vault_pda);
    msg!("Token A Vault Bump Seed: {}", token_a_bump);
    msg!("Token B Vault PDA: {}", token_b_vault_pda);
    msg!("Token B Vault Bump Seed: {}", token_b_bump);
    
    Ok(())
}

// ================================================================================================
// TEST-SPECIFIC VIEW/GETTER INSTRUCTIONS
// ================================================================================================

/// **VIEW INSTRUCTION**: Returns comprehensive pool information
/// 
/// # Purpose
/// Logs structured pool information for debugging, testing, and frontend integration.
/// Outputs all critical pool state data in a human-readable format.
/// 
/// # Account Layout (Read-Only)
/// 0. Pool State PDA (read-only)
/// 
/// # Returns
/// * `ProgramResult` - Logs comprehensive pool information
pub fn get_pool_info(accounts: &[AccountInfo]) -> ProgramResult {
    msg!("DEBUG: get_pool_info: Retrieving comprehensive pool information");
    
    let pool_state_account = &accounts[0];
    let pool_state = PoolState::deserialize(&mut &pool_state_account.data.borrow()[..])?;
    
    msg!("=== POOL STATE INFORMATION ===");
    msg!("Pool Owner: {}", pool_state.owner);
    msg!("Pool State PDA: {}", pool_state_account.key);
    msg!("Token A Mint: {}", pool_state.token_a_mint);
    msg!("Token B Mint: {}", pool_state.token_b_mint);
    msg!("Token A Vault: {}", pool_state.token_a_vault);
    msg!("Token B Vault: {}", pool_state.token_b_vault);
    msg!("LP Token A Mint: {}", pool_state.lp_token_a_mint);
    msg!("LP Token B Mint: {}", pool_state.lp_token_b_mint);
    msg!("Ratio A Numerator: {}", pool_state.ratio_a_numerator);
    msg!("Ratio B Denominator: {}", pool_state.ratio_b_denominator);
    msg!("Pool Authority Bump Seed: {}", pool_state.pool_authority_bump_seed);
    msg!("Token A Vault Bump Seed: {}", pool_state.token_a_vault_bump_seed);
    msg!("Token B Vault Bump Seed: {}", pool_state.token_b_vault_bump_seed);
    msg!("Is Initialized: {}", pool_state.is_initialized);
    msg!("Is Paused: {}", pool_state.is_paused);
    msg!("Swaps Paused: {}", pool_state.swaps_paused);
    msg!("Swap Fee Basis Points: {}", pool_state.swap_fee_basis_points);
    msg!("===============================");
    
    Ok(())
}

/// **VIEW INSTRUCTION**: Returns current pool pause status - publicly accessible
/// 
/// # Purpose
/// Provides public visibility into pool operation status, distinguishing between
/// system-wide pause and pool-specific swap pause for user transparency.
/// 
/// # Account Layout (Read-Only)
/// 0. Pool State PDA (read-only)
/// 
/// # Returns
/// * `ProgramResult` - Logs comprehensive pause status information
pub fn get_pool_pause_status(accounts: &[AccountInfo]) -> ProgramResult {
    let pool_state_account = &accounts[0];
    let pool_state_data = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    
    // Log comprehensive pause status for public visibility
    msg!("=== POOL STATUS ===");
    msg!("Swaps: {}", if pool_state_data.swaps_paused { "PAUSED" } else { "ENABLED" });
    msg!("Deposits: ENABLED");  // Always enabled (only system pause affects)
    msg!("Withdrawals: ENABLED"); // Always enabled (only system pause affects)
    
    if pool_state_data.swaps_paused {
        msg!("=== PAUSE DETAILS ===");
        msg!("Paused by: {:?}", pool_state_data.swaps_pause_requested_by);
        msg!("Paused at: {}", pool_state_data.swaps_pause_initiated_timestamp);
        msg!("Governance: Managed by delegate contract");
        msg!("Note: No auto-unpause - requires manual unpause action");
    }
    
    msg!("==================");
    
    Ok(())
}

/// **VIEW INSTRUCTION**: Returns detailed liquidity information for both tokens.
/// 
/// This function provides easy access to liquidity data, useful for calculating
/// exchange rates, available liquidity, and pool utilization metrics.
/// 
/// # Arguments
/// * `accounts` - Must contain pool state account as first account
/// 
/// # Returns
/// * `ProgramResult` - Logs detailed liquidity information
pub fn get_liquidity_info(accounts: &[AccountInfo]) -> ProgramResult {
    msg!("DEBUG: get_liquidity_info: Retrieving liquidity information");
    
    let pool_state_account = &accounts[0];
    let pool_state = PoolState::deserialize(&mut &pool_state_account.data.borrow()[..])?;
    
    msg!("=== LIQUIDITY INFORMATION ===");
    msg!("Total Token A Liquidity: {}", pool_state.total_token_a_liquidity);
    msg!("Total Token B Liquidity: {}", pool_state.total_token_b_liquidity);
    msg!("Exchange Rate (A per B): {}", 
         if pool_state.ratio_b_denominator != 0 { 
             pool_state.ratio_a_numerator as f64 / pool_state.ratio_b_denominator as f64 
         } else { 0.0 });
    msg!("Exchange Rate (B per A): {}", 
         if pool_state.ratio_a_numerator != 0 { 
             pool_state.ratio_b_denominator as f64 / pool_state.ratio_a_numerator as f64 
         } else { 0.0 });
    
    // Calculate utilization if available
    let total_value_locked = pool_state.total_token_a_liquidity + pool_state.total_token_b_liquidity;
    msg!("Total Value Locked (TVL): {} tokens", total_value_locked);
    msg!("==============================");
    
    Ok(())
}

/// **VIEW INSTRUCTION**: Returns delegate management information.
/// 
/// This function provides comprehensive delegate system information including
/// delegate list, withdrawal history, and pending requests for transparency.
/// 
/// # Arguments
/// * `accounts` - Must contain pool state account as first account
/// 
/// # Returns
/// * `ProgramResult` - Logs delegate management information
pub fn get_delegate_info(accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_state_account = next_account_info(account_info_iter)?;

    let pool_state = PoolState::deserialize(&mut &pool_state_account.data.borrow()[..])?;

    msg!("Delegate Info:");
    msg!("Total Delegates: {}", pool_state.delegate_management.delegate_count);
    for i in 0..pool_state.delegate_management.delegate_count as usize {
        let delegate = pool_state.delegate_management.delegates[i];
        let time_limits = pool_state.delegate_management.time_limits[i];
        msg!("Delegate {}: {}", i, delegate);
        msg!("  Fee Change Wait Time: {} seconds", time_limits.fee_change_wait_time);
        msg!("  Withdrawal Wait Time: {} seconds", time_limits.withdraw_wait_time);
        msg!("  Pause Wait Time: {} seconds", time_limits.pause_wait_time);
    }

    msg!("\nPending Actions:");
    for action in pool_state.delegate_management.pending_actions.iter() {
        msg!("Action ID: {}, Delegate: {}, Type: {:?}, Ready At: {}", 
             action.action_id, action.delegate, action.action_type, action.execution_timestamp);
    }

    msg!("\nAction History:");
    for action in pool_state.delegate_management.action_history.iter() {
        msg!("Action ID: {}, Delegate: {}, Type: {:?}, Requested At: {}", 
             action.action_id, action.delegate, action.action_type, action.request_timestamp);
    }

    Ok(())
}

/// **VIEW INSTRUCTION**: Returns fee information including collected fees and rates.
/// 
/// This function provides comprehensive fee information essential for fee tracking,
/// transparency, and financial reporting.
/// 
/// # Arguments
/// * `accounts` - Must contain pool state account as first account
/// 
/// # Returns
/// * `ProgramResult` - Logs detailed fee information
pub fn get_fee_info(accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_state_account = next_account_info(account_info_iter)?;

    let pool_state = PoolState::deserialize(&mut &pool_state_account.data.borrow()[..])?;

    msg!("Fee Info:");
    msg!("Current Swap Fee: {} basis points", pool_state.swap_fee_basis_points);
    msg!("Collected Fees:");
    msg!("  Token A: {}", pool_state.collected_fees_token_a);
    msg!("  Token B: {}", pool_state.collected_fees_token_b);
    msg!("  SOL: {}", pool_state.collected_sol_fees);
    msg!("Total Fees Withdrawn:");
    msg!("  Token A: {}", pool_state.total_fees_withdrawn_token_a);
    msg!("  Token B: {}", pool_state.total_fees_withdrawn_token_b);
    msg!("  SOL: {}", pool_state.total_sol_fees_withdrawn);

    Ok(())
}

/// Validates that an account is owned by the expected program.
pub fn validate_account_owner(account: &AccountInfo, expected_owner: &Pubkey) -> ProgramResult {
    if account.owner != expected_owner {
        msg!("Account {} has incorrect owner. Expected: {}, Actual: {}", 
             account.key, expected_owner, account.owner);
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Validates that an account is a signer.
pub fn validate_signer(account: &AccountInfo, context: &str) -> ProgramResult {
    if !account.is_signer {
        msg!("{} must be a signer", context);
        return Err(ProgramError::MissingRequiredSignature);
    }
    Ok(())
}

/// Validates that an account is writable.
pub fn validate_writable(account: &AccountInfo, context: &str) -> ProgramResult {
    if !account.is_writable {
        msg!("{} must be writable", context);
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Validates that a swap fee is within allowed bounds.
pub fn validate_swap_fee(fee_basis_points: u16) -> ProgramResult {
    if fee_basis_points > 50 { // 0.5% maximum fee
        msg!("Swap fee {} basis points exceeds maximum of {}", 
             fee_basis_points, 50);
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Validates that an amount is non-zero.
pub fn validate_non_zero_amount(amount: u64, context: &str) -> ProgramResult {
    if amount == 0 {
        msg!("{} amount cannot be zero", context);
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Validates that two tokens are different.
pub fn validate_different_tokens(token_a: &Pubkey, token_b: &Pubkey) -> ProgramResult {
    if token_a == token_b {
        msg!("Cannot create pool with identical tokens: {}", token_a);
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Validates that a wait time is within allowed bounds.
pub fn validate_wait_time(wait_time: u64) -> ProgramResult {
    if wait_time < 300 || wait_time > 259200 { // 5 minutes to 72 hours
        msg!("Wait time {} seconds is outside allowed range [{}, {}]", 
             wait_time, 300, 259200);
        return Err(PoolError::InvalidWaitTime { wait_time }.into());
    }
    Ok(())
}

/// Validates that a pool is initialized.
pub fn validate_pool_initialized(pool_state: &PoolState) -> ProgramResult {
    if !pool_state.is_initialized {
        msg!("Pool is not yet initialized");
        return Err(ProgramError::UninitializedAccount);
    }
    Ok(())
}

/// Validates that a pool is not paused.
pub fn validate_pool_not_paused(pool_state: &PoolState) -> ProgramResult {
    if pool_state.is_paused {
        msg!("Pool operations are currently paused");
        return Err(PoolError::PoolPaused.into());
    }
    Ok(())
}

/// Gets the wait time for a delegate action based on action type.
pub fn get_action_wait_time(pool_state: &PoolState, delegate: &Pubkey, action_type: &DelegateActionType) -> Option<u64> {
    if let Some(time_limits) = pool_state.delegate_management.get_delegate_time_limits(delegate) {
        match action_type {
            DelegateActionType::FeeChange => Some(time_limits.fee_change_wait_time),
            DelegateActionType::Withdrawal => Some(time_limits.withdraw_wait_time),
            DelegateActionType::PausePoolSwaps => Some(time_limits.pause_wait_time),
            DelegateActionType::UnpausePoolSwaps => Some(time_limits.pause_wait_time),
        }
    } else {
        None
    }
}

/// Gets the action history for a pool.
pub fn get_action_history(pool_state: &PoolState) -> ProgramResult {
    msg!("Action History (last 10 actions):");
    for (i, action) in pool_state.delegate_management.action_history.iter().enumerate() {
        msg!("Record {}: Delegate: {}, Action Type: {:?}, Action ID: {}, Timestamp: {}", 
             i, action.delegate, action.action_type, action.action_id, action.request_timestamp);
    }
    Ok(())
} 