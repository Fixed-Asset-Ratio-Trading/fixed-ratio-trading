//! Pool Fee Update Tests
//! 
//! This module contains comprehensive tests for the UpdatePoolFees instruction
//! to verify that pool fees can be updated correctly and securely.

use {
    fixed_ratio_trading::{
        constants::*,
        error::PoolError,
        state::pool_state::PoolState,
        types::instructions::PoolInstruction,
    },
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        system_instruction,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
    },
    std::str::FromStr,
    borsh::BorshSerialize,
};

mod common;

use common::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Test successful fee update for liquidity fee only
#[tokio::test]
async fn test_update_liquidity_fee_only() -> TestResult {
    println!("🧪 TEST: Update liquidity fee only");
    
    // Setup test environment
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create a pool with default fees
    let pool_info = create_test_pool(&mut ctx, 1000, 1).await?;
    let pool_state_pda = pool_info.pool_state_pda;
    
    // Get initial pool state
    let initial_pool_state = get_pool_state(&mut ctx.banks_client, &pool_state_pda).await?;
    let initial_liquidity_fee = initial_pool_state.contract_liquidity_fee;
    let initial_swap_fee = initial_pool_state.swap_contract_fee;
    
    println!("📊 Initial fees - Liquidity: {} lamports, Swap: {} lamports", 
             initial_liquidity_fee, initial_swap_fee);
    
    // Define new liquidity fee (increase by 50%)
    let new_liquidity_fee = initial_liquidity_fee + (initial_liquidity_fee / 2);
    let new_swap_fee = initial_swap_fee; // Keep swap fee unchanged
    
    // Create fee update instruction
    let update_instruction = Instruction {
        program_id: ctx.program_id,
        accounts: vec![
            AccountMeta::new_readonly(ctx.payer.pubkey(), true), // Program authority signer
            AccountMeta::new_readonly(ctx.system_state_pda, false), // System state PDA
            AccountMeta::new(pool_state_pda, false), // Pool state PDA (writable)
            AccountMeta::new_readonly(Pubkey::from_str("BPFLoaderUpgradeab1e11111111111111111111111")?, false), // Program data account
        ],
        data: PoolInstruction::UpdatePoolFees {
            update_flags: FEE_UPDATE_FLAG_LIQUIDITY,
            new_liquidity_fee,
            new_swap_fee,
        }
        .try_to_vec()?,
    };
    
    // Execute the transaction
    let transaction = Transaction::new_signed_with_payer(
        &[update_instruction],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.recent_blockhash,
    );
    
    ctx.banks_client.process_transaction(transaction).await?;
    
    // Verify the update
    let updated_pool_state = get_pool_state(&mut ctx.banks_client, &pool_state_pda).await?;
    
    println!("📊 Updated fees - Liquidity: {} lamports, Swap: {} lamports", 
             updated_pool_state.contract_liquidity_fee, updated_pool_state.swap_contract_fee);
    
    // Assertions
    assert_eq!(updated_pool_state.contract_liquidity_fee, new_liquidity_fee, 
               "Liquidity fee should be updated");
    assert_eq!(updated_pool_state.swap_contract_fee, initial_swap_fee, 
               "Swap fee should remain unchanged");
    
    println!("✅ Test passed: Liquidity fee updated successfully");
    Ok(())
}

