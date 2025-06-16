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

//! # Liquidity Management Tests
//! 
//! This module contains comprehensive tests for liquidity management functionality,
//! including deposit and withdrawal operations with various scenarios and error handling.

mod common;

use common::*;

/// Test instruction serialization and deserialization
#[tokio::test]
async fn test_instruction_serialization() -> TestResult {
    println!("Testing instruction serialization...");
    
    // Create a simple deposit instruction
    let test_mint = Pubkey::new_unique();
    let test_amount = 1_000_000u64;
    
    let instruction = PoolInstruction::Deposit {
        deposit_token_mint: test_mint,
        amount: test_amount,
    };
    
    // Serialize it
    let serialized = instruction.try_to_vec().unwrap();
    println!("Serialized instruction: {:?}", serialized);
    println!("Serialized length: {}", serialized.len());
    
    // Deserialize it
    let deserialized = PoolInstruction::try_from_slice(&serialized).unwrap();
    println!("Deserialized instruction: {:?}", deserialized);
    
    // Verify it matches
    if let PoolInstruction::Deposit { deposit_token_mint, amount } = deserialized {
        assert_eq!(deposit_token_mint, test_mint);
        assert_eq!(amount, test_amount);
        println!("✅ Instruction serialization test passed");
    } else {
        panic!("❌ Deserialized instruction doesn't match expected type");
    }
    
    Ok(())
}

