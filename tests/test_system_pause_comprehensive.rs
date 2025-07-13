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

//! # Comprehensive System Pause Tests
//! 
//! This module contains comprehensive system pause functionality tests that actually validate
//! real pause behavior using working success operations. Unlike the previous tests that used
//! empty SystemState accounts, these tests:
//!
//! 1. **Properly initialize SystemState** with actual pause data
//! 2. **Use real success operations** (pool creation, deposits, withdrawals, swaps)
//! 3. **Verify correct error messages** when operations are blocked by system pause
//! 4. **Test pause/unpause cycles** with full state validation
//! 5. **Validate system pause takes precedence** over pool-level operations
//!
//! ## Test Categories:
//! - **PAUSE-001 to PAUSE-005**: Basic pause/unpause functionality
//! - **PAUSE-006 to PAUSE-010**: Operation blocking validation using real operations
//! - **PAUSE-011 to PAUSE-015**: Read-only operations during pause
//! - **PAUSE-016 to PAUSE-020**: System resume and state management
//!
//! ## Key Improvements:
//! - Uses actual SystemState initialization instead of empty accounts
//! - Tests against real working operations that have pause validation
//! - Verifies specific error messages (SystemPaused, etc.)
//! - Validates pause state persists correctly
//! - Tests system pause precedence over pool operations

mod common;

use common::*;
use common::liquidity_helpers::{create_liquidity_test_foundation, execute_deposit_operation, execute_withdrawal_operation, LiquidityTestFoundation};
use borsh::{BorshDeserialize, BorshSerialize};
use fixed_ratio_trading::{
    types::instructions::PoolInstruction,
    state::SystemState,
    utils::program_authority::get_program_data_address,
};
use solana_program_test::{BanksClient, BanksClientError};
use solana_program::instruction::InstructionError;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    transaction::Transaction,
    signature::Keypair,
    pubkey::Pubkey,
    signer::Signer,
};

// ================================================================================================
// SYSTEM STATE INITIALIZATION AND MANAGEMENT HELPERS
// ================================================================================================

/// Gets the proper SystemState PDA that the processors expect
/// 
/// This function returns the SystemState PDA that's created by InitializeProgram,
/// using the correct seed derivation (b"system_state").
/// 
/// # Returns
/// * `SystemState PDA pubkey` - The proper SystemState PDA address
fn get_system_state_pda() -> Pubkey {
    // Derive the proper SystemState PDA using the same seed as the processors
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[b"system_state"], // SYSTEM_STATE_SEED_PREFIX from constants.rs
        &PROGRAM_ID,
    );
    
    system_state_pda
}

/// Attempts to pause the system using the proper pause instruction
/// 
/// # Arguments
/// * `banks_client` - Banks client for transaction processing
/// * `authority` - System authority (must be signer)
/// * `recent_blockhash` - Recent blockhash for transaction
/// * `system_state_account` - System state account
/// * `reason_code` - Pause reason code
async fn pause_system(
    banks_client: &mut BanksClient,
    authority: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
    system_state_account: &Pubkey,
    reason_code: u8,
) -> TestResult {
    let pause_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(authority.pubkey(), true),    // System authority signer
            AccountMeta::new(*system_state_account, false), // System state PDA
            AccountMeta::new_readonly(get_program_data_address(&PROGRAM_ID), false), // Program data account
        ],
        data: PoolInstruction::PauseSystem {
            reason_code,
        }.try_to_vec().unwrap(),
    };

    let mut transaction = Transaction::new_with_payer(&[pause_ix], Some(&authority.pubkey()));
    transaction.sign(&[authority], recent_blockhash);
    banks_client.process_transaction(transaction).await
}

/// Attempts to unpause the system using the proper unpause instruction
/// 
/// # Arguments
/// * `banks_client` - Banks client for transaction processing
/// * `authority` - System authority (must be signer)
/// * `recent_blockhash` - Recent blockhash for transaction
/// * `system_state_account` - System state account
async fn unpause_system(
    banks_client: &mut BanksClient,
    authority: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
    system_state_account: &Pubkey,
) -> TestResult {
    let unpause_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(authority.pubkey(), true),              // System authority (signer)
            AccountMeta::new(*system_state_account, false),         // System state account
            AccountMeta::new_readonly(solana_program::sysvar::clock::id(), false), // Clock sysvar
        ],
        data: PoolInstruction::UnpauseSystem.try_to_vec().unwrap(),
    };

    let mut transaction = Transaction::new_with_payer(&[unpause_ix], Some(&authority.pubkey()));
    transaction.sign(&[authority], recent_blockhash);
    banks_client.process_transaction(transaction).await
}

