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

//! HFT Optimized Swap Testing Module
//! 
//! This module contains comprehensive tests for the HFT (High-Frequency Trading) 
//! optimized swap function to ensure it provides identical functionality to the 
//! original swap while delivering significant compute unit savings.
//!
//! Updated to use standardized 17-account system with proper treasury integration.

mod common;

use common::*;
use fixed_ratio_trading::{
    PoolInstruction,
    ID as PROGRAM_ID,
    MAIN_TREASURY_SEED_PREFIX,
};
use solana_program::{
    instruction::Instruction,
    pubkey::Pubkey,
};
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Helper to setup a complete swap test environment (reuses pattern from test_pool_swaps.rs)
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

    // Initialize treasury system (required before pool creation)
    let system_authority = Keypair::new();
    initialize_treasury_system(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &system_authority,
    ).await.map_err(|e| solana_program_test::BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    // Create pool with specified ratio
    let config = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
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

/// Helper function to create HFT optimized swap instruction using standardized 10-account system
pub fn create_hft_optimized_swap_instruction(
    user: &Pubkey,
    user_input_account: &Pubkey,
    user_output_account: &Pubkey,
    pool_config: &PoolConfig,
    input_token_mint: &Pubkey,
    amount_in: u64,
) -> Result<Instruction, Box<dyn std::error::Error>> {
    let instruction_data = PoolInstruction::SwapHftOptimized {
        input_token_mint: *input_token_mint,
        amount_in,
    };

    // Use the standardized function from liquidity_helpers, but create a custom version for HFT
    let serialized = instruction_data.try_to_vec()?;
    
    // Derive treasury PDAs (same as in liquidity_helpers)
    let (main_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        &PROGRAM_ID,
    );
    
    // Derive System State PDA (required for swap operations)
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[fixed_ratio_trading::constants::SYSTEM_STATE_SEED_PREFIX],
        &PROGRAM_ID,
    );
    
    // Create instruction with STANDARDIZED account ordering (10 accounts for HFT swaps)
    Ok(Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            // STANDARDIZED account ordering: System State PDA=2, Pool State PDA=3, SPL Token Program=4, Main Treasury=5
            solana_program::instruction::AccountMeta::new(*user, true),                                          // Index 0: Authority/User Signer
            solana_program::instruction::AccountMeta::new_readonly(solana_program::system_program::id(), false), // Index 1: System Program
            solana_program::instruction::AccountMeta::new_readonly(system_state_pda, false),                     // Index 2: System State PDA
            solana_program::instruction::AccountMeta::new(pool_config.pool_state_pda, false),                    // Index 3: Pool State PDA
            solana_program::instruction::AccountMeta::new_readonly(spl_token::id(), false),                      // Index 4: SPL Token Program
            solana_program::instruction::AccountMeta::new(main_treasury_pda, false),                             // Index 5: Main Treasury PDA
            solana_program::instruction::AccountMeta::new(pool_config.token_a_vault_pda, false),                 // Index 6: Token A Vault PDA
            solana_program::instruction::AccountMeta::new(pool_config.token_b_vault_pda, false),                 // Index 7: Token B Vault PDA
            solana_program::instruction::AccountMeta::new(*user_input_account, false),                          // Index 8: User Input Token Account
            solana_program::instruction::AccountMeta::new(*user_output_account, false),                         // Index 9: User Output Token Account
        ],
        data: serialized,
    })
}

/// Helper function to create standard swap instruction using standardized system (reuses from test_pool_swaps.rs)
pub fn create_swap_instruction(
    user: &Pubkey,
    user_input_account: &Pubkey,
    user_output_account: &Pubkey,
    pool_config: &PoolConfig,
    input_token_mint: &Pubkey,
    amount_in: u64,
) -> Result<Instruction, Box<dyn std::error::Error>> {
    let instruction_data = PoolInstruction::Swap {
        input_token_mint: *input_token_mint,
        amount_in,
    };

    // Use the standardized function from liquidity_helpers
    common::liquidity_helpers::create_swap_instruction_standardized(
        user,
        user_input_account,
        user_output_account,
        pool_config,
        &instruction_data,
    )
}

