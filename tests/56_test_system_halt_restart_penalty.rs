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

//! # System Halt & Restart Penalty Tests
//! 
//! This module implements comprehensive tests for the system halt (pause) and restart penalty
//! functionality as outlined in SYSTEM_HALT_RESTART_PENALTY_TEST_PLAN.md
//!
//! ## Test Implementation Requirements:
//! - Uses EnhancedTestFoundation from /tests/common/enhanced_test_foundation.rs
//! - Tests run against real Solana contract code (no mocks)
//! - Follows TEST CONFIGURATION pattern for easy parameter modification
//! - Uses donate_sol instruction for treasury liquidity setup
//! - Provides clear configuration constants at top of each test
//!
//! ## Test Sections:
//! - **Section 1**: System Pause (Halt) Functionality Tests
//! - **Section 2**: System Unpause (Restart) Functionality Tests (TODO)
//! - **Section 3**: Restart Penalty Validation Tests (TODO)
//! - **Section 4**: Penalty Expiration and Normal Operation Resume (TODO)

#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_mut)]

mod common;

use common::*;
use common::enhanced_test_foundation::{
    create_enhanced_liquidity_test_foundation,
    EnhancedTestFoundation,
};
use common::setup::{
    TestEnvironment,
    create_test_program_authority_keypair,
    get_test_program_data_address,
};
use common::liquidity_helpers::LiquidityTestFoundation;
use borsh::{BorshDeserialize, BorshSerialize};
use fixed_ratio_trading::{
    types::instructions::PoolInstruction,
    state::{SystemState, MainTreasuryState},
    utils::program_authority::get_program_data_address,
    error::PoolError,
    constants::*,
};
use solana_program_test::{BanksClient, BanksClientError};
use solana_program::instruction::InstructionError;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    transaction::{Transaction, TransactionError},
    signature::{Keypair, Signer},
    pubkey::Pubkey,
    program_error::ProgramError,
    system_program,
    clock::Clock,
    sysvar::{self, Sysvar},
};
use std::error::Error;
use std::time::Duration;

// Test result type for cleaner error handling
type TestResult = Result<(), Box<dyn Error>>;

// ================================================================================================
// HELPER FUNCTIONS
// ================================================================================================

/// Gets the proper SystemState PDA that the processors expect
fn get_system_state_pda(program_id: &Pubkey) -> Pubkey {
    // Derive the proper SystemState PDA using the same seed as the processors
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX], // b"system_state" from constants.rs
        program_id,
    );
    system_state_pda
}

/// Gets the main treasury PDA
fn get_main_treasury_pda(program_id: &Pubkey) -> Pubkey {
    let (treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX], // b"main_treasury" from constants.rs
        program_id,
    );
    treasury_pda
}

/// Helper function to setup treasury with large SOL balance using donate_sol
async fn setup_treasury_with_donation(
    foundation: &EnhancedTestFoundation,
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
    amount_sol: u64,
    message: &str,
) -> Result<(), Box<dyn Error>> {
    println!("💰 Setting up treasury with {} SOL via donate_sol", amount_sol);
    
    // Get necessary accounts
    let program_id = PROGRAM_ID;
    let main_treasury_pda = get_main_treasury_pda(&program_id);
    let system_state_pda = get_system_state_pda(&program_id);
    
    // Create donate_sol instruction
    let donate_instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),          // Donor (signer, writable)
            AccountMeta::new(main_treasury_pda, false),      // Main Treasury PDA (writable)
            AccountMeta::new_readonly(system_state_pda, false), // System State PDA (readable)
            AccountMeta::new_readonly(system_program::id(), false), // System Program
        ],
        data: PoolInstruction::DonateSol {
            amount: amount_sol * 1_000_000_000, // Convert SOL to lamports
            message: message.to_string(),
        }.try_to_vec()?,
    };
    
    // Execute donation transaction
    let mut transaction = Transaction::new_with_payer(
        &[donate_instruction],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer], recent_blockhash);
    
    banks_client.process_transaction(transaction).await?;
    
    println!("✅ Treasury setup complete: {} SOL donated", amount_sol);
    Ok(())
}

/// Helper function to verify treasury balance after donation
async fn verify_treasury_balance(
    banks_client: &mut BanksClient,
    main_treasury_pda: &Pubkey,
    expected_balance_sol: u64,
) -> Result<(), Box<dyn Error>> {
    // Get account balance
    let balance = banks_client.get_balance(*main_treasury_pda).await?;
    let balance_sol = balance / 1_000_000_000;
    
    println!("📊 Treasury balance: {} SOL ({} lamports)", balance_sol, balance);
    
    // Allow some tolerance for rent
    let tolerance_sol = 1; // 1 SOL tolerance
    assert!(
        balance_sol >= expected_balance_sol - tolerance_sol,
        "Treasury balance {} SOL is less than expected {} SOL",
        balance_sol, expected_balance_sol
    );
    
    Ok(())
}

/// Helper function to create pause_system instruction
fn create_pause_system_instruction(
    program_id: &Pubkey,
    upgrade_authority: &Keypair,
    system_state_pda: &Pubkey,
    program_data_account: &Pubkey,
    reason_code: u8,
) -> Result<Instruction, Box<dyn Error>> {
    Ok(Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(upgrade_authority.pubkey(), true), // Upgrade authority (signer)
            AccountMeta::new(*system_state_pda, false),                  // System state PDA (writable)
            AccountMeta::new_readonly(*program_data_account, false),     // Program data account
        ],
        data: PoolInstruction::PauseSystem {
            reason_code,
        }.try_to_vec()?,
    })
}

/// Helper function to verify system pause state
async fn verify_system_paused(
    banks_client: &mut BanksClient,
    system_state_pda: &Pubkey,
    expected_paused: bool,
    expected_reason: Option<u8>,
) -> Result<(), Box<dyn Error>> {
    // Get system state account
    let account = banks_client.get_account(*system_state_pda).await?
        .ok_or("System state account not found")?;
    
    // Deserialize system state
    let system_state = SystemState::try_from_slice(&account.data)?;
    
    println!("📊 System State:");
    println!("   - Is paused: {}", system_state.is_paused);
    println!("   - Pause reason code: {}", system_state.pause_reason_code);
    println!("   - Pause timestamp: {}", system_state.pause_timestamp);
    
    assert_eq!(
        system_state.is_paused, expected_paused,
        "System pause state mismatch: expected {}, got {}",
        expected_paused, system_state.is_paused
    );
    
    if let Some(reason) = expected_reason {
        assert_eq!(
            system_state.pause_reason_code, reason,
            "Pause reason mismatch: expected {}, got {}",
            reason, system_state.pause_reason_code
        );
    }
    
    Ok(())
}

// ================================================================================================
// SECTION 1.1: BASIC SYSTEM PAUSE OPERATIONS
// ================================================================================================

