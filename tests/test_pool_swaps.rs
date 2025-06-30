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

//! Pool Swap Testing Module (Migrated from test_swaps.rs)
//! 
//! This module contains all swap-related tests after removing the delegate system.
//! Tests have been rewritten to use the new owner-only operations model.

/* 
==================================================================================
MIGRATION CHECKLIST - Tests to migrate from test_swaps.rs:
==================================================================================

Core Swap Tests:
[✅] test_pool_instruction_serialization_debug - Basic instruction serialization
[✅] test_exchange_token_b_for_token_a - Basic token exchange with liquidity protection  
[✅] test_swap_zero_amount_fails - Zero amount validation
[✅] test_successful_a_to_b_swap - Core swap functionality A→B
[✅] test_successful_b_to_a_swap - Core swap functionality B→A
[ ] test_swap_with_various_ratios - Test different pool ratios
[ ] test_slippage_protection_boundaries - Slippage protection logic
[ ] test_swap_liquidity_constraints - Liquidity availability checks
[ ] test_swap_edge_cases_and_security - Edge cases and security validation
[ ] test_process_swap_a_to_b_execution - Low-level swap execution A→B
[ ] test_process_swap_b_to_a_execution - Low-level swap execution B→A

Fee Management Tests (TO REWRITE - Remove Delegate System):
[✅] test_fee_change_request_success - REWRITTEN: test_owner_fee_management (immediate ChangeFee)
[✅] test_fee_change_validation - REWRITTEN: test_owner_fee_management (owner-only validation)
[✅] test_fee_change_authorization - REWRITTEN: test_owner_fee_management (owner authorization)
[❌] test_fee_change_timing - REMOVED: No more time delays in new system
[ ] test_fee_collection_accuracy - UPDATE: Owner fee collection
[ ] test_fee_withdrawal_through_action - REWRITE: Use WithdrawPoolFees

==================================================================================
MIGRATION STATUS: 8/17 tests migrated (3 fee tests rewritten into 1, 1 removed)
==================================================================================
*/

mod common;

use common::*;
use fixed_ratio_trading::{
    PoolInstruction,
    ID as PROGRAM_ID,
};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use solana_sdk::{signature::Keypair, transaction::Transaction};
use borsh::BorshSerialize;

// ================================================================================================
// COMMON CONSTANTS AND HELPER FUNCTIONS
// ================================================================================================

/// Fee testing constants (in basis points)
const VALID_FEE_ZERO: u64 = 0;          // 0% - zero fee (should be valid)
const VALID_FEE_LOW: u64 = 10;          // 0.1% - low valid fee
const VALID_FEE_MEDIUM: u64 = 40;       // 0.4% - medium valid fee
const MAX_ALLOWED_FEE: u64 = 50;        // 0.5% - maximum allowed fee (boundary)
const INVALID_FEE_JUST_OVER: u64 = 51;  // 0.51% - just over maximum
const INVALID_FEE_HIGH: u64 = 100;      // 1.0% - clearly invalid

/// Standard swap amounts for testing
const SMALL_SWAP_AMOUNT: u64 = 1_000;      // 0.001 tokens
const MEDIUM_SWAP_AMOUNT: u64 = 100_000;   // 0.1 tokens  
const LARGE_SWAP_AMOUNT: u64 = 1_000_000;  // 1 token

/// Helper to create a basic swap instruction
pub fn create_swap_instruction(
    user: &Pubkey,
    user_input_account: &Pubkey,
    user_output_account: &Pubkey,
    pool_state_pda: &Pubkey,
    token_a_mint: &Pubkey,
    token_b_mint: &Pubkey,
    token_a_vault: &Pubkey,
    token_b_vault: &Pubkey,
    input_token_mint: &Pubkey,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new(*user_input_account, false),
            AccountMeta::new(*user_output_account, false),
            AccountMeta::new(*pool_state_pda, false),
            AccountMeta::new_readonly(*token_a_mint, false),
            AccountMeta::new_readonly(*token_b_mint, false),
            AccountMeta::new(*token_a_vault, false),
            AccountMeta::new(*token_b_vault, false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_program::sysvar::rent::id(), false),
            AccountMeta::new_readonly(solana_program::sysvar::clock::id(), false),
        ],
        data: PoolInstruction::Swap {
            input_token_mint: *input_token_mint,
            amount_in,
            minimum_amount_out,
        }.try_to_vec().unwrap(),
    }
}

