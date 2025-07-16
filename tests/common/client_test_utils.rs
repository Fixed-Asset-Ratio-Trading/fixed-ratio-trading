//! Client Test Utilities
//!
//! This module contains test utility functions for client-side testing, moved from main contract code.

use fixed_ratio_trading::client_sdk::{PoolConfig, PoolClient, PoolClientError};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
    sysvar::{self, rent, clock},
};
use spl_token;
use borsh::BorshSerialize;

use fixed_ratio_trading::types::instructions::PoolInstruction;

/// Creates a test pool configuration for testing purposes.
/// 
/// # Returns
/// * `PoolConfig` - A test configuration with random mints and 1000:1 ratio
#[allow(dead_code)]
pub fn create_test_pool_config() -> PoolConfig {
    PoolConfig {
        multiple_token_mint: Pubkey::new_unique(),
        base_token_mint: Pubkey::new_unique(),
        ratio_a_numerator: 1000,
        ratio_b_denominator: 1,
    }
}

/// Test-only PoolState struct for client SDK testing
/// 
/// This is a simplified version of the main PoolState for testing purposes.
/// The actual PoolState is defined in src/state/pool_state.rs
#[derive(Debug, Clone)]
pub struct TestPoolState {
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub ratio_a_numerator: u64,
    pub ratio_b_denominator: u64,
    pub paused: bool,
    /// Future feature: Single LP token mode
    /// NOTE: Currently not implemented - remains false regardless of input
    pub only_lp_token_a_for_both: bool,
}

/// Test-only deposit instruction creation
/// 
/// Creates a deposit instruction for adding liquidity to a pool.
/// 
/// # Arguments
/// * `user` - The user performing the deposit
/// * `config` - Pool configuration
/// * `deposit_token_mint` - Token being deposited
/// * `amount` - Amount to deposit
/// * `user_source_account` - User's token account
/// * `user_lp_account` - User's LP token account
/// 
/// # Returns
/// * `Result<Instruction, PoolClientError>` - The deposit instruction or an error
#[allow(dead_code)]
pub fn create_deposit_instruction(
    pool_client: &PoolClient,
    user: &Pubkey,
    config: &PoolConfig,
    deposit_token_mint: &Pubkey,
    amount: u64,
    user_source_account: &Pubkey,
    user_lp_account: &Pubkey,
) -> Result<Instruction, PoolClientError> {
    let addresses = pool_client.derive_pool_addresses(config);
    
    // Validate deposit token
    if *deposit_token_mint != config.multiple_token_mint && *deposit_token_mint != config.base_token_mint {
        return Err(PoolClientError::InvalidDepositToken);
    }

    let instruction_data = PoolInstruction::Deposit {
        deposit_token_mint: *deposit_token_mint,
        amount,
    };

    let data = instruction_data.try_to_vec()?;

    Ok(Instruction {
        program_id: pool_client.program_id(),
        accounts: vec![
            AccountMeta::new(*user, true),                          // User (signer)
            AccountMeta::new(addresses.pool_state, false),          // Pool state
            AccountMeta::new(*user_source_account, false),          // User source token account
            AccountMeta::new(*user_lp_account, false),              // User LP token account
            AccountMeta::new(addresses.token_a_vault, false),       // Token A vault
            AccountMeta::new(addresses.token_b_vault, false),       // Token B vault
            AccountMeta::new_readonly(system_program::id(), false), // System program
            AccountMeta::new_readonly(spl_token::id(), false),      // SPL Token program
            AccountMeta::new_readonly(rent::id(), false),           // Rent sysvar
            AccountMeta::new_readonly(clock::id(), false),          // Clock sysvar
        ],
        data,
    })
}

