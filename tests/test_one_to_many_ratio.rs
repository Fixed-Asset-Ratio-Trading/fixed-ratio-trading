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