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

// ⚠️ IMPORTANT: When changing the program ID, also update PROGRAM_AUTHORITY in constants.rs
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
    // fees module contains only governance-controlled fee architecture documentation
    swap::{
        process_swap,
        process_set_swap_owner_only,
    },
    // security module contains only governance-controlled security architecture documentation
    process_initialize::{
        process_initialize_program,
    },
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
        process_get_treasury_info,
    },
    consolidation::{
        process_consolidate_pool_fees,
        get_consolidation_status,
    },
    pool_management::{
        process_pause_pool,
        process_unpause_pool,
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
        PoolInstruction::InitializeProgram {
            // No fields to extract - system authority comes from accounts[0]
        } => process_initialize_program(program_id, accounts),

        PoolInstruction::InitializePool {
            ratio_a_numerator,
            ratio_b_denominator,
        } => process_initialize_pool(program_id, ratio_a_numerator, ratio_b_denominator, accounts),

        PoolInstruction::Deposit {
            deposit_token_mint,
            amount,
        } => process_deposit(program_id, amount, deposit_token_mint, accounts),

        PoolInstruction::Withdraw {
            withdraw_token_mint,
            lp_amount_to_burn,
        } => process_withdraw(program_id, lp_amount_to_burn, withdraw_token_mint, accounts),

        PoolInstruction::Swap {
            input_token_mint: _,
            amount_in,
        } => process_swap(program_id, amount_in, accounts),

        PoolInstruction::SetSwapOwnerOnly {
            enable_restriction,
            designated_owner,
        } => process_set_swap_owner_only(program_id, enable_restriction, designated_owner, accounts),



        // Pool owner management instructions not implemented (governance-controlled architecture)

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
            reason_code,
        } => process_pause_system(program_id, reason_code, accounts),

        PoolInstruction::UnpauseSystem => process_unpause_system(program_id, accounts),

        PoolInstruction::GetVersion => process_get_version(),
        
        // Treasury Management Instructions
        PoolInstruction::WithdrawTreasuryFees {
            amount,
        } => process_withdraw_treasury_fees(program_id, amount, accounts),

        PoolInstruction::GetTreasuryInfo {} => process_get_treasury_info(program_id, accounts),
        
        // Consolidation Instructions
        PoolInstruction::ConsolidatePoolFees {
            pool_count,
        } => process_consolidate_pool_fees(program_id, pool_count, accounts),
        
        PoolInstruction::GetConsolidationStatus {
            pool_count,
        } => get_consolidation_status(program_id, &accounts[..pool_count as usize]),
        
        // Pool Management Instructions
        PoolInstruction::PausePool {
            pause_flags,
        } => process_pause_pool(program_id, pause_flags, accounts),
        
        PoolInstruction::UnpausePool {
            unpause_flags,
        } => process_unpause_pool(program_id, unpause_flags, accounts),
    }
}

pub use crate::types::errors::PoolError;

// Public utilities
pub use crate::processors::utilities::derive_pool_id;

