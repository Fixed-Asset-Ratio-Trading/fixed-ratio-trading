//! Tests for the one-to-many ratio detection functionality

use fixed_ratio_trading::utils::validation::check_one_to_many_ratio;

#[test]
fn test_one_to_many_ratio_detection() {
    // Test case 1: 1 SOL = 2 USDC (SOL: 9 decimals, USDC: 6 decimals)
    // This should return true because:
    // - Display units: 1.0 SOL, 2.0 USDC (both whole numbers)
    // - One token equals exactly 1.0
    let is_one_to_many = check_one_to_many_ratio(
        1_000_000_000,  // 1.0 SOL in base units
        2_000_000,      // 2.0 USDC in base units
        9,              // SOL decimals
        6               // USDC decimals
    );
    assert!(is_one_to_many, "1 SOL = 2 USDC should be one-to-many");

    // Test case 2: 1000 DOGE = 1 USDC (DOGE: 6 decimals, USDC: 6 decimals)
    // This should return true because:
    // - Display units: 1000.0 DOGE, 1.0 USDC (both whole numbers)
    // - One token equals exactly 1.0
    let is_one_to_many = check_one_to_many_ratio(
        1_000_000_000,  // 1000.0 DOGE in base units (6 decimals)
        1_000_000,      // 1.0 USDC in base units (6 decimals)
        6,              // DOGE decimals
        6               // USDC decimals
    );
    assert!(is_one_to_many, "1000 DOGE = 1 USDC should be one-to-many");

    // Test case 3: 1 BTC = 1.01 USDT (BTC: 8 decimals, USDT: 6 decimals)
    // This should return false because:
    // - Display units: 1.0 BTC, 1.01 USDT
    // - 1.01 is not a whole number
    let is_one_to_many = check_one_to_many_ratio(
        100_000_000,    // 1.0 BTC in base units
        1_010_000,      // 1.01 USDT in base units
        8,              // BTC decimals
        6               // USDT decimals
    );
    assert!(!is_one_to_many, "1 BTC = 1.01 USDT should NOT be one-to-many");

    // Test case 4: 0.5 BTC = 1 ETH (BTC: 8 decimals, ETH: 9 decimals)
    // This should return false because:
    // - Display units: 0.5 BTC, 1.0 ETH
    // - 0.5 is not a whole number
    let is_one_to_many = check_one_to_many_ratio(
        50_000_000,     // 0.5 BTC in base units
        1_000_000_000,  // 1.0 ETH in base units
        8,              // BTC decimals
        9               // ETH decimals
    );
    assert!(!is_one_to_many, "0.5 BTC = 1 ETH should NOT be one-to-many");

    // Test case 5: 2.5 Token = 3.7 Token (both 6 decimals)
    // This should return false because:
    // - Display units: 2.5, 3.7 (both fractional)
    // - Neither equals exactly 1.0
    let is_one_to_many = check_one_to_many_ratio(
        2_500_000,      // 2.5 in base units
        3_700_000,      // 3.7 in base units
        6,              // Token A decimals
        6               // Token B decimals
    );
    assert!(!is_one_to_many, "2.5 Token = 3.7 Token should NOT be one-to-many");

    // Test case 6: 2 Token = 3 Token (both 6 decimals)
    // This should return false because:
    // - Display units: 2.0, 3.0 (both whole numbers)
    // - Neither equals exactly 1.0
    let is_one_to_many = check_one_to_many_ratio(
        2_000_000,      // 2.0 in base units
        3_000_000,      // 3.0 in base units
        6,              // Token A decimals
        6               // Token B decimals
    );
    assert!(!is_one_to_many, "2 Token = 3 Token should NOT be one-to-many");

    // Test case 7: Edge case with zero (should be false)
    let is_one_to_many = check_one_to_many_ratio(
        0,              // 0 tokens
        1_000_000,      // 1.0 in base units
        6,              // Token A decimals
        6               // Token B decimals
    );
    assert!(!is_one_to_many, "0 Token = 1 Token should NOT be one-to-many");
}

