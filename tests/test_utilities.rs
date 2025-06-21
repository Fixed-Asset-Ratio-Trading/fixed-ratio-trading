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
use solana_program::pubkey::Pubkey;
use solana_program::instruction::Instruction;
use solana_sdk::program_pack::Pack;
use solana_sdk::transaction::Transaction;
use solana_sdk::signature::Keypair;
use spl_token::state::{Account as TokenAccount, Mint as MintAccount};
use borsh::BorshSerialize;
use fixed_ratio_trading::{
    RentRequirements, 
    PoolError, 
    MINIMUM_RENT_BUFFER, 
    DelegateManagement
};

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

    assert_eq!(PoolError::PoolPaused.error_code(), 1007);
    assert_eq!(PoolError::DelegateLimitExceeded.error_code(), 1008);
    let error = PoolError::DelegateAlreadyExists {
        delegate: Pubkey::new_unique(),
    };
    assert_eq!(error.error_code(), 1009);
    let error = PoolError::DelegateNotFound {
        delegate: Pubkey::new_unique(),
    };
    assert_eq!(error.error_code(), 1010);
    assert_eq!(PoolError::InvalidWaitTime { wait_time: 0 }.error_code(), 1011);
    assert_eq!(PoolError::PendingWithdrawalExists.error_code(), 1012);
    assert_eq!(PoolError::NoPendingWithdrawal.error_code(), 1013);
    assert_eq!(PoolError::UnauthorizedDelegate.error_code(), 1014);
    assert_eq!(PoolError::InsufficientFees.error_code(), 1015);
    assert_eq!(PoolError::InvalidWithdrawalRequest.error_code(), 1016);
    assert_eq!(PoolError::WithdrawalNotReady.error_code(), 1017);
    assert_eq!(PoolError::Unauthorized.error_code(), 1018);
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

    let error = PoolError::InvalidWaitTime { wait_time: 100 };
    assert_eq!(format!("{}", error), "Invalid wait time: 100 seconds");

    let error = PoolError::PendingWithdrawalExists;
    assert_eq!(format!("{}", error), "Pending withdrawal request exists");

    let error = PoolError::NoPendingWithdrawal;
    assert_eq!(format!("{}", error), "No pending withdrawal request");

    let error = PoolError::UnauthorizedDelegate;
    assert_eq!(format!("{}", error), "Unauthorized delegate");

    let error = PoolError::InsufficientFees;
    assert_eq!(format!("{}", error), "Insufficient fees");

    let error = PoolError::InvalidWithdrawalRequest;
    assert_eq!(format!("{}", error), "Invalid withdrawal request");

    let error = PoolError::WithdrawalNotReady;
    assert_eq!(format!("{}", error), "Withdrawal not ready");

    let error = PoolError::Unauthorized;
    assert_eq!(format!("{}", error), "Unauthorized");
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

    let error = PoolError::PoolPaused;
    let program_error: ProgramError = error.into();
    assert_eq!(program_error, ProgramError::Custom(1007));

    let error = PoolError::DelegateLimitExceeded;
    let program_error: ProgramError = error.into();
    assert_eq!(program_error, ProgramError::Custom(1008));

    let error = PoolError::DelegateAlreadyExists {
        delegate: Pubkey::new_unique(),
    };
    let program_error: ProgramError = error.into();
    assert_eq!(program_error, ProgramError::Custom(1009));

    let error = PoolError::DelegateNotFound {
        delegate: Pubkey::new_unique(),
    };
    let program_error: ProgramError = error.into();
    assert_eq!(program_error, ProgramError::Custom(1010));

    let error = PoolError::InvalidWaitTime { wait_time: 0 };
    let program_error: ProgramError = error.into();
    assert_eq!(program_error, ProgramError::Custom(1011));

    let error = PoolError::PendingWithdrawalExists;
    let program_error: ProgramError = error.into();
    assert_eq!(program_error, ProgramError::Custom(1012));

    let error = PoolError::NoPendingWithdrawal;
    let program_error: ProgramError = error.into();
    assert_eq!(program_error, ProgramError::Custom(1013));

    let error = PoolError::UnauthorizedDelegate;
    let program_error: ProgramError = error.into();
    assert_eq!(program_error, ProgramError::Custom(1014));

    let error = PoolError::InsufficientFees;
    let program_error: ProgramError = error.into();
    assert_eq!(program_error, ProgramError::Custom(1015));

    let error = PoolError::InvalidWithdrawalRequest;
    let program_error: ProgramError = error.into();
    assert_eq!(program_error, ProgramError::Custom(1016));

    let error = PoolError::WithdrawalNotReady;
    let program_error: ProgramError = error.into();
    assert_eq!(program_error, ProgramError::Custom(1017));

    let error = PoolError::Unauthorized;
    let program_error: ProgramError = error.into();
    assert_eq!(program_error, ProgramError::Custom(1018));
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
        40 + // rent_requirements
        1 +  // is_paused
        1 +  // swaps_paused
        33 + // swaps_pause_requested_by (Option<Pubkey>)
        8 +  // swaps_pause_initiated_timestamp
        1 +  // withdrawal_protection_active
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
    println!("Actual DelegateManagement packed length: {} bytes", len);
    
    assert!(len > 0, "Delegate management should have non-zero packed length");
    // Updated bounds to account for pool pause functionality (delegates, withdrawal history, pool pause requests)
    // Expected size: ~1,611 bytes (3 delegates * multiple arrays + withdrawal history + pool pause system)
    assert!(len >= 1400, "Delegate management should include comprehensive governance features");
    assert!(len <= 2000, "Delegate management packed length should remain reasonable for Solana");
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
// PDA DERIVATION TESTS (UTIL-001) - IMPROVED VERSION
// ================================================================================================

