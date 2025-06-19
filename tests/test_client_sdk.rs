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
    
    // This is a placeholder for the next test in the sequence
    // Implementation will be added in a separate task
    
    Ok(())
}
