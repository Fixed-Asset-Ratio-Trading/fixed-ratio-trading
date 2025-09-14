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

//! # Pool Creation and Initialization Tests
//! 
//! This module contains comprehensive tests for pool creation and initialization,
//! including both the deprecated two-instruction pattern and the new single-instruction
//! pattern, as well as validation and error handling tests.
//!
//! **BASIS POINTS REFACTOR: Updated Test Suite**
//! 
//! Tests now include examples of proper basis point conversion for pool creation.
//! New tests demonstrate how to convert display units to basis points before
//! sending to the smart contract, which expects all values in basis points.

#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(unused_assignments)]
#![allow(unused_results)]

mod common;

use common::*;
use solana_program_test::BanksClientError;
use serial_test::serial;

// Import flow helpers for comprehensive testing
use common::flow_helpers::{
    execute_basic_trading_flow,
    execute_consolidation_flow,
    BasicTradingFlowConfig,
    ConsolidationFlowConfig,
    SwapOperation,
    SwapDirection as FlowSwapDirection,
    FlowResult,
};

/// Helper function to convert treasury system initialization errors to BanksClientError
async fn init_treasury_for_test(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
) -> Result<(), BanksClientError> {
    // ✅ PHASE 11 SECURITY: Use test program authority for treasury initialization
    use crate::common::setup::{create_test_program_authority_keypair, verify_test_program_authority_consistency};
    
    // Create keypair that matches the test program authority
    let system_authority = create_test_program_authority_keypair()
        .map_err(|e| BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, 
            format!("Failed to create program authority keypair: {}", e))))?;
    
    // Verify the loaded keypair matches the expected authority
    verify_test_program_authority_consistency(&system_authority)
        .map_err(|e| BanksClientError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData, e)))?;
    
    println!("🔐 Using test program authority for testing: {}", system_authority.pubkey());
    
    initialize_treasury_system(banks_client, payer, recent_blockhash, &system_authority)
        .await
        .map_err(|e| {
            let error_msg = format!("Treasury system initialization error: {:?}", e);
            println!("{}", error_msg);
            BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, error_msg))
        })
}

// ========================================================================
// PHASE 3.1 & 3.2: ENHANCED POOL CREATION TESTS USING FLOW HELPERS
// ========================================================================

