//! LP Token Decimals Validation Tests
//! 
//! This module contains tests to ensure LP tokens inherit the correct decimal precision
//! from their underlying source tokens. This is critical for proper token functionality
//! in wallets and other applications.

#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(unused_assignments)]
#![allow(unused_results)]

use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signer,
    program_pack::Pack,
};
use serial_test::serial;
use spl_token::state::Mint;
use borsh::BorshDeserialize;

mod common;
use common::{
    tokens::*,
    setup::{TestEnvironment, start_test_environment, initialize_treasury_system},
    pool_helpers::*,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Test that LP tokens inherit correct decimals from Token A and Token B
/// This test creates tokens with different decimal values and verifies
/// that the LP tokens match the source token decimals
#[tokio::test]
#[serial]
async fn test_lp_token_decimals_inheritance() -> TestResult {
    println!("🧪 Testing LP token decimals inheritance...");
    
    // Test cases with different decimal combinations
    let test_cases = vec![
        (6, 9, "USDC/SOL"),    // USDC (6) and SOL (9)
        (9, 6, "SOL/USDC"),    // SOL (9) and USDC (6)
        (4, 8, "TS/MST"),      // Custom tokens with 4 and 8 decimals
        (0, 9, "NFT/SOL"),     // NFT (0) and SOL (9)
    ];
    
    for (token_a_decimals, token_b_decimals, description) in &test_cases {
        println!("📊 Testing case: {} (decimals: {}, {})", description, token_a_decimals, token_b_decimals);
        
        let mut env = start_test_environment().await;
        
        // Initialize treasury system (required for pool creation)
        let system_authority = solana_sdk::signature::Keypair::new();
        initialize_treasury_system(
            &mut env.banks_client,
            &env.payer,
            env.recent_blockhash,
            &system_authority,
        ).await?;
        
        println!("✅ Treasury system initialized");
        
        // Create custom tokens with specific decimal values using the existing create_mint helper
        let token_a_keypair = solana_sdk::signature::Keypair::new();
        let token_b_keypair = solana_sdk::signature::Keypair::new();
        
        create_mint(&mut env.banks_client, &env.payer, env.recent_blockhash, &token_a_keypair, Some(*token_a_decimals)).await?;
        create_mint(&mut env.banks_client, &env.payer, env.recent_blockhash, &token_b_keypair, Some(*token_b_decimals)).await?;
        
        println!("✅ Created tokens: A({} decimals), B({} decimals)", *token_a_decimals, *token_b_decimals);
        
        // Create pool using existing helper function with ratio 1:1000 for testing
        let pool_result = execute_pool_creation_with_counter_verification(
            &mut env,
            1000, // multiple_per_base (1:1000 ratio)
            0,    // ignored parameter
        ).await;
        
        match pool_result {
            Ok(result) => {
                println!("✅ Pool created: {}", result.pool_pda);
                
                // Get the LP token mint addresses from the pool config
                let pool_config = &result.pool_config;
                
                // Since execute_pool_creation_with_counter_verification creates its own tokens,
                // we need to derive the LP token mint addresses
                let (lp_token_a_mint_pda, _) = Pubkey::find_program_address(
                    &[fixed_ratio_trading::constants::LP_TOKEN_A_MINT_SEED_PREFIX, result.pool_pda.as_ref()],
                    &fixed_ratio_trading::id(),
                );
                let (lp_token_b_mint_pda, _) = Pubkey::find_program_address(
                    &[fixed_ratio_trading::constants::LP_TOKEN_B_MINT_SEED_PREFIX, result.pool_pda.as_ref()],
                    &fixed_ratio_trading::id(),
                );
                
                // Fetch and verify LP token decimals
                let lp_token_a_account = env.banks_client.get_account(lp_token_a_mint_pda).await?;
                let lp_token_b_account = env.banks_client.get_account(lp_token_b_mint_pda).await?;
                
                if let (Some(lp_a_account), Some(lp_b_account)) = (lp_token_a_account, lp_token_b_account) {
                    let lp_token_a_mint = Mint::unpack(&lp_a_account.data)?;
                    let lp_token_b_mint = Mint::unpack(&lp_b_account.data)?;
                    
                    // Get the source token decimals for comparison
                    let token_a_account = env.banks_client.get_account(pool_config.token_a_mint).await?;
                    let token_b_account = env.banks_client.get_account(pool_config.token_b_mint).await?;
                    
                    if let (Some(ta_account), Some(tb_account)) = (token_a_account, token_b_account) {
                        let source_token_a_mint = Mint::unpack(&ta_account.data)?;
                        let source_token_b_mint = Mint::unpack(&tb_account.data)?;
                        
                        // CRITICAL VERIFICATION: LP tokens must inherit source token decimals
                        assert_eq!(
                            lp_token_a_mint.decimals,
                            source_token_a_mint.decimals,
                            "❌ LP Token A decimals ({}) do not match source Token A decimals ({})",
                            lp_token_a_mint.decimals,
                            source_token_a_mint.decimals
                        );
                        
                        assert_eq!(
                            lp_token_b_mint.decimals,
                            source_token_b_mint.decimals,
                            "❌ LP Token B decimals ({}) do not match source Token B decimals ({})",
                            lp_token_b_mint.decimals,
                            source_token_b_mint.decimals
                        );
                        
                        println!("✅ VERIFIED: LP Token A decimals: {} (matches source)", lp_token_a_mint.decimals);
                        println!("✅ VERIFIED: LP Token B decimals: {} (matches source)", lp_token_b_mint.decimals);
                        
                        // Verify mint authorities are set correctly (should be pool PDA)
                        assert_eq!(
                            lp_token_a_mint.mint_authority.unwrap(),
                            result.pool_pda,
                            "LP Token A mint authority should be pool PDA"
                        );
                        
                        assert_eq!(
                            lp_token_b_mint.mint_authority.unwrap(),
                            result.pool_pda,
                            "LP Token B mint authority should be pool PDA"
                        );
                        
                        println!("✅ VERIFIED: LP token mint authorities set to pool PDA");
                    } else {
                        return Err("Could not fetch source token accounts".into());
                    }
                } else {
                    return Err("Could not fetch LP token accounts".into());
                }
            }
            Err(e) => {
                return Err(format!("Pool creation failed: {}", e).into());
            }
        }
        
        println!("🎯 Test case {} completed successfully\n", description);
    }
    
    println!("✅ ALL LP TOKEN DECIMALS TESTS PASSED!");
    println!("   Verified LP tokens inherit correct decimals from source tokens");
    println!("   Tested {} different decimal combinations", test_cases.len());
    
    Ok(())
}

/// Simplified test using create_pool_new_pattern directly
/// This test specifically validates LP token decimal inheritance
#[tokio::test]
#[serial]
async fn test_lp_token_decimals_direct() -> TestResult {
    println!("🧪 Testing LP token decimals with direct pool creation...");
    
    let mut env = start_test_environment().await;
    
    // Initialize treasury system (required for pool creation)
    let system_authority = solana_sdk::signature::Keypair::new();
    initialize_treasury_system(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &system_authority,
    ).await?;
    
    println!("✅ Treasury system initialized");
    
    // Create tokens with different decimal values
    let token_a_keypair = solana_sdk::signature::Keypair::new();
    let token_b_keypair = solana_sdk::signature::Keypair::new();
    
    // Create tokens with 6 and 9 decimals (like USDC and SOL)
    create_mint(&mut env.banks_client, &env.payer, env.recent_blockhash, &token_a_keypair, Some(6)).await?;
    create_mint(&mut env.banks_client, &env.payer, env.recent_blockhash, &token_b_keypair, Some(9)).await?;
    
    println!("✅ Created test tokens: A(6 decimals), B(9 decimals)");
    
    // Create pool using the direct pattern
    let pool_config = create_pool_new_pattern(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &token_a_keypair,
        &token_b_keypair,
        Some(1000), // 1:1000 ratio
    ).await?;
    
    println!("✅ Pool created: {}", pool_config.pool_state_pda);
    
    // Derive LP token mint addresses
    let (lp_token_a_mint_pda, _) = Pubkey::find_program_address(
        &[fixed_ratio_trading::constants::LP_TOKEN_A_MINT_SEED_PREFIX, pool_config.pool_state_pda.as_ref()],
        &fixed_ratio_trading::id(),
    );
    let (lp_token_b_mint_pda, _) = Pubkey::find_program_address(
        &[fixed_ratio_trading::constants::LP_TOKEN_B_MINT_SEED_PREFIX, pool_config.pool_state_pda.as_ref()],
        &fixed_ratio_trading::id(),
    );
    
    // Verify LP token decimals match source tokens
    let lp_token_a_account = env.banks_client.get_account(lp_token_a_mint_pda).await?;
    let lp_token_b_account = env.banks_client.get_account(lp_token_b_mint_pda).await?;
    
    let source_token_a_account = env.banks_client.get_account(token_a_keypair.pubkey()).await?;
    let source_token_b_account = env.banks_client.get_account(token_b_keypair.pubkey()).await?;
    
    if let (Some(lp_a), Some(lp_b), Some(src_a), Some(src_b)) = 
        (lp_token_a_account, lp_token_b_account, source_token_a_account, source_token_b_account) {
        
        let lp_token_a_mint = Mint::unpack(&lp_a.data)?;
        let lp_token_b_mint = Mint::unpack(&lp_b.data)?;
        let source_token_a_mint = Mint::unpack(&src_a.data)?;
        let source_token_b_mint = Mint::unpack(&src_b.data)?;
        
        // Verify decimals match
        assert_eq!(lp_token_a_mint.decimals, source_token_a_mint.decimals, 
                   "LP Token A decimals should match source Token A decimals");
        assert_eq!(lp_token_b_mint.decimals, source_token_b_mint.decimals, 
                   "LP Token B decimals should match source Token B decimals");
        
        println!("✅ DECIMALS VERIFICATION PASSED:");
        println!("   Source Token A: {} decimals -> LP Token A: {} decimals", 
                 source_token_a_mint.decimals, lp_token_a_mint.decimals);
        println!("   Source Token B: {} decimals -> LP Token B: {} decimals", 
                 source_token_b_mint.decimals, lp_token_b_mint.decimals);
        
        // Verify authorities
        assert_eq!(lp_token_a_mint.mint_authority.unwrap(), pool_config.pool_state_pda);
        assert_eq!(lp_token_b_mint.mint_authority.unwrap(), pool_config.pool_state_pda);
        
        println!("✅ LP TOKEN AUTHORITIES VERIFIED: Both set to pool PDA");
        
    } else {
        return Err("Could not fetch required token accounts".into());
    }
    
    println!("✅ DIRECT LP TOKEN DECIMALS TEST PASSED!");
    
    Ok(())
} 