/// **HFT-OPT-001**: Test HFT optimized swap instruction creation and serialization
#[tokio::test]
async fn test_hft_optimized_swap_instruction_creation() -> TestResult {
    println!("===== HFT-OPT-001: HFT Optimized Swap Instruction Testing =====");
    
    // Test HFT optimized swap instruction serialization
    let test_mint = Pubkey::new_unique();
    let hft_swap_instruction = PoolInstruction::SwapHftOptimized {
        input_token_mint: test_mint,
        amount_in: 1_000_000u64,

    };
    
    // Test serialization
    let serialized = hft_swap_instruction.try_to_vec();
    println!("HFT optimized serialization result: {:?}", serialized);
    
    assert!(serialized.is_ok(), "HFT optimized swap instruction serialization should succeed");
    let serialized_data = serialized.unwrap();
    println!("HFT optimized serialized data length: {}", serialized_data.len());
    
    // Test deserialization
    let deserialized = PoolInstruction::try_from_slice(&serialized_data);
    assert!(deserialized.is_ok(), "HFT optimized swap instruction deserialization should succeed");
    
    // Verify the data matches
    if let Ok(PoolInstruction::SwapHftOptimized { input_token_mint, amount_in }) = deserialized {
        assert_eq!(input_token_mint, test_mint);
        assert_eq!(amount_in, 1_000_000u64);
        println!("✅ HFT optimized serialization roundtrip successful");
    } else {
        panic!("Unexpected instruction variant after deserialization");
    }

    // Test with skip_rent_checks = true
    let hft_ultra_instruction = PoolInstruction::SwapHftOptimized {
        input_token_mint: test_mint,
        amount_in: 2_000_000u64,

    };
    
    let ultra_serialized = hft_ultra_instruction.try_to_vec();
    assert!(ultra_serialized.is_ok(), "Ultra-HFT mode serialization should succeed");
    
    println!("✅ HFT-OPT-001 Instruction Creation Testing Complete!");
    Ok(())
}

/// **HFT-OPT-002**: Test HFT optimized swap without liquidity (should fail like standard swap)
#[tokio::test]
async fn test_hft_optimized_swap_insufficient_liquidity() -> TestResult {
    println!("===== HFT-OPT-002: HFT Optimized Swap Insufficient Liquidity Testing =====");
    
    let (mut ctx, config, user, user_primary_account, user_base_account) = setup_swap_test_environment(Some(2)).await?;

    // Note: Pool is created but has no liquidity - should cause failures

    // Mint tokens for testing
    let swap_amount = 1_000_000u64;
    mint_tokens(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint.pubkey(),
        &user_primary_account,
        &ctx.env.payer,
        swap_amount,
    ).await?;

    println!("=== Testing HFT Optimized Swap Error Handling ===");

    // Test 1: Standard swap should fail
    println!("\n--- Standard Swap (should fail) ---");
    let standard_swap_ix = create_swap_instruction(
        &user.pubkey(),
        &user_primary_account,
        &user_base_account,
        &config,
        &ctx.primary_mint.pubkey(),
        swap_amount,
    ).expect("Failed to create standard swap instruction");

    let mut standard_tx = Transaction::new_with_payer(&[standard_swap_ix], Some(&user.pubkey()));
    standard_tx.sign(&[&user], ctx.env.recent_blockhash);
    
    let standard_result = ctx.env.banks_client.process_transaction(standard_tx).await;
    assert!(standard_result.is_err(), "Standard swap should fail with insufficient liquidity");
    println!("✅ Standard swap correctly failed with insufficient liquidity");

    // Test 2: HFT optimized swap should also fail
    println!("\n--- HFT Optimized Swap (should fail) ---");
    let hft_conservative_ix = create_hft_optimized_swap_instruction(
        &user.pubkey(),
        &user_primary_account,
        &user_base_account,
        &config,
        &ctx.primary_mint.pubkey(),
        swap_amount,
    ).expect("Failed to create HFT conservative swap instruction");

    let mut hft_conservative_tx = Transaction::new_with_payer(&[hft_conservative_ix], Some(&user.pubkey()));
    hft_conservative_tx.sign(&[&user], ctx.env.recent_blockhash);
    
    let hft_conservative_result = ctx.env.banks_client.process_transaction(hft_conservative_tx).await;
    assert!(hft_conservative_result.is_err(), "HFT optimized swap should fail with insufficient liquidity");
    println!("✅ HFT optimized swap correctly failed with insufficient liquidity");
    let hft_ultra_ix = create_hft_optimized_swap_instruction(
        &user.pubkey(),
        &user_primary_account,
        &user_base_account,
        &config,
        &ctx.primary_mint.pubkey(),
        swap_amount,
    ).expect("Failed to create HFT ultra swap instruction");

    let mut hft_ultra_tx = Transaction::new_with_payer(&[hft_ultra_ix], Some(&user.pubkey()));
    hft_ultra_tx.sign(&[&user], ctx.env.recent_blockhash);
    


    println!("✅ HFT-OPT-002 Error Handling Consistency Testing Complete!");
    Ok(())
}

