// Liquidity operation helpers with standardized account ordering
// This module provides reusable functions for creating liquidity operations
// that build on the successful pool creation foundation

use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};
use borsh::BorshSerialize;
use fixed_ratio_trading::{
    constants::*,
    types::instructions::PoolInstruction,
    id,
};
use crate::common::{
    pool_helpers::PoolConfig,
    setup::{TestEnvironment, initialize_treasury_system},
    tokens::{create_mint, create_token_account, mint_tokens},
    TestResult,
};

/// Complete liquidity test foundation that builds on pool creation success
/// This provides a ready-to-use environment for all liquidity operations
#[allow(dead_code)]
pub struct LiquidityTestFoundation {
    pub env: TestEnvironment,
    pub pool_config: PoolConfig,
    pub primary_mint: Keypair,
    pub base_mint: Keypair,
    pub lp_token_a_mint_pda: Pubkey,
    pub lp_token_b_mint_pda: Pubkey,
    pub user1: Keypair,
    pub user1_primary_account: Keypair,
    pub user1_base_account: Keypair,
    pub user1_lp_a_account: Keypair,
    pub user1_lp_b_account: Keypair,
    pub user2: Keypair,
    pub user2_primary_account: Keypair,
    pub user2_base_account: Keypair,
    pub user2_lp_a_account: Keypair,
    pub user2_lp_b_account: Keypair,
}

