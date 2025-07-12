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

//! # CU Measurement Example
//! 
//! This example demonstrates how to use the CU measurement tools to estimate
//! compute units for your process functions.
//! 
//! Run this example with:
//! ```bash
//! cargo run --example cu_measurement_example
//! ```

use fixed_ratio_trading::types::instructions::PoolInstruction;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use borsh::BorshSerialize;

/// Example: Create a simple instruction for CU measurement
fn create_get_pool_info_instruction() -> Instruction {
    let instruction_data = PoolInstruction::GetPoolInfo {};
    
    Instruction {
        program_id: fixed_ratio_trading::id(),
        accounts: vec![
            AccountMeta::new_readonly(Pubkey::new_unique(), false), // Pool state
        ],
        data: instruction_data.try_to_vec().expect("Failed to serialize instruction"),
    }
}

/// Example: Create a swap instruction for CU measurement
fn create_swap_instruction(amount: u64) -> Instruction {
    let instruction_data = PoolInstruction::Swap {
        input_token_mint: Pubkey::new_unique(),
        amount_in: amount,
    };
    
    Instruction {
        program_id: fixed_ratio_trading::id(),
        accounts: vec![
            AccountMeta::new(Pubkey::new_unique(), true),  // User (signer)
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false), // System program
            AccountMeta::new_readonly(Pubkey::new_unique(), false), // System state
            AccountMeta::new(Pubkey::new_unique(), false), // Pool state
            AccountMeta::new_readonly(spl_token::id(), false), // SPL Token program
            AccountMeta::new(Pubkey::new_unique(), false), // Main treasury
            AccountMeta::new(Pubkey::new_unique(), false), // Token A vault
            AccountMeta::new(Pubkey::new_unique(), false), // Token B vault
            AccountMeta::new(Pubkey::new_unique(), false), // User input account
            AccountMeta::new(Pubkey::new_unique(), false), // User output account
        ],
        data: instruction_data.try_to_vec().expect("Failed to serialize instruction"),
    }
}

/// Example: Create an HFT swap instruction for CU measurement
fn create_hft_swap_instruction(amount: u64) -> Instruction {
    let instruction_data = PoolInstruction::SwapHftOptimized {
        input_token_mint: Pubkey::new_unique(),
        amount_in: amount,
    };
    
    Instruction {
        program_id: fixed_ratio_trading::id(),
        accounts: vec![
            AccountMeta::new(Pubkey::new_unique(), true),  // User (signer)
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false), // System program
            AccountMeta::new_readonly(Pubkey::new_unique(), false), // System state
            AccountMeta::new(Pubkey::new_unique(), false), // Pool state
            AccountMeta::new_readonly(spl_token::id(), false), // SPL Token program
            AccountMeta::new(Pubkey::new_unique(), false), // Main treasury
            AccountMeta::new(Pubkey::new_unique(), false), // Token A vault
            AccountMeta::new(Pubkey::new_unique(), false), // Token B vault
            AccountMeta::new(Pubkey::new_unique(), false), // User input account
            AccountMeta::new(Pubkey::new_unique(), false), // User output account
        ],
        data: instruction_data.try_to_vec().expect("Failed to serialize instruction"),
    }
}

/// Calculate static CU estimate based on instruction complexity
fn estimate_cu_static(instruction: &Instruction) -> u64 {
    let base_cost = 2_000; // Base instruction cost
    let account_cost = instruction.accounts.len() as u64 * 100; // ~100 CU per account
    let data_cost = instruction.data.len() as u64 * 10; // ~10 CU per byte
    
    // Add complexity modifiers
    let complexity_modifier = match instruction.data.len() {
        0..=32 => 1.0,      // Simple instructions
        33..=64 => 1.2,     // Medium complexity
        65..=128 => 1.5,    // Complex instructions
        _ => 2.0,           // Very complex
    };
    
    ((base_cost + account_cost + data_cost) as f64 * complexity_modifier) as u64
}