/// Test successful fee update for swap fee only
#[tokio::test]
async fn test_update_swap_fee_only() -> TestResult {
    println!("🧪 TEST: Update swap fee only");
    
    // Setup test environment
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create a pool with default fees
    let pool_info = create_test_pool(&mut ctx, 1000, 1).await?;
    let pool_state_pda = pool_info.pool_state_pda;
    
    // Get initial pool state
    let initial_pool_state = get_pool_state(&mut ctx.banks_client, &pool_state_pda).await?;
    let initial_liquidity_fee = initial_pool_state.contract_liquidity_fee;
    let initial_swap_fee = initial_pool_state.swap_contract_fee;
    
    println!("📊 Initial fees - Liquidity: {} lamports, Swap: {} lamports", 
             initial_liquidity_fee, initial_swap_fee);
    
    // Define new swap fee (decrease by 25%)
    let new_liquidity_fee = initial_liquidity_fee; // Keep liquidity fee unchanged
    let new_swap_fee = initial_swap_fee - (initial_swap_fee / 4);
    
    // Create fee update instruction
    let update_instruction = Instruction {
        program_id: ctx.program_id,
        accounts: vec![
            AccountMeta::new_readonly(ctx.payer.pubkey(), true), // Program authority signer
            AccountMeta::new_readonly(ctx.system_state_pda, false), // System state PDA
            AccountMeta::new(pool_state_pda, false), // Pool state PDA (writable)
            AccountMeta::new_readonly(Pubkey::from_str("BPFLoaderUpgradeab1e11111111111111111111111")?, false), // Program data account
        ],
        data: PoolInstruction::UpdatePoolFees {
            update_flags: FEE_UPDATE_FLAG_SWAP,
            new_liquidity_fee,
            new_swap_fee,
        }
        .try_to_vec()?,
    };
    
    // Execute the transaction
    let transaction = Transaction::new_signed_with_payer(
        &[update_instruction],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.recent_blockhash,
    );
    
    ctx.banks_client.process_transaction(transaction).await?;
    
    // Verify the update
    let updated_pool_state = get_pool_state(&mut ctx.banks_client, &pool_state_pda).await?;
    
    println!("📊 Updated fees - Liquidity: {} lamports, Swap: {} lamports", 
             updated_pool_state.contract_liquidity_fee, updated_pool_state.swap_contract_fee);
    
    // Assertions
    assert_eq!(updated_pool_state.contract_liquidity_fee, initial_liquidity_fee, 
               "Liquidity fee should remain unchanged");
    assert_eq!(updated_pool_state.swap_contract_fee, new_swap_fee, 
               "Swap fee should be updated");
    
    println!("✅ Test passed: Swap fee updated successfully");
    Ok(())
}

/// Test successful fee update for both fees
#[tokio::test]
async fn test_update_both_fees() -> TestResult {
    println!("🧪 TEST: Update both fees");
    
    // Setup test environment
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create a pool with default fees
    let pool_info = create_test_pool(&mut ctx, 1000, 1).await?;
    let pool_state_pda = pool_info.pool_state_pda;
    
    // Get initial pool state
    let initial_pool_state = get_pool_state(&mut ctx.banks_client, &pool_state_pda).await?;
    let initial_liquidity_fee = initial_pool_state.contract_liquidity_fee;
    let initial_swap_fee = initial_pool_state.swap_contract_fee;
    
    println!("📊 Initial fees - Liquidity: {} lamports, Swap: {} lamports", 
             initial_liquidity_fee, initial_swap_fee);
    
    // Define new fees
    let new_liquidity_fee = initial_liquidity_fee * 2; // Double liquidity fee
    let new_swap_fee = initial_swap_fee * 3; // Triple swap fee
    
    // Create fee update instruction
    let update_instruction = Instruction {
        program_id: ctx.program_id,
        accounts: vec![
            AccountMeta::new_readonly(ctx.payer.pubkey(), true), // Program authority signer
            AccountMeta::new_readonly(ctx.system_state_pda, false), // System state PDA
            AccountMeta::new(pool_state_pda, false), // Pool state PDA (writable)
            AccountMeta::new_readonly(Pubkey::from_str("BPFLoaderUpgradeab1e11111111111111111111111")?, false), // Program data account
        ],
        data: PoolInstruction::UpdatePoolFees {
            update_flags: FEE_UPDATE_FLAG_BOTH,
            new_liquidity_fee,
            new_swap_fee,
        }
        .try_to_vec()?,
    };
    
    // Execute the transaction
    let transaction = Transaction::new_signed_with_payer(
        &[update_instruction],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.recent_blockhash,
    );
    
    ctx.banks_client.process_transaction(transaction).await?;
    
    // Verify the update
    let updated_pool_state = get_pool_state(&mut ctx.banks_client, &pool_state_pda).await?;
    
    println!("📊 Updated fees - Liquidity: {} lamports, Swap: {} lamports", 
             updated_pool_state.contract_liquidity_fee, updated_pool_state.swap_contract_fee);
    
    // Assertions
    assert_eq!(updated_pool_state.contract_liquidity_fee, new_liquidity_fee, 
               "Liquidity fee should be updated");
    assert_eq!(updated_pool_state.swap_contract_fee, new_swap_fee, 
               "Swap fee should be updated");
    
    println!("✅ Test passed: Both fees updated successfully");
    Ok(())
}