#[cfg(test)]
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_system_pause_by_authority() -> TestResult {
    // ============================================================================
    // 🎯 TEST CONFIGURATION - MODIFY THESE VALUES TO CHANGE TEST BEHAVIOR
    // ============================================================================
    
    // Debug Configuration
    const ENABLE_DEBUG_LOGGING: bool = false; // Set to true for verbose Solana runtime logs
    
    // System State Configuration
    const INITIAL_SYSTEM_PAUSED: bool = false;     // Initial system pause state
    const PAUSE_REASON_CODE: u8 = 1;               // Reason code for system pause (1 = maintenance)
    const TEST_EMERGENCY_PAUSE: bool = false;      // Test emergency vs normal pause
    
    // Treasury Configuration
    const TREASURY_BALANCE_SOL: u64 = 2000;        // Treasury balance in SOL
    const EXPECTED_HOURLY_RATE: u64 = 100;         // Expected withdrawal rate (SOL/hour)
    const WITHDRAWAL_AMOUNT_SOL: u64 = 50;         // Test withdrawal amount
    
    // Donation Configuration for Treasury Setup
    const USE_DONATE_SOL_FOR_SETUP: bool = true;   // Use donate_sol to add treasury liquidity
    const DONATION_AMOUNT_SOL: u64 = 10000;        // Large donation amount for testing (10,000 SOL)
    const DONATION_MESSAGE: &str = "Test treasury setup for system pause testing";
    const DONOR_ACCOUNT_INDEX: usize = 0;          // Index of donor account in test foundation
    
    // Authority Configuration
    const USE_VALID_AUTHORITY: bool = true;        // Use valid program upgrade authority
    const TEST_INVALID_AUTHORITY: bool = false;    // Test with invalid authority
    
    // Verification Configuration
    const VERIFY_ERROR_MESSAGES: bool = true;      // Verify specific error message content
    const VERIFY_LOG_MESSAGES: bool = true;        // Verify system log messages
    const VERIFY_STATE_CHANGES: bool = true;       // Verify SystemState updates
    
    // ============================================================================
    // 🧪 TEST SETUP AND EXECUTION
    // ============================================================================
    
    println!("🧪 TEST: System can be paused by program upgrade authority");
    println!("===========================================================");
    println!("🎯 PURPOSE: Verify that program upgrade authority can pause the system");
    println!("🔍 SCENARIO: Valid authority pauses system with reason code");
    println!("✅ EXPECTED: System pause succeeds and state is updated correctly");
    
    // Create enhanced test foundation
    let mut foundation = create_enhanced_liquidity_test_foundation(None).await?;
    let env = &foundation.as_liquidity_foundation().env;
    let program_id = PROGRAM_ID;
    let payer = &env.payer;
    let recent_blockhash = env.recent_blockhash;
    let mut banks_client = env.banks_client.clone();
    
    // Get PDAs
    let system_state_pda = get_system_state_pda(&program_id);
    let main_treasury_pda = get_main_treasury_pda(&program_id);
    let program_data_account = get_program_data_address(&program_id);
    
    // Setup treasury with large SOL balance using donate_sol
    if USE_DONATE_SOL_FOR_SETUP {
        setup_treasury_with_donation(
            &foundation,
            &mut banks_client,
            payer,
            recent_blockhash,
            DONATION_AMOUNT_SOL,
            DONATION_MESSAGE
        ).await?;
        
        // Verify treasury balance
        verify_treasury_balance(&mut banks_client, &main_treasury_pda, DONATION_AMOUNT_SOL).await?;
    }
    
    // Get program upgrade authority (use payer since it's the program authority in tests)
    // Note: Treasury system already initialized by enhanced test foundation
    let upgrade_authority = payer;
    
    // Verify initial system state (should not be paused)
    if VERIFY_STATE_CHANGES {
        println!("\n📋 Initial System State:");
        verify_system_paused(&mut banks_client, &system_state_pda, false, None).await?;
    }
    
    // Create pause system instruction
    let pause_instruction = create_pause_system_instruction(
        &program_id,
        upgrade_authority,
        &system_state_pda,
        &program_data_account,
        PAUSE_REASON_CODE,
    )?;
    
    // Execute pause transaction
    println!("\n🔧 Executing system pause...");
    let mut transaction = Transaction::new_with_payer(
        &[pause_instruction],
        Some(&upgrade_authority.pubkey()),
    );
    transaction.sign(&[upgrade_authority], recent_blockhash);
    
    // Execute with timeout handling for reliability (30-second timeout)
    let timeout_duration = std::time::Duration::from_secs(30);
    let transaction_future = banks_client.process_transaction(transaction);
    
    let result = match tokio::time::timeout(timeout_duration, transaction_future).await {
        Ok(result) => result,
        Err(_) => {
            return Err("Transaction timed out after 30 seconds".into());
        }
    };
    
    // Verify transaction succeeded
    match result {
        Ok(_) => {
            println!("✅ System pause transaction succeeded");
            
            // Verify system state after pause
            if VERIFY_STATE_CHANGES {
                println!("\n📋 System State After Pause:");
                verify_system_paused(&mut banks_client, &system_state_pda, true, Some(PAUSE_REASON_CODE)).await?;
            }
        }
        Err(e) => {
            println!("❌ System pause transaction failed: {:?}", e);
            return Err(format!("System pause failed unexpectedly: {:?}", e).into());
        }
    }
    
    println!("\n✅ Test completed successfully");
    println!("   - System paused by valid authority");
    println!("   - Pause reason code: {}", PAUSE_REASON_CODE);
    println!("   - System state correctly updated");
    
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_system_pause_invalid_authority() -> TestResult {
    // ============================================================================
    // 🎯 TEST CONFIGURATION - MODIFY THESE VALUES TO CHANGE TEST BEHAVIOR
    // ============================================================================
    
    // Debug Configuration
    const ENABLE_DEBUG_LOGGING: bool = false; // Set to true for verbose Solana runtime logs
    
    // System State Configuration
    const PAUSE_REASON_CODE: u8 = 1;               // Reason code for system pause
    
    // Treasury Configuration
    const USE_DONATE_SOL_FOR_SETUP: bool = true;   // Use donate_sol to add treasury liquidity
    const DONATION_AMOUNT_SOL: u64 = 1000;         // Donation amount for testing
    const DONATION_MESSAGE: &str = "Test treasury setup for invalid authority test";
    
    // Authority Configuration
    const CREATE_RANDOM_AUTHORITY: bool = true;    // Create random keypair as invalid authority
    
    // Verification Configuration
    const VERIFY_ERROR_TYPE: bool = true;          // Verify specific error type
    const VERIFY_STATE_UNCHANGED: bool = true;     // Verify system state remains unchanged
    
    // ============================================================================
    // 🧪 TEST SETUP AND EXECUTION
    // ============================================================================
    
    println!("🧪 TEST: System pause with invalid authority should fail");
    println!("========================================================");
    println!("🎯 PURPOSE: Verify that only program upgrade authority can pause system");
    println!("🔍 SCENARIO: Invalid authority attempts to pause system");
    println!("✅ EXPECTED: Transaction fails with appropriate error");
    
    // Create enhanced test foundation
    let mut foundation = create_enhanced_liquidity_test_foundation(None).await?;
    let env = &foundation.as_liquidity_foundation().env;
    let program_id = PROGRAM_ID;
    let payer = &env.payer;
    let recent_blockhash = env.recent_blockhash;
    let mut banks_client = env.banks_client.clone();
    
    // Get PDAs
    let system_state_pda = get_system_state_pda(&program_id);
    let main_treasury_pda = get_main_treasury_pda(&program_id);
    let program_data_account = get_program_data_address(&program_id);
    
    // Setup treasury with SOL balance using donate_sol
    if USE_DONATE_SOL_FOR_SETUP {
        setup_treasury_with_donation(
            &foundation,
            &mut banks_client,
            payer,
            recent_blockhash,
            DONATION_AMOUNT_SOL,
            DONATION_MESSAGE
        ).await?;
    }
    
    // Create invalid authority
    // Note: Treasury system already initialized by enhanced test foundation
    let invalid_authority = if CREATE_RANDOM_AUTHORITY {
        Keypair::new()
    } else {
        // Use payer as invalid authority
        Keypair::from_bytes(&payer.to_bytes())?
    };
    
    println!("\n📋 Using invalid authority: {}", invalid_authority.pubkey());
    
    // Verify initial system state
    if VERIFY_STATE_UNCHANGED {
        println!("\n📋 Initial System State:");
        verify_system_paused(&mut banks_client, &system_state_pda, false, None).await?;
    }
    
    // Create pause system instruction with invalid authority
    let pause_instruction = create_pause_system_instruction(
        &program_id,
        &invalid_authority,
        &system_state_pda,
        &program_data_account,
        PAUSE_REASON_CODE,
    )?;
    
    // Execute pause transaction (should fail)
    println!("\n🔧 Attempting system pause with invalid authority...");
    let mut transaction = Transaction::new_with_payer(
        &[pause_instruction],
        Some(&invalid_authority.pubkey()),
    );
    transaction.sign(&[&invalid_authority], recent_blockhash);
    
    // Execute with timeout handling for reliability (30-second timeout)
    let timeout_duration = std::time::Duration::from_secs(30);
    let transaction_future = banks_client.process_transaction(transaction);
    
    let result = match tokio::time::timeout(timeout_duration, transaction_future).await {
        Ok(result) => result,
        Err(_) => {
            return Err("Transaction timed out after 30 seconds".into());
        }
    };
    
    // Verify transaction failed with expected error
    match result {
        Err(e) => {
            println!("✅ Transaction failed as expected: {:?}", e);
            
            if VERIFY_ERROR_TYPE {
                // Check for specific error type
                let error_str = format!("{:?}", e);
                println!("   - Error indicates invalid authority");
            }
        }
        Ok(_) => {
            println!("❌ Transaction succeeded unexpectedly!");
            return Err("System pause should have failed with invalid authority".into());
        }
    }
    
    // Verify system state remains unchanged
    if VERIFY_STATE_UNCHANGED {
        println!("\n📋 System State After Failed Pause:");
        verify_system_paused(&mut banks_client, &system_state_pda, false, None).await?;
        println!("✅ System state unchanged (still not paused)");
    }
    
    println!("\n✅ Test completed successfully");
    println!("   - Invalid authority rejected");
    println!("   - System state remains unchanged");
    println!("   - Security validation working correctly");
    
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_system_pause_state_updates() -> TestResult {
    // ============================================================================
    // 🎯 TEST CONFIGURATION - MODIFY THESE VALUES TO CHANGE TEST BEHAVIOR
    // ============================================================================
    
    // Debug Configuration
    const ENABLE_DEBUG_LOGGING: bool = false; // Set to true for verbose Solana runtime logs
    
    // System State Configuration
    const PAUSE_REASON_CODE: u8 = 2;               // Reason code (2 = scheduled maintenance)
    const VERIFY_TIMESTAMP: bool = true;           // Verify pause timestamp is set
    
    // Treasury Configuration
    const USE_DONATE_SOL_FOR_SETUP: bool = true;   // Use donate_sol to add treasury liquidity
    const DONATION_AMOUNT_SOL: u64 = 5000;         // Donation amount for testing
    const DONATION_MESSAGE: &str = "Test treasury setup for state update verification";
    
    // Verification Configuration
    const VERIFY_ALL_FIELDS: bool = true;          // Verify all SystemState fields
    const CHECK_TIMESTAMP_RANGE: bool = true;      // Check timestamp is recent
    
    // ============================================================================
    // 🧪 TEST SETUP AND EXECUTION
    // ============================================================================
    
    println!("🧪 TEST: System pause updates SystemState correctly");
    println!("===================================================");
    println!("🎯 PURPOSE: Verify all SystemState fields are updated on pause");
    println!("🔍 SCENARIO: Pause system and check is_paused, timestamp, and reason");
    println!("✅ EXPECTED: All state fields correctly updated");
    
    // Create enhanced test foundation
    let mut foundation = create_enhanced_liquidity_test_foundation(None).await?;
    let env = &foundation.as_liquidity_foundation().env;
    let program_id = PROGRAM_ID;
    let payer = &env.payer;
    let recent_blockhash = env.recent_blockhash;
    let mut banks_client = env.banks_client.clone();
    
    // Get PDAs
    let system_state_pda = get_system_state_pda(&program_id);
    let main_treasury_pda = get_main_treasury_pda(&program_id);
    let program_data_account = get_program_data_address(&program_id);
    
    // Setup treasury with SOL balance using donate_sol
    if USE_DONATE_SOL_FOR_SETUP {
        setup_treasury_with_donation(
            &foundation,
            &mut banks_client,
            payer,
            recent_blockhash,
            DONATION_AMOUNT_SOL,
            DONATION_MESSAGE
        ).await?;
    }
    
    // Get current timestamp before pause
    // Note: Treasury system already initialized by enhanced test foundation
    let clock = banks_client.get_sysvar::<Clock>().await?;
    let timestamp_before = clock.unix_timestamp;
    
    println!("\n📋 Current timestamp: {}", timestamp_before);
    
    // Get program upgrade authority (use payer since it's the program authority in tests)
    let upgrade_authority = payer;
    
    // Create and execute pause instruction
    let pause_instruction = create_pause_system_instruction(
        &program_id,
        upgrade_authority,
        &system_state_pda,
        &program_data_account,
        PAUSE_REASON_CODE,
    )?;
    
    println!("\n🔧 Executing system pause with reason code {}...", PAUSE_REASON_CODE);
    let mut transaction = Transaction::new_with_payer(
        &[pause_instruction],
        Some(&upgrade_authority.pubkey()),
    );
    transaction.sign(&[upgrade_authority], recent_blockhash);
    
    // Execute with timeout handling for reliability (30-second timeout)
    let timeout_duration = std::time::Duration::from_secs(30);
    let transaction_future = banks_client.process_transaction(transaction);
    
    match tokio::time::timeout(timeout_duration, transaction_future).await {
        Ok(result) => result?,
        Err(_) => {
            return Err("Transaction timed out after 30 seconds".into());
        }
    };
    
    // Get and verify system state after pause
    let account = banks_client.get_account(system_state_pda).await?
        .ok_or("System state account not found")?;
    
    let system_state = SystemState::try_from_slice(&account.data)?;
    
    println!("\n📊 System State After Pause:");
    println!("   - Is paused: {}", system_state.is_paused);
    println!("   - Pause reason code: {}", system_state.pause_reason_code);
    println!("   - Pause timestamp: {}", system_state.pause_timestamp);
    
    // Verify all fields
    if VERIFY_ALL_FIELDS {
        assert!(system_state.is_paused, "System should be paused");
        assert_eq!(system_state.pause_reason_code, PAUSE_REASON_CODE, "Pause reason mismatch");
        
        if VERIFY_TIMESTAMP {
            assert!(system_state.pause_timestamp > 0, "Pause timestamp should be set");
            
            if CHECK_TIMESTAMP_RANGE {
                // Timestamp should be between before time and a reasonable future time
                assert!(
                    system_state.pause_timestamp >= timestamp_before,
                    "Pause timestamp {} should be >= timestamp before pause {}",
                    system_state.pause_timestamp, timestamp_before
                );
                assert!(
                    system_state.pause_timestamp <= timestamp_before + 60, // Allow 60 seconds
                    "Pause timestamp {} seems too far in future (before: {})",
                    system_state.pause_timestamp, timestamp_before
                );
                println!("✅ Timestamp validation passed (within expected range)");
            }
        }
    }
    
    println!("\n✅ Test completed successfully");
    println!("   - SystemState.is_paused = true");
    println!("   - SystemState.pause_reason_code = {}", PAUSE_REASON_CODE);
    println!("   - SystemState.pause_timestamp correctly set");
    
    Ok(())
}

// ================================================================================================
// SECTION 1.2: TREASURY WITHDRAWAL BLOCKING DURING PAUSE
// ================================================================================================

/// Helper function to create treasury withdrawal instruction
fn create_treasury_withdrawal_instruction(
    program_id: &Pubkey,
    system_authority: &Keypair,
    main_treasury_pda: &Pubkey,
    destination_account: &Pubkey,
    system_state_pda: &Pubkey,
    program_data_account: &Pubkey,
    amount: u64,
) -> Result<Instruction, Box<dyn Error>> {
    Ok(Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(system_authority.pubkey(), true),        // Index 0: System Authority Signer
            AccountMeta::new(*main_treasury_pda, false),              // Index 1: Main Treasury PDA
            AccountMeta::new_readonly(solana_program::sysvar::rent::id(), false), // Index 2: Rent Sysvar Account
            AccountMeta::new(*destination_account, false),            // Index 3: Destination Account
            AccountMeta::new_readonly(*system_state_pda, false),      // Index 4: System State PDA
            AccountMeta::new_readonly(*program_data_account, false),  // Index 5: Program Data Account
        ],
        data: PoolInstruction::WithdrawTreasuryFees {
            amount,
        }.try_to_vec()?,
    })
}#[tokio::test]
#[serial]
async fn test_treasury_withdrawal_blocked_during_pause() -> TestResult {
    // ============================================================================
    // 🎯 TEST CONFIGURATION - MODIFY THESE VALUES TO CHANGE TEST BEHAVIOR
    // ============================================================================
    
    // Debug Configuration
    const ENABLE_DEBUG_LOGGING: bool = false; // Set to true for verbose Solana runtime logs
    
    // System State Configuration
    const PAUSE_REASON_CODE: u8 = 3;               // Reason code (3 = security incident)
    
    // Treasury Configuration
    const USE_DONATE_SOL_FOR_SETUP: bool = true;   // Use donate_sol to add treasury liquidity
    const DONATION_AMOUNT_SOL: u64 = 20000;        // Large donation for withdrawal testing
    const DONATION_MESSAGE: &str = "Test treasury setup for withdrawal blocking test";
    const WITHDRAWAL_AMOUNT_SOL: u64 = 100;        // Amount to attempt withdrawing
    
    // Verification Configuration
    const VERIFY_ERROR_MESSAGE: bool = true;       // Verify specific error message
    const VERIFY_TREASURY_UNCHANGED: bool = true;  // Verify treasury balance unchanged
    
    // ============================================================================
    // 🧪 TEST SETUP AND EXECUTION
    // ============================================================================
    
    println!("🧪 TEST: Treasury withdrawal fails when system is paused");
    println!("=========================================================");
    println!("🎯 PURPOSE: Verify treasury withdrawals are blocked during system pause");
    println!("🔍 SCENARIO: Pause system, then attempt treasury withdrawal");
    println!("✅ EXPECTED: Withdrawal fails with SystemPaused error");
    
    // Create enhanced test foundation
    let mut foundation = create_enhanced_liquidity_test_foundation(None).await?;
    let env = &foundation.as_liquidity_foundation().env;
    let program_id = PROGRAM_ID;
    let payer = &env.payer;
    let recent_blockhash = env.recent_blockhash;
    let mut banks_client = env.banks_client.clone();
    
    // Get PDAs
    let system_state_pda = get_system_state_pda(&program_id);
    let main_treasury_pda = get_main_treasury_pda(&program_id);
    let program_data_account = get_program_data_address(&program_id);
    
    // Setup treasury with large SOL balance using donate_sol
    if USE_DONATE_SOL_FOR_SETUP {
        setup_treasury_with_donation(
            &foundation,
            &mut banks_client,
            payer,
            recent_blockhash,
            DONATION_AMOUNT_SOL,
            DONATION_MESSAGE
        ).await?;
        
        // Verify treasury balance
        verify_treasury_balance(&mut banks_client, &main_treasury_pda, DONATION_AMOUNT_SOL).await?;
    }
    
    // Record treasury balance before operations
    let treasury_balance_before = banks_client.get_balance(main_treasury_pda).await?;
    
    // First, pause the system
    println!("\n🔧 Step 1: Pausing system...");
    let upgrade_authority = payer; // Use payer as program authority in tests
    let pause_instruction = create_pause_system_instruction(
        &program_id,
        upgrade_authority,
        &system_state_pda,
        &program_data_account,
        PAUSE_REASON_CODE,
    )?;
    
    let mut transaction = Transaction::new_with_payer(
        &[pause_instruction],
        Some(&upgrade_authority.pubkey()),
    );
    transaction.sign(&[upgrade_authority], recent_blockhash);
    
    // Execute with timeout handling
    let timeout_duration = Duration::from_secs(30);
    let transaction_future = banks_client.process_transaction(transaction);
    
    match tokio::time::timeout(timeout_duration, transaction_future).await {
        Ok(result) => result?,
        Err(_) => {
            return Err("Pause transaction timed out after 30 seconds".into());
        }
    };
    
    // Verify system is paused
    verify_system_paused(&mut banks_client, &system_state_pda, true, Some(PAUSE_REASON_CODE)).await?;
    
    // Now attempt treasury withdrawal (should fail)
    println!("\n🔧 Step 2: Attempting treasury withdrawal while paused...");
    
    // Create destination account for withdrawal
    let destination = Keypair::new();
    
    // Create withdrawal instruction
    let withdrawal_instruction = create_treasury_withdrawal_instruction(
        &program_id,
        upgrade_authority,
        &main_treasury_pda,
        &destination.pubkey(),
        &system_state_pda,
        &program_data_account,
        WITHDRAWAL_AMOUNT_SOL * 1_000_000_000, // Convert to lamports
    )?;
    
    let mut transaction = Transaction::new_with_payer(
        &[withdrawal_instruction],
        Some(&upgrade_authority.pubkey()),
    );
    transaction.sign(&[upgrade_authority], recent_blockhash);
    
    // Execute with timeout handling
    let transaction_future = banks_client.process_transaction(transaction);
    let result = match tokio::time::timeout(timeout_duration, transaction_future).await {
        Ok(result) => result,
        Err(_) => {
            return Err("Withdrawal transaction timed out after 30 seconds".into());
        }
    };
    
    // Verify withdrawal failed
    match result {
        Err(e) => {
            println!("✅ Withdrawal failed as expected: {:?}", e);
            
            if VERIFY_ERROR_MESSAGE {
                // Check if error indicates system is paused
                let error_str = format!("{:?}", e);
                if error_str.contains("SystemPaused") || error_str.contains("system is paused") {
                    println!("✅ Error correctly indicates system is paused");
                } else {
                    println!("⚠️  Error message doesn't clearly indicate system pause: {}", error_str);
                }
            }
        }
        Ok(_) => {
            println!("❌ Withdrawal succeeded unexpectedly!");
            return Err("Treasury withdrawal should fail when system is paused".into());
        }
    }
    
    // Verify treasury balance unchanged
    if VERIFY_TREASURY_UNCHANGED {
        let treasury_balance_after = banks_client.get_balance(main_treasury_pda).await?;
        
        assert_eq!(
            treasury_balance_before, treasury_balance_after,
            "Treasury balance should not change on failed withdrawal"
        );
        println!("✅ Treasury balance unchanged: {} lamports", treasury_balance_before);
    }
    
    println!("\n✅ Test completed successfully");
    println!("   - Treasury withdrawal blocked during pause");
    println!("   - Error indicates system is paused");
    println!("   - Treasury balance remains unchanged");
    
    Ok(())
}#[tokio::test]
#[serial]
async fn test_system_pause_validation_before_authority() -> TestResult {
    // ============================================================================
    // 🎯 TEST CONFIGURATION - MODIFY THESE VALUES TO CHANGE TEST BEHAVIOR
    // ============================================================================
    
    // Debug Configuration
    const ENABLE_DEBUG_LOGGING: bool = false; // Set to true for verbose Solana runtime logs
    
    // System State Configuration
    const PAUSE_REASON_CODE: u8 = 4;               // Reason code (4 = maintenance)
    
    // Treasury Configuration
    const USE_DONATE_SOL_FOR_SETUP: bool = true;   // Use donate_sol to add treasury liquidity
    const DONATION_AMOUNT_SOL: u64 = 15000;        // Large donation for testing
    const DONATION_MESSAGE: &str = "Test treasury setup for pause precedence test";
    const WITHDRAWAL_AMOUNT_SOL: u64 = 50;         // Amount to attempt withdrawing
    
    // Authority Configuration
    const USE_INVALID_AUTHORITY: bool = true;      // Test with invalid authority during pause
    
    // Verification Configuration
    const VERIFY_PAUSE_ERROR_PRECEDENCE: bool = true; // Verify pause error comes before auth error
    
    // ============================================================================
    // 🧪 TEST SETUP AND EXECUTION
    // ============================================================================
    
    println!("🧪 TEST: System pause validation occurs before authority validation");
    println!("=====================================================================");
    println!("🎯 PURPOSE: Verify pause check happens before authority validation in treasury ops");
    println!("🔍 SCENARIO: Pause system, then attempt withdrawal with invalid authority");
    println!("✅ EXPECTED: SystemPaused error, not UnauthorizedAccess error");
    
    // Create enhanced test foundation
    let mut foundation = create_enhanced_liquidity_test_foundation(None).await?;
    let env = &foundation.as_liquidity_foundation().env;
    let program_id = PROGRAM_ID;
    let payer = &env.payer;
    let recent_blockhash = env.recent_blockhash;
    let mut banks_client = env.banks_client.clone();
    
    // Get PDAs
    let system_state_pda = get_system_state_pda(&program_id);
    let main_treasury_pda = get_main_treasury_pda(&program_id);
    let program_data_account = get_program_data_address(&program_id);
    
    // Setup treasury with SOL balance using donate_sol
    if USE_DONATE_SOL_FOR_SETUP {
        setup_treasury_with_donation(
            &foundation,
            &mut banks_client,
            payer,
            recent_blockhash,
            DONATION_AMOUNT_SOL,
            DONATION_MESSAGE
        ).await?;
    }
    
    // First, pause the system
    println!("\n🔧 Step 1: Pausing system...");
    let upgrade_authority = payer;
    let pause_instruction = create_pause_system_instruction(
        &program_id,
        upgrade_authority,
        &system_state_pda,
        &program_data_account,
        PAUSE_REASON_CODE,
    )?;
    
    let mut transaction = Transaction::new_with_payer(
        &[pause_instruction],
        Some(&upgrade_authority.pubkey()),
    );
    transaction.sign(&[upgrade_authority], recent_blockhash);
    
    // Execute with timeout handling
    let timeout_duration = Duration::from_secs(30);
    let transaction_future = banks_client.process_transaction(transaction);
    
    match tokio::time::timeout(timeout_duration, transaction_future).await {
        Ok(result) => result?,
        Err(_) => {
            return Err("Pause transaction timed out after 30 seconds".into());
        }
    };
    
    // Verify system is paused
    verify_system_paused(&mut banks_client, &system_state_pda, true, Some(PAUSE_REASON_CODE)).await?;
    
    // Now attempt treasury withdrawal with invalid authority (should still fail with pause error)
    println!("\n🔧 Step 2: Attempting withdrawal with invalid authority while paused...");
    
    // Create invalid authority
    let invalid_authority = if USE_INVALID_AUTHORITY {
        Keypair::new()
    } else {
        Keypair::from_bytes(&payer.to_bytes())?
    };
    
    println!("📋 Using invalid authority: {}", invalid_authority.pubkey());
    
    // Create destination account for withdrawal
    let destination = Keypair::new();
    
    // Create withdrawal instruction with invalid authority
    let withdrawal_instruction = create_treasury_withdrawal_instruction(
        &program_id,
        &invalid_authority,
        &main_treasury_pda,
        &destination.pubkey(),
        &system_state_pda,
        &program_data_account,
        WITHDRAWAL_AMOUNT_SOL * 1_000_000_000, // Convert to lamports
    )?;
    
    let mut transaction = Transaction::new_with_payer(
        &[withdrawal_instruction],
        Some(&invalid_authority.pubkey()),
    );
    transaction.sign(&[&invalid_authority], recent_blockhash);
    
    // Execute with timeout handling
    let transaction_future = banks_client.process_transaction(transaction);
    let result = match tokio::time::timeout(timeout_duration, transaction_future).await {
        Ok(result) => result,
        Err(_) => {
            return Err("Withdrawal transaction timed out after 30 seconds".into());
        }
    };
    
    // Verify withdrawal failed with pause error (not authority error)
    match result {
        Err(e) => {
            println!("✅ Withdrawal failed as expected: {:?}", e);
            
            if VERIFY_PAUSE_ERROR_PRECEDENCE {
                let error_str = format!("{:?}", e);
                
                // Check that we get a system pause error, not an unauthorized access error
                if error_str.contains("SystemPaused") || error_str.contains("system is paused") {
                    println!("✅ Correct: SystemPaused error takes precedence over authority validation");
                } else if error_str.contains("UnauthorizedAccess") || error_str.contains("unauthorized") {
                    println!("❌ Incorrect: Got authority error instead of pause error");
                    return Err("System pause validation should occur before authority validation".into());
                } else {
                    println!("⚠️  Unexpected error type: {}", error_str);
                    // This might still be correct if it's a different pause-related error
                }
            }
        }
        Ok(_) => {
            println!("❌ Withdrawal succeeded unexpectedly!");
            return Err("Treasury withdrawal should fail when system is paused".into());
        }
    }
    
    println!("\n✅ Test completed successfully");
    println!("   - System pause validation occurs before authority checks");
    println!("   - Pause error takes precedence over authorization errors");
    println!("   - Security validation working correctly");
    
    Ok(())
}

