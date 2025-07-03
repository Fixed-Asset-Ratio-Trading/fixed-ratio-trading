//! View Test Utilities
//!
//! This module contains test-specific view/getter functions moved from main contract code.
//! These functions are primarily used for debugging, testing, and frontend integration.

use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    account_info::next_account_info,
};
use borsh::BorshDeserialize;
use fixed_ratio_trading::{PoolState};

/// **VIEW INSTRUCTION**: Returns comprehensive pool information
/// 
/// # Purpose
/// Logs structured pool information for debugging, testing, and frontend integration.
/// Outputs all critical pool state data in a human-readable format.
/// 
/// **⚠️ RACE CONDITION NOTICE**: Pool status reflects real-time state.
/// Temporary pause during large withdrawals (≥5% threshold) is expected behavior.
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
    msg!("Pool Paused: {}", pool_state.paused);
    msg!("Swaps Paused: {}", pool_state.swaps_paused);
    msg!("Swap Fee Basis Points: {}", pool_state.swap_fee_basis_points);
    
    // Enhanced operations status with race condition awareness
    msg!("=== OPERATIONS STATUS ===");
    msg!("Deposits: ENABLED");
    msg!("Withdrawals: ENABLED");
    
    if pool_state.swaps_paused {
        if pool_state.withdrawal_protection_active {
            msg!("Swaps: TEMPORARILY PAUSED (MEV Protection during large withdrawal)");
            msg!("  - Auto-clearing protection, not owner action");
            msg!("  - Will resume automatically after withdrawal completion");
        } else {
            msg!("Swaps: PAUSED (Owner Action)");
            msg!("  - Requires manual unpause by owner");
            msg!("  - Controlled by pool owner");
        }
    } else {
        msg!("Swaps: ENABLED");
    }
    
    msg!("===============================");
    
    Ok(())
}

/// **VIEW INSTRUCTION**: Returns current pool pause status - publicly accessible
/// 
/// # Purpose
/// Provides public visibility into pool operation status, distinguishing between
/// system-wide pause and pool-specific swap pause for user transparency.
/// 
/// **⚠️ RACE CONDITION NOTICE**: This query returns real-time status.
/// During large withdrawals (≥5% of pool), you may see temporary 
/// "swaps paused" status due to automatic MEV protection.
/// This is **expected behavior** and will auto-clear after withdrawal completion.
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
        // Distinguish between temporary withdrawal protection and owner pause
        if pool_state_data.withdrawal_protection_active {
            msg!("=== TEMPORARY WITHDRAWAL PROTECTION ===");
            msg!("Swaps temporarily paused during large withdrawal (≥5% of pool)");
            msg!("Protection will auto-clear after withdrawal completion");
            msg!("NOTE: This is MEV protection, not an owner action");
        } else {
            msg!("=== OWNER PAUSE ===");
            msg!("Swaps paused by owner action");
            msg!("Control: Pool owner");
            msg!("Note: No auto-unpause - requires manual unpause action");
        }
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

