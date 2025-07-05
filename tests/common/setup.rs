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
use solana_sdk::{
    signature::Keypair, 
    signer::Signer,
    instruction::{AccountMeta, Instruction},
    transaction::Transaction,
    pubkey::Pubkey,
    system_instruction,
};
use crate::common::constants;
use fixed_ratio_trading::{
    process_instruction,
    PoolInstruction,
    ID as PROGRAM_ID,
    constants::{
        SYSTEM_STATE_SEED_PREFIX,
        MAIN_TREASURY_SEED_PREFIX,
        SWAP_TREASURY_SEED_PREFIX,
        HFT_TREASURY_SEED_PREFIX,
    },
};
use std::env;
use env_logger;
use borsh::BorshSerialize;

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
#[allow(dead_code)]
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
#[allow(dead_code)]
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

/// Update pool state by directly modifying its data (for testing fee simulation)
/// 
/// # Arguments
/// * `banks` - Banks client
/// * `pool_state_pda` - Pool state account
/// * `update_fn` - Function to update the pool state
/// 
/// Note: This function applies the update to the pool state in memory but doesn't
/// persist changes back to the blockchain. In a real test scenario, you would need
/// to use actual program instructions to modify pool state.
#[allow(dead_code)]
pub async fn update_pool_state<F>(
    banks: &mut BanksClient,
    pool_state_pda: &solana_program::pubkey::Pubkey,
    update_fn: F,
) -> Result<fixed_ratio_trading::PoolState, Box<dyn std::error::Error>>
where
    F: FnOnce(&mut fixed_ratio_trading::PoolState),
{
    use fixed_ratio_trading::PoolState;
    use borsh::{BorshDeserialize};
    
    // Get current pool state
    let account = banks.get_account(*pool_state_pda).await?
        .ok_or("Pool state account not found")?;
    
    let mut pool_state = PoolState::deserialize(&mut &account.data[..])?;
    
    // Apply update in memory only
    update_fn(&mut pool_state);
    
    println!("Note: update_pool_state only modifies the pool state in memory");
    println!("For testing, ensure your program has proper instructions to handle fee collection");
    println!("✓ Updated pool state (in memory only): collected fees A: {}, B: {}", 
             pool_state.collected_fees_token_a, pool_state.collected_fees_token_b);
    
    // Return the updated pool state (but it's not persisted on-chain)
    Ok(pool_state)
}

/// Transfer SOL between accounts (convenience function)
#[allow(dead_code)]
pub async fn transfer_sol(
    banks: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
    from: &Keypair,
    to: &solana_program::pubkey::Pubkey,
    amount: u64,
) -> Result<(), solana_program_test::BanksClientError> {
    use solana_sdk::{system_instruction, transaction::Transaction};
    
    let transfer_ix = system_instruction::transfer(&from.pubkey(), to, amount);
    let mut transfer_tx = Transaction::new_with_payer(&[transfer_ix], Some(&payer.pubkey()));
    transfer_tx.sign(&[payer, from], recent_blockhash);
    
    banks.process_transaction(transfer_tx).await
}

/// Initialize the treasury system for tests
/// 
/// This creates all treasury PDAs (MainTreasury, SwapTreasury, HftTreasury) that are
/// required for SOL fee collection in liquidity and swap operations.
pub async fn initialize_treasury_system(
    banks_client: &mut solana_program_test::BanksClient,
    payer: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
    system_authority: &Keypair,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🏦 Initializing treasury system for tests...");
    
    // Fund the system authority account with SOL for account creation fees
    let system_authority_balance = banks_client.get_balance(system_authority.pubkey()).await?;
    if system_authority_balance < 10_000_000_000 {  // 10 SOL
        println!("📦 Airdropping SOL to system authority for account creation...");
        // Transfer SOL from payer to system authority
        let transfer_ix = system_instruction::transfer(
            &payer.pubkey(),
            &system_authority.pubkey(),
            10_000_000_000,  // 10 SOL
        );
        let mut transfer_tx = Transaction::new_with_payer(&[transfer_ix], Some(&payer.pubkey()));
        transfer_tx.sign(&[payer], recent_blockhash);
        banks_client.process_transaction(transfer_tx).await?;
        println!("✅ System authority funded with 10 SOL");
    }
    
    // Derive all required PDA addresses using the actual program constants
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX], 
        &PROGRAM_ID
    );
    let (main_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX], 
        &PROGRAM_ID
    );
    let (swap_treasury_pda, _) = Pubkey::find_program_address(
        &[SWAP_TREASURY_SEED_PREFIX], 
        &PROGRAM_ID
    );
    let (hft_treasury_pda, _) = Pubkey::find_program_address(
        &[HFT_TREASURY_SEED_PREFIX], 
        &PROGRAM_ID
    );
    
    // Create InitializeProgram instruction with standardized account ordering (16 accounts minimum)
    let initialize_program_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            // Standardized account ordering (indices 0-14 + function-specific at 15+)
            AccountMeta::new(system_authority.pubkey(), true),                       // Index 0: Authority/User Signer
            AccountMeta::new_readonly(solana_program::system_program::id(), false), // Index 1: System Program
            AccountMeta::new_readonly(solana_program::sysvar::rent::id(), false),   // Index 2: Rent Sysvar
            AccountMeta::new_readonly(solana_program::sysvar::clock::id(), false),  // Index 3: Clock Sysvar (placeholder)
            AccountMeta::new(payer.pubkey(), false),                                // Index 4: Pool State PDA (placeholder)
            AccountMeta::new_readonly(payer.pubkey(), false),                       // Index 5: Token A Mint (placeholder)
            AccountMeta::new_readonly(payer.pubkey(), false),                       // Index 6: Token B Mint (placeholder)
            AccountMeta::new(payer.pubkey(), false),                                // Index 7: Token A Vault PDA (placeholder)
            AccountMeta::new(payer.pubkey(), false),                                // Index 8: Token B Vault PDA (placeholder)
            AccountMeta::new_readonly(spl_token::id(), false),                      // Index 9: SPL Token Program (placeholder)
            AccountMeta::new(payer.pubkey(), false),                                // Index 10: User Input Token Account (placeholder)
            AccountMeta::new(payer.pubkey(), false),                                // Index 11: User Output Token Account (placeholder)
            AccountMeta::new(main_treasury_pda, false),                             // Index 12: Main Treasury PDA
            AccountMeta::new(swap_treasury_pda, false),                             // Index 13: Swap Treasury PDA
            AccountMeta::new(hft_treasury_pda, false),                              // Index 14: HFT Treasury PDA
            AccountMeta::new(system_state_pda, false),                              // Index 15: System State PDA (function-specific)
        ],
        data: PoolInstruction::InitializeProgram {
            // No fields needed - system authority comes from accounts[0]
        }.try_to_vec().unwrap(),
    };

    let mut transaction = Transaction::new_with_payer(&[initialize_program_ix], Some(&payer.pubkey()));
    transaction.sign(&[payer, system_authority], recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    
    println!("✅ Treasury system initialized successfully");
    println!("   • SystemState PDA: {}", system_state_pda);
    println!("   • MainTreasury PDA: {}", main_treasury_pda);
    println!("   • SwapTreasury PDA: {}", swap_treasury_pda);
    println!("   • HftTreasury PDA: {}", hft_treasury_pda);
    Ok(())
}