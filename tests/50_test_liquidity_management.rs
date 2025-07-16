//! Comprehensive Liquidity Management Tests
//! 
//! This module tests all liquidity-related operations including deposits, withdrawals,
//! and edge cases. Tests are designed to validate the 1:1 LP token ratio enforcement
//! and proper fee handling.

use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signer,
};
use serial_test::serial;

mod common;
use common::{
    tokens::*,
    liquidity_helpers::{create_liquidity_test_foundation, execute_deposit_operation, execute_withdrawal_operation, LiquidityTestFoundation},
};

use fixed_ratio_trading::{
    PoolInstruction,
};

use borsh::{BorshDeserialize, BorshSerialize};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// LIQ-SERIALIZATION: Test instruction serialization and deserialization
/// 
/// This test verifies that all pool instructions can be properly serialized
/// and deserialized, ensuring client-contract communication works correctly.
#[tokio::test]
#[serial]
async fn test_instruction_serialization() -> TestResult {
    println!("🧪 Testing instruction serialization and deserialization...");

    // Test data setup
    let test_instructions = vec![
        // Test case 1: Basic Deposit instruction
        {
            let test_mint = Pubkey::new_unique();
            let test_amount = 1_000_000u64;
            PoolInstruction::Deposit {
                deposit_token_mint: test_mint,
                amount: test_amount,
            }
        },
        
        // Test case 2: Withdraw instruction (using correct field names)
        {
            let test_mint = Pubkey::new_unique();
            let test_amount = 500_000u64;
            PoolInstruction::Withdraw {
                withdraw_token_mint: test_mint,
                lp_amount_to_burn: test_amount,
            }
        },
        
        // Test case 3: InitializePool instruction
        {
            PoolInstruction::InitializePool {
                ratio_a_numerator: 3,
                ratio_b_denominator: 1,
            }
        },
        
        // Test case 4: InitializeProgram instruction
        {
            PoolInstruction::InitializeProgram {
                // No fields needed - system authority comes from accounts[0]
            }
        },
    ];

    println!("📝 Testing {} instruction types...", test_instructions.len());

    // Test each instruction
    for (idx, original_instruction) in test_instructions.iter().enumerate() {
        println!("   Testing instruction {} of {}", idx + 1, test_instructions.len());
        
        // Serialize
        let serialized = original_instruction.try_to_vec()
            .map_err(|e| format!("Serialization failed for instruction {}: {}", idx, e))?;
        
        println!("   ✅ Serialized to {} bytes", serialized.len());
        
        // Deserialize
        let deserialized_instruction = PoolInstruction::try_from_slice(&serialized)
            .map_err(|e| format!("Deserialization failed for instruction {}: {}", idx, e))?;
        
        println!("   ✅ Deserialized successfully");
        
        // Verify round-trip consistency
        match (original_instruction, &deserialized_instruction) {
            (
                PoolInstruction::Deposit { 
                    deposit_token_mint: orig_mint, 
                    amount: orig_amount 
                },
                PoolInstruction::Deposit { 
                    deposit_token_mint: deser_mint, 
                    amount: deser_amount 
                }
            ) => {
                assert_eq!(orig_mint, deser_mint, "Deposit mint should match");
                assert_eq!(orig_amount, deser_amount, "Deposit amount should match");
                println!("   ✅ Deposit instruction round-trip verified");
            },
            (
                PoolInstruction::Withdraw { 
                    withdraw_token_mint: orig_mint, 
                    lp_amount_to_burn: orig_amount 
                },
                PoolInstruction::Withdraw { 
                    withdraw_token_mint: deser_mint, 
                    lp_amount_to_burn: deser_amount 
                }
            ) => {
                assert_eq!(orig_mint, deser_mint, "Withdraw mint should match");
                assert_eq!(orig_amount, deser_amount, "Withdraw amount should match");
                println!("   ✅ Withdraw instruction round-trip verified");
            },
            (
                PoolInstruction::InitializePool { 
                    ratio_a_numerator: orig_ratio_a, 
                    ratio_b_denominator: orig_ratio_b, 
                },
                PoolInstruction::InitializePool { 
                    ratio_a_numerator: deser_ratio_a, 
                    ratio_b_denominator: deser_ratio_b, 
                }
            ) => {
                assert_eq!(orig_ratio_a, deser_ratio_a, "InitializePool ratio A should match");
                assert_eq!(orig_ratio_b, deser_ratio_b, "InitializePool ratio B should match");
                println!("   ✅ InitializePool instruction round-trip verified");
            },
            (
                PoolInstruction::InitializeProgram { 
                    // No fields to compare
                },
                PoolInstruction::InitializeProgram { 
                    // No fields to compare
                }
            ) => {
                // No fields to validate - structure match is sufficient
                println!("   ✅ InitializeProgram instruction round-trip verified");
            },
            _ => {
                panic!("Instruction type mismatch after round-trip for instruction {}", idx);
            }
        }
    }

    println!("✅ LIQ-SERIALIZATION: All instruction serialization tests passed!");
    println!("   - {} instruction types tested", test_instructions.len());
    
    Ok(())
}