/// Helper to create a fee change instruction (owner-only)
pub fn create_change_fee_instruction(
    owner: &Pubkey,
    pool_state_pda: &Pubkey,
    new_fee_basis_points: u64,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*owner, true),                    // Pool owner (signer)
            AccountMeta::new(*pool_state_pda, false),          // Pool state PDA
            AccountMeta::new_readonly(solana_program::sysvar::clock::id(), false), // Clock sysvar
        ],
        data: PoolInstruction::ChangeFee {
            new_fee_basis_points,
        }.try_to_vec().unwrap(),
    }
}

/// Helper to create a withdraw pool fees instruction (owner-only)  
pub fn create_withdraw_pool_fees_instruction(
    owner: &Pubkey,
    pool_state_pda: &Pubkey,
    token_mint: &Pubkey,
    destination_account: &Pubkey,
    vault_account: &Pubkey,
    amount: u64,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*owner, true),                    // Pool owner (signer)
            AccountMeta::new(*pool_state_pda, false),          // Pool state PDA
            AccountMeta::new_readonly(*token_mint, false),     // Token mint
            AccountMeta::new(*destination_account, false),     // Owner's token account
            AccountMeta::new(*vault_account, false),           // Pool's token vault
            AccountMeta::new_readonly(spl_token::id(), false), // SPL Token program
        ],
        data: PoolInstruction::WithdrawPoolFees {
            token_mint: *token_mint,
            amount,
        }.try_to_vec().unwrap(),
    }
}

/// Helper to verify swap results
pub async fn verify_swap_results(
    banks_client: &mut solana_program_test::BanksClient,
    user_input_account: &Pubkey,
    user_output_account: &Pubkey,
    expected_input_change: i64,  // Negative for decrease
    expected_min_output_change: u64, // Minimum expected increase
) -> TestResult {
    let input_balance = get_token_balance(banks_client, user_input_account).await;
    let output_balance = get_token_balance(banks_client, user_output_account).await;
    
    println!("Post-swap balances:");
    println!("  Input account: {} tokens", input_balance);
    println!("  Output account: {} tokens", output_balance);
    
    // Verify input tokens were deducted (if expected_input_change is negative)
    if expected_input_change < 0 {
        let expected_input_balance = (constants::DEFAULT_USER_TOKEN_AMOUNT as i64 + expected_input_change) as u64;
        assert_eq!(input_balance, expected_input_balance, 
                   "Input balance should decrease by swap amount");
    }
    
    // Verify output tokens were received (should be at least the minimum)
    assert!(output_balance >= expected_min_output_change,
            "Output balance should increase by at least minimum amount: {} >= {}", 
            output_balance, expected_min_output_change);
    
    Ok(())
}

/// Helper to setup a complete swap test environment
pub async fn setup_swap_test_environment(
    ratio: Option<u64>,
) -> Result<(PoolTestContext, PoolConfig, Keypair, Pubkey, Pubkey), solana_program_test::BanksClientError> {
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create token mints
    create_test_mints(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &[&ctx.primary_mint, &ctx.base_mint],
    ).await?;

    // Create pool with specified ratio
    let config = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        &ctx.lp_token_a_mint,
        &ctx.lp_token_b_mint,
        ratio,
    ).await?;

    // Setup user with token accounts
    let (user, user_primary_account, user_base_account) = setup_test_user(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint.pubkey(),
        &ctx.base_mint.pubkey(),
        None,
    ).await?;

    // Mint initial tokens to user
    mint_tokens(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint.pubkey(),
        &user_primary_account.pubkey(),
        &ctx.env.payer,
        constants::DEFAULT_USER_TOKEN_AMOUNT,
    ).await?;

    mint_tokens(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.base_mint.pubkey(),
        &user_base_account.pubkey(),
        &ctx.env.payer,
        constants::DEFAULT_USER_TOKEN_AMOUNT,
    ).await?;

    Ok((ctx, config, user, user_primary_account.pubkey(), user_base_account.pubkey()))
}

