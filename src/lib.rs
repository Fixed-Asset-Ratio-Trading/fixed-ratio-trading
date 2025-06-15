#![allow(deprecated)]
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

// This is the main library for the fixed-ratio-trading program
// It contains the program's instructions, error handling, and other functionality
// It also contains the program's constants and PDA seeds
// It is used by the program's entrypoint and other modules


use borsh::BorshDeserialize;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    declare_id,
};

declare_id!("quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD");

// Declare the entrypoint to the Solana runtime
#[cfg(not(feature = "no-entrypoint"))]
use solana_program::entrypoint;
#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

// Module declarations
pub mod client_sdk;
pub mod constants;
pub mod types;
pub mod processors;
pub mod utils;

// Re-export all modules for public API
pub use constants::*;
pub use types::*;
pub use processors::*;
pub use utils::*;











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
    // Deserialize instruction
    let instruction = PoolInstruction::try_from_slice(instruction_data)?;
    
    // Check if pool is paused (skip for management operations)
    if !matches!(instruction, 
        PoolInstruction::WithdrawFees 
        | PoolInstruction::UpdateSecurityParams { .. }
        | PoolInstruction::CreatePoolStateAccount { .. }
        | PoolInstruction::InitializePoolData { .. }
        | PoolInstruction::InitializePool { .. }
    ) {
        // Get the correct pool state account index based on instruction type
        let pool_state_index = match instruction {
            PoolInstruction::Deposit { .. } => 2,                    // accounts[2] for deposit
            PoolInstruction::DepositWithFeatures { .. } => 2,        // accounts[2] for deposit with features
            PoolInstruction::Withdraw { .. } => 3,                   // accounts[3] for withdraw  
            PoolInstruction::Swap { .. } => 2,                       // accounts[2] for swap (assuming similar to deposit)
            PoolInstruction::SetSwapFee { .. } => 1,                 // accounts[1] for set swap fee
            PoolInstruction::RequestFeeWithdrawal { .. } => 1,       // accounts[1] for fee withdrawal request
            PoolInstruction::CancelWithdrawalRequest => 1,           // accounts[1] for cancel request
            PoolInstruction::AddDelegate { .. } => 1,                // accounts[1] for add delegate
            PoolInstruction::RemoveDelegate { .. } => 1,             // accounts[1] for remove delegate
            PoolInstruction::WithdrawFeesToDelegate { .. } => 1,     // accounts[1] for delegate withdrawal
            PoolInstruction::SetDelegateWaitTime { .. } => 1,        // accounts[1] for delegate wait time
            PoolInstruction::GetWithdrawalHistory => 1,              // accounts[1] for withdrawal history
            PoolInstruction::RequestPoolPause { .. } => 1,           // accounts[1] for pool pause request
            PoolInstruction::CancelPoolPause => 1,                   // accounts[1] for cancel pool pause
            PoolInstruction::SetPoolPauseWaitTime { .. } => 1,       // accounts[1] for pause wait time
            PoolInstruction::GetPoolStatePDA { .. } => return Ok(()), // Utility function, no pause check needed
            PoolInstruction::GetTokenVaultPDAs { .. } => return Ok(()), // Utility function, no pause check needed
            PoolInstruction::GetPoolInfo { .. } => 0,                // accounts[0] for pool info
            PoolInstruction::GetLiquidityInfo { .. } => 0,           // accounts[0] for liquidity info
            PoolInstruction::GetDelegateInfo { .. } => 0,            // accounts[0] for delegate info
            PoolInstruction::GetFeeInfo { .. } => 0,                 // accounts[0] for fee info
            _ => 0, // Default fallback for any missed instructions
        };
        
        if pool_state_index < accounts.len() {
            let pool_state_account = &accounts[pool_state_index];
            if let Ok(pool_state) = PoolState::try_from_slice(&pool_state_account.data.borrow()) {
                if pool_state.is_paused {
                    return Err(PoolError::PoolPaused.into());
                }
            }
        }
    }
    
    // Dispatch to appropriate handler
    match instruction {
        // Core pool operations
        PoolInstruction::InitializePool { 
            ratio_primary_per_base, pool_authority_bump_seed, 
            primary_token_vault_bump_seed, base_token_vault_bump_seed 
        } => process_initialize_pool(program_id, accounts, ratio_primary_per_base, 
                                   pool_authority_bump_seed, primary_token_vault_bump_seed, base_token_vault_bump_seed),
        
        PoolInstruction::Deposit { deposit_token_mint, amount } => 
            process_deposit(program_id, accounts, deposit_token_mint, amount),
        
        PoolInstruction::DepositWithFeatures { deposit_token_mint, amount, minimum_lp_tokens_out, fee_recipient } => 
            process_deposit_with_features(program_id, accounts, deposit_token_mint, amount, minimum_lp_tokens_out, fee_recipient),
        
        PoolInstruction::Withdraw { withdraw_token_mint, lp_amount_to_burn } => 
            process_withdraw(program_id, accounts, withdraw_token_mint, lp_amount_to_burn),
        
        PoolInstruction::Swap { input_token_mint, amount_in, minimum_amount_out } => 
            process_swap(program_id, accounts, input_token_mint, amount_in, minimum_amount_out),

        // Fee management
        PoolInstruction::WithdrawFees => process_withdraw_fees(program_id, accounts),
        PoolInstruction::SetSwapFee { fee_basis_points } => process_set_swap_fee(program_id, accounts, fee_basis_points),
        PoolInstruction::RequestFeeWithdrawal { token_mint, amount } => 
            process_request_fee_withdrawal(program_id, accounts, token_mint, amount),
        PoolInstruction::CancelWithdrawalRequest => process_cancel_withdrawal_request(program_id, accounts),

        // Security and governance
        PoolInstruction::UpdateSecurityParams { max_withdrawal_percentage, withdrawal_cooldown, is_paused } => 
            process_update_security_params(program_id, accounts, max_withdrawal_percentage, withdrawal_cooldown, is_paused),
        
        // Delegate management
        PoolInstruction::AddDelegate { delegate } => process_add_delegate(program_id, accounts, delegate),
        PoolInstruction::RemoveDelegate { delegate } => process_remove_delegate(program_id, accounts, delegate),
        PoolInstruction::WithdrawFeesToDelegate { token_mint, amount } => 
            process_withdraw_fees_to_delegate(program_id, accounts, token_mint, amount),
        PoolInstruction::SetDelegateWaitTime { delegate, wait_time } => 
            process_set_delegate_wait_time(program_id, accounts, delegate, wait_time),
        PoolInstruction::GetWithdrawalHistory => process_get_withdrawal_history(program_id, accounts),

        // Pool pause governance
        PoolInstruction::RequestPoolPause { reason, duration_seconds } => 
            process_request_pool_pause(program_id, accounts, reason, duration_seconds),
        PoolInstruction::CancelPoolPause => process_cancel_pool_pause(program_id, accounts),
        PoolInstruction::SetPoolPauseWaitTime { delegate, wait_time } => 
            process_set_pool_pause_wait_time(program_id, accounts, delegate, wait_time),
        
        // Utility functions
        PoolInstruction::GetPoolStatePDA { primary_token_mint, base_token_mint, ratio_primary_per_base } => 
            get_pool_state_pda(program_id, primary_token_mint, base_token_mint, ratio_primary_per_base),
        PoolInstruction::GetTokenVaultPDAs { pool_state_pda } => get_token_vault_pdas(program_id, pool_state_pda),
        PoolInstruction::GetPoolInfo {} => get_pool_info(accounts),
        PoolInstruction::GetLiquidityInfo {} => get_liquidity_info(accounts),
        PoolInstruction::GetDelegateInfo {} => get_delegate_info(accounts),
        PoolInstruction::GetFeeInfo {} => get_fee_info(accounts),

        // Legacy deprecated handlers (backward compatibility)
        PoolInstruction::CreatePoolStateAccount { 
            ratio_primary_per_base, pool_authority_bump_seed, 
            primary_token_vault_bump_seed, base_token_vault_bump_seed 
        } => process_create_pool_state_account(program_id, accounts, ratio_primary_per_base, 
                                             pool_authority_bump_seed, primary_token_vault_bump_seed, base_token_vault_bump_seed),
        
        PoolInstruction::InitializePoolData { 
            ratio_primary_per_base, pool_authority_bump_seed, 
            primary_token_vault_bump_seed, base_token_vault_bump_seed 
        } => process_initialize_pool_data(program_id, accounts, ratio_primary_per_base, 
                                        pool_authority_bump_seed, primary_token_vault_bump_seed, base_token_vault_bump_seed),
    }
}