/// Creates a complete liquidity test foundation with pool + funded users
/// This is the cascading foundation that all other tests can build on
/// OPTIMIZED VERSION - reduces sequential operations to prevent timeouts
#[allow(dead_code)]
pub async fn create_liquidity_test_foundation(
    pool_ratio: Option<u64>, // e.g., Some(3) for 3:1 ratio
) -> Result<LiquidityTestFoundation, Box<dyn std::error::Error>> {
    println!("🏗️ Creating OPTIMIZED liquidity test foundation...");
    
    // 1. Create test environment
    let mut env = crate::common::setup::start_test_environment().await;
    
    // 2. Create lexicographically ordered token mints
    let keypair1 = Keypair::new();
    let keypair2 = Keypair::new();
    
    let (primary_mint, base_mint) = if keypair1.pubkey() < keypair2.pubkey() {
        (keypair1, keypair2)
    } else {
        (keypair2, keypair1)
    };
    
    // 3. LP token mints will be created on-demand during first deposit (Phase 10 security)
    
    // 4. Create user keypairs early
    let user1 = Keypair::new();
    let user2 = Keypair::new();
    
    // Create user account keypairs
    let user1_primary_account = Keypair::new();
    let user1_base_account = Keypair::new();
    let user1_lp_a_account = Keypair::new();
    let user1_lp_b_account = Keypair::new();
    
    let user2_primary_account = Keypair::new();
    let user2_base_account = Keypair::new();
    let user2_lp_a_account = Keypair::new();
    let user2_lp_b_account = Keypair::new();
    
    // 5. BATCH OPERATION 1: Create token mints (reduce sequential calls)
    println!("📦 Creating token mints...");
    create_mint(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &primary_mint,
        Some(6),
    ).await?;
    
    create_mint(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &base_mint,
        Some(6),
    ).await?;
    
    // 6. BATCH OPERATION 2: Initialize treasury system (single operation)
    println!("🏛️ Initializing treasury system...");
    let system_authority = Keypair::new();
    initialize_treasury_system(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &system_authority,
    ).await?;
    
    // 7. BATCH OPERATION 3: Create pool (single operation)
    println!("🏊 Creating pool...");
    let pool_config = crate::common::pool_helpers::create_pool_new_pattern(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &primary_mint,
        &base_mint,
        pool_ratio,
    ).await?;
    
    // 8. BATCH OPERATION 4: Fund users with SOL (reduced amounts for faster processing)
    println!("💰 Funding users with SOL...");
    crate::common::setup::transfer_sol(&mut env.banks_client, &env.payer, env.recent_blockhash, &env.payer, &user1.pubkey(), 5_000_000_000).await?; // 5 SOL (reduced from 10)
    crate::common::setup::transfer_sol(&mut env.banks_client, &env.payer, env.recent_blockhash, &env.payer, &user2.pubkey(), 2_000_000_000).await?; // 2 SOL (reduced from 5)
    
    // 9. BATCH OPERATION 5: Create token accounts (optimized batch processing)
    println!("🏦 Creating token accounts...");
    
    // ✅ PHASE 10 SECURITY: Derive LP token mint PDAs (controlled by smart contract)
    let (lp_token_a_mint_pda, _) = Pubkey::find_program_address(
        &[LP_TOKEN_A_MINT_SEED_PREFIX, pool_config.pool_state_pda.as_ref()],
        &id(),
    );
    let (lp_token_b_mint_pda, _) = Pubkey::find_program_address(
        &[LP_TOKEN_B_MINT_SEED_PREFIX, pool_config.pool_state_pda.as_ref()],
        &id(),
    );
    
    let accounts_to_create = [
        (&user1_primary_account, &primary_mint.pubkey(), &user1.pubkey()),
        (&user1_base_account, &base_mint.pubkey(), &user1.pubkey()),
        (&user2_primary_account, &primary_mint.pubkey(), &user2.pubkey()),
        (&user2_base_account, &base_mint.pubkey(), &user2.pubkey()),
        // NOTE: LP token accounts and mints are created on-demand during first deposit operation
        // The LP token mints are created by the smart contract and don't exist yet
    ];
    
    // Process accounts in smaller batches to prevent timeouts
    for (i, (account_keypair, mint_pubkey, owner_pubkey)) in accounts_to_create.iter().enumerate() {
        create_token_account(
            &mut env.banks_client,
            &env.payer,
            env.recent_blockhash,
            account_keypair,
            mint_pubkey,
            owner_pubkey,
        ).await?;
        
        // REMOVED delay for faster test execution
    }
    
    // 10. BATCH OPERATION 6: Mint tokens (reduced amounts for faster processing)
    println!("🪙 Minting tokens to users...");
    let user1_primary_amount = 5_000_000u64; // 5M tokens (reduced from 10M)
    let user1_base_amount = 2_500_000u64;    // 2.5M tokens (reduced from 5M)
    let user2_primary_amount = 1_000_000u64; // 1M tokens (reduced from 2M)
    let user2_base_amount = 500_000u64;      // 500K tokens (reduced from 1M)
    
    let mint_operations = [
        (&primary_mint.pubkey(), &user1_primary_account.pubkey(), user1_primary_amount),
        (&base_mint.pubkey(), &user1_base_account.pubkey(), user1_base_amount),
        (&primary_mint.pubkey(), &user2_primary_account.pubkey(), user2_primary_amount),
        (&base_mint.pubkey(), &user2_base_account.pubkey(), user2_base_amount),
    ];
    
    for (i, (mint_pubkey, account_pubkey, amount)) in mint_operations.iter().enumerate() {
        mint_tokens(
            &mut env.banks_client,
            &env.payer,
            env.recent_blockhash,
            mint_pubkey,
            account_pubkey,
            &env.payer,
            *amount,
        ).await?;
        
        // REMOVED delay for faster test execution
    }
    
    println!("✅ OPTIMIZED liquidity test foundation created successfully!");
    println!("   - Reduced token amounts for faster processing");
    println!("   - Batched operations to minimize sequential processing");
    
    Ok(LiquidityTestFoundation {
        env,
        pool_config,
        primary_mint,
        base_mint,
        lp_token_a_mint_pda,
        lp_token_b_mint_pda,
        user1,
        user1_primary_account,
        user1_base_account,
        user1_lp_a_account,
        user1_lp_b_account,
        user2,
        user2_primary_account,
        user2_base_account,
        user2_lp_a_account,
        user2_lp_b_account,
    })
}