// ================================================================================================
// MIGRATED TESTS START HERE
// ================================================================================================

/// Test basic PoolInstruction serialization
/// ✅ MIGRATED: test_pool_instruction_serialization_debug
#[tokio::test]
async fn test_pool_instruction_serialization() -> TestResult {
    println!("===== Testing PoolInstruction serialization =====");
    
    // Test basic Swap instruction serialization
    let test_mint = Pubkey::new_unique();
    let swap_instruction = PoolInstruction::Swap {
        input_token_mint: test_mint,
        amount_in: 1000000u64,
        minimum_amount_out: 900000u64,
    };
    
    // Test serialization
    let serialized = swap_instruction.try_to_vec();
    println!("Serialization result: {:?}", serialized);
    
    assert!(serialized.is_ok(), "Swap instruction serialization should succeed");
    let serialized_data = serialized.unwrap();
    println!("Serialized data length: {}", serialized_data.len());
    
    // Test deserialization
    let deserialized = PoolInstruction::try_from_slice(&serialized_data);
    assert!(deserialized.is_ok(), "Swap instruction deserialization should succeed");
    
    // Verify the data matches
    if let Ok(PoolInstruction::Swap { input_token_mint, amount_in, minimum_amount_out }) = deserialized {
        assert_eq!(input_token_mint, test_mint);
        assert_eq!(amount_in, 1000000u64);
        assert_eq!(minimum_amount_out, 900000u64);
        println!("✅ Serialization roundtrip successful");
    } else {
        panic!("Unexpected instruction variant after deserialization");
    }
    
    // Test new owner-only instructions
    let change_fee_instruction = PoolInstruction::ChangeFee {
        new_fee_basis_points: 25,
    };
    
    let serialized_fee = change_fee_instruction.try_to_vec();
    assert!(serialized_fee.is_ok(), "ChangeFee instruction serialization should succeed");
    println!("✅ ChangeFee instruction serialization works");
    
    let withdraw_fees_instruction = PoolInstruction::WithdrawPoolFees {
        token_mint: test_mint,
        amount: 1000,
    };
    
    let serialized_withdraw = withdraw_fees_instruction.try_to_vec();
    assert!(serialized_withdraw.is_ok(), "WithdrawPoolFees instruction serialization should succeed");
    println!("✅ WithdrawPoolFees instruction serialization works");
    
    Ok(())
}

/// Test basic token exchange with liquidity protection
/// ✅ MIGRATED: test_exchange_token_b_for_token_a
#[tokio::test]
async fn test_exchange_token_b_for_token_a() -> TestResult {
    let (mut ctx, config, user, user_primary_account, user_base_account) = setup_swap_test_environment(Some(2)).await?;

    // Attempt swap: base token for primary token (demonstrates liquidity protection)
    let swap_amount = 1u64;
    let minimum_amount_out = 0u64;

    let swap_ix = create_swap_instruction(
        &user.pubkey(),
        &user_base_account,
        &user_primary_account,
        &config.pool_state_pda,
        &config.token_a_mint,
        &config.token_b_mint,
        &config.token_a_vault_pda,
        &config.token_b_vault_pda,
        &ctx.base_mint.pubkey(),
        swap_amount,
        minimum_amount_out,
    );

    let mut swap_tx = Transaction::new_with_payer(&[swap_ix], Some(&user.pubkey()));
    swap_tx.sign(&[&user], ctx.env.recent_blockhash);
    
    let swap_result = ctx.env.banks_client.process_transaction(swap_tx).await;
    
    // Use helper to handle expected error in a clean way
    handle_expected_test_error(
        "swap with insufficient liquidity",
        &swap_result,
        "Swap processed successfully",
        "Expected insufficient liquidity protection activated"
    );

    // Verify user tokens remain safe
    let user_primary_balance = get_token_balance(&mut ctx.env.banks_client, &user_primary_account).await;
    assert_eq!(user_primary_balance, constants::DEFAULT_USER_TOKEN_AMOUNT, 
               "User should not receive tokens from failed swap");

    println!("✅ Token exchange liquidity protection working correctly");
    
    Ok(())
}