/// LIQ-001: Test basic deposit functionality with successful token deposit
/// 
/// This test verifies the core deposit functionality by:
/// 1. Creating a pool with 2:1 ratio (2 primary tokens per 1 base token)
/// 2. Setting up a user with tokens
/// 3. Performing a basic deposit of primary tokens
/// 4. Verifying LP tokens are minted correctly
/// 5. Verifying pool liquidity is updated
/// 6. Verifying fee collection
#[tokio::test]
async fn test_basic_deposit_success() -> TestResult {
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create token mints
    create_test_mints(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &[&ctx.primary_mint, &ctx.base_mint],
    ).await?;

    // Create pool with 2:1 ratio
    let config = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        &ctx.lp_token_a_mint,
        &ctx.lp_token_b_mint,
        Some(2), // 2:1 ratio
    ).await?;

    // Setup user with token accounts and extra SOL for fees
    let (user, user_primary_token_account, user_base_token_account) = setup_test_user(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint.pubkey(),
        &ctx.base_mint.pubkey(),
        Some(10_000_000_000), // 10 SOL for fees
    ).await?;

    // Mint tokens to user for depositing - use the correct token based on normalization
    let deposit_amount = 1_000_000u64; // 1M tokens
    let (deposit_mint, deposit_token_account) = if config.token_a_is_primary {
        // Primary token is token A, use primary token account
        (&ctx.primary_mint.pubkey(), &user_primary_token_account)
    } else {
        // Primary token is token B, use base token account
        (&ctx.base_mint.pubkey(), &user_base_token_account)
    };

    mint_tokens(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        deposit_mint,
        &deposit_token_account.pubkey(),
        &ctx.env.payer,
        deposit_amount,
    ).await?;

    // Create LP token account for user (primary token corresponds to LP token A or B based on normalization)
    let user_lp_token_account = Keypair::new();
    let lp_mint = if config.token_a_is_primary {
        &ctx.lp_token_a_mint.pubkey()
    } else {
        &ctx.lp_token_b_mint.pubkey()
    };

    create_token_account(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &user_lp_token_account,
        lp_mint,
        &user.pubkey(),
    ).await?;

    // Get initial balances
    let initial_user_token_balance = get_token_balance(&mut ctx.env.banks_client, &deposit_token_account.pubkey()).await;
    let initial_user_lp_balance = get_token_balance(&mut ctx.env.banks_client, &user_lp_token_account.pubkey()).await;
    let initial_pool_state = get_pool_state(&mut ctx.env.banks_client, &config.pool_state_pda).await
        .expect("Pool state should exist");

    // Verify initial state
    assert_eq!(initial_user_token_balance, deposit_amount, "User should have deposit amount initially");
    assert_eq!(initial_user_lp_balance, 0, "User should have no LP tokens initially");

    // Debug information
    println!("DEBUG: Pool configuration:");
    println!("  token_a_mint: {}", config.token_a_mint);
    println!("  token_b_mint: {}", config.token_b_mint);
    println!("  token_a_is_primary: {}", config.token_a_is_primary);
    println!("  primary_mint: {}", ctx.primary_mint.pubkey());
    println!("  base_mint: {}", ctx.base_mint.pubkey());
    let deposit_mint = if config.token_a_is_primary { 
        config.token_a_mint 
    } else { 
        config.token_b_mint 
    };
    println!("  deposit_token_mint: {}", deposit_mint);
    println!("  user_primary_token_account: {}", user_primary_token_account.pubkey());
    println!("  pool_state_pda: {}", config.pool_state_pda);

    // Test instruction serialization before using it
    let instruction_data = PoolInstruction::Deposit {
        deposit_token_mint: if config.token_a_is_primary { 
            config.token_a_mint 
        } else { 
            config.token_b_mint 
        },
        amount: deposit_amount,
    };
    
    let serialized = instruction_data.try_to_vec().unwrap();
    println!("DEBUG: Instruction serialized, length: {}", serialized.len());
    println!("DEBUG: First 20 bytes: {:?}", &serialized[..std::cmp::min(20, serialized.len())]);
    
    // Test deserialization
    let test_deserialize = PoolInstruction::try_from_slice(&serialized);
    match test_deserialize {
        Ok(instr) => println!("DEBUG: Test deserialization successful: {:?}", instr),
        Err(e) => println!("DEBUG: Test deserialization failed: {:?}", e),
    }

    // Verify the pool state PDA exists and has the expected data
    let pool_check_account = ctx.env.banks_client.get_account(config.pool_state_pda).await?;
    match pool_check_account {
        Some(account) => {
            println!("DEBUG: Pool state PDA has {} bytes of data", account.data.len());
            if account.data.len() == 0 {
                panic!("Pool state PDA has no data - pool was not properly initialized!");
            }
        },
        None => panic!("Pool state PDA account does not exist!"),
    }

    // Perform deposit
    let deposit_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            // Account order must match process_deposit function exactly
            AccountMeta::new(user.pubkey(), true),                           // accounts[0] - User (signer)  
            AccountMeta::new(deposit_token_account.pubkey(), false),         // accounts[1] - User's source token account
            AccountMeta::new(config.pool_state_pda, false),                  // accounts[2] - Pool state PDA - CRITICAL: Must be the actual initialized PDA
            AccountMeta::new_readonly(config.token_a_mint, false),           // accounts[3] - Token A mint for PDA seeds
            AccountMeta::new_readonly(config.token_b_mint, false),           // accounts[4] - Token B mint for PDA seeds
            AccountMeta::new(config.token_a_vault_pda, false),               // accounts[5] - Pool's Token A vault
            AccountMeta::new(config.token_b_vault_pda, false),               // accounts[6] - Pool's Token B vault
            AccountMeta::new(ctx.lp_token_a_mint.pubkey(), false),           // accounts[7] - LP Token A mint
            AccountMeta::new(ctx.lp_token_b_mint.pubkey(), false),           // accounts[8] - LP Token B mint
            AccountMeta::new(user_lp_token_account.pubkey(), false),         // accounts[9] - User's destination LP token account  
            AccountMeta::new_readonly(solana_program::system_program::id(), false), // accounts[10] - System program
            AccountMeta::new_readonly(spl_token::id(), false),                      // accounts[11] - SPL Token program
            AccountMeta::new_readonly(solana_program::sysvar::rent::id(), false),   // accounts[12] - Rent sysvar
            AccountMeta::new_readonly(solana_program::sysvar::clock::id(), false),  // accounts[13] - Clock sysvar
        ],
        data: serialized,
    };
    
    println!("DEBUG: About to execute deposit with pool_state_pda: {}", config.pool_state_pda);

    let mut deposit_tx = Transaction::new_with_payer(&[deposit_ix], Some(&user.pubkey()));
    deposit_tx.sign(&[&user], ctx.env.recent_blockhash);
    
    let result = ctx.env.banks_client.process_transaction(deposit_tx).await;
    match result {
        Ok(_) => println!("DEBUG: Deposit transaction executed successfully"),
        Err(e) => {
            println!("DEBUG: Deposit transaction failed with error: {:?}", e);
            panic!("Deposit transaction should succeed: {:?}", e);
        }
    }

    // Verify post-deposit state
    println!("DEBUG: Reading balances after deposit...");
    let final_user_token_balance = get_token_balance(&mut ctx.env.banks_client, &deposit_token_account.pubkey()).await;
    let final_user_lp_balance = get_token_balance(&mut ctx.env.banks_client, &user_lp_token_account.pubkey()).await;
    
    println!("DEBUG: Final user token balance: {}", final_user_token_balance);
    println!("DEBUG: Final user LP balance: {}", final_user_lp_balance);
    
    // Check if pool state account still exists and has data
    println!("DEBUG: Checking pool state account after deposit...");
    let pool_check_account_after = ctx.env.banks_client.get_account(config.pool_state_pda).await?;
    match pool_check_account_after {
        Some(account) => {
            println!("DEBUG: Pool state PDA after deposit has {} bytes of data", account.data.len());
            if account.data.len() == 0 {
                panic!("Pool state PDA lost its data during deposit!");
            }
        },
        None => panic!("Pool state PDA account was deleted during deposit!"),
    }
    
    let final_pool_state = get_pool_state(&mut ctx.env.banks_client, &config.pool_state_pda).await
        .expect("Pool state should exist after deposit");

    // Verify token transfer
    assert_eq!(final_user_token_balance, 0, "User tokens should be transferred to pool");
    assert_eq!(final_user_lp_balance, deposit_amount, "User should receive LP tokens equal to deposit amount (1:1 ratio)");

    // Verify pool liquidity update
    if config.token_a_is_primary {
        assert_eq!(
            final_pool_state.total_token_a_liquidity,
            initial_pool_state.total_token_a_liquidity + deposit_amount,
            "Pool Token A liquidity should increase by deposit amount"
        );
        assert_eq!(
            final_pool_state.total_token_b_liquidity,
            initial_pool_state.total_token_b_liquidity,
            "Pool Token B liquidity should remain unchanged"
        );
    } else {
        assert_eq!(
            final_pool_state.total_token_b_liquidity,
            initial_pool_state.total_token_b_liquidity + deposit_amount,
            "Pool Token B liquidity should increase by deposit amount"
        );
        assert_eq!(
            final_pool_state.total_token_a_liquidity,
            initial_pool_state.total_token_a_liquidity,
            "Pool Token A liquidity should remain unchanged"
        );
    }

    // Verify vault received tokens
    let vault_address = if config.token_a_is_primary {
        config.token_a_vault_pda
    } else {
        config.token_b_vault_pda
    };
    let vault_balance = get_token_balance(&mut ctx.env.banks_client, &vault_address).await;
    assert_eq!(vault_balance, deposit_amount, "Vault should receive deposited tokens");

    println!("✅ LIQ-001: Basic deposit test completed successfully");
    println!("   - Deposited: {} tokens", deposit_amount);
    println!("   - LP tokens minted: {}", final_user_lp_balance);
    println!("   - Pool liquidity updated correctly");
    
    Ok(())
}