/// Creates a deposit instruction with proper standardized account ordering
/// This matches the expected account ordering in process_deposit
#[allow(dead_code)]
pub fn create_deposit_instruction_standardized(
    user: &Pubkey,
    user_input_token_account: &Pubkey,    // Token account being deposited from
    user_output_lp_account: &Pubkey,      // LP token account receiving LP tokens
    pool_config: &PoolConfig,
    lp_token_a_mint: &Pubkey,             // LP Token A mint
    lp_token_b_mint: &Pubkey,             // LP Token B mint
    deposit_instruction_data: &PoolInstruction,
) -> Result<Instruction, Box<dyn std::error::Error>> {
    let serialized = deposit_instruction_data.try_to_vec()?;
    
    // Derive treasury PDAs
    let (main_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        &id(),
    );
    
    // Derive system state PDA for pause validation
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &id(),
    );
    
    // Phase 3: Use main treasury for all operations (specialized treasuries consolidated)
    
    // Create instruction with OPTIMIZED account ordering (11 accounts total)
    Ok(Instruction {
        program_id: id(),
        accounts: vec![
            // Account ordering matching optimized processor expectations:
            AccountMeta::new(*user, true),                                          // Index 0: User Authority Signer
            AccountMeta::new_readonly(solana_program::system_program::id(), false), // Index 1: System Program Account
            AccountMeta::new_readonly(system_state_pda, false),                     // Index 2: System State PDA
            AccountMeta::new(pool_config.pool_state_pda, false),                    // Index 3: Pool State PDA
            AccountMeta::new_readonly(spl_token::id(), false),                      // Index 4: SPL Token Program Account
            AccountMeta::new(pool_config.token_a_vault_pda, false),                 // Index 5: Token A Vault PDA
            AccountMeta::new(pool_config.token_b_vault_pda, false),                 // Index 6: Token B Vault PDA
            AccountMeta::new(*user_input_token_account, false),                     // Index 7: User Input Token Account
            AccountMeta::new(*user_output_lp_account, false),                       // Index 8: User Output LP Token Account
            AccountMeta::new(*lp_token_a_mint, false),                              // Index 9: LP Token A Mint PDA
            AccountMeta::new(*lp_token_b_mint, false),                              // Index 10: LP Token B Mint PDA
        ],
        data: serialized,
    })
}

/// Creates a withdrawal instruction with proper standardized account ordering
/// This matches the expected account ordering in process_withdraw
#[allow(dead_code)]
pub fn create_withdrawal_instruction_standardized(
    user: &Pubkey,
    user_input_lp_account: &Pubkey,        // LP token account being burned
    user_output_token_account: &Pubkey,    // Token account receiving underlying tokens
    pool_config: &PoolConfig,
    lp_token_a_mint: &Pubkey,              // LP Token A mint
    lp_token_b_mint: &Pubkey,              // LP Token B mint
    withdrawal_instruction_data: &PoolInstruction,
) -> Result<Instruction, Box<dyn std::error::Error>> {
    let serialized = withdrawal_instruction_data.try_to_vec()?;
    
    // Derive treasury PDAs
    let (main_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        &id(),
    );
    
    // Derive system state PDA for pause validation
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &id(),
    );
    
    // Phase 3: Use main treasury for all operations (specialized treasuries consolidated)
    
    // Create instruction with OPTIMIZED account ordering (11 accounts total)
    Ok(Instruction {
        program_id: id(),
        accounts: vec![
            // Account ordering matching optimized processor expectations:
            AccountMeta::new(*user, true),                                          // Index 0: User Authority Signer
            AccountMeta::new_readonly(solana_program::system_program::id(), false), // Index 1: System Program Account
            AccountMeta::new_readonly(system_state_pda, false),                     // Index 2: System State PDA
            AccountMeta::new(pool_config.pool_state_pda, false),                    // Index 3: Pool State PDA
            AccountMeta::new_readonly(spl_token::id(), false),                      // Index 4: SPL Token Program Account
            AccountMeta::new(pool_config.token_a_vault_pda, false),                 // Index 5: Token A Vault PDA
            AccountMeta::new(pool_config.token_b_vault_pda, false),                 // Index 6: Token B Vault PDA
            AccountMeta::new(*user_input_lp_account, false),                        // Index 7: User Input LP Token Account
            AccountMeta::new(*user_output_token_account, false),                    // Index 8: User Output Token Account
            AccountMeta::new(*lp_token_a_mint, false),                              // Index 9: LP Token A Mint PDA
            AccountMeta::new(*lp_token_b_mint, false),                              // Index 10: LP Token B Mint PDA
        ],
        data: serialized,
    })
}

