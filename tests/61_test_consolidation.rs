//! Consolidation Tests
//! 
//! This module tests the fee consolidation functionality

use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signer,
    instruction::{AccountMeta, Instruction},
    transaction::Transaction,
};
use serial_test::serial;

mod common;
use common::{
    setup::{start_test_environment, get_sol_balance},
    liquidity_helpers::{create_liquidity_test_foundation},
};

use fixed_ratio_trading::{
    PoolInstruction,
    constants::*,
    state::PoolState,
};

use borsh::{BorshDeserialize, BorshSerialize};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// CONSOLIDATION-001: Test basic consolidation instruction
/// 
/// This test verifies that the consolidation instruction can be called
/// and behaves correctly when a pool is paused.
#[tokio::test]
#[serial]
async fn test_basic_consolidation_instruction() -> TestResult {
    println!("🧪 Testing CONSOLIDATION-001: Basic consolidation instruction...");
    
    // Create pool foundation
    let mut foundation = create_liquidity_test_foundation(Some(2)).await?;
    println!("✅ Pool foundation created with 2:1 ratio");
    
    // Get PDAs
    let (main_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        &fixed_ratio_trading::id(),
    );
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &fixed_ratio_trading::id(),
    );
    
    // Get initial balances
    let initial_treasury_balance = get_sol_balance(&mut foundation.env.banks_client, &main_treasury_pda).await;
    let initial_pool_balance = get_sol_balance(&mut foundation.env.banks_client, &foundation.pool_config.pool_state_pda).await;
    
    println!("Initial balances - Treasury: {} lamports, Pool: {} lamports", 
             initial_treasury_balance, initial_pool_balance);
    
    // Step 1: Pause the pool for consolidation eligibility
    println!("⏸️ Pausing pool for consolidation...");
    
    let pause_instruction = PoolInstruction::PausePool {
        pause_flags: PAUSE_FLAG_ALL,
    };
    
    let accounts = vec![
        AccountMeta::new(foundation.env.payer.pubkey(), true), // Pool owner (payer is the owner)
        AccountMeta::new(system_state_pda, false),
        AccountMeta::new(foundation.pool_config.pool_state_pda, false),
    ];
    
    let instruction = Instruction {
        program_id: fixed_ratio_trading::id(),
        accounts,
        data: pause_instruction.try_to_vec()?,
    };
    
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&foundation.env.payer.pubkey()),
        &[&foundation.env.payer],
        foundation.env.recent_blockhash,
    );
    
    foundation.env.banks_client.process_transaction(transaction).await?;
    println!("✅ Pool paused");
    
    // Step 2: Test consolidation instruction
    println!("💰 Testing consolidation instruction...");
    
    let consolidate_instruction = PoolInstruction::ConsolidatePoolFees {
        pool_count: 1,
    };
    
    let accounts = vec![
        AccountMeta::new(system_state_pda, false),
        AccountMeta::new(main_treasury_pda, false),
        AccountMeta::new(foundation.pool_config.pool_state_pda, false),
    ];
    
    let instruction = Instruction {
        program_id: fixed_ratio_trading::id(),
        accounts,
        data: consolidate_instruction.try_to_vec()?,
    };
    
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&foundation.env.payer.pubkey()),
        &[&foundation.env.payer],
        foundation.env.recent_blockhash,
    );
    
    // This should succeed (even if no fees to consolidate)
    foundation.env.banks_client.process_transaction(transaction).await?;
    println!("✅ Consolidation instruction executed successfully");
    
    // Step 3: Verify pool state is still correct
    let pool_state = foundation.env.banks_client.get_account(foundation.pool_config.pool_state_pda).await?.unwrap();
    let pool_state: PoolState = PoolState::try_from_slice(&pool_state.data)?;
    
    println!("Pool state after consolidation:");
    println!("  - Swaps paused: {}", pool_state.swaps_paused());
    println!("  - Liquidity paused: {}", pool_state.liquidity_paused());
    
    // Verify pool is still paused
    assert!(pool_state.swaps_paused(), "Pool swaps should still be paused");
    assert!(pool_state.liquidity_paused(), "Pool liquidity should still be paused");
    
    println!("✅ CONSOLIDATION-001: Basic consolidation instruction test passed!");
    println!("   - Pool paused successfully");
    println!("   - Consolidation instruction executed without errors");
    println!("   - Pool state remains consistent");
    
    Ok(())
} 