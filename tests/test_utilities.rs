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

//! # Utility Functions and Helper Component Tests
//! 
//! This module contains unit tests for utility functions, helper components,
//! and core functionality that doesn't require full integration testing.

mod common;

use common::*;
use solana_program::rent::Rent;
use solana_sdk::program_pack::Pack;
use spl_token::state::{Account as TokenAccount, Mint as MintAccount};
use fixed_ratio_trading::{RentRequirements, PoolError, MINIMUM_RENT_BUFFER, DelegateManagement};

// ================================================================================================
// RENT REQUIREMENTS TESTS
// ================================================================================================

#[test]
fn test_rent_requirements_new() {
    // Create a mock Rent object
    let rent = Rent {
        lamports_per_byte_year: 3480,
        exemption_threshold: 2.0,
        burn_percent: 50,
    };

    let rent_req = RentRequirements::new(&rent);

    // Verify initial values
    assert_eq!(rent_req.last_update_slot, 0);
    assert_eq!(rent_req.rent_exempt_minimum, rent.minimum_balance(0));
    assert_eq!(rent_req.pool_state_rent, rent.minimum_balance(PoolState::get_packed_len()));
    assert_eq!(rent_req.token_vault_rent, rent.minimum_balance(TokenAccount::LEN));
    assert_eq!(rent_req.lp_mint_rent, rent.minimum_balance(MintAccount::LEN));
}

#[test]
fn test_rent_requirements_update_if_needed() {
    let rent = Rent {
        lamports_per_byte_year: 3480,
        exemption_threshold: 2.0,
        burn_percent: 50,
    };

    let mut rent_req = RentRequirements::new(&rent);
    
    // Test that update is needed when last_update_slot is 0
    assert_eq!(rent_req.update_if_needed(&rent, 0), true);
    assert_eq!(rent_req.last_update_slot, 0);

    // Set last_update_slot to simulate initialized state
    rent_req.last_update_slot = 100;

    // Test that no update is needed for small slot differences
    assert_eq!(rent_req.update_if_needed(&rent, 200), false);
    assert_eq!(rent_req.last_update_slot, 100);

    // Test that update happens after 1000 slots
    assert_eq!(rent_req.update_if_needed(&rent, 1101), true);
    assert_eq!(rent_req.last_update_slot, 1101);

    // Test that no update is needed immediately after
    assert_eq!(rent_req.update_if_needed(&rent, 1102), false);
    
    // Test that update happens if rent parameters change
    let new_rent = Rent {
        lamports_per_byte_year: 4000, // Changed
        exemption_threshold: 2.0,
        burn_percent: 50,
    };
    assert_eq!(rent_req.update_if_needed(&new_rent, 1103), true);
    assert_eq!(rent_req.last_update_slot, 1103);
}

#[test]
fn test_rent_requirements_get_total_required_rent() {
    let rent = Rent {
        lamports_per_byte_year: 3480,
        exemption_threshold: 2.0,
        burn_percent: 50,
    };

    let rent_req = RentRequirements::new(&rent);
    
    // Calculate expected total
    let expected_total = rent_req.pool_state_rent + 
                       (2 * rent_req.token_vault_rent) + 
                       (2 * rent_req.lp_mint_rent) + 
                       MINIMUM_RENT_BUFFER;
    
    assert_eq!(rent_req.get_total_required_rent(), expected_total);
}

#[test]
fn test_rent_requirements_get_packed_len() {
    // Test that get_packed_len returns the correct size
    let expected_len = 8 + // last_update_slot
                      8 + // rent_exempt_minimum
                      8 + // pool_state_rent
                      8 + // token_vault_rent
                      8;  // lp_mint_rent
    
    assert_eq!(RentRequirements::get_packed_len(), expected_len);
    assert_eq!(RentRequirements::get_packed_len(), 40); // Corrected expected value
}

// ================================================================================================
// POOL ERROR TESTS
// ================================================================================================

