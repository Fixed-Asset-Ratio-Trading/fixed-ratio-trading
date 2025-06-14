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
    pubkey::Pubkey,
};
use borsh::{BorshDeserialize, BorshSerialize};

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

/// **VIEW INSTRUCTION**: Returns comprehensive pool state information.
/// 
/// This function provides easy access to all pool state data in a structured format.
/// Ideal for testing, debugging, frontend integration, and transparency.
/// 
/// # Arguments
/// * `accounts` - Must contain pool state account as first account
/// 
/// # Returns
/// * `ProgramResult` - Logs comprehensive pool information
pub fn get_pool_info(accounts: &[AccountInfo]) -> ProgramResult {
    msg!("DEBUG: get_pool_info: Retrieving comprehensive pool information");
    
    let pool_state_account = &accounts[0];
    let pool_state = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    
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
    msg!("Swap Fee Basis Points: {}", pool_state.swap_fee_basis_points);
    msg!("===============================");
    
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
    let pool_state = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    
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
    msg!("DEBUG: get_delegate_info: Retrieving delegate information");
    
    let pool_state_account = &accounts[0];
    let pool_state = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    
    msg!("=== DELEGATE INFORMATION ===");
    msg!("Total Delegates: {}", pool_state.delegate_management.delegate_count);
    
    // List all delegates
    for (i, delegate) in pool_state.delegate_management.delegates.iter().enumerate() {
        if i < pool_state.delegate_management.delegate_count as usize {
            msg!("Delegate {}: {}", i + 1, delegate);
            
            // Show wait time for this delegate
            if let Some(wait_time) = pool_state.delegate_management.get_delegate_wait_time(delegate) {
                msg!("  Wait Time: {} seconds", wait_time);
            }
            
            // Show any pending withdrawal request
            if let Some(request) = pool_state.delegate_management.get_withdrawal_request(delegate) {
                msg!("  Pending Withdrawal: {} of token {}", request.amount, request.token_mint);
                msg!("  Request Timestamp: {}", request.request_timestamp);
            }
        }
    }
    
    // Show recent withdrawal history
    msg!("Recent Withdrawal History:");
    msg!("History Index: {}", pool_state.delegate_management.withdrawal_history_index);
    for (i, record) in pool_state.delegate_management.withdrawal_history.iter().enumerate() {
        if record.delegate != Pubkey::default() { // Only show non-empty records
            msg!("  Record {}: Delegate {}, Amount {}, Token {}, Slot {}", 
                 i, record.delegate, record.amount, record.token_mint, record.slot);
        }
    }
    msg!("============================");
    
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
    msg!("DEBUG: get_fee_info: Retrieving fee information");
    
    let pool_state_account = &accounts[0];
    let pool_state = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    
    msg!("=== FEE INFORMATION ===");
    
    // Fee rates
    msg!("Swap Fee Rate: {} basis points ({:.4}%)", 
         pool_state.swap_fee_basis_points, 
         pool_state.swap_fee_basis_points as f64 / 100.0);
    msg!("Registration Fee: {} lamports ({:.9} SOL)", REGISTRATION_FEE, REGISTRATION_FEE as f64 / 1_000_000_000.0);
    msg!("Deposit/Withdrawal Fee: {} lamports ({:.9} SOL)", DEPOSIT_WITHDRAWAL_FEE, DEPOSIT_WITHDRAWAL_FEE as f64 / 1_000_000_000.0);
    msg!("Swap Fee: {} lamports ({:.9} SOL)", SWAP_FEE, SWAP_FEE as f64 / 1_000_000_000.0);
    
    // Collected fees
    msg!("Collected Token A Fees: {}", pool_state.collected_fees_token_a);
    msg!("Collected Token B Fees: {}", pool_state.collected_fees_token_b);
    msg!("Collected SOL Fees: {} lamports ({:.9} SOL)", 
         pool_state.collected_sol_fees, 
         pool_state.collected_sol_fees as f64 / 1_000_000_000.0);
    
    // Withdrawn fees (for tracking)
    msg!("Total Token A Fees Withdrawn: {}", pool_state.total_fees_withdrawn_token_a);
    msg!("Total Token B Fees Withdrawn: {}", pool_state.total_fees_withdrawn_token_b);
    msg!("Total SOL Fees Withdrawn: {} lamports ({:.9} SOL)", 
         pool_state.total_sol_fees_withdrawn, 
         pool_state.total_sol_fees_withdrawn as f64 / 1_000_000_000.0);
    
    // Available fees (collected minus withdrawn)
    let available_token_a_fees = pool_state.collected_fees_token_a.saturating_sub(pool_state.total_fees_withdrawn_token_a);
    let available_token_b_fees = pool_state.collected_fees_token_b.saturating_sub(pool_state.total_fees_withdrawn_token_b);
    let available_sol_fees = pool_state.collected_sol_fees.saturating_sub(pool_state.total_sol_fees_withdrawn);
    
    msg!("Available Token A Fees: {}", available_token_a_fees);
    msg!("Available Token B Fees: {}", available_token_b_fees);
    msg!("Available SOL Fees: {} lamports ({:.9} SOL)", 
         available_sol_fees, 
         available_sol_fees as f64 / 1_000_000_000.0);
    
    msg!("=======================");
    
    Ok(())
} 