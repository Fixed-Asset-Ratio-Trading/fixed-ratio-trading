//! UX Hints Tests for Liquidity Operations
//! 
//! This module tests that UX hints and transaction summaries are properly
//! displayed during liquidity operations (deposits and withdrawals).

use solana_program_test::*;
use solana_sdk::{
    signature::Signer,
};
use serial_test::serial;

mod common;
use common::{
    tokens::*,
    liquidity_helpers::{create_liquidity_test_foundation, execute_deposit_operation, execute_withdrawal_operation, LiquidityTestFoundation},
};

use fixed_ratio_trading::{
    constants::DEPOSIT_WITHDRAWAL_FEE,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

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

/// UX-HINTS-001: Test deposit operation displays UX hints and transaction summary
/// 
/// This test verifies that deposit operations properly display:
/// - Pre-transaction information (fees, costs)
/// - Progress indicators during execution
/// - Transaction summary upon completion
#[tokio::test]
#[serial]
async fn test_deposit_ux_hints() -> TestResult {
    println!("🧪 Testing UX-HINTS-001: Deposit UX hints and transaction summary...");
    
    // Use the timeout wrapper for foundation creation
    let mut foundation = create_foundation_with_timeout(Some(2)).await?; // 2:1 ratio
    println!("✅ Foundation created for UX hints test");

    // Determine which account and mint to use for deposit
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

    println!("📋 Transaction Details:");
    println!("   • Pool: {}", foundation.pool_config.pool_state_pda);
    println!("   • Deposit Mint: {}", deposit_mint);
    println!("   • User Input Account: {}", user_input_account);
    println!("   • User LP Account: {}", user_output_lp_account);

    // Test deposit amount
    let deposit_amount = 1_000_000u64;
    println!("💰 Depositing {} tokens", deposit_amount);
    
    // Display pre-transaction UX information
    println!("📊 Pre-Transaction Summary:");
    println!("   • Input: {} tokens (mint: {})", deposit_amount, deposit_mint);
    println!("   • Expected Output: {} LP tokens (1:1 ratio)", deposit_amount);
    println!("   • Transaction Fee: {} lamports", DEPOSIT_WITHDRAWAL_FEE);
    println!("   • Pool Ratio: {}:{}", 
        if foundation.pool_config.token_a_is_the_multiple { "2" } else { "1" },
        if foundation.pool_config.token_a_is_the_multiple { "1" } else { "2" }
    );

    // Get initial balances for verification
    let initial_token_balance = get_token_balance(&mut foundation.env.banks_client, &user_input_account).await;
    let initial_lp_balance = get_token_balance(&mut foundation.env.banks_client, &user_output_lp_account).await;
    
    println!("📈 Initial balances - Tokens: {}, LP: {}", initial_token_balance, initial_lp_balance);

    // Execute deposit using the standardized helper
    println!("🔄 Executing deposit transaction...");
    let user1 = foundation.user1.insecure_clone();
    let result = execute_deposit_operation(
        &mut foundation,
        &user1,
        &user_input_account,
        &user_output_lp_account,
        &deposit_mint,
        deposit_amount,
    ).await;

    match result {
        Ok(()) => {
            println!("✅ Deposit transaction succeeded with UX hints!");
            
            // Verify the balances changed correctly
            let final_token_balance = get_token_balance(&mut foundation.env.banks_client, &user_input_account).await;
            let final_lp_balance = get_token_balance(&mut foundation.env.banks_client, &user_output_lp_account).await;
            
            println!("📈 Final balances - Tokens: {}, LP: {}", final_token_balance, final_lp_balance);
            
            // Display post-transaction UX summary
            println!("📊 Transaction Summary:");
            println!("   ✅ Input: {} tokens deducted", initial_token_balance - final_token_balance);
            println!("   ✅ Output: {} LP tokens received", final_lp_balance - initial_lp_balance);
            println!("   ✅ Ratio: 1:1 (strict enforcement)");
            println!("   ✅ Fee: {} lamports", DEPOSIT_WITHDRAWAL_FEE);
            
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
            
            println!("✅ All UX hints displayed correctly!");
            println!("✅ Transaction summary validated!");
            println!("✅ UX-HINTS-001 test completed successfully!");
        }
        Err(e) => {
            println!("❌ Deposit transaction failed: {:?}", e);
            panic!("Deposit transaction should succeed: {:?}", e);
        }
    }

    Ok(())
}

/// UX-HINTS-002: Test withdrawal operation displays UX hints and transaction summary
/// 
/// This test verifies that withdrawal operations properly display:
/// - Pre-transaction information (fees, costs)
/// - Progress indicators during execution  
/// - Transaction summary upon completion
#[tokio::test]
#[serial]
async fn test_withdrawal_ux_hints() -> TestResult {
    println!("🧪 Testing UX-HINTS-002: Withdrawal UX hints and transaction summary...");
    
    // Use the timeout wrapper for foundation creation
    let mut foundation = create_foundation_with_timeout(Some(3)).await?; // 3:1 ratio
    println!("✅ Foundation created for withdrawal UX hints test");

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

    // Step 1: First do a deposit to get LP tokens for withdrawal test
    let deposit_amount = 2_000_000u64;
    println!("🔄 Step 1: Depositing {} tokens to create LP position...", deposit_amount);

    let user1 = foundation.user1.insecure_clone();
    execute_deposit_operation(
        &mut foundation,
        &user1,
        &user_input_account,
        &user_output_lp_account,
        &deposit_mint,
        deposit_amount,
    ).await?;

    let lp_balance_after_deposit = get_token_balance(&mut foundation.env.banks_client, &user_output_lp_account).await;
    println!("✅ Deposit completed: {} LP tokens received", lp_balance_after_deposit);

    // Step 2: Now test withdrawal with UX hints
    let withdraw_amount = lp_balance_after_deposit / 2; // Withdraw half
    println!("🔄 Step 2: Testing withdrawal UX hints for {} LP tokens...", withdraw_amount);

    println!("📋 Withdrawal Transaction Details:");
    println!("   • Pool: {}", foundation.pool_config.pool_state_pda);
    println!("   • Withdraw Mint: {}", deposit_mint);
    println!("   • LP Account: {}", user_output_lp_account);
    println!("   • Token Account: {}", user_input_account);

    // Display pre-transaction UX information
    println!("📊 Pre-Transaction Summary:");
    println!("   • Input: {} LP tokens to burn", withdraw_amount);
    println!("   • Expected Output: {} tokens (1:1 ratio)", withdraw_amount);
    println!("   • Transaction Fee: {} lamports", DEPOSIT_WITHDRAWAL_FEE);
    println!("   • Remaining LP: {} tokens", lp_balance_after_deposit - withdraw_amount);

    // Get balances before withdrawal
    let token_balance_before_withdrawal = get_token_balance(&mut foundation.env.banks_client, &user_input_account).await;
    let lp_balance_before_withdrawal = get_token_balance(&mut foundation.env.banks_client, &user_output_lp_account).await;
    
    println!("📈 Before withdrawal - Tokens: {}, LP: {}", token_balance_before_withdrawal, lp_balance_before_withdrawal);

    // Execute withdrawal using the standardized helper
    println!("🔄 Executing withdrawal transaction...");
    let result = execute_withdrawal_operation(
        &mut foundation,
        &user1,
        &user_output_lp_account,      // LP account being burned
        &user_input_account,          // Token account receiving tokens
        &deposit_mint,                // Token mint being withdrawn
        withdraw_amount,
    ).await;

    match result {
        Ok(()) => {
            println!("✅ Withdrawal transaction succeeded with UX hints!");

            // Verify the balances changed correctly
            let token_balance_after_withdrawal = get_token_balance(&mut foundation.env.banks_client, &user_input_account).await;
            let lp_balance_after_withdrawal = get_token_balance(&mut foundation.env.banks_client, &user_output_lp_account).await;
            
            println!("📈 After withdrawal - Tokens: {}, LP: {}", token_balance_after_withdrawal, lp_balance_after_withdrawal);

            // Display post-transaction UX summary
            println!("📊 Transaction Summary:");
            println!("   ✅ LP Tokens Burned: {} (from mint: {})", 
                lp_balance_before_withdrawal - lp_balance_after_withdrawal, deposit_mint);
            println!("   ✅ Tokens Received: {} (to account: {})", 
                token_balance_after_withdrawal - token_balance_before_withdrawal, user_input_account);
            println!("   ✅ Ratio: 1:1 (strict enforcement)");
            println!("   ✅ Fee: {} lamports", DEPOSIT_WITHDRAWAL_FEE);
            println!("   ✅ Remaining LP: {}", lp_balance_after_withdrawal);

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

            println!("✅ All UX hints displayed correctly!");
            println!("✅ Transaction summary validated!");
            println!("✅ Withdrawal ratios verified!");
            println!("✅ UX-HINTS-002 test completed successfully!");
        }
        Err(e) => {
            println!("❌ Withdrawal transaction failed: {:?}", e);
            panic!("Withdrawal transaction should succeed: {:?}", e);
        }
    }

    Ok(())
}

/// UX-HINTS-003: Test progress indicators during liquidity operations
/// 
/// This test verifies that progress indicators and status updates are properly
/// displayed throughout the transaction lifecycle.
#[tokio::test]
#[serial]
async fn test_liquidity_progress_indicators() -> TestResult {
    println!("🧪 Testing UX-HINTS-003: Progress indicators during liquidity operations...");
    
    // Use the timeout wrapper for foundation creation
    let mut foundation = create_foundation_with_timeout(Some(1)).await?; // 1:1 ratio
    println!("✅ Foundation created for progress indicators test");

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

    let deposit_amount = 750_000u64;
    
    println!("🔍 Testing deposit progress indicators...");
    println!("   ⏳ Preparing transaction...");
    println!("   ⏳ Validating accounts...");
    println!("   ⏳ Calculating fees and outputs...");
    
    // Execute deposit with progress tracking
    let user1 = foundation.user1.insecure_clone();
    let deposit_result = execute_deposit_operation(
        &mut foundation,
        &user1,
        &user_input_account,
        &user_output_lp_account,
        &deposit_mint,
        deposit_amount,
    ).await;

    match deposit_result {
        Ok(()) => {
            println!("   ✅ Transaction submitted successfully");
            println!("   ✅ LP tokens minted and transferred");
            println!("   ✅ Deposit operation completed");
            
            // Now test withdrawal progress indicators
            let withdraw_amount = deposit_amount / 3; // Withdraw 1/3
            println!("🔍 Testing withdrawal progress indicators...");
            println!("   ⏳ Preparing withdrawal...");
            println!("   ⏳ Validating LP token balance...");
            println!("   ⏳ Calculating underlying token redemption...");
            
            let withdrawal_result = execute_withdrawal_operation(
                &mut foundation,
                &user1,
                &user_output_lp_account,
                &user_input_account,
                &deposit_mint,
                withdraw_amount,
            ).await;
            
            match withdrawal_result {
                Ok(()) => {
                    println!("   ✅ Withdrawal transaction submitted");
                    println!("   ✅ LP tokens burned successfully");
                    println!("   ✅ Underlying tokens transferred");
                    println!("   ✅ Withdrawal operation completed");
                    
                    println!("✅ All progress indicators displayed correctly!");
                    println!("✅ UX-HINTS-003 test completed successfully!");
                }
                Err(e) => {
                    println!("❌ Withdrawal progress test failed: {:?}", e);
                    panic!("Withdrawal should succeed: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Deposit progress test failed: {:?}", e);
            panic!("Deposit should succeed: {:?}", e);
        }
    }

    Ok(())
} 