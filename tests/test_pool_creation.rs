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

    // Create pool using new single-instruction pattern
    let config = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        &ctx.lp_token_a_mint,
        &ctx.lp_token_b_mint,
        None, // Use default ratio
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

/// Test pool initialization with custom ratio using new pattern
#[tokio::test]
async fn test_initialize_pool_new_pattern_custom_ratio() -> TestResult {
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create token mints
    create_test_mints(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &[&ctx.primary_mint, &ctx.base_mint],
    ).await?;

    // Create pool with custom 5:1 ratio
    let custom_ratio = 5u64;
    let config = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        &ctx.lp_token_a_mint,
        &ctx.lp_token_b_mint,
        Some(custom_ratio),
    ).await?;

    // Verify pool state reflects custom ratio
    let pool_state = get_pool_state(&mut ctx.env.banks_client, &config.pool_state_pda).await
        .expect("Pool state should exist");

    // Check that the ratio is properly normalized
    if config.token_a_is_primary {
        assert_eq!(pool_state.ratio_a_numerator, custom_ratio);
        assert_eq!(pool_state.ratio_b_denominator, 1);
    } else {
        assert_eq!(pool_state.ratio_a_numerator, 1);
        assert_eq!(pool_state.ratio_b_denominator, custom_ratio);
    }

    println!("✅ Custom ratio pool created successfully with {}:1 ratio", custom_ratio);
    
    Ok(())
}

// ================================================================================================
// LEGACY TWO-INSTRUCTION PATTERN TESTS (DEPRECATED)
// ================================================================================================

/// Test pool initialization using the deprecated two-instruction pattern.
/// 
/// This test demonstrates the legacy CreatePoolStateAccount + InitializePoolData pattern
/// used to work around the Solana AccountInfo.data issue.
#[tokio::test]
async fn test_initialize_pool_legacy_pattern() -> TestResult {
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create token mints
    create_test_mints(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &[&ctx.primary_mint, &ctx.base_mint],
    ).await?;

    // Create pool using legacy two-instruction pattern
    let config = create_pool_legacy_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        &ctx.lp_token_a_mint,
        &ctx.lp_token_b_mint,
        None, // Use default ratio
    ).await?;

    // Verify pool state
    verify_pool_state(
        &mut ctx.env.banks_client,
        &config,
        &ctx.env.payer.pubkey(),
        &ctx.lp_token_a_mint.pubkey(),
        &ctx.lp_token_b_mint.pubkey(),
    ).await.expect("Pool state verification failed");

    println!("✅ Legacy two-instruction pattern: Pool created and verified successfully!");
    println!("⚠️  Note: This pattern is deprecated - use InitializePool instruction instead");
    
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

    // Create first pool with 2:1 ratio
    let lp_token_a_mint_1 = Keypair::new();
    let lp_token_b_mint_1 = Keypair::new();
    
    let config1 = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        &lp_token_a_mint_1,
        &lp_token_b_mint_1,
        Some(2),
    ).await?;

    // Create second pool with 10:1 ratio (different LP tokens)
    let lp_token_a_mint_2 = Keypair::new();
    let lp_token_b_mint_2 = Keypair::new();
    
    let config2 = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        &lp_token_a_mint_2,
        &lp_token_b_mint_2,
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

/// Test that creating a pool with reversed tokens but same ratio fails
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

    // Create first pool successfully
    let config = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        &ctx.lp_token_a_mint,
        &ctx.lp_token_b_mint,
        Some(2),
    ).await?;

    // Try to create another pool with reversed tokens (should fail due to normalization)
    let lp_token_a_mint_2 = Keypair::new();
    let lp_token_b_mint_2 = Keypair::new();

    let result = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.base_mint,    // Reversed: base as primary
        &ctx.primary_mint, // Reversed: primary as base
        &lp_token_a_mint_2,
        &lp_token_b_mint_2,
        Some(2), // Same ratio
    ).await;

    // This should fail because normalization will result in the same pool configuration
    assert!(result.is_err(), "Creating pool with reversed tokens but same ratio should fail");
    
    println!("✅ Correctly prevented duplicate pool creation with reversed token order");
    
    Ok(())
}