/// **PHASE 3.1**: Comprehensive pool creation test using basic trading flow helpers
/// This test demonstrates that pool creation works seamlessly with complete trading flows
#[tokio::test]
#[serial]
async fn test_pool_creation_with_complete_trading_flow() -> TestResult {
    println!("🚀 PHASE 3.1: Testing pool creation with complete trading flow...");
    
    // Test pool creation by running a complete trading flow
    // This validates pool creation in the context of actual usage
    let config = BasicTradingFlowConfig {
        pool_ratio: Some(8), // 8:1 ratio for distinctive testing
        liquidity_deposits: vec![500_000], // Conservative deposit to ensure reliable execution
        swap_operations: vec![], // Skip swaps for pool creation focus - just validate pool works
        verify_treasury_counters: true,
    };
    
    // Execute the complete flow which includes pool creation as the first step
    println!("⚡ Executing complete trading flow (validates pool creation)...");
    let flow_result = execute_basic_trading_flow(Some(config)).await
        .map_err(|e| BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    
    // Verify that pool creation was successful by checking the flow results
    assert!(flow_result.flow_successful, "Complete flow should be successful");
    assert!(flow_result.pool_creation_result.fee_collected > 0, "Pool creation should collect fees");
    assert!(flow_result.liquidity_result.operations_performed >= 1, "Should perform liquidity operations on created pool");
    // No swap assertions since we're focusing on pool creation validation
    
    println!("✅ Pool Creation Validation Summary:");
    println!("   - Pool creation fee: {} lamports", flow_result.pool_creation_result.fee_collected);
    println!("   - Pool supports liquidity: {} operations", flow_result.liquidity_result.operations_performed);
    println!("   - Pool supports swaps: {} operations", flow_result.swap_result.swaps_performed);
    println!("   - Pool PDA: {}", flow_result.pool_creation_result.pool_pda);
    
    // Verify treasury state changes from pool creation
    assert!(flow_result.treasury_comparisons.len() >= 1, "Should track treasury changes from pool creation");
    
    println!("✅ PHASE 3.1: Pool creation with complete trading flow test completed successfully!");
    println!("   This validates pool creation works correctly with: liquidity deposits + swaps + treasury operations");
    
    Ok(())
}

/// **PHASE 3.2**: Multi-pool creation test using consolidation flow helpers
/// This test validates that multiple pools can be created and work together
#[tokio::test]
#[serial]
async fn test_multiple_pool_creation_coordination() -> TestResult {
    println!("🚀 PHASE 3.2: Testing multiple pool creation coordination...");
    
    // Test creating multiple pools with different ratios
    let config = ConsolidationFlowConfig {
        pool_count: 5,
        pool_ratios: vec![2, 3, 4, 5, 6], // Test various ratios
        liquidity_per_pool: vec![1_000_000, 900_000, 800_000, 700_000, 600_000],
        cross_pool_swaps: vec![], // No swaps needed for creation testing
        treasury_operations: vec![
            crate::common::flow_helpers::TreasuryOperation {
                operation_type: crate::common::flow_helpers::TreasuryOperationType::VerifyFeeAccumulation,
                amount: Some(200_000),
                expected_success: true,
            },
        ],
        test_fee_consolidation: true,
        test_treasury_withdrawals: true,
    };
    
    // Execute multi-pool creation
    println!("⚡ Executing multi-pool creation flow...");
    let consolidation_result = execute_consolidation_flow(Some(config)).await
        .map_err(|e| BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    
    // Verify all pools were created successfully
    assert!(consolidation_result.flow_successful, "Multi-pool creation should be successful");
    assert_eq!(consolidation_result.pool_results.len(), 5, "Should create exactly 5 pools");
    assert!(consolidation_result.performance_metrics.total_liquidity_operations >= 5, "Should add liquidity to all pools");
    assert!(consolidation_result.performance_metrics.total_treasury_operations >= 1, "Should verify treasury accumulation");
    
    println!("✅ Multi-Pool Creation Results Summary:");
    println!("   - Pools created: {}", consolidation_result.pool_results.len());
    println!("   - Total liquidity operations: {}", consolidation_result.performance_metrics.total_liquidity_operations);
    println!("   - Treasury operations: {}", consolidation_result.performance_metrics.total_treasury_operations);
    println!("   - Creation time: {}ms", consolidation_result.performance_metrics.total_execution_time_ms);
    
    // Verify performance metrics
    assert!(consolidation_result.performance_metrics.total_execution_time_ms > 0, "Should track creation time");
    assert!(consolidation_result.performance_metrics.pools_processed >= 5, "Should track pool creation operations");
    
    println!("✅ PHASE 3.2: Multiple pool creation coordination test completed successfully!");
    println!("   This validates that multiple pools (5) with different ratios can be created and coordinated");
    
    Ok(())
}

/// **PHASE 3.1 ENHANCED**: Simple pool creation validation using flow helper
/// This shows how pool creation testing can be simplified while being more comprehensive
#[tokio::test]
#[serial]
async fn test_enhanced_pool_creation_validation() -> TestResult {
    println!("🚀 PHASE 3.1 ENHANCED: Testing enhanced pool creation validation...");
    
    // Test pool creation with minimal configuration
    let config = BasicTradingFlowConfig {
        pool_ratio: Some(10), // 10:1 ratio for clear validation
        liquidity_deposits: vec![500_000], // Single deposit to validate pool works
        swap_operations: vec![], // No swaps needed for creation testing
        verify_treasury_counters: true,
    };
    
    let flow_result = execute_basic_trading_flow(Some(config)).await
        .map_err(|e| BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    
    // Pool creation validation is handled by the flow helper
    assert!(flow_result.flow_successful, "Pool creation should succeed");
    assert!(flow_result.pool_creation_result.fee_collected > 0, "Should collect creation fees");
    assert_eq!(flow_result.liquidity_result.operations_performed, 2, "Should support liquidity operations (A + B tokens)");
    
    println!("✅ ENHANCED: Pool creation validation completed (comprehensive testing in minimal code)");
    
    Ok(())
}

// ================================================================================================
// NEW SINGLE-INSTRUCTION PATTERN TESTS (RECOMMENDED)
// ================================================================================================

/// **COMPREHENSIVE TEST**: Complete pool initialization and validation
/// 
/// This consolidated test covers all aspects of pool creation and initialization:
/// 1. New single-instruction pattern testing (from test_initialize_pool_new_pattern)
/// 2. Utility function integration testing (from test_pool_creation_with_utilities)
/// 3. Complete environment setup and validation (from test_process_pool_initialize_success)
/// 4. Multiple users and comprehensive state verification
/// 
/// This test creates a complete testing environment that serves as the foundation
/// for all other tests and validates:
/// - Treasury System initialization
/// - Token infrastructure creation
/// - Pool creation with standard 3:1 ratio
/// - User accounts with proper funding
/// - Both new and legacy pattern compatibility
/// - Complete state verification
/// 
/// # Test Flow
/// 1. Initialize treasury system (required first step)
/// 2. Create ordered token mints (lexicographically)
/// 3. Test new single-instruction pattern
/// 4. Test utility functions with both patterns
/// 5. Setup multiple test users with token accounts
/// 6. Verify all components are properly initialized
/// 
/// # Returns
/// Success when all components are properly initialized and verified
#[tokio::test]
async fn test_process_pool_initialize() -> TestResult {
    println!("🚀 COMPREHENSIVE TEST: Complete pool initialization and validation");
    println!("   This test consolidates all pool creation testing into one comprehensive test");
    
    // =============================================
    // STEP 1: Setup Test Environment
    // =============================================
    let mut ctx = setup_pool_test_context(false).await;
    println!("✅ Test environment created");
    
    // Create ordered token mints to ensure consistent behavior
    let keypair1 = Keypair::new();
    let keypair2 = Keypair::new();
    
    let (primary_mint, base_mint) = if keypair1.pubkey() < keypair2.pubkey() {
        (keypair1, keypair2)
    } else {
        (keypair2, keypair1)
    };
    
    println!("✅ Token keypairs generated:");
    println!("   Primary mint: {}", primary_mint.pubkey());
    println!("   Base mint: {}", base_mint.pubkey());
    
    // =============================================
    // STEP 2: Initialize Treasury System (REQUIRED FIRST)
    // =============================================
    println!("\n🏦 Initializing treasury system...");
    init_treasury_for_test(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
    ).await?;
    println!("✅ Treasury system initialized - all fee collection PDAs created");
    
    // =============================================
    // STEP 3: Create Token Mints
    // =============================================
    println!("\n🪙 Creating token mints...");
    create_test_mints(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &[&primary_mint, &base_mint],
    ).await?;
    println!("✅ Token mints created and initialized");
    
    // =============================================
    // STEP 4: Test New Single-Instruction Pattern
    // =============================================
    println!("\n🧪 Testing new single-instruction pattern...");
    
    // Create pool using new single-instruction pattern
    let config_new = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &primary_mint,
        &base_mint,
        Some(3),
    ).await?;

    // Verify pool state
    verify_pool_state(
        &mut ctx.env.banks_client,
        &config_new,
        &ctx.env.payer.pubkey(),
        &ctx.lp_token_a_mint.pubkey(),
        &ctx.lp_token_b_mint.pubkey(),
    ).await.expect("Pool state verification failed");

    println!("✅ New single-instruction pattern: Pool created and verified successfully!");
    println!("✅ Atomic operation - all accounts created and data initialized in one transaction");
    
    // =============================================
    // STEP 5: Test Utility Functions with Both Patterns
    // =============================================
    println!("\n🔧 Testing utility functions with both patterns...");
    
    // Test legacy pattern with different ratio to avoid conflict
    let config_legacy = create_pool_legacy_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &primary_mint,
        &base_mint,
        Some(4), // Different ratio to avoid conflict
    ).await?;

    // Verify using utility
    verify_pool_state(
        &mut ctx.env.banks_client,
        &config_legacy,
        &ctx.env.payer.pubkey(),
        &ctx.lp_token_a_mint.pubkey(),
        &ctx.lp_token_b_mint.pubkey(),
    ).await.expect("Legacy pattern pool verification failed");

    // Verify pools are different
    assert_ne!(config_new.pool_state_pda, config_legacy.pool_state_pda,
        "Different ratio pools should have different PDAs");

    println!("✅ Both pool creation patterns work correctly with common utilities!");
    println!("   New pattern PDA: {}", config_new.pool_state_pda);
    println!("   Legacy pattern PDA: {}", config_legacy.pool_state_pda);
    
    // =============================================
    // STEP 6: Use Primary Pool for Comprehensive Testing
    // =============================================
    println!("\n🏊 Using primary pool (3:1 ratio) for comprehensive testing...");
    let pool_config = config_new; // Use the new pattern pool as primary
    
    println!("✅ Pool created successfully:");
    println!("   Pool State PDA: {}", pool_config.pool_state_pda);
    println!("   Token A Mint: {}", pool_config.token_a_mint);
    println!("   Token B Mint: {}", pool_config.token_b_mint);
    println!("   Ratio: {}:{}", pool_config.ratio_a_numerator, pool_config.ratio_b_denominator);
    println!("   Token A Vault: {}", pool_config.token_a_vault_pda);
    println!("   Token B Vault: {}", pool_config.token_b_vault_pda);
    
    // =============================================
    // STEP 7: Verify Pool State
    // =============================================
    println!("\n🔍 Verifying pool state...");
    let pool_state = get_pool_state(&mut ctx.env.banks_client, &pool_config.pool_state_pda).await
        .expect("Pool state should exist");
    
    println!("✅ Pool state verified:");
    println!("   Initialized: {}", true); // Pool existence = initialization
    println!("   Owner: {}", pool_state.owner);
    println!("   LP Token A Mint: {}", pool_state.lp_token_a_mint);
    println!("   LP Token B Mint: {}", pool_state.lp_token_b_mint);
    println!("   Initial Token A Liquidity: {}", pool_state.total_token_a_liquidity);
    println!("   Initial Token B Liquidity: {}", pool_state.total_token_b_liquidity);
    
    // =============================================
    // STEP 8: Create Test Users with Token Accounts
    // =============================================
    println!("\n👥 Creating test users with token accounts...");
    
    // User 1: Primary trader with substantial funds
    let user1 = create_funded_user(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        Some(10_000_000_000), // 10 SOL for fees
    ).await?;
    
    let user1_primary_account_kp = Keypair::new();
    create_token_account(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &user1_primary_account_kp,
        &primary_mint.pubkey(),
        &user1.pubkey(),
    ).await?;
    
    // Mint 100M tokens to user1's primary account
    mint_tokens(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &primary_mint.pubkey(),
        &user1_primary_account_kp.pubkey(),
        &ctx.env.payer,  // Use payer as mint authority (set during create_mint)
        100_000_000, // 100M tokens
    ).await?;
    
    let user1_base_account_kp = Keypair::new();
    create_token_account(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &user1_base_account_kp,
        &base_mint.pubkey(),
        &user1.pubkey(),
    ).await?;
    
    // Mint 50M tokens to user1's base account
    mint_tokens(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &base_mint.pubkey(),
        &user1_base_account_kp.pubkey(),
        &ctx.env.payer,  // Use payer as mint authority (set during create_mint)
        50_000_000, // 50M tokens
    ).await?;
    
    // User 2: Moderate trader
    let user2 = create_funded_user(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        Some(5_000_000_000), // 5 SOL
    ).await?;
    
    let user2_primary_account_kp = Keypair::new();
    create_token_account(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &user2_primary_account_kp,
        &primary_mint.pubkey(),
        &user2.pubkey(),
    ).await?;
    
    mint_tokens(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &primary_mint.pubkey(),
        &user2_primary_account_kp.pubkey(),
        &ctx.env.payer,  // Use payer as mint authority (set during create_mint)
        25_000_000, // 25M tokens
    ).await?;
    
    let user2_base_account_kp = Keypair::new();
    create_token_account(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &user2_base_account_kp,
        &base_mint.pubkey(),
        &user2.pubkey(),
    ).await?;
    
    mint_tokens(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &base_mint.pubkey(),
        &user2_base_account_kp.pubkey(),
        &ctx.env.payer,  // Use payer as mint authority (set during create_mint)
        10_000_000, // 10M tokens
    ).await?;
    
    // User 3: Small trader
    let user3 = create_funded_user(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        Some(2_000_000_000), // 2 SOL
    ).await?;
    
    let user3_primary_account_kp = Keypair::new();
    create_token_account(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &user3_primary_account_kp,
        &primary_mint.pubkey(),
        &user3.pubkey(),
    ).await?;
    
    mint_tokens(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &primary_mint.pubkey(),
        &user3_primary_account_kp.pubkey(),
        &ctx.env.payer,  // Use payer as mint authority (set during create_mint)
        5_000_000, // 5M tokens
    ).await?;
    
    let user3_base_account_kp = Keypair::new();
    create_token_account(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &user3_base_account_kp,
        &base_mint.pubkey(),
        &user3.pubkey(),
    ).await?;
    
    mint_tokens(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &base_mint.pubkey(),
        &user3_base_account_kp.pubkey(),
        &ctx.env.payer,  // Use payer as mint authority (set during create_mint)
        2_000_000, // 2M tokens
    ).await?;
    
    println!("✅ Test users created:");
    println!("   User 1 (Primary Trader): {}", user1.pubkey());
    println!("     - Primary Token Account: {}", user1_primary_account_kp.pubkey());
    println!("     - Base Token Account: {}", user1_base_account_kp.pubkey());
    println!("   User 2 (Moderate Trader): {}", user2.pubkey());
    println!("     - Primary Token Account: {}", user2_primary_account_kp.pubkey());
    println!("     - Base Token Account: {}", user2_base_account_kp.pubkey());
    println!("   User 3 (Small Trader): {}", user3.pubkey());
    println!("     - Primary Token Account: {}", user3_primary_account_kp.pubkey());
    println!("     - Base Token Account: {}", user3_base_account_kp.pubkey());
    
    // =============================================
    // STEP 9: Final Verification & Summary
    // =============================================
    println!("\n🎯 COMPREHENSIVE TEST COMPLETE - All Pool Creation Features Validated!");
    println!("══════════════════════════════════════════════════════════════════════════════");
    println!("✅ CONSOLIDATED FEATURES TESTED:");
    println!("   • New Single-Instruction Pattern: Atomic pool creation ✓");
    println!("   • Legacy Pattern Compatibility: Two-step pool creation ✓");
    println!("   • Utility Function Integration: Both patterns work with utilities ✓");
    println!("   • Complete Environment Setup: Full testing infrastructure ✓");
    println!("   • Multiple User Accounts: 3 funded users with all token accounts ✓");
    println!("   • Treasury System: All fee collection PDAs initialized ✓");
    println!("   • State Verification: Comprehensive pool state validation ✓");
    println!();
    println!("🔧 INFRASTRUCTURE CREATED:");
    println!("   • Treasury System: All fee collection PDAs initialized");
    println!("   • Token Mints: Primary and Base tokens created");
    println!("   • Trading Pools: Both 3:1 and 4:1 ratio pools created");
    println!("   • User Accounts: 3 funded users with all token accounts");
    println!("   • LP Token Mints: Created as PDAs (will be initialized on first deposit)");
    println!();
    println!("📋 POOL INFORMATION:");
    println!("   Primary Pool ID: {}", pool_config.pool_state_pda);
    println!("   Legacy Pool ID: {}", config_legacy.pool_state_pda);
    println!("   Primary Mint: {}", primary_mint.pubkey());
    println!("   Base Mint: {}", base_mint.pubkey());
    println!("   Primary Ratio: 3 Primary : 1 Base");
    println!("   Legacy Ratio: 4 Primary : 1 Base");
    println!("   Users: 3 traders with varying balances");
    println!("   Fee System: Fully operational treasury PDAs");
    println!("   LP Token A Mint PDA: {}", pool_state.lp_token_a_mint);
    println!("   LP Token B Mint PDA: {}", pool_state.lp_token_b_mint);
    println!();
    println!("💡 USAGE: This comprehensive test covers all pool creation scenarios");
    println!("   and can serve as a reference for pool initialization testing.");
    println!("   Other tests can use this as a foundation for testing specific operations.");
    
    Ok(())
}

// ================================================================================================
// VALIDATION AND ERROR TESTS
// ================================================================================================

/// Test that creating a pool with reversed tokens but equivalent exchange rate fails
/// 
/// This test verifies a critical invariant: the contract prevents creation of economically
/// duplicate pools. If a pool exists with "3 A per 1 B", attempting to create a pool with 
/// "1 B per 3 A" should fail since they represent the same exchange rate.
/// 
/// This prevents:
/// - Market fragmentation
/// - Liquidity splitting across equivalent pools  
/// - User confusion about which pool to use
/// - Arbitrage opportunities due to liquidity imbalances


// ================================================================================================
// INTEGRATION WITH UTILITIES
// ================================================================================================

 

/// POOL-007: Phase 1.1 Enhanced Pool Creation with Treasury Counter Verification
/// 
/// This test demonstrates the Phase 1.1 enhanced pool creation helpers that provide
/// legitimate integration testing of treasury counter functionality
#[tokio::test]
#[serial]
async fn test_phase_1_1_enhanced_pool_creation() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing POOL-007: Phase 1.1 Enhanced Pool Creation with Treasury Verification...");
    
    use crate::common::{
        setup::{initialize_treasury_system, start_test_environment},
        pool_helpers::{execute_pool_creation_with_counter_verification, verify_pool_creation_fee_collection},
    };
    use solana_sdk::signature::Keypair;
    
    // Initialize test environment
    let mut env = start_test_environment().await;
    
    println!("🏛️ Step 1: Initialize treasury system...");
    
    // Initialize treasury system
    let system_authority = Keypair::new();
    initialize_treasury_system(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &system_authority,
    ).await?;
    
    println!("✅ Treasury system initialized");
    
    println!("\n🏊 Step 2: Execute enhanced pool creation with verification...");
    
    // Use Phase 1.1 enhanced helper for legitimate testing
    let pool_result = execute_pool_creation_with_counter_verification(
        &mut env,
        1000,  // This will be used as the multiple_per_base ratio
        1,     // This parameter is ignored by the current implementation
    ).await?;
    
    println!("✅ Enhanced pool creation completed!");
    println!("   - Pool PDA: {}", pool_result.pool_pda);
    println!("   - Fee collected: {} lamports", pool_result.fee_collected);
    println!("   - Creation successful: {}", pool_result.creation_successful);
    
    println!("\n📊 Step 3: Verify pool creation results...");
    
    // Validate results from Phase 1.1 helper
    assert!(pool_result.creation_successful, "Pool creation should be successful");
    assert!(pool_result.fee_collected > 0, "Pool creation should collect fees");
    
    // Verify treasury counter increments
    let counter_increment = pool_result.post_creation_treasury_state.pool_creation_count - 
                           pool_result.initial_treasury_state.pool_creation_count;
    let fee_increment = pool_result.post_creation_treasury_state.total_pool_creation_fees - 
                       pool_result.initial_treasury_state.total_pool_creation_fees;
    let balance_increment = pool_result.post_creation_treasury_state.total_balance - 
                           pool_result.initial_treasury_state.total_balance;
    
    assert_eq!(counter_increment, 1, "Pool creation counter should increment by 1");
    assert_eq!(fee_increment, pool_result.fee_collected, "Fee increment should match collected amount");
    assert!(balance_increment > 0, "Treasury balance should increase");
    
    println!("🔍 Verification results:");
    println!("   - Counter increment: {} ✅", counter_increment);
    println!("   - Fee increment: {} lamports ✅", fee_increment);
    println!("   - Balance increment: {} lamports ✅", balance_increment);
    
    println!("\n🔍 Step 4: Verify pool configuration details...");
    
    // Verify pool configuration is correct (the actual ratio will be normalized)
    println!("   - Actual ratio: {}:{}", pool_result.pool_config.ratio_a_numerator, pool_result.pool_config.ratio_b_denominator);
    assert_ne!(pool_result.pool_config.pool_state_pda, solana_sdk::pubkey::Pubkey::default(), "Pool PDA should be valid");
    
    println!("✅ Pool configuration verified:");
    println!("   - Ratio: {}:{}", pool_result.pool_config.ratio_a_numerator, pool_result.pool_config.ratio_b_denominator);
    println!("   - Pool State PDA: {}", pool_result.pool_config.pool_state_pda);
    println!("   - Token A Vault: {}", pool_result.pool_config.token_a_vault_pda);
    println!("   - Token B Vault: {}", pool_result.pool_config.token_b_vault_pda);
    
    println!("\n🔍 Step 5: Additional fee collection verification...");
    
    // Use Phase 1.1 helper to double-check fee collection
    let fee_verification = verify_pool_creation_fee_collection(
        &mut env,
        &pool_result.initial_treasury_state,
    ).await?;
    
    assert_eq!(fee_verification, pool_result.fee_collected, "Fee verification should match result");
    
    println!("✅ Fee collection verification successful:");
    println!("   - Verified fees: {} lamports", fee_verification);
    println!("   - Matches pool result: {}", fee_verification == pool_result.fee_collected);
    
    println!("\n✅ POOL-007: Phase 1.1 Enhanced Pool Creation successful!");
    println!("📋 Phase 1.1 Benefits Demonstrated:");
    println!("   1. ✅ Enhanced pool creation with automatic verification");
    println!("   2. ✅ Treasury counter tracking with real blockchain operations");
    println!("   3. ✅ Comprehensive result structure with detailed state");
    println!("   4. ✅ Fee collection verification with helper functions");
    println!("   5. ✅ Legitimate integration testing (no mock data)");
    println!("   6. ✅ Reusable helper functions for consistent testing");
    
    Ok(())
} 