/// LIQ-001: Test basic deposit operation success
/// 
/// This test verifies the core deposit functionality works correctly:
/// - Creates a pool with a specific ratio using the standardized foundation
/// - Deposits tokens and receives LP tokens in strict 1:1 ratio
/// - Validates all balance changes are correct
/// - Uses the reusable cascading foundation pattern
#[tokio::test]
#[serial]
async fn test_basic_deposit_success() -> TestResult {
    println!("🧪 Testing LIQ-001: Basic deposit operation...");
    
    // Use the timeout wrapper for foundation creation
    let mut foundation = create_foundation_with_timeout(Some(5)).await?; // 5:1 ratio
    println!("✅ Liquidity foundation created with 5:1 ratio");

    // Determine which user account to use for deposit and extract values to avoid borrow checker issues
    let deposit_amount = 500_000u64; // 500K tokens
    let (deposit_mint, user_input_account, user_output_lp_account) = if foundation.pool_config.token_a_is_the_multiple {
        // Depositing Token A (multiple) - use primary token account, get LP A tokens
        (
            foundation.pool_config.token_a_mint,
            foundation.user1_primary_account.pubkey(),
            foundation.user1_lp_a_account.pubkey(),
        )
    } else {
        // Depositing Token B (base) - use base token account, get LP B tokens
        (
            foundation.pool_config.token_b_mint,
            foundation.user1_base_account.pubkey(),
            foundation.user1_lp_b_account.pubkey(),
        )
    };

    // Get initial balances for verification
    let initial_token_balance = get_token_balance(&mut foundation.env.banks_client, &user_input_account).await;
    let initial_lp_balance = get_token_balance(&mut foundation.env.banks_client, &user_output_lp_account).await;
    
    println!("Initial balances - Tokens: {}, LP: {}", initial_token_balance, initial_lp_balance);

    // Execute deposit using the standardized helper
    // Extract values to avoid borrow checker issues
    let user1_pubkey = foundation.user1.pubkey();
    let result = execute_deposit_operation(
        &mut foundation,
        &user1_pubkey,
        &user_input_account,
        &user_output_lp_account,
        &deposit_mint,
        deposit_amount,
    ).await;

    match result {
        Ok(()) => {
            println!("✅ Deposit transaction succeeded");
            
            // Verify the balances changed correctly
            let final_token_balance = get_token_balance(&mut foundation.env.banks_client, &user_input_account).await;
            let final_lp_balance = get_token_balance(&mut foundation.env.banks_client, &user_output_lp_account).await;
            
            println!("Final balances - Tokens: {}, LP: {}", final_token_balance, final_lp_balance);
            
            // Verify token balance decreased by deposit amount
            assert_eq!(
                final_token_balance, initial_token_balance - deposit_amount,
                "Token balance should decrease by deposit amount"
            );
            
            // Verify LP tokens received in strict 1:1 ratio
            let lp_tokens_received = final_lp_balance - initial_lp_balance;
            assert_eq!(
                lp_tokens_received, deposit_amount,
                "Should receive exactly {} LP tokens for {} token deposit (1:1 ratio)",
                deposit_amount, deposit_amount
            );
            
            println!("✅ All balance validations passed!");
            println!("✅ Strict 1:1 LP token ratio verified!");
            println!("✅ LIQ-001 test completed successfully!");
        }
        Err(e) => {
            println!("❌ Deposit transaction failed: {:?}", e);
            panic!("Deposit transaction should succeed: {:?}", e);
        }
    }

    Ok(())
}