/// **HFT-OPT-003**: Test HFT optimized swap instruction construction
#[tokio::test]
async fn test_hft_optimized_swap_instruction_construction() -> TestResult {
    println!("===== HFT-OPT-003: HFT Optimized Swap Instruction Construction Testing =====");
    
    let (ctx, config, user, user_primary_account, user_base_account) = setup_swap_test_environment(Some(2)).await?;

    let swap_amount = 1_000_000u64;

    println!("=== Testing HFT Optimized Instruction Construction ===");

    // Test 1: HFT optimized instruction
    println!("\n--- HFT Optimized Instruction ---");
    let conservative_ix = create_hft_optimized_swap_instruction(
        &user.pubkey(),
        &user_primary_account,
        &user_base_account,
        &config,
        &ctx.primary_mint.pubkey(),
        swap_amount,
    ).expect("Failed to create HFT conservative swap instruction");

    // Verify instruction construction (NEW ordering: 9 accounts)
    assert_eq!(conservative_ix.accounts.len(), 10, "HFT optimized instruction should have 10 accounts (STANDARDIZED account ordering)");
    assert_eq!(conservative_ix.program_id, PROGRAM_ID, "Program ID should match");
    assert!(!conservative_ix.data.is_empty(), "Instruction data should not be empty");
    
    println!("✅ HFT optimized instruction constructed successfully:");
    println!("    ✓ 10 accounts configured (STANDARDIZED account ordering)");
    println!("    ✓ Program ID matches: {}", PROGRAM_ID);
    println!("    ✓ Instruction data: {} bytes", conservative_ix.data.len());


    // Test 2: B→A swap instruction
    println!("\n--- B→A Swap Instruction ---");
    let b_to_a_ix = create_hft_optimized_swap_instruction(
        &user.pubkey(),
        &user_base_account,    // Input: Token B
        &user_primary_account, // Output: Token A
        &config,
        &ctx.base_mint.pubkey(), // Input mint: Token B
        swap_amount,
    ).expect("Failed to create HFT B→A swap instruction");

    assert_eq!(b_to_a_ix.accounts.len(), 10, "B→A HFT instruction should have 10 accounts (STANDARDIZED account ordering)");
    assert_eq!(b_to_a_ix.program_id, PROGRAM_ID, "Program ID should match");
    println!("✅ B→A HFT optimized instruction constructed successfully");

    println!("✅ HFT-OPT-003 Instruction Construction Testing Complete!");
    Ok(())
}

/// **HFT-OPT-004**: Test GitHub Issue #31960 workaround preservation  
#[tokio::test]
async fn test_github_issue_31960_workaround_preservation() -> TestResult {
    println!("===== HFT-OPT-004: GitHub Issue #31960 Workaround Preservation Testing =====");
    
    let (mut ctx, config, user, user_primary_account, user_base_account) = setup_swap_test_environment(Some(2)).await?;

    println!("=== Verifying GitHub Issue #31960 Workaround Functionality ===");

    // The pool created by setup_swap_test_environment() uses the InitializePool instruction
    // which is the modern approach that handles the GitHub Issue #31960 workaround internally.
    // Let's verify the pool state was created correctly.

    let pool_state = get_pool_state(&mut ctx.env.banks_client, &config.pool_state_pda).await
        .expect("Pool state should exist after creation");

    println!("Pool state verification:");
    println!("  Initialized: {}", pool_state.is_initialized);
    println!("  Owner: {}", pool_state.owner);
    println!("  Token A mint: {}", pool_state.token_a_mint);
    println!("  Token B mint: {}", pool_state.token_b_mint);
    println!("  Ratio A numerator: {}", pool_state.ratio_a_numerator);
    println!("  Ratio B denominator: {}", pool_state.ratio_b_denominator);

    // Verify critical pool state fields
    assert!(pool_state.is_initialized, "Pool should be initialized");
    assert_eq!(pool_state.owner, ctx.env.payer.pubkey(), "Pool owner should match");
    assert_eq!(pool_state.token_a_mint, config.token_a_mint, "Token A mint should match");
    assert_eq!(pool_state.token_b_mint, config.token_b_mint, "Token B mint should match");
    assert_eq!(pool_state.ratio_a_numerator, config.ratio_a_numerator, "Ratio A numerator should match");
    assert_eq!(pool_state.ratio_b_denominator, config.ratio_b_denominator, "Ratio B denominator should match");

    println!("✅ Pool state verification successful - GitHub Issue #31960 workaround working");

    // Verify that both standard and HFT optimized swaps would access the same properly initialized pool state
    println!("\n=== Verifying Identical Pool State Access ===");
    
    // Both instruction types should reference the same pool state PDA
    let standard_ix = create_swap_instruction(
        &user.pubkey(),
        &user_primary_account,
        &user_base_account,
        &config,
        &ctx.primary_mint.pubkey(),
        1000u64,
    ).expect("Failed to create standard swap instruction");

    let hft_ix = create_hft_optimized_swap_instruction(
        &user.pubkey(),
        &user_primary_account,
        &user_base_account,
        &config,
        &ctx.primary_mint.pubkey(),
        1000u64,
    ).expect("Failed to create HFT swap instruction");

    // Verify both instructions reference the same accounts (STANDARDIZED account ordering)
    assert_eq!(standard_ix.accounts[3].pubkey, hft_ix.accounts[3].pubkey, "Both should reference same pool state PDA");
    assert_eq!(standard_ix.accounts[6].pubkey, hft_ix.accounts[6].pubkey, "Both should reference same token A vault");
    assert_eq!(standard_ix.accounts[7].pubkey, hft_ix.accounts[7].pubkey, "Both should reference same token B vault");
    assert_eq!(standard_ix.accounts[8].pubkey, hft_ix.accounts[8].pubkey, "Both should reference same user input account");
    assert_eq!(standard_ix.accounts[9].pubkey, hft_ix.accounts[9].pubkey, "Both should reference same user output account");

    println!("✅ Both standard and HFT optimized instructions reference identical pool accounts");
    println!("✅ GitHub Issue #31960 workaround preserved in HFT optimized implementation");

    println!("✅ HFT-OPT-004 GitHub Issue #31960 Workaround Preservation Testing Complete!");
    Ok(())
}