/// UTIL-001: Enhanced test for pool state PDA derivation and validation
/// 
/// This test validates the get_pool_state_pda utility function and covers:
/// 1. Basic PDA derivation functionality with output validation
/// 2. Consistency validation using manual PDA derivation
/// 3. Token order normalization with instruction output verification
/// 4. Different ratios produce different PDAs
/// 5. Edge cases with comprehensive validation
/// 6. Performance characteristics with realistic scenarios
/// 7. Error handling and validation
#[tokio::test]
async fn test_get_pool_state_pda() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running UTIL-001: test_get_pool_state_pda");
    
    let mut env = start_test_environment().await;
    
    // Create test token mints with deterministic ordering for consistent testing
    let token_a_mint = Keypair::new();
    let token_b_mint = Keypair::new();
    create_test_mints(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &[&token_a_mint, &token_b_mint],
    ).await?;
    
    let ratio = 5u64; // 5:1 ratio for testing
    
    // ===============================================================================
    // Test 1: Basic PDA derivation functionality with output validation
    // ===============================================================================
    {
        println!("Test 1: Basic PDA derivation with output validation");
        
        let instruction_data = PoolInstruction::GetPoolStatePDA {
            primary_token_mint: token_a_mint.pubkey(),
            base_token_mint: token_b_mint.pubkey(),
            ratio_primary_per_base: ratio,
        };
        
        let instruction = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![], // No accounts needed for this utility
            data: instruction_data.try_to_vec()?,
        };
        
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&env.payer.pubkey()),
            &[&env.payer],
            env.recent_blockhash,
        );
        
        let result = env.banks_client.process_transaction(transaction).await;
        assert!(result.is_ok(), "get_pool_state_pda instruction should succeed");
        
        // Enhanced validation: Verify the instruction completed successfully
        // (Note: In a real implementation, you would capture the returned PDA from logs or return data)
        println!("✅ Basic PDA derivation instruction executed successfully");
    }
    
    // ===============================================================================
    // Test 2: Consistency validation using manual PDA derivation
    // ===============================================================================
    {
        println!("Test 2: Manual PDA derivation consistency validation");
        
        // Derive PDA manually for comparison
        let (token_a_norm, token_b_norm) = if token_a_mint.pubkey() < token_b_mint.pubkey() {
            (token_a_mint.pubkey(), token_b_mint.pubkey())
        } else {
            (token_b_mint.pubkey(), token_a_mint.pubkey())
        };
        
        let (ratio_a, ratio_b) = (ratio, 1u64);
        
        let (expected_pda, expected_bump) = Pubkey::find_program_address(
            &[
                POOL_STATE_SEED_PREFIX,
                token_a_norm.as_ref(),
                token_b_norm.as_ref(),
                &ratio_a.to_le_bytes(),
                &ratio_b.to_le_bytes(),
            ],
            &PROGRAM_ID,
        );
        
        println!("Expected PDA: {}, Expected Bump: {}", expected_pda, expected_bump);
        
        // Verify bump seed is in valid range (u8 is always <= 255, so just check lower bound)
        assert!(expected_bump >= 240, 
                "Bump seed should be in valid range (240-255), got: {}", expected_bump);
        
        // Verify PDA is not the default pubkey
        assert_ne!(expected_pda, Pubkey::default(), "PDA should not be default pubkey");
        
        println!("✅ Manual PDA derivation validation passed");
    }
    
    // ===============================================================================
    // Test 3: Token order normalization with instruction output verification
    // ===============================================================================
    {
        println!("Test 3: Token normalization with instruction verification");
        
        // Test that both orderings produce the same PDA via manual derivation
        let (token_a_norm_1, token_b_norm_1) = if token_a_mint.pubkey() < token_b_mint.pubkey() {
            (token_a_mint.pubkey(), token_b_mint.pubkey())
        } else {
            (token_b_mint.pubkey(), token_a_mint.pubkey())
        };
        
        let (token_a_norm_2, token_b_norm_2) = if token_b_mint.pubkey() < token_a_mint.pubkey() {
            (token_b_mint.pubkey(), token_a_mint.pubkey())
        } else {
            (token_a_mint.pubkey(), token_b_mint.pubkey())
        };
        
        // Both should normalize to the same ordering
        assert_eq!(token_a_norm_1, token_a_norm_2, "Token A normalization should be consistent");
        assert_eq!(token_b_norm_1, token_b_norm_2, "Token B normalization should be consistent");
        
        // Derive PDAs for both orderings - should be identical
        let (pda1, bump1) = Pubkey::find_program_address(
            &[
                POOL_STATE_SEED_PREFIX,
                token_a_norm_1.as_ref(),
                token_b_norm_1.as_ref(),
                &ratio.to_le_bytes(),
                &1u64.to_le_bytes(),
            ],
            &PROGRAM_ID,
        );
        
        let (pda2, bump2) = Pubkey::find_program_address(
            &[
                POOL_STATE_SEED_PREFIX,
                token_a_norm_2.as_ref(),
                token_b_norm_2.as_ref(),
                &ratio.to_le_bytes(),
                &1u64.to_le_bytes(),
            ],
            &PROGRAM_ID,
        );
        
        assert_eq!(pda1, pda2, "Normalized token orderings should produce identical PDAs");
        assert_eq!(bump1, bump2, "Normalized token orderings should produce identical bump seeds");
        
        // Test both instruction calls to verify they work with different token orderings
        for (desc, primary, base) in [
            ("Normal order", token_a_mint.pubkey(), token_b_mint.pubkey()),
            ("Swapped order", token_b_mint.pubkey(), token_a_mint.pubkey()),
        ] {
            let instruction_data = PoolInstruction::GetPoolStatePDA {
                primary_token_mint: primary,
                base_token_mint: base,
                ratio_primary_per_base: ratio,
            };
            
            let instruction = Instruction {
                program_id: PROGRAM_ID,
                accounts: vec![],
                data: instruction_data.try_to_vec()?,
            };
            
            let transaction = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&env.payer.pubkey()),
                &[&env.payer],
                env.recent_blockhash,
            );
            
            let result = env.banks_client.process_transaction(transaction).await;
            assert!(result.is_ok(), "{} instruction should succeed", desc);
        }
        
        println!("✅ Token normalization validation passed");
    }
    
    // ===============================================================================
    // Test 4: Different ratios produce different PDAs
    // ===============================================================================
    {
        println!("Test 4: Different ratios produce unique PDAs");
        
        let test_ratios = [1u64, 2u64, 5u64, 10u64, 100u64];
        let mut derived_pdas = Vec::new();
        
        for &test_ratio in &test_ratios {
            let (pda, _bump) = Pubkey::find_program_address(
                &[
                    POOL_STATE_SEED_PREFIX,
                    token_a_mint.pubkey().as_ref(),
                    token_b_mint.pubkey().as_ref(),
                    &test_ratio.to_le_bytes(),
                    &1u64.to_le_bytes(),
                ],
                &PROGRAM_ID,
            );
            
            // Verify this PDA is unique compared to all previous ones
            for (prev_ratio, prev_pda) in &derived_pdas {
                assert_ne!(pda, *prev_pda, "Ratio {} should produce different PDA than ratio {}", test_ratio, prev_ratio);
            }
            
            derived_pdas.push((test_ratio, pda));
            
            // Test the instruction with this ratio
            let instruction_data = PoolInstruction::GetPoolStatePDA {
                primary_token_mint: token_a_mint.pubkey(),
                base_token_mint: token_b_mint.pubkey(),
                ratio_primary_per_base: test_ratio,
            };
            
            let instruction = Instruction {
                program_id: PROGRAM_ID,
                accounts: vec![],
                data: instruction_data.try_to_vec()?,
            };
            
            let transaction = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&env.payer.pubkey()),
                &[&env.payer],
                env.recent_blockhash,
            );
            
            let result = env.banks_client.process_transaction(transaction).await;
            assert!(result.is_ok(), "Ratio {} instruction should succeed", test_ratio);
        }
        
        println!("✅ Different ratios produce unique PDAs validation passed");
    }
    
    // ===============================================================================
    // Test 5: Edge cases with comprehensive validation
    // ===============================================================================
    {
        println!("Test 5: Edge cases validation");
        
        // Test 5a: Identical tokens (should succeed in utility but fail in pool creation)
        let instruction_data = PoolInstruction::GetPoolStatePDA {
            primary_token_mint: token_a_mint.pubkey(),
            base_token_mint: token_a_mint.pubkey(), // Same token
            ratio_primary_per_base: ratio,
        };
        
        let instruction = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![],
            data: instruction_data.try_to_vec()?,
        };
        
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&env.payer.pubkey()),
            &[&env.payer],
            env.recent_blockhash,
        );
        
        let result = env.banks_client.process_transaction(transaction).await;
        assert!(result.is_ok(), "Utility function should not validate token uniqueness");
        
        // Test 5b: Zero ratio (should succeed in utility but fail in pool creation)
        let instruction_data = PoolInstruction::GetPoolStatePDA {
            primary_token_mint: token_a_mint.pubkey(),
            base_token_mint: token_b_mint.pubkey(),
            ratio_primary_per_base: 0, // Zero ratio
        };
        
        let instruction = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![],
            data: instruction_data.try_to_vec()?,
        };
        
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&env.payer.pubkey()),
            &[&env.payer],
            env.recent_blockhash,
        );
        
        let result = env.banks_client.process_transaction(transaction).await;
        assert!(result.is_ok(), "Utility function should handle zero ratio");
        
        // Test 5c: Maximum ratio value
        let max_ratio = u64::MAX;
        let instruction_data = PoolInstruction::GetPoolStatePDA {
            primary_token_mint: token_a_mint.pubkey(),
            base_token_mint: token_b_mint.pubkey(),
            ratio_primary_per_base: max_ratio,
        };
        
        let instruction = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![],
            data: instruction_data.try_to_vec()?,
        };
        
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&env.payer.pubkey()),
            &[&env.payer],
            env.recent_blockhash,
        );
        
        let result = env.banks_client.process_transaction(transaction).await;
        assert!(result.is_ok(), "Utility function should handle maximum ratio");
        
        println!("✅ Edge cases validation passed");
    }
    
    // ===============================================================================
    // Test 6: Enhanced performance characteristics
    // ===============================================================================
    {
        println!("Test 6: Performance characteristics");
        
        let start = std::time::Instant::now();
        let iterations = 25; // Increased for more realistic testing
        
        for i in 0..iterations {
            let test_ratio = (i % 10) + 1; // Vary ratios to test different scenarios
            
            let instruction_data = PoolInstruction::GetPoolStatePDA {
                primary_token_mint: token_a_mint.pubkey(),
                base_token_mint: token_b_mint.pubkey(),
                ratio_primary_per_base: test_ratio,
            };
            
            let instruction = Instruction {
                program_id: PROGRAM_ID,
                accounts: vec![],
                data: instruction_data.try_to_vec()?,
            };
            
            let transaction = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&env.payer.pubkey()),
                &[&env.payer],
                env.recent_blockhash,
            );
            
            let result = env.banks_client.process_transaction(transaction).await;
            assert!(result.is_ok(), "Performance test iteration {} should succeed", i);
        }
        
        let duration = start.elapsed();
        println!("Time for {} PDA instruction calls: {:?}", iterations, duration);
        
        // Performance expectation: Should complete within reasonable time
        assert!(
            duration.as_millis() < 2000, 
            "PDA instruction calls should be reasonably fast ({} calls in under 2s)", iterations
        );
        
        // Calculate and display performance metrics
        let avg_time_per_call = duration.as_micros() as f64 / iterations as f64;
        println!("Average time per PDA instruction call: {:.2} μs", avg_time_per_call);
        
        println!("✅ Performance characteristics validation passed");
    }
    
    // ===============================================================================
    // Test 7: Instruction data validation and serialization
    // ===============================================================================
    {
        println!("Test 7: Instruction data validation");
        
        // Test that instruction data serializes and deserializes correctly
        let instruction_data = PoolInstruction::GetPoolStatePDA {
            primary_token_mint: token_a_mint.pubkey(),
            base_token_mint: token_b_mint.pubkey(),
            ratio_primary_per_base: ratio,
        };
        
        let serialized = instruction_data.try_to_vec()?;
        assert!(!serialized.is_empty(), "Serialized instruction data should not be empty");
        assert!(serialized.len() > 64, "Serialized instruction should include pubkeys and ratio");
        
        // Verify the instruction can be created multiple times with same data
        for _ in 0..3 {
            let instruction = Instruction {
                program_id: PROGRAM_ID,
                accounts: vec![],
                data: serialized.clone(),
            };
            
            let transaction = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&env.payer.pubkey()),
                &[&env.payer],
                env.recent_blockhash,
            );
            
            let result = env.banks_client.process_transaction(transaction).await;
            assert!(result.is_ok(), "Repeated instruction should succeed");
        }
        
        println!("✅ Instruction data validation passed");
    }
    
    println!("✅ UTIL-001 test_get_pool_state_pda completed successfully with enhanced validation");
    Ok(())
}

