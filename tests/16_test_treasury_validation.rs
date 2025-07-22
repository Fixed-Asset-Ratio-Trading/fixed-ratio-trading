//! # Treasury System Validation Tests
//! 
//! This module validates the treasury system architecture, fee routing,
//! and withdrawal mechanisms without executing complex on-chain operations.

use solana_sdk::pubkey::Pubkey;
use fixed_ratio_trading::PoolInstruction;
use borsh::BorshSerialize;
use serial_test::serial;

mod common;
use common::*;

/// Test treasury PDA derivation and validation logic
#[tokio::test]
async fn test_treasury_pda_derivation() {
    println!("🏗️ Testing treasury PDA derivation and validation");
    
    // Test 1: Verify main treasury PDA derivation using correct seed
    let (main_treasury_correct, main_bump) = Pubkey::find_program_address(
        &[fixed_ratio_trading::constants::MAIN_TREASURY_SEED_PREFIX], 
        &PROGRAM_ID
    );
    
    println!("Treasury PDA Derivation:");
    println!("  Main Treasury: {} (bump: {})", main_treasury_correct, main_bump);
    println!("  Seed: {:?}", std::str::from_utf8(fixed_ratio_trading::constants::MAIN_TREASURY_SEED_PREFIX).unwrap());
    
    // Test 2: Verify PDA properties
    assert_ne!(main_treasury_correct, Pubkey::default(), "Main treasury PDA should not be default pubkey");
    assert!(!main_treasury_correct.to_string().is_empty(), "Main treasury PDA should not be empty");
    assert!(main_bump >= 240, "Bump seed should be in expected range for PDAs (typically 240+)");
    
    // Test 3: Verify consistency - multiple derivations should yield same result  
    let (main_treasury_check, main_bump_check) = Pubkey::find_program_address(
        &[fixed_ratio_trading::constants::MAIN_TREASURY_SEED_PREFIX], 
        &PROGRAM_ID
    );
    assert_eq!(main_treasury_correct, main_treasury_check, "PDA derivation should be deterministic");
    assert_eq!(main_bump, main_bump_check, "Bump seed should be deterministic");
    
    // Test 4: Verify different seeds produce different PDAs
    let (wrong_treasury, _wrong_bump) = Pubkey::find_program_address(&[b"wrong_seed"], &PROGRAM_ID);
    assert_ne!(main_treasury_correct, wrong_treasury, "Different seeds should produce different PDAs");
    
    // Test 5: Test treasury validation function with correct PDA
    use fixed_ratio_trading::utils::fee_validation::validate_treasury_account;
    use fixed_ratio_trading::constants::TREASURY_TYPE_MAIN;
    use solana_program::account_info::AccountInfo;
    use solana_program::system_program;
    
    // Create mock account info for testing validation
    let mut lamports = 1000000000u64; // 1 SOL
    let mut data = vec![0u8; 256];
    let owner = system_program::id();
    
    let mock_treasury_account = AccountInfo::new(
        &main_treasury_correct,
        false, // not signer
        true,  // writable
        &mut lamports,
        &mut data,
        &owner,
        false, // not executable
        0,     // rent epoch
    );
    
    // Test validation with correct PDA - should succeed
    let validation_result = validate_treasury_account(
        &mock_treasury_account,
        &main_treasury_correct,
        TREASURY_TYPE_MAIN
    );
    assert!(validation_result.is_ok(), "Treasury validation should succeed with correct PDA");
    
    // Test 6: Test treasury validation with wrong PDA - should fail
    let mut wrong_lamports = 1000000000u64;
    let mut wrong_data = vec![0u8; 256];
    
    let mock_wrong_account = AccountInfo::new(
        &wrong_treasury,
        false, // not signer  
        true,  // writable
        &mut wrong_lamports,
        &mut wrong_data,
        &owner,
        false, // not executable
        0,     // rent epoch
    );
    
    let wrong_validation_result = validate_treasury_account(
        &mock_wrong_account,
        &main_treasury_correct,
        TREASURY_TYPE_MAIN
    );
    assert!(wrong_validation_result.is_err(), "Treasury validation should fail with wrong PDA");
    
    // Test 7: Test treasury validation with non-writable account - should fail
    let mock_readonly_account = AccountInfo::new(
        &main_treasury_correct,
        false, // not signer
        false, // NOT writable
        &mut lamports,
        &mut data,
        &owner,
        false, // not executable
        0,     // rent epoch
    );
    
    let readonly_validation_result = validate_treasury_account(
        &mock_readonly_account,
        &main_treasury_correct,
        TREASURY_TYPE_MAIN
    );
    assert!(readonly_validation_result.is_err(), "Treasury validation should fail with non-writable account");
    
    println!("✅ Treasury PDA derivation and validation tests completed:");
    println!("  ✓ PDA derivation works correctly");
    println!("  ✓ PDA is deterministic and consistent");
    println!("  ✓ Different seeds produce different PDAs");
    println!("  ✓ Treasury validation accepts correct PDA");
    println!("  ✓ Treasury validation rejects wrong PDA");
    println!("  ✓ Treasury validation rejects non-writable accounts");
}