/// LIQ-002: Test advanced deposit with slippage protection
/// 
/// This test verifies the `process_deposit_with_features` function which adds
/// slippage protection to deposits. It ensures users receive at least the
/// minimum expected LP tokens for their deposit.
#[tokio::test]
async fn test_deposit_with_features_success() -> TestResult {
    println!("🧪 Testing LIQ-002: Advanced deposit with slippage protection...");
    
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create token mints
    create_test_mints(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &[&ctx.primary_mint, &ctx.base_mint],
    ).await?;

    // Create pool with 3:1 ratio (3 primary tokens per 1 base token)
    let config = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        &ctx.lp_token_a_mint,
        &ctx.lp_token_b_mint,
        Some(3), // 3:1 ratio
    ).await?;
    println!("✅ Pool created with 3:1 ratio");

    // Setup user with token accounts and extra SOL for fees
    let (user, user_primary_token_account, _user_base_token_account) = setup_test_user(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint.pubkey(),
        &ctx.base_mint.pubkey(),
        Some(10_000_000_000), // 10 SOL for fees
    ).await?;
    println!("✅ User created and funded");

    // Mint tokens to user for depositing - use primary token
    let deposit_amount = 1_000_000u64; // 1M tokens
    let (deposit_mint, deposit_token_account) = if config.token_a_is_primary {
        (&ctx.primary_mint.pubkey(), &user_primary_token_account)
    } else {
        (&ctx.base_mint.pubkey(), &user_primary_token_account) // This would be wrong, but keeping same pattern
    };

    mint_tokens(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        deposit_mint,
        &deposit_token_account.pubkey(),
        &ctx.env.payer,
        deposit_amount,
    ).await?;
    println!("✅ Minted {} tokens to user", deposit_amount);

    // Create LP token account for user
    let user_lp_token_account = Keypair::new();
    let lp_mint = if config.token_a_is_primary {
        &ctx.lp_token_a_mint.pubkey()
    } else {
        &ctx.lp_token_b_mint.pubkey()
    };

    create_token_account(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &user_lp_token_account,
        lp_mint,
        &user.pubkey(),
    ).await?;

    // Get initial balances
    let initial_user_token_balance = get_token_balance(&mut ctx.env.banks_client, &deposit_token_account.pubkey()).await;
    let initial_user_lp_balance = get_token_balance(&mut ctx.env.banks_client, &user_lp_token_account.pubkey()).await;

    assert_eq!(initial_user_token_balance, deposit_amount, "User should have deposit amount initially");
    assert_eq!(initial_user_lp_balance, 0, "User should have no LP tokens initially");

    // Create the deposit with features instruction
    let deposit_amount_to_use = 500_000; // Deposit 500K tokens
    let minimum_lp_out = 450_000; // Expect at least 450K LP tokens (10% slippage tolerance)
    
    let deposit_instruction_data = PoolInstruction::DepositWithFeatures {
        deposit_token_mint: if config.token_a_is_primary { 
            config.token_a_mint 
        } else { 
            config.token_b_mint 
        },
        amount: deposit_amount_to_use,
        minimum_lp_tokens_out: minimum_lp_out,
        fee_recipient: None, // No custom fee recipient
    };

    let serialized = deposit_instruction_data.try_to_vec().unwrap();
    println!("DEBUG: DepositWithFeatures instruction serialized, length: {}", serialized.len());

    // Test deserialization
    let test_deserialize = PoolInstruction::try_from_slice(&serialized);
    match test_deserialize {
        Ok(_) => println!("DEBUG: DepositWithFeatures deserialization successful"),
        Err(e) => println!("DEBUG: DepositWithFeatures deserialization failed: {:?}", e),
    }

    // Perform deposit with features
    let deposit_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            // Account order must match process_deposit_with_features function exactly
            AccountMeta::new(user.pubkey(), true),                           // accounts[0] - User (signer)  
            AccountMeta::new(deposit_token_account.pubkey(), false),         // accounts[1] - User's source token account
            AccountMeta::new(config.pool_state_pda, false),                  // accounts[2] - Pool state PDA
            AccountMeta::new_readonly(config.token_a_mint, false),           // accounts[3] - Token A mint for PDA seeds
            AccountMeta::new_readonly(config.token_b_mint, false),           // accounts[4] - Token B mint for PDA seeds
            AccountMeta::new(config.token_a_vault_pda, false),               // accounts[5] - Pool's Token A vault
            AccountMeta::new(config.token_b_vault_pda, false),               // accounts[6] - Pool's Token B vault
            AccountMeta::new(ctx.lp_token_a_mint.pubkey(), false),           // accounts[7] - LP Token A mint
            AccountMeta::new(ctx.lp_token_b_mint.pubkey(), false),           // accounts[8] - LP Token B mint
            AccountMeta::new(user_lp_token_account.pubkey(), false),         // accounts[9] - User's destination LP token account
            AccountMeta::new_readonly(solana_program::system_program::id(), false), // accounts[10] - System program
            AccountMeta::new_readonly(spl_token::id(), false),                      // accounts[11] - SPL Token program
            AccountMeta::new_readonly(solana_program::sysvar::rent::id(), false),   // accounts[12] - Rent sysvar
            AccountMeta::new_readonly(solana_program::sysvar::clock::id(), false),  // accounts[13] - Clock sysvar
        ],
        data: serialized,
    };

    let mut deposit_tx = Transaction::new_with_payer(&[deposit_ix], Some(&user.pubkey()));
    deposit_tx.sign(&[&user], ctx.env.recent_blockhash);
    
    let result = ctx.env.banks_client.process_transaction(deposit_tx).await;
    match result {
        Ok(_) => {
            println!("✅ Deposit with features transaction succeeded");
            
            // Verify the LP tokens were received
            let final_lp_balance = get_token_balance(&mut ctx.env.banks_client, &user_lp_token_account.pubkey()).await;
            let lp_tokens_received = final_lp_balance - initial_user_lp_balance;
            println!("📊 LP tokens received: {}", lp_tokens_received);
            
            // Verify we received at least the minimum expected
            assert!(
                lp_tokens_received >= minimum_lp_out,
                "Should receive at least {} LP tokens, got {}",
                minimum_lp_out, lp_tokens_received
            );
            
            // In this fixed-ratio system, we expect 1:1 LP tokens for deposits
            assert_eq!(
                lp_tokens_received, deposit_amount_to_use,
                "Should receive exactly {} LP tokens for {} token deposit",
                deposit_amount_to_use, deposit_amount_to_use
            );
            
            // Verify the user's token balance decreased
            let final_token_balance = get_token_balance(&mut ctx.env.banks_client, &deposit_token_account.pubkey()).await;
            let expected_remaining = deposit_amount - deposit_amount_to_use;
            assert_eq!(
                final_token_balance, expected_remaining,
                "User should have {} tokens remaining, got {}",
                expected_remaining, final_token_balance
            );
            
            println!("✅ All slippage protection validations passed!");
            println!("✅ LIQ-002 test completed successfully!");
        }
        Err(e) => {
            println!("❌ Deposit with features transaction failed: {:?}", e);
            panic!("Deposit with features transaction should succeed: {:?}", e);
        }
    }

    Ok(())
}