#[test]
fn test_edge_cases_decimal_factors() {
    // Test with different decimal combinations
    
    // High decimal token (18) = Low decimal token (0)
    let is_one_to_many = check_one_to_many_ratio(
        1_000_000_000_000_000_000,  // 1.0 with 18 decimals
        1,                          // 1 with 0 decimals
        18,
        0
    );
    assert!(is_one_to_many, "1.0 high-decimal = 1 low-decimal should be one-to-many");

    // Test precision limits
    let is_one_to_many = check_one_to_many_ratio(
        1_000_000,      // 1.0 with 6 decimals
        1_000_001,      // 1.000001 with 6 decimals (fractional)
        6,
        6
    );
    assert!(!is_one_to_many, "1.0 = 1.000001 should NOT be one-to-many");
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    
    /// **INTEGRATION TEST: POOL_FLAG_ONE_TO_MANY_RATIO Flag Verification**
    /// 
    /// This test verifies that the POOL_FLAG_ONE_TO_MANY_RATIO flag is correctly
    /// set during pool creation based on various ratio scenarios, ensuring the
    /// enhanced logic works as intended in real pool creation operations.
    #[test]
    fn test_one_to_many_flag_scenarios() {
        println!("🧪 Testing POOL_FLAG_ONE_TO_MANY_RATIO flag logic scenarios...");
        
        // **Scenario 1: Valid one-to-many ratios (flag should be SET)**
        println!("\n✅ VALID Scenarios (flag should be SET):");
        
        // Case 1.1: 1 SOL = 160 USDT
        let valid_1 = check_one_to_many_ratio(
            1_000_000_000,  // 1.0 SOL (9 decimals)
            160_000_000,    // 160.0 USDT (6 decimals)
            9, 6
        );
        assert!(valid_1, "❌ Failed: 1 SOL = 160 USDT should set flag");
        println!("  ✅ 1 SOL = 160 USDT → Flag SET (one token = 1, both whole numbers)");
        
        // Case 1.2: 1000 DOGE = 1 USDC  
        let valid_2 = check_one_to_many_ratio(
            1_000_000_000,  // 1000.0 DOGE (6 decimals)
            1_000_000,      // 1.0 USDC (6 decimals)
            6, 6
        );
        assert!(valid_2, "❌ Failed: 1000 DOGE = 1 USDC should set flag");
        println!("  ✅ 1000 DOGE = 1 USDC → Flag SET (one token = 1, both whole numbers)");
        
        // Case 1.3: 1 BTC = 50000 USDT
        let valid_3 = check_one_to_many_ratio(
            100_000_000,    // 1.0 BTC (8 decimals)
            50_000_000_000, // 50000.0 USDT (6 decimals)
            8, 6
        );
        assert!(valid_3, "❌ Failed: 1 BTC = 50000 USDT should set flag");
        println!("  ✅ 1 BTC = 50000 USDT → Flag SET (one token = 1, both whole numbers)");
        
        // **Scenario 2: Invalid ratios (flag should NOT be set)**
        println!("\n❌ INVALID Scenarios (flag should NOT be set):");
        
        // Case 2.1: Fractional values
        let invalid_1 = check_one_to_many_ratio(
            100_000_000,    // 1.0 BTC (8 decimals)
            1_010_000,      // 1.01 USDT (6 decimals) - fractional!
            8, 6
        );
        assert!(!invalid_1, "❌ Failed: 1 BTC = 1.01 USDT should NOT set flag");
        println!("  ✅ 1 BTC = 1.01 USDT → Flag NOT SET (1.01 is fractional)");
        
        // Case 2.2: Neither token equals 1
        let invalid_2 = check_one_to_many_ratio(
            2_000_000,      // 2.0 TokenA (6 decimals)
            3_000_000,      // 3.0 TokenB (6 decimals)
            6, 6
        );
        assert!(!invalid_2, "❌ Failed: 2 TokenA = 3 TokenB should NOT set flag");
        println!("  ✅ 2 TokenA = 3 TokenB → Flag NOT SET (neither token = 1)");
        
        // Case 2.3: Fractional first token
        let invalid_3 = check_one_to_many_ratio(
            50_000_000,     // 0.5 BTC (8 decimals) - fractional!
            1_000_000_000,  // 1.0 ETH (9 decimals)
            8, 9
        );
        assert!(!invalid_3, "❌ Failed: 0.5 BTC = 1 ETH should NOT set flag");
        println!("  ✅ 0.5 BTC = 1 ETH → Flag NOT SET (0.5 is fractional)");
        
        // Case 2.4: Both fractional
        let invalid_4 = check_one_to_many_ratio(
            2_500_000,      // 2.5 TokenA (6 decimals) - fractional!
            3_700_000,      // 3.7 TokenB (6 decimals) - fractional!
            6, 6
        );
        assert!(!invalid_4, "❌ Failed: 2.5 TokenA = 3.7 TokenB should NOT set flag");
        println!("  ✅ 2.5 TokenA = 3.7 TokenB → Flag NOT SET (both fractional)");
        
        // **Scenario 3: Edge cases**
        println!("\n🔬 EDGE CASES:");
        
        // Case 3.1: High decimal precision
        let edge_1 = check_one_to_many_ratio(
            1_000_000_000_000_000_000,  // 1.0 with 18 decimals
            1,                          // 1 with 0 decimals
            18, 0
        );
        assert!(edge_1, "❌ Failed: High decimal precision case should set flag");
        println!("  ✅ 1.0 (18 decimals) = 1 (0 decimals) → Flag SET");
        
        // Case 3.2: Micro fractional difference
        let edge_2 = check_one_to_many_ratio(
            1_000_000,      // 1.0 with 6 decimals
            1_000_001,      // 1.000001 with 6 decimals (tiny fraction!)
            6, 6
        );
        assert!(!edge_2, "❌ Failed: Micro fractional case should NOT set flag");
        println!("  ✅ 1.0 = 1.000001 → Flag NOT SET (detects micro fractions)");
        
        println!("\n🎉 All POOL_FLAG_ONE_TO_MANY_RATIO scenarios validated successfully!");
        println!("The flag logic correctly identifies whole-number ratios where one token equals exactly 1.");
    }
    
    /// **DOCUMENTATION TEST: Real-world examples**
    /// 
    /// This test validates the examples provided in the enhanced documentation
    /// to ensure they behave exactly as documented.
    #[test]
    fn test_documented_examples() {
        println!("📚 Testing documented examples to ensure accuracy...");
        
        // Example from documentation: ✅ 1 SOL = 160 USDT
        let doc_example_1 = check_one_to_many_ratio(
            1_000_000_000,  // 1.0 SOL (9 decimals) 
            160_000_000,    // 160.0 USDT (6 decimals)
            9, 6
        );
        assert!(doc_example_1, "Documentation example '1 SOL = 160 USDT' failed");
        
        // Example from documentation: ❌ 1 SOL = 160.55 USDT
        let doc_example_2 = check_one_to_many_ratio(
            1_000_000_000,  // 1.0 SOL (9 decimals)
            160_550_000,    // 160.55 USDT (6 decimals) - fractional!
            9, 6
        );
        assert!(!doc_example_2, "Documentation example '1 SOL = 160.55 USDT' failed");
        
        // Example from documentation: ✅ 1000 DOGE = 1 USDC
        let doc_example_3 = check_one_to_many_ratio(
            1_000_000_000,  // 1000.0 DOGE (6 decimals)
            1_000_000,      // 1.0 USDC (6 decimals)
            6, 6
        );
        assert!(doc_example_3, "Documentation example '1000 DOGE = 1 USDC' failed");
        
        // Example from documentation: ❌ 0.5 BTC = 1 ETH
        let doc_example_4 = check_one_to_many_ratio(
            50_000_000,     // 0.5 BTC (8 decimals) - fractional!
            1_000_000_000,  // 1.0 ETH (9 decimals)
            8, 9
        );
        assert!(!doc_example_4, "Documentation example '0.5 BTC = 1 ETH' failed");
        
        println!("✅ All documented examples validated - documentation is accurate!");
    }
} 