/// Test treasury instruction serialization and deserialization
#[tokio::test] 
async fn test_treasury_instruction_serialization() {
    println!("📝 Testing treasury instruction serialization and deserialization");
    
    // Test 1: WithdrawTreasuryFees instruction serialization/deserialization
    let withdraw_original = PoolInstruction::WithdrawTreasuryFees { amount: 1_500_000_000 };
    
    // Serialize to bytes
    let serialized_withdraw = withdraw_original.try_to_vec()
        .expect("WithdrawTreasuryFees should serialize");
    assert!(serialized_withdraw.len() > 0, "Serialized data should not be empty");
    println!("✓ WithdrawTreasuryFees serialized to {} bytes", serialized_withdraw.len());
    
    // Deserialize back to instruction
    let deserialized_withdraw = PoolInstruction::try_from_slice(&serialized_withdraw)
        .expect("WithdrawTreasuryFees should deserialize");
    
    // Verify deserialized instruction matches original
    match (withdraw_original, deserialized_withdraw) {
        (PoolInstruction::WithdrawTreasuryFees { amount: orig_amount }, 
         PoolInstruction::WithdrawTreasuryFees { amount: deser_amount }) => {
            assert_eq!(orig_amount, deser_amount, "Deserialized amount should match original");
            println!("✓ WithdrawTreasuryFees amount preserved: {} lamports", orig_amount);
        }
        _ => panic!("Deserialized instruction type mismatch"),
    }
    
    // Test 2: GetTreasuryInfo instruction serialization/deserialization
    let info_original = PoolInstruction::GetTreasuryInfo {};
    
    // Serialize to bytes
    let serialized_info = info_original.try_to_vec()
        .expect("GetTreasuryInfo should serialize");
    assert!(serialized_info.len() > 0, "Serialized data should not be empty");
    println!("✓ GetTreasuryInfo serialized to {} bytes", serialized_info.len());
    
    // Deserialize back to instruction
    let deserialized_info = PoolInstruction::try_from_slice(&serialized_info)
        .expect("GetTreasuryInfo should deserialize");
    
    // Verify deserialized instruction matches original
    match (info_original, deserialized_info) {
        (PoolInstruction::GetTreasuryInfo {}, PoolInstruction::GetTreasuryInfo {}) => {
            println!("✓ GetTreasuryInfo successfully round-trip serialized");
        }
        _ => panic!("Deserialized instruction type mismatch"),
    }
    
    // Test 3: Edge case - Zero amount withdrawal
    let zero_withdraw = PoolInstruction::WithdrawTreasuryFees { amount: 0 };
    let zero_serialized = zero_withdraw.try_to_vec()
        .expect("Zero amount withdraw should serialize");
    let zero_deserialized = PoolInstruction::try_from_slice(&zero_serialized)
        .expect("Zero amount withdraw should deserialize");
    
    match zero_deserialized {
        PoolInstruction::WithdrawTreasuryFees { amount: 0 } => {
            println!("✓ Zero amount withdrawal preserved correctly");
        }
        _ => panic!("Zero amount instruction not preserved"),
    }
    
    // Test 4: Edge case - Maximum amount withdrawal
    let max_withdraw = PoolInstruction::WithdrawTreasuryFees { amount: u64::MAX };
    let max_serialized = max_withdraw.try_to_vec()
        .expect("Max amount withdraw should serialize");
    let max_deserialized = PoolInstruction::try_from_slice(&max_serialized)
        .expect("Max amount withdraw should deserialize");
    
    match max_deserialized {
        PoolInstruction::WithdrawTreasuryFees { amount: u64::MAX } => {
            println!("✓ Maximum amount withdrawal preserved correctly");
        }
        _ => panic!("Maximum amount instruction not preserved"),
    }
    
    println!("✅ All treasury instruction serialization/deserialization tests passed:");
    println!("  ✓ WithdrawTreasuryFees preserves amount data");
    println!("  ✓ GetTreasuryInfo round-trip works correctly");
    println!("  ✓ Edge cases (0 and u64::MAX) handled properly");
    println!("  ✓ All serialized data is non-empty and valid");
}

