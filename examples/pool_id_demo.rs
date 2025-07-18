//! # Pool ID Example
//! 
//! This example demonstrates how to derive pool IDs (unique identifiers) for pools
//! before creating them. This is useful for:
//! - Checking if a pool already exists
//! - Pre-calculating addresses for UI/UX
//! - Building complex transactions that reference pools

use fixed_ratio_trading::{
    client_sdk::{PoolClient, PoolConfig},
    ID as PROGRAM_ID,
};
use solana_program::pubkey::Pubkey;

fn main() {
    println!("🎯 Pool ID Derivation Demo");
    println!("=========================");
    
    // Create some example token mints
    let usdc_mint = Pubkey::new_from_array([1; 32]);
    let sol_mint = Pubkey::new_from_array([2; 32]);
    
    // Example 1: Using the client SDK
    println!("\n📋 Method 1: Using PoolClient");
    let pool_client = PoolClient::new(PROGRAM_ID);
    
    let config = PoolConfig {
        multiple_token_mint: usdc_mint,
        base_token_mint: sol_mint,
        ratio_a_numerator: 1000,
        ratio_b_denominator: 1,
    };
    
    let pool_id_1 = pool_client.derive_pool_id(&config);
    println!("   Pool ID: {}", pool_id_1);
    
    // Example 2: Demonstrate normalization (order doesn't matter)
    println!("\n📋 Method 2: Demonstrating token order normalization");
    
    // Swap the token order - should get the same pool ID
    let config_swapped = PoolConfig {
        multiple_token_mint: sol_mint,      // Swapped
        base_token_mint: usdc_mint,         // Swapped
        ratio_a_numerator: 1000,
        ratio_b_denominator: 1,
    };
    
    let pool_id_2 = pool_client.derive_pool_id(&config_swapped);
    println!("   Pool ID (swapped order): {}", pool_id_2);
    assert_eq!(pool_id_1, pool_id_2);
    println!("   ✅ Same Pool ID regardless of token parameter order!");
    
    // Example 3: Different ratios produce different pools
    println!("\n📋 Method 3: Different ratios = different pools");
    
    let config_different_ratio = PoolConfig {
        multiple_token_mint: usdc_mint,
        base_token_mint: sol_mint,
        ratio_a_numerator: 2000,  // Different ratio
        ratio_b_denominator: 1,
    };
    
    let pool_id_3 = pool_client.derive_pool_id(&config_different_ratio);
    println!("   Pool ID (2000:1 ratio): {}", pool_id_3);
    assert_ne!(pool_id_1, pool_id_3);
    println!("   ✅ Different ratios produce different Pool IDs!");
    
    println!("\n🎉 Pool ID derivation demo completed!");
    println!("\n💡 Key Takeaways:");
    println!("   • Pool ID = Pool State PDA");
    println!("   • Deterministically derived from pool parameters");
    println!("   • Token order doesn't matter (automatically normalized)");
    println!("   • Different ratios = different pools");
    println!("   • Can be calculated before pool creation");
} 