#[test]
fn test_pool_error_error_code() {
    // Test each error variant returns the correct error code
    let error = PoolError::InvalidTokenPair {
        token_a: Pubkey::new_unique(),
        token_b: Pubkey::new_unique(),
        reason: "test".to_string(),
    };
    assert_eq!(error.error_code(), 1001);

    let error = PoolError::InvalidRatio {
        ratio: 0,
        min_ratio: 1,
        max_ratio: 100,
    };
    assert_eq!(error.error_code(), 1002);

    let error = PoolError::InsufficientFunds {
        required: 100,
        available: 50,
        account: Pubkey::new_unique(),
    };
    assert_eq!(error.error_code(), 1003);

    let error = PoolError::InvalidTokenAccount {
        account: Pubkey::new_unique(),
        reason: "test".to_string(),
    };
    assert_eq!(error.error_code(), 1004);

    let error = PoolError::InvalidSwapAmount {
        amount: 0,
        min_amount: 1,
        max_amount: 100,
    };
    assert_eq!(error.error_code(), 1005);

    let error = PoolError::RentExemptError {
        account: Pubkey::new_unique(),
        required: 100,
        available: 50,
    };
    assert_eq!(error.error_code(), 1006);

    assert_eq!(PoolError::WithdrawalTooLarge.error_code(), 1007);
    assert_eq!(PoolError::WithdrawalCooldown.error_code(), 1008);
    assert_eq!(PoolError::PoolPaused.error_code(), 1009);
    assert_eq!(PoolError::DelegateLimitExceeded.error_code(), 1010);
    
    let error = PoolError::DelegateAlreadyExists {
        delegate: Pubkey::new_unique(),
    };
    assert_eq!(error.error_code(), 1011);
    
    let error = PoolError::DelegateNotFound {
        delegate: Pubkey::new_unique(),
    };
    assert_eq!(error.error_code(), 1012);
}

#[test]
fn test_pool_error_display() {
    let token_a = Pubkey::new_unique();
    let token_b = Pubkey::new_unique();
    let error = PoolError::InvalidTokenPair {
        token_a,
        token_b,
        reason: "test reason".to_string(),
    };
    let display_str = format!("{}", error);
    assert!(display_str.contains(&token_a.to_string()));
    assert!(display_str.contains(&token_b.to_string()));
    assert!(display_str.contains("test reason"));

    let error = PoolError::InvalidRatio {
        ratio: 0,
        min_ratio: 1,
        max_ratio: 100,
    };
    let display_str = format!("{}", error);
    assert!(display_str.contains("0"));
    assert!(display_str.contains("1"));
    assert!(display_str.contains("100"));

    let error = PoolError::WithdrawalTooLarge;
    assert_eq!(format!("{}", error), "Withdrawal amount exceeds maximum allowed percentage");

    let error = PoolError::WithdrawalCooldown;
    assert_eq!(format!("{}", error), "Withdrawal is currently in cooldown period");

    let error = PoolError::PoolPaused;
    assert_eq!(format!("{}", error), "Pool operations are currently paused");

    let error = PoolError::DelegateLimitExceeded;
    assert_eq!(format!("{}", error), "Delegate limit exceeded");

    let delegate_key = Pubkey::new_unique();
    let error = PoolError::DelegateAlreadyExists { delegate: delegate_key };
    let display_str = format!("{}", error);
    assert!(display_str.contains(&delegate_key.to_string()));
    assert!(display_str.contains("Delegate already exists"));

    let error = PoolError::DelegateNotFound { delegate: delegate_key };
    let display_str = format!("{}", error);
    assert!(display_str.contains(&delegate_key.to_string()));
    assert!(display_str.contains("Delegate not found"));
}

#[test]
fn test_pool_error_to_program_error() {
    use solana_program::program_error::ProgramError;

    // Test conversion from PoolError to ProgramError
    let error = PoolError::InvalidTokenPair {
        token_a: Pubkey::new_unique(),
        token_b: Pubkey::new_unique(),
        reason: "test".to_string(),
    };
    let program_error: ProgramError = error.into();
    assert_eq!(program_error, ProgramError::Custom(1001));

    let error = PoolError::InvalidRatio {
        ratio: 0,
        min_ratio: 1,
        max_ratio: 100,
    };
    let program_error: ProgramError = error.into();
    assert_eq!(program_error, ProgramError::Custom(1002));

    let error = PoolError::WithdrawalTooLarge;
    let program_error: ProgramError = error.into();
    assert_eq!(program_error, ProgramError::Custom(1007));

    let error = PoolError::DelegateLimitExceeded;
    let program_error: ProgramError = error.into();
    assert_eq!(program_error, ProgramError::Custom(1010));

    let error = PoolError::DelegateAlreadyExists {
        delegate: Pubkey::new_unique(),
    };
    let program_error: ProgramError = error.into();
    assert_eq!(program_error, ProgramError::Custom(1011));

    let error = PoolError::DelegateNotFound {
        delegate: Pubkey::new_unique(),
    };
    let program_error: ProgramError = error.into();
    assert_eq!(program_error, ProgramError::Custom(1012));
}

// ================================================================================================
// POOL STATE TESTS
// ================================================================================================