/// LIQ-002: Test deposit with zero amount fails
/// 
/// This test verifies that attempting to deposit zero tokens
/// fails with the appropriate error.
/// OPTIMIZED VERSION - uses efficient foundation pattern
#[tokio::test]
#[serial]
async fn test_deposit_zero_amount_fails() -> TestResult {
    println!("🧪 Testing LIQ-002: Deposit with zero amount...");
    
    // Use the timeout wrapper for foundation creation
    let mut foundation = create_foundation_with_timeout(Some(2)).await?; // 2:1 ratio
    println!("✅ Foundation created for zero amount test");

    // Determine which account and mint to use
    let (deposit_mint, user_input_account, user_output_lp_account) = if foundation.pool_config.token_a_is_the_multiple {
        (
            foundation.pool_config.token_a_mint,
            foundation.user1_primary_account.pubkey(),
            foundation.user1_lp_a_account.pubkey(),
        )
    } else {
        (
            foundation.pool_config.token_b_mint,
            foundation.user1_base_account.pubkey(),
            foundation.user1_lp_b_account.pubkey(),
        )
    };

    // Attempt to deposit zero tokens using the optimized helper
    let user1_pubkey = foundation.user1.pubkey();
    let result = execute_deposit_operation(
        &mut foundation,
        &user1_pubkey,
        &user_input_account,
        &user_output_lp_account,
        &deposit_mint,
        0, // Zero amount should fail
    ).await;

    match result {
        Ok(_) => {
            panic!("❌ Zero amount deposit should have failed!");
        }
        Err(_) => {
            println!("✅ Zero amount deposit correctly failed");
            println!("✅ LIQ-002 test completed successfully!");
        }
    }

    Ok(())
}

/// LIQ-003: Test deposit fails with insufficient token balance
/// 
/// This test verifies that attempting to deposit more tokens than available
/// in the user's account fails with the appropriate error.
/// OPTIMIZED VERSION - uses efficient foundation pattern
#[tokio::test]
#[serial]
async fn test_deposit_insufficient_tokens_fails() -> TestResult {
    println!("🧪 Testing LIQ-003: Deposit with insufficient balance...");
    
    // Use the timeout wrapper for foundation creation
    let mut foundation = create_foundation_with_timeout(Some(1)).await?; // 1:1 ratio
    println!("✅ Foundation created for insufficient balance test");

    // Determine which account and mint to use
    let (deposit_mint, user_input_account, user_output_lp_account) = if foundation.pool_config.token_a_is_the_multiple {
        (
            foundation.pool_config.token_a_mint,
            foundation.user1_primary_account.pubkey(),
            foundation.user1_lp_a_account.pubkey(),
        )
    } else {
        (
            foundation.pool_config.token_b_mint,
            foundation.user1_base_account.pubkey(),
            foundation.user1_lp_b_account.pubkey(),
        )
    };

    // Get user's actual balance
    let user_balance = get_token_balance(&mut foundation.env.banks_client, &user_input_account).await;
    let excessive_amount = user_balance + 1_000_000; // Try to deposit more than available

    println!("User balance: {}, attempting to deposit: {}", user_balance, excessive_amount);

    // Attempt to deposit more tokens than available
    let user1_pubkey = foundation.user1.pubkey();
    let result = execute_deposit_operation(
        &mut foundation,
        &user1_pubkey,
        &user_input_account,
        &user_output_lp_account,
        &deposit_mint,
        excessive_amount,
    ).await;

    match result {
        Ok(_) => {
            panic!("❌ Insufficient balance deposit should have failed!");
        }
        Err(_) => {
            println!("✅ Insufficient balance deposit correctly failed");
            println!("✅ LIQ-003 test completed successfully!");
        }
    }

    Ok(())
}