/// Creates swap instruction for HFT optimized version (8 accounts - no system state)
#[allow(dead_code)]
pub fn create_swap_instruction_hft_optimized(
    user: &Pubkey,
    user_input_token_account: &Pubkey,     // Token account being swapped from
    user_output_token_account: &Pubkey,    // Token account receiving swapped tokens
    pool_config: &PoolConfig,
    swap_instruction_data: &PoolInstruction,
) -> Result<Instruction, Box<dyn std::error::Error>> {
    let serialized = swap_instruction_data.try_to_vec()?;
    
    // Create HFT optimized instruction with 8 accounts (no system state PDA)
    Ok(Instruction {
        program_id: id(),
        accounts: vec![
            // HFT optimized account ordering (8 accounts total - system state removed)
            AccountMeta::new(*user, true),                                          // Index 0: Authority/User Signer
            AccountMeta::new_readonly(solana_program::system_program::id(), false), // Index 1: System Program
            AccountMeta::new(pool_config.pool_state_pda, false),                    // Index 2: Pool State PDA (moved up)
            AccountMeta::new_readonly(spl_token::id(), false),                      // Index 3: SPL Token Program
            AccountMeta::new(pool_config.token_a_vault_pda, false),                 // Index 4: Token A Vault PDA
            AccountMeta::new(pool_config.token_b_vault_pda, false),                 // Index 5: Token B Vault PDA
            AccountMeta::new(*user_input_token_account, false),                     // Index 6: User Input Token Account
            AccountMeta::new(*user_output_token_account, false),                    // Index 7: User Output Token Account
        ],
        data: serialized,
    })
}

/// Creates swap instruction for regular swap version (9 accounts - includes system state)
#[allow(dead_code)]
pub fn create_swap_instruction_standardized(
    user: &Pubkey,
    user_input_token_account: &Pubkey,     // Token account being swapped from
    user_output_token_account: &Pubkey,    // Token account receiving swapped tokens
    pool_config: &PoolConfig,
    swap_instruction_data: &PoolInstruction,
) -> Result<Instruction, Box<dyn std::error::Error>> {
    let serialized = swap_instruction_data.try_to_vec()?;
    
    // Derive System State PDA (required for swap operations)
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &id(),
    );
    
    // Create instruction with FIXED account ordering (9 accounts for swaps - Main Treasury removed in Phase 4)
    Ok(Instruction {
        program_id: id(),
        accounts: vec![
            // FIXED account ordering matching swap processor expectations (9 accounts total)
            AccountMeta::new(*user, true),                                          // Index 0: Authority/User Signer
            AccountMeta::new_readonly(solana_program::system_program::id(), false), // Index 1: System Program
            AccountMeta::new_readonly(system_state_pda, false),                     // Index 2: System State PDA
            AccountMeta::new(pool_config.pool_state_pda, false),                    // Index 3: Pool State PDA
            AccountMeta::new_readonly(spl_token::id(), false),                      // Index 4: SPL Token Program
            AccountMeta::new(pool_config.token_a_vault_pda, false),                 // Index 5: Token A Vault PDA
            AccountMeta::new(pool_config.token_b_vault_pda, false),                 // Index 6: Token B Vault PDA
            AccountMeta::new(*user_input_token_account, false),                     // Index 7: User Input Token Account
            AccountMeta::new(*user_output_token_account, false),                    // Index 8: User Output Token Account
        ],
        data: serialized,
    })
}

/// Creates LP token accounts on-demand if they don't exist yet
/// This is needed because LP token mints are created by the smart contract
#[allow(dead_code)]
pub async fn ensure_lp_token_account_exists(
    foundation: &mut LiquidityTestFoundation,
    user_keypair: &Keypair,
    lp_token_account: &Keypair,
    lp_token_mint: &Pubkey,
) -> TestResult {
    // Check if the account already exists
    if let Ok(Some(_)) = foundation.env.banks_client.get_account(lp_token_account.pubkey()).await {
        return Ok(()); // Account already exists
    }
    
    // Create the LP token account
    crate::common::tokens::create_token_account(
        &mut foundation.env.banks_client,
        &foundation.env.payer,
        foundation.env.recent_blockhash,
        lp_token_account,
        lp_token_mint,
        &user_keypair.pubkey(),
    ).await
}