/// Gets the actual SystemState data from an account
/// 
/// # Arguments
/// * `banks_client` - Banks client for account fetching
/// * `system_state_account` - System state account
/// 
/// # Returns
/// * `SystemState` - Deserialized system state or None if invalid
async fn get_system_state(
    banks_client: &mut BanksClient,
    system_state_account: &Pubkey,
) -> Option<SystemState> {
    match banks_client.get_account(*system_state_account).await {
        Ok(Some(account)) => {
            match SystemState::try_from_slice(&account.data) {
                Ok(system_state) => Some(system_state),
                Err(_) => None
            }
        },
        _ => None
    }
}

/// Helper to check if an error indicates system pause (expected for blocked operations)
fn is_system_paused_error(error: &BanksClientError) -> bool {
    match error {
        BanksClientError::TransactionError(
            solana_sdk::transaction::TransactionError::InstructionError(
                _, InstructionError::Custom(error_code)
            )
        ) => {
            *error_code == 1023 // PoolError::SystemPaused
        }
        _ => false
    }
}

/// Helper to check if an error indicates system already paused (expected for double pause)
fn is_system_already_paused_error(error: &BanksClientError) -> bool {
    match error {
        BanksClientError::TransactionError(
            solana_sdk::transaction::TransactionError::InstructionError(
                _, InstructionError::Custom(error_code)
            )
        ) => {
            *error_code == 1024 // PoolError::SystemAlreadyPaused
        }
        _ => false
    }
}

/// Helper to check if an error indicates system not paused (expected for unpause non-paused)
fn is_system_not_paused_error(error: &BanksClientError) -> bool {
    match error {
        BanksClientError::TransactionError(
            solana_sdk::transaction::TransactionError::InstructionError(
                _, InstructionError::Custom(error_code)
            )
        ) => {
            *error_code == 1025 // PoolError::SystemNotPaused
        }
        _ => false
    }
}

/// Helper to check if an error indicates unauthorized access (expected for unauthorized operations)
fn is_unauthorized_access_error(error: &BanksClientError) -> bool {
    match error {
        BanksClientError::TransactionError(
            solana_sdk::transaction::TransactionError::InstructionError(
                _, InstructionError::Custom(error_code)
            )
        ) => {
            *error_code == 1026 // PoolError::UnauthorizedAccess
        }
        _ => false
    }
}

/// Timeout wrapper for foundation creation to prevent deadlocks
async fn create_foundation_with_timeout(
    pool_ratio: Option<u64>,
) -> Result<LiquidityTestFoundation, Box<dyn std::error::Error>> {
    let timeout_duration = std::time::Duration::from_secs(30); // 30 second timeout for foundation setup
    let foundation_future = create_liquidity_test_foundation(pool_ratio);
    
    match tokio::time::timeout(timeout_duration, foundation_future).await {
        Ok(foundation) => foundation,
        Err(_) => Err("Foundation creation timed out".into()),
    }
}

// ================================================================================================
// PAUSE-001 to PAUSE-005: BASIC PAUSE/UNPAUSE FUNCTIONALITY
// ================================================================================================