/// Test that updated fees are applied during swap operations
#[tokio::test]
async fn test_updated_fees_applied_to_swaps() -> TestResult {
    println!("🧪 TEST: Updated fees applied to swaps");
    
    // Setup test environment
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create a pool with default fees
    let pool_info = create_test_pool(&mut ctx, 1000, 1).await?;
    let pool_state_pda = pool_info.pool_state_pda;
    
    // Add liquidity to the pool
    add_liquidity_to_pool(&mut ctx, &pool_info, 1_000_000).await?;
    
    // Get initial pool state
    let initial_pool_state = get_pool_state(&mut ctx.banks_client, &pool_state_pda).await?;
    let initial_swap_fee = initial_pool_state.swap_contract_fee;
    
    // Update swap fee to a higher value
    let new_swap_fee = initial_swap_fee * 2; // Double the swap fee
    
    // Create fee update instruction
    let update_instruction = Instruction {
        program_id: ctx.program_id,
        accounts: vec![
            AccountMeta::new_readonly(ctx.payer.pubkey(), true), // Program authority signer
            AccountMeta::new_readonly(ctx.system_state_pda, false), // System state PDA
            AccountMeta::new(pool_state_pda, false), // Pool state PDA (writable)
            AccountMeta::new_readonly(Pubkey::from_str("BPFLoaderUpgradeab1e11111111111111111111111")?, false), // Program data account
        ],
        data: PoolInstruction::UpdatePoolFees {
            update_flags: FEE_UPDATE_FLAG_SWAP,
            new_liquidity_fee: initial_pool_state.contract_liquidity_fee,
            new_swap_fee,
        }
        .try_to_vec()?,
    };
    
    // Execute the fee update
    let update_transaction = Transaction::new_signed_with_payer(
        &[update_instruction],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.recent_blockhash,
    );
    
    ctx.banks_client.process_transaction(update_transaction).await?;
    
    // Get pool state after fee update
    let updated_pool_state = get_pool_state(&mut ctx.banks_client, &pool_state_pda).await?;
    assert_eq!(updated_pool_state.swap_contract_fee, new_swap_fee, 
               "Swap fee should be updated");
    
    // Perform a swap to verify the new fee is applied
    let swap_amount = 100_000;
    let swap_result = perform_swap(&mut ctx, &pool_info, swap_amount).await;
    
    // The swap should succeed with the new fee
    assert!(swap_result.is_ok(), "Swap should succeed with updated fee");
    
    // Verify that the new fee was collected
    let final_pool_state = get_pool_state(&mut ctx.banks_client, &pool_state_pda).await?;
    let fee_collected = final_pool_state.collected_swap_contract_fees - updated_pool_state.collected_swap_contract_fees;
    
    assert_eq!(fee_collected, new_swap_fee, 
               "New swap fee should be collected");
    
    println!("✅ Test passed: Updated swap fee applied correctly");
    Ok(())
}

/// Test that updated fees are applied during liquidity operations
#[tokio::test]
async fn test_updated_fees_applied_to_liquidity() -> TestResult {
    println!("🧪 TEST: Updated fees applied to liquidity operations");
    
    // Setup test environment
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create a pool with default fees
    let pool_info = create_test_pool(&mut ctx, 1000, 1).await?;
    let pool_state_pda = pool_info.pool_state_pda;
    
    // Get initial pool state
    let initial_pool_state = get_pool_state(&mut ctx.banks_client, &pool_state_pda).await?;
    let initial_liquidity_fee = initial_pool_state.contract_liquidity_fee;
    
    // Update liquidity fee to a higher value
    let new_liquidity_fee = initial_liquidity_fee * 2; // Double the liquidity fee
    
    // Create fee update instruction
    let update_instruction = Instruction {
        program_id: ctx.program_id,
        accounts: vec![
            AccountMeta::new_readonly(ctx.payer.pubkey(), true), // Program authority signer
            AccountMeta::new_readonly(ctx.system_state_pda, false), // System state PDA
            AccountMeta::new(pool_state_pda, false), // Pool state PDA (writable)
            AccountMeta::new_readonly(Pubkey::from_str("BPFLoaderUpgradeab1e11111111111111111111111")?, false), // Program data account
        ],
        data: PoolInstruction::UpdatePoolFees {
            update_flags: FEE_UPDATE_FLAG_LIQUIDITY,
            new_liquidity_fee,
            new_swap_fee: initial_pool_state.swap_contract_fee,
        }
        .try_to_vec()?,
    };
    
    // Execute the fee update
    let update_transaction = Transaction::new_signed_with_payer(
        &[update_instruction],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.recent_blockhash,
    );
    
    ctx.banks_client.process_transaction(update_transaction).await?;
    
    // Get pool state after fee update
    let updated_pool_state = get_pool_state(&mut ctx.banks_client, &pool_state_pda).await?;
    assert_eq!(updated_pool_state.contract_liquidity_fee, new_liquidity_fee, 
               "Liquidity fee should be updated");
    
    // Perform a deposit to verify the new fee is applied
    let deposit_amount = 1_000_000;
    let deposit_result = add_liquidity_to_pool(&mut ctx, &pool_info, deposit_amount).await;
    
    // The deposit should succeed with the new fee
    assert!(deposit_result.is_ok(), "Deposit should succeed with updated fee");
    
    // Verify that the new fee was collected
    let final_pool_state = get_pool_state(&mut ctx.banks_client, &pool_state_pda).await?;
    let fee_collected = final_pool_state.collected_liquidity_fees - updated_pool_state.collected_liquidity_fees;
    
    assert_eq!(fee_collected, new_liquidity_fee, 
               "New liquidity fee should be collected");
    
    println!("✅ Test passed: Updated liquidity fee applied correctly");
    Ok(())
}