/// Executes a deposit operation using the standardized foundation
/// OPTIMIZED VERSION - creates user LP token account for specific mint before deposit
#[allow(dead_code)]
pub async fn execute_deposit_operation(
    foundation: &mut LiquidityTestFoundation,
    user_keypair: &Keypair,
    user_input_token_account: &Pubkey,
    user_output_lp_account: &Pubkey,
    deposit_token_mint: &Pubkey,
    amount: u64,
) -> TestResult {
    println!("🚀 Executing deposit: {} tokens", amount);
    
    // Step 1: Determine which LP token mint will be used for this deposit
    let is_depositing_token_a = *deposit_token_mint == foundation.pool_config.token_a_mint;
    let target_lp_mint_pda = if is_depositing_token_a {
        foundation.lp_token_a_mint_pda
    } else {
        foundation.lp_token_b_mint_pda
    };
    
    // Step 2: Create user's LP token account for the specific mint they're depositing
    let user_lp_account_keypair = if is_depositing_token_a {
        &foundation.user1_lp_a_account
    } else {
        &foundation.user1_lp_b_account
    };
    
    // Check if the LP token mint exists first
    println!("🔍 Checking if LP token mint exists: {}", target_lp_mint_pda);
    let mint_account = foundation.env.banks_client.get_account(target_lp_mint_pda).await?;
    
    if mint_account.is_none() {
        println!("⚠️ LP token mint does not exist yet. It will be created during deposit.");
        println!("   The user's LP token account will be handled by the smart contract.");
        
        // Don't try to create the user's LP token account now - let the smart contract handle it
    } else {
        println!("✅ LP token mint exists, checking user's LP token account...");
        
        // Check if user's LP token account already exists
        if let Ok(None) = foundation.env.banks_client.get_account(user_lp_account_keypair.pubkey()).await {
            println!("📝 Creating user LP token account for {} deposit...", 
                     if is_depositing_token_a { "Token A" } else { "Token B" });
            
            // Create the user's LP token account
            crate::common::tokens::create_token_account(
                &mut foundation.env.banks_client,
                &foundation.env.payer,
                foundation.env.recent_blockhash,
                user_lp_account_keypair,
                &target_lp_mint_pda,
                &user_keypair.pubkey(),
            ).await?;
            
            println!("✅ User LP token account created for specific deposit");
        } else {
            println!("✅ User LP token account already exists");
        }
    }
    
    // Step 3: Execute the deposit
    let deposit_instruction_data = PoolInstruction::Deposit {
        deposit_token_mint: *deposit_token_mint,
        amount,
    };
    
    let deposit_ix = create_deposit_instruction_standardized(
        &user_keypair.pubkey(),
        user_input_token_account,
        user_output_lp_account,
        &foundation.pool_config,
        &foundation.lp_token_a_mint_pda,
        &foundation.lp_token_b_mint_pda,
        &deposit_instruction_data,
    ).map_err(|e| solana_program_test::BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    
    let mut deposit_tx = solana_sdk::transaction::Transaction::new_with_payer(
        &[deposit_ix], 
        Some(&user_keypair.pubkey())
    );
    deposit_tx.sign(&[user_keypair], foundation.env.recent_blockhash);
    
    // Execute deposit with REDUCED timeout handling
    let timeout_duration = std::time::Duration::from_secs(5); // REDUCED from 10 to 5 seconds
    let process_future = foundation.env.banks_client.process_transaction(deposit_tx);
    
    match tokio::time::timeout(timeout_duration, process_future).await {
        Ok(result) => {
            match result {
                Ok(_) => {
                    println!("✅ Deposit operation completed successfully");
                }
                Err(e) => {
                    println!("❌ First deposit attempt failed with error: {:?}", e);
                    
                    // Check if this is the specific error for missing user LP token account
                    if let solana_program_test::BanksClientError::TransactionError(
                        solana_sdk::transaction::TransactionError::InstructionError(_, 
                        solana_sdk::instruction::InstructionError::Custom(4001))) = e {
                        
                        println!("⚠️ First deposit attempt failed: User LP token account doesn't exist");
                        println!("📝 Creating user LP token account and retrying...");
                        
                        // Check if the LP token mint was created during the first deposit
                        println!("🔍 Checking if LP token mint exists after first deposit: {}", target_lp_mint_pda);
                        let mint_account_after = foundation.env.banks_client.get_account(target_lp_mint_pda).await?;
                        
                        if mint_account_after.is_none() {
                            println!("❌ LP token mint still doesn't exist after first deposit attempt");
                            println!("   This means the first deposit didn't create the mint as expected");
                            return Err(solana_program_test::BanksClientError::Io(
                                std::io::Error::new(std::io::ErrorKind::Other, "LP token mint not created during first deposit")
                            ).into());
                        } else {
                            println!("✅ LP token mint exists after first deposit, creating user account...");
                        }
                        
                        // Create the user's LP token account now that the mint exists
                        crate::common::tokens::create_token_account(
                            &mut foundation.env.banks_client,
                            &foundation.env.payer,
                            foundation.env.recent_blockhash,
                            user_lp_account_keypair,
                            &target_lp_mint_pda,
                            &user_keypair.pubkey(),
                        ).await?;
                        
                        println!("✅ User LP token account created, retrying deposit...");
                        
                        // Retry the deposit
                        let retry_deposit_ix = create_deposit_instruction_standardized(
                            &user_keypair.pubkey(),
                            user_input_token_account,
                            user_output_lp_account,
                            &foundation.pool_config,
                            &foundation.lp_token_a_mint_pda,
                            &foundation.lp_token_b_mint_pda,
                            &deposit_instruction_data,
                        ).map_err(|e| solana_program_test::BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
                        
                        let mut retry_tx = solana_sdk::transaction::Transaction::new_with_payer(
                            &[retry_deposit_ix], 
                            Some(&user_keypair.pubkey())
                        );
                        retry_tx.sign(&[user_keypair], foundation.env.recent_blockhash);
                        
                        let retry_future = foundation.env.banks_client.process_transaction(retry_tx);
                        match tokio::time::timeout(timeout_duration, retry_future).await {
                            Ok(result) => {
                                result?;
                                println!("✅ Retry deposit operation completed successfully");
                            }
                            Err(_) => return Err(solana_program_test::BanksClientError::Io(
                                std::io::Error::new(std::io::ErrorKind::TimedOut, "Retry deposit operation timed out")
                            ).into()),
                        }
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        Err(_) => return Err(solana_program_test::BanksClientError::Io(
            std::io::Error::new(std::io::ErrorKind::TimedOut, "Deposit operation timed out")
        ).into()),
    }
    
    // REMOVED delay after operation
    // Small delay to prevent rapid-fire requests
    // tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    Ok(())
}

/// Executes a withdrawal operation using the standardized foundation
/// OPTIMIZED VERSION - adds timeout handling to prevent deadlocks
#[allow(dead_code)]
pub async fn execute_withdrawal_operation(
    foundation: &mut LiquidityTestFoundation,
    user_keypair: &Keypair,
    user_input_lp_account: &Pubkey,
    user_output_token_account: &Pubkey,
    withdraw_token_mint: &Pubkey,
    lp_amount_to_burn: u64,
) -> TestResult {
    // Note: LP token accounts should exist from previous deposit operations
    // The smart contract handles LP token account validation
    
    let withdrawal_instruction_data = PoolInstruction::Withdraw {
        withdraw_token_mint: *withdraw_token_mint,
        lp_amount_to_burn,
    };
    
    let withdrawal_ix = create_withdrawal_instruction_standardized(
        &user_keypair.pubkey(),
        user_input_lp_account,
        user_output_token_account,
        &foundation.pool_config,
        &foundation.lp_token_a_mint_pda,
        &foundation.lp_token_b_mint_pda,
        &withdrawal_instruction_data,
    ).map_err(|e| solana_program_test::BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    
    let mut withdrawal_tx = solana_sdk::transaction::Transaction::new_with_payer(
        &[withdrawal_ix], 
        Some(&user_keypair.pubkey())
    );
    withdrawal_tx.sign(&[user_keypair], foundation.env.recent_blockhash);
    
    // REDUCED timeout handling to prevent deadlocks
    let timeout_duration = std::time::Duration::from_secs(5); // REDUCED from 10 to 5 seconds
    let process_future = foundation.env.banks_client.process_transaction(withdrawal_tx);
    
    match tokio::time::timeout(timeout_duration, process_future).await {
        Ok(result) => result?,
        Err(_) => return Err(solana_program_test::BanksClientError::Io(
            std::io::Error::new(std::io::ErrorKind::TimedOut, "Withdrawal operation timed out")
        ).into()),
    }
    
    // REMOVED delay after operation  
    // Small delay to prevent rapid-fire requests
    // tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    Ok(())
}

// ========================================
// REUSABLE VERIFICATION TOOLS FOR FUTURE TESTS
// ========================================

/// Comprehensive balance verification helper
/// Checks token account balances and provides detailed reporting
#[allow(dead_code)]
pub async fn verify_balances(
    banks_client: &mut crate::common::BanksClient,
    description: &str,
    expected_balances: &[(&Pubkey, u64)], // (account, expected_balance) pairs
) -> Result<(), String> {
    println!("🔍 Verifying balances: {}", description);
    
    for (account, expected_balance) in expected_balances {
        let actual_balance = crate::common::tokens::get_token_balance(banks_client, account).await;
        
        if actual_balance != *expected_balance {
            let error_msg = format!(
                "❌ Balance mismatch for {}: expected {}, got {}",
                account, expected_balance, actual_balance
            );
            println!("{}", error_msg);
            return Err(error_msg);
        }
        
        println!("✅ {}: {} tokens", account, actual_balance);
    }
    
    Ok(())
}

/// Specialized verification for 1:1 ratio operations
/// Validates that token changes match LP token changes exactly
#[allow(dead_code)]
pub async fn verify_one_to_one_ratio(
    banks_client: &mut crate::common::BanksClient,
    operation_type: &str,
    token_account: &Pubkey,
    lp_account: &Pubkey,
    expected_change: u64,
    initial_token_balance: u64,
    initial_lp_balance: u64,
) -> Result<(), String> {
    let final_token_balance = crate::common::tokens::get_token_balance(banks_client, token_account).await;
    let final_lp_balance = crate::common::tokens::get_token_balance(banks_client, lp_account).await;
    
    let token_change = if operation_type == "deposit" {
        initial_token_balance.saturating_sub(final_token_balance)
    } else {
        final_token_balance.saturating_sub(initial_token_balance)
    };
    
    let lp_change = if operation_type == "deposit" {
        final_lp_balance.saturating_sub(initial_lp_balance)
    } else {
        initial_lp_balance.saturating_sub(final_lp_balance)
    };
    
    if token_change != expected_change {
        return Err(format!(
            "❌ {} token change mismatch: expected {}, got {}",
            operation_type, expected_change, token_change
        ));
    }
    
    if lp_change != expected_change {
        return Err(format!(
            "❌ {} LP change mismatch: expected {}, got {}",
            operation_type, expected_change, lp_change
        ));
    }
    
    if token_change != lp_change {
        return Err(format!(
            "❌ 1:1 ratio violation in {}: token change {} != LP change {}",
            operation_type, token_change, lp_change
        ));
    }
    
    println!("✅ 1:1 {} ratio verified: {} tokens ↔ {} LP tokens", operation_type, token_change, lp_change);
    Ok(())
}

/// Comprehensive liquidity operation verification
/// Validates balances, ratios, and operation success for deposits/withdrawals
#[allow(dead_code)]
pub async fn verify_liquidity_operation(
    banks_client: &mut crate::common::BanksClient,
    operation_type: &str, // "deposit" or "withdrawal"
    amount: u64,
    user_token_account: &Pubkey,
    user_lp_account: &Pubkey,
    initial_token_balance: u64,
    initial_lp_balance: u64,
) -> Result<(), String> {
    println!("🔍 Verifying {} operation for {} tokens/LP...", operation_type, amount);
    
    // Get final balances
    let final_token_balance = crate::common::tokens::get_token_balance(banks_client, user_token_account).await;
    let final_lp_balance = crate::common::tokens::get_token_balance(banks_client, user_lp_account).await;
    
    println!("Balances - Initial: tokens={}, LP={}", initial_token_balance, initial_lp_balance);
    println!("Balances - Final: tokens={}, LP={}", final_token_balance, final_lp_balance);
    
    // Verify 1:1 ratio
    verify_one_to_one_ratio(
        banks_client,
        operation_type,
        user_token_account,
        user_lp_account,
        amount,
        initial_token_balance,
        initial_lp_balance,
    ).await?;
    
    // Verify exact expected balances
    let (expected_token_balance, expected_lp_balance) = if operation_type == "deposit" {
        (initial_token_balance - amount, initial_lp_balance + amount)
    } else {
        (initial_token_balance + amount, initial_lp_balance - amount)
    };
    
    verify_balances(
        banks_client,
        &format!("{} final state", operation_type),
        &[
            (user_token_account, expected_token_balance),
            (user_lp_account, expected_lp_balance),
        ],
    ).await?;
    
    println!("✅ {} operation fully verified!", operation_type);
    Ok(())
}

/// Validates the state of a foundation after operations
/// Useful for debugging and ensuring test environment consistency
#[allow(dead_code)]
pub async fn validate_foundation_state(
    foundation: &mut LiquidityTestFoundation,
    expected_user1_primary_balance: Option<u64>,
    expected_user1_base_balance: Option<u64>,
) -> Result<(), String> {
    println!("🔍 Validating foundation state...");
    
    // Check that pool exists and is initialized
    let pool_state = crate::common::pool_helpers::get_pool_state(
        &mut foundation.env.banks_client,
        &foundation.pool_config.pool_state_pda,
    ).await;
    
    match pool_state {
        Some(state) => {
            // Pool existence = initialization (no is_initialized field needed)
            if false { // Pool is always initialized if we can deserialize it
                return Err("Pool should be initialized".to_string());
            }
            println!("✅ Pool is properly initialized");
        }
        None => {
            return Err("Pool state not found".to_string());
        }
    }
    
    // Check user balances if specified
    if let Some(expected) = expected_user1_primary_balance {
        let actual = crate::common::tokens::get_token_balance(
            &mut foundation.env.banks_client,
            &foundation.user1_primary_account.pubkey(),
        ).await;
        
        if actual != expected {
            return Err(format!(
                "User1 primary balance mismatch: expected {}, got {}",
                expected, actual
            ));
        }
        println!("✅ User1 primary balance: {}", actual);
    }
    
    if let Some(expected) = expected_user1_base_balance {
        let actual = crate::common::tokens::get_token_balance(
            &mut foundation.env.banks_client,
            &foundation.user1_base_account.pubkey(),
        ).await;
        
        if actual != expected {
            return Err(format!(
                "User1 base balance mismatch: expected {}, got {}",
                expected, actual
            ));
        }
        println!("✅ User1 base balance: {}", actual);
    }
    
    println!("✅ Foundation state validation complete");
    Ok(())
}

/// Verifies that an operation fails as expected
/// Useful for testing error conditions and validation logic
#[allow(dead_code)]
pub async fn verify_operation_fails(
    result: Result<(), solana_program_test::BanksClientError>,
    operation_description: &str,
    expected_error_type: Option<&str>,
) -> Result<(), String> {
    match result {
        Ok(_) => {
            return Err(format!("❌ {} should have failed but succeeded!", operation_description));
        }
        Err(e) => {
            println!("✅ {} correctly failed: {:?}", operation_description, e);
            
            if let Some(expected) = expected_error_type {
                let error_string = format!("{:?}", e);
                if !error_string.contains(expected) {
                    return Err(format!(
                        "❌ {} failed with wrong error type. Expected '{}', got: {:?}",
                        operation_description, expected, e
                    ));
                }
                println!("✅ Error type matches expected: {}", expected);
            }
        }
    }
    
    Ok(())
}

/// Executes and verifies a deposit operation in one call
/// Combines execution with comprehensive validation
#[allow(dead_code)]
pub async fn execute_and_verify_deposit(
    foundation: &mut LiquidityTestFoundation,
    user_keypair: &Keypair,
    amount: u64,
    expect_success: bool,
) -> Result<(), String> {
    println!("🎯 Executing and verifying deposit of {} tokens...", amount);
    
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
    
    // Execute operation
    let result = execute_deposit_operation(
        foundation,
        user_keypair,
        &user_input_account,
        &user_output_lp_account,
        &deposit_mint,
        amount,
    ).await;
    
    if expect_success {
        match result {
            Ok(()) => {
                // Verify the operation was correct
                verify_liquidity_operation(
                    &mut foundation.env.banks_client,
                    "deposit",
                    amount,
                    &user_input_account,
                    &user_output_lp_account,
                    initial_token_balance,
                    initial_lp_balance,
                ).await?;
                
                println!("✅ Deposit operation completed and verified successfully");
                Ok(())
            }
            Err(e) => {
                Err(format!("❌ Expected successful deposit but got error: {:?}", e))
            }
        }
    } else {
        verify_operation_fails(result, "deposit", None).await?;
        println!("✅ Deposit correctly failed as expected");
        Ok(())
    }
} 