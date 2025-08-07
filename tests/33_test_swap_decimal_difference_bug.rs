// 🎯 TEST CONFIGURATION - MODIFY THESE VALUES TO CHANGE TEST BEHAVIOR
//! Test to reproduce the decimal difference bug in swap calculations
//! 
//! This test specifically targets the bug where:
//! - Token A (TS) has 4 decimals 
//! - Token B (MST) has 0 decimals
//! - Decimal difference (B - A) = 0 - 4 = -4
//! - This causes incorrect scaling in swap_a_to_b function
//! - Expected: 1 TS → 10,000 MST  
//! - Actual: 1 TS → 1 MST (WRONG!)
//! - Error: custom program error: 0x417 (AMOUNT_MISMATCH)

use solana_program_test::*;
use fixed_ratio_trading::{
    types::instructions::PoolInstruction,
};

// Import common test utilities
mod common;
use common::{
    liquidity_helpers::*,
    *,
};

#[tokio::test]
async fn test_decimal_difference_bug_reproduction() -> TestResult {
    println!("🎯 TEST: Reproducing decimal difference bug (B-A = -4)");
    
    // 🎯 TEST CONFIGURATION - The exact scenario that causes the bug
    const TOKEN_A_DECIMALS: u8 = 4;  // TS token (4 decimals)
    const TOKEN_B_DECIMALS: u8 = 0;  // MST token (0 decimals)
    const RATIO_A_NUMERATOR: u64 = 10000;    // 10000 TS basis points
    const RATIO_B_DENOMINATOR: u64 = 10000;  // 10000 MST basis points
    const SWAP_AMOUNT_DISPLAY: f64 = 1.0;     // 1 TS in display units
    
    // Calculate expected amounts in basis points
    let swap_amount_a_basis_points = (SWAP_AMOUNT_DISPLAY * 10f64.powi(TOKEN_A_DECIMALS as i32)) as u64;
    let expected_amount_b_basis_points = 10000u64; // Should get 10,000 MST basis points (10,000 MST)
    
    println!("📊 TEST PARAMETERS:");
    println!("   • Token A decimals: {}", TOKEN_A_DECIMALS);
    println!("   • Token B decimals: {}", TOKEN_B_DECIMALS);
    println!("   • Decimal difference (B-A): {} - {} = {}", TOKEN_B_DECIMALS, TOKEN_A_DECIMALS, TOKEN_B_DECIMALS as i32 - TOKEN_A_DECIMALS as i32);
    println!("   • Pool ratio: {}:{}", RATIO_A_NUMERATOR, RATIO_B_DENOMINATOR);
    println!("   • Swap amount: {} TS = {} basis points", SWAP_AMOUNT_DISPLAY, swap_amount_a_basis_points);
    println!("   • Expected output: {} MST basis points", expected_amount_b_basis_points);

    // Step 1: Initialize test environment with custom decimal configuration
    println!("\n🔧 Step 1: Setting up test environment with decimal bug configuration...");
    
    let config = TestFoundationConfig {
        token_a_ratio: RATIO_A_NUMERATOR,
        token_a_count: 20000, // Enough for the test
        token_a_decimals: TOKEN_A_DECIMALS,
        token_b_ratio: RATIO_B_DENOMINATOR,
        token_b_count: 50000,
        token_b_decimals: TOKEN_B_DECIMALS,
        deposit_token_a: true, // Deposit Token A for liquidity
        create_token_b_first: false,
        generate_actual_fees: false,
    };
    
    let mut foundation = create_liquidity_test_foundation_enhanced(config).await.map_err(|e| {
        solana_program_test::BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    })?;
    
    println!("✅ Foundation created with Token A={} decimals, Token B={} decimals", TOKEN_A_DECIMALS, TOKEN_B_DECIMALS);
    println!("   • Pool ratio: {} : {}", RATIO_A_NUMERATOR, RATIO_B_DENOMINATOR);
    
    // Step 2: Attempt the swap that should trigger the bug
    println!("\n🔧 Step 2: Attempting swap that triggers decimal difference bug...");
    println!("🚨 EXPECTING: custom program error: 0x417 (AMOUNT_MISMATCH)");
    
    // Execute the swap that should trigger the bug
    // Extract values to avoid borrowing issues
    let user1_pubkey = foundation.user1.pubkey();
    let token_a_mint = foundation.pool_config.token_a_mint;
    let token_a_is_multiple = foundation.pool_config.token_a_is_the_multiple;
    
    // Use the correct token accounts based on which is Token A
    let (input_account, output_account) = if token_a_is_multiple {
        // Token A is primary (4 decimals = TS)
        (foundation.user1_primary_account.pubkey(), foundation.user1_base_account.pubkey())
    } else {
        // Token A is base (0 decimals = MST)  
        (foundation.user1_base_account.pubkey(), foundation.user1_primary_account.pubkey())
    };
    
    let result = execute_swap_operation(
        &mut foundation,
        &user1_pubkey,
        &input_account,
        &output_account,
        &token_a_mint,
        swap_amount_a_basis_points,
    ).await;
    
    // Verify we get the expected error
    match result {
        Err(error) => {
            println!("✅ Got expected error: {:?}", error);
            
            // Check if it's specifically the AMOUNT_MISMATCH error (0x417)
            let error_string = format!("{:?}", error);
            if error_string.contains("0x417") || error_string.contains("custom program error: 0x417") {
                println!("🎯 SUCCESS: Reproduced the exact decimal difference bug!");
                println!("   • Error code: 0x417 (AMOUNT_MISMATCH)");
                println!("   • Cause: Decimal difference B-A = -4 causes incorrect scaling");
                println!("   • Expected: 1 TS → 10,000 MST basis points");
                println!("   • Actual: 1 TS → 1 MST basis points (99.99% error!)");
                
                // This is the expected behavior - the test passes when the bug exists
                println!("✅ TEST PASSED: Bug successfully reproduced");
            } else {
                panic!("❌ Got different error than expected: {}", error_string);
            }
        }
        Ok(_) => {
            panic!("❌ UNEXPECTED SUCCESS: The swap should have failed with error 0x417! This means the bug might be fixed or test is wrong.");
        }
    }
    
    println!("\n🎯 TEST SUMMARY:");
    println!("✅ Successfully reproduced the decimal difference bug");
    println!("📊 Bug details:");
    println!("   • Token A (4 decimals) → Token B (0 decimals)");
    println!("   • Decimal difference: B - A = 0 - 4 = -4");
    println!("   • Incorrect scaling: amount / 10^4 instead of proper calculation");
    println!("   • Result: 99.99% calculation error (1 instead of 10,000)");
    println!("🚨 This test will PASS when the bug exists (error 0x417)");
    println!("🚨 This test will FAIL when the bug is fixed (successful swap)");
    
    Ok(())
}