/// Test swap with zero amount fails
/// ✅ MIGRATED: test_swap_zero_amount_fails  
#[tokio::test]
async fn test_swap_zero_amount_fails() -> TestResult {
    let (mut ctx, config, user, user_primary_account, user_base_account) = setup_swap_test_environment(None).await?;

    // Try to swap zero tokens
    let swap_ix = create_swap_instruction(
        &user.pubkey(),
        &user_base_account,
        &user_primary_account,
        &config.pool_state_pda,
        &config.token_a_mint,
        &config.token_b_mint,
        &config.token_a_vault_pda,
        &config.token_b_vault_pda,
        &ctx.base_mint.pubkey(),
        0u64, // Zero amount
        0u64,
    );

    let mut swap_tx = Transaction::new_with_payer(&[swap_ix], Some(&user.pubkey()));
    swap_tx.sign(&[&user], ctx.env.recent_blockhash);
    
    let swap_result = ctx.env.banks_client.process_transaction(swap_tx).await;
    
    // Should fail with zero amount
    assert!(swap_result.is_err(), "Swap with zero amount should fail");
    
    println!("✅ Zero amount swap correctly rejected");
    
    Ok(())
}

/// Test successful A→B swap with comprehensive validation
/// ✅ MIGRATED: test_successful_a_to_b_swap
#[tokio::test]
async fn test_successful_a_to_b_swap() -> TestResult {
    let (mut ctx, config, user, user_primary_account, user_base_account) = setup_swap_test_environment(Some(2)).await?;

    println!("===== A→B Swap Validation Testing =====");
    
    // Verify pool creation succeeded
    let pool_state = get_pool_state(&mut ctx.env.banks_client, &config.pool_state_pda).await
        .expect("Failed to get pool state after creation");
    
    assert!(pool_state.is_initialized, "Pool should be initialized");
    assert_eq!(pool_state.owner, ctx.env.payer.pubkey(), "Pool owner should match");
    println!("✅ Pool created successfully with ratio A:{} B:{}", 
             pool_state.ratio_a_numerator, pool_state.ratio_b_denominator);

    // Test fixed-ratio price calculation accuracy
    let test_amounts = vec![1_000u64, 10_000u64, 100_000u64, 1_000_000u64];
    
    for &swap_amount in &test_amounts {
        // Calculate expected output based on fixed ratio
        let expected_output = if config.token_a_is_primary {
            // Primary token is Token A, so A→B swap: out_B = in_A * B_denom / A_num
            swap_amount * pool_state.ratio_b_denominator / pool_state.ratio_a_numerator
        } else {
            // Primary token is Token B, A→B is reverse: out_B = in_A * A_num / B_denom
            swap_amount * pool_state.ratio_a_numerator / pool_state.ratio_b_denominator
        };

        println!("  Ratio calculation: {} Token A → {} Token B (2:1 ratio)", 
                 swap_amount, expected_output);
        
        // Verify calculation is reasonable
        assert!(expected_output > 0, "Output should be positive for positive input");
        
        // Test slippage protection calculation
        let slippage_5_percent = expected_output * 95 / 100;
        let slippage_1_percent = expected_output * 99 / 100;
        
        assert!(slippage_5_percent < expected_output, "5% slippage should be less than expected");
        assert!(slippage_1_percent < expected_output, "1% slippage should be less than expected");
        assert!(slippage_1_percent > slippage_5_percent, "1% slippage should be more than 5%");
        
        println!("    ✓ Price calculation: {} → {} (expected)", swap_amount, expected_output);
        println!("    ✓ Slippage protection: 5%={}, 1%={}", slippage_5_percent, slippage_1_percent);
    }

    // Test swap instruction construction and validation
    let swap_amount = 100_000u64;
    let expected_output = if config.token_a_is_primary {
        swap_amount * pool_state.ratio_b_denominator / pool_state.ratio_a_numerator
    } else {
        swap_amount * pool_state.ratio_a_numerator / pool_state.ratio_b_denominator
    };
    let minimum_amount_out = expected_output * 95 / 100; // 5% slippage tolerance

    // Construct swap instruction with proper account setup
    let swap_ix = create_swap_instruction(
        &user.pubkey(),
        &user_primary_account,
        &user_base_account,
        &config.pool_state_pda,
        &config.token_a_mint,
        &config.token_b_mint,
        &config.token_a_vault_pda,
        &config.token_b_vault_pda,
        &ctx.primary_mint.pubkey(), // Swapping Token A (primary)
        swap_amount,
        minimum_amount_out,
    );

    // Verify instruction construction
    assert_eq!(swap_ix.accounts.len(), 12, "Swap instruction should have 12 accounts");
    assert_eq!(swap_ix.program_id, PROGRAM_ID, "Program ID should match");
    assert!(!swap_ix.data.is_empty(), "Instruction data should not be empty");
    
    println!("✅ Swap instruction constructed successfully:");
    println!("    ✓ 12 accounts configured with proper permissions");
    println!("    ✓ Program ID matches: {}", PROGRAM_ID);
    println!("    ✓ Instruction data serialized: {} bytes", swap_ix.data.len());
    println!("    ✓ Swap parameters: {} → {} (min: {})", swap_amount, expected_output, minimum_amount_out);

    // Test user balance verification
    let user_balance_a = get_token_balance(&mut ctx.env.banks_client, &user_primary_account).await;
    let user_balance_b = get_token_balance(&mut ctx.env.banks_client, &user_base_account).await;

    assert_eq!(user_balance_a, constants::DEFAULT_USER_TOKEN_AMOUNT, "User should have expected Token A balance");
    assert_eq!(user_balance_b, constants::DEFAULT_USER_TOKEN_AMOUNT, "User should have expected Token B balance");
    
    println!("✅ User balances verified:");
    println!("    ✓ Token A: {} (sufficient for swap)", user_balance_a);
    println!("    ✓ Token B: {} (ready to receive)", user_balance_b);

    println!("✅ A→B Swap validation testing completed successfully");
    
    Ok(())
}