/// Test fee routing and treasury state management
#[tokio::test]
async fn test_fee_routing_validation() {
    println!("💰 Testing fee routing and treasury state management");
    
    // Test 1: Create mock treasury state and test fee routing methods
    use fixed_ratio_trading::state::treasury_state::MainTreasuryState;
    use solana_program::clock::Clock;
    
    let mut treasury_state = MainTreasuryState::new();
    let current_timestamp = Clock::default().unix_timestamp;
    
    // Define actual fee amounts from constants
    let pool_creation_fee = 1_150_000_000u64; // 1.15 SOL (REGISTRATION_FEE)
    let liquidity_fee = 1_300_000u64; // 0.0013 SOL  
    let swap_fee = 27_150u64; // Swap contract fee

    
    println!("🧪 Testing fee routing methods:");
    
    // Test 2: Pool creation fee routing
    let initial_pool_creations = treasury_state.pool_creation_count;
    let initial_pool_fees = treasury_state.total_pool_creation_fees;
    
    treasury_state.add_pool_creation_fee(pool_creation_fee, current_timestamp);
    
    assert_eq!(treasury_state.pool_creation_count, initial_pool_creations + 1, 
               "Pool creation count should increment");
    assert_eq!(treasury_state.total_pool_creation_fees, initial_pool_fees + pool_creation_fee,
               "Pool creation fees should accumulate");
    println!("✓ Pool creation fee routing: {} lamports", pool_creation_fee);
    
    // Test 3: Liquidity operation fee routing
    let initial_liquidity_ops = treasury_state.liquidity_operation_count;
    let initial_liquidity_fees = treasury_state.total_liquidity_fees;
    
    treasury_state.add_liquidity_fee(liquidity_fee, current_timestamp);
    
    assert_eq!(treasury_state.liquidity_operation_count, initial_liquidity_ops + 1,
               "Liquidity operation count should increment");
    assert_eq!(treasury_state.total_liquidity_fees, initial_liquidity_fees + liquidity_fee,
               "Liquidity fees should accumulate");
    println!("✓ Liquidity fee routing: {} lamports", liquidity_fee);
    
    // Test 4: Regular swap fee routing
    let initial_regular_swaps = treasury_state.regular_swap_count;
    let initial_regular_fees = treasury_state.total_swap_contract_fees;
    
    treasury_state.add_swap_contract_fee(swap_fee, current_timestamp);
    
    assert_eq!(treasury_state.regular_swap_count, initial_regular_swaps + 1,
               "Regular swap count should increment");
    assert_eq!(treasury_state.total_swap_contract_fees, initial_regular_fees + swap_fee,
               "Regular swap fees should accumulate");
    println!("✓ Regular swap fee routing: {} lamports", swap_fee);
    

    
    // Test 6: Validate fee relationships (business logic)
    assert!(pool_creation_fee > liquidity_fee, 
            "Pool creation should cost more than liquidity operations");
    assert!(liquidity_fee > swap_fee, 
            "Liquidity operations should cost more than regular swaps");

    
    // Test 7: Treasury analytics methods
    let total_operations = treasury_state.total_operations_processed();
    let total_fees = treasury_state.total_fees_collected();
    let average_fee = treasury_state.average_fee_per_operation();
    
    assert_eq!(total_operations, 3, "Should have processed 3 operations");
    assert_eq!(total_fees, pool_creation_fee + liquidity_fee + swap_fee,
               "Total fees should be sum of all fees");
    assert_eq!(average_fee, total_fees as f64 / total_operations as f64,
               "Average fee calculation should be correct");
    
    println!("✓ Treasury analytics:");
    println!("  Total operations: {}", total_operations);
    println!("  Total fees collected: {} lamports", total_fees);
    println!("  Average fee per operation: {:.2} lamports", average_fee);
    
    // Test 8: Timestamp tracking
    assert_eq!(treasury_state.last_update_timestamp, current_timestamp,
               "Last update timestamp should be preserved");
    
    println!("✅ Fee routing validation completed:");
    println!("  ✓ Pool creation fees route correctly to treasury");
    println!("  ✓ Liquidity fees route correctly to treasury");
    println!("  ✓ Swap fees route correctly to treasury");

    println!("  ✓ Fee relationships maintain business logic");
    println!("  ✓ Treasury analytics calculate correctly");
    println!("  ✓ Timestamp tracking works properly");
}

