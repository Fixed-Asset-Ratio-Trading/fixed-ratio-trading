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

#[cfg(not(test))]
use solana_program::msg;

// Conditional logging macro - disable verbose logging during tests
#[cfg(not(test))]
#[macro_export]
macro_rules! debug_msg {
    ($($arg:tt)*) => {
        msg!($($arg)*);
    };
}

#[cfg(test)]
#[macro_export]
macro_rules! debug_msg {
    ($($arg:tt)*) => {
        // Logging disabled during tests for cleaner output
    };
}

#[cfg(all(not(feature = "no-entrypoint"), target_os = "solana"))]
use solana_program::entrypoint;

// ⚠️ IMPORTANT: When changing the program ID, also update PROGRAM_AUTHORITY in constants.rs
// Network-specific program IDs using conditional compilation
//
// Build commands for different networks:
//   LocalNet (default): cargo build-bpf
//   DevNet:            cargo build-bpf --features devnet --no-default-features
//   MainNet:           cargo build-bpf --features mainnet --no-default-features
//
#[cfg(feature = "localnet")]
declare_id!("4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn"); // LocalNet Program ID

#[cfg(feature = "devnet")]
declare_id!("9iqh69RqeG3RRrFBNZVoE77TMRvYboFUtC2sykaFVzB7"); // DevNet Program ID

#[cfg(feature = "mainnet")]
declare_id!("quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD"); // MainNet Program ID

// Default to LocalNet if no network is specified (shouldn't happen with default=["localnet"])
#[cfg(not(any(feature = "localnet", feature = "devnet", feature = "mainnet")))]
declare_id!("4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn"); // Default to LocalNet

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
    pool::{
        process_pool_initialize,
        process_pool_pause,
        process_pool_unpause,
        process_pool_update_fees,
    },
    liquidity::{
        process_liquidity_deposit,
        process_liquidity_withdraw,
    },
    // fees module contains only governance-controlled fee architecture documentation
    swap::{
        process_swap_execute,
        process_swap_set_owner_only,
    },
    // security module contains only governance-controlled security architecture documentation
    system::{
        process_system_initialize,
        process_system_pause,
        process_system_unpause,
        process_system_get_version,
        process_admin_change,
    },
    utilities::{
        get_pool_state_pda,
        get_token_vault_pdas,
        get_pool_info,
        get_pool_pause_status,
        get_liquidity_info,
        get_fee_info,
        get_pool_sol_balance,

    },
    treasury::{
        process_treasury_withdraw_fees,
        process_treasury_get_info,
        process_treasury_donate_sol,
    },
    consolidation::{
        process_consolidate_pool_fees,
        get_consolidation_status,
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
/// 
/// ⚠️  SECURITY NOTE: Lifetime annotations <'a> are CRITICAL for reentrancy protection
/// These annotations ensure AccountInfo references in reentrancy protection structs
/// live long enough to prevent memory safety issues. DO NOT remove them to fix tests.
pub fn process_instruction<'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> ProgramResult {
    debug_msg!("🚨🚨🚨 PROGRAM ENTRY POINT - INSTRUCTION RECEIVED! 🚨🚨🚨");
    debug_msg!("🎯 ENTRY POINT: Processing instruction with {} bytes", instruction_data.len());
    
    // Validate instruction data has minimum size
    use crate::utils::input_validation::*;
    validate_instruction_data_size(instruction_data, MIN_INSTRUCTION_DATA_SIZE, "Any instruction")?;
    
    let instruction = PoolInstruction::try_from_slice(instruction_data)?;
    debug_msg!("✅ DESERIALIZATION: Instruction deserialized successfully");

    match instruction {
        PoolInstruction::InitializeProgram {
            admin_authority,
        } => {
            validate_account_count(accounts, INITIALIZE_PROGRAM_ACCOUNTS, "InitializeProgram")?;
            process_system_initialize(program_id, admin_authority, accounts)
        },

        PoolInstruction::InitializePool {
            ratio_a_numerator,
            ratio_b_denominator,
        } => {
            validate_account_count(accounts, INITIALIZE_POOL_ACCOUNTS, "InitializePool")?;
            process_pool_initialize(program_id, ratio_a_numerator, ratio_b_denominator, accounts)
        },

        PoolInstruction::Deposit {
            deposit_token_mint,
            amount,
            pool_id,
        } => {
            validate_account_count(accounts, DEPOSIT_ACCOUNTS, "Deposit")?;
            process_liquidity_deposit(program_id, amount, deposit_token_mint, pool_id, accounts)
        },

        PoolInstruction::Withdraw {
            withdraw_token_mint,
            lp_amount_to_burn,
            pool_id,
        } => {
            validate_account_count(accounts, WITHDRAW_ACCOUNTS, "Withdraw")?;
            process_liquidity_withdraw(program_id, lp_amount_to_burn, withdraw_token_mint, pool_id, accounts)
        },

        PoolInstruction::Swap {
            input_token_mint: _,
            amount_in,
            expected_amount_out,
            pool_id,
        } => {
            validate_account_count(accounts, SWAP_ACCOUNTS, "Swap")?;
            process_swap_execute(program_id, amount_in, expected_amount_out, pool_id, accounts)
        },

        PoolInstruction::SetSwapOwnerOnly {
            enable_restriction,
            designated_owner,
            pool_id,
        } => {
            validate_account_count(accounts, SET_SWAP_OWNER_ONLY_ACCOUNTS, "SetSwapOwnerOnly")?;
            process_swap_set_owner_only(program_id, enable_restriction, designated_owner, pool_id, accounts)
        },

        PoolInstruction::UpdatePoolFees {
            update_flags,
            new_liquidity_fee,
            new_swap_fee,
            pool_id,
        } => process_pool_update_fees(program_id, accounts, update_flags, new_liquidity_fee, new_swap_fee, pool_id),



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
        } => process_system_pause(program_id, reason_code, accounts),

        PoolInstruction::UnpauseSystem => process_system_unpause(program_id, accounts),

        PoolInstruction::GetVersion => process_system_get_version(accounts),
        
        // Treasury Management Instructions
        PoolInstruction::WithdrawTreasuryFees {
            amount,
        } => process_treasury_withdraw_fees(program_id, amount, accounts),

        PoolInstruction::GetTreasuryInfo {} => process_treasury_get_info(program_id, accounts),
        
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
            pool_id,
        } => {
            validate_account_count(accounts, PAUSE_POOL_ACCOUNTS, "PausePool")?;
            process_pool_pause(program_id, pause_flags, pool_id, accounts)
        },
        
        PoolInstruction::UnpausePool {
            unpause_flags,
            pool_id,
        } => {
            validate_account_count(accounts, UNPAUSE_POOL_ACCOUNTS, "UnpausePool")?;
            process_pool_unpause(program_id, unpause_flags, pool_id, accounts)
        },
        
        PoolInstruction::DonateSol {
            amount,
            message,
        } => {
            validate_account_count(accounts, DONATE_SOL_ACCOUNTS, "DonateSol")?;
            process_treasury_donate_sol(program_id, amount, message, accounts)
        },
        
        PoolInstruction::ProcessAdminChange {
            new_admin,
        } => {
            validate_account_count(accounts, PROCESS_ADMIN_CHANGE_ACCOUNTS, "ProcessAdminChange")?;
            process_admin_change(program_id, new_admin, accounts)
        },
    }
}

pub use crate::types::errors::PoolError;

// Public utilities