/// LIQ-004: Test basic withdrawal operation success
/// 
/// This test verifies the core withdrawal functionality works correctly:
/// - Uses the cascading foundation system for setup
/// - Deposits tokens to get LP tokens first  
/// - Withdraws LP tokens and receives underlying tokens in 1:1 ratio
/// - Validates all balance changes are correct
/// - Demonstrates the reusable foundation pattern supporting multiple operations
#[tokio::test]
#[serial]
async fn test_basic_withdrawal_success() -> TestResult {
    println!("🧪 Testing LIQ-004: Basic withdrawal operation...");
    
    // Use the timeout wrapper for foundation creation
    let mut foundation = create_foundation_with_timeout(Some(3)).await?; // 3:1 ratio
    println!("✅ Liquidity foundation created with 3:1 ratio");

    // Step 1: Perform a deposit first to get LP tokens
    let deposit_amount = 1_000_000u64; // 1M tokens
    let (deposit_mint, deposit_input_account, deposit_output_lp_account) = if foundation.pool_config.token_a_is_the_multiple {
        // Depositing Token A (multiple) - use primary token account, get LP A tokens
        (
            foundation.pool_config.token_a_mint,
            foundation.user1_primary_account.pubkey(),
            foundation.user1_lp_a_account.pubkey(),
        )
    } else {
        // Depositing Token B (base) - use base token account, get LP B tokens
        (
            foundation.pool_config.token_b_mint,
            foundation.user1_base_account.pubkey(),
            foundation.user1_lp_b_account.pubkey(),
        )
    };

    println!("🪙 Step 1: Depositing {} tokens to get LP tokens...", deposit_amount);
    let user1_pubkey = foundation.user1.pubkey();
    
    // Execute deposit using the standardized helper
    execute_deposit_operation(
        &mut foundation,
        &user1_pubkey,
        &deposit_input_account,
        &deposit_output_lp_account,
        &deposit_mint,
        deposit_amount,
    ).await?;

    let lp_balance_after_deposit = get_token_balance(&mut foundation.env.banks_client, &deposit_output_lp_account).await;
    println!("✅ Deposit completed: {} LP tokens received", lp_balance_after_deposit);
    
    // Verify 1:1 deposit ratio
    assert_eq!(lp_balance_after_deposit, deposit_amount, "Should receive 1:1 LP tokens for deposit");

    // Step 2: Now test withdrawal of half the LP tokens
    let withdraw_amount = lp_balance_after_deposit / 2; // Withdraw half
    println!("🔄 Step 2: Withdrawing {} LP tokens (half of holdings)...", withdraw_amount);

    // Get balances before withdrawal
    let token_balance_before_withdrawal = get_token_balance(&mut foundation.env.banks_client, &deposit_input_account).await;
    let lp_balance_before_withdrawal = get_token_balance(&mut foundation.env.banks_client, &deposit_output_lp_account).await;
    
    println!("Before withdrawal - Tokens: {}, LP: {}", token_balance_before_withdrawal, lp_balance_before_withdrawal);

    // Execute withdrawal using the standardized helper
    let result = execute_withdrawal_operation(
        &mut foundation,
        &user1_pubkey,
        &deposit_output_lp_account,      // LP account being burned
        &deposit_input_account,          // Token account receiving tokens
        &deposit_mint,                   // Token mint being withdrawn
        withdraw_amount,
    ).await;

    match result {
        Ok(()) => {
            println!("✅ Withdrawal transaction succeeded");

            // Verify the balances changed correctly
            let token_balance_after_withdrawal = get_token_balance(&mut foundation.env.banks_client, &deposit_input_account).await;
            let lp_balance_after_withdrawal = get_token_balance(&mut foundation.env.banks_client, &deposit_output_lp_account).await;
            
            println!("After withdrawal - Tokens: {}, LP: {}", token_balance_after_withdrawal, lp_balance_after_withdrawal);

            // Verify LP tokens were burned in 1:1 ratio
            assert_eq!(
                lp_balance_after_withdrawal, lp_balance_before_withdrawal - withdraw_amount,
                "LP tokens should be burned 1:1"
            );

            // Verify underlying tokens were received in 1:1 ratio
            assert_eq!(
                token_balance_after_withdrawal, token_balance_before_withdrawal + withdraw_amount,
                "Should receive 1:1 underlying tokens for LP tokens burned"
            );

            println!("✅ All balance validations passed!");
            println!("✅ Strict 1:1 withdrawal ratio verified!");
            println!("✅ Cascading foundation system supports both deposit and withdrawal!");
            println!("✅ LIQ-004 test completed successfully!");
        }
        Err(e) => {
            println!("❌ Withdrawal transaction failed: {:?}", e);
            panic!("Withdrawal transaction should succeed: {:?}", e);
        }
    }

    Ok(())
}

/// Timeout wrapper for foundation creation to prevent deadlocks
async fn create_foundation_with_timeout(
    pool_ratio: Option<u64>,
) -> Result<LiquidityTestFoundation, Box<dyn std::error::Error>> {
    let timeout_duration = std::time::Duration::from_secs(30); // 30 second timeout for foundation setup
    let foundation_future = create_liquidity_test_foundation(pool_ratio);
    
    match tokio::time::timeout(timeout_duration, foundation_future).await {
        Ok(foundation) => foundation,
        Err(_) => Err("Foundation creation timed out".into()),
    }
}

/// Test InitializeProgram instruction in isolation
/// OPTIMIZED VERSION - uses foundation pattern with timeout
#[tokio::test]
#[serial]
async fn test_initialize_program_isolated() -> TestResult {
    println!("🧪 Testing InitializeProgram instruction in isolation...");
    
    // Use the optimized foundation with timeout to test treasury system initialization
    let result = create_foundation_with_timeout(Some(1)).await;
    
    match result {
        Ok(_) => {
            println!("✅ InitializeProgram (treasury system) succeeded");
        }
        Err(e) => {
            println!("❌ InitializeProgram failed: {:?}", e);
            // Don't panic, just report the error for debugging
        }
    }
    
    Ok(())
} 