#[test]
fn test_pool_state_get_packed_len() {
    // Test that get_packed_len returns the expected size
    let expected_size = 
        32 + // owner
        32 + // token_a_mint
        32 + // token_b_mint
        32 + // token_a_vault
        32 + // token_b_vault
        32 + // lp_token_a_mint
        32 + // lp_token_b_mint
        8 +  // ratio_a_numerator
        8 +  // ratio_b_denominator
        8 +  // total_token_a_liquidity
        8 +  // total_token_b_liquidity
        1 +  // pool_authority_bump_seed
        1 +  // token_a_vault_bump_seed
        1 +  // token_b_vault_bump_seed
        1 +  // is_initialized
        RentRequirements::get_packed_len() + // rent_requirements
        1 +  // is_paused
        DelegateManagement::get_packed_len() + // delegate_management
        8 +  // collected_fees_token_a
        8 +  // collected_fees_token_b
        8 +  // total_fees_withdrawn_token_a
        8 +  // total_fees_withdrawn_token_b
        8 +  // swap_fee_basis_points
        8 +  // collected_sol_fees
        8;   // total_sol_fees_withdrawn

    assert_eq!(PoolState::get_packed_len(), expected_size);
}

// ================================================================================================
// NORMALIZATION TESTS
// ================================================================================================

#[test]
fn test_normalize_pool_config_functionality() {
    // Create test keypairs
    let primary_mint = Keypair::new();
    let base_mint = Keypair::new();
    
    let config = normalize_pool_config(&primary_mint.pubkey(), &base_mint.pubkey(), 2);
    
    // Verify normalization worked
    assert!(config.token_a_mint <= config.token_b_mint, "Token A should be lexicographically smaller");
    assert!(config.ratio_a_numerator > 0, "Ratio A numerator should be positive");
    assert!(config.ratio_b_denominator > 0, "Ratio B denominator should be positive");
    
    // Test with reversed tokens
    let config_reversed = normalize_pool_config(&base_mint.pubkey(), &primary_mint.pubkey(), 2);
    
    // Should result in same normalized configuration
    assert_eq!(config.token_a_mint, config_reversed.token_a_mint);
    assert_eq!(config.token_b_mint, config_reversed.token_b_mint);
    assert_eq!(config.pool_state_pda, config_reversed.pool_state_pda);
}

#[test]
#[should_panic(expected = "Primary and Base token mints cannot be the same")]
fn test_normalize_pool_config_identical_tokens_panics() {
    let mint = Keypair::new();
    normalize_pool_config(&mint.pubkey(), &mint.pubkey(), 2);
}

// ================================================================================================
// DELEGATE MANAGEMENT TESTS
// ================================================================================================

#[test]
fn test_delegate_management_get_packed_len() {
    // Test that delegate management has a reasonable packed length
    let len = DelegateManagement::get_packed_len();
    assert!(len > 0, "Delegate management should have non-zero packed length");
    
    // Updated bounds to account for pool pause functionality (delegates, withdrawal history, pool pause requests)
    // Expected size: ~1,509 bytes (3 delegates * multiple arrays + withdrawal history + pool pause system)
    assert!(len >= 1400, "Delegate management should include comprehensive governance features");
    assert!(len <= 2000, "Delegate management packed length should remain reasonable for Solana");
    
    // Verify the calculated size matches expected structure
    let expected_size = 
        (32 * 3) +        // delegates array (3 delegates)
        1 +               // delegate_count
        (88 * 10) +       // withdrawal_history (10 records, 88 bytes each)
        1 +               // withdrawal_history_index  
        (96 * 3) +        // withdrawal_requests (3 requests, 96 bytes each)
        (8 * 3) +         // delegate_wait_times (3 delegates, 8 bytes each)
        (65 * 3) +        // pool_pause_requests (3 requests, 65 bytes each)
        (8 * 3);          // pool_pause_wait_times (3 delegates, 8 bytes each)
    
    assert_eq!(len, expected_size, 
        "Packed length should match calculated structure size. Got: {}, Expected: {}", 
        len, expected_size);
}

// ================================================================================================
// COMMON UTILITIES TESTS
// ================================================================================================

#[tokio::test]
async fn test_test_environment_setup() -> TestResult {
    let env = start_test_environment().await;
    
    // Verify environment setup
    assert!(env.payer.pubkey() != Pubkey::default(), "Payer should have valid pubkey");
    assert!(env.recent_blockhash != solana_sdk::hash::Hash::default(), "Should have valid blockhash");
    
    println!("✅ Test environment setup working correctly");
    
    Ok(())
}