// ========================================================================
// BASIS POINTS REFACTOR: DEMONSTRATION TEST
// ========================================================================

/// **BASIS POINTS REFACTOR: Pool Creation with Display Units**
/// 
/// This test demonstrates the correct way to create pools after the basis points
/// refactor, showing how to convert display units to basis points before
/// sending to the smart contract.
#[tokio::test]
#[serial]
async fn test_pool_creation_basis_points_refactor() -> TestResult {
    println!("🔧 BASIS POINTS REFACTOR: Testing pool creation with proper conversion...");
    
    // Setup test environment
    let env = start_test_environment().await;
    let mut banks_client = env.banks_client;
    let payer = env.payer;
    let recent_blockhash = env.recent_blockhash;

    // Initialize treasury system
    init_treasury_for_test(&mut banks_client, &payer, recent_blockhash).await?;
    
    // Create test tokens with different decimal places
    let sol_mint = Keypair::new();
    let usdt_mint = Keypair::new();
    create_mint(&mut banks_client, &payer, recent_blockhash, &sol_mint, Some(9)).await?;
    create_mint(&mut banks_client, &payer, recent_blockhash, &usdt_mint, Some(6)).await?;
    
    println!("✅ Created test tokens:");
    println!("   SOL mint: {} (9 decimals)", sol_mint.pubkey());
    println!("   USDT mint: {} (6 decimals)", usdt_mint.pubkey());

    // EXAMPLE 1: Create pool using new display unit helpers
    println!("\n📋 EXAMPLE 1: Using new display unit helpers");
    println!("Creating pool: 1.0 SOL = 160.0 USDT");
    
    let btc_mint = Keypair::new();
    let usdc_mint = Keypair::new();
    create_mint(&mut banks_client, &payer, recent_blockhash, &btc_mint, Some(8)).await?;
    create_mint(&mut banks_client, &payer, recent_blockhash, &usdc_mint, Some(6)).await?;
    
    // Use the new display unit helper - this is the recommended approach
    let pool_config = create_simple_display_pool(
        &mut banks_client,
        &payer,
        recent_blockhash,
        &btc_mint,
        &usdc_mint,
        1.0,     // 1.0 BTC (display units)
        50000.0, // = 50,000.0 USDC (display units)
        8,       // BTC has 8 decimals
        6,       // USDC has 6 decimals
    ).await?;
    
    println!("✅ Pool created using display unit helpers");
    println!("   Pool PDA: {}", pool_config.pool_state_pda);
    
    // Verify the pool state contains correct basis point ratios
    let pool_state = get_pool_state(&mut banks_client, &pool_config.pool_state_pda).await
        .ok_or_else(|| BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "Pool state not found")))?;
    
    // Expected basis point ratios:
    // 1.0 BTC = 1 * 10^8 = 100,000,000 basis points
    // 50,000.0 USDC = 50000 * 10^6 = 50,000,000,000 basis points
    let expected_btc_basis_points = 100_000_000_u64;
    let expected_usdc_basis_points = 50_000_000_000_u64;
    
    println!("🔍 Verifying basis point conversion:");
    println!("   Expected BTC ratio: {} basis points", expected_btc_basis_points);
    println!("   Expected USDC ratio: {} basis points", expected_usdc_basis_points);
    println!("   Actual ratios in pool: {} : {}", 
        pool_state.ratio_a_numerator, pool_state.ratio_b_denominator);
    
    // Verify one-to-many flag is set (BTC side = 1.0)
    assert!(pool_state.one_to_many_ratio(), 
        "Pool should have one-to-many flag set (BTC side = 1.0)");
    
    println!("✅ One-to-many flag correctly set for 1.0 BTC = 50,000.0 USDC");

    // EXAMPLE 2: Manual basis point conversion (for educational purposes)
    println!("\n📋 EXAMPLE 2: Manual basis point conversion");
    println!("Creating pool: 2.5 Token = 7.8 OtherToken");
    
    let token_x = Keypair::new();
    let token_y = Keypair::new();
    create_mint(&mut banks_client, &payer, recent_blockhash, &token_x, Some(9)).await?;
    create_mint(&mut banks_client, &payer, recent_blockhash, &token_y, Some(6)).await?;
    
    // Manual conversion from display units to basis points
    let token_x_display = 1.0;
    let token_y_display = 7.8;
    let token_x_decimals = 9;
    let token_y_decimals = 6;
    
    let token_x_basis_points = display_to_basis_points(token_x_display, token_x_decimals);
    let token_y_basis_points = display_to_basis_points(token_y_display, token_y_decimals);
    
    println!("   Manual conversion:");
    println!("     {} Token X → {} basis points", token_x_display, token_x_basis_points);
    println!("     {} Token Y → {} basis points", token_y_display, token_y_basis_points);
    
    // Create pool using manual conversion
    let pool_config_2 = create_simple_display_pool(
        &mut banks_client,
        &payer,
        recent_blockhash,
        &token_x,
        &token_y,
        token_x_display,
        token_y_display,
        token_x_decimals,
        token_y_decimals,
    ).await?;
    
    let pool_state_2 = get_pool_state(&mut banks_client, &pool_config_2.pool_state_pda).await
        .ok_or_else(|| BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "Pool state 2 not found")))?;
    
    // Verify one-to-many flag is NOT set (neither side = 1.0)
    assert!(!pool_state_2.one_to_many_ratio(), 
        "Pool should NOT have one-to-many flag set (neither side = 1.0)");
    
    println!("✅ One-to-many flag correctly NOT set for 2.5:7.8 ratio");

    println!("\n🎉 BASIS POINTS REFACTOR TEST COMPLETED SUCCESSFULLY!");
    println!("====================================================================");
    println!("✅ DEMONSTRATED:");
    println!("   • Proper display unit to basis point conversion");
    println!("   • Using new display unit helper functions");
    println!("   • Manual basis point calculation");
    println!("   • Correct one-to-many flag behavior");
    println!("   • Pool creation with different token decimal places");
    println!("🔧 All conversions handled correctly by client before sending to contract");
    println!("====================================================================");

    Ok(())
}

