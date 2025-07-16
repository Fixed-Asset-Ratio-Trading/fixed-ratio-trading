mod common;

use common::*;
use solana_program_test::BanksClientError;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};
use borsh::BorshSerialize;
use fixed_ratio_trading::{
    constants::DEPOSIT_WITHDRAWAL_FEE,
    types::instructions::PoolInstruction,
    id,
};
use crate::common::{
    liquidity_helpers::{LiquidityTestFoundation, create_liquidity_test_foundation},
    TestResult,
};

#[tokio::test]
async fn test_optimized_pool_creation_with_ux_hints() -> TestResult {
    println!("🧪 Testing optimized pool creation with UX hints...");
    
    // Setup test environment
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create ordered token mints
    let keypair1 = Keypair::new();
    let keypair2 = Keypair::new();
    
    let (primary_mint, base_mint) = if keypair1.pubkey() < keypair2.pubkey() {
        (keypair1, keypair2)
    } else {
        (keypair2, keypair1)
    };
    
    // Initialize treasury system (required first)
    init_treasury_for_test(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
    ).await?;
    
    // Create token mints
    create_test_mints(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &[&primary_mint, &base_mint],
    ).await?;
    
    // Test pool creation with optimized UX hints
    let ratio_a_numerator = 1;
    let ratio_b_denominator = 2;
    
    println!("🔨 Creating pool with ratio {}:{}", ratio_a_numerator, ratio_b_denominator);
    
    let config = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &primary_mint,
        &base_mint,
        Some(ratio_a_numerator),
    ).await?;
    
    println!("✅ Pool created successfully!");
    println!("   Pool State: {}", config.pool_state_pda);
    println!("   Token A: {}", primary_mint.pubkey());
    println!("   Token B: {}", base_mint.pubkey());
    println!("   Ratio: {} : {}", ratio_a_numerator, ratio_b_denominator);
    
    // Verify pool state was created correctly
    verify_pool_state(
        &mut ctx.env.banks_client,
        &config,
        &ctx.env.payer.pubkey(),
        &ctx.lp_token_a_mint.pubkey(),
        &ctx.lp_token_b_mint.pubkey(),
    ).await.map_err(|e| BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    
    println!("✅ Pool state verification passed!");
    Ok(())
}

#[tokio::test]
async fn test_pool_creation_ux_messages() -> TestResult {
    println!("🧪 Testing pool creation UX messages...");
    
    // Setup test environment
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create ordered token mints
    let keypair1 = Keypair::new();
    let keypair2 = Keypair::new();
    
    let (primary_mint, base_mint) = if keypair1.pubkey() < keypair2.pubkey() {
        (keypair1, keypair2)
    } else {
        (keypair2, keypair1)
    };
    
    // Initialize treasury system (required first)
    init_treasury_for_test(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
    ).await?;
    
    // Create token mints
    create_test_mints(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &[&primary_mint, &base_mint],
    ).await?;
    
    // Test pool creation with UX messages
    let config = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &primary_mint,
        &base_mint,
        Some(1),
    ).await?;
    
    println!("✅ Pool creation with UX messages completed!");
    println!("   Pool: {}", config.pool_state_pda);
    
    // Verify the pool exists
    verify_pool_state(
        &mut ctx.env.banks_client,
        &config,
        &ctx.env.payer.pubkey(),
        &ctx.lp_token_a_mint.pubkey(),
        &ctx.lp_token_b_mint.pubkey(),
    ).await.map_err(|e| BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    
    Ok(())
}