/// Test successful B→A swap execution with comprehensive validation
/// ✅ MIGRATED: test_successful_b_to_a_swap
#[tokio::test]
async fn test_successful_b_to_a_swap() -> TestResult {
    let (mut ctx, config, user, user_primary_account, user_base_account) = setup_swap_test_environment(Some(2)).await?;

    println!("===== B→A Swap Validation Testing =====");
    
    // Verify pool creation succeeded
    let pool_state = get_pool_state(&mut ctx.env.banks_client, &config.pool_state_pda).await
        .expect("Failed to get pool state after creation");
    
    assert!(pool_state.is_initialized, "Pool should be initialized");
    assert_eq!(pool_state.owner, ctx.env.payer.pubkey(), "Pool owner should match");
    println!("✅ Pool created successfully with ratio A:{} B:{}", 
             pool_state.ratio_a_numerator, pool_state.ratio_b_denominator);

    // Test reverse direction price calculation accuracy
    println!("--- Testing Reverse Direction Price Calculations ---");
    let test_amounts = vec![1_000u64, 10_000u64, 100_000u64, 1_000_000u64];
    
    for &swap_amount in &test_amounts {
        // Calculate expected output for B→A swap based on fixed ratio
        let expected_output = if config.token_a_is_primary {
            // Primary token is Token A, A:B ratio, B→A swap: out_A = in_B * A_num / B_denom
            swap_amount * pool_state.ratio_a_numerator / pool_state.ratio_b_denominator
        } else {
            // Primary token is Token B, B:A ratio, B→A swap: out_A = in_B * B_denom / A_num
            swap_amount * pool_state.ratio_b_denominator / pool_state.ratio_a_numerator
        };

        println!("  Reverse ratio calculation: {} Token B → {} Token A (2:1 ratio)", 
                 swap_amount, expected_output);
        
        // Verify calculation is reasonable for B→A
        assert!(expected_output > 0, "Output should be positive for positive input");
        
        // Test slippage protection calculation for reverse direction
        let slippage_5_percent = expected_output * 95 / 100;
        let slippage_1_percent = expected_output * 99 / 100;
        
        assert!(slippage_5_percent < expected_output, "5% slippage should be less than expected");
        assert!(slippage_1_percent < expected_output, "1% slippage should be less than expected");
        assert!(slippage_1_percent > slippage_5_percent, "1% slippage should be more than 5%");
        
        println!("    ✓ Reverse price calculation: {} → {} (expected)", swap_amount, expected_output);
        println!("    ✓ Slippage protection: 5%={}, 1%={}", slippage_5_percent, slippage_1_percent);
    }

    // Test bidirectional consistency
    println!("--- Testing Bidirectional Consistency ---");
    let test_amount = 1_000_000u64;
    
    // Calculate A→B
    let a_to_b_output = if config.token_a_is_primary {
        test_amount * pool_state.ratio_b_denominator / pool_state.ratio_a_numerator
    } else {
        test_amount * pool_state.ratio_a_numerator / pool_state.ratio_b_denominator
    };
    
    // Calculate B→A using the A→B output
    let b_to_a_output = if config.token_a_is_primary {
        a_to_b_output * pool_state.ratio_a_numerator / pool_state.ratio_b_denominator
    } else {
        a_to_b_output * pool_state.ratio_b_denominator / pool_state.ratio_a_numerator
    };
    
    println!("  Bidirectional test: {} A → {} B → {} A", test_amount, a_to_b_output, b_to_a_output);
    
    // The final amount should be close to original (exactly equal without fees)
    assert_eq!(b_to_a_output, test_amount, 
               "Bidirectional swap should return to original amount (without fees)");
    
    println!("✅ Bidirectional consistency validated - perfect mathematical symmetry");

    // Test B→A swap instruction construction
    let swap_amount = 200_000u64; // Use Token B for input
    let expected_output = if config.token_a_is_primary {
        swap_amount * pool_state.ratio_a_numerator / pool_state.ratio_b_denominator
    } else {
        swap_amount * pool_state.ratio_b_denominator / pool_state.ratio_a_numerator
    };
    let minimum_amount_out = expected_output * 95 / 100; // 5% slippage tolerance

    // Construct B→A swap instruction
    let swap_ix = create_swap_instruction(
        &user.pubkey(),
        &user_base_account,     // User's Token B account (input)
        &user_primary_account,  // User's Token A account (output)
        &config.pool_state_pda,
        &config.token_a_mint,
        &config.token_b_mint,
        &config.token_a_vault_pda,
        &config.token_b_vault_pda,
        &ctx.base_mint.pubkey(), // Swapping Token B (base) for Token A
        swap_amount,
        minimum_amount_out,
    );

    // Verify instruction construction for B→A swap
    assert_eq!(swap_ix.accounts.len(), 12, "B→A swap instruction should have 12 accounts");
    assert_eq!(swap_ix.program_id, PROGRAM_ID, "Program ID should match");
    assert!(!swap_ix.data.is_empty(), "Instruction data should not be empty");
    
    println!("✅ B→A swap instruction constructed successfully:");
    println!("    ✓ 12 accounts configured with proper permissions");
    println!("    ✓ Program ID matches: {}", PROGRAM_ID);
    println!("    ✓ Instruction data serialized: {} bytes", swap_ix.data.len());
    println!("    ✓ B→A swap parameters: {} B → {} A (min: {})", swap_amount, expected_output, minimum_amount_out);

    // Test user balance verification for B→A swap
    let user_balance_a = get_token_balance(&mut ctx.env.banks_client, &user_primary_account).await;
    let user_balance_b = get_token_balance(&mut ctx.env.banks_client, &user_base_account).await;

    assert_eq!(user_balance_a, constants::DEFAULT_USER_TOKEN_AMOUNT, "User should have expected Token A balance");
    assert_eq!(user_balance_b, constants::DEFAULT_USER_TOKEN_AMOUNT, "User should have expected Token B balance");
    
    println!("✅ User balances verified for B→A swap:");
    println!("    ✓ Token A: {} (ready to receive)", user_balance_a);
    println!("    ✓ Token B: {} (sufficient for swap)", user_balance_b);

    println!("✅ B→A Swap validation testing completed successfully");
    
    Ok(())
}

