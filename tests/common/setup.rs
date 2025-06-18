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

//! # Test Environment Setup Utilities
//! 
//! This module provides utilities for setting up test environments,
//! including program test creation, account initialization, and
//! common test scaffolding.

use solana_program_test::{BanksClient, ProgramTest, processor};
use solana_sdk::{signature::Keypair, signer::Signer};
use crate::common::{constants, PROGRAM_ID};
use fixed_ratio_trading::process_instruction;
use std::env;
use env_logger;

/// Test environment context
/// 
/// Contains all the basic components needed for a test environment
pub struct TestEnvironment {
    pub banks_client: BanksClient,
    pub payer: Keypair,
    pub recent_blockhash: solana_sdk::hash::Hash,
}

/// Complete pool test context
/// 
/// Contains a test environment plus pool-specific components
pub struct PoolTestContext {
    pub env: TestEnvironment,
    #[allow(dead_code)]
    pub primary_mint: Keypair,
    #[allow(dead_code)]
    pub base_mint: Keypair,
    pub lp_token_a_mint: Keypair,
    pub lp_token_b_mint: Keypair,
}

/// Create a basic program test environment
/// 
/// Sets up the program test with the fixed-ratio-trading program
/// 
/// # Returns
/// Configured ProgramTest instance
pub fn create_program_test() -> ProgramTest {
    let mut program_test = ProgramTest::new(
        "fixed-ratio-trading",
        PROGRAM_ID,
        processor!(process_instruction),
    );
    
    // Minimize logging output
    program_test.set_compute_max_units(100_000);
    program_test
}

/// Start a basic test environment
/// 
/// Creates and starts a test environment with the fixed-ratio-trading program
/// 
/// # Returns
/// TestEnvironment with banks client, payer, and recent blockhash
pub async fn start_test_environment() -> TestEnvironment {
    // Set minimal logging
    env::set_var("RUST_LOG", "error,solana_runtime::message_processor::stable_log=error");
    let _ = env_logger::try_init();
    
    let program_test = create_program_test();
    let (banks_client, payer, recent_blockhash) = program_test.start().await;
    
    TestEnvironment {
        banks_client,
        payer,
        recent_blockhash,
    }
}

/// Start a test environment with debug logging
/// 
/// Same as start_test_environment but with enhanced logging for debugging
/// 
/// # Returns
/// TestEnvironment with debug logging enabled
pub async fn start_test_environment_with_debug() -> TestEnvironment {
    std::env::set_var("RUST_LOG", "debug,solana_runtime::message_processor::stable_log=debug");
    let _ = env_logger::try_init();
    
    let program_test = create_program_test();
    let (banks_client, payer, recent_blockhash) = program_test.start().await;
    
    TestEnvironment {
        banks_client,
        payer,
        recent_blockhash,
    }
}

/// Setup a complete pool test context
/// 
/// Creates a test environment and initializes all the keypairs needed for pool testing
/// 
/// # Arguments
/// * `with_debug_logging` - Whether to enable debug logging
/// 
/// # Returns
/// PoolTestContext with environment and all required keypairs
pub async fn setup_pool_test_context(with_debug_logging: bool) -> PoolTestContext {
    let env = if with_debug_logging {
        start_test_environment_with_debug().await
    } else {
        start_test_environment().await
    };

    let primary_mint = Keypair::new();
    let base_mint = Keypair::new();
    let lp_token_a_mint = Keypair::new();
    let lp_token_b_mint = Keypair::new();

    PoolTestContext {
        env,
        primary_mint,
        base_mint,
        lp_token_a_mint,
        lp_token_b_mint,
    }
}

/// Create and fund a test user account
#[allow(dead_code)]
/// 
/// Creates a new keypair and funds it with SOL from the payer
/// 
/// # Arguments
/// * `banks` - Banks client for transaction processing
/// * `payer` - Account that provides the funding
/// * `recent_blockhash` - Recent blockhash for transaction
/// * `sol_amount` - Amount of SOL to fund (uses default if None)
/// 
/// # Returns
/// Funded user keypair
pub async fn create_funded_user(
    banks: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
    sol_amount: Option<u64>,
) -> Result<Keypair, solana_program_test::BanksClientError> {
    let user = Keypair::new();
    let amount = sol_amount.unwrap_or(constants::DEFAULT_SOL_AIRDROP);

    let transfer_ix = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &user.pubkey(),
        amount,
    );

    let mut transaction = solana_sdk::transaction::Transaction::new_with_payer(
        &[transfer_ix],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer], recent_blockhash);
    banks.process_transaction(transaction).await?;

    Ok(user)
}