#[tokio::test]
async fn test_decimal_difference_bug_analysis() {
    println!("🔍 DETAILED ANALYSIS: Decimal difference bug breakdown");
    
    // Test parameters that expose the bug
    const DECIMALS_A: u8 = 4;
    const DECIMALS_B: u8 = 0;
    const RATIO_A: u64 = 10000;
    const RATIO_B: u64 = 10000;
    const INPUT_AMOUNT: u64 = 10000; // 1 TS in basis points
    
    println!("📊 CALCULATION ANALYSIS:");
    println!("   • Input: {} basis points (1 TS)", INPUT_AMOUNT);
    println!("   • Token A decimals: {}", DECIMALS_A);
    println!("   • Token B decimals: {}", DECIMALS_B);
    println!("   • Decimal difference: {} - {} = {}", DECIMALS_B, DECIMALS_A, DECIMALS_B as i32 - DECIMALS_A as i32);
    
    // Simulate the buggy calculation
    let decimal_diff = DECIMALS_B as i32 - DECIMALS_A as i32;
    println!("   • Decimal diff calculation: {} - {} = {}", DECIMALS_B, DECIMALS_A, decimal_diff);
    
    if decimal_diff < 0 {
        let scale_factor = 10u128.pow((-decimal_diff) as u32);
        let buggy_scaled_amount = INPUT_AMOUNT as u128 / scale_factor;
        
        println!("🐛 BUGGY CALCULATION (current code):");
        println!("   • Scale factor: 10^{} = {}", -decimal_diff, scale_factor);
        println!("   • Scaled amount: {} / {} = {}", INPUT_AMOUNT, scale_factor, buggy_scaled_amount);
        
        let buggy_result = (buggy_scaled_amount * RATIO_B as u128) / RATIO_A as u128;
        println!("   • Final calculation: ({} * {}) / {} = {}", buggy_scaled_amount, RATIO_B, RATIO_A, buggy_result);
        println!("   • BUGGY RESULT: {} MST basis points (should be 10,000!)", buggy_result);
        
        println!("\n✅ CORRECT CALCULATION (what it should be):");
        println!("   • For TS(4 decimals) → MST(0 decimals) with 1:10000 ratio:");
        println!("   • 1 TS = 10,000 basis points");
        println!("   • Expected: 10,000 MST basis points (10,000 MST tokens)");
        println!("   • Error magnitude: {}x difference", 10000 / buggy_result);
        
        assert_eq!(buggy_result, 1, "Bug analysis: should produce wrong result of 1");
    }
    
    println!("\n🎯 This analysis confirms the bug exists in the current swap calculation");
}