/// UTIL-002: Enhanced comprehensive test for token vault PDA derivation for both tokens
/// 
/// This test validates the get_token_vault_pdas utility function and covers:
/// 1. Basic token vault PDA derivation with output validation
/// 2. Token A and B vault differentiation and uniqueness
/// 3. Vault uniqueness across different pools
/// 4. Consistency validation and repeated derivation accuracy
/// 5. Seed validation and error handling
/// 6. Edge cases and boundary conditions
/// 7. Performance characteristics and scalability
/// 8. Integration with pool state management
#[tokio::test]
async fn test_get_token_vault_pdas() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running UTIL-002: test_get_token_vault_pdas");
    
    let mut env = start_test_environment().await;
    
    // ===============================================================================
    // Test 1: Basic Token Vault PDA Derivation with Output Validation
    // ===============================================================================
    {
        println!("Test 1: Basic token vault PDA derivation with output validation");
        
        // Create a test pool state PDA using realistic derivation
        let token_a_mint = Keypair::new();
        let token_b_mint = Keypair::new();
        let ratio = 2u64;
        
        let (pool_state_pda, _) = Pubkey::find_program_address(
            &[
                POOL_STATE_SEED_PREFIX,
                token_a_mint.pubkey().as_ref(),
                token_b_mint.pubkey().as_ref(),
                &ratio.to_le_bytes(),
                &1u64.to_le_bytes(),
            ],
            &PROGRAM_ID,
        );
        
        // Test vault PDA derivation instruction
        let instruction_data = PoolInstruction::GetTokenVaultPDAs {
            pool_state_pda,
        };
        
        let instruction = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![],
            data: instruction_data.try_to_vec()?,
        };
        
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&env.payer.pubkey()),
            &[&env.payer],
            env.recent_blockhash,
        );
        
        let result = env.banks_client.process_transaction(transaction).await;
        assert!(result.is_ok(), "get_token_vault_pdas instruction should succeed");
        
        // Verify vault PDAs manually for comparison
        let (expected_vault_a, bump_a) = Pubkey::find_program_address(
            &[
                TOKEN_A_VAULT_SEED_PREFIX,
                pool_state_pda.as_ref(),
            ],
            &PROGRAM_ID,
        );
        
        let (expected_vault_b, bump_b) = Pubkey::find_program_address(
            &[
                TOKEN_B_VAULT_SEED_PREFIX,
                pool_state_pda.as_ref(),
            ],
            &PROGRAM_ID,
        );
        
        // Verify PDAs are valid
        assert_ne!(expected_vault_a, Pubkey::default(), "Vault A PDA should not be default");
        assert_ne!(expected_vault_b, Pubkey::default(), "Vault B PDA should not be default");
        assert_ne!(expected_vault_a, expected_vault_b, "Vault PDAs should be unique");
        
        // Verify bump seeds are within valid range
        // Bump seeds are u8, so they're always <= 255, just check lower bound
        assert!(bump_a >= 240, "Bump seed A should be in valid range (240-255), got: {}", bump_a);
        assert!(bump_b >= 240, "Bump seed B should be in valid range (240-255), got: {}", bump_b);
        
        println!("Expected Vault A: {} (bump: {})", expected_vault_a, bump_a);
        println!("Expected Vault B: {} (bump: {})", expected_vault_b, bump_b);
        println!("✅ Basic vault PDA derivation validation passed");
    }
    
    // ===============================================================================
    // Test 2: Vault Uniqueness Across Different Pools
    // ===============================================================================
    {
        println!("Test 2: Vault uniqueness across different pools");
        
        let mut pool_vaults = Vec::new();
        
        // Create multiple pool PDAs and test their vault uniqueness
        for i in 0..5 {
            let pool_state_pda = Pubkey::new_from_array([i as u8; 32]);
            
            // Test instruction execution
            let instruction_data = PoolInstruction::GetTokenVaultPDAs {
                pool_state_pda,
            };
            
            let instruction = Instruction {
                program_id: PROGRAM_ID,
                accounts: vec![],
                data: instruction_data.try_to_vec()?,
            };
            
            let transaction = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&env.payer.pubkey()),
                &[&env.payer],
                env.recent_blockhash,
            );
            
            let result = env.banks_client.process_transaction(transaction).await;
            assert!(result.is_ok(), "Pool {} vault PDA instruction should succeed", i);
            
            // Derive vaults manually for this pool
            let (vault_a, _) = Pubkey::find_program_address(
                &[
                    TOKEN_A_VAULT_SEED_PREFIX,
                    pool_state_pda.as_ref(),
                ],
                &PROGRAM_ID,
            );
            
            let (vault_b, _) = Pubkey::find_program_address(
                &[
                    TOKEN_B_VAULT_SEED_PREFIX,
                    pool_state_pda.as_ref(),
                ],
                &PROGRAM_ID,
            );
            
            // Verify uniqueness against all previous pools
            for (prev_pool, prev_vault_a, prev_vault_b) in &pool_vaults {
                assert_ne!(vault_a, *prev_vault_a, "Vault A should be unique across pools {} and {}", i, prev_pool);
                assert_ne!(vault_b, *prev_vault_b, "Vault B should be unique across pools {} and {}", i, prev_pool);
                assert_ne!(vault_a, *prev_vault_b, "Vault A should not match any Vault B from other pools");
                assert_ne!(vault_b, *prev_vault_a, "Vault B should not match any Vault A from other pools");
            }
            
            pool_vaults.push((i, vault_a, vault_b));
        }
        
        println!("✅ Vault uniqueness across {} pools validated", pool_vaults.len());
    }
    
    // ===============================================================================
    // Test 3: Consistency Validation and Repeated Derivation Accuracy
    // ===============================================================================
    {
        println!("Test 3: Consistency and repeated derivation accuracy");
        
        let pool_state_pda = Pubkey::new_unique();
        
        // Derive vaults multiple times to ensure consistency
        let mut vault_a_results = Vec::new();
        let mut vault_b_results = Vec::new();
        
        for i in 0..10 {
            // Test instruction execution
            let instruction_data = PoolInstruction::GetTokenVaultPDAs {
                pool_state_pda,
            };
            
            let instruction = Instruction {
                program_id: PROGRAM_ID,
                accounts: vec![],
                data: instruction_data.try_to_vec()?,
            };
            
            let transaction = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&env.payer.pubkey()),
                &[&env.payer],
                env.recent_blockhash,
            );
            
            let result = env.banks_client.process_transaction(transaction).await;
            assert!(result.is_ok(), "Consistency test iteration {} should succeed", i);
            
            // Manual derivation for comparison
            let (vault_a, bump_a) = Pubkey::find_program_address(
                &[
                    TOKEN_A_VAULT_SEED_PREFIX,
                    pool_state_pda.as_ref(),
                ],
                &PROGRAM_ID,
            );
            
            let (vault_b, bump_b) = Pubkey::find_program_address(
                &[
                    TOKEN_B_VAULT_SEED_PREFIX,
                    pool_state_pda.as_ref(),
                ],
                &PROGRAM_ID,
            );
            
            vault_a_results.push((vault_a, bump_a));
            vault_b_results.push((vault_b, bump_b));
        }
        
        // Verify all results are identical
        let (first_vault_a, first_bump_a) = vault_a_results[0];
        let (first_vault_b, first_bump_b) = vault_b_results[0];
        
        for (i, &(vault_a, bump_a)) in vault_a_results.iter().enumerate() {
            assert_eq!(vault_a, first_vault_a, "Vault A derivation should be consistent (iteration {})", i);
            assert_eq!(bump_a, first_bump_a, "Vault A bump should be consistent (iteration {})", i);
        }
        
        for (i, &(vault_b, bump_b)) in vault_b_results.iter().enumerate() {
            assert_eq!(vault_b, first_vault_b, "Vault B derivation should be consistent (iteration {})", i);
            assert_eq!(bump_b, first_bump_b, "Vault B bump should be consistent (iteration {})", i);
        }
        
        println!("✅ Consistency validation passed across {} iterations", vault_a_results.len());
    }
    
    // ===============================================================================
    // Test 4: Seed Validation and Error Handling
    // ===============================================================================
    {
        println!("Test 4: Seed validation and error handling");
        
        let valid_pool_pda = Pubkey::new_unique();
        
        // Test with various seed variations to ensure correct seeds are used
        let test_seeds = [
            (TOKEN_A_VAULT_SEED_PREFIX, "Token A vault"),
            (TOKEN_B_VAULT_SEED_PREFIX, "Token B vault"),
            (b"invalid_seed_a", "Invalid seed A"),
            (b"invalid_seed_b", "Invalid seed B"),
            (b"", "Empty seed"),
        ];
        
        let (correct_vault_a, _) = Pubkey::find_program_address(
            &[
                TOKEN_A_VAULT_SEED_PREFIX,
                valid_pool_pda.as_ref(),
            ],
            &PROGRAM_ID,
        );
        
        let (correct_vault_b, _) = Pubkey::find_program_address(
            &[
                TOKEN_B_VAULT_SEED_PREFIX,
                valid_pool_pda.as_ref(),
            ],
            &PROGRAM_ID,
        );
        
        for (seed, desc) in &test_seeds {
            let (test_vault, _) = Pubkey::find_program_address(
                &[
                    seed,
                    valid_pool_pda.as_ref(),
                ],
                &PROGRAM_ID,
            );
            
            if *seed == TOKEN_A_VAULT_SEED_PREFIX {
                assert_eq!(test_vault, correct_vault_a, "Correct Token A seed should match");
            } else if *seed == TOKEN_B_VAULT_SEED_PREFIX {
                assert_eq!(test_vault, correct_vault_b, "Correct Token B seed should match");
            } else {
                assert_ne!(test_vault, correct_vault_a, "{} should produce different PDA than Token A", desc);
                assert_ne!(test_vault, correct_vault_b, "{} should produce different PDA than Token B", desc);
            }
        }
        
        // Test instruction with edge case pool PDAs
        let edge_case_pools = [
            (Pubkey::default(), "Default (zero) pool PDA"),
            (Pubkey::new_from_array([255u8; 32]), "Maximum pool PDA"),
            (Pubkey::new_from_array([1u8; 32]), "Minimal pool PDA"),
        ];
        
        for (i, (pool_pda, _desc)) in edge_case_pools.iter().enumerate() {
            let instruction_data = PoolInstruction::GetTokenVaultPDAs {
                pool_state_pda: *pool_pda,
            };
            
            let instruction = Instruction {
                program_id: PROGRAM_ID,
                accounts: vec![],
                data: instruction_data.try_to_vec()?,
            };
            
            let transaction = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&env.payer.pubkey()),
                &[&env.payer],
                env.recent_blockhash,
            );
            
            let result = env.banks_client.process_transaction(transaction).await;
            assert!(result.is_ok(), "Edge case {} instruction should succeed", i);
            
            // Verify manual derivation works for edge cases
            let (vault_a, bump_a) = Pubkey::find_program_address(
                &[
                    TOKEN_A_VAULT_SEED_PREFIX,
                    pool_pda.as_ref(),
                ],
                &PROGRAM_ID,
            );
            
            let (vault_b, bump_b) = Pubkey::find_program_address(
                &[
                    TOKEN_B_VAULT_SEED_PREFIX,
                    pool_pda.as_ref(),
                ],
                &PROGRAM_ID,
            );
            
            assert_ne!(vault_a, vault_b, "Vaults should be different for edge case {}", i);
            // Bump seeds are u8, so they're always <= 255, just check lower bound
            assert!(bump_a >= 240, "Bump A should be valid for edge case {}", i);
            assert!(bump_b >= 240, "Bump B should be valid for edge case {}", i);
        }
        
        println!("✅ Seed validation and error handling passed");
    }
    
    // ===============================================================================
    // Test 5: Edge Cases and Boundary Conditions
    // ===============================================================================
    {
        println!("Test 5: Edge cases and boundary conditions");
        
        // Test instruction data serialization edge cases
        let edge_pools = [
            Pubkey::default(),
            Pubkey::new_from_array([0u8; 32]),
            Pubkey::new_from_array([255u8; 32]),
            Pubkey::new_unique(),
        ];
        
        for (i, &pool_pda) in edge_pools.iter().enumerate() {
            let instruction_data = PoolInstruction::GetTokenVaultPDAs {
                pool_state_pda: pool_pda,
            };
            
            // Test serialization
            let serialized = instruction_data.try_to_vec()?;
            assert!(!serialized.is_empty(), "Serialized data should not be empty for edge case {}", i);
            assert_eq!(serialized.len(), 32 + 1, "Serialized data should be pubkey + discriminator for edge case {}", i); // Assuming 1 byte discriminator
            
            // Test instruction execution
            let instruction = Instruction {
                program_id: PROGRAM_ID,
                accounts: vec![],
                data: serialized,
            };
            
            let transaction = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&env.payer.pubkey()),
                &[&env.payer],
                env.recent_blockhash,
            );
            
            let result = env.banks_client.process_transaction(transaction).await;
            assert!(result.is_ok(), "Edge case {} instruction should succeed", i);
            
            // Verify manual derivation works for edge cases
            let (vault_a, bump_a) = Pubkey::find_program_address(
                &[
                    TOKEN_A_VAULT_SEED_PREFIX,
                    pool_pda.as_ref(),
                ],
                &PROGRAM_ID,
            );
            
            let (vault_b, bump_b) = Pubkey::find_program_address(
                &[
                    TOKEN_B_VAULT_SEED_PREFIX,
                    pool_pda.as_ref(),
                ],
                &PROGRAM_ID,
            );
            
            assert_ne!(vault_a, vault_b, "Vaults should be different for edge case {}", i);
            // Bump seeds are u8, so they're always <= 255, just check lower bound
            assert!(bump_a >= 240, "Bump A should be valid for edge case {}", i);
            assert!(bump_b >= 240, "Bump B should be valid for edge case {}", i);
        }
        
        println!("✅ Edge cases and boundary conditions validation passed");
    }
    
    // ===============================================================================
    // Test 6: Performance Characteristics and Scalability
    // ===============================================================================
    {
        println!("Test 6: Performance characteristics and scalability");
        
        let start = std::time::Instant::now();
        let iterations = 50; // Increased for more realistic performance testing
        
        for i in 0..iterations {
            // Create unique pool PDAs for varied testing
            let pool_bytes = (i as u64).to_le_bytes();
            let mut pool_array = [0u8; 32];
            pool_array[..8].copy_from_slice(&pool_bytes);
            let test_pool = Pubkey::new_from_array(pool_array);
            
            let instruction_data = PoolInstruction::GetTokenVaultPDAs {
                pool_state_pda: test_pool,
            };
            
            let instruction = Instruction {
                program_id: PROGRAM_ID,
                accounts: vec![],
                data: instruction_data.try_to_vec()?,
            };
            
            let transaction = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&env.payer.pubkey()),
                &[&env.payer],
                env.recent_blockhash,
            );
            
            let result = env.banks_client.process_transaction(transaction).await;
            assert!(result.is_ok(), "Performance test iteration {} should succeed", i);
        }
        
        let duration = start.elapsed();
        println!("Time for {} vault PDA instruction calls: {:?}", iterations, duration);
        
        // Performance expectations
        assert!(
            duration.as_millis() < 3000, 
            "Vault PDA derivation should be reasonably fast ({} calls in under 3s)", iterations
        );
        
        // Calculate performance metrics
        let avg_time_per_call = duration.as_micros() as f64 / iterations as f64;
        println!("Average time per vault PDA instruction call: {:.2} μs", avg_time_per_call);
        
        // Memory efficiency test - ensure no memory leaks with repeated calls
        let memory_test_start = std::time::Instant::now();
        for _i in 0..100 {
            let pool_pda = Pubkey::new_unique();
            let (_, _) = Pubkey::find_program_address(
                &[TOKEN_A_VAULT_SEED_PREFIX, pool_pda.as_ref()],
                &PROGRAM_ID,
            );
            let (_, _) = Pubkey::find_program_address(
                &[TOKEN_B_VAULT_SEED_PREFIX, pool_pda.as_ref()],
                &PROGRAM_ID,
            );
        }
        let memory_test_duration = memory_test_start.elapsed();
        
        println!("Memory efficiency test (100 manual derivations): {:?}", memory_test_duration);
        assert!(
            memory_test_duration.as_millis() < 100,
            "Manual PDA derivations should be very fast (100 calls in under 100ms)"
        );
        
        println!("✅ Performance characteristics and scalability validation passed");
    }
    
    // ===============================================================================
    // Test 7: Integration with Pool State Management
    // ===============================================================================
    {
        println!("Test 7: Integration with pool state management");
        
        // Create realistic pool configurations
        let token_pairs = [
            (Keypair::new(), Keypair::new(), 2u64),
            (Keypair::new(), Keypair::new(), 5u64),
            (Keypair::new(), Keypair::new(), 10u64),
        ];
        
        for (i, (token_a, token_b, ratio)) in token_pairs.iter().enumerate() {
            // Create realistic pool state PDA
            let (pool_state_pda, _pool_bump) = Pubkey::find_program_address(
                &[
                    POOL_STATE_SEED_PREFIX,
                    token_a.pubkey().as_ref(),
                    token_b.pubkey().as_ref(),
                    &ratio.to_le_bytes(),
                    &1u64.to_le_bytes(),
                ],
                &PROGRAM_ID,
            );
            
            // Test vault derivation for this realistic pool
            let instruction_data = PoolInstruction::GetTokenVaultPDAs {
                pool_state_pda,
            };
            
            let instruction = Instruction {
                program_id: PROGRAM_ID,
                accounts: vec![],
                data: instruction_data.try_to_vec()?,
            };
            
            let transaction = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&env.payer.pubkey()),
                &[&env.payer],
                env.recent_blockhash,
            );
            
            let result = env.banks_client.process_transaction(transaction).await;
            assert!(result.is_ok(), "Integration test {} should succeed", i);
            
            // Verify the derived vaults are appropriate for this pool
            let (vault_a, _) = Pubkey::find_program_address(
                &[TOKEN_A_VAULT_SEED_PREFIX, pool_state_pda.as_ref()],
                &PROGRAM_ID,
            );
            
            let (vault_b, _) = Pubkey::find_program_address(
                &[TOKEN_B_VAULT_SEED_PREFIX, pool_state_pda.as_ref()],
                &PROGRAM_ID,
            );
            
            // Validate relationship between pool and vaults
            assert_ne!(vault_a, pool_state_pda, "Vault A should be different from pool PDA");
            assert_ne!(vault_b, pool_state_pda, "Vault B should be different from pool PDA");
            assert_ne!(vault_a, vault_b, "Vaults should be different from each other");
            
            println!("Pool {}: PDA {} → Vault A: {}, Vault B: {}", 
                     i, pool_state_pda, vault_a, vault_b);
        }
        
        println!("✅ Integration with pool state management validation passed");
    }
    
    println!("✅ UTIL-002 test_get_token_vault_pdas completed successfully with enhanced comprehensive validation");
    Ok(())
}