/// Create multiple funded test users
/// 
/// # Arguments
/// * `banks` - Banks client for transaction processing
/// * `payer` - Account that provides the funding
/// * `recent_blockhash` - Recent blockhash for transaction
/// * `count` - Number of users to create
/// * `sol_amount` - Amount of SOL to fund each user (uses default if None)
/// 
/// # Returns
/// Vector of funded user keypairs
#[allow(dead_code)]
pub async fn create_multiple_funded_users(
    banks: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
    count: usize,
    sol_amount: Option<u64>,
) -> Result<Vec<Keypair>, solana_program_test::BanksClientError> {
    let mut users = Vec::with_capacity(count);
    
    for _ in 0..count {
        let user = create_funded_user(banks, payer, recent_blockhash, sol_amount).await?;
        users.push(user);
    }
    
    Ok(users)
}

/// Get account balance in SOL
/// 
/// # Arguments
/// * `banks` - Banks client for account fetching
/// * `account` - Account to check balance for
/// 
/// # Returns
/// Account balance in lamports, or 0 if account doesn't exist
#[allow(dead_code)]
pub async fn get_sol_balance(
    banks: &mut BanksClient,
    account: &solana_program::pubkey::Pubkey,
) -> u64 {
    match banks.get_account(*account).await {
        Ok(Some(account)) => account.lamports,
        _ => 0,
    }
}

/// Wait for a specified number of slots
/// 
/// Useful for testing time-dependent functionality like cooldowns
/// 
/// # Arguments
/// * `banks` - Banks client 
/// * `slots` - Number of slots to wait
#[allow(dead_code)]
pub async fn wait_slots(banks: &mut BanksClient, slots: u64) -> Result<(), solana_program_test::BanksClientError> {
    // In solana-program-test, we can't actually wait for slots to pass
    // This is a placeholder for when time-dependent tests are needed
    // In a real environment, you might use warp_to_slot or similar
    let _ = banks.get_root_slot().await?;
    
    // For testing purposes, this is a no-op
    // Real implementations would advance the clock
    println!("Note: wait_slots is a no-op in test environment (requested {} slots)", slots);
    
    Ok(())
}

/// Advance the test environment clock
/// 
/// # Arguments
/// * `banks` - Banks client
/// * `seconds` - Number of seconds to advance
#[allow(dead_code)]
pub async fn advance_clock(banks: &mut BanksClient, seconds: u64) -> Result<(), solana_program_test::BanksClientError> {
    // Similar to wait_slots, this is a placeholder for clock advancement
    // In solana-program-test, time manipulation is limited
    let _ = banks.get_root_slot().await?;
    
    println!("Note: advance_clock is a no-op in test environment (requested {} seconds)", seconds);
    
    Ok(())
}

/// Setup test environment with custom configuration
/// 
/// # Arguments
/// * `debug_logging` - Enable debug logging
/// * `additional_accounts` - Additional accounts to create and fund
/// 
/// # Returns
/// TestEnvironment with additional setup
#[allow(dead_code)]
pub async fn setup_custom_test_environment(
    debug_logging: bool,
    additional_accounts: Option<Vec<u64>>, // SOL amounts for additional accounts
) -> Result<(TestEnvironment, Vec<Keypair>), solana_program_test::BanksClientError> {
    let mut env = if debug_logging {
        start_test_environment_with_debug().await
    } else {
        start_test_environment().await
    };

    let mut additional_keypairs = Vec::new();

    if let Some(sol_amounts) = additional_accounts {
        for sol_amount in sol_amounts {
            let keypair = create_funded_user(
                &mut env.banks_client,
                &env.payer,
                env.recent_blockhash,
                Some(sol_amount),
            ).await?;
            additional_keypairs.push(keypair);
        }
    }

    Ok((env, additional_keypairs))
}

/// Test helper to verify account exists
/// 
/// # Arguments
/// * `banks` - Banks client
/// * `account` - Account to check
/// 
/// # Returns
/// True if account exists, false otherwise
#[allow(dead_code)]
pub async fn account_exists(
    banks: &mut BanksClient,
    account: &solana_program::pubkey::Pubkey,
) -> bool {
    banks.get_account(*account).await.unwrap_or(None).is_some()
}

/// Test helper to get account data length
/// 
/// # Arguments
/// * `banks` - Banks client
/// * `account` - Account to check
/// 
/// # Returns
/// Account data length, or 0 if account doesn't exist
#[allow(dead_code)]
pub async fn get_account_data_len(
    banks: &mut BanksClient,
    account: &solana_program::pubkey::Pubkey,
) -> usize {
    match banks.get_account(*account).await {
        Ok(Some(account)) => account.data.len(),
        _ => 0,
    }
} 