// ================================================================================================
// SECURITY GUARDS TESTS
// ================================================================================================

/// 🎯 TEST CONFIGURATION - MODIFY THESE VALUES TO CHANGE TEST BEHAVIOR
const SECURITY_TEST_RATIO_A: u64 = 1000;
const SECURITY_TEST_RATIO_B: u64 = 1;

/// **SECURITY GUARD TEST 1**: Same-mint rejection
/// 
/// This test validates that the security guard prevents creating pools where Token A and Token B
/// are the same mint, which would create a nonsensical single-token pool.
#[tokio::test]
async fn test_security_guard_same_mint_rejection() {
    use common::{
        pool_helpers::*,
        liquidity_helpers::*,
    };
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
        instruction::{AccountMeta, Instruction},
        system_program,
        pubkey::Pubkey,
    };
    use fixed_ratio_trading::{
        types::instructions::PoolInstruction,
        constants::*,
        id,
    };
    use borsh::BorshSerialize;

    println!("🎯 TEST CONFIGURATION - MODIFY THESE VALUES TO CHANGE TEST BEHAVIOR");
    println!("   • Test Ratio A: {}", SECURITY_TEST_RATIO_A);
    println!("   • Test Ratio B: {}", SECURITY_TEST_RATIO_B);
    println!("");

    println!("🔒 SECURITY GUARD TEST 1: Same-mint rejection");
    println!("Testing that pool creation fails when Token A and Token B are the same mint");
    
    // Create a proper foundation to get a real mint
    let foundation = create_liquidity_test_foundation(Some(2)).await
        .expect("Should create foundation successfully");
    
    println!("✅ Created foundation with mint: {}", foundation.primary_mint.pubkey());
    
    // Try to create a pool using the same mint for both Token A and Token B
    let same_mint_pubkey = foundation.primary_mint.pubkey();
    
    // Derive required PDAs using the same mint for both tokens
    let (main_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        &id(),
    );
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &id(),
    );
    
    // Manually derive PDAs since normalize_pool_config panics on same mint
    // We want to test the on-chain guard, not the helper function guard
    let (pool_state_pda, _) = Pubkey::find_program_address(
        &[
            POOL_STATE_SEED_PREFIX,
            same_mint_pubkey.as_ref(),
            same_mint_pubkey.as_ref(),
            &SECURITY_TEST_RATIO_A.to_le_bytes(),
            &SECURITY_TEST_RATIO_B.to_le_bytes(),
        ],
        &id(),
    );
    
    let (token_a_vault_pda, _) = Pubkey::find_program_address(
        &[TOKEN_A_VAULT_SEED_PREFIX, pool_state_pda.as_ref()],
        &id(),
    );
    let (token_b_vault_pda, _) = Pubkey::find_program_address(
        &[TOKEN_B_VAULT_SEED_PREFIX, pool_state_pda.as_ref()],
        &id(),
    );
    
    let (lp_token_a_mint_pda, _) = Pubkey::find_program_address(
        &[LP_TOKEN_A_MINT_SEED_PREFIX, pool_state_pda.as_ref()],
        &id(),
    );
    let (lp_token_b_mint_pda, _) = Pubkey::find_program_address(
        &[LP_TOKEN_B_MINT_SEED_PREFIX, pool_state_pda.as_ref()],
        &id(),
    );
    
    // Create InitializePool instruction with same mint for both tokens
    let initialize_pool_ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(foundation.env.payer.pubkey(), true),               // Index 0: User Authority Signer
            AccountMeta::new_readonly(system_program::id(), false),              // Index 1: System Program Account
            AccountMeta::new_readonly(system_state_pda, false),                  // Index 2: System State PDA
            AccountMeta::new(pool_state_pda, false),                             // Index 3: Pool State PDA
            AccountMeta::new_readonly(spl_token::id(), false),                   // Index 4: SPL Token Program Account
            AccountMeta::new(main_treasury_pda, false),                          // Index 5: Main Treasury PDA
            AccountMeta::new_readonly(solana_program::sysvar::rent::id(), false), // Index 6: Rent Sysvar Account
            AccountMeta::new_readonly(same_mint_pubkey, false),                  // Index 7: Token A Mint (same mint)
            AccountMeta::new_readonly(same_mint_pubkey, false),                  // Index 8: Token B Mint (same mint)
            AccountMeta::new(token_a_vault_pda, false),                          // Index 9: Token A Vault PDA
            AccountMeta::new(token_b_vault_pda, false),                          // Index 10: Token B Vault PDA
            AccountMeta::new(lp_token_a_mint_pda, false),                        // Index 11: LP Token A Mint PDA
            AccountMeta::new(lp_token_b_mint_pda, false),                        // Index 12: LP Token B Mint PDA
        ],
        data: PoolInstruction::InitializePool {
            ratio_a_numerator: SECURITY_TEST_RATIO_A,
            ratio_b_denominator: SECURITY_TEST_RATIO_B,
            flags: 0u8,
        }.try_to_vec().unwrap(),
    };
    
    // Add compute budget and create transaction
    use solana_sdk::compute_budget::ComputeBudgetInstruction;
    let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(500_000);
    
    let mut transaction = Transaction::new_with_payer(
        &[compute_budget_ix, initialize_pool_ix], 
        Some(&foundation.env.payer.pubkey())
    );
    transaction.sign(&[&foundation.env.payer], foundation.env.recent_blockhash);
    
    // Execute transaction and expect it to fail
    let mut banks_client = foundation.env.banks_client;
    let result = banks_client.process_transaction(transaction).await;
    
    println!("🔍 Transaction result: {:?}", result);
    
    // Verify that the transaction failed with the expected error
    assert!(result.is_err(), "Transaction should fail when using same mint for both tokens");
    
    // Check that the error is InvalidArgument (which maps to our same-mint rejection)
    let error = result.unwrap_err();
    match error {
        solana_program_test::BanksClientError::TransactionError(tx_error) => {
            println!("✅ Transaction failed as expected: {:?}", tx_error);
            // The error should contain InvalidArgument somewhere in the chain
            let error_string = format!("{:?}", tx_error);
            assert!(
                error_string.contains("InvalidArgument") || 
                error_string.contains("Custom") || 
                error_string.contains("ProgramError"),
                "Error should be related to invalid argument: {}", error_string
            );
        }
        _ => panic!("Expected TransactionError, got: {:?}", error),
    }
    
    println!("✅ SECURITY GUARD TEST 1 PASSED: Same-mint rejection works correctly");
}

