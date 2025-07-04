/*
MIT License

Copyright (c) 2024 Davinci

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

//! # Fixed Ratio Trading Pool Program
//! 
//! This is the main library for the fixed-ratio-trading program.
//! It contains the program's instructions, error handling, and other functionality.
//! It also contains the program's constants and PDA seeds.
//! It is used by the program's entrypoint and other modules.
//!
//! ## CRITICAL: GITHUB_ISSUE_31960_WORKAROUND
//! 
//! **This program implements a workaround for Solana GitHub Issue #31960**
//! 
//! ### The Problem:
//! Solana's AccountInfo.data doesn't get updated after CPI account creation within 
//! the same instruction. This causes issues when:
//! 1. Creating accounts via CPI (system_instruction::create_account)
//! 2. Immediately trying to read/write data to those accounts
//! 3. The AccountInfo.data reference still points to empty/uninitialized memory
//! 
//! ### The Solution:
//! We implement a **single-instruction pattern** for pool creation:
//! 
//! #### Modern Approach: InitializePool (RECOMMENDED)
//! - Single instruction that handles all pool creation atomically
//! - Creates all required accounts via CPI
//! - Creates Pool State PDA, LP token mints, token vaults
//! - Writes PoolState data structure with buffer serialization
//! - Uses careful account handling to avoid the GitHub Issue #31960
//! 
//! ### Where This Affects:
//! - Pool creation functions in `processors/pool_creation.rs`
//! - Test helpers in `tests/common/pool_helpers.rs`
//! - Any code that creates and immediately uses accounts
//! 
//! ### Buffer Serialization Workaround:
//! Instead of direct serialization, we use a two-step process:
//! 1. Serialize to temporary buffer
//! 2. Copy buffer to account data atomically
//! 
//! This prevents "silent failures" where serialization reports success
//! but data doesn't persist due to stale AccountInfo references.

use borsh::BorshDeserialize;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    declare_id,
};

#[cfg(all(not(feature = "no-entrypoint"), target_os = "solana"))]
use solana_program::entrypoint;

declare_id!("4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn");

// Declare the entrypoint
#[cfg(all(not(feature = "no-entrypoint"), target_os = "solana"))]
entrypoint!(process_instruction);

// Module declarations
pub mod client_sdk;
pub mod constants;
pub mod error;
pub mod state;
pub mod types;
pub mod utils;
pub mod processors;

// Re-export all modules for public API
// IMPORTANT: These must be public re-exports to allow test access
pub use constants::*;
pub use state::*;
pub use types::*;
pub use utils::*;

// Import specific processor functions for internal use only
// Note: We only import processors, not types, to avoid shadowing public re-exports
use crate::processors::{
    pool_creation::{
        process_initialize_pool,
    },
    liquidity::{
        process_deposit,
        process_withdraw,
    },
    fees::{
        process_change_fee,
        process_withdraw_pool_fees,
        process_pause_pool_swaps,
        process_unpause_pool_swaps,
    },
    swap::{
        process_swap,
        process_swap_hft_optimized,
    },
    security::process_update_security_params,
    system_pause::{
        process_pause_system,
        process_unpause_system,
    },
    utilities::{
        get_pool_state_pda,
        get_token_vault_pdas,
        get_pool_info,
        get_pool_pause_status,
        get_liquidity_info,
        get_fee_info,
        get_pool_sol_balance,
        process_get_version,
    },
    treasury::{
        process_withdraw_treasury_fees,
        process_consolidate_treasuries,
        process_get_treasury_info,
        process_get_specialized_treasury_balances,
    },
};

/// Main entry point for the fixed-ratio trading pool Solana program.
///
/// This function serves as the central dispatcher for all pool operations, routing
/// instructions to their appropriate handler functions with global security checks.
///
/// # Features
/// - Central instruction routing and dispatch
/// - Global pause state enforcement (blocks user operations when paused)
/// - Instruction deserialization and validation
/// - Comprehensive error handling and logging
///
/// # Arguments
/// * `program_id` - The program ID for validation
/// * `accounts` - Array of accounts for the operation
/// * `instruction_data` - Serialized instruction data
///
/// # Security
/// - Pause enforcement: User operations blocked when pool is paused
/// - Owner operations (fees, security, pool creation) remain accessible during pause
/// - All instructions validated before dispatch to handlers
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = PoolInstruction::try_from_slice(instruction_data)?;

    match instruction {
        PoolInstruction::InitializePool {
            ratio_a_numerator,
            ratio_b_denominator,
            pool_authority_bump_seed,
            token_a_vault_bump_seed,
            token_b_vault_bump_seed,
        } => process_initialize_pool(program_id, accounts, ratio_a_numerator, ratio_b_denominator,
            pool_authority_bump_seed, token_a_vault_bump_seed, token_b_vault_bump_seed),

        PoolInstruction::Deposit {
            deposit_token_mint,
            amount,
        } => process_deposit(program_id, accounts, deposit_token_mint, amount),

        PoolInstruction::Withdraw {
            withdraw_token_mint,
            lp_amount_to_burn,
        } => process_withdraw(program_id, accounts, withdraw_token_mint, lp_amount_to_burn),

        PoolInstruction::Swap {
            input_token_mint,
            amount_in,
        } => process_swap(program_id, accounts, input_token_mint, amount_in),

        PoolInstruction::SwapHftOptimized {
            input_token_mint,
            amount_in,
            skip_rent_checks,
        } => process_swap_hft_optimized(program_id, accounts, input_token_mint, amount_in, skip_rent_checks),

        PoolInstruction::UpdateSecurityParams {
            paused,
            only_lp_token_a_for_both,
        } => process_update_security_params(program_id, accounts, paused, only_lp_token_a_for_both),

        PoolInstruction::ChangeFee {
            new_fee_basis_points,
        } => process_change_fee(program_id, accounts, new_fee_basis_points),

        PoolInstruction::WithdrawPoolFees {
            token_mint,
            amount,
        } => process_withdraw_pool_fees(program_id, accounts, token_mint, amount),

        PoolInstruction::PausePoolSwaps => process_pause_pool_swaps(program_id, accounts),

        PoolInstruction::UnpausePoolSwaps => process_unpause_pool_swaps(program_id, accounts),

        PoolInstruction::GetPoolStatePDA {
            multiple_token_mint,
            base_token_mint,
            multiple_per_base,
        } => get_pool_state_pda(program_id, multiple_token_mint, base_token_mint, multiple_per_base),

        PoolInstruction::GetTokenVaultPDAs {
            pool_state_pda,
        } => get_token_vault_pdas(program_id, pool_state_pda),

        PoolInstruction::GetPoolInfo {} => get_pool_info(accounts),

        PoolInstruction::GetPoolPauseStatus {} => get_pool_pause_status(accounts),

        PoolInstruction::GetLiquidityInfo {} => get_liquidity_info(accounts),

        PoolInstruction::GetFeeInfo {} => get_fee_info(accounts),
        
        PoolInstruction::GetPoolSolBalance {} => get_pool_sol_balance(accounts),
        
        PoolInstruction::PauseSystem {
            reason,
        } => process_pause_system(program_id, accounts, reason),

        PoolInstruction::UnpauseSystem => process_unpause_system(program_id, accounts),

        PoolInstruction::GetVersion => process_get_version(),
        
        // Treasury Management Instructions
        PoolInstruction::WithdrawTreasuryFees {
            amount,
        } => process_withdraw_treasury_fees(program_id, accounts, amount),

        PoolInstruction::ConsolidateTreasuries => process_consolidate_treasuries(program_id, accounts),

        PoolInstruction::GetTreasuryInfo {} => process_get_treasury_info(program_id, accounts),

        PoolInstruction::GetSpecializedTreasuryBalances {} => process_get_specialized_treasury_balances(program_id, accounts),
    }
}

pub use crate::types::errors::PoolError;