fn main() {
    println!("🔬 CU Measurement Example for Fixed Ratio Trading");
    println!("==================================================\n");
    
    // Example 1: Static CU estimation
    println!("📊 Example 1: Static CU Estimation");
    println!("-----------------------------------");
    
    let pool_info_ix = create_get_pool_info_instruction();
    let swap_ix = create_swap_instruction(1000);
    let hft_swap_ix = create_hft_swap_instruction(1000);
    
    let pool_info_cu = estimate_cu_static(&pool_info_ix);
    let swap_cu = estimate_cu_static(&swap_ix);
    let hft_swap_cu = estimate_cu_static(&hft_swap_ix);
    
    println!("GetPoolInfo estimated CUs: {}", pool_info_cu);
    println!("Regular Swap estimated CUs: {}", swap_cu);
    println!("HFT Swap estimated CUs: {}", hft_swap_cu);
    
    let hft_savings = swap_cu as i64 - hft_swap_cu as i64;
    let savings_percent = (hft_savings as f64 / swap_cu as f64) * 100.0;
    println!("HFT Optimization savings: {} CUs ({:.1}%)", hft_savings, savings_percent);
    
    // Example 2: Instruction analysis
    println!("\n📋 Example 2: Instruction Analysis");
    println!("-----------------------------------");
    
    println!("GetPoolInfo instruction:");
    println!("  - Accounts: {}", pool_info_ix.accounts.len());
    println!("  - Data size: {} bytes", pool_info_ix.data.len());
    println!("  - Expected CU range: 2,000 - 3,000 (view instruction)");
    
    println!("\nRegular Swap instruction:");
    println!("  - Accounts: {}", swap_ix.accounts.len());
    println!("  - Data size: {} bytes", swap_ix.data.len());
    println!("  - Expected CU range: 18,000 - 23,000 (documented)");
    
    println!("\nHFT Swap instruction:");
    println!("  - Accounts: {}", hft_swap_ix.accounts.len());
    println!("  - Data size: {} bytes", hft_swap_ix.data.len());
    println!("  - Expected CU range: 13,000 - 16,000 (documented)");
    
    // Example 3: Cost analysis
    println!("\n💰 Example 3: Cost Analysis");
    println!("----------------------------");
    
    let cu_cost_per_lamport = 1; // Approximate cost per CU
    let sol_price = 100.0; // Example SOL price in USD
    
    println!("Cost per transaction (at {} lamports/CU):", cu_cost_per_lamport);
    println!("  - GetPoolInfo: ~{} lamports (${:.6})", 
             pool_info_cu * cu_cost_per_lamport, 
             (pool_info_cu * cu_cost_per_lamport) as f64 / 1_000_000_000.0 * sol_price);
    
    println!("  - Regular Swap: ~{} lamports (${:.6})", 
             swap_cu * cu_cost_per_lamport, 
             (swap_cu * cu_cost_per_lamport) as f64 / 1_000_000_000.0 * sol_price);
    
    println!("  - HFT Swap: ~{} lamports (${:.6})", 
             hft_swap_cu * cu_cost_per_lamport, 
             (hft_swap_cu * cu_cost_per_lamport) as f64 / 1_000_000_000.0 * sol_price);
    
    // Example 4: Volume analysis
    println!("\n📈 Example 4: Volume Analysis");
    println!("------------------------------");
    
    let daily_swaps = 1000;
    let monthly_swaps = daily_swaps * 30;
    
    let daily_regular_cost = (swap_cu * cu_cost_per_lamport * daily_swaps) as f64 / 1_000_000_000.0 * sol_price;
    let daily_hft_cost = (hft_swap_cu * cu_cost_per_lamport * daily_swaps) as f64 / 1_000_000_000.0 * sol_price;
    let daily_savings = daily_regular_cost - daily_hft_cost;
    
    println!("For {} swaps/day:", daily_swaps);
    println!("  - Regular swap cost: ${:.2}/day", daily_regular_cost);
    println!("  - HFT swap cost: ${:.2}/day", daily_hft_cost);
    println!("  - Daily savings: ${:.2}/day", daily_savings);
    println!("  - Monthly savings: ${:.2}/month", daily_savings * 30.0);
    println!("  - Annual savings: ${:.2}/year", daily_savings * 365.0);
    
    // Example 5: Testing recommendations
    println!("\n🧪 Example 5: Testing Recommendations");
    println!("--------------------------------------");
    
    println!("To measure actual CUs in your tests:");
    println!("1. Use the CU measurement framework:");
    println!("   cargo test test_cu_measurement_comprehensive_report");
    println!();
    println!("2. Run specific instruction measurements:");
    println!("   cargo test test_cu_measurement_swap_comparison");
    println!();
    println!("3. Generate detailed reports:");
    println!("   cargo test test_cu_measurement_comprehensive_report");
    println!("   # Creates: cu_measurement_report.md");
    println!();
    println!("4. Test with different compute budgets:");
    println!("   cargo test test_cu_measurement_config");
    println!();
    println!("5. Benchmark performance:");
    println!("   cargo test test_cu_measurement_benchmark");
    
    println!("\n✅ CU Measurement Example Complete!");
    println!("See docs/CU_ESTIMATION_TOOLS.md for more details.");
} 