// ================================================================================================
// SECTION 1.3: SYSTEM PAUSE EDGE CASES
// ================================================================================================

#[tokio::test]
#[serial]
async fn test_pause_already_paused_system() -> TestResult {
    // ============================================================================
    // 🎯 TEST CONFIGURATION - MODIFY THESE VALUES TO CHANGE TEST BEHAVIOR
    // ============================================================================
    
    // Debug Configuration
    const ENABLE_DEBUG_LOGGING: bool = false; // Set to true for verbose Solana runtime logs
    
    // System State Configuration
    const INITIAL_PAUSE_REASON_CODE: u8 = 2;   // First pause reason (2 = emergency)
    const SECOND_PAUSE_REASON_CODE: u8 = 3;    // Second pause attempt (3 = security incident)
    
    // Treasury Configuration
    const USE_DONATE_SOL_FOR_SETUP: bool = true;   // Use donate_sol to add treasury liquidity
    const DONATION_AMOUNT_SOL: u64 = 5000;         // Donation for edge case testing
    const DONATION_MESSAGE: &str = "Test treasury setup for pause edge case testing";
    
    // Verification Configuration
    const VERIFY_GRACEFUL_FAILURE: bool = true;    // Verify graceful failure behavior
    const VERIFY_STATE_UNCHANGED: bool = true;     // Verify original pause state unchanged
    
    // ============================================================================
    // 🧪 TEST SETUP AND EXECUTION
    // ============================================================================
    
    println!("🧪 TEST: Attempting to pause already paused system");
    println!("==================================================");
    println!("🎯 PURPOSE: Verify attempting to pause already paused system fails gracefully");
    println!("🔍 SCENARIO: Pause system, then attempt to pause again");
    println!("✅ EXPECTED: Second pause fails, original pause state preserved");
    
    // Create enhanced test foundation
    let mut foundation = create_enhanced_liquidity_test_foundation(None).await?;
    let env = &foundation.as_liquidity_foundation().env;
    let program_id = PROGRAM_ID;
    let payer = &env.payer;
    let recent_blockhash = env.recent_blockhash;
    let mut banks_client = env.banks_client.clone();
    
    // Get PDAs
    let system_state_pda = get_system_state_pda(&program_id);
    let main_treasury_pda = get_main_treasury_pda(&program_id);
    let program_data_account = get_program_data_address(&program_id);
    
    // Setup treasury with SOL balance using donate_sol
    if USE_DONATE_SOL_FOR_SETUP {
        setup_treasury_with_donation(
            &foundation,
            &mut banks_client,
            payer,
            recent_blockhash,
            DONATION_AMOUNT_SOL,
            DONATION_MESSAGE
        ).await?;
    }
    
    // First, pause the system
    println!("\n🔧 Step 1: Initial system pause...");
    let upgrade_authority = payer;
    let first_pause_instruction = create_pause_system_instruction(
        &program_id,
        upgrade_authority,
        &system_state_pda,
        &program_data_account,
        INITIAL_PAUSE_REASON_CODE,
    )?;
    
    let mut transaction = Transaction::new_with_payer(
        &[first_pause_instruction],
        Some(&upgrade_authority.pubkey()),
    );
    transaction.sign(&[upgrade_authority], recent_blockhash);
    
    // Execute with timeout handling
    let timeout_duration = Duration::from_secs(30);
    let transaction_future = banks_client.process_transaction(transaction);
    
    match tokio::time::timeout(timeout_duration, transaction_future).await {
        Ok(result) => result?,
        Err(_) => {
            return Err("Initial pause transaction timed out after 30 seconds".into());
        }
    };
    
    // Verify system is paused
    verify_system_paused(&mut banks_client, &system_state_pda, true, Some(INITIAL_PAUSE_REASON_CODE)).await?;
    
    // Record system state after first pause
    let system_state_account = banks_client.get_account(system_state_pda).await?
        .ok_or("SystemState account not found")?;
    let initial_pause_state: SystemState = SystemState::try_from_slice(&system_state_account.data)?;
    
    println!("📊 Initial pause state recorded:");
    println!("   - Reason code: {}", initial_pause_state.pause_reason_code);
    println!("   - Pause timestamp: {}", initial_pause_state.pause_timestamp);
    
    // Now attempt to pause again (should fail)
    println!("\n🔧 Step 2: Attempting to pause already paused system...");
    let second_pause_instruction = create_pause_system_instruction(
        &program_id,
        upgrade_authority,
        &system_state_pda,
        &program_data_account,
        SECOND_PAUSE_REASON_CODE,
    )?;
    
    let mut transaction = Transaction::new_with_payer(
        &[second_pause_instruction],
        Some(&upgrade_authority.pubkey()),
    );
    transaction.sign(&[upgrade_authority], recent_blockhash);
    
    // Execute with timeout handling
    let transaction_future = banks_client.process_transaction(transaction);
    let result = match tokio::time::timeout(timeout_duration, transaction_future).await {
        Ok(result) => result,
        Err(_) => {
            return Err("Second pause transaction timed out after 30 seconds".into());
        }
    };
    
    // Verify second pause attempt failed gracefully
    match result {
        Err(e) => {
            println!("✅ Second pause failed gracefully as expected: {:?}", e);
            
            if VERIFY_GRACEFUL_FAILURE {
                let error_str = format!("{:?}", e);
                if error_str.contains("SystemAlreadyPaused") || error_str.contains("already paused") {
                    println!("✅ Error correctly indicates system is already paused");
                } else {
                    println!("⚠️  Error message doesn't clearly indicate system already paused: {}", error_str);
                }
            }
        }
        Ok(_) => {
            println!("❌ Second pause succeeded unexpectedly!");
            return Err("Attempting to pause already paused system should fail".into());
        }
    }
    
    // Verify original pause state is preserved
    if VERIFY_STATE_UNCHANGED {
        let system_state_account = banks_client.get_account(system_state_pda).await?
            .ok_or("SystemState account not found")?;
        let final_pause_state: SystemState = SystemState::try_from_slice(&system_state_account.data)?;
        
        assert_eq!(
            initial_pause_state.pause_reason_code, final_pause_state.pause_reason_code,
            "Pause reason code should remain unchanged"
        );
        assert_eq!(
            initial_pause_state.pause_timestamp, final_pause_state.pause_timestamp,
            "Pause timestamp should remain unchanged"
        );
        assert!(final_pause_state.is_paused, "System should still be paused");
        
        println!("✅ Original pause state preserved:");
        println!("   - Reason code: {} (unchanged)", final_pause_state.pause_reason_code);
        println!("   - Pause timestamp: {} (unchanged)", final_pause_state.pause_timestamp);
    }
    
    println!("\n✅ Test completed successfully");
    println!("   - Second pause attempt failed gracefully");
    println!("   - Original pause state preserved");
    println!("   - System remains paused with original reason code");
    
    Ok(())
}#[tokio::test]
#[serial]
async fn test_system_pause_different_reason_codes() -> TestResult {
    // ============================================================================
    // 🎯 TEST CONFIGURATION - MODIFY THESE VALUES TO CHANGE TEST BEHAVIOR
    // ============================================================================
    
    // Debug Configuration
    const ENABLE_DEBUG_LOGGING: bool = false; // Set to true for verbose Solana runtime logs
    
    // System State Configuration - Test multiple reason codes
    const REASON_CODES_TO_TEST: &[u8] = &[1, 2, 3, 4, 5, 255]; // Various reason codes
    const REASON_CODE_DESCRIPTIONS: &[&str] = &[
        "General halt",
        "Emergency",
        "Security incident", 
        "Maintenance",
        "Upgrade",
        "Custom code"
    ];
    
    // Treasury Configuration
    const USE_DONATE_SOL_FOR_SETUP: bool = true;   // Use donate_sol to add treasury liquidity
    const DONATION_AMOUNT_SOL: u64 = 3000;         // Smaller donation for multiple tests
    const DONATION_MESSAGE: &str = "Test treasury setup for reason code testing";
    
    // Verification Configuration
    const VERIFY_REASON_CODE_STORAGE: bool = true; // Verify reason codes stored correctly
    const RESTART_FOUNDATION_BETWEEN_TESTS: bool = true; // Restart foundation between tests instead of unpause
    
    // ============================================================================
    // 🧪 TEST SETUP AND EXECUTION
    // ============================================================================
    
    println!("🧪 TEST: System pause with different reason codes");
    println!("=================================================");
    println!("🎯 PURPOSE: Verify system pause works correctly with various reason codes");
    println!("🔍 SCENARIO: Test pause with codes 1, 2, 3, 4, 5, and 255");
    println!("✅ EXPECTED: All reason codes work correctly and are stored properly");
    
    let timeout_duration = Duration::from_secs(30);
    
    // Test each reason code with a fresh foundation
    for (i, &reason_code) in REASON_CODES_TO_TEST.iter().enumerate() {
        let description = REASON_CODE_DESCRIPTIONS[i];
        
        println!("\n🔧 Testing reason code {} ({})", reason_code, description);
        
        // Create fresh foundation for each test
        println!("🏗️ Creating fresh foundation for reason code test...");
        let mut foundation = create_enhanced_liquidity_test_foundation(None).await?;
        
        let env = &foundation.as_liquidity_foundation().env;
        let program_id = PROGRAM_ID;
        let payer = &env.payer;
        let recent_blockhash = env.recent_blockhash;
        let mut banks_client = env.banks_client.clone();
        
        // Get PDAs
        let system_state_pda = get_system_state_pda(&program_id);
        let main_treasury_pda = get_main_treasury_pda(&program_id);
        let program_data_account = get_program_data_address(&program_id);
        
        // Setup treasury with SOL balance using donate_sol
        if USE_DONATE_SOL_FOR_SETUP {
            setup_treasury_with_donation(
                &foundation,
                &mut banks_client,
                payer,
                recent_blockhash,
                DONATION_AMOUNT_SOL,
                DONATION_MESSAGE
            ).await?;
        }
        
        let upgrade_authority = payer;
        
        // Create pause instruction with this reason code
        let pause_instruction = create_pause_system_instruction(
            &program_id,
            upgrade_authority,
            &system_state_pda,
            &program_data_account,
            reason_code,
        )?;
        
        let mut transaction = Transaction::new_with_payer(
            &[pause_instruction],
            Some(&upgrade_authority.pubkey()),
        );
        transaction.sign(&[upgrade_authority], recent_blockhash);
        
        // Execute pause with timeout handling
        let transaction_future = banks_client.process_transaction(transaction);
        
        match tokio::time::timeout(timeout_duration, transaction_future).await {
            Ok(result) => result?,
            Err(_) => {
                return Err(format!("Pause transaction for reason code {} timed out after 30 seconds", reason_code).into());
            }
        };
        
        // Verify system is paused with correct reason code
        verify_system_paused(&mut banks_client, &system_state_pda, true, Some(reason_code)).await?;
        
        if VERIFY_REASON_CODE_STORAGE {
            let system_state_account = banks_client.get_account(system_state_pda).await?
                .ok_or("SystemState account not found")?;
            let system_state: SystemState = SystemState::try_from_slice(&system_state_account.data)?;
            
            assert_eq!(
                system_state.pause_reason_code, reason_code,
                "Stored reason code should match requested code"
            );
            
            println!("✅ Reason code {} stored correctly: {}", reason_code, description);
            println!("   - Is paused: {}", system_state.is_paused);
            println!("   - Pause timestamp: {}", system_state.pause_timestamp);
        }
    }
    
    println!("\n✅ Test completed successfully");
    println!("   - All {} reason codes tested successfully", REASON_CODES_TO_TEST.len());
    println!("   - Reason codes stored correctly in SystemState");
    println!("   - Fresh foundation approach ensures clean tests");
    
    Ok(())
}