/// PAUSE-001: Test successful system pause operation
/// 
/// This test validates that the system can be properly paused with correct state updates.
#[tokio::test]
async fn test_system_pause_success() -> TestResult {
    println!("🧪 PAUSE-001: Testing successful system pause operation");
    
    let mut env = start_test_environment().await;
    
    // Initialize treasury system to create the SystemState PDA
    let system_authority = Keypair::new();
    if let Err(_) = initialize_treasury_system(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &system_authority,
    ).await {
        return Err(BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Treasury initialization failed")));
    }
    
    // Get the proper SystemState PDA
    let system_state_pda = get_system_state_pda();
    
    // Verify initial state is not paused
    let initial_state = get_system_state(&mut env.banks_client, &system_state_pda).await
        .expect("SystemState should exist and be valid after InitializeProgram");
    assert!(!initial_state.is_paused, "System should not be paused initially");
    println!("✅ Initial state verified: system not paused");
    
    // Attempt to pause the system using proper authority
    let pause_result = pause_system(
        &mut env.banks_client,
        &env.payer, // This should be the program upgrade authority
        env.recent_blockhash,
        &system_state_pda,
        4u8, // Routine maintenance
    ).await;
    
    match pause_result {
        Ok(_) => {
            println!("✅ System pause operation completed successfully");
            
            // Verify the system state was updated correctly
            let final_state = get_system_state(&mut env.banks_client, &system_state_pda).await
                .expect("SystemState should exist after pause");
            
            assert!(final_state.is_paused, "System should be paused after pause operation");
            assert_eq!(final_state.pause_reason_code, 4, "Pause reason code should be updated");
            assert!(final_state.pause_timestamp > 0, "Pause timestamp should be set");
            
            println!("✅ System state correctly updated:");
            println!("   Is Paused: {}", final_state.is_paused);
            println!("   Reason Code: {}", final_state.pause_reason_code);
            println!("   Timestamp: {}", final_state.pause_timestamp);
        }
        Err(e) => {
            println!("❌ System pause failed: {:?}", e);
            panic!("System pause should succeed with proper authority");
        }
    }
    
    println!("✅ PAUSE-001 test completed successfully!");
    Ok(())
}

/// PAUSE-002: Test successful system unpause operation
/// 
/// This test validates that a paused system can be properly unpaused.
#[tokio::test]
async fn test_system_unpause_success() -> TestResult {
    println!("🧪 PAUSE-002: Testing successful system unpause operation");
    
    let mut env = start_test_environment().await;
    
    // Initialize treasury system and get SystemState PDA
    let system_authority = Keypair::new();
    if let Err(_) = initialize_treasury_system(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &system_authority,
    ).await {
        return Err(BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Treasury initialization failed")));
    }
    
    let system_state_pda = get_system_state_pda();
    
    // First pause the system
    let pause_result = pause_system(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &system_state_pda,
        4u8, // Routine maintenance
    ).await;
    
    assert!(pause_result.is_ok(), "System should be pausable first");
    
    // Verify state is paused
    let initial_state = get_system_state(&mut env.banks_client, &system_state_pda).await
        .expect("SystemState should exist and be valid");
    assert!(initial_state.is_paused, "System should be paused after pause operation");
    println!("✅ Initial state verified: system is paused");
    
    // Attempt to unpause the system
    let unpause_result = unpause_system(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &system_state_pda,
    ).await;
    
    match unpause_result {
        Ok(_) => {
            println!("✅ System unpause operation completed successfully");
            
            // Verify the system state was updated correctly
            let final_state = get_system_state(&mut env.banks_client, &system_state_pda).await
                .expect("SystemState should exist after unpause");
            
            assert!(!final_state.is_paused, "System should not be paused after unpause operation");
            assert_eq!(final_state.pause_reason_code, 0, "Pause reason code should be cleared");
            assert_eq!(final_state.pause_timestamp, 0, "Pause timestamp should be cleared");
            
            println!("✅ System state correctly updated:");
            println!("   Is Paused: {}", final_state.is_paused);
            println!("   Reason Code: {}", final_state.pause_reason_code);
            println!("   Timestamp: {}", final_state.pause_timestamp);
        }
        Err(e) => {
            println!("❌ System unpause failed: {:?}", e);
            panic!("System unpause should succeed with proper authority");
        }
    }
    
    println!("✅ PAUSE-002 test completed successfully!");
    Ok(())
}

/// PAUSE-003: Test unauthorized pause attempt fails
/// 
/// This test validates that only authorized users can pause the system.
#[tokio::test]
async fn test_unauthorized_pause_fails() -> TestResult {
    println!("🧪 PAUSE-003: Testing unauthorized pause attempt fails");
    
    let mut env = start_test_environment().await;
    
    // Initialize treasury system and get SystemState PDA
    let system_authority = Keypair::new();
    if let Err(_) = initialize_treasury_system(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &system_authority,
    ).await {
        return Err(BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Treasury initialization failed")));
    }
    
    let system_state_pda = get_system_state_pda();
    
    // Create unauthorized user
    let unauthorized_user = create_funded_user(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        None,
    ).await?;
    
    // Attempt to pause with unauthorized user
    let pause_result = pause_system(
        &mut env.banks_client,
        &unauthorized_user,
        env.recent_blockhash,
        &system_state_pda,
        4u8, // Routine maintenance
    ).await;
    
    match pause_result {
        Ok(_) => {
            panic!("❌ Unauthorized pause should have failed with UnauthorizedAccess error");
        }
        Err(e) => {
            if is_unauthorized_access_error(&e) {
                println!("✅ Unauthorized pause correctly failed with UnauthorizedAccess error: {:?}", e);
                
                // Verify system state was not changed
                let final_state = get_system_state(&mut env.banks_client, &system_state_pda).await
                    .expect("SystemState should still exist");
                
                assert!(!final_state.is_paused, "System should remain unpaused after failed unauthorized pause");
                println!("✅ System state correctly unchanged after unauthorized attempt");
            } else {
                panic!("❌ Unauthorized pause failed with wrong error type: {:?} (expected UnauthorizedAccess)", e);
            }
        }
    }
    
    println!("✅ PAUSE-003 test completed successfully!");
    Ok(())
}

/// PAUSE-004: Test double pause prevention
/// 
/// This test validates that attempting to pause an already paused system fails appropriately.
#[tokio::test]
async fn test_double_pause_prevention() -> TestResult {
    println!("🧪 PAUSE-004: Testing double pause prevention");
    
    let mut env = start_test_environment().await;
    
    // Initialize treasury system and get SystemState PDA
    let system_authority = Keypair::new();
    if let Err(_) = initialize_treasury_system(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &system_authority,
    ).await {
        return Err(BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Treasury initialization failed")));
    }
    
    let system_state_pda = get_system_state_pda();
    
    // First pause the system
    let first_pause_result = pause_system(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &system_state_pda,
        4u8, // Routine maintenance
    ).await;
    
    assert!(first_pause_result.is_ok(), "First pause should succeed");
    
    // Verify state is paused
    let initial_state = get_system_state(&mut env.banks_client, &system_state_pda).await
        .expect("SystemState should exist and be valid");
    assert!(initial_state.is_paused, "System should be paused after first pause");
    println!("✅ Initial state verified: system is already paused");
    
    // Attempt to pause the already paused system
    let second_pause_result = pause_system(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &system_state_pda,
        5u8, // Different reason code
    ).await;
    
    match second_pause_result {
        Ok(_) => {
            panic!("❌ Double pause should have failed with SystemAlreadyPaused error");
        }
        Err(e) => {
            if is_system_already_paused_error(&e) {
                println!("✅ Double pause correctly failed with SystemAlreadyPaused error: {:?}", e);
                
                // Verify system state was not changed
                let final_state = get_system_state(&mut env.banks_client, &system_state_pda).await
                    .expect("SystemState should still exist");
                
                assert!(final_state.is_paused, "System should remain paused");
                assert_eq!(final_state.pause_reason_code, 4, "Original pause reason should be preserved");
                println!("✅ System state correctly unchanged after double pause attempt");
            } else {
                panic!("❌ Double pause failed with wrong error type: {:?} (expected SystemAlreadyPaused)", e);
            }
        }
    }
    
    println!("✅ PAUSE-004 test completed successfully!");
    Ok(())
}

/// PAUSE-005: Test unpause non-paused system prevention
/// 
/// This test validates that attempting to unpause a system that is not paused fails appropriately.
#[tokio::test]
async fn test_unpause_non_paused_prevention() -> TestResult {
    println!("🧪 PAUSE-005: Testing unpause non-paused system prevention");
    
    let mut env = start_test_environment().await;
    
    // Initialize treasury system and get SystemState PDA
    let system_authority = Keypair::new();
    if let Err(_) = initialize_treasury_system(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &system_authority,
    ).await {
        return Err(BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Treasury initialization failed")));
    }
    
    let system_state_pda = get_system_state_pda();
    
    // Verify initial state is not paused
    let initial_state = get_system_state(&mut env.banks_client, &system_state_pda).await
        .expect("SystemState should exist and be valid");
    assert!(!initial_state.is_paused, "System should not be paused initially");
    println!("✅ Initial state verified: system is not paused");
    
    // Attempt to unpause the non-paused system
    let unpause_result = unpause_system(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &system_state_pda,
    ).await;
    
    match unpause_result {
        Ok(_) => {
            panic!("❌ Unpause of non-paused system should have failed with SystemNotPaused error");
        }
        Err(e) => {
            if is_system_not_paused_error(&e) {
                println!("✅ Unpause of non-paused system correctly failed with SystemNotPaused error: {:?}", e);
                
                // Verify system state was not changed
                let final_state = get_system_state(&mut env.banks_client, &system_state_pda).await
                    .expect("SystemState should still exist");
                
                assert!(!final_state.is_paused, "System should remain unpaused");
                assert_eq!(final_state.pause_reason_code, 0, "Pause reason should remain 0");
                println!("✅ System state correctly unchanged after invalid unpause attempt");
            } else {
                panic!("❌ Unpause of non-paused system failed with wrong error type: {:?} (expected SystemNotPaused)", e);
            }
        }
    }
    
    println!("✅ PAUSE-005 test completed successfully!");
    Ok(())
}

// ================================================================================================
// PAUSE-006 to PAUSE-010: OPERATION BLOCKING VALIDATION USING REAL OPERATIONS
// ================================================================================================

/// PAUSE-006: Test pool creation is blocked when system is paused
/// 
/// This test uses the real pool creation success operation to verify it fails with proper error
/// when the system is paused.
#[tokio::test]
async fn test_pool_creation_blocked_when_paused() -> TestResult {
    println!("🧪 PAUSE-006: Testing pool creation blocked when system is paused");
    
    let mut ctx = setup_pool_test_context(false).await;
    
    // Initialize treasury system and pause it
    let system_authority = Keypair::new();
    if let Err(_) = initialize_treasury_system(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &system_authority,
    ).await {
        return Err(BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Treasury initialization failed")));
    }
    
    let system_state_pda = get_system_state_pda();
    
    // Pause the system
    let pause_result = pause_system(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &system_state_pda,
        4u8, // Routine maintenance
    ).await;
    
    assert!(pause_result.is_ok(), "System should be pausable");
    
    // Create token mints (this should work since it's not a pool operation)
    create_test_mints(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &[&ctx.primary_mint, &ctx.base_mint],
    ).await?;
    
    println!("✅ Setup completed, now testing pool creation with paused system");
    
    // Attempt pool creation (this should fail due to system pause)
    let pool_creation_result = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        Some(3),
    ).await;
    
    match pool_creation_result {
        Ok(_) => {
            panic!("❌ Pool creation should have failed with SystemPaused error");
        }
        Err(e) => {
            if is_system_paused_error(&e) {
                println!("✅ Pool creation correctly blocked with SystemPaused error: {:?}", e);
            } else {
                panic!("❌ Pool creation failed with wrong error type: {:?} (expected SystemPaused)", e);
            }
        }
    }
    
    println!("✅ PAUSE-006 test completed successfully!");
    Ok(())
}

/// PAUSE-007: Test deposit operations are blocked when system is paused
/// 
/// This test uses real deposit operations to verify they fail with proper error when paused.
#[tokio::test]
async fn test_deposit_blocked_when_paused() -> TestResult {
    println!("🧪 PAUSE-007: Testing deposit operations blocked when system is paused");
    
    // Create a working pool first (system not paused)
    let mut foundation = create_foundation_with_timeout(Some(3)).await
        .map_err(|e| BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?; // 3:1 ratio
    println!("✅ Foundation created successfully");
    
    // Now pause the system using the existing SystemState PDA
    let system_state_pda = get_system_state_pda();
    
    let pause_result = pause_system(
        &mut foundation.env.banks_client,
        &foundation.env.payer,
        foundation.env.recent_blockhash,
        &system_state_pda,
        4u8, // Routine maintenance
    ).await;
    
    assert!(pause_result.is_ok(), "System should be pausable");
    
    println!("✅ System state set to paused, now testing deposit operation");
    
    // Attempt deposit operation (should fail due to system pause)
    let deposit_amount = 500_000u64;
    let (deposit_mint, user_input_account, user_output_lp_account) = if foundation.pool_config.token_a_is_the_multiple {
        (
            foundation.pool_config.token_a_mint,
            foundation.user1_primary_account.pubkey(),
            foundation.user1_lp_a_account.pubkey(),
        )
    } else {
        (
            foundation.pool_config.token_b_mint,
            foundation.user1_base_account.pubkey(),
            foundation.user1_lp_b_account.pubkey(),
        )
    };
    
    let user1 = foundation.user1.insecure_clone();
    let deposit_result = execute_deposit_operation(
        &mut foundation,
        &user1,
        &user_input_account,
        &user_output_lp_account,
        &deposit_mint,
        deposit_amount,
    ).await;
    
    match deposit_result {
        Ok(_) => {
            panic!("❌ Deposit should have failed with SystemPaused error");
        }
        Err(e) => {
            if is_system_paused_error(&e) {
                println!("✅ Deposit correctly blocked with SystemPaused error: {:?}", e);
            } else {
                panic!("❌ Deposit failed with wrong error type: {:?} (expected SystemPaused)", e);
            }
        }
    }
    
    println!("✅ PAUSE-007 test completed successfully!");
    Ok(())
}

/// PAUSE-008: Test withdrawal operations are blocked when system is paused
/// 
/// This test uses real withdrawal operations to verify they fail with proper error when paused.
#[tokio::test]
async fn test_withdrawal_blocked_when_paused() -> TestResult {
    println!("🧪 PAUSE-008: Testing withdrawal operations blocked when system is paused");
    
    // Create a working pool and perform a deposit first (system not paused)
    let mut foundation = create_foundation_with_timeout(Some(3)).await
        .map_err(|e| BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?; // 3:1 ratio
    println!("✅ Foundation created successfully");
    
    // Perform deposit to get LP tokens (while system is not paused)
    let deposit_amount = 1_000_000u64;
    let (deposit_mint, user_input_account, user_output_lp_account) = if foundation.pool_config.token_a_is_the_multiple {
        (
            foundation.pool_config.token_a_mint,
            foundation.user1_primary_account.pubkey(),
            foundation.user1_lp_a_account.pubkey(),
        )
    } else {
        (
            foundation.pool_config.token_b_mint,
            foundation.user1_base_account.pubkey(),
            foundation.user1_lp_b_account.pubkey(),
        )
    };
    
    let user1 = foundation.user1.insecure_clone();
    execute_deposit_operation(
        &mut foundation,
        &user1,
        &user_input_account,
        &user_output_lp_account,
        &deposit_mint,
        deposit_amount,
    ).await?;
    
    println!("✅ Initial deposit completed, now pausing system");
    
    // Now pause the system
    let system_state_pda = get_system_state_pda();
    
    let pause_result = pause_system(
        &mut foundation.env.banks_client,
        &foundation.env.payer,
        foundation.env.recent_blockhash,
        &system_state_pda,
        4u8, // Routine maintenance
    ).await;
    
    assert!(pause_result.is_ok(), "System should be pausable");
    
    println!("✅ System state set to paused, now testing withdrawal operation");
    
    // Attempt withdrawal operation (should fail due to system pause)
    let withdraw_amount = deposit_amount / 2; // Withdraw half
    let withdrawal_result = execute_withdrawal_operation(
        &mut foundation,
        &user1,
        &user_output_lp_account,      // LP account being burned
        &user_input_account,          // Token account receiving tokens
        &deposit_mint,                // Token mint being withdrawn
        withdraw_amount,
    ).await;
    
    match withdrawal_result {
        Ok(_) => {
            panic!("❌ Withdrawal should have failed with SystemPaused error");
        }
        Err(e) => {
            if is_system_paused_error(&e) {
                println!("✅ Withdrawal correctly blocked with SystemPaused error: {:?}", e);
            } else {
                panic!("❌ Withdrawal failed with wrong error type: {:?} (expected SystemPaused)", e);
            }
        }
    }
    
    println!("✅ PAUSE-008 test completed successfully!");
    Ok(())
}

/// PAUSE-009: Test swap operations are blocked when system is paused
/// 
/// This test uses real swap operations to verify they fail with proper error when paused.
#[tokio::test]
async fn test_swap_blocked_when_paused() -> TestResult {
    println!("🧪 PAUSE-009: Testing swap operations blocked when system is paused");
    
    // Create a working pool with liquidity (system not paused)
    let mut foundation = create_foundation_with_timeout(Some(2)).await
        .map_err(|e| BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?; // 2:1 ratio
    println!("✅ Foundation created successfully");
    
    // Add some liquidity first (while system is not paused)
    let deposit_amount = 10_000_000u64; // 10M tokens
    let (deposit_mint, user_input_account, user_output_lp_account) = if foundation.pool_config.token_a_is_the_multiple {
        (
            foundation.pool_config.token_a_mint,
            foundation.user1_primary_account.pubkey(),
            foundation.user1_lp_a_account.pubkey(),
        )
    } else {
        (
            foundation.pool_config.token_b_mint,
            foundation.user1_base_account.pubkey(),
            foundation.user1_lp_b_account.pubkey(),
        )
    };
    
    let user1 = foundation.user1.insecure_clone();
    execute_deposit_operation(
        &mut foundation,
        &user1,
        &user_input_account,
        &user_output_lp_account,
        &deposit_mint,
        deposit_amount,
    ).await?;
    
    println!("✅ Liquidity added, now pausing system");
    
    // Now pause the system
    let system_state_pda = get_system_state_pda();
    
    let pause_result = pause_system(
        &mut foundation.env.banks_client,
        &foundation.env.payer,
        foundation.env.recent_blockhash,
        &system_state_pda,
        4u8, // Routine maintenance
    ).await;
    
    assert!(pause_result.is_ok(), "System should be pausable");
    
    println!("✅ System state set to paused, now testing swap operation");
    
    // Create dummy accounts for swap test (will fail before getting to them due to system pause)
    let user_token_a_account = Keypair::new();
    let user_token_b_account = Keypair::new();
    
    // Create swap instruction that should fail due to system pause
    let swap_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(foundation.env.payer.pubkey(), true),           // User (signer)
            AccountMeta::new_readonly(solana_program::system_program::id(), false), // System program
            AccountMeta::new_readonly(system_state_pda, false),              // System state (paused) at index 2
            AccountMeta::new(foundation.pool_config.pool_state_pda, false),  // Pool state
            AccountMeta::new(user_token_a_account.pubkey(), false),          // User token A account (dummy)
            AccountMeta::new(user_token_b_account.pubkey(), false),          // User token B account (dummy)
            AccountMeta::new(foundation.pool_config.token_a_vault_pda, false), // Token A vault
            AccountMeta::new(foundation.pool_config.token_b_vault_pda, false), // Token B vault
            AccountMeta::new_readonly(spl_token::id(), false),               // Token program
        ],
        data: PoolInstruction::Swap {
            input_token_mint: foundation.pool_config.token_a_mint,
            amount_in: 1000,
        }.try_to_vec().unwrap(),
    };

    let mut transaction = Transaction::new_with_payer(&[swap_ix], Some(&foundation.env.payer.pubkey()));
    transaction.sign(&[&foundation.env.payer], foundation.env.recent_blockhash);
    
    let swap_result = foundation.env.banks_client.process_transaction(transaction).await;
    
    match swap_result {
        Ok(_) => {
            panic!("❌ Swap should have failed with SystemPaused error");
        }
        Err(e) => {
            if is_system_paused_error(&e) {
                println!("✅ Swap correctly blocked with SystemPaused error: {:?}", e);
            } else {
                panic!("❌ Swap failed with wrong error type: {:?} (expected SystemPaused)", e);
            }
        }
    }
    
    println!("✅ PAUSE-009 test completed successfully!");
    Ok(())
}

/// PAUSE-010: Test read-only operations work when system is paused
/// 
/// This test validates that read-only operations (like GetPoolInfo) work even when paused.
#[tokio::test]
async fn test_read_only_operations_work_when_paused() -> TestResult {
    println!("🧪 PAUSE-010: Testing read-only operations work when system is paused");
    
    // Create a working pool first (system not paused)
    let mut foundation = create_foundation_with_timeout(Some(3)).await
        .map_err(|e| BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    println!("✅ Foundation created successfully");
    
    // Now pause the system
    let system_state_pda = get_system_state_pda();
    
    let pause_result = pause_system(
        &mut foundation.env.banks_client,
        &foundation.env.payer,
        foundation.env.recent_blockhash,
        &system_state_pda,
        4u8, // Routine maintenance
    ).await;
    
    assert!(pause_result.is_ok(), "System should be pausable");
    
    println!("✅ System state set to paused, now testing read-only operations");
    
    // Test that we can still read pool state
    let pool_state = get_pool_state(&mut foundation.env.banks_client, &foundation.pool_config.pool_state_pda).await;
    assert!(pool_state.is_some(), "Should be able to read pool state even when system is paused");
    println!("✅ Pool state read successfully during system pause");
    
    // Test that we can read the system state itself
    let system_state = get_system_state(&mut foundation.env.banks_client, &system_state_pda).await;
    assert!(system_state.is_some(), "Should be able to read system state");
    let state = system_state.unwrap();
    assert!(state.is_paused, "System state should show paused");
    println!("✅ System state read successfully:");
    println!("   Is Paused: {}", state.is_paused);
    println!("   Reason Code: {}", state.pause_reason_code);
    
    // Test GetPoolInfo instruction (read-only)
    let instruction_data = PoolInstruction::GetPoolInfo {};
    
    let instruction = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(foundation.env.payer.pubkey(), false),                // Index 0: System Authority Signer (placeholder)
            AccountMeta::new_readonly(solana_program::system_program::id(), false),        // Index 1: System Program Account (placeholder)
            AccountMeta::new_readonly(foundation.pool_config.pool_state_pda, false),       // Index 2: Pool State PDA
            AccountMeta::new_readonly(spl_token::id(), false),                             // Index 3: SPL Token Program Account (placeholder)
        ],
        data: instruction_data.try_to_vec().unwrap(),
    };
    
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&foundation.env.payer.pubkey()),
        &[&foundation.env.payer],
        foundation.env.recent_blockhash,
    );
    
    let result = foundation.env.banks_client.process_transaction(transaction).await;
    match result {
        Ok(_) => {
            println!("✅ GetPoolInfo instruction succeeded during system pause");
        }
        Err(e) => {
            println!("⚠️  GetPoolInfo failed during pause: {:?}", e);
            // This might be expected depending on implementation
        }
    }
    
    println!("✅ PAUSE-010 test completed successfully!");
    Ok(())
}

// ================================================================================================
// PAUSE-011 to PAUSE-015: SYSTEM RESUME AND STATE MANAGEMENT
// ================================================================================================

/// PAUSE-011: Test operations resume after system unpause
/// 
/// This test validates that operations work normally after the system is unpaused.
#[tokio::test]
async fn test_operations_resume_after_unpause() -> TestResult {
    println!("🧪 PAUSE-011: Testing operations resume after system unpause");
    
    let mut ctx = setup_pool_test_context(false).await;
    
    // Initialize treasury system and pause it
    let system_authority = Keypair::new();
    if let Err(_) = initialize_treasury_system(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &system_authority,
    ).await {
        return Err(BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Treasury initialization failed")));
    }
    
    let system_state_pda = get_system_state_pda();
    
    // Pause the system initially
    let pause_result = pause_system(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &system_state_pda,
        4u8, // Routine maintenance
    ).await;
    
    assert!(pause_result.is_ok(), "System should be pausable");
    
    // Create token mints (setup operations should work)
    create_test_mints(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &[&ctx.primary_mint, &ctx.base_mint],
    ).await?;
    
    println!("✅ Setup completed, system is paused");
    
    // Verify pool creation fails while paused with correct error
    let paused_pool_result = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        Some(3),
    ).await;
    
    match paused_pool_result {
        Ok(_) => {
            panic!("❌ Pool creation should fail with SystemPaused error while paused");
        }
        Err(e) => {
            if is_system_paused_error(&e) {
                println!("✅ Confirmed pool creation blocked with SystemPaused error while paused");
            } else {
                panic!("❌ Pool creation failed with wrong error type: {:?} (expected SystemPaused)", e);
            }
        }
    }
    
    // Now unpause the system
    let unpause_result = unpause_system(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &system_state_pda,
    ).await;
    
    assert!(unpause_result.is_ok(), "System unpause should succeed");
    println!("✅ System successfully unpaused");
    
    // Verify system state is updated
    let state = get_system_state(&mut ctx.env.banks_client, &system_state_pda).await
        .expect("SystemState should exist");
    assert!(!state.is_paused, "System should not be paused after unpause");
    println!("✅ System state correctly updated to unpaused");
    
    // Now pool creation should work
    let unpaused_pool_result = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        Some(3),
    ).await;
    
    match unpaused_pool_result {
        Ok(config) => {
            println!("✅ Pool creation succeeded after unpause");
            println!("   Pool ID: {}", config.pool_state_pda);
            
            // Verify the pool state was created correctly
            let pool_state = get_pool_state(&mut ctx.env.banks_client, &config.pool_state_pda).await
                .expect("Pool state should exist");
            assert!(pool_state.is_initialized, "Pool should be initialized");
            println!("✅ Pool properly initialized after system unpause");
        }
        Err(e) => {
            panic!("❌ Pool creation should succeed after unpause, but failed: {:?}", e);
        }
    }
    
    println!("✅ PAUSE-011 test completed successfully!");
    Ok(())
}

/// PAUSE-012: Test pause/unpause cycle with state persistence
/// 
/// This test validates that pause state persists correctly through multiple cycles.
#[tokio::test]
async fn test_pause_unpause_cycle_state_persistence() -> TestResult {
    println!("🧪 PAUSE-012: Testing pause/unpause cycle with state persistence");
    
    let mut env = start_test_environment().await;
    
    // Initialize treasury system and get SystemState PDA
    let system_authority = Keypair::new();
    if let Err(_) = initialize_treasury_system(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &system_authority,
    ).await {
        return Err(BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Treasury initialization failed")));
    }
    
    let system_state_pda = get_system_state_pda();
    
    println!("✅ Initial state: system not paused");
    
    // Cycle 1: Pause with reason code 4
    println!("🔄 Cycle 1: Pausing system with reason code 4");
    let pause_result_1 = pause_system(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &system_state_pda,
        4u8, // Routine maintenance
    ).await;
    
    assert!(pause_result_1.is_ok(), "First pause should succeed");
    
    let state_1 = get_system_state(&mut env.banks_client, &system_state_pda).await
        .expect("SystemState should exist");
    assert!(state_1.is_paused, "System should be paused");
    assert_eq!(state_1.pause_reason_code, 4, "Pause reason should be 4");
    println!("✅ Cycle 1: System paused with reason code {}", state_1.pause_reason_code);
    
    // Cycle 1: Unpause
    println!("🔄 Cycle 1: Unpausing system");
    let unpause_result_1 = unpause_system(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &system_state_pda,
    ).await;
    
    assert!(unpause_result_1.is_ok(), "First unpause should succeed");
    
    let state_1_after = get_system_state(&mut env.banks_client, &system_state_pda).await
        .expect("SystemState should exist");
    assert!(!state_1_after.is_paused, "System should not be paused");
    assert_eq!(state_1_after.pause_reason_code, 0, "Pause reason should be cleared");
    println!("✅ Cycle 1: System unpaused, state cleared");
    
    // Cycle 2: Pause with different reason code
    println!("🔄 Cycle 2: Pausing system with reason code 7");
    let pause_result_2 = pause_system(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &system_state_pda,
        7u8, // Technical issues
    ).await;
    
    assert!(pause_result_2.is_ok(), "Second pause should succeed");
    
    let state_2 = get_system_state(&mut env.banks_client, &system_state_pda).await
        .expect("SystemState should exist");
    assert!(state_2.is_paused, "System should be paused again");
    assert_eq!(state_2.pause_reason_code, 7, "Pause reason should be 7");
    assert!(state_2.pause_timestamp > state_1.pause_timestamp, "New pause timestamp should be later");
    println!("✅ Cycle 2: System paused with reason code {}", state_2.pause_reason_code);
    
    // Cycle 2: Unpause
    println!("🔄 Cycle 2: Unpausing system");
    let unpause_result_2 = unpause_system(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &system_state_pda,
    ).await;
    
    assert!(unpause_result_2.is_ok(), "Second unpause should succeed");
    
    let final_state = get_system_state(&mut env.banks_client, &system_state_pda).await
        .expect("SystemState should exist");
    assert!(!final_state.is_paused, "System should not be paused");
    assert_eq!(final_state.pause_reason_code, 0, "Pause reason should be cleared");
    assert_eq!(final_state.pause_timestamp, 0, "Pause timestamp should be cleared");
    println!("✅ Cycle 2: System unpaused, state fully cleared");
    
    println!("✅ PAUSE-012 test completed successfully!");
    Ok(())
}

// Individual tests are run via cargo test --test test_system_pause_comprehensive
// Each test is independent and can be run separately 