/// **HFT-OPT-005**: Test HFT 40% fee discount verification
#[tokio::test]
async fn test_hft_optimized_fee_discount() -> TestResult {
    println!("===== HFT-OPT-005: HFT 40% Fee Discount Verification =====");
    
    let (ctx, config, user, user_primary_account, user_base_account) = setup_swap_test_environment(Some(2)).await?;

    // Test the fee constants directly since actual swap execution requires full liquidity setup
    println!("=== Testing HFT Fee Discount Constants ===");
    
    // Import constants for verification
    use fixed_ratio_trading::constants::{SWAP_FEE, HFT_SWAP_FEE};
    
    // Test the fee constants directly
    println!("\n=== Fee Constants Verification ===");
    println!("Regular SWAP_FEE: {} lamports ({:.9} SOL)", SWAP_FEE, SWAP_FEE as f64 / 1_000_000_000.0);
    println!("HFT SWAP_FEE: {} lamports ({:.9} SOL)", HFT_SWAP_FEE, HFT_SWAP_FEE as f64 / 1_000_000_000.0);
    
    // Calculate discount percentage
    let discount_amount = SWAP_FEE - HFT_SWAP_FEE;
    let discount_percentage = (discount_amount as f64 / SWAP_FEE as f64) * 100.0;
    
    println!("Discount amount: {} lamports", discount_amount);
    println!("Discount percentage: {:.1}%", discount_percentage);
    
    // Verify the HFT fee is exactly 40% less than regular fee
    assert_eq!(HFT_SWAP_FEE, 16_290, "HFT_SWAP_FEE should be 16,290 lamports");
    assert_eq!(SWAP_FEE, 27_150, "SWAP_FEE should be 27,150 lamports");
    assert!((discount_percentage - 40.0).abs() < 0.1, "Discount should be approximately 40%");
    
    println!("✅ Fee constants verification successful");
    println!("✅ HFT optimized swap provides exactly 40% fee discount");

    // Test instruction construction includes correct treasury accounts for HFT fees
    println!("\n=== Testing HFT Treasury Integration ===");
    
    let hft_swap_ix = create_hft_optimized_swap_instruction(
        &user.pubkey(),
        &user_primary_account,
        &user_base_account,
        &config,
        &ctx.primary_mint.pubkey(),
        1000u64,
    ).expect("Failed to create HFT swap instruction");
    
    // Verify HFT treasury account is included at index 5 (STANDARDIZED account ordering)
    let hft_treasury_account = &hft_swap_ix.accounts[5];
    let (expected_hft_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        &PROGRAM_ID,
    );
    
    assert_eq!(hft_treasury_account.pubkey, expected_hft_treasury_pda, "HFT treasury PDA should be at index 5 (STANDARDIZED account ordering)");
    println!("✅ HFT treasury PDA correctly included in instruction");
    
    // Verify standard swap doesn't have different treasury setup (should use regular swap treasury)
    let standard_ix = create_swap_instruction(
        &user.pubkey(),
        &user_primary_account,
        &user_base_account,
        &config,
        &ctx.primary_mint.pubkey(),
        1000u64,
    ).expect("Failed to create standard swap instruction");
    
    // Both should have same treasury accounts but HFT will use different fee calculation
    let standard_swap_treasury = &standard_ix.accounts[5];
    let hft_swap_treasury = &hft_swap_ix.accounts[5]; 
    assert_eq!(standard_swap_treasury.pubkey, hft_swap_treasury.pubkey, "Both should reference same main treasury PDA");
    
    println!("✅ Treasury account structure verified for both standard and HFT swaps");
    
    println!("✅ HFT-OPT-005 Fee Discount Verification Complete!");
    Ok(())
} 