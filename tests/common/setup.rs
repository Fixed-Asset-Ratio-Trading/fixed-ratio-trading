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
    constants::{
        SYSTEM_STATE_SEED_PREFIX,
        MAIN_TREASURY_SEED_PREFIX,
    },
    process_instruction,
};


use std::env;
use env_logger;
use borsh::BorshSerialize;

// =============================================================================
// TEST-ONLY CONSTANTS
// =============================================================================
// WARNING: These constants are for testing purposes ONLY and should NEVER be
// used in production deployments. The private keys are publicly visible and
// provide no security.

/// Test program authority public key for testing
/// 
/// This is the program authority used specifically for testing. The corresponding
/// keypair is loaded from target/deploy/PROGRAM_AUTHORITY-keypair.json.
/// 
/// **IMPORTANT:** This is a test-only keypair generated specifically for testing.
/// The private key is stored in the repository for testing purposes only.
/// 
/// **NEVER use this authority in production deployments!**
#[allow(dead_code)]
pub const TEST_PROGRAM_AUTHORITY: &str = "6SBHtCjRodUsFrsHEGjf4WH1v1kU2CMKHNQKFhTfYNQn";

/// **HARDCODED TEST PROGRAM AUTHORITY KEYPAIR**
/// 
/// This keypair is hardcoded for testing purposes to avoid any risk of accidental
/// key releases or confusion about which key is being used. The private key is
/// intentionally visible in the source code as it's ONLY for testing.
/// 
/// **SECURITY WARNING:** This keypair is hardcoded in the repository for testing
/// purposes only. It should NEVER be used in production deployments.
/// 
/// **Public Key:** 6SBHtCjRodUsFrsHEGjf4WH1v1kU2CMKHNQKFhTfYNQn
/// 
/// # Returns
/// * `Result<Keypair, Box<dyn std::error::Error>>` - The test authority keypair or error
#[allow(dead_code)]
pub fn create_test_program_authority_keypair() -> Result<solana_sdk::signature::Keypair, Box<dyn std::error::Error>> {
    use solana_sdk::signature::Keypair;
    use std::str::FromStr;
    
    // HARDCODED test keypair bytes - NEVER use in production!
    // This ensures consistent testing without file dependencies or accidental key releases
    let keypair_bytes = [
        163, 234,  36, 177,  75, 126, 161, 135,
        163, 241, 103,  15,  75,  15, 167,  73,
        233,  11, 113, 216, 162, 207,  50,  60,
         60, 172,  13, 230,  60,  27,  56, 134,
         80, 189, 151,  77,  71, 242, 203, 226,
         23, 157,  38,  50, 145, 212, 227, 241,
         10, 174,   8,  87, 229,  18, 141,  49,
        234,  58,  87,  52, 160,   2, 239, 207,
    ];
    
    let keypair = Keypair::from_bytes(&keypair_bytes)
        .map_err(|e| format!("Failed to create hardcoded test keypair: {}", e))?;
    
    // Verify the keypair matches our expected public key
    let expected_pubkey = solana_program::pubkey::Pubkey::from_str(TEST_PROGRAM_AUTHORITY)
        .map_err(|e| format!("Invalid TEST_PROGRAM_AUTHORITY constant: {}", e))?;
    
    if keypair.pubkey() != expected_pubkey {
        return Err(format!(
            "Hardcoded keypair mismatch! Expected: {}, Got: {}",
            expected_pubkey, keypair.pubkey()
        ).into());
    }
    
    Ok(keypair)
}

/// Helper function to get program data account address for testing
/// 
/// This function derives the program data account address for the test program,
/// which is needed for program upgrade authority validation.
/// 
/// # Arguments
/// * `program_id` - The program ID
/// 
/// # Returns
/// * `Pubkey` - The program data account address
pub fn get_test_program_data_address(program_id: &Pubkey) -> Pubkey {
    use solana_program::bpf_loader_upgradeable;
    Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::id()).0
}

