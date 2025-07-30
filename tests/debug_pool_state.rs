use solana_program_test::*;
use fixed_ratio_trading::state::PoolState;
use borsh::BorshDeserialize;
use spl_token::state::Mint;
use solana_sdk::program_pack::Pack;

mod common;

#[tokio::test]
async fn debug_pool_state_data() {
    println!("🔍 ===== DEBUGGING POOL STATE FROM BLOCKCHAIN =====");
    
    // Create the same foundation as the failing test
    let mut foundation = crate::common::liquidity_helpers::create_liquidity_test_foundation(Some(1000)).await.unwrap();
    
    // Skip liquidity for now, just check the initial pool setup
    println!("📊 READING POOL CONFIG FROM FOUNDATION:");
    println!("   • ratio_a_numerator: {}", foundation.pool_config.ratio_a_numerator);
    println!("   • ratio_b_denominator: {}", foundation.pool_config.ratio_b_denominator);
    println!("   • token_a_mint: {}", foundation.pool_config.token_a_mint);
    println!("   • token_b_mint: {}", foundation.pool_config.token_b_mint);
    
    // Read the pool state from blockchain
    let pool_account = foundation.env.banks_client
        .get_account(foundation.pool_config.pool_state_pda)
        .await
        .unwrap()
        .unwrap();
    
    let pool_state = PoolState::deserialize(&mut &pool_account.data[..]).unwrap();
    
    println!("📊 READING POOL STATE FROM BLOCKCHAIN:");
    println!("   • ratio_a_numerator: {}", pool_state.ratio_a_numerator);
    println!("   • ratio_b_denominator: {}", pool_state.ratio_b_denominator);
    println!("   • token_a_mint: {}", pool_state.token_a_mint);
    println!("   • token_b_mint: {}", pool_state.token_b_mint);
    
    // Read token mint data to get decimals
    println!("🪙 READING TOKEN MINT DATA:");
    let token_a_account = foundation.env.banks_client
        .get_account(pool_state.token_a_mint)
        .await
        .unwrap()
        .unwrap();
    let token_a_mint = Mint::unpack_from_slice(&token_a_account.data).unwrap();
    
    let token_b_account = foundation.env.banks_client
        .get_account(pool_state.token_b_mint)
        .await
        .unwrap()
        .unwrap();
    let token_b_mint = Mint::unpack_from_slice(&token_b_account.data).unwrap();
    
    println!("   • Token A decimals: {}", token_a_mint.decimals);
    println!("   • Token B decimals: {}", token_b_mint.decimals);
    
    // Calculate what the ratio means for swaps
    println!("📐 RATIO ANALYSIS:");
    if pool_state.ratio_a_numerator > 0 && pool_state.ratio_b_denominator > 0 {
        let a_to_b_rate = pool_state.ratio_b_denominator as f64 / pool_state.ratio_a_numerator as f64;
        
        println!("   • Stored ratio: {}:{}", pool_state.ratio_a_numerator, pool_state.ratio_b_denominator);
        println!("   • 1 Token A = {} Token B", a_to_b_rate);
        
        // Test the calculation that should happen
        let test_input = 1000u64;
        let expected_output_manual = (test_input as f64) * a_to_b_rate;
        println!("   • For {} Token A → Expected {} Token B", test_input, expected_output_manual);
        
        println!("📋 DIAGNOSIS:");
        println!("   • Test expects: 1000 Token A → 1 Token B (1000:1 ratio)");
        println!("   • Actual pool: 1000 Token A → {} Token B", expected_output_manual);
        
        if (expected_output_manual - 1.0).abs() < 0.001 {
            println!("   ✅ RATIO IS CORRECT!");
            println!("   • Issue must be in calculation logic or decimal handling");
        } else {
            println!("   ❌ RATIO IS WRONG!");
            println!("   • Expected ratio: 1000:1 (ratio_a_numerator=1000, ratio_b_denominator=1)");
            println!("   • Actual ratio: {}:{}", pool_state.ratio_a_numerator, pool_state.ratio_b_denominator);
            println!("   • This explains why we get {} tokens instead of 1", expected_output_manual);
        }
    }
} 