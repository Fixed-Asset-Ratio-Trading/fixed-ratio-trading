use solana_program::msg;

/// Test the swap_a_to_b calculation with hardcoded values
/// Expected: 1000 Token A with 1000:1 ratio should produce 1 Token B
fn test_swap_a_to_b_calculation() {
    println!("🧪 Testing swap_a_to_b calculation with hardcoded data:");
    println!("   • Input: 1000 tokens");
    println!("   • Ratio: 1000:1 (ratio_a_numerator=1000, ratio_b_denominator=1)");
    println!("   • Token decimals: both 6 decimals");
    println!("   • Expected output: 1 token");
    
    let result = swap_a_to_b_standalone(
        1000,  // amount_a
        1000,  // ratio_a_numerator  
        1,     // ratio_b_denominator
        6,     // token_a_decimals
        6,     // token_b_decimals
    );
    
    match result {
        Ok(amount) => {
            println!("✅ Calculation succeeded: {} tokens", amount);
            if amount == 1 {
                println!("🎉 CORRECT: Got expected result of 1 token!");
            } else {
                println!("❌ WRONG: Expected 1 token, got {} tokens", amount);
            }
        }
        Err(e) => {
            println!("❌ Calculation failed: {:?}", e);
        }
    }
}

/// Test the swap_b_to_a calculation with hardcoded values  
/// Expected: 1 Token B with 1000:1 ratio should produce 1000 Token A
fn test_swap_b_to_a_calculation() {
    println!("\n🧪 Testing swap_b_to_a calculation with hardcoded data:");
    println!("   • Input: 1 token");
    println!("   • Ratio: 1000:1 (ratio_a_numerator=1000, ratio_b_denominator=1)");
    println!("   • Token decimals: both 6 decimals");
    println!("   • Expected output: 1000 tokens");
    
    let result = swap_b_to_a_standalone(
        1,     // amount_b
        1000,  // ratio_a_numerator
        1,     // ratio_b_denominator
        6,     // token_b_decimals
        6,     // token_a_decimals
    );
    
    match result {
        Ok(amount) => {
            println!("✅ Calculation succeeded: {} tokens", amount);
            if amount == 1000 {
                println!("🎉 CORRECT: Got expected result of 1000 tokens!");
            } else {
                println!("❌ WRONG: Expected 1000 tokens, got {} tokens", amount);
            }
        }
        Err(e) => {
            println!("❌ Calculation failed: {:?}", e);
        }
    }
}

/// Standalone version of swap_a_to_b for testing
fn swap_a_to_b_standalone(
    amount_a: u64,
    ratio_a_numerator: u64,
    ratio_b_denominator: u64,
    token_a_decimals: u8,
    token_b_decimals: u8,
) -> Result<u64, String> {
    println!("🔍 SWAP_A_TO_B STANDALONE DEBUG:");
    println!("   • Input amount_a: {}", amount_a);
    println!("   • ratio_a_numerator: {}", ratio_a_numerator);
    println!("   • ratio_b_denominator: {}", ratio_b_denominator);
    println!("   • token_a_decimals: {}, token_b_decimals: {}", token_a_decimals, token_b_decimals);
    
    // Convert to u128 to prevent overflow during calculation
    let amount_a_base = amount_a as u128;
    
    // Calculate: amount_b = (amount_a * ratio_b_denominator) / ratio_a_numerator
    let numerator = amount_a_base * (ratio_b_denominator as u128);
    let denominator = ratio_a_numerator as u128;
    
    println!("   • Calculation: ({} * {}) / {} = {} / {}", amount_a, ratio_b_denominator, ratio_a_numerator, numerator, denominator);
    
    if denominator == 0 {
        return Err("ratio_a_numerator is zero".to_string());
    }
    
    let amount_b_base = numerator / denominator;
    println!("   • Base result: {}", amount_b_base);
    
    // Handle decimal differences between tokens
    let amount_b_adjusted = if token_b_decimals >= token_a_decimals {
        // Output token has more or equal decimals, scale up
        let scale_factor = 10_u128.pow((token_b_decimals - token_a_decimals) as u32);
        let result = amount_b_base * scale_factor;
        println!("   • Scaling UP: {} * {} = {}", amount_b_base, scale_factor, result);
        result
    } else {
        // Output token has fewer decimals, scale down
        let scale_factor = 10_u128.pow((token_a_decimals - token_b_decimals) as u32);
        let result = amount_b_base / scale_factor;
        println!("   • Scaling DOWN: {} / {} = {}", amount_b_base, scale_factor, result);
        result
    };
    
    println!("   • Final adjusted result: {}", amount_b_adjusted);
    
    // Convert back to u64 and check for overflow
    if amount_b_adjusted > u64::MAX as u128 {
        return Err("Result exceeds u64::MAX".to_string());
    }
    
    Ok(amount_b_adjusted as u64)
}

/// Standalone version of swap_b_to_a for testing
fn swap_b_to_a_standalone(
    amount_b: u64,
    ratio_a_numerator: u64,
    ratio_b_denominator: u64,
    token_b_decimals: u8,
    token_a_decimals: u8,
) -> Result<u64, String> {
    println!("🔍 SWAP_B_TO_A STANDALONE DEBUG:");
    println!("   • Input amount_b: {}", amount_b);
    println!("   • ratio_a_numerator: {}", ratio_a_numerator);
    println!("   • ratio_b_denominator: {}", ratio_b_denominator);
    println!("   • token_b_decimals: {}, token_a_decimals: {}", token_b_decimals, token_a_decimals);
    
    // Convert to u128 to prevent overflow during calculation
    let amount_b_base = amount_b as u128;
    
    // Calculate: amount_a = (amount_b * ratio_a_numerator) / ratio_b_denominator
    let numerator = amount_b_base * (ratio_a_numerator as u128);
    let denominator = ratio_b_denominator as u128;
    
    println!("   • Calculation: ({} * {}) / {} = {} / {}", amount_b, ratio_a_numerator, ratio_b_denominator, numerator, denominator);
    
    if denominator == 0 {
        return Err("ratio_b_denominator is zero".to_string());
    }
    
    let amount_a_base = numerator / denominator;
    println!("   • Base result: {}", amount_a_base);
    
    // Handle decimal differences between tokens
    let amount_a_adjusted = if token_a_decimals >= token_b_decimals {
        // Output token has more or equal decimals, scale up
        let scale_factor = 10_u128.pow((token_a_decimals - token_b_decimals) as u32);
        let result = amount_a_base * scale_factor;
        println!("   • Scaling UP: {} * {} = {}", amount_a_base, scale_factor, result);
        result
    } else {
        // Output token has fewer decimals, scale down
        let scale_factor = 10_u128.pow((token_b_decimals - token_a_decimals) as u32);
        let result = amount_a_base / scale_factor;
        println!("   • Scaling DOWN: {} / {} = {}", amount_a_base, scale_factor, result);
        result
    };
    
    println!("   • Final adjusted result: {}", amount_a_adjusted);
    
    // Convert back to u64 and check for overflow
    if amount_a_adjusted > u64::MAX as u128 {
        return Err("Result exceeds u64::MAX".to_string());
    }
    
    Ok(amount_a_adjusted as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standalone_calculations() {
        println!("🚀 STANDALONE CALCULATION TESTS");
        println!("================================");
        
        test_swap_a_to_b_calculation();
        test_swap_b_to_a_calculation();
        
        println!("\n📋 SUMMARY:");
        println!("These tests verify the calculation logic with known inputs");
        println!("If both pass, the issue is in the data being passed to the functions");
        println!("If they fail, the calculation logic itself needs to be fixed");
    }
} 