/// Test unauthorized fee update attempt
#[tokio::test]
async fn test_unauthorized_fee_update() -> TestResult {
    println!("🧪 TEST: Unauthorized fee update attempt");
    
    // Setup test environment
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create a pool with default fees
    let pool_info = create_test_pool(&mut ctx, 1000, 1).await?;
    let pool_state_pda = pool_info.pool_state_pda;
    
    // Create a different user (not the program authority)
    let unauthorized_user = Keypair::new();
    
    // Fund the unauthorized user
    let fund_instruction = system_instruction::transfer(
        &ctx.payer.pubkey(),
        &unauthorized_user.pubkey(),
        1_000_000_000, // 1 SOL
    );
    
    let fund_transaction = Transaction::new_signed_with_payer(
        &[fund_instruction],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.recent_blockhash,
    );
    
    ctx.banks_client.process_transaction(fund_transaction).await?;
    
    // Try to update fees with unauthorized user
    let update_instruction = Instruction {
        program_id: ctx.program_id,
        accounts: vec![
            AccountMeta::new_readonly(unauthorized_user.pubkey(), true), // Unauthorized signer
            AccountMeta::new_readonly(ctx.system_state_pda, false), // System state PDA
            AccountMeta::new(pool_state_pda, false), // Pool state PDA (writable)
            AccountMeta::new_readonly(Pubkey::from_str("BPFLoaderUpgradeab1e11111111111111111111111")?, false), // Program data account
        ],
        data: PoolInstruction::UpdatePoolFees {
            update_flags: FEE_UPDATE_FLAG_LIQUIDITY,
            new_liquidity_fee: 2_000_000,
            new_swap_fee: 50_000,
        }
        .try_to_vec()?,
    };
    
    // Execute the transaction (should fail)
    let transaction = Transaction::new_signed_with_payer(
        &[update_instruction],
        Some(&unauthorized_user.pubkey()),
        &[&unauthorized_user],
        ctx.recent_blockhash,
    );
    
    let result = ctx.banks_client.process_transaction(transaction).await;
    
    // Should fail with unauthorized error
    assert!(result.is_err(), "Unauthorized fee update should fail");
    
    println!("✅ Test passed: Unauthorized fee update properly rejected");
    Ok(())
}

/// Test invalid fee update flags
#[tokio::test]
async fn test_invalid_fee_update_flags() -> TestResult {
    println!("🧪 TEST: Invalid fee update flags");
    
    // Setup test environment
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create a pool with default fees
    let pool_info = create_test_pool(&mut ctx, 1000, 1).await?;
    let pool_state_pda = pool_info.pool_state_pda;
    
    // Try to update fees with invalid flags
    let update_instruction = Instruction {
        program_id: ctx.program_id,
        accounts: vec![
            AccountMeta::new_readonly(ctx.payer.pubkey(), true), // Program authority signer
            AccountMeta::new_readonly(ctx.system_state_pda, false), // System state PDA
            AccountMeta::new(pool_state_pda, false), // Pool state PDA (writable)
            AccountMeta::new_readonly(Pubkey::from_str("BPFLoaderUpgradeab1e11111111111111111111111")?, false), // Program data account
        ],
        data: PoolInstruction::UpdatePoolFees {
            update_flags: 0, // Invalid: no flags set
            new_liquidity_fee: 2_000_000,
            new_swap_fee: 50_000,
        }
        .try_to_vec()?,
    };
    
    // Execute the transaction (should fail)
    let transaction = Transaction::new_signed_with_payer(
        &[update_instruction],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.recent_blockhash,
    );
    
    let result = ctx.banks_client.process_transaction(transaction).await;
    
    // Should fail with invalid flags error
    assert!(result.is_err(), "Invalid fee update flags should fail");
    
    println!("✅ Test passed: Invalid fee update flags properly rejected");
    Ok(())
}

