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

//! # Client SDK Tests
//! 
//! This module contains tests for the client SDK functionality,
//! including client initialization, PDA derivation, and instruction building.

mod common;

use common::*;
use fixed_ratio_trading::{ID as PROGRAM_ID, client_sdk::{PoolClient, PoolConfig}};
use solana_program::pubkey::Pubkey;

/// Test PoolClient initialization and configuration (SDK-001)
#[tokio::test]
async fn test_pool_client_new() -> TestResult {
    println!("Running SDK-001: test_pool_client_new - PoolClient initialization and configuration");
    
    // Create a new pool client with the program ID
    let pool_client = PoolClient::new(PROGRAM_ID);
    
    // 1. Verify the program ID is stored correctly
    assert_eq!(
        pool_client.program_id, 
        PROGRAM_ID,
        "PoolClient should store the provided program ID correctly"
    );
    println!("✅ PoolClient stores program ID correctly");
    
    // 2. Test creating a client with a random program ID to ensure flexible initialization
    let random_program_id = Pubkey::new_unique();
    let custom_client = PoolClient::new(random_program_id);
    assert_eq!(
        custom_client.program_id, 
        random_program_id,
        "PoolClient should accept any valid program ID"
    );
    println!("✅ PoolClient accepts custom program IDs");
    
    // 3. Test creating a pool configuration with the client
    let primary_token_mint = Pubkey::new_unique();
    let base_token_mint = Pubkey::new_unique();
    let ratio = 1000; // 1000:1 ratio
    let pool_config = PoolConfig {
        primary_token_mint,
        base_token_mint,
        ratio_primary_per_base: ratio,
    };
    
    // 4. Verify pool configuration values
    assert_eq!(pool_config.primary_token_mint, primary_token_mint);
    assert_eq!(pool_config.base_token_mint, base_token_mint);
    assert_eq!(pool_config.ratio_primary_per_base, ratio);
    println!("✅ Pool configuration initialized with correct values");
    
    // 5. Test pool configuration creation through factory method
    let pool_config_alt = PoolConfig::new(
        primary_token_mint,
        base_token_mint,
        ratio
    ).expect("Pool config creation should succeed");
    
    assert_eq!(pool_config_alt.primary_token_mint, primary_token_mint);
    assert_eq!(pool_config_alt.base_token_mint, base_token_mint);
    assert_eq!(pool_config_alt.ratio_primary_per_base, ratio);
    println!("✅ Pool configuration created via factory method correctly");
    
    // 6. Test error case: zero ratio
    let zero_ratio_result = PoolConfig::new(
        primary_token_mint,
        base_token_mint,
        0
    );
    assert!(zero_ratio_result.is_err(), "Zero ratio should be rejected");
    println!("✅ Zero ratio correctly rejected");
    
    // 7. Test error case: identical tokens
    let identical_tokens_result = PoolConfig::new(
        primary_token_mint,
        primary_token_mint, // Same token for both primary and base
        ratio
    );
    assert!(identical_tokens_result.is_err(), "Identical tokens should be rejected");
    println!("✅ Identical tokens correctly rejected");
    
    // 8. Test using the testing utility function
    let test_config = fixed_ratio_trading::client_sdk::testing::create_test_pool_config();
    assert_ne!(test_config.primary_token_mint, test_config.base_token_mint);
    assert!(test_config.ratio_primary_per_base > 0);
    println!("✅ Test utility function creates valid configuration");
    
    // 9. Test keypair creation utility
    let test_keypairs = fixed_ratio_trading::client_sdk::testing::create_test_keypairs(5);
    assert_eq!(test_keypairs.len(), 5);
    // Ensure all keypairs are unique
    let unique_keypairs: std::collections::HashSet<_> = test_keypairs.iter().collect();
    assert_eq!(unique_keypairs.len(), 5);
    println!("✅ Test keypair utility generates unique keypairs");
    
    println!("✅ SDK-001 test completed successfully");
    Ok(())
}

