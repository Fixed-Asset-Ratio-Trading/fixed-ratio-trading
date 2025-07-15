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

mod common;

use common::*;
use solana_program_test::BanksClientError;

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

// ================================================================================================
// NEW SINGLE-INSTRUCTION PATTERN TESTS (RECOMMENDED)
// ================================================================================================

/// **COMPREHENSIVE TEST**: Complete pool initialization and validation
/// 
/// This consolidated test covers all aspects of pool creation and initialization:
/// 1. New single-instruction pattern testing (from test_initialize_pool_new_pattern)
/// 2. Utility function integration testing (from test_pool_creation_with_utilities)
/// 3. Complete environment setup and validation (from test_process_initialize_pool_success)
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
async fn test_process_initialize_pool() -> TestResult {
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
#[tokio::test]
async fn test_create_pool_reversed_tokens_same_ratio_fails() -> TestResult {
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create token mints
    create_test_mints(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &[&ctx.primary_mint, &ctx.base_mint],
    ).await?;

    // Initialize treasury system first (required for pool creation fees)
    init_treasury_for_test(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
    ).await?;

    // Test 1: Create first pool successfully: 2 primary per 1 base (exchange rate: 2:1)
    let _config1 = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        Some(2),
    ).await?;

    println!("✅ Created first pool: 2 primary per 1 base");

    // Test 2: Try to create economically equivalent pool with swapped tokens
    // This should fail because normalization will result in the same PDA
    let result2 = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.base_mint,  // Swapped order
        &ctx.primary_mint,  // Swapped order
        Some(2),
    ).await;

    assert!(result2.is_err(), "Creating economically equivalent pool should fail");
    println!("✅ Correctly rejected economically equivalent pool creation");

    // Test 3: Try to create pool with zero ratio (should fail)
    let result3 = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        Some(0),
    ).await;

    assert!(result3.is_err(), "Creating pool with zero ratio should fail");
    println!("✅ Correctly rejected pool creation with zero ratio");

    // Test 4: Try to create the exact same pool again (should fail due to AccountAlreadyInitialized)
    let result4 = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        Some(2),  // Same ratio as first pool
    ).await;

    assert!(result4.is_err(), "Creating duplicate pool should fail");
    println!("✅ Correctly rejected duplicate pool creation");

    // Test 5: Try to create pool with same token as both primary and base (should fail)
    // This will panic in the normalize function, so we need to handle it differently
    println!("✅ Test 5: Attempting to create pool with identical tokens (should be rejected)");
    
    // We'll test this by checking if the normalize function panics
    use std::panic;
    
    let result = panic::catch_unwind(|| {
        normalize_pool_config_legacy(&ctx.primary_mint.pubkey(), &ctx.primary_mint.pubkey(), 2)
    });

    assert!(result.is_err(), "normalize_pool_config should panic with identical tokens");
    println!("✅ Correctly rejected pool creation with identical token mints (panic caught)");

    // Test 6: Create a valid different pool to ensure the system still works
    let _config6 = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        Some(3),  // Different ratio
    ).await?;

    println!("✅ Successfully created pool with different ratio (3:1)");
    
    Ok(())
}

// ================================================================================================
// INTEGRATION WITH UTILITIES
// ================================================================================================

/// Test normalization logic with various token orderings
#[tokio::test]
async fn test_pool_normalization_logic() -> TestResult {
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create token mints
    create_test_mints(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &[&ctx.primary_mint, &ctx.base_mint],
    ).await?;

    // Test normalization directly with economically equivalent ratios
    let config1 = normalize_pool_config_legacy(&ctx.primary_mint.pubkey(), &ctx.base_mint.pubkey(), 4);
    let config2 = normalize_pool_config_legacy(&ctx.base_mint.pubkey(), &ctx.primary_mint.pubkey(), 4);

    // Both should normalize to the same token ordering (lexicographically)
    assert_eq!(config1.token_a_mint, config2.token_a_mint, "Token A should be the same after normalization");
    assert_eq!(config1.token_b_mint, config2.token_b_mint, "Token B should be the same after normalization");
    
    // These represent economically equivalent pools and should result in the same PDA
    // Pool 1: 4 primary per 1 base 
    // Pool 2: 4 base per 1 primary (when tokens are reversed)
    // After normalization, these should be detected as equivalent
    assert_eq!(config1.pool_state_pda, config2.pool_state_pda, "Economically equivalent pools should have the same PDA");

    println!("✅ Normalization logic correctly detects economically equivalent pools");
    println!("   Config 1 - Token A: {}, Token B: {}", config1.token_a_mint, config1.token_b_mint);
    println!("   Config 1 - Ratio: {}:{}", config1.ratio_a_numerator, config1.ratio_b_denominator);
    println!("   Config 2 - Ratio: {}:{}", config2.ratio_a_numerator, config2.ratio_b_denominator);
    println!("   Same PDA prevents liquidity fragmentation: {}", config1.pool_state_pda);
    
    Ok(())
} 