/// **VIEW INSTRUCTION**: Returns fee information including collected fees and rates.
/// 
/// This function provides comprehensive fee information essential for fee tracking,
/// transparency, and financial reporting. Shows both tracked fee amounts and 
/// actual account balances for complete transparency.
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

    msg!("=== FEE INFORMATION ===");
    
    // Pool fees (percentage-based on tokens)
    msg!("Pool Fees (Trading Fees):");
    msg!("  Current Swap Fee Rate: {} basis points ({:.2}%)", 
         pool_state.swap_fee_basis_points, 
         pool_state.swap_fee_basis_points as f64 / 100.0);
    msg!("  Collected Token A Fees: {}", pool_state.collected_fees_token_a);
    msg!("  Collected Token B Fees: {}", pool_state.collected_fees_token_b);
    msg!("  Total Token A Fees Withdrawn: {}", pool_state.total_fees_withdrawn_token_a);
    msg!("  Total Token B Fees Withdrawn: {}", pool_state.total_fees_withdrawn_token_b);
    
    // Contract fees (fixed SOL amounts)
    msg!("Contract Fees (SOL):");
    msg!("  Tracked SOL Fees Collected: {} lamports ({:.6} SOL)", 
         pool_state.collected_sol_fees,
         pool_state.collected_sol_fees as f64 / 1_000_000_000.0);
    msg!("  Total SOL Fees Withdrawn: {} lamports ({:.6} SOL)", 
         pool_state.total_sol_fees_withdrawn,
         pool_state.total_sol_fees_withdrawn as f64 / 1_000_000_000.0);
    
    // Actual pool state PDA balance
    let current_pool_balance = pool_state_account.lamports();
    msg!("Pool State PDA Balance:");
    msg!("  Current SOL Balance: {} lamports ({:.6} SOL)", 
         current_pool_balance,
         current_pool_balance as f64 / 1_000_000_000.0);
    
    // Calculate available fees for withdrawal (balance minus rent-exempt minimum)
    // Note: This is an approximation since we don't have rent sysvar here
    let estimated_rent_minimum = 2_500_000; // Conservative estimate for pool state account
    let estimated_available_fees = if current_pool_balance > estimated_rent_minimum {
        current_pool_balance - estimated_rent_minimum
    } else {
        0
    };
    
    msg!("  Estimated Available for Withdrawal: {} lamports ({:.6} SOL)", 
         estimated_available_fees,
         estimated_available_fees as f64 / 1_000_000_000.0);
    msg!("  (Note: Exact amount calculated during withdrawal with current rent rates)");
    
    msg!("=======================");

    Ok(())
}

/// **VIEW INSTRUCTION**: Returns the actual SOL balance of the pool state PDA.
/// 
/// This function provides direct access to the pool state account's SOL balance,
/// allowing users to see exactly how much SOL is held by the pool.
/// 
/// # Arguments
/// * `accounts` - Must contain pool state account as first account
/// 
/// # Returns
/// * `ProgramResult` - Logs pool state PDA SOL balance information
pub fn get_pool_sol_balance(accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_state_account = next_account_info(account_info_iter)?;

    let current_balance = pool_state_account.lamports();
    let estimated_rent_minimum = 2_500_000; // Conservative estimate
    let estimated_available = if current_balance > estimated_rent_minimum {
        current_balance - estimated_rent_minimum
    } else {
        0
    };

    msg!("=== POOL SOL BALANCE ===");
    msg!("Pool State PDA: {}", pool_state_account.key);
    msg!("Current SOL Balance: {} lamports", current_balance);
    msg!("Current SOL Balance: {:.6} SOL", current_balance as f64 / 1_000_000_000.0);
    msg!("Estimated Rent-Exempt Minimum: {} lamports", estimated_rent_minimum);
    msg!("Estimated Available for Withdrawal: {} lamports", estimated_available);
    msg!("Estimated Available for Withdrawal: {:.6} SOL", estimated_available as f64 / 1_000_000_000.0);
    msg!("Note: Use WithdrawFees instruction for exact calculations with current rent rates");
    msg!("========================");

    Ok(())
}

/// **VIEW INSTRUCTION**: Returns smart contract version information.
/// 
/// This function provides version information for the smart contract including
/// the main contract version from Cargo.toml and the schema version for data structures.
/// 
/// # Purpose
/// - Frontend/client version compatibility checking
/// - Deployment verification and audit trails
/// - API compatibility detection
/// - Development and debugging support
/// 
/// # Returns
/// * `ProgramResult` - Logs comprehensive version information
pub fn process_get_version() -> ProgramResult {
    msg!("=== SMART CONTRACT VERSION ===");
    msg!("Contract Name: {}", env!("CARGO_PKG_NAME"));
    msg!("Contract Version: {}", env!("CARGO_PKG_VERSION"));
    msg!("Contract Description: {}", env!("CARGO_PKG_DESCRIPTION"));
    msg!("Schema Version: v2"); // From POOL_STATE_SEED_PREFIX
    msg!("Solana Program: Yes");
    msg!("License: {}", env!("CARGO_PKG_LICENSE"));
    msg!("Program ID: 4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn");
    msg!("===============================");
    
    Ok(())
} 