/// Helper function to create program upgrade authority account meta for testing
/// 
/// This creates the AccountMeta needed for program upgrade authority validation
/// in test transactions.
/// 
/// # Arguments
/// * `program_id` - The program ID
/// * `authority_keypair` - The authority keypair
/// 
/// # Returns
/// * `Vec<AccountMeta>` - Account metas for authority validation
#[allow(dead_code)]
pub fn create_program_authority_account_metas(
    program_id: &Pubkey,
    authority_keypair: &Keypair,
) -> Vec<AccountMeta> {
    let program_data_address = get_test_program_data_address(program_id);
    
    vec![
        AccountMeta::new(authority_keypair.pubkey(), true),  // Program authority (signer)
        AccountMeta::new_readonly(solana_program::system_program::id(), false), // System program
        AccountMeta::new_readonly(solana_program::sysvar::rent::id(), false),   // Rent sysvar
        AccountMeta::new_readonly(program_data_address, false),  // Program data account
    ]
}

/// Verify that the test program authority matches the hardcoded keypair
/// 
/// This function ensures that the TEST_PROGRAM_AUTHORITY constant matches
/// the hardcoded keypair. This is a safety check to prevent mismatches.
/// 
/// # Arguments
/// * `keypair` - The hardcoded keypair
/// 
/// # Returns
/// * `Result<(), String>` - Ok if they match, error message if they don't
#[allow(dead_code)]
pub fn verify_test_program_authority_consistency(keypair: &Keypair) -> Result<(), String> {
    use std::str::FromStr;
    
    let expected_pubkey = Pubkey::from_str(TEST_PROGRAM_AUTHORITY)
        .map_err(|e| format!("Invalid TEST_PROGRAM_AUTHORITY constant: {}", e))?;
    
    if keypair.pubkey() != expected_pubkey {
        return Err(format!(
            "TEST_PROGRAM_AUTHORITY constant ({}) does not match hardcoded keypair ({})",
            expected_pubkey,
            keypair.pubkey()
        ));
    }
    
    Ok(())
}

// =============================================================================
// TEST ENVIRONMENT STRUCTURES
// =============================================================================

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
        fixed_ratio_trading::id(),
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

/// Initialize treasury system for tests (creates the SystemState and Treasury PDAs)
/// This creates the foundation treasury infrastructure required for pool operations
#[allow(dead_code)]
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
        &fixed_ratio_trading::id()
    );
    let (main_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX], 
        &fixed_ratio_trading::id()
    );
    let program_data_address = get_test_program_data_address(&fixed_ratio_trading::id());
    
    // Create InitializeProgram instruction with Phase 12 program upgrade authority account ordering (6 accounts)
    let initialize_program_ix = Instruction {
        program_id: fixed_ratio_trading::id(),
        accounts: vec![
            // Phase 12 program upgrade authority account ordering (6 accounts total)
            AccountMeta::new(system_authority.pubkey(), true),                       // Index 0: Program Authority (signer, writable)
            AccountMeta::new_readonly(solana_program::system_program::id(), false), // Index 1: System Program (readable)
            AccountMeta::new_readonly(solana_program::sysvar::rent::id(), false),   // Index 2: Rent Sysvar (readable)
            AccountMeta::new(system_state_pda, false),                              // Index 3: System State PDA (writable)
            AccountMeta::new(main_treasury_pda, false),                             // Index 4: Main Treasury PDA (writable)
            AccountMeta::new_readonly(program_data_address, false),                 // Index 5: Program Data Account (readable)
        ],
        data: fixed_ratio_trading::PoolInstruction::InitializeProgram {
            // No fields needed - system authority comes from accounts[0]
        }.try_to_vec().unwrap(),
    };

    let mut transaction = Transaction::new_with_payer(&[initialize_program_ix], Some(&payer.pubkey()));
    transaction.sign(&[payer, system_authority], recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    
    println!("✅ Treasury system initialized successfully");
    println!("   • SystemState PDA: {}", system_state_pda);
    println!("   • MainTreasury PDA: {}", main_treasury_pda);
    Ok(())
}