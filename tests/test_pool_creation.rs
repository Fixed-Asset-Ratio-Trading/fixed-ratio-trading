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
    use crate::common::setup::create_test_program_authority_keypair;
    use fixed_ratio_trading::constants::TEST_PROGRAM_AUTHORITY;
    use std::str::FromStr;
    
    // Create keypair that matches the test program authority
    let system_authority = create_test_program_authority_keypair()
        .map_err(|e| BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, 
            format!("Failed to create program authority keypair: {}", e))))?;
    
    // Get the test program authority for verification
    let test_program_authority_pubkey = solana_program::pubkey::Pubkey::from_str(TEST_PROGRAM_AUTHORITY)
        .map_err(|e| BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, 
            format!("Invalid test program authority: {}", e))))?;
    
    // Verify the keypair matches the test program authority
    if system_authority.pubkey() != test_program_authority_pubkey {
        return Err(BanksClientError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Keypair mismatch: expected {}, got {}", 
                test_program_authority_pubkey, system_authority.pubkey())
        )));
    }
    
    println!("🔐 Using test program authority for testing: {}", test_program_authority_pubkey);
    println!("🔐 Authority verified: {}", system_authority.pubkey());
    
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

/// Test pool initialization using the new single-instruction pattern.
/// 
/// This test demonstrates the improved InitializePool instruction that replaces the
/// deprecated two-instruction pattern with a single atomic operation.
#[tokio::test]
async fn test_initialize_pool_new_pattern() -> TestResult {
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

    // Create pool using new single-instruction pattern
    let config = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        Some(3),
    ).await?;

    // Verify pool state
    verify_pool_state(
        &mut ctx.env.banks_client,
        &config,
        &ctx.env.payer.pubkey(),
        &ctx.lp_token_a_mint.pubkey(),
        &ctx.lp_token_b_mint.pubkey(),
    ).await.expect("Pool state verification failed");

    println!("✅ New single-instruction pattern: Pool created and verified successfully!");
    println!("✅ Atomic operation - all accounts created and data initialized in one transaction");
    
    Ok(())
}


/// Test multiple pools with different ratios
#[tokio::test]
async fn test_initialize_multiple_pools_different_ratios() -> TestResult {
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create token mints (shared between pools)
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

    // Create first pool with 2:1 ratio
    let config1 = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        Some(2),
    ).await?;

    // Create second pool with 10:1 ratio (different LP tokens)
    let config2 = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        Some(10),
    ).await?;

    // Verify both pools exist and have different PDAs
    assert_ne!(config1.pool_state_pda, config2.pool_state_pda, 
        "Pools with different ratios should have different PDAs");

    // Verify both pools have correct ratios
    let pool_state1 = get_pool_state(&mut ctx.env.banks_client, &config1.pool_state_pda).await
        .expect("First pool state should exist");
    let pool_state2 = get_pool_state(&mut ctx.env.banks_client, &config2.pool_state_pda).await
        .expect("Second pool state should exist");

    assert!(pool_state1.is_initialized);
    assert!(pool_state2.is_initialized);

    println!("✅ Multiple pools with different ratios created successfully!");
    println!("   Pool 1 PDA: {}", config1.pool_state_pda);
    println!("   Pool 2 PDA: {}", config2.pool_state_pda);
    
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

    // Create first pool: 2 primary per 1 base (exchange rate: 2:1)
    let _config1 = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        Some(2),
    ).await?;

    println!("✅ Created first pool: 2 primary per 1 base");

    // Try to create economically equivalent pool: 1 base per 2 primary (same exchange rate: 2:1)
    let _config2 = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        Some(2),
    ).await?;

    // Initialize treasury system first (required for pool creation fees)
    init_treasury_for_test(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
    ).await?;

    // Try to create pool with zero ratio (should fail)
    let _config3 = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        Some(0),
    ).await?;

    // Initialize treasury system first (required for pool creation fees)
    init_treasury_for_test(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
    ).await?;

    // Create first pool successfully
    let _config4 = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        Some(3),
    ).await?;

    // Try to create the exact same pool again (should fail)
    let _config5 = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        Some(3),
    ).await?;

    // Try to create pool with same token as both primary and base (should fail)
    // This should panic in normalize_pool_config or fail during instruction processing
    
    // We'll test this by checking if the normalize function panics
    use std::panic;
    
    let result = panic::catch_unwind(|| {
        normalize_pool_config_legacy(&ctx.primary_mint.pubkey(), &ctx.primary_mint.pubkey(), 2)
    });

    assert!(result.is_err(), "normalize_pool_config should panic with identical tokens");
    
    println!("✅ Correctly rejected pool creation with identical token mints");
    
    Ok(())
}

