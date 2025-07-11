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
        (&user1_lp_a_account, &lp_token_a_mint_pda, &user1.pubkey()),
        (&user1_lp_b_account, &lp_token_b_mint_pda, &user1.pubkey()),
        (&user2_primary_account, &primary_mint.pubkey(), &user2.pubkey()),
        (&user2_base_account, &base_mint.pubkey(), &user2.pubkey()),
        (&user2_lp_a_account, &lp_token_a_mint_pda, &user2.pubkey()),
        (&user2_lp_b_account, &lp_token_b_mint_pda, &user2.pubkey()),
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
        
        // Add a small delay every 4 accounts to prevent timeout accumulation
        if i % 4 == 3 {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
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
        
        // Add a small delay every 2 mint operations to prevent timeout accumulation
        if i % 2 == 1 {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }
    
    println!("✅ OPTIMIZED liquidity test foundation created successfully!");
    println!("   - Reduced token amounts for faster processing");
    println!("   - Added micro-delays to prevent timeout accumulation");
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
    // Phase 3: Use main treasury for all operations (specialized treasuries consolidated)
    
    // Create instruction with Phase 8 ultra-optimized account ordering (12 accounts total)
    Ok(Instruction {
        program_id: id(),
        accounts: vec![
            // Phase 8 ultra-optimized account ordering (12 accounts total)
            AccountMeta::new(*user, true),                                          // Index 0: Authority/User Signer
            AccountMeta::new_readonly(solana_program::system_program::id(), false), // Index 1: System Program
            AccountMeta::new_readonly(solana_program::sysvar::clock::id(), false),  // Index 2: Clock Sysvar
            AccountMeta::new(pool_config.pool_state_pda, false),                    // Index 3: Pool State PDA
            AccountMeta::new(pool_config.token_a_vault_pda, false),                 // Index 4: Token A Vault PDA
            AccountMeta::new(pool_config.token_b_vault_pda, false),                 // Index 5: Token B Vault PDA
            AccountMeta::new_readonly(spl_token::id(), false),                      // Index 6: SPL Token Program
            AccountMeta::new(*user_input_token_account, false),                     // Index 7: User Input Token Account
            AccountMeta::new(*user_output_lp_account, false),                       // Index 8: User Output LP Token Account
            AccountMeta::new(main_treasury_pda, false),                             // Index 9: Main Treasury PDA
            AccountMeta::new(*lp_token_a_mint, false),                              // Index 10: LP Token A Mint
            AccountMeta::new(*lp_token_b_mint, false),                              // Index 11: LP Token B Mint
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
    // Phase 3: Use main treasury for all operations (specialized treasuries consolidated)
    
    // Create instruction with Phase 8 ultra-optimized account ordering (12 accounts total)
    Ok(Instruction {
        program_id: id(),
        accounts: vec![
            // Phase 8 ultra-optimized account ordering (12 accounts total)
            AccountMeta::new(*user, true),                                          // Index 0: Authority/User Signer
            AccountMeta::new_readonly(solana_program::system_program::id(), false), // Index 1: System Program
            AccountMeta::new_readonly(solana_program::sysvar::clock::id(), false),  // Index 2: Clock Sysvar
            AccountMeta::new(pool_config.pool_state_pda, false),                    // Index 3: Pool State PDA
            AccountMeta::new(pool_config.token_a_vault_pda, false),                 // Index 4: Token A Vault PDA
            AccountMeta::new(pool_config.token_b_vault_pda, false),                 // Index 5: Token B Vault PDA
            AccountMeta::new_readonly(spl_token::id(), false),                      // Index 6: SPL Token Program
            AccountMeta::new(*user_input_lp_account, false),                        // Index 7: User Input LP Token Account
            AccountMeta::new(*user_output_token_account, false),                    // Index 8: User Output Token Account
            AccountMeta::new(main_treasury_pda, false),                             // Index 9: Main Treasury PDA
            AccountMeta::new(*lp_token_a_mint, false),                              // Index 10: LP Token A Mint
            AccountMeta::new(*lp_token_b_mint, false),                              // Index 11: LP Token B Mint
        ],
        data: serialized,
    })
}

/// Creates a swap instruction with proper standardized account ordering  
/// This will be used for swap operations (to be implemented)
#[allow(dead_code)]
pub fn create_swap_instruction_standardized(
    user: &Pubkey,
    user_input_token_account: &Pubkey,     // Token account being swapped from
    user_output_token_account: &Pubkey,    // Token account receiving swapped tokens
    pool_config: &PoolConfig,
    swap_instruction_data: &PoolInstruction,
) -> Result<Instruction, Box<dyn std::error::Error>> {
    let serialized = swap_instruction_data.try_to_vec()?;
    
    // Derive treasury PDAs
    let (main_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        &id(),
    );
    // Phase 3: Use main treasury for all operations (specialized treasuries consolidated)
    
    // Create instruction with Phase 6 ultra-optimized account ordering (10 accounts for swaps)
    Ok(Instruction {
        program_id: id(),
        accounts: vec![
            // Phase 6 ultra-optimized account ordering (10 accounts total)
            AccountMeta::new(*user, true),                                          // Index 0: Authority/User Signer
            AccountMeta::new_readonly(solana_program::system_program::id(), false), // Index 1: System Program
            AccountMeta::new(pool_config.pool_state_pda, false),                    // Index 2: Pool State PDA
            AccountMeta::new(pool_config.token_a_vault_pda, false),                 // Index 3: Token A Vault PDA
            AccountMeta::new(pool_config.token_b_vault_pda, false),                 // Index 4: Token B Vault PDA
            AccountMeta::new_readonly(spl_token::id(), false),                      // Index 5: SPL Token Program
            AccountMeta::new(*user_input_token_account, false),                     // Index 6: User Input Token Account
            AccountMeta::new(*user_output_token_account, false),                    // Index 7: User Output Token Account
            AccountMeta::new(main_treasury_pda, false),                             // Index 8: Main Treasury PDA
            AccountMeta::new_readonly(solana_program::system_program::id(), false), // Index 9: Placeholder (function-specific)
        ],
        data: serialized,
    })
}

/// Executes a deposit operation using the standardized foundation
/// OPTIMIZED VERSION - adds timeout handling to prevent deadlocks
#[allow(dead_code)]
pub async fn execute_deposit_operation(
    foundation: &mut LiquidityTestFoundation,
    user_keypair: &Keypair,
    user_input_token_account: &Pubkey,
    user_output_lp_account: &Pubkey,
    deposit_token_mint: &Pubkey,
    amount: u64,
) -> TestResult {
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
    
    // Add timeout handling to prevent deadlocks
    let timeout_duration = std::time::Duration::from_secs(10); // 10 second timeout
    let process_future = foundation.env.banks_client.process_transaction(deposit_tx);
    
    match tokio::time::timeout(timeout_duration, process_future).await {
        Ok(result) => result?,
        Err(_) => return Err(solana_program_test::BanksClientError::Io(
            std::io::Error::new(std::io::ErrorKind::TimedOut, "Deposit operation timed out")
        ).into()),
    }
    
    // Small delay to prevent rapid-fire requests
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
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
    
    // Add timeout handling to prevent deadlocks
    let timeout_duration = std::time::Duration::from_secs(10); // 10 second timeout
    let process_future = foundation.env.banks_client.process_transaction(withdrawal_tx);
    
    match tokio::time::timeout(timeout_duration, process_future).await {
        Ok(result) => result?,
        Err(_) => return Err(solana_program_test::BanksClientError::Io(
            std::io::Error::new(std::io::ErrorKind::TimedOut, "Withdrawal operation timed out")
        ).into()),
    }
    
    // Small delay to prevent rapid-fire requests
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
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
            if !state.is_initialized {
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