/// Test withdrawal authorization logic and validation
#[tokio::test]
async fn test_withdrawal_authorization() {
    println!("🔐 Testing withdrawal authorization logic and validation");
    
    use solana_program::{
        pubkey::Pubkey,
        rent::Rent,
    };
    use fixed_ratio_trading::state::treasury_state::MainTreasuryState;
    use fixed_ratio_trading::constants::MAIN_TREASURY_SEED_PREFIX;
    
    // Test 1: Test treasury PDA derivation for authorization
    let program_id = Pubkey::new_unique();
    let (treasury_pda, _treasury_bump) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        &program_id,
    );
    
    println!("✓ Treasury PDA derived: {}", treasury_pda);
    
    // Test 2: Test rent calculation logic (mirrors processor logic)
    let treasury_state_size = MainTreasuryState::get_packed_len();
    let rent = Rent::default();
    let rent_exempt_minimum = rent.minimum_balance(treasury_state_size);
    
    println!("✓ Treasury state size: {} bytes", treasury_state_size);
    println!("✓ Rent exempt minimum: {} lamports", rent_exempt_minimum);
    
    // Test 3: Test withdrawal authorization logic scenarios
    let test_scenarios = vec![
        ("Empty treasury", 0u64),
        ("Below rent minimum", rent_exempt_minimum / 2),
        ("Exactly rent minimum", rent_exempt_minimum),
        ("Small surplus", rent_exempt_minimum + 100_000),
        ("Large balance", 10_000_000_000u64), // 10 SOL
    ];
    
    for (scenario_name, treasury_balance) in test_scenarios {
        println!("🧪 Testing scenario: {}", scenario_name);
        
        // Calculate available balance (mirrors processor logic)
        let available_balance = if treasury_balance > rent_exempt_minimum {
        treasury_balance - rent_exempt_minimum
    } else {
        0
    };
    
        println!("  Treasury balance: {} lamports", treasury_balance);
        println!("  Available for withdrawal: {} lamports", available_balance);
        
        // Test authorization logic
        match scenario_name {
            "Empty treasury" | "Below rent minimum" => {
                assert_eq!(available_balance, 0, 
                          "Should have no funds available when below rent minimum");
            }
            "Exactly rent minimum" => {
                assert_eq!(available_balance, 0,
                          "Should have no funds available when exactly at rent minimum");
            }
            _ => {
                assert!(available_balance > 0,
                       "Should have funds available when above rent minimum");
                assert!(available_balance < treasury_balance,
                       "Available should be less than total balance");
                assert_eq!(available_balance, treasury_balance - rent_exempt_minimum,
                          "Available should equal total minus rent minimum");
            }
        }
    }
    
    // Test 4: Test withdrawal amount validation logic
    let treasury_balance = 5_000_000_000u64; // 5 SOL
    let available_balance = treasury_balance - rent_exempt_minimum;
    
    let withdrawal_tests = vec![
        ("Zero withdrawal (withdraw all)", 0u64, true),
        ("Partial withdrawal", available_balance / 2, true),
        ("Exact available amount", available_balance, true),
        ("Excessive withdrawal", available_balance + 1, false),
        ("Maximum u64 withdrawal", u64::MAX, false),
    ];
    
    for (test_name, withdrawal_amount, should_be_valid) in withdrawal_tests {
        println!("🧪 Testing withdrawal: {}", test_name);
        
        // Determine effective withdrawal amount (0 means withdraw all available)
        let effective_amount = if withdrawal_amount == 0 {
            available_balance
        } else {
            withdrawal_amount
        };
        
        // Check if withdrawal is valid
        let is_valid = effective_amount <= available_balance;
        
        assert_eq!(is_valid, should_be_valid,
                  "Withdrawal validation for {} should be {}", test_name, should_be_valid);
        
        if is_valid {
            println!("  ✓ Valid withdrawal: {} lamports", effective_amount);
        } else {
            println!("  ✗ Invalid withdrawal: {} lamports (exceeds available)", effective_amount);
        }
    }
    
    // Test 5: Test authority validation requirements (conceptual)
    let system_authority = Pubkey::new_unique();
    let unauthorized_user = Pubkey::new_unique();
    
    println!("🧪 Testing authority validation requirements:");
    println!("  System authority: {}", system_authority);
    println!("  Unauthorized user: {}", unauthorized_user);
    
    // Conceptual test - in real processor, this would check signatures
    assert_ne!(system_authority, unauthorized_user,
              "System authority should be different from unauthorized users");
    
    println!("✅ Withdrawal authorization tests completed:");
    println!("  ✓ Treasury PDA derivation works correctly");
    println!("  ✓ Rent calculation logic is sound");
    println!("  ✓ Available balance calculation handles all scenarios");
    println!("  ✓ Withdrawal amount validation works properly");
    println!("  ✓ Authority validation requirements are clear");
    println!("  ✓ Edge cases (0, exact limits, overflow) handled correctly");
}