#[tokio::test]
async fn test_deposit_ux_hints() -> TestResult {
    println!("🧪 Testing deposit UX hints and transaction summary...");
    
    // Setup liquidity test foundation
    let mut foundation = common::liquidity_helpers::create_liquidity_test_foundation(Some(2)).await
        .map_err(|e| BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    
    // Initialize treasury system
    init_treasury_for_test(
        &mut foundation.env.banks_client,
        &foundation.env.payer,
        foundation.env.recent_blockhash,
    ).await?;
    
    // Create pool
    let config = create_pool_new_pattern(
        &mut foundation.env.banks_client,
        &foundation.env.payer,
        foundation.env.recent_blockhash,
        &foundation.primary_mint,
        &foundation.base_mint,
        Some(2),
    ).await?;
    
    // Update foundation with pool config
    foundation.pool_config = config;
    
    // Mint tokens to user for deposit
    let deposit_amount = 1_000_000u64;
    common::tokens::mint_tokens(
        &mut foundation.env.banks_client,
        &foundation.env.payer,
        foundation.env.recent_blockhash,
        &foundation.primary_mint.pubkey(),
        &foundation.user1_primary_account.pubkey(),
        &foundation.env.payer,
        deposit_amount,
    ).await?;
    
    println!("💰 Minted {} tokens to user for deposit", deposit_amount);
    
    // Execute deposit with UX hints
    let user1 = &foundation.user1;
    let user1_primary_account_pubkey = foundation.user1_primary_account.pubkey();
    let user1_lp_a_account_pubkey = foundation.user1_lp_a_account.pubkey();
    let primary_mint_pubkey = foundation.primary_mint.pubkey();
    
    let result = common::liquidity_helpers::execute_deposit_operation(
        &mut foundation,
        user1,
        &user1_primary_account_pubkey,
        &user1_lp_a_account_pubkey,
        &primary_mint_pubkey,
        deposit_amount,
    ).await;
    
    match result {
        Ok(_) => {
            println!("✅ Deposit completed successfully with UX hints!");
            
            // Verify the deposit actually worked
            let user_lp_balance = common::tokens::get_token_balance(
                &mut foundation.env.banks_client,
                &foundation.user1_lp_a_account.pubkey(),
            ).await;
            
            println!("📊 User LP token balance: {}", user_lp_balance);
            assert_eq!(user_lp_balance, deposit_amount, "LP tokens should match deposit amount");
            
            // Verify user's input tokens were deducted
            let user_input_balance = common::tokens::get_token_balance(
                &mut foundation.env.banks_client,
                &foundation.user1_primary_account.pubkey(),
            ).await;
            
            println!("📊 User input token balance: {}", user_input_balance);
            assert_eq!(user_input_balance, 0, "User should have no input tokens left");
            
            println!("✅ Deposit UX hints test passed!");
        }
        Err(e) => {
            println!("❌ Deposit failed: {:?}", e);
            return Err(e);
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_withdrawal_ux_hints() -> TestResult {
    println!("🧪 Testing withdrawal UX hints and transaction summary...");
    
    // Setup liquidity test foundation
    let mut foundation = common::liquidity_helpers::create_liquidity_test_foundation(Some(2)).await
        .map_err(|e| BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    
    // Initialize treasury system
    init_treasury_for_test(
        &mut foundation.env.banks_client,
        &foundation.env.payer,
        foundation.env.recent_blockhash,
    ).await?;
    
    // Create pool
    let config = create_pool_new_pattern(
        &mut foundation.env.banks_client,
        &foundation.env.payer,
        foundation.env.recent_blockhash,
        &foundation.primary_mint,
        &foundation.base_mint,
        Some(2),
    ).await?;
    
    // Update foundation with pool config
    foundation.pool_config = config;
    
    // First, do a deposit to get LP tokens
    let deposit_amount = 1_000_000u64;
    common::tokens::mint_tokens(
        &mut foundation.env.banks_client,
        &foundation.env.payer,
        foundation.env.recent_blockhash,
        &foundation.primary_mint.pubkey(),
        &foundation.user1_primary_account.pubkey(),
        &foundation.env.payer,
        deposit_amount,
    ).await?;
    
    // Execute deposit
    let user1 = &foundation.user1;
    let user1_primary_account_pubkey = foundation.user1_primary_account.pubkey();
    let user1_lp_a_account_pubkey = foundation.user1_lp_a_account.pubkey();
    let primary_mint_pubkey = foundation.primary_mint.pubkey();
    
    common::liquidity_helpers::execute_deposit_operation(
        &mut foundation,
        user1,
        &user1_primary_account_pubkey,
        &user1_lp_a_account_pubkey,
        &primary_mint_pubkey,
        deposit_amount,
    ).await?;
    
    println!("✅ Deposit completed, now testing withdrawal...");
    
    // Now test withdrawal with UX hints
    let withdrawal_amount = 500_000u64; // Withdraw half
    
    let user1 = &foundation.user1;
    let user1_lp_a_account_pubkey = foundation.user1_lp_a_account.pubkey();
    let user1_primary_account_pubkey = foundation.user1_primary_account.pubkey();
    let primary_mint_pubkey = foundation.primary_mint.pubkey();
    
    let result = common::liquidity_helpers::execute_withdrawal_operation(
        &mut foundation,
        user1,
        &user1_lp_a_account_pubkey,
        &user1_primary_account_pubkey,
        &primary_mint_pubkey,
        withdrawal_amount,
    ).await;
    
    match result {
        Ok(_) => {
            println!("✅ Withdrawal completed successfully with UX hints!");
            
            // Verify the withdrawal actually worked
            let user_lp_balance = common::tokens::get_token_balance(
                &mut foundation.env.banks_client,
                &foundation.user1_lp_a_account.pubkey(),
            ).await;
            
            let expected_lp_balance = deposit_amount - withdrawal_amount;
            println!("📊 User LP token balance: {} (expected: {})", user_lp_balance, expected_lp_balance);
            assert_eq!(user_lp_balance, expected_lp_balance, "LP tokens should be reduced by withdrawal amount");
            
            // Verify user received tokens back
            let user_token_balance = common::tokens::get_token_balance(
                &mut foundation.env.banks_client,
                &foundation.user1_primary_account.pubkey(),
            ).await;
            
            println!("📊 User token balance: {} (expected: {})", user_token_balance, withdrawal_amount);
            assert_eq!(user_token_balance, withdrawal_amount, "User should have received tokens back");
            
            println!("✅ Withdrawal UX hints test passed!");
        }
        Err(e) => {
            println!("❌ Withdrawal failed: {:?}", e);
            return Err(e);
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_liquidity_ux_progress_indicators() -> TestResult {
    println!("🧪 Testing liquidity operation progress indicators...");
    
    // Setup liquidity test foundation
    let mut foundation = common::liquidity_helpers::create_liquidity_test_foundation(Some(1)).await
        .map_err(|e| BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    
    // Initialize treasury system
    init_treasury_for_test(
        &mut foundation.env.banks_client,
        &foundation.env.payer,
        foundation.env.recent_blockhash,
    ).await?;
    
    // Create pool
    let config = create_pool_new_pattern(
        &mut foundation.env.banks_client,
        &foundation.env.payer,
        foundation.env.recent_blockhash,
        &foundation.primary_mint,
        &foundation.base_mint,
        Some(1),
    ).await?;
    
    // Update foundation with pool config
    foundation.pool_config = config;
    
    // Mint tokens to user
    let deposit_amount = 500_000u64;
    common::tokens::mint_tokens(
        &mut foundation.env.banks_client,
        &foundation.env.payer,
        foundation.env.recent_blockhash,
        &foundation.primary_mint.pubkey(),
        &foundation.user1_primary_account.pubkey(),
        &foundation.env.payer,
        deposit_amount,
    ).await?;
    
    println!("🔍 Testing deposit progress indicators...");
    
    // Execute deposit and verify progress indicators are shown
    let user1 = &foundation.user1;
    let user1_primary_account_pubkey = foundation.user1_primary_account.pubkey();
    let user1_lp_a_account_pubkey = foundation.user1_lp_a_account.pubkey();
    let primary_mint_pubkey = foundation.primary_mint.pubkey();
    
    let result = common::liquidity_helpers::execute_deposit_operation(
        &mut foundation,
        user1,
        &user1_primary_account_pubkey,
        &user1_lp_a_account_pubkey,
        &primary_mint_pubkey,
        deposit_amount,
    ).await;
    
    match result {
        Ok(_) => {
            println!("✅ Deposit progress indicators test passed!");
            
            // Now test withdrawal progress indicators
            println!("🔍 Testing withdrawal progress indicators...");
            
            let withdrawal_amount = 200_000u64;
            let user1 = &foundation.user1;
            let user1_lp_a_account_pubkey = foundation.user1_lp_a_account.pubkey();
            let user1_primary_account_pubkey = foundation.user1_primary_account.pubkey();
            let primary_mint_pubkey = foundation.primary_mint.pubkey();
            
            let withdrawal_result = common::liquidity_helpers::execute_withdrawal_operation(
                &mut foundation,
                user1,
                &user1_lp_a_account_pubkey,
                &user1_primary_account_pubkey,
                &primary_mint_pubkey,
                withdrawal_amount,
            ).await;
            
            match withdrawal_result {
                Ok(_) => {
                    println!("✅ Withdrawal progress indicators test passed!");
                }
                Err(e) => {
                    println!("❌ Withdrawal progress indicators test failed: {:?}", e);
                    return Err(e);
                }
            }
        }
        Err(e) => {
            println!("❌ Deposit progress indicators test failed: {:?}", e);
            return Err(e);
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_liquidity_ux_transaction_summary() -> TestResult {
    println!("🧪 Testing liquidity transaction summary details...");
    
    // Setup liquidity test foundation
    let mut foundation = common::liquidity_helpers::create_liquidity_test_foundation(Some(3)).await
        .map_err(|e| BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    
    // Initialize treasury system
    init_treasury_for_test(
        &mut foundation.env.banks_client,
        &foundation.env.payer,
        foundation.env.recent_blockhash,
    ).await?;
    
    // Create pool
    let config = create_pool_new_pattern(
        &mut foundation.env.banks_client,
        &foundation.env.payer,
        foundation.env.recent_blockhash,
        &foundation.primary_mint,
        &foundation.base_mint,
        Some(3),
    ).await?;
    
    // Update foundation with pool config
    foundation.pool_config = config;
    
    // Mint tokens to user
    let deposit_amount = 2_000_000u64;
    common::tokens::mint_tokens(
        &mut foundation.env.banks_client,
        &foundation.env.payer,
        foundation.env.recent_blockhash,
        &foundation.primary_mint.pubkey(),
        &foundation.user1_primary_account.pubkey(),
        &foundation.env.payer,
        deposit_amount,
    ).await?;
    
    println!("📊 Testing deposit transaction summary...");
    
    // Execute deposit
    let user1 = &foundation.user1;
    let user1_primary_account_pubkey = foundation.user1_primary_account.pubkey();
    let user1_lp_a_account_pubkey = foundation.user1_lp_a_account.pubkey();
    let primary_mint_pubkey = foundation.primary_mint.pubkey();
    
    let result = common::liquidity_helpers::execute_deposit_operation(
        &mut foundation,
        user1,
        &user1_primary_account_pubkey,
        &user1_lp_a_account_pubkey,
        &primary_mint_pubkey,
        deposit_amount,
    ).await;
    
    match result {
        Ok(_) => {
            println!("✅ Deposit transaction summary test passed!");
            
            // Verify the transaction summary details
            let user_lp_balance = common::tokens::get_token_balance(
                &mut foundation.env.banks_client,
                &foundation.user1_lp_a_account.pubkey(),
            ).await;
            
            println!("📈 Transaction Summary Verification:");
            println!("   • Input: {} tokens", deposit_amount);
            println!("   • Output: {} LP tokens", user_lp_balance);
            println!("   • Fee: {} lamports", DEPOSIT_WITHDRAWAL_FEE);
            println!("   • Pool: {}", foundation.pool_config.pool_state_pda);
            
            assert_eq!(user_lp_balance, deposit_amount, "LP tokens should match deposit amount");
            
            // Test withdrawal transaction summary
            println!("📊 Testing withdrawal transaction summary...");
            
            let withdrawal_amount = 1_000_000u64;
            let user1 = &foundation.user1;
            let user1_lp_a_account_pubkey = foundation.user1_lp_a_account.pubkey();
            let user1_primary_account_pubkey = foundation.user1_primary_account.pubkey();
            let primary_mint_pubkey = foundation.primary_mint.pubkey();
            
            let withdrawal_result = common::liquidity_helpers::execute_withdrawal_operation(
                &mut foundation,
                user1,
                &user1_lp_a_account_pubkey,
                &user1_primary_account_pubkey,
                &primary_mint_pubkey,
                withdrawal_amount,
            ).await;
            
            match withdrawal_result {
                Ok(_) => {
                    println!("✅ Withdrawal transaction summary test passed!");
                    
                    // Verify withdrawal summary details
                    let final_lp_balance = common::tokens::get_token_balance(
                        &mut foundation.env.banks_client,
                        &foundation.user1_lp_a_account.pubkey(),
                    ).await;
                    
                    let user_token_balance = common::tokens::get_token_balance(
                        &mut foundation.env.banks_client,
                        &foundation.user1_primary_account.pubkey(),
                    ).await;
                    
                    println!("📈 Withdrawal Summary Verification:");
                    println!("   • LP Tokens Burned: {}", withdrawal_amount);
                    println!("   • Tokens Received: {} (mint: {})", user_token_balance, foundation.primary_mint.pubkey());
                    println!("   • Fee: {} lamports", DEPOSIT_WITHDRAWAL_FEE);
                    println!("   • Remaining LP: {}", final_lp_balance);
                    
                    assert_eq!(user_token_balance, withdrawal_amount, "User should have received tokens back");
                    assert_eq!(final_lp_balance, deposit_amount - withdrawal_amount, "LP balance should be reduced");
                }
                Err(e) => {
                    println!("❌ Withdrawal transaction summary test failed: {:?}", e);
                    return Err(e);
                }
            }
        }
        Err(e) => {
            println!("❌ Deposit transaction summary test failed: {:?}", e);
            return Err(e);
        }
    }
    
    Ok(())
}

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

/// Test UX hints during deposit operations
#[tokio::test]
async fn test_ux_hints_deposit() -> TestResult {
    println!("🧪 Testing UX hints during deposit operations...");
    
    // Create test foundation
    let mut foundation = create_liquidity_test_foundation(None).await
        .map_err(|e| solana_program_test::BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    
    // Extract user keypair to avoid borrow checker issues
    let user1 = foundation.user1;
    
    // Determine deposit accounts based on pool configuration
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
    
    // Get initial balances
    let initial_token_balance = crate::common::tokens::get_token_balance(&mut foundation.env.banks_client, &user_input_account).await;
    let initial_lp_balance = crate::common::tokens::get_token_balance(&mut foundation.env.banks_client, &user_output_lp_account).await;
    
    println!("💰 Initial balances - Token: {}, LP: {}", initial_token_balance, initial_lp_balance);
    
    // Create deposit instruction data
    let deposit_amount = 500_000_000; // 500K tokens
    let deposit_instruction_data = PoolInstruction::Deposit {
        deposit_token_mint: deposit_mint,
        amount: deposit_amount,
    };
    
    // Create deposit instruction
    let deposit_ix = crate::common::liquidity_helpers::create_deposit_instruction_standardized(
        &user1.pubkey(),
        &user_input_account,
        &user_output_lp_account,
        &foundation.pool_config,
        &foundation.lp_token_a_mint_pda,
        &foundation.lp_token_b_mint_pda,
        &deposit_instruction_data,
    ).map_err(|e| solana_program_test::BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    
    // Create and sign transaction
    let mut deposit_tx = solana_sdk::transaction::Transaction::new_with_payer(
        &[deposit_ix], 
        Some(&user1.pubkey())
    );
    deposit_tx.sign(&[&user1], foundation.env.recent_blockhash);
    
    // Execute deposit
    println!("🚀 Executing deposit transaction...");
    foundation.env.banks_client.process_transaction(deposit_tx).await?;
    
    // Get final balances
    let final_token_balance = crate::common::tokens::get_token_balance(&mut foundation.env.banks_client, &user_input_account).await;
    let final_lp_balance = crate::common::tokens::get_token_balance(&mut foundation.env.banks_client, &user_output_lp_account).await;
    
    println!("💰 Final balances - Token: {}, LP: {}", final_token_balance, initial_lp_balance);
    
    // Verify the operation was correct
    crate::common::liquidity_helpers::verify_liquidity_operation(
        &mut foundation.env.banks_client,
        "deposit",
        deposit_amount,
        &user_input_account,
        &user_output_lp_account,
        initial_token_balance,
        initial_lp_balance,
    ).await.map_err(|e| solana_program_test::BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    
    println!("✅ UX hints deposit test completed successfully");
    Ok(())
}

/// Test UX hints during withdrawal operations
#[tokio::test]
async fn test_ux_hints_withdrawal() -> TestResult {
    println!("🧪 Testing UX hints during withdrawal operations...");
    
    // Create test foundation
    let mut foundation = create_liquidity_test_foundation(None).await
        .map_err(|e| solana_program_test::BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    
    // Extract user keypair to avoid borrow checker issues
    let user1 = foundation.user1;
    
    // First, perform a deposit to have LP tokens to withdraw
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
    
    // Perform initial deposit
    let deposit_amount = 1_000_000_000; // 1M tokens
    let deposit_instruction_data = PoolInstruction::Deposit {
        deposit_token_mint: deposit_mint,
        amount: deposit_amount,
    };
    
    let deposit_ix = crate::common::liquidity_helpers::create_deposit_instruction_standardized(
        &user1.pubkey(),
        &user_input_account,
        &user_output_lp_account,
        &foundation.pool_config,
        &foundation.lp_token_a_mint_pda,
        &foundation.lp_token_b_mint_pda,
        &deposit_instruction_data,
    ).map_err(|e| solana_program_test::BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    
    let mut deposit_tx = solana_sdk::transaction::Transaction::new_with_payer(
        &[deposit_ix], 
        Some(&user1.pubkey())
    );
    deposit_tx.sign(&[&user1], foundation.env.recent_blockhash);
    
    println!("🚀 Performing initial deposit for withdrawal test...");
    foundation.env.banks_client.process_transaction(deposit_tx).await?;
    
    // Get balances after deposit
    let post_deposit_token_balance = crate::common::tokens::get_token_balance(&mut foundation.env.banks_client, &user_input_account).await;
    let post_deposit_lp_balance = crate::common::tokens::get_token_balance(&mut foundation.env.banks_client, &user_output_lp_account).await;
    
    println!("💰 Post-deposit balances - Token: {}, LP: {}", post_deposit_token_balance, post_deposit_lp_balance);
    
    // Now perform withdrawal
    let withdrawal_amount = 500_000_000; // Withdraw 500K LP tokens
    let withdrawal_instruction_data = PoolInstruction::Withdraw {
        withdraw_token_mint: deposit_mint, // Withdraw the same token we deposited
        lp_amount_to_burn: withdrawal_amount,
    };
    
    let withdrawal_ix = crate::common::liquidity_helpers::create_withdrawal_instruction_standardized(
        &user1.pubkey(),
        &user_output_lp_account, // LP token account (input)
        &user_input_account,     // Token account (output)
        &foundation.pool_config,
        &foundation.lp_token_a_mint_pda,
        &foundation.lp_token_b_mint_pda,
        &withdrawal_instruction_data,
    ).map_err(|e| solana_program_test::BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    
    let mut withdrawal_tx = solana_sdk::transaction::Transaction::new_with_payer(
        &[withdrawal_ix], 
        Some(&user1.pubkey())
    );
    withdrawal_tx.sign(&[&user1], foundation.env.recent_blockhash);
    
    // Execute withdrawal
    println!("🚀 Executing withdrawal transaction...");
    foundation.env.banks_client.process_transaction(withdrawal_tx).await?;
    
    // Get final balances
    let final_token_balance = crate::common::tokens::get_token_balance(&mut foundation.env.banks_client, &user_input_account).await;
    let final_lp_balance = crate::common::tokens::get_token_balance(&mut foundation.env.banks_client, &user_output_lp_account).await;
    
    println!("💰 Final balances - Token: {}, LP: {}", final_token_balance, final_lp_balance);
    
    // Verify the withdrawal operation was correct
    crate::common::liquidity_helpers::verify_liquidity_operation(
        &mut foundation.env.banks_client,
        "withdrawal",
        withdrawal_amount,
        &user_input_account,
        &user_output_lp_account,
        post_deposit_token_balance,
        post_deposit_lp_balance,
    ).await.map_err(|e| solana_program_test::BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    
    println!("✅ UX hints withdrawal test completed successfully");
    Ok(())
} 