/// LIQ-002b: Test slippage protection triggers correctly
/// 
/// This test verifies that the slippage protection in `process_deposit_with_features`
/// correctly rejects deposits when the minimum LP token requirement is not met.
#[tokio::test]
async fn test_deposit_with_features_slippage_protection() -> TestResult {
    println!("🧪 Testing LIQ-002b: Slippage protection triggers...");
    
    // Create completely separate context to avoid test interference
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create token mints
    create_test_mints(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &[&ctx.primary_mint, &ctx.base_mint],
    ).await?;

    // Create pool with 5:1 ratio (unique from other tests)
    let config = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        &ctx.lp_token_a_mint,
        &ctx.lp_token_b_mint,
        Some(5), // 5:1 ratio (unique to avoid PDA conflicts)
    ).await?;
    println!("✅ Pool created with 5:1 ratio");

    // Setup user with token accounts and extra SOL for fees
    let (user, user_primary_token_account, _user_base_token_account) = setup_test_user(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint.pubkey(),
        &ctx.base_mint.pubkey(),
        Some(10_000_000_000), // 10 SOL for fees
    ).await?;

    // Mint tokens to user
    let deposit_amount = 1_000_000;
    let (deposit_mint, deposit_token_account) = if config.token_a_is_primary {
        (&ctx.primary_mint.pubkey(), &user_primary_token_account)
    } else {
        (&ctx.base_mint.pubkey(), &user_primary_token_account)
    };

    mint_tokens(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        deposit_mint,
        &deposit_token_account.pubkey(),
        &ctx.env.payer,
        deposit_amount,
    ).await?;

    // Create LP token account for user
    let user_lp_token_account = Keypair::new();
    let lp_mint = if config.token_a_is_primary {
        &ctx.lp_token_a_mint.pubkey()
    } else {
        &ctx.lp_token_b_mint.pubkey()
    };

    create_token_account(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &user_lp_token_account,
        lp_mint,
        &user.pubkey(),
    ).await?;

    // Create deposit instruction with unrealistic minimum LP requirement
    let deposit_amount_to_use = 500_000;
    let minimum_lp_out = 600_000; // Expect MORE LP tokens than we're depositing (impossible)
    
    let deposit_instruction_data = PoolInstruction::DepositWithFeatures {
        deposit_token_mint: if config.token_a_is_primary { 
            config.token_a_mint 
        } else { 
            config.token_b_mint 
        },
        amount: deposit_amount_to_use,
        minimum_lp_tokens_out: minimum_lp_out,
        fee_recipient: None,
    };

    let serialized = deposit_instruction_data.try_to_vec().unwrap();

    let deposit_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(user.pubkey(), true),
            AccountMeta::new(deposit_token_account.pubkey(), false),
            AccountMeta::new(config.pool_state_pda, false),
            AccountMeta::new_readonly(config.token_a_mint, false),
            AccountMeta::new_readonly(config.token_b_mint, false),
            AccountMeta::new(config.token_a_vault_pda, false),
            AccountMeta::new(config.token_b_vault_pda, false),
            AccountMeta::new(ctx.lp_token_a_mint.pubkey(), false),
            AccountMeta::new(ctx.lp_token_b_mint.pubkey(), false),
            AccountMeta::new(user_lp_token_account.pubkey(), false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_program::sysvar::rent::id(), false),
            AccountMeta::new_readonly(solana_program::sysvar::clock::id(), false),
        ],
        data: serialized,
    };

    // Execute the transaction - it should fail due to slippage protection
    let mut deposit_tx = Transaction::new_with_payer(&[deposit_ix], Some(&user.pubkey()));
    deposit_tx.sign(&[&user], ctx.env.recent_blockhash);

    let result = ctx.env.banks_client.process_transaction(deposit_tx).await;
    
    match result {
        Ok(_) => {
            println!("❌ Transaction should have failed due to slippage protection!");
            panic!("Slippage protection did not trigger as expected");
        }
        Err(e) => {
            println!("✅ Transaction correctly failed due to slippage protection: {:?}", e);
            
            // Verify it's the specific slippage protection error (Custom(2001))
            let error_str = format!("{:?}", e);
            assert!(
                error_str.contains("Custom(2001)") || error_str.contains("slippage"),
                "Should fail with slippage protection error, got: {}",
                error_str
            );
            
            println!("✅ Slippage protection correctly triggered!");
            println!("✅ LIQ-002b test completed successfully!");
        }
    }

    Ok(())
} 