/// Test fee validation limits
#[tokio::test]
async fn test_fee_validation_limits() -> TestResult {
    println!("🧪 TEST: Fee validation limits");
    
    // Setup test environment
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create a pool with default fees
    let pool_info = create_test_pool(&mut ctx, 1000, 1).await?;
    let pool_state_pda = pool_info.pool_state_pda;
    
    // Test liquidity fee too low
    let update_instruction_low = Instruction {
        program_id: ctx.program_id,
        accounts: vec![
            AccountMeta::new_readonly(ctx.payer.pubkey(), true), // Program authority signer
            AccountMeta::new_readonly(ctx.system_state_pda, false), // System state PDA
            AccountMeta::new(pool_state_pda, false), // Pool state PDA (writable)
            AccountMeta::new_readonly(Pubkey::from_str("BPFLoaderUpgradeab1e11111111111111111111111")?, false), // Program data account
        ],
        data: PoolInstruction::UpdatePoolFees {
            update_flags: FEE_UPDATE_FLAG_LIQUIDITY,
            new_liquidity_fee: MIN_LIQUIDITY_FEE - 1, // Too low
            new_swap_fee: 50_000,
        }
        .try_to_vec()?,
    };
    
    let transaction_low = Transaction::new_signed_with_payer(
        &[update_instruction_low],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.recent_blockhash,
    );
    
    let result_low = ctx.banks_client.process_transaction(transaction_low).await;
    assert!(result_low.is_err(), "Liquidity fee too low should fail");
    
    // Test liquidity fee too high
    let update_instruction_high = Instruction {
        program_id: ctx.program_id,
        accounts: vec![
            AccountMeta::new_readonly(ctx.payer.pubkey(), true), // Program authority signer
            AccountMeta::new_readonly(ctx.system_state_pda, false), // System state PDA
            AccountMeta::new(pool_state_pda, false), // Pool state PDA (writable)
            AccountMeta::new_readonly(Pubkey::from_str("BPFLoaderUpgradeab1e11111111111111111111111")?, false), // Program data account
        ],
        data: PoolInstruction::UpdatePoolFees {
            update_flags: FEE_UPDATE_FLAG_LIQUIDITY,
            new_liquidity_fee: MAX_LIQUIDITY_FEE + 1, // Too high
            new_swap_fee: 50_000,
        }
        .try_to_vec()?,
    };
    
    let transaction_high = Transaction::new_signed_with_payer(
        &[update_instruction_high],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.recent_blockhash,
    );
    
    let result_high = ctx.banks_client.process_transaction(transaction_high).await;
    assert!(result_high.is_err(), "Liquidity fee too high should fail");
    
    println!("✅ Test passed: Fee validation limits working correctly");
    Ok(())
}