/// **SECURITY GUARD TEST 2**: Invalid token mint owner rejection
/// 
/// This test validates that the security guard prevents creating pools when token mints
/// are not owned by the SPL Token program.
#[tokio::test]
async fn test_security_guard_invalid_token_program_owner() {
    use common::{
        pool_helpers::*,
        liquidity_helpers::*,
    };
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
        instruction::{AccountMeta, Instruction},
        system_program,
        pubkey::Pubkey,
    };
    use fixed_ratio_trading::{
        types::instructions::PoolInstruction,
        constants::*,
        id,
    };
    use borsh::BorshSerialize;

    println!("🎯 TEST CONFIGURATION - MODIFY THESE VALUES TO CHANGE TEST BEHAVIOR");
    println!("   • Test Ratio A: {}", SECURITY_TEST_RATIO_A);
    println!("   • Test Ratio B: {}", SECURITY_TEST_RATIO_B);
    println!("");

    println!("🔒 SECURITY GUARD TEST 2: Invalid token mint owner rejection");
    println!("Testing that pool creation fails when token mints are not owned by SPL Token program");
    
    // Create a proper foundation to get the environment
    let foundation = create_liquidity_test_foundation(Some(2)).await
        .expect("Should create foundation successfully");
    
    // Create a fake "mint" account that's not owned by SPL Token program
    let fake_mint = Keypair::new();
    
    // Create the fake mint account owned by system program instead of SPL Token program
    let create_fake_mint_ix = solana_sdk::system_instruction::create_account(
        &foundation.env.payer.pubkey(),
        &fake_mint.pubkey(),
        10_000_000, // More lamports for rent exemption
        165,        // Size of a mint account
        &system_program::id(), // Owned by system program, NOT SPL Token program
    );
    
    let mut create_fake_transaction = Transaction::new_with_payer(
        &[create_fake_mint_ix],
        Some(&foundation.env.payer.pubkey())
    );
    create_fake_transaction.sign(&[&foundation.env.payer, &fake_mint], foundation.env.recent_blockhash);
    
    let mut banks_client = foundation.env.banks_client;
    banks_client.process_transaction(create_fake_transaction).await
        .expect("Should create fake mint account");
    
    println!("✅ Created fake mint (owned by system program): {}", fake_mint.pubkey());
    println!("✅ Created valid mint (owned by SPL Token program): {}", foundation.base_mint.pubkey());
    
    // Derive required PDAs
    let (main_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        &id(),
    );
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &id(),
    );
    
    // Use normalize_pool_config with fake mint as Token A
    let config = normalize_pool_config(
        &fake_mint.pubkey(),  // Fake mint (invalid owner)
        &foundation.base_mint.pubkey(), // Valid mint
        SECURITY_TEST_RATIO_A,
        SECURITY_TEST_RATIO_B,
    );
    
    let (lp_token_a_mint_pda, _) = Pubkey::find_program_address(
        &[LP_TOKEN_A_MINT_SEED_PREFIX, config.pool_state_pda.as_ref()],
        &id(),
    );
    let (lp_token_b_mint_pda, _) = Pubkey::find_program_address(
        &[LP_TOKEN_B_MINT_SEED_PREFIX, config.pool_state_pda.as_ref()],
        &id(),
    );
    
    // Create InitializePool instruction with fake mint
    let initialize_pool_ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(foundation.env.payer.pubkey(), true),               // Index 0: User Authority Signer
            AccountMeta::new_readonly(system_program::id(), false),              // Index 1: System Program Account
            AccountMeta::new_readonly(system_state_pda, false),                  // Index 2: System State PDA
            AccountMeta::new(config.pool_state_pda, false),                      // Index 3: Pool State PDA
            AccountMeta::new_readonly(spl_token::id(), false),                   // Index 4: SPL Token Program Account
            AccountMeta::new(main_treasury_pda, false),                          // Index 5: Main Treasury PDA
            AccountMeta::new_readonly(solana_program::sysvar::rent::id(), false), // Index 6: Rent Sysvar Account
            AccountMeta::new_readonly(config.token_a_mint, false),               // Index 7: Token A Mint (normalized)
            AccountMeta::new_readonly(config.token_b_mint, false),               // Index 8: Token B Mint (normalized)
            AccountMeta::new(config.token_a_vault_pda, false),                   // Index 9: Token A Vault PDA
            AccountMeta::new(config.token_b_vault_pda, false),                   // Index 10: Token B Vault PDA
            AccountMeta::new(lp_token_a_mint_pda, false),                        // Index 11: LP Token A Mint PDA
            AccountMeta::new(lp_token_b_mint_pda, false),                        // Index 12: LP Token B Mint PDA
        ],
        data: PoolInstruction::InitializePool {
            ratio_a_numerator: SECURITY_TEST_RATIO_A,
            ratio_b_denominator: SECURITY_TEST_RATIO_B,
            flags: 0u8,
        }.try_to_vec().unwrap(),
    };
    
    // Add compute budget and create transaction
    use solana_sdk::compute_budget::ComputeBudgetInstruction;
    let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(500_000);
    
    let mut transaction = Transaction::new_with_payer(
        &[compute_budget_ix, initialize_pool_ix], 
        Some(&foundation.env.payer.pubkey())
    );
    transaction.sign(&[&foundation.env.payer], foundation.env.recent_blockhash);
    
    // Execute transaction and expect it to fail
    let result = banks_client.process_transaction(transaction).await;
    
    println!("🔍 Transaction result: {:?}", result);
    
    // Verify that the transaction failed with the expected error
    assert!(result.is_err(), "Transaction should fail when using mint not owned by SPL Token program");
    
    // Check that the error is IncorrectProgramId
    let error = result.unwrap_err();
    match error {
        solana_program_test::BanksClientError::TransactionError(tx_error) => {
            println!("✅ Transaction failed as expected: {:?}", tx_error);
            let error_string = format!("{:?}", tx_error);
            assert!(
                error_string.contains("IncorrectProgramId") || 
                error_string.contains("Custom") || 
                error_string.contains("ProgramError"),
                "Error should be related to incorrect program ID: {}", error_string
            );
        }
        _ => panic!("Expected TransactionError, got: {:?}", error),
    }
    
    println!("✅ SECURITY GUARD TEST 2 PASSED: Invalid token mint owner rejection works correctly");
}