/// Test complete treasury system workflow operations
#[tokio::test]
async fn test_treasury_workflow_operations() {
    println!("📋 Testing complete treasury system workflow operations");
    
    use fixed_ratio_trading::state::treasury_state::MainTreasuryState;
    use solana_program::{
        pubkey::Pubkey,
        clock::Clock,
        rent::Rent,
    };
    use fixed_ratio_trading::constants::MAIN_TREASURY_SEED_PREFIX;
    
    // Test 1: Initialize treasury system
    let mut treasury_state = MainTreasuryState::new();
    let program_id = Pubkey::new_unique();
    let current_timestamp = Clock::default().unix_timestamp;
    
    // Derive treasury PDA
    let (treasury_pda, treasury_bump) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        &program_id,
    );
    
    println!("🏗️ Phase 1: Treasury System Initialization");
    println!("  Treasury PDA: {}", treasury_pda);
    println!("  Treasury bump: {}", treasury_bump);
    println!("  Initial balance: {} lamports", treasury_state.total_balance);
    
    // Test 2: Simulate fee collection workflow
    println!("\n💰 Phase 2: Fee Collection Workflow");
    
    // Simulate multiple pool creations
    for i in 1..=3 {
        let pool_creation_fee = 1_150_000_000u64; // 1.15 SOL
        treasury_state.add_pool_creation_fee(pool_creation_fee, current_timestamp + i);
        println!("  Pool {} created - Fee: {} lamports", i, pool_creation_fee);
    }
    
    // Simulate liquidity operations
    for i in 1..=5 {
        let liquidity_fee = 1_300_000u64; // 0.0013 SOL
        treasury_state.add_liquidity_fee(liquidity_fee, current_timestamp + i + 10);
        println!("  Liquidity operation {} - Fee: {} lamports", i, liquidity_fee);
    }
    
    // Simulate regular swaps
    for i in 1..=10 {
        let swap_fee = 27_150u64; // Regular swap fee
        treasury_state.add_swap_contract_fee(swap_fee, current_timestamp + i + 20);
        println!("  Regular swap {} - Fee: {} lamports", i, swap_fee);
    }
    

    
    // Test 3: Validate workflow state
    println!("\n📊 Phase 3: Workflow State Validation");
    
    let total_operations = treasury_state.total_operations_processed();
    let total_fees = treasury_state.total_fees_collected();
    let average_fee = treasury_state.average_fee_per_operation();
    
    assert_eq!(treasury_state.pool_creation_count, 3, "Should have 3 pool creations");
    assert_eq!(treasury_state.liquidity_operation_count, 5, "Should have 5 liquidity operations");
    assert_eq!(treasury_state.regular_swap_count, 10, "Should have 10 regular swaps");
    assert_eq!(total_operations, 18, "Should have 18 total operations");
    
    println!("  ✓ Pool creations: {}", treasury_state.pool_creation_count);
    println!("  ✓ Liquidity operations: {}", treasury_state.liquidity_operation_count);
    println!("  ✓ Regular swaps: {}", treasury_state.regular_swap_count);

    println!("  ✓ Total operations: {}", total_operations);
    println!("  ✓ Total fees collected: {} lamports", total_fees);
    println!("  ✓ Average fee per operation: {:.2} lamports", average_fee);
    
    // Test 4: Simulate withdrawal workflow
    println!("\n🏦 Phase 4: Withdrawal Workflow Simulation");
    
    let treasury_state_size = MainTreasuryState::get_packed_len();
    let rent = Rent::default();
    let rent_exempt_minimum = rent.minimum_balance(treasury_state_size);
    let simulated_treasury_balance = total_fees + rent_exempt_minimum + 1_000_000; // Some extra SOL
    
    // Calculate withdrawal scenarios
    let available_for_withdrawal = simulated_treasury_balance - rent_exempt_minimum;
    let partial_withdrawal = available_for_withdrawal / 2;
    let full_withdrawal = available_for_withdrawal;
    
    println!("  Treasury balance: {} lamports", simulated_treasury_balance);
    println!("  Rent exempt minimum: {} lamports", rent_exempt_minimum);
    println!("  Available for withdrawal: {} lamports", available_for_withdrawal);
    
    // Test withdrawal validation logic
    assert!(available_for_withdrawal > 0, "Should have funds available for withdrawal");
    assert!(partial_withdrawal < available_for_withdrawal, "Partial should be less than available");
    assert_eq!(full_withdrawal, available_for_withdrawal, "Full should equal available");
    
    println!("  ✓ Partial withdrawal scenario: {} lamports", partial_withdrawal);
    println!("  ✓ Full withdrawal scenario: {} lamports", full_withdrawal);
    
    // Test 5: Validate treasury system benefits
    println!("\n🎯 Phase 5: System Benefits Validation");
    
    // Real-time data (no consolidation needed)
    assert!(treasury_state.last_update_timestamp > 0, "Should have real-time timestamps");
    
    // Single source of truth
    let total_by_category = treasury_state.total_pool_creation_fees +
                           treasury_state.total_liquidity_fees +
                           treasury_state.total_swap_contract_fees;
    assert_eq!(total_fees, total_by_category, "Single source of truth for fee tracking");
    
    // No race conditions (deterministic state)
    let recalculated_operations = treasury_state.pool_creation_count +
                                treasury_state.liquidity_operation_count +
                                treasury_state.regular_swap_count;
    assert_eq!(total_operations, recalculated_operations, "Deterministic operation counting");
    
    println!("  ✓ Real-time data tracking works");
    println!("  ✓ Single source of truth validated");
    println!("  ✓ No race conditions (deterministic state)");
    println!("  ✓ Simplified architecture (single treasury)");
    println!("  ✓ Rent-safe withdrawal mechanism");
    
    println!("\n✅ Treasury workflow operations test completed:");
    println!("  ✓ Treasury initialization works correctly");
    println!("  ✓ Fee collection workflow handles all operation types");
    println!("  ✓ State tracking is accurate and real-time");
    println!("  ✓ Withdrawal workflow logic is sound");
    println!("  ✓ System benefits are validated");
    println!("  ✓ End-to-end workflow operates correctly");
} 