/// Test PDA derivation accuracy and consistency (SDK-002)
#[tokio::test]
async fn test_derive_pool_addresses() -> TestResult {
    println!("Running SDK-002: test_derive_pool_addresses - PDA derivation accuracy and consistency");
    
    // Create a new pool client with program ID
    let pool_client = PoolClient::new(PROGRAM_ID);
    
    // 1. Create a standard pool configuration for testing
    let primary_token_mint = Pubkey::new_unique();
    let base_token_mint = Pubkey::new_unique();
    let ratio = 1000; // 1000:1 ratio
    let pool_config = PoolConfig {
        primary_token_mint,
        base_token_mint,
        ratio_primary_per_base: ratio,
    };
    
    // 2. Derive addresses for the pool
    let addresses = pool_client.derive_pool_addresses(&pool_config);
    
    // 3. Verify normalization of token mints (lexicographic ordering)
    let (expected_token_a, expected_token_b) = if primary_token_mint < base_token_mint {
        (primary_token_mint, base_token_mint)
    } else {
        (base_token_mint, primary_token_mint)
    };
    
    assert_eq!(addresses.token_a_mint, expected_token_a, "Token A mint should follow lexicographic ordering");
    assert_eq!(addresses.token_b_mint, expected_token_b, "Token B mint should follow lexicographic ordering");
    println!("✅ Token mint normalization works correctly");
    
    // 4. Manually calculate expected PDAs to verify against SDK-derived values
    use fixed_ratio_trading::{
        POOL_STATE_SEED_PREFIX, 
        TOKEN_A_VAULT_SEED_PREFIX, 
        TOKEN_B_VAULT_SEED_PREFIX
    };
    
    let (expected_pool_state, expected_pool_bump) = Pubkey::find_program_address(
        &[
            POOL_STATE_SEED_PREFIX,
            addresses.token_a_mint.as_ref(),
            addresses.token_b_mint.as_ref(),
            &addresses.ratio_a_numerator.to_le_bytes(),
            &addresses.ratio_b_denominator.to_le_bytes(),
        ],
        &PROGRAM_ID,
    );
    
    let (expected_token_a_vault, expected_token_a_bump) = Pubkey::find_program_address(
        &[TOKEN_A_VAULT_SEED_PREFIX, expected_pool_state.as_ref()],
        &PROGRAM_ID,
    );
    
    let (expected_token_b_vault, expected_token_b_bump) = Pubkey::find_program_address(
        &[TOKEN_B_VAULT_SEED_PREFIX, expected_pool_state.as_ref()],
        &PROGRAM_ID,
    );
    
    // 5. Verify pool state PDA derivation
    assert_eq!(addresses.pool_state, expected_pool_state, "Pool state PDA should match manual calculation");
    assert_eq!(addresses.pool_state_bump, expected_pool_bump, "Pool state bump should match manual calculation");
    println!("✅ Pool state PDA derivation is correct");
    
    // 6. Verify token vault PDAs
    assert_eq!(addresses.token_a_vault, expected_token_a_vault, "Token A vault PDA should match manual calculation");
    assert_eq!(addresses.token_a_vault_bump, expected_token_a_bump, "Token A vault bump should match manual calculation");
    
    assert_eq!(addresses.token_b_vault, expected_token_b_vault, "Token B vault PDA should match manual calculation");
    assert_eq!(addresses.token_b_vault_bump, expected_token_b_bump, "Token B vault bump should match manual calculation");
    println!("✅ Token vault PDAs derivation is correct");
    
    // 7. Verify that multiple derivations produce the same results (consistency)
    let addresses_repeat = pool_client.derive_pool_addresses(&pool_config);
    assert_eq!(addresses.pool_state, addresses_repeat.pool_state, "Pool state PDA should be consistent across calls");
    assert_eq!(addresses.token_a_vault, addresses_repeat.token_a_vault, "Token A vault should be consistent across calls");
    assert_eq!(addresses.token_b_vault, addresses_repeat.token_b_vault, "Token B vault should be consistent across calls");
    println!("✅ PDA derivation is consistent across multiple calls");
    
    // 8. Test with swapped primary and base tokens to verify normalization effectiveness
    let swapped_config = PoolConfig {
        primary_token_mint: base_token_mint,  // Swapped
        base_token_mint: primary_token_mint,  // Swapped
        ratio_primary_per_base: ratio,
    };
    
    let swapped_addresses = pool_client.derive_pool_addresses(&swapped_config);
    
    // Check if the same pool state is derived even with swapped tokens
    assert_eq!(addresses.pool_state, swapped_addresses.pool_state, 
        "Pool state PDA should be the same regardless of token parameter order");
    
    assert_eq!(addresses.token_a_vault, swapped_addresses.token_a_vault, 
        "Token A vault should be the same regardless of token parameter order");
    
    assert_eq!(addresses.token_b_vault, swapped_addresses.token_b_vault, 
        "Token B vault should be the same regardless of token parameter order");
    println!("✅ Token normalization ensures consistent PDAs regardless of parameter order");
    
    // 9. Verify ratio normalization works correctly
    assert_eq!(addresses.ratio_a_numerator, ratio, "Ratio A numerator should be preserved");
    assert_eq!(addresses.ratio_b_denominator, 1u64, "Ratio B denominator should be normalized to 1");
    println!("✅ Ratio normalization works correctly");
    
    // 10. Verify with different ratios to ensure PDA uniqueness
    let different_ratio_config = PoolConfig {
        primary_token_mint,
        base_token_mint,
        ratio_primary_per_base: ratio * 2, // Double the ratio
    };
    
    let different_ratio_addresses = pool_client.derive_pool_addresses(&different_ratio_config);
    
    assert_ne!(addresses.pool_state, different_ratio_addresses.pool_state, 
        "Different ratios should produce different pool state PDAs");
    
    // Token vaults will be different because they depend on the pool state
    assert_ne!(addresses.token_a_vault, different_ratio_addresses.token_a_vault, 
        "Different ratios should produce different token A vault PDAs");
    
    assert_ne!(addresses.token_b_vault, different_ratio_addresses.token_b_vault, 
        "Different ratios should produce different token B vault PDAs");
    println!("✅ Different ratios produce different PDAs as expected");
    
    println!("✅ SDK-002 test completed successfully");
    Ok(())
}