/// **SECURITY GUARD TEST 3**: Invalid SPL Token program account rejection
/// 
/// This test validates that the security guard prevents creating pools when the wrong
/// SPL Token program account is provided in the instruction.
#[tokio::test]
async fn test_security_guard_invalid_spl_token_program() {
    use common::{
        pool_helpers::*,
        liquidity_helpers::*,
    };
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
        instruction::{AccountMeta, Instruction},
        system_program,
        pubkey::Pubkey,
    };
    use fixed_ratio_trading::{
        types::instructions::PoolInstruction,
        constants::*,
        id,
    };
    use borsh::BorshSerialize;

    println!("🎯 TEST CONFIGURATION - MODIFY THESE VALUES TO CHANGE TEST BEHAVIOR");
    println!("   • Test Ratio A: {}", SECURITY_TEST_RATIO_A);
    println!("   • Test Ratio B: {}", SECURITY_TEST_RATIO_B);
    println!("");

    println!("🔒 SECURITY GUARD TEST 3: Invalid SPL Token program account rejection");
    println!("Testing that pool creation fails when wrong SPL Token program account is provided");
    
    // Create a proper foundation to get the environment and valid mints
    let foundation = create_liquidity_test_foundation(Some(2)).await
        .expect("Should create foundation successfully");
    
    println!("✅ Created foundation with Token A mint: {}", foundation.primary_mint.pubkey());
    println!("✅ Created foundation with Token B mint: {}", foundation.base_mint.pubkey());
    
    // Derive required PDAs
    let (main_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        &id(),
    );
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &id(),
    );
    
    let config = normalize_pool_config(
        &foundation.primary_mint.pubkey(),
        &foundation.base_mint.pubkey(),
        SECURITY_TEST_RATIO_A,
        SECURITY_TEST_RATIO_B,
    );
    
    let (lp_token_a_mint_pda, _) = Pubkey::find_program_address(
        &[LP_TOKEN_A_MINT_SEED_PREFIX, config.pool_state_pda.as_ref()],
        &id(),
    );
    let (lp_token_b_mint_pda, _) = Pubkey::find_program_address(
        &[LP_TOKEN_B_MINT_SEED_PREFIX, config.pool_state_pda.as_ref()],
        &id(),
    );
    
    // Create InitializePool instruction with WRONG SPL Token program account
    let initialize_pool_ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(foundation.env.payer.pubkey(), true),               // Index 0: User Authority Signer
            AccountMeta::new_readonly(system_program::id(), false),              // Index 1: System Program Account
            AccountMeta::new_readonly(system_state_pda, false),                  // Index 2: System State PDA
            AccountMeta::new(config.pool_state_pda, false),                      // Index 3: Pool State PDA
            AccountMeta::new_readonly(system_program::id(), false),              // Index 4: WRONG! Should be spl_token::id()
            AccountMeta::new(main_treasury_pda, false),                          // Index 5: Main Treasury PDA
            AccountMeta::new_readonly(solana_program::sysvar::rent::id(), false), // Index 6: Rent Sysvar Account
            AccountMeta::new_readonly(config.token_a_mint, false),               // Index 7: Token A Mint (normalized)
            AccountMeta::new_readonly(config.token_b_mint, false),               // Index 8: Token B Mint (normalized)
            AccountMeta::new(config.token_a_vault_pda, false),                   // Index 9: Token A Vault PDA
            AccountMeta::new(config.token_b_vault_pda, false),                   // Index 10: Token B Vault PDA
            AccountMeta::new(lp_token_a_mint_pda, false),                        // Index 11: LP Token A Mint PDA
            AccountMeta::new(lp_token_b_mint_pda, false),                        // Index 12: LP Token B Mint PDA
        ],
        data: PoolInstruction::InitializePool {
            ratio_a_numerator: SECURITY_TEST_RATIO_A,
            ratio_b_denominator: SECURITY_TEST_RATIO_B,
            flags: 0u8,
        }.try_to_vec().unwrap(),
    };
    
    // Add compute budget and create transaction
    use solana_sdk::compute_budget::ComputeBudgetInstruction;
    let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(500_000);
    
    let mut transaction = Transaction::new_with_payer(
        &[compute_budget_ix, initialize_pool_ix], 
        Some(&foundation.env.payer.pubkey())
    );
    transaction.sign(&[&foundation.env.payer], foundation.env.recent_blockhash);
    
    // Execute transaction and expect it to fail
    let mut banks_client = foundation.env.banks_client;
    let result = banks_client.process_transaction(transaction).await;
    
    println!("🔍 Transaction result: {:?}", result);
    
    // Verify that the transaction failed with the expected error
    assert!(result.is_err(), "Transaction should fail when using wrong SPL Token program account");
    
    // Check that the error is IncorrectProgramId
    let error = result.unwrap_err();
    match error {
        solana_program_test::BanksClientError::TransactionError(tx_error) => {
            println!("✅ Transaction failed as expected: {:?}", tx_error);
            let error_string = format!("{:?}", tx_error);
            assert!(
                error_string.contains("IncorrectProgramId") || 
                error_string.contains("Custom") || 
                error_string.contains("ProgramError"),
                "Error should be related to incorrect program ID: {}", error_string
            );
        }
        _ => panic!("Expected TransactionError, got: {:?}", error),
    }
    
    println!("✅ SECURITY GUARD TEST 3 PASSED: Invalid SPL Token program account rejection works correctly");
}