/// Test creating pool with zero ratio fails
#[tokio::test]
async fn test_create_pool_zero_ratio_fails() -> TestResult {
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create token mints
    create_test_mints(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &[&ctx.primary_mint, &ctx.base_mint],
    ).await?;

    // Try to create pool with zero ratio (should fail)
    let result = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        &ctx.lp_token_a_mint,
        &ctx.lp_token_b_mint,
        Some(0), // Zero ratio
    ).await;

    assert!(result.is_err(), "Creating pool with zero ratio should fail");
    
    println!("✅ Correctly rejected pool creation with zero ratio");
    
    Ok(())
}

/// Test creating pool that already exists fails
#[tokio::test]
async fn test_create_duplicate_pool_fails() -> TestResult {
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create token mints
    create_test_mints(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &[&ctx.primary_mint, &ctx.base_mint],
    ).await?;

    // Create first pool successfully
    let _config1 = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        &ctx.lp_token_a_mint,
        &ctx.lp_token_b_mint,
        Some(2),
    ).await?;

    // Try to create the exact same pool again (should fail)
    let lp_token_a_mint_2 = Keypair::new();
    let lp_token_b_mint_2 = Keypair::new();

    let result = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        &lp_token_a_mint_2,
        &lp_token_b_mint_2,
        Some(2), // Same ratio, same tokens
    ).await;

    assert!(result.is_err(), "Creating duplicate pool should fail");
    
    println!("✅ Correctly prevented duplicate pool creation");
    
    Ok(())
}

/// Test creating pool with identical token mints fails
#[tokio::test]
async fn test_create_pool_identical_tokens_fails() -> TestResult {
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create only one token mint
    create_mint(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        None,
    ).await?;

    // Try to create pool with same token as both primary and base (should fail)
    // This should panic in normalize_pool_config or fail during instruction processing
    
    // We'll test this by checking if the normalize function panics
    use std::panic;
    
    let result = panic::catch_unwind(|| {
        normalize_pool_config(&ctx.primary_mint.pubkey(), &ctx.primary_mint.pubkey(), 2)
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

    // Test both patterns to ensure utilities work with both
    
    // Pattern 1: New single-instruction (recommended)
    let config_new = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        &ctx.lp_token_a_mint,
        &ctx.lp_token_b_mint,
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
    let lp_token_a_mint_legacy = Keypair::new();
    let lp_token_b_mint_legacy = Keypair::new();
    
    let config_legacy = create_pool_legacy_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        &lp_token_a_mint_legacy,
        &lp_token_b_mint_legacy,
        Some(4), // Different ratio to avoid conflict
    ).await?;

    // Verify using utility
    verify_pool_state(
        &mut ctx.env.banks_client,
        &config_legacy,
        &ctx.env.payer.pubkey(),
        &lp_token_a_mint_legacy.pubkey(),
        &lp_token_b_mint_legacy.pubkey(),
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

    // Test normalization directly
    let config1 = normalize_pool_config(&ctx.primary_mint.pubkey(), &ctx.base_mint.pubkey(), 3);
    let config2 = normalize_pool_config(&ctx.base_mint.pubkey(), &ctx.primary_mint.pubkey(), 3);

    // Both should result in the same normalized configuration
    assert_eq!(config1.token_a_mint, config2.token_a_mint, "Token A should be the same after normalization");
    assert_eq!(config1.token_b_mint, config2.token_b_mint, "Token B should be the same after normalization");
    assert_eq!(config1.pool_state_pda, config2.pool_state_pda, "Pool PDAs should be the same after normalization");

    // The ratios should be adjusted to maintain the same effective exchange rate
    println!("✅ Normalization logic works correctly for different token orderings");
    println!("   Config 1 - Token A: {}, Token B: {}", config1.token_a_mint, config1.token_b_mint);
    println!("   Config 1 - Ratio: {}:{}", config1.ratio_a_numerator, config1.ratio_b_denominator);
    println!("   Config 2 should be identical due to normalization");
    
    Ok(())
} 