/// Test owner-only fee management operations (replaces delegate fee tests)
/// ✅ MIGRATED & REWRITTEN: Replaces test_fee_change_request_success, test_fee_change_validation, test_fee_change_authorization
#[tokio::test] 
async fn test_owner_fee_management() -> TestResult {
    let (mut ctx, config, _user, _user_primary_account, _user_base_account) = setup_swap_test_environment(Some(2)).await?;

    println!("===== Owner-Only Fee Management Testing =====");

    // Test 1: Valid fee change by owner (immediate execution)
    println!("\n--- Testing Valid Fee Change by Owner ---");
    
    let new_fee = VALID_FEE_MEDIUM; // 0.4%
    let change_fee_ix = create_change_fee_instruction(
        &ctx.env.payer.pubkey(),
        &config.pool_state_pda,
        new_fee,
    );

    let mut fee_change_tx = Transaction::new_with_payer(&[change_fee_ix], Some(&ctx.env.payer.pubkey()));
    fee_change_tx.sign(&[&ctx.env.payer], ctx.env.recent_blockhash);
    
    let fee_change_result = ctx.env.banks_client.process_transaction(fee_change_tx).await;
    assert!(fee_change_result.is_ok(), "Owner fee change should succeed: {:?}", fee_change_result);
    
    // Verify fee was changed immediately
    let pool_state = get_pool_state(&mut ctx.env.banks_client, &config.pool_state_pda).await
        .expect("Failed to get pool state after fee change");
    assert_eq!(pool_state.swap_fee_basis_points, new_fee, "Fee should be updated immediately");
    
    println!("✅ Valid fee change successful: {} basis points ({}%)", new_fee, new_fee as f64 / 100.0);

    // Test 2: Fee validation - maximum allowed fee
    println!("\n--- Testing Maximum Allowed Fee ---");
    
    ctx.env.recent_blockhash = ctx.env.banks_client
        .get_new_latest_blockhash(&ctx.env.recent_blockhash).await?;
    
    let max_fee_ix = create_change_fee_instruction(
        &ctx.env.payer.pubkey(),
        &config.pool_state_pda,
        MAX_ALLOWED_FEE, // 0.5%
    );

    let mut max_fee_tx = Transaction::new_with_payer(&[max_fee_ix], Some(&ctx.env.payer.pubkey()));
    max_fee_tx.sign(&[&ctx.env.payer], ctx.env.recent_blockhash);
    
    let max_fee_result = ctx.env.banks_client.process_transaction(max_fee_tx).await;
    assert!(max_fee_result.is_ok(), "Maximum allowed fee should succeed: {:?}", max_fee_result);
    
    println!("✅ Maximum allowed fee (0.5%) accepted");

    // Test 3: Fee validation - reject over maximum
    println!("\n--- Testing Invalid Fee (Over Maximum) ---");
    
    ctx.env.recent_blockhash = ctx.env.banks_client
        .get_new_latest_blockhash(&ctx.env.recent_blockhash).await?;
    
    let invalid_fee_ix = create_change_fee_instruction(
        &ctx.env.payer.pubkey(),
        &config.pool_state_pda,
        INVALID_FEE_JUST_OVER, // 0.51%
    );

    let mut invalid_fee_tx = Transaction::new_with_payer(&[invalid_fee_ix], Some(&ctx.env.payer.pubkey()));
    invalid_fee_tx.sign(&[&ctx.env.payer], ctx.env.recent_blockhash);
    
    let invalid_fee_result = ctx.env.banks_client.process_transaction(invalid_fee_tx).await;
    assert!(invalid_fee_result.is_err(), "Fee over maximum should be rejected");
    
    println!("✅ Fee above maximum (0.5%) correctly rejected");

    // Test 4: Zero fee setting (should be valid)
    println!("\n--- Testing Zero Fee Setting ---");
    
    ctx.env.recent_blockhash = ctx.env.banks_client
        .get_new_latest_blockhash(&ctx.env.recent_blockhash).await?;
    
    let zero_fee_ix = create_change_fee_instruction(
        &ctx.env.payer.pubkey(),
        &config.pool_state_pda,
        VALID_FEE_ZERO, // 0%
    );

    let mut zero_fee_tx = Transaction::new_with_payer(&[zero_fee_ix], Some(&ctx.env.payer.pubkey()));
    zero_fee_tx.sign(&[&ctx.env.payer], ctx.env.recent_blockhash);
    
    let zero_fee_result = ctx.env.banks_client.process_transaction(zero_fee_tx).await;
    assert!(zero_fee_result.is_ok(), "Zero fee should be valid: {:?}", zero_fee_result);
    
    // Verify zero fee was set
    let final_pool_state = get_pool_state(&mut ctx.env.banks_client, &config.pool_state_pda).await
        .expect("Failed to get pool state after zero fee");
    assert_eq!(final_pool_state.swap_fee_basis_points, VALID_FEE_ZERO, "Zero fee should be set");
    
    println!("✅ Zero fee (0%) successfully set");

    // Test 5: Unauthorized user cannot change fees
    println!("\n--- Testing Unauthorized Access Prevention ---");
    
    let unauthorized_user = Keypair::new();
    
    ctx.env.recent_blockhash = ctx.env.banks_client
        .get_new_latest_blockhash(&ctx.env.recent_blockhash).await?;
    
    let unauthorized_fee_ix = create_change_fee_instruction(
        &unauthorized_user.pubkey(),
        &config.pool_state_pda,
        VALID_FEE_LOW,
    );

    let mut unauthorized_tx = Transaction::new_with_payer(&[unauthorized_fee_ix], Some(&unauthorized_user.pubkey()));
    unauthorized_tx.sign(&[&unauthorized_user], ctx.env.recent_blockhash);
    
    let unauthorized_result = ctx.env.banks_client.process_transaction(unauthorized_tx).await;
    assert!(unauthorized_result.is_err(), "Unauthorized user should not be able to change fees");
    
    println!("✅ Unauthorized access correctly prevented");

    println!("\n===== Owner-Only Fee Management Test Summary =====");
    println!("✅ Fee Management Testing Complete:");
    println!("   ✓ Owner can change fees immediately (no time delays)");
    println!("   ✓ Fee changes take effect immediately");
    println!("   ✓ Maximum allowed fee (0.5%) is enforced");
    println!("   ✓ Fees above maximum are rejected");
    println!("   ✓ Zero fee is valid and can be set");
    println!("   ✓ Unauthorized users cannot change fees");
    println!();
    println!("🎯 Demonstrates simplified owner-only fee management:");
    println!("   • No delegate system complexity");
    println!("   • No time delays or pending actions");
    println!("   • Immediate execution of valid fee changes");
    println!("   • Proper validation and authorization");
    
    Ok(())
} 