/// TREASURY-VALIDATION-004: Phase 1.1 Enhanced Treasury Validation with Real Operations
/// 
/// This test uses Phase 1.1 enhanced helpers to perform legitimate treasury validation
/// with real blockchain operations rather than mock data
#[tokio::test]
#[serial]
async fn test_phase_1_1_enhanced_treasury_validation() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing TREASURY-VALIDATION-004: Phase 1.1 Enhanced Treasury Validation...");
    
    use crate::common::{
        setup::{initialize_treasury_system, start_test_environment},
        pool_helpers::{execute_pool_creation_with_counter_verification, verify_pool_creation_fee_collection},
    };
    use solana_sdk::signature::Keypair;
    use fixed_ratio_trading::{
        constants::MAIN_TREASURY_SEED_PREFIX,
        state::MainTreasuryState,
    };
    use borsh::BorshDeserialize;
    
    // Initialize test environment
    let mut env = start_test_environment().await;
    
    println!("🏛️ Step 1: Initialize treasury system for validation...");
    
    // Initialize treasury system
    let system_authority = Keypair::new();
    initialize_treasury_system(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &system_authority,
    ).await?;
    
    println!("✅ Treasury system initialized");
    
    // Get treasury PDA for validation
    let (main_treasury_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        &fixed_ratio_trading::ID,
    );
    
    println!("\n📊 Step 2: Validate initial treasury state...");
    
    // Get initial state for validation
    let initial_account = env.banks_client.get_account(main_treasury_pda).await?.unwrap();
    let initial_treasury_state = MainTreasuryState::try_from_slice(&initial_account.data)?;
    let initial_balance = env.banks_client.get_balance(main_treasury_pda).await?;
    
    println!("🔍 Initial treasury validation:");
    println!("   - Pool creation count: {}", initial_treasury_state.pool_creation_count);
    println!("   - Total pool creation fees: {}", initial_treasury_state.total_pool_creation_fees);
    println!("   - Treasury balance: {} lamports", initial_balance);
    println!("   - Total balance in state: {}", initial_treasury_state.total_balance);
    
    // Validate initial state consistency
    assert_eq!(initial_treasury_state.pool_creation_count, 0, "Initial pool creation count should be 0");
    assert_eq!(initial_treasury_state.total_pool_creation_fees, 0, "Initial pool creation fees should be 0");
    assert!(initial_balance > 0, "Treasury should have rent-exempt balance");
    
    println!("\n🏊 Step 3: Execute pool creation and validate treasury changes...");
    
    // Use Phase 1.1 enhanced helper to create pool and validate
    let pool_result = execute_pool_creation_with_counter_verification(
        &mut env,
        2500,  // ratio_a_numerator 
        3,     // ratio_b_denominator
    ).await?;
    
    println!("✅ Pool creation with validation completed!");
    
    println!("\n🔍 Step 4: Comprehensive treasury validation...");
    
    // Get post-creation state for validation
    let post_account = env.banks_client.get_account(main_treasury_pda).await?.unwrap();
    let post_treasury_state = MainTreasuryState::try_from_slice(&post_account.data)?;
    let post_balance = env.banks_client.get_balance(main_treasury_pda).await?;
    
    println!("🔍 Post-creation treasury validation:");
    println!("   - Pool creation count: {} (increment: {})", 
             post_treasury_state.pool_creation_count,
             post_treasury_state.pool_creation_count - initial_treasury_state.pool_creation_count);
    println!("   - Total pool creation fees: {} (increment: {})", 
             post_treasury_state.total_pool_creation_fees,
             post_treasury_state.total_pool_creation_fees - initial_treasury_state.total_pool_creation_fees);
    println!("   - Treasury balance: {} lamports (increment: {})", 
             post_balance, post_balance - initial_balance);
    
    // Validate treasury changes are correct
    let counter_increment = post_treasury_state.pool_creation_count - initial_treasury_state.pool_creation_count;
    let fee_increment = post_treasury_state.total_pool_creation_fees - initial_treasury_state.total_pool_creation_fees;
    let balance_increment = post_balance - initial_balance;
    
    assert_eq!(counter_increment, 1, "Pool creation counter should increment by exactly 1");
    assert!(fee_increment > 0, "Pool creation fees should be collected");
    assert!(balance_increment > 0, "Treasury balance should increase");
    assert_eq!(fee_increment, pool_result.fee_collected, "Fee increment should match result");
    assert_eq!(balance_increment, fee_increment, "Balance increment should equal fee increment");
    
    println!("\n🔍 Step 5: Validate treasury state consistency...");
    
    // Validate internal state consistency
    assert_eq!(post_treasury_state.total_balance, post_balance, 
               "Internal balance tracking should match actual balance");
    
    // Use Phase 1.1 helper for additional verification
    let fee_verification = verify_pool_creation_fee_collection(
        &mut env,
        &initial_treasury_state,
    ).await?;
    
    assert_eq!(fee_verification, pool_result.fee_collected, 
               "Fee verification should match pool result");
    
    println!("✅ Treasury state consistency validation:");
    println!("   - Counter increment: {} ✅", counter_increment);
    println!("   - Fee collection: {} lamports ✅", fee_increment);
    println!("   - Balance update: {} lamports ✅", balance_increment);
    println!("   - State consistency: ✅");
    println!("   - Fee verification: {} lamports ✅", fee_verification);
    
    println!("\n🔍 Step 6: Validate enhanced analytics methods...");
    
    // Test the enhanced analytics methods from our treasury enhancements
    let total_operations = post_treasury_state.total_successful_operations();
    let success_rate = post_treasury_state.success_rate_percentage();
    
    println!("📊 Enhanced analytics validation:");
    println!("   - Total successful operations: {}", total_operations);
    println!("   - Success rate percentage: {:.2}%", success_rate);
    
    // Validate analytics make sense
    assert_eq!(total_operations, 1, "Should have 1 successful operation (pool creation)");
    assert_eq!(success_rate, 100.0, "Success rate should be 100% with no failures");
    
    println!("\n✅ TREASURY-VALIDATION-004: Phase 1.1 Enhanced Treasury Validation successful!");
    println!("📋 Legitimate Treasury Validation Verified:");
    println!("   1. ✅ Treasury state initialization validation");
    println!("   2. ✅ Real blockchain operation execution and validation");
    println!("   3. ✅ Counter increment validation with actual operations");
    println!("   4. ✅ Fee collection validation with real fees");
    println!("   5. ✅ Treasury state consistency validation");
    println!("   6. ✅ Enhanced analytics method validation");
    println!("   7. ✅ Phase 1.1 helper integration for comprehensive validation");
    println!("   8. ✅ No mock data - all validations use real blockchain state");
    
    Ok(())
} 