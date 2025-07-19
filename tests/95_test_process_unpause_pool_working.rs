//! Test process_unpause_pool functionality
//! 
//! This test verifies that process_unpause_pool works correctly by first pausing a pool
//! and then unpausing it, testing the complete pause/unpause cycle.

use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Signer, Keypair},
    instruction::{AccountMeta, Instruction},
    transaction::Transaction,
};
use serial_test::serial;

mod common;
use common::{
    liquidity_helpers::create_liquidity_test_foundation,
};

use fixed_ratio_trading::{
    PoolInstruction,
    constants::*,
    state::PoolState,
};

use borsh::{BorshDeserialize, BorshSerialize};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Test process_unpause_pool with PAUSE_FLAG_LIQUIDITY
/// 
/// This test verifies the complete pause/unpause cycle:
/// 1. Create a pool and verify it's active
/// 2. Pause liquidity operations using PausePool
/// 3. Verify the pool is paused
/// 4. Unpause liquidity operations using UnpausePool  
/// 5. Verify the pool is unpaused
#[tokio::test]
#[serial]
async fn test_process_unpause_pool_liquidity() -> TestResult {
    println!("🧪 Testing process_unpause_pool with PAUSE_FLAG_LIQUIDITY...");
    
    // Create pool foundation
    let mut foundation = create_liquidity_test_foundation(Some(2)).await?;
    println!("✅ Pool foundation created with 2:1 ratio");
    
    // Get PDAs
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &fixed_ratio_trading::id(),
    );
    
    // Step 1: Verify pool is initially active
    println!("🔍 Verifying pool is initially active...");
    let pool_state = foundation.env.banks_client.get_account(foundation.pool_config.pool_state_pda).await?.unwrap();
    let pool_state: PoolState = PoolState::try_from_slice(&pool_state.data)?;
    
    assert!(!pool_state.liquidity_paused(), "Pool liquidity should initially be active");
    println!("✅ Pool liquidity is initially active");
    
    // Step 2: Pause the pool liquidity operations
    println!("⏸️ Pausing pool liquidity operations...");
    
    let pause_instruction = PoolInstruction::PausePool {
        pause_flags: PAUSE_FLAG_LIQUIDITY,
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
    println!("✅ Pool liquidity operations paused");
    
    // Step 3: Verify pool is paused
    println!("🔍 Verifying pool is paused...");
    let pool_state = foundation.env.banks_client.get_account(foundation.pool_config.pool_state_pda).await?.unwrap();
    let pool_state: PoolState = PoolState::try_from_slice(&pool_state.data)?;
    
    assert!(pool_state.liquidity_paused(), "Pool liquidity should be paused");
    println!("✅ Pool liquidity is paused as expected");
    
    // Step 4: Unpause the pool liquidity operations using UnpausePool
    println!("▶️ Unpausing pool liquidity operations...");
    
    let unpause_instruction = PoolInstruction::UnpausePool {
        unpause_flags: PAUSE_FLAG_LIQUIDITY,
    };
    
    let accounts = vec![
        AccountMeta::new(foundation.env.payer.pubkey(), true), // Pool owner (payer is the owner)
        AccountMeta::new(system_state_pda, false),
        AccountMeta::new(foundation.pool_config.pool_state_pda, false),
    ];
    
    let instruction = Instruction {
        program_id: fixed_ratio_trading::id(),
        accounts,
        data: unpause_instruction.try_to_vec()?,
    };
    
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&foundation.env.payer.pubkey()),
        &[&foundation.env.payer],
        foundation.env.recent_blockhash,
    );
    
    foundation.env.banks_client.process_transaction(transaction).await?;
    println!("✅ Pool liquidity operations unpaused");
    
    // Step 5: Verify pool is unpaused
    println!("🔍 Verifying pool is unpaused...");
    let pool_state = foundation.env.banks_client.get_account(foundation.pool_config.pool_state_pda).await?.unwrap();
    let pool_state: PoolState = PoolState::try_from_slice(&pool_state.data)?;
    
    assert!(!pool_state.liquidity_paused(), "Pool liquidity should be unpaused");
    println!("✅ Pool liquidity is unpaused as expected");
    
    println!("🎉 process_unpause_pool test passed! Complete pause/unpause cycle works correctly.");
    
    Ok(())
}

/// Test process_unpause_pool with PAUSE_FLAG_ALL
/// 
/// This test verifies unpausing all operations at once.
#[tokio::test]
#[serial]
async fn test_process_unpause_pool_all_operations() -> TestResult {
    println!("🧪 Testing process_unpause_pool with PAUSE_FLAG_ALL...");
    
    // Create pool foundation
    let mut foundation = create_liquidity_test_foundation(Some(3)).await?;
    println!("✅ Pool foundation created with 3:1 ratio");
    
    // Get PDAs
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &fixed_ratio_trading::id(),
    );
    
    // Step 1: Pause all pool operations
    println!("⏸️ Pausing all pool operations...");
    
    let pause_instruction = PoolInstruction::PausePool {
        pause_flags: PAUSE_FLAG_ALL,
    };
    
    let accounts = vec![
        AccountMeta::new(foundation.env.payer.pubkey(), true),
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
    println!("✅ All pool operations paused");
    
    // Step 2: Verify all operations are paused
    let pool_state = foundation.env.banks_client.get_account(foundation.pool_config.pool_state_pda).await?.unwrap();
    let pool_state: PoolState = PoolState::try_from_slice(&pool_state.data)?;
    
    assert!(pool_state.liquidity_paused(), "Pool liquidity should be paused");
    assert!(pool_state.swaps_paused(), "Pool swaps should be paused");
    println!("✅ All operations are paused as expected");
    
    // Step 3: Unpause all pool operations
    println!("▶️ Unpausing all pool operations...");
    
    let unpause_instruction = PoolInstruction::UnpausePool {
        unpause_flags: PAUSE_FLAG_ALL,
    };
    
    let accounts = vec![
        AccountMeta::new(foundation.env.payer.pubkey(), true),
        AccountMeta::new(system_state_pda, false),
        AccountMeta::new(foundation.pool_config.pool_state_pda, false),
    ];
    
    let instruction = Instruction {
        program_id: fixed_ratio_trading::id(),
        accounts,
        data: unpause_instruction.try_to_vec()?,
    };
    
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&foundation.env.payer.pubkey()),
        &[&foundation.env.payer],
        foundation.env.recent_blockhash,
    );
    
    foundation.env.banks_client.process_transaction(transaction).await?;
    println!("✅ All pool operations unpaused");
    
    // Step 4: Verify all operations are unpaused
    let pool_state = foundation.env.banks_client.get_account(foundation.pool_config.pool_state_pda).await?.unwrap();
    let pool_state: PoolState = PoolState::try_from_slice(&pool_state.data)?;
    
    assert!(!pool_state.liquidity_paused(), "Pool liquidity should be unpaused");
    assert!(!pool_state.swaps_paused(), "Pool swaps should be unpaused");
    println!("✅ All operations are unpaused as expected");
    
    println!("🎉 process_unpause_pool ALL operations test passed!");
    
    Ok(())
} 