#[tokio::test]
async fn test_pool_test_context_setup() -> TestResult {
    let ctx = setup_pool_test_context(false).await;
    
    // Verify pool context setup
    assert!(ctx.env.payer.pubkey() != Pubkey::default(), "Pool context payer should be valid");
    assert!(ctx.primary_mint.pubkey() != Pubkey::default(), "Primary mint should be valid");
    assert!(ctx.base_mint.pubkey() != Pubkey::default(), "Base mint should be valid");
    assert!(ctx.lp_token_a_mint.pubkey() != Pubkey::default(), "LP Token A mint should be valid");
    assert!(ctx.lp_token_b_mint.pubkey() != Pubkey::default(), "LP Token B mint should be valid");
    
    // Verify mints are unique
    assert_ne!(ctx.primary_mint.pubkey(), ctx.base_mint.pubkey(), "Primary and base mints should be different");
    assert_ne!(ctx.lp_token_a_mint.pubkey(), ctx.lp_token_b_mint.pubkey(), "LP mints should be different");
    
    println!("✅ Pool test context setup working correctly");
    
    Ok(())
}

#[tokio::test]
async fn test_create_funded_user() -> TestResult {
    let mut env = start_test_environment().await;
    
    let user = create_funded_user(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        Some(1_000_000), // 1M lamports
    ).await?;
    
    // Verify user was created and funded
    let balance = get_sol_balance(&mut env.banks_client, &user.pubkey()).await;
    assert!(balance >= 1_000_000, "User should have at least 1M lamports");
    
    println!("✅ Create funded user utility working correctly");
    
    Ok(())
}

// ================================================================================================
// INTEGRATION HELPERS TESTS
// ================================================================================================

#[tokio::test]
async fn test_create_test_mints() -> TestResult {
    let mut env = start_test_environment().await;
    
    let mint1 = Keypair::new();
    let mint2 = Keypair::new();
    
    create_test_mints(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &[&mint1, &mint2],
    ).await?;
    
    // Verify mints were created
    assert!(account_exists(&mut env.banks_client, &mint1.pubkey()).await, "Mint 1 should exist");
    assert!(account_exists(&mut env.banks_client, &mint2.pubkey()).await, "Mint 2 should exist");
    
    println!("✅ Create test mints utility working correctly");
    
    Ok(())
}

#[tokio::test]
async fn test_setup_test_user() -> TestResult {
    let mut env = start_test_environment().await;
    
    let primary_mint = Keypair::new();
    let base_mint = Keypair::new();
    
    // Create mints first
    create_test_mints(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &[&primary_mint, &base_mint],
    ).await?;
    
    let (user, primary_token_account, base_token_account) = setup_test_user(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &primary_mint.pubkey(),
        &base_mint.pubkey(),
        None,
    ).await?;
    
    // Verify user and token accounts were created
    assert!(account_exists(&mut env.banks_client, &user.pubkey()).await, "User should exist");
    assert!(account_exists(&mut env.banks_client, &primary_token_account.pubkey()).await, "Primary token account should exist");
    assert!(account_exists(&mut env.banks_client, &base_token_account.pubkey()).await, "Base token account should exist");
    
    println!("✅ Setup test user utility working correctly");
    
    Ok(())
}

// ================================================================================================
// CONSTANTS TESTS
// ================================================================================================

#[test]
fn test_constants_values() {
    // Test that constants have reasonable values
    assert_eq!(constants::DEFAULT_RATIO, 2, "Default ratio should be 2");
    assert_eq!(constants::TOKEN_DECIMALS, 9, "Token decimals should be 9");
    assert!(constants::DEFAULT_LIQUIDITY_AMOUNT > 0, "Default liquidity should be positive");
    assert!(constants::DEFAULT_USER_TOKEN_AMOUNT > 0, "Default user tokens should be positive");
    assert!(constants::DEFAULT_SOL_AIRDROP > 0, "Default SOL airdrop should be positive");
    
    // Verify reasonable relationships
    assert!(constants::DEFAULT_LIQUIDITY_AMOUNT < constants::DEFAULT_USER_TOKEN_AMOUNT, 
        "User tokens should be more than default liquidity for testing");
}

#[test]
fn test_program_constants() {
    // Test program-specific constants
    assert!(PROGRAM_ID != Pubkey::default(), "Program ID should not be default");
    assert!(MINIMUM_RENT_BUFFER > 0, "Minimum rent buffer should be positive");
    
    // Test seed prefixes are reasonable
    assert!(!POOL_STATE_SEED_PREFIX.is_empty(), "Pool state seed prefix should not be empty");
    assert!(!TOKEN_A_VAULT_SEED_PREFIX.is_empty(), "Token A vault seed prefix should not be empty");
    assert!(!TOKEN_B_VAULT_SEED_PREFIX.is_empty(), "Token B vault seed prefix should not be empty");
} 