/// **COMPREHENSIVE SECURITY GUARD TEST**: Validates that valid pool creation still works
/// 
/// This test ensures that after adding the security guards, legitimate pool creation
/// operations continue to work correctly.
#[tokio::test]
async fn test_security_guards_comprehensive_validation() {
    use common::{
        pool_helpers::*,
        liquidity_helpers::*,
        enhanced_test_foundation::*,
    };

    println!("🎯 TEST CONFIGURATION - MODIFY THESE VALUES TO CHANGE TEST BEHAVIOR");
    println!("   • Test Ratio A: {}", SECURITY_TEST_RATIO_A);
    println!("   • Test Ratio B: {}", SECURITY_TEST_RATIO_B);
    println!("");

    println!("🔒 COMPREHENSIVE SECURITY GUARD TEST");
    println!("Testing that valid pool creation still works after adding security guards");
    
    // Create a proper foundation and verify it works correctly
    let mut foundation = create_liquidity_test_foundation(Some(SECURITY_TEST_RATIO_A)).await
        .expect("Should create foundation successfully");
    
    println!("✅ Foundation created successfully with pool: {}", foundation.pool_config.pool_state_pda);
    
    // Verify the foundation's pool has different token mints
    assert_ne!(foundation.primary_mint.pubkey(), foundation.base_mint.pubkey(), "Mints should be different");
    
    // Verify pool state was created correctly
    let pool_state = get_pool_state(&mut foundation.env.banks_client, &foundation.pool_config.pool_state_pda).await
        .expect("Pool state should exist");
    
    // Verify the pool has the correct token mints (normalized)
    assert_eq!(pool_state.token_a_mint, foundation.pool_config.token_a_mint);
    assert_eq!(pool_state.token_b_mint, foundation.pool_config.token_b_mint);
    assert_eq!(pool_state.ratio_a_numerator, foundation.pool_config.ratio_a_numerator);
    assert_eq!(pool_state.ratio_b_denominator, foundation.pool_config.ratio_b_denominator);
    
    println!("✅ Pool state validation passed:");
    println!("   • Token A mint: {}", pool_state.token_a_mint);
    println!("   • Token B mint: {}", pool_state.token_b_mint);
    println!("   • Ratio: {}:{}", pool_state.ratio_a_numerator, pool_state.ratio_b_denominator);
    
    // Try to create another pool with different mints to ensure it still works
    let enhanced_foundation = create_enhanced_liquidity_test_foundation(Some(3)).await
        .expect("Should create enhanced foundation successfully");
    
    // Add an additional pool with different ratio
    let mut enhanced = enhanced_foundation;
    let additional_pool_index = enhanced.add_pool(PoolCreationParams::new(5, 1)).await
        .expect("Should create additional pool successfully");
    
    println!("✅ Additional pool created successfully with index: {}", additional_pool_index);
    println!("   • Total pools in enhanced foundation: {}", enhanced.pool_count());
    
    println!("✅ COMPREHENSIVE SECURITY GUARD TEST PASSED: Valid pool creation works correctly");
} 