/// Test fee update with system paused
#[tokio::test]
async fn test_fee_update_with_system_paused() -> TestResult {
    println!("🧪 TEST: Fee update with system paused");
    
    // Setup test environment
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create a pool with default fees
    let pool_info = create_test_pool(&mut ctx, 1000, 1).await?;
    let pool_state_pda = pool_info.pool_state_pda;
    
    // Pause the system
    let pause_instruction = Instruction {
        program_id: ctx.program_id,
        accounts: vec![
            AccountMeta::new_readonly(ctx.payer.pubkey(), true), // System authority signer
            AccountMeta::new(ctx.system_state_pda, false), // System state PDA (writable)
        ],
        data: PoolInstruction::PauseSystem { reason_code: 1 }
            .try_to_vec()?,
    };
    
    let pause_transaction = Transaction::new_signed_with_payer(
        &[pause_instruction],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.recent_blockhash,
    );
    
    ctx.banks_client.process_transaction(pause_transaction).await?;
    
    // Try to update fees while system is paused
    let update_instruction = Instruction {
        program_id: ctx.program_id,
        accounts: vec![
            AccountMeta::new_readonly(ctx.payer.pubkey(), true), // Program authority signer
            AccountMeta::new_readonly(ctx.system_state_pda, false), // System state PDA
            AccountMeta::new(pool_state_pda, false), // Pool state PDA (writable)
            AccountMeta::new_readonly(Pubkey::from_str("BPFLoaderUpgradeab1e11111111111111111111111")?, false), // Program data account
        ],
        data: PoolInstruction::UpdatePoolFees {
            update_flags: FEE_UPDATE_FLAG_LIQUIDITY,
            new_liquidity_fee: 2_000_000,
            new_swap_fee: 50_000,
        }
        .try_to_vec()?,
    };
    
    let transaction = Transaction::new_signed_with_payer(
        &[update_instruction],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.recent_blockhash,
    );
    
    let result = ctx.banks_client.process_transaction(transaction).await;
    
    // Should fail because system is paused
    assert!(result.is_err(), "Fee update should fail when system is paused");
    
    println!("✅ Test passed: Fee update properly blocked when system is paused");
    Ok(())
}

/// Helper function to get pool state
async fn get_pool_state(
    banks_client: &mut solana_program_test::BanksClient,
    pool_state_pda: &Pubkey,
) -> Result<PoolState, Box<dyn std::error::Error>> {
    let account = banks_client.get_account(*pool_state_pda).await?
        .ok_or("Pool state account not found")?;
    let pool_state = PoolState::try_from_slice(&account.data)?;
    Ok(pool_state)
}

/// Helper function to create a test pool
async fn create_test_pool(
    ctx: &mut PoolTestContext,
    ratio_a: u64,
    ratio_b: u64,
) -> Result<PoolInfo, Box<dyn std::error::Error>> {
    use crate::common::pool_helpers::*;
    
    // Create a real pool using existing helpers
    let pool_result = create_pool_with_ratio_enhanced(ctx, ratio_a, ratio_b).await?;
    
    let pool_info = PoolInfo {
        pool_state_pda: pool_result.pool_state_pda,
        token_a_vault_pda: pool_result.token_a_vault_pda,
        token_b_vault_pda: pool_result.token_b_vault_pda,
        lp_token_a_mint: pool_result.lp_token_a_mint_pda,
        lp_token_b_mint: pool_result.lp_token_b_mint_pda,
        primary_mint: ctx.primary_mint.pubkey(),
        base_mint: ctx.base_mint.pubkey(),
    };
    Ok(pool_info)
}

/// Helper function to add liquidity to a pool
async fn add_liquidity_to_pool(
    ctx: &mut PoolTestContext,
    pool_info: &PoolInfo,
    amount: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::common::liquidity_helpers::*;
    
    // Use real liquidity helper to add liquidity
    let deposit_result = DepositTestConfig {
        pool_state_pda: pool_info.pool_state_pda,
        user_input_token_account: ctx.user_primary_token_account,
        user_output_lp_token_account: ctx.user_lp_token_a_account,
        deposit_token_mint: pool_info.primary_mint,
        amount,
        expected_lp_tokens: amount, // 1:1 ratio
    };
    
    perform_deposit_operation(ctx, &deposit_result).await?;
    Ok(())
}

/// Helper function to perform a swap
async fn perform_swap(
    ctx: &mut PoolTestContext,
    pool_info: &PoolInfo,
    amount: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::common::flow_helpers::*;
    
    // Use real swap helper to perform swap
    let swap_config = SwapTestConfig {
        pool_state_pda: pool_info.pool_state_pda,
        user_input_token_account: ctx.user_primary_token_account,
        user_output_token_account: ctx.user_base_token_account,
        input_token_mint: pool_info.primary_mint,
        amount_in: amount,
        expected_amount_out: amount, // 1:1 ratio
    };
    
    perform_swap_operation_comprehensive(ctx, &swap_config).await?;
    Ok(())
}

/// Pool information structure for testing
struct PoolInfo {
    pool_state_pda: Pubkey,
    token_a_vault_pda: Pubkey,
    token_b_vault_pda: Pubkey,
    lp_token_a_mint: Pubkey,
    lp_token_b_mint: Pubkey,
    primary_mint: Pubkey,
    base_mint: Pubkey,
} 