/// Test-only withdraw instruction creation
/// 
/// Creates a withdraw instruction for removing liquidity from a pool.
/// 
/// # Arguments
/// * `user` - The user performing the withdrawal
/// * `config` - Pool configuration
/// * `withdraw_token_mint` - Token being withdrawn
/// * `lp_amount_to_burn` - Amount of LP tokens to burn
/// * `user_destination_account` - User's destination token account
/// * `user_lp_account` - User's LP token account
/// 
/// # Returns
/// * `Result<Instruction, PoolClientError>` - The withdraw instruction or an error
#[allow(dead_code)]
pub fn create_withdraw_instruction(
    pool_client: &PoolClient,
    user: &Pubkey,
    config: &PoolConfig,
    withdraw_token_mint: &Pubkey,
    lp_amount_to_burn: u64,
    user_destination_account: &Pubkey,
    user_lp_account: &Pubkey,
) -> Result<Instruction, PoolClientError> {
    let addresses = pool_client.derive_pool_addresses(config);

    let instruction_data = PoolInstruction::Withdraw {
        withdraw_token_mint: *withdraw_token_mint,
        lp_amount_to_burn,
    };

    let data = instruction_data.try_to_vec()?;

    Ok(Instruction {
        program_id: pool_client.program_id(),
        accounts: vec![
            AccountMeta::new(*user, true),                          // User (signer)
            AccountMeta::new(addresses.pool_state, false),          // Pool state
            AccountMeta::new(*user_destination_account, false),     // User destination token account
            AccountMeta::new(*user_lp_account, false),              // User LP token account
            AccountMeta::new(addresses.token_a_vault, false),       // Token A vault
            AccountMeta::new(addresses.token_b_vault, false),       // Token B vault
            AccountMeta::new_readonly(system_program::id(), false), // System program
            AccountMeta::new_readonly(spl_token::id(), false),      // SPL Token program
            AccountMeta::new_readonly(rent::id(), false),           // Rent sysvar
            AccountMeta::new_readonly(clock::id(), false),          // Clock sysvar
        ],
        data,
    })
}

/// Test-only swap instruction creation
/// 
/// Creates a Swap instruction
/// 
/// # Arguments
/// * `user_signer` - User account performing the swap
/// * `user_input_token_account` - User's input token account
/// * `user_output_token_account` - User's output token account  
/// * `pool_state_pda` - Pool state PDA account
/// * `token_a_mint` - Token A mint account
/// * `token_b_mint` - Token B mint account
/// * `pool_token_a_vault` - Pool's Token A vault
/// * `pool_token_b_vault` - Pool's Token B vault
/// * `input_token_mint` - Mint of the token being swapped in
/// * `amount_in` - Amount of input tokens to swap
/// 
/// # Returns
/// * `Result<Instruction, ProgramError>` - The swap instruction or error
#[allow(dead_code)]
pub fn create_swap_instruction(
    pool_client: &PoolClient,
    user_signer: &Pubkey,
    user_input_token_account: &Pubkey,
    user_output_token_account: &Pubkey,
    pool_state_pda: &Pubkey,
    token_a_mint: &Pubkey,
    token_b_mint: &Pubkey,
    pool_token_a_vault: &Pubkey,
    pool_token_b_vault: &Pubkey,
    input_token_mint: Pubkey,
    amount_in: u64,
) -> Result<Instruction, ProgramError> {
    let instruction_data = PoolInstruction::Swap {
        input_token_mint,
        amount_in,
    };

    let accounts = vec![
        AccountMeta::new(*user_signer, true),                     // User (signer)
        AccountMeta::new(*user_input_token_account, false),       // User input token account
        AccountMeta::new(*user_output_token_account, false),      // User output token account
        AccountMeta::new(*pool_state_pda, false),                 // Pool state PDA
        AccountMeta::new_readonly(*token_a_mint, false),          // Token A mint
        AccountMeta::new_readonly(*token_b_mint, false),          // Token B mint
        AccountMeta::new(*pool_token_a_vault, false),             // Pool Token A vault
        AccountMeta::new(*pool_token_b_vault, false),             // Pool Token B vault
        AccountMeta::new_readonly(system_program::id(), false),   // System program
        AccountMeta::new_readonly(spl_token::id(), false),        // SPL Token program
        AccountMeta::new_readonly(rent::id(), false),             // Rent sysvar
        AccountMeta::new_readonly(clock::id(), false),            // Clock sysvar
    ];

    let data = instruction_data.try_to_vec().map_err(|_| ProgramError::InvalidInstructionData)?;

    Ok(Instruction {
        program_id: pool_client.program_id(),
        accounts,
        data,
    })
}

/// Test-only additional operations method
/// 
/// Placeholder for future operations that may be added to the client SDK.
/// Currently returns NotImplemented error.
/// 
/// # Returns
/// * `Result<(), PoolClientError>` - Currently returns NotImplemented
#[allow(dead_code)]
pub fn additional_operations(pool_client: &PoolClient) -> Result<(), PoolClientError> {
    Err(PoolClientError::NotImplemented)
} 