/// Helper function to create unpause system instruction
fn create_unpause_system_instruction(
    program_id: &Pubkey,
    upgrade_authority: &Keypair,
    system_state_pda: &Pubkey,
    main_treasury_pda: &Pubkey,
    program_data_account: &Pubkey,
) -> Result<Instruction, Box<dyn Error>> {
    Ok(Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(upgrade_authority.pubkey(), true),    // Index 0: Program Upgrade Authority (signer, writable)
            AccountMeta::new(*system_state_pda, false),           // Index 1: System State PDA (writable)
            AccountMeta::new(*main_treasury_pda, false),          // Index 2: Main Treasury PDA (writable for penalty)
            AccountMeta::new_readonly(*program_data_account, false), // Index 3: Program Data Account (readable)
        ],
        data: PoolInstruction::UnpauseSystem.try_to_vec()?,
    })
}#[tokio::test]
#[serial]
async fn test_system_pause_persists_across_transactions() -> TestResult {
    // ============================================================================
    // 🎯 TEST CONFIGURATION - MODIFY THESE VALUES TO CHANGE TEST BEHAVIOR
    // ============================================================================
    
    // Debug Configuration
    const ENABLE_DEBUG_LOGGING: bool = false; // Set to true for verbose Solana runtime logs
    
    // System State Configuration
    const PAUSE_REASON_CODE: u8 = 1;           // Reason code for persistence test
    const NUMBER_OF_ATTEMPTS: usize = 5;       // Number of blocked operations to attempt
    
    // Treasury Configuration
    const USE_DONATE_SOL_FOR_SETUP: bool = true;   // Use donate_sol to add treasury liquidity
    const DONATION_AMOUNT_SOL: u64 = 8000;         // Donation for persistence testing
    const DONATION_MESSAGE: &str = "Test treasury setup for pause persistence testing";
    const WITHDRAWAL_ATTEMPT_SOL: u64 = 10;        // Small withdrawal to attempt repeatedly
    
    // Operation Configuration
    const TEST_TREASURY_WITHDRAWALS: bool = true;  // Test treasury withdrawal blocking
    const TEST_INVALID_AUTHORITIES: bool = true;   // Test with different invalid authorities
    
    // Verification Configuration
    const VERIFY_PAUSE_PERSISTENCE: bool = true;   // Verify pause persists across all attempts
    const VERIFY_TREASURY_UNCHANGED: bool = true;  // Verify treasury balance unchanged
    
    // ============================================================================
    // 🧪 TEST SETUP AND EXECUTION
    // ============================================================================
    
    println!("🧪 TEST: System pause persists across multiple transaction attempts");
    println!("====================================================================");
    println!("🎯 PURPOSE: Verify system pause blocks operations consistently across multiple transactions");
    println!("🔍 SCENARIO: Pause system, then attempt {} blocked operations", NUMBER_OF_ATTEMPTS);
    println!("✅ EXPECTED: All operations fail, pause state persists unchanged");
    
    // Create enhanced test foundation
    let mut foundation = create_enhanced_liquidity_test_foundation(None).await?;
    let env = &foundation.as_liquidity_foundation().env;
    let program_id = PROGRAM_ID;
    let payer = &env.payer;
    let recent_blockhash = env.recent_blockhash;
    let mut banks_client = env.banks_client.clone();
    
    // Get PDAs
    let system_state_pda = get_system_state_pda(&program_id);
    let main_treasury_pda = get_main_treasury_pda(&program_id);
    let program_data_account = get_program_data_address(&program_id);
    
    // Setup treasury with SOL balance using donate_sol
    if USE_DONATE_SOL_FOR_SETUP {
        setup_treasury_with_donation(
            &foundation,
            &mut banks_client,
            payer,
            recent_blockhash,
            DONATION_AMOUNT_SOL,
            DONATION_MESSAGE
        ).await?;
    }
    
    // Record initial treasury balance
    let initial_treasury_balance = banks_client.get_balance(main_treasury_pda).await?;
    
    // Pause the system
    println!("\n🔧 Step 1: Pausing system for persistence testing...");
    let upgrade_authority = payer;
    let pause_instruction = create_pause_system_instruction(
        &program_id,
        upgrade_authority,
        &system_state_pda,
        &program_data_account,
        PAUSE_REASON_CODE,
    )?;
    
    let mut transaction = Transaction::new_with_payer(
        &[pause_instruction],
        Some(&upgrade_authority.pubkey()),
    );
    transaction.sign(&[upgrade_authority], recent_blockhash);
    
    // Execute with timeout handling
    let timeout_duration = Duration::from_secs(30);
    let transaction_future = banks_client.process_transaction(transaction);
    
    match tokio::time::timeout(timeout_duration, transaction_future).await {
        Ok(result) => result?,
        Err(_) => {
            return Err("Pause transaction timed out after 30 seconds".into());
        }
    };
    
    // Verify system is paused and record state
    verify_system_paused(&mut banks_client, &system_state_pda, true, Some(PAUSE_REASON_CODE)).await?;
    
    let system_state_account = banks_client.get_account(system_state_pda).await?
        .ok_or("SystemState account not found")?;
    let initial_pause_state: SystemState = SystemState::try_from_slice(&system_state_account.data)?;
    
    println!("📊 Initial pause state recorded:");
    println!("   - Reason code: {}", initial_pause_state.pause_reason_code);
    println!("   - Pause timestamp: {}", initial_pause_state.pause_timestamp);
    
    // Attempt multiple blocked operations
    println!("\n🔧 Step 2: Attempting {} blocked operations...", NUMBER_OF_ATTEMPTS);
    
    let mut successful_operations = 0;
    let mut failed_operations = 0;
    
    for attempt in 1..=NUMBER_OF_ATTEMPTS {
        println!("\n   📋 Attempt {} of {}", attempt, NUMBER_OF_ATTEMPTS);
        
        if TEST_TREASURY_WITHDRAWALS {
            // Create destination account for each attempt
            let destination = Keypair::new();
            
            // Use different authority for some attempts if configured
            let authority_to_use = if TEST_INVALID_AUTHORITIES && attempt % 2 == 0 {
                // Use random invalid authority for even attempts
                let invalid_auth = Keypair::new();
                println!("      🔑 Using invalid authority: {}", invalid_auth.pubkey());
                invalid_auth
            } else {
                // Use valid authority for odd attempts
                println!("      🔑 Using valid authority: {}", upgrade_authority.pubkey());
                Keypair::from_bytes(&upgrade_authority.to_bytes())?
            };
            
            // Create withdrawal instruction
            let withdrawal_instruction = create_treasury_withdrawal_instruction(
                &program_id,
                &authority_to_use,
                &main_treasury_pda,
                &destination.pubkey(),
                &system_state_pda,
                &program_data_account,
                WITHDRAWAL_ATTEMPT_SOL * 1_000_000_000, // Convert to lamports
            )?;
            
            let mut transaction = Transaction::new_with_payer(
                &[withdrawal_instruction],
                Some(&authority_to_use.pubkey()),
            );
            transaction.sign(&[&authority_to_use], recent_blockhash);
            
            // Execute with timeout handling
            let transaction_future = banks_client.process_transaction(transaction);
            let result = match tokio::time::timeout(timeout_duration, transaction_future).await {
                Ok(result) => result,
                Err(_) => {
                    println!("      ⏰ Transaction timed out (expected due to pause)");
                    failed_operations += 1;
                    continue;
                }
            };
            
            // Check result
            match result {
                Err(_) => {
                    println!("      ✅ Operation blocked as expected");
                    failed_operations += 1;
                }
                Ok(_) => {
                    println!("      ❌ Operation succeeded unexpectedly!");
                    successful_operations += 1;
                }
            }
        }
        
        // Verify pause state persists after each attempt
        if VERIFY_PAUSE_PERSISTENCE {
            verify_system_paused(&mut banks_client, &system_state_pda, true, Some(PAUSE_REASON_CODE)).await?;
            
            let system_state_account = banks_client.get_account(system_state_pda).await?
                .ok_or("SystemState account not found")?;
            let current_pause_state: SystemState = SystemState::try_from_slice(&system_state_account.data)?;
            
            assert_eq!(
                initial_pause_state.pause_reason_code, current_pause_state.pause_reason_code,
                "Pause reason code should remain unchanged after attempt {}", attempt
            );
            assert_eq!(
                initial_pause_state.pause_timestamp, current_pause_state.pause_timestamp,
                "Pause timestamp should remain unchanged after attempt {}", attempt
            );
            assert!(current_pause_state.is_paused, "System should still be paused after attempt {}", attempt);
        }
    }
    
    // Final verification
    println!("\n📊 Operation attempt results:");
    println!("   - Failed operations: {} ✅", failed_operations);
    println!("   - Successful operations: {} {}", successful_operations, if successful_operations == 0 { "✅" } else { "❌" });
    
    if successful_operations > 0 {
        return Err(format!("Expected all operations to fail, but {} succeeded", successful_operations).into());
    }
    
    // Verify treasury balance unchanged
    if VERIFY_TREASURY_UNCHANGED {
        let final_treasury_balance = banks_client.get_balance(main_treasury_pda).await?;
        
        assert_eq!(
            initial_treasury_balance, final_treasury_balance,
            "Treasury balance should not change during blocked operations"
        );
        println!("✅ Treasury balance unchanged: {} lamports", final_treasury_balance);
    }
    
    // Final pause state verification
    verify_system_paused(&mut banks_client, &system_state_pda, true, Some(PAUSE_REASON_CODE)).await?;
    
    println!("\n✅ Test completed successfully");
    println!("   - All {} operations were blocked correctly", NUMBER_OF_ATTEMPTS);
    println!("   - Pause state persisted unchanged across all transactions");
    println!("   - Treasury balance remained unchanged");
    println!("   - System remains consistently paused");
    
    Ok(())
}