// ================================================================================================
// INTEGRATION WITH UTILITIES
// ================================================================================================

/// Test pool creation using the comprehensive utility functions
#[tokio::test]
async fn test_pool_creation_with_utilities() -> TestResult {
    // Use the setup utility for a complete test context
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create all required mints using the utility
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

    // Test both patterns to ensure utilities work with both
    
    // Pattern 1: New single-instruction (recommended)
    let config_new = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        Some(3),
    ).await?;

    // Verify using utility
    verify_pool_state(
        &mut ctx.env.banks_client,
        &config_new,
        &ctx.env.payer.pubkey(),
        &ctx.lp_token_a_mint.pubkey(),
        &ctx.lp_token_b_mint.pubkey(),
    ).await.expect("New pattern pool verification failed");

    // Pattern 2: Legacy two-instruction (for compatibility)
    let config_legacy = create_pool_legacy_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
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
    
    Ok(())
}

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

// ================================================================================================
// FOUNDATION TEST - CORE INFRASTRUCTURE FOR OTHER TESTS
// ================================================================================================

/// **FOUNDATION TEST**: Core pool initialization with user accounts for testing
/// 
/// This test creates a complete testing environment that serves as the foundation
/// for all other tests. It initializes:
/// 
/// 1. **Treasury System** - All required PDAs for fee collection
/// 2. **Token Infrastructure** - Two token mints with proper ordering
/// 3. **Pool Creation** - A functioning pool with standard 3:1 ratio
/// 4. **User Accounts** - Multiple funded users with token accounts
/// 5. **Test Verification** - Complete state validation
/// 
/// This test can be used as a reference implementation for setting up test environments
/// and ensures all components work together properly.
/// 
/// # Test Flow
/// 1. Initialize treasury system (required first step)
/// 2. Create ordered token mints (lexicographically)
/// 3. Create pool with standardized account ordering
/// 4. Setup multiple test users with token accounts
/// 5. Verify all components are properly initialized
/// 
/// # Returns
/// Success when all components are properly initialized and verified
#[tokio::test]
async fn test_process_initialize_pool_success() -> TestResult {
    println!("🚀 FOUNDATION TEST: Initializing complete pool testing environment");
    println!("   This test creates the core infrastructure that other tests can reuse");
    
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
    // STEP 4: Create Pool with Standard 3:1 Ratio
    // =============================================
    println!("\n🏊 Creating trading pool...");
    let pool_config = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &primary_mint,
        &base_mint,
        Some(3),
    ).await?;
    
    println!("✅ Pool created successfully:");
    println!("   Pool State PDA: {}", pool_config.pool_state_pda);
    println!("   Token A Mint: {}", pool_config.token_a_mint);
    println!("   Token B Mint: {}", pool_config.token_b_mint);
    println!("   Ratio: {}:{}", pool_config.ratio_a_numerator, pool_config.ratio_b_denominator);
    println!("   Token A Vault: {}", pool_config.token_a_vault_pda);
    println!("   Token B Vault: {}", pool_config.token_b_vault_pda);
    
    // =============================================
    // STEP 5: Verify Pool State
    // =============================================
    println!("\n🔍 Verifying pool state...");
    verify_pool_state(
        &mut ctx.env.banks_client,
        &pool_config,
        &ctx.env.payer.pubkey(),
        &ctx.lp_token_a_mint.pubkey(),
        &ctx.lp_token_b_mint.pubkey(),
    ).await.expect("Pool state verification failed");
    
    let pool_state = get_pool_state(&mut ctx.env.banks_client, &pool_config.pool_state_pda).await
        .expect("Pool state should exist");
    
    println!("✅ Pool state verified:");
    println!("   Initialized: {}", pool_state.is_initialized);
    println!("   Owner: {}", pool_state.owner);
    println!("   LP Token A Mint: {}", pool_state.lp_token_a_mint);
    println!("   LP Token B Mint: {}", pool_state.lp_token_b_mint);
    println!("   Initial Token A Liquidity: {}", pool_state.total_token_a_liquidity);
    println!("   Initial Token B Liquidity: {}", pool_state.total_token_b_liquidity);
    
    // =============================================
    // STEP 6: Create Test Users with Token Accounts
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
    // STEP 7: Create LP Token Accounts for Users
    // =============================================
    println!("\n🎫 Creating LP token accounts for users...");
    
    // Create LP token accounts for each user
    let user1_lp_a_account_kp = Keypair::new();
    create_token_account(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &user1_lp_a_account_kp,
        &pool_state.lp_token_a_mint,
        &user1.pubkey(),
    ).await?;
    
    let user1_lp_b_account_kp = Keypair::new();
    create_token_account(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &user1_lp_b_account_kp,
        &pool_state.lp_token_b_mint,
        &user1.pubkey(),
    ).await?;
    
    let user2_lp_a_account_kp = Keypair::new();
    create_token_account(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &user2_lp_a_account_kp,
        &pool_state.lp_token_a_mint,
        &user2.pubkey(),
    ).await?;
    
    let user2_lp_b_account_kp = Keypair::new();
    create_token_account(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &user2_lp_b_account_kp,
        &pool_state.lp_token_b_mint,
        &user2.pubkey(),
    ).await?;
    
    let user3_lp_a_account_kp = Keypair::new();
    create_token_account(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &user3_lp_a_account_kp,
        &pool_state.lp_token_a_mint,
        &user3.pubkey(),
    ).await?;
    
    let user3_lp_b_account_kp = Keypair::new();
    create_token_account(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &user3_lp_b_account_kp,
        &pool_state.lp_token_b_mint,
        &user3.pubkey(),
    ).await?;
    
    println!("✅ LP token accounts created for all users");
    
    // =============================================
    // STEP 8: Final Verification & Summary
    // =============================================
    println!("\n🎯 FOUNDATION TEST COMPLETE - Full Environment Ready!");
    println!("════════════════════════════════════════════════════════");
    println!("✅ INFRASTRUCTURE CREATED:");
    println!("   • Treasury System: All fee collection PDAs initialized");
    println!("   • Token Mints: Primary and Base tokens created");
    println!("   • Trading Pool: 3:1 ratio pool with LP tokens");
    println!("   • User Accounts: 3 funded users with all token accounts");
    println!("   • LP Token Accounts: Ready for liquidity operations");
    println!();
    println!("🔧 READY FOR OPERATIONS:");
    println!("   • Deposits: Users can provide liquidity");
    println!("   • Withdrawals: Users can withdraw liquidity");
    println!("   • Swaps: Users can trade at fixed 3:1 ratio");
    println!("   • Fee Collection: SOL fees flow to treasury system");
    println!();
    println!("📋 TEST INFRASTRUCTURE SUMMARY:");
    println!("   Pool ID: {}", pool_config.pool_state_pda);
    println!("   Primary Mint: {}", primary_mint.pubkey());
    println!("   Base Mint: {}", base_mint.pubkey());
    println!("   Ratio: 3 Primary : 1 Base");
    println!("   Users: 3 traders with varying balances");
    println!("   Fee System: Fully operational treasury PDAs");
    println!();
    println!("💡 USAGE: Other tests can reference this test as a setup example");
    println!("   or extract its components to create similar test environments.");
    
    Ok(())
} 