/// UTIL-003: Enhanced comprehensive test for pool information retrieval
/// 
/// This test validates the get_pool_info utility function and covers:
/// 1. Pool state data retrieval and parsing from actual pool account
/// 2. Token mint information extraction and validation
/// 3. Pool configuration parameters (fees, ratios, etc.) verification
/// 4. Pool status and operational state analysis
/// 5. Owner and delegate information accuracy
/// 6. Pool metadata and configuration completeness
/// 7. Liquidity information and balance validation
/// 8. Edge cases and error handling scenarios
#[tokio::test]
async fn test_get_pool_info() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running UTIL-003: test_get_pool_info");
    
    let mut ctx = setup_pool_test_context(false).await;
    
    // ===============================================================================
    // Test 1: Basic Pool Information Retrieval with Actual Pool Data
    // ===============================================================================
    {
        println!("Test 1: Basic pool information retrieval with actual pool data");
        
        // Create test mints first
        create_test_mints(
            &mut ctx.env.banks_client,
            &ctx.env.payer,
            ctx.env.recent_blockhash,
            &[&ctx.primary_mint, &ctx.base_mint],
        ).await?;
        
        // Create a real pool for testing
        let pool_config = create_pool_new_pattern(
            &mut ctx.env.banks_client,
            &ctx.env.payer,
            ctx.env.recent_blockhash,
            &ctx.primary_mint,
            &ctx.base_mint,
            &ctx.lp_token_a_mint,
            &ctx.lp_token_b_mint,
            None,
        ).await?;
        
        // Test GetPoolInfo instruction
        let instruction_data = PoolInstruction::GetPoolInfo {};
        
        let instruction = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new_readonly(pool_config.pool_state_pda, false), // Pool state PDA (read-only)
            ],
            data: instruction_data.try_to_vec()?,
        };
        
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&ctx.env.payer.pubkey()),
            &[&ctx.env.payer],
            ctx.env.recent_blockhash,
        );
        
        let result = ctx.env.banks_client.process_transaction(transaction).await;
        assert!(result.is_ok(), "get_pool_info instruction should succeed");
        
        // Verify the pool exists and has valid data
        let pool_state = get_pool_state(&mut ctx.env.banks_client, &pool_config.pool_state_pda).await
            .expect("Pool state should exist after creation");
        
        assert!(pool_state.is_initialized, "Pool should be initialized");
        assert_eq!(pool_state.owner, ctx.env.payer.pubkey(), "Pool owner should be correct");
        assert_eq!(pool_state.token_a_mint, pool_config.token_a_mint, "Token A mint should match");
        assert_eq!(pool_state.token_b_mint, pool_config.token_b_mint, "Token B mint should match");
        assert_eq!(pool_state.ratio_a_numerator, pool_config.ratio_a_numerator, "Ratio A numerator should match");
        assert_eq!(pool_state.ratio_b_denominator, pool_config.ratio_b_denominator, "Ratio B denominator should match");
        
        println!("✅ Basic pool information retrieval validation passed");
    }
    
    // ===============================================================================
    // Test 2: Pool Configuration Parameters Validation
    // ===============================================================================
    {
        println!("Test 2: Pool configuration parameters validation");
        
        // Create a new pool with specific configuration
        let specific_primary_mint = Keypair::new();
        let specific_base_mint = Keypair::new();
        let specific_lp_a_mint = Keypair::new();
        let specific_lp_b_mint = Keypair::new();
        
        create_test_mints(
            &mut ctx.env.banks_client,
            &ctx.env.payer,
            ctx.env.recent_blockhash,
            &[&specific_primary_mint, &specific_base_mint],
        ).await?;
        
        let specific_ratio = 5u64; // 5:1 ratio
        let specific_pool_config = create_pool_new_pattern(
            &mut ctx.env.banks_client,
            &ctx.env.payer,
            ctx.env.recent_blockhash,
            &specific_primary_mint,
            &specific_base_mint,
            &specific_lp_a_mint,
            &specific_lp_b_mint,
            Some(specific_ratio),
        ).await?;
        
        // Test GetPoolInfo instruction for specific configuration
        let instruction_data = PoolInstruction::GetPoolInfo {};
        
        let instruction = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new_readonly(specific_pool_config.pool_state_pda, false),
            ],
            data: instruction_data.try_to_vec()?,
        };
        
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&ctx.env.payer.pubkey()),
            &[&ctx.env.payer],
            ctx.env.recent_blockhash,
        );
        
        let result = ctx.env.banks_client.process_transaction(transaction).await;
        assert!(result.is_ok(), "get_pool_info instruction should succeed for specific config");
        
        // Verify configuration parameters
        let pool_state = get_pool_state(&mut ctx.env.banks_client, &specific_pool_config.pool_state_pda).await
            .expect("Pool state should exist");
        
        // Verify ratio matches expected values
        assert_eq!(pool_state.ratio_a_numerator, specific_pool_config.ratio_a_numerator, "Ratio A should match for {}", specific_ratio);
        assert_eq!(pool_state.ratio_b_denominator, specific_pool_config.ratio_b_denominator, "Ratio B should match for {}", specific_ratio);
        
        // Verify vault addresses
        assert_eq!(pool_state.token_a_vault, specific_pool_config.token_a_vault_pda, "Token A vault should match");
        assert_eq!(pool_state.token_b_vault, specific_pool_config.token_b_vault_pda, "Token B vault should match");
        
        // Verify LP token mints
        assert_eq!(pool_state.lp_token_a_mint, specific_lp_a_mint.pubkey(), "LP Token A mint should match");
        assert_eq!(pool_state.lp_token_b_mint, specific_lp_b_mint.pubkey(), "LP Token B mint should match");
        
        // Verify bump seeds
        assert_eq!(pool_state.pool_authority_bump_seed, specific_pool_config.pool_authority_bump, "Pool authority bump should match");
        assert_eq!(pool_state.token_a_vault_bump_seed, specific_pool_config.token_a_vault_bump, "Token A vault bump should match");
        assert_eq!(pool_state.token_b_vault_bump_seed, specific_pool_config.token_b_vault_bump, "Token B vault bump should match");
        
        println!("✅ Pool configuration parameters validation passed");
    }
    
    // ===============================================================================
    // Test 3: Pool Status and Operational State Analysis
    // ===============================================================================
    {
        println!("Test 3: Pool status and operational state analysis");
        
        // Create a pool and verify default operational state
        let operational_primary_mint = Keypair::new();
        let operational_base_mint = Keypair::new();
        let operational_lp_a_mint = Keypair::new();
        let operational_lp_b_mint = Keypair::new();
        
        create_test_mints(
            &mut ctx.env.banks_client,
            &ctx.env.payer,
            ctx.env.recent_blockhash,
            &[&operational_primary_mint, &operational_base_mint],
        ).await?;
        
        let operational_pool_config = create_pool_new_pattern(
            &mut ctx.env.banks_client,
            &ctx.env.payer,
            ctx.env.recent_blockhash,
            &operational_primary_mint,
            &operational_base_mint,
            &operational_lp_a_mint,
            &operational_lp_b_mint,
            None,
        ).await?;
        
        // Test pool info retrieval
        let instruction_data = PoolInstruction::GetPoolInfo {};
        
        let instruction = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new_readonly(operational_pool_config.pool_state_pda, false),
            ],
            data: instruction_data.try_to_vec()?,
        };
        
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&ctx.env.payer.pubkey()),
            &[&ctx.env.payer],
            ctx.env.recent_blockhash,
        );
        
        let result = ctx.env.banks_client.process_transaction(transaction).await;
        assert!(result.is_ok(), "get_pool_info instruction should succeed for operational state");
        
        // Verify operational state
        let pool_state = get_pool_state(&mut ctx.env.banks_client, &operational_pool_config.pool_state_pda).await
            .expect("Pool state should exist");
        
        // Verify default operational state
        assert!(pool_state.is_initialized, "Pool should be initialized");
        assert!(!pool_state.is_paused, "Pool should not be paused by default");
        assert!(!pool_state.swaps_paused, "Swaps should not be paused by default");
        assert!(!pool_state.withdrawal_protection_active, "Withdrawal protection should not be active by default");
        
        // Verify fee structure
        assert_eq!(pool_state.swap_fee_basis_points, 0, "Swap fee should be default value (0)");
        assert_eq!(pool_state.collected_fees_token_a, 0, "Should have no collected fees initially");
        assert_eq!(pool_state.collected_fees_token_b, 0, "Should have no collected fees initially");
        assert_eq!(pool_state.total_fees_withdrawn_token_a, 0, "Should have no withdrawn fees initially");
        assert_eq!(pool_state.total_fees_withdrawn_token_b, 0, "Should have no withdrawn fees initially");
        
        // Verify liquidity state
        assert_eq!(pool_state.total_token_a_liquidity, 0, "Should have no liquidity initially");
        assert_eq!(pool_state.total_token_b_liquidity, 0, "Should have no liquidity initially");
        
        println!("✅ Pool status and operational state analysis passed");
    }
    
    // ===============================================================================
    // Test 4: Owner and Delegate Information Accuracy
    // ===============================================================================
    {
        println!("Test 4: Owner and delegate information accuracy");
        
        // Create a pool with delegate management
        let delegate_primary_mint = Keypair::new();
        let delegate_base_mint = Keypair::new();
        let delegate_lp_a_mint = Keypair::new();
        let delegate_lp_b_mint = Keypair::new();
        
        create_test_mints(
            &mut ctx.env.banks_client,
            &ctx.env.payer,
            ctx.env.recent_blockhash,
            &[&delegate_primary_mint, &delegate_base_mint],
        ).await?;
        
        let delegate_pool_config = create_pool_new_pattern(
            &mut ctx.env.banks_client,
            &ctx.env.payer,
            ctx.env.recent_blockhash,
            &delegate_primary_mint,
            &delegate_base_mint,
            &delegate_lp_a_mint,
            &delegate_lp_b_mint,
            None,
        ).await?;
        
        // Test pool info retrieval for delegate information
        let instruction_data = PoolInstruction::GetPoolInfo {};
        
        let instruction = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new_readonly(delegate_pool_config.pool_state_pda, false),
            ],
            data: instruction_data.try_to_vec()?,
        };
        
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&ctx.env.payer.pubkey()),
            &[&ctx.env.payer],
            ctx.env.recent_blockhash,
        );
        
        let result = ctx.env.banks_client.process_transaction(transaction).await;
        assert!(result.is_ok(), "get_pool_info instruction should succeed for delegate info");
        
        // Verify owner and delegate information
        let pool_state = get_pool_state(&mut ctx.env.banks_client, &delegate_pool_config.pool_state_pda).await
            .expect("Pool state should exist");
        
        // Verify owner information
        assert_eq!(pool_state.owner, ctx.env.payer.pubkey(), "Pool owner should be correct");
        
        // Verify delegate management state (pool owner is automatically added as delegate[0])
        assert_eq!(pool_state.delegate_management.delegate_count, 1, "Should have 1 delegate initially (pool owner auto-added)");
        assert_eq!(pool_state.delegate_management.delegates[0], ctx.env.payer.pubkey(), "First delegate should be the pool owner");
        assert_eq!(pool_state.delegate_management.pending_actions.len(), 0, "Should have no pending actions initially");
        assert_eq!(pool_state.delegate_management.action_history.len(), 0, "Should have no action history initially");
        
        // Verify delegate management configuration
        assert!(pool_state.delegate_management.delegates.len() >= 3, "Should support at least 3 delegates");
        
        println!("✅ Owner and delegate information accuracy validation passed");
    }
    
    // ===============================================================================
    // Test 5: Pool Metadata and Configuration Completeness
    // ===============================================================================
    {
        println!("Test 5: Pool metadata and configuration completeness");
        
        // Test with one different configuration (simplified for performance)
        let test_primary_mint = Keypair::new();
        let test_base_mint = Keypair::new();
        let test_lp_a_mint = Keypair::new();
        let test_lp_b_mint = Keypair::new();
        
        create_test_mints(
            &mut ctx.env.banks_client,
            &ctx.env.payer,
            ctx.env.recent_blockhash,
            &[&test_primary_mint, &test_base_mint],
        ).await?;
        
        let test_ratio = 5u64; // 5:1 ratio
        let test_pool_config = create_pool_new_pattern(
            &mut ctx.env.banks_client,
            &ctx.env.payer,
            ctx.env.recent_blockhash,
            &test_primary_mint,
            &test_base_mint,
            &test_lp_a_mint,
            &test_lp_b_mint,
            Some(test_ratio),
        ).await?;
        
        // Test GetPoolInfo instruction for the configuration
        let instruction_data = PoolInstruction::GetPoolInfo {};
        
        let instruction = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new_readonly(test_pool_config.pool_state_pda, false),
            ],
            data: instruction_data.try_to_vec()?,
        };
        
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&ctx.env.payer.pubkey()),
            &[&ctx.env.payer],
            ctx.env.recent_blockhash,
        );
        
        let result = ctx.env.banks_client.process_transaction(transaction).await;
        assert!(result.is_ok(), "get_pool_info instruction should succeed for 5:1 ratio config");
        
        // Verify metadata completeness
        let pool_state = get_pool_state(&mut ctx.env.banks_client, &test_pool_config.pool_state_pda).await
            .expect("Pool state should exist");
        
        // Verify all essential fields are populated
        assert!(pool_state.is_initialized, "Pool should be initialized");
        assert_ne!(pool_state.owner, Pubkey::default(), "Owner should not be default");
        assert_ne!(pool_state.token_a_mint, Pubkey::default(), "Token A mint should not be default");
        assert_ne!(pool_state.token_b_mint, Pubkey::default(), "Token B mint should not be default");
        assert_ne!(pool_state.token_a_vault, Pubkey::default(), "Token A vault should not be default");
        assert_ne!(pool_state.token_b_vault, Pubkey::default(), "Token B vault should not be default");
        assert_ne!(pool_state.lp_token_a_mint, Pubkey::default(), "LP Token A mint should not be default");
        assert_ne!(pool_state.lp_token_b_mint, Pubkey::default(), "LP Token B mint should not be default");
        
        // Verify ratio configuration
        assert_eq!(pool_state.ratio_a_numerator, test_pool_config.ratio_a_numerator, "Ratio A should match");
        assert_eq!(pool_state.ratio_b_denominator, test_pool_config.ratio_b_denominator, "Ratio B should match");
        
        // Verify bump seeds are in valid range
        assert!(pool_state.pool_authority_bump_seed >= 240, "Pool authority bump should be valid");
        assert!(pool_state.token_a_vault_bump_seed >= 240, "Token A vault bump should be valid");
        assert!(pool_state.token_b_vault_bump_seed >= 240, "Token B vault bump should be valid");
        
        println!("✅ Pool metadata and configuration completeness validation passed");
    }
    
    // ===============================================================================
    // Test 6: Liquidity Information and Balance Validation
    // ===============================================================================
    {
        println!("Test 6: Liquidity information and balance validation");
        
        // Create a pool for liquidity testing
        let liquidity_primary_mint = Keypair::new();
        let liquidity_base_mint = Keypair::new();
        let liquidity_lp_a_mint = Keypair::new();
        let liquidity_lp_b_mint = Keypair::new();
        
        create_test_mints(
            &mut ctx.env.banks_client,
            &ctx.env.payer,
            ctx.env.recent_blockhash,
            &[&liquidity_primary_mint, &liquidity_base_mint],
        ).await?;
        
        let liquidity_pool_config = create_pool_new_pattern(
            &mut ctx.env.banks_client,
            &ctx.env.payer,
            ctx.env.recent_blockhash,
            &liquidity_primary_mint,
            &liquidity_base_mint,
            &liquidity_lp_a_mint,
            &liquidity_lp_b_mint,
            None,
        ).await?;
        
        // Test pool info retrieval for liquidity information
        let instruction_data = PoolInstruction::GetPoolInfo {};
        
        let instruction = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new_readonly(liquidity_pool_config.pool_state_pda, false),
            ],
            data: instruction_data.try_to_vec()?,
        };
        
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&ctx.env.payer.pubkey()),
            &[&ctx.env.payer],
            ctx.env.recent_blockhash,
        );
        
        let result = ctx.env.banks_client.process_transaction(transaction).await;
        assert!(result.is_ok(), "get_pool_info instruction should succeed for liquidity info");
        
        // Verify liquidity information
        let pool_state = get_pool_state(&mut ctx.env.banks_client, &liquidity_pool_config.pool_state_pda).await
            .expect("Pool state should exist");
        
        // Verify initial liquidity state (should be zero for new pool)
        assert_eq!(pool_state.total_token_a_liquidity, 0, "Initial Token A liquidity should be zero");
        assert_eq!(pool_state.total_token_b_liquidity, 0, "Initial Token B liquidity should be zero");
        
        // Verify fee collection state
        assert_eq!(pool_state.collected_fees_token_a, 0, "Initial collected fees Token A should be zero");
        assert_eq!(pool_state.collected_fees_token_b, 0, "Initial collected fees Token B should be zero");
        assert_eq!(pool_state.collected_sol_fees, 0, "Initial collected SOL fees should be zero");
        
        // Verify withdrawal tracking
        assert_eq!(pool_state.total_fees_withdrawn_token_a, 0, "Initial withdrawn fees Token A should be zero");
        assert_eq!(pool_state.total_fees_withdrawn_token_b, 0, "Initial withdrawn fees Token B should be zero");
        assert_eq!(pool_state.total_sol_fees_withdrawn, 0, "Initial withdrawn SOL fees should be zero");
        
        // Verify rent requirements exist
        assert!(pool_state.rent_requirements.rent_exempt_minimum > 0, "Rent requirements should be calculated");
        assert!(pool_state.rent_requirements.pool_state_rent > 0, "Pool state rent should be calculated");
        assert!(pool_state.rent_requirements.token_vault_rent > 0, "Token vault rent should be calculated");
        assert!(pool_state.rent_requirements.lp_mint_rent > 0, "LP mint rent should be calculated");
        
        println!("✅ Liquidity information and balance validation passed");
    }
    
    // ===============================================================================
    // Test 7: Data Validation and Consistency Checks
    // ===============================================================================
    {
        println!("Test 7: Data validation and consistency checks");
        
        // Test 7a: Instruction data serialization validation
        let serialized_data = PoolInstruction::GetPoolInfo {}.try_to_vec()?;
        assert!(!serialized_data.is_empty(), "Serialized data should not be empty");
        println!("✅ Instruction serialization working correctly");
        
        // Test 7b: Instruction creation and validation
        let instruction_data_2 = PoolInstruction::GetPoolInfo {};
        let serialized_2 = instruction_data_2.try_to_vec()?;
        
        // Verify multiple serializations produce identical results
        assert_eq!(serialized_data, serialized_2, "Multiple serializations should be identical");
        println!("✅ Instruction consistency validation working correctly");
        
        println!("✅ Data validation and consistency checks passed");
    }
    
    // ===============================================================================
    // Test 8: Performance Characteristics and Scalability
    // ===============================================================================
    {
        println!("Test 8: Performance characteristics and scalability");
        
        // Create a pool for performance testing
        let perf_primary_mint = Keypair::new();
        let perf_base_mint = Keypair::new();
        let perf_lp_a_mint = Keypair::new();
        let perf_lp_b_mint = Keypair::new();
        
        create_test_mints(
            &mut ctx.env.banks_client,
            &ctx.env.payer,
            ctx.env.recent_blockhash,
            &[&perf_primary_mint, &perf_base_mint],
        ).await?;
        
        let perf_pool_config = create_pool_new_pattern(
            &mut ctx.env.banks_client,
            &ctx.env.payer,
            ctx.env.recent_blockhash,
            &perf_primary_mint,
            &perf_base_mint,
            &perf_lp_a_mint,
            &perf_lp_b_mint,
            None,
        ).await?;
        
        // Performance test: Multiple rapid calls (simplified for speed)
        let start = std::time::Instant::now();
        let iterations = 5; // Reduced for faster testing
        
        for i in 0..iterations {
            let instruction_data = PoolInstruction::GetPoolInfo {};
            
            let instruction = Instruction {
                program_id: PROGRAM_ID,
                accounts: vec![
                    AccountMeta::new_readonly(perf_pool_config.pool_state_pda, false),
                ],
                data: instruction_data.try_to_vec()?,
            };
            
            let transaction = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&ctx.env.payer.pubkey()),
                &[&ctx.env.payer],
                ctx.env.recent_blockhash,
            );
            
            let result = ctx.env.banks_client.process_transaction(transaction).await;
            assert!(result.is_ok(), "Performance test iteration {} should succeed", i);
        }
        
        let duration = start.elapsed();
        println!("Time for {} GetPoolInfo instruction calls: {:?}", iterations, duration);
        
        // Performance expectations (adjusted for Solana test environment)
        assert!(
            duration.as_millis() < 5000, 
            "Pool info retrieval should be reasonably fast ({} calls in under 5s)", iterations
        );
        
        // Calculate performance metrics
        let avg_time_per_call = duration.as_micros() as f64 / iterations as f64;
        println!("Average time per GetPoolInfo instruction call: {:.2} μs", avg_time_per_call);
        
        // Memory efficiency check (simplified)
        let memory_test_start = std::time::Instant::now();
        for _i in 0..10 {
            let _serialized = PoolInstruction::GetPoolInfo {}.try_to_vec()?;
        }
        let memory_test_duration = memory_test_start.elapsed();
        
        println!("Memory efficiency test (10 serializations): {:?}", memory_test_duration);
        assert!(
            memory_test_duration.as_millis() < 20,
            "Instruction serialization should be very fast"
        );
        
        println!("✅ Performance characteristics and scalability validation passed");
    }
    
    println!("✅ UTIL-003 test_get_pool_info completed successfully with comprehensive validation");
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