# Pool Normalization and Ratio Configuration Guide

**File:** `docs/FRT/POOL_NORMALIZATION_GUIDE.md`  
**Purpose:** Comprehensive guide to understanding pool normalization, ratio configuration, and how to code against it  
**Audience:** Developers, integrators, and smart contract implementers  
**Last Updated:** 2025-07-30  

## 📋 Overview

The Fixed Ratio Trading system uses a sophisticated pool normalization process to ensure consistency, prevent duplicate pools, and maintain mathematical precision across all token pair configurations. This guide explains how normalization works and how to code against it correctly.

## 🏗️ Core Concepts

### **1. What is Pool Normalization?**

Pool normalization is the process of:
- **Lexicographic Token Ordering**: Ensuring tokens are always ordered consistently regardless of input order
- **Ratio Adjustment**: Modifying ratios to match the normalized token order
- **PDA Derivation**: Creating deterministic Program Derived Addresses using normalized values
- **Preventing Duplicates**: Ensuring A/B and B/A pools resolve to the same configuration

### **2. Why Normalization Matters**

```rust
// Without normalization, these would create different pools:
create_pool(USDC, WETH, 1000, 1);  // 1000 USDC = 1 WETH
create_pool(WETH, USDC, 1, 1000);  // 1 WETH = 1000 USDC

// With normalization, both resolve to the same pool configuration
```

## 🔧 Technical Implementation

### **Normalization Process**

The `normalize_pool_config()` function performs these steps:

```rust
pub fn normalize_pool_config(
    multiple_mint: &Pubkey,     // Input: "abundant" token mint
    base_mint: &Pubkey,         // Input: "valuable" token mint  
    ratio_a_numerator: u64,     // Input: multiple token ratio
    ratio_b_denominator: u64,   // Input: base token ratio
) -> PoolConfig {
    // Step 1: Lexicographic token ordering
    let (token_a_mint, token_b_mint) = 
        if multiple_mint.to_bytes() < base_mint.to_bytes() {
            (*multiple_mint, *base_mint)        // multiple becomes Token A
        } else {
            (*base_mint, *multiple_mint)        // multiple becomes Token B
        };
    
    // Step 2: Determine which token is the "multiple"
    let token_a_is_the_multiple = multiple_mint.to_bytes() < base_mint.to_bytes();
    
    // Step 3: Adjust ratios based on reordering
    let (final_ratio_a_numerator, final_ratio_b_denominator) = 
        if token_a_is_the_multiple {
            // Multiple mint became Token A - ratios stay as provided
            (ratio_a_numerator, ratio_b_denominator)
        } else {
            // Multiple mint became Token B - ratios must be swapped!
            (ratio_b_denominator, ratio_a_numerator)
        };
    
    // Step 4: Create deterministic PDA using normalized values
    let (pool_state_pda, _) = Pubkey::find_program_address(&[
        POOL_STATE_SEED_PREFIX,
        token_a_mint.as_ref(),
        token_b_mint.as_ref(), 
        &final_ratio_a_numerator.to_le_bytes(),
        &final_ratio_b_denominator.to_le_bytes(),
    ], &program_id);
    
    // Return normalized configuration
    PoolConfig { ... }
}
```

### **Critical Understanding: Ratio Swapping**

When tokens are reordered during normalization, **ratios must also be swapped**:

```rust
// Input: create_pool(TokenX, TokenY, 100, 1) 
// Meaning: 100 TokenX = 1 TokenY

// Case 1: TokenX < TokenY (lexicographically)
// Result: token_a = TokenX, token_b = TokenY
// Ratios: ratio_a_numerator = 100, ratio_b_denominator = 1
// Pool State: 100A = 1B ✅ CORRECT

// Case 2: TokenX > TokenY (lexicographically) 
// Result: token_a = TokenY, token_b = TokenX  
// Ratios: ratio_a_numerator = 1, ratio_b_denominator = 100  
// Pool State: 1A = 100B ✅ CORRECT (equivalent relationship)
```

## 📊 Swap Calculation Logic

### **Smart Contract Calculation**

The smart contract uses this logic for swap calculations:

```rust
// Determine calculation parameters based on input direction
let (numerator, denominator) = if input_is_token_a {
    // A → B: use stored ratio values directly
    (pool_state.ratio_a_numerator, pool_state.ratio_b_denominator)
} else {
    // B → A: swap the ratio values
    (pool_state.ratio_b_denominator, pool_state.ratio_a_numerator)  
};

// Apply formula: output = input * denominator / numerator
let amount_out = if input_is_token_a {
    swap_a_to_b(amount_in, numerator, denominator, input_decimals, output_decimals)
} else {
    swap_b_to_a(amount_in, denominator, numerator, input_decimals, output_decimals)
};
```

### **Formula Derivation**

For a pool with ratio `ratio_a_numerator : ratio_b_denominator`:

```
Pool Relationship: ratio_a_numerator × Token_A = ratio_b_denominator × Token_B

Token A → Token B:
amount_out_B = (amount_in_A × ratio_b_denominator) / ratio_a_numerator

Token B → Token A:  
amount_out_A = (amount_in_B × ratio_a_numerator) / ratio_b_denominator
```

## 🧪 Testing Against Normalization

### **❌ Incorrect Test Logic (Common Mistake)**

```rust
// WRONG: Assumes specific ratio format
let expected_output = if config.token_a_is_the_multiple {
    input * ratio_b / ratio_a  // Assumes A is always the multiple
} else {
    input * ratio_a / ratio_b  // Wrong assumption
};
```

### **✅ Correct Test Logic**

```rust
// CORRECT: Uses actual pool state values
let a_to_b_output = swap_amount * pool_state.ratio_b_denominator / pool_state.ratio_a_numerator;
let b_to_a_output = swap_amount * pool_state.ratio_a_numerator / pool_state.ratio_b_denominator;

// Verify against actual pool state, not input assumptions
assert_eq!(calculated_output, expected_output, 
          "Should follow pool ratio: {}A = {}B", 
          pool_state.ratio_a_numerator, pool_state.ratio_b_denominator);
```

### **Adaptive Testing Pattern**

```rust
// Read actual pool state after normalization
let pool_state = get_pool_state(&pool_state_pda).await?;
let actual_ratio_a = pool_state.ratio_a_numerator;
let actual_ratio_b = pool_state.ratio_b_denominator;

// Calculate expected values based on actual normalized ratios
let expected_a_to_b = input * actual_ratio_b / actual_ratio_a;
let expected_b_to_a = input * actual_ratio_a / actual_ratio_b;

// Test against normalized reality, not input assumptions
assert_eq!(output, expected_a_to_b, "A→B should follow pool ratio");
```

## 🔍 Common Issues and Solutions

### **Issue 1: Ratio Inversion**

**Problem**: Test expects 100:1 ratio but gets 1:100 behavior

**Root Cause**: Token reordering during normalization caused ratio swapping

**Solution**: Always read actual pool state after creation and test against those values

```rust
// BEFORE: Hardcoded expectation (fails with normalization)
assert_eq!(output, input * 100, "Should multiply by 100");

// AFTER: Dynamic expectation based on actual pool state  
let pool_state = get_pool_state(&pool_pda).await?;
let expected = input * pool_state.ratio_b_denominator / pool_state.ratio_a_numerator;
assert_eq!(output, expected, "Should follow actual pool ratio");
```

### **Issue 2: Duplicate Pool Creation**

**Problem**: Attempting to create pools with swapped token orders

**Solution**: Always normalize before checking if pool exists

```rust
// Normalize configuration first
let config = normalize_pool_config(&token_x, &token_y, ratio_num, ratio_den);

// Check if pool already exists using normalized PDA
if pool_exists(&config.pool_state_pda) {
    return Err("Pool already exists");
}
```

### **Issue 3: PDA Derivation Mismatch**

**Problem**: Client-side PDA derivation doesn't match smart contract

**Solution**: Use the same normalization logic on both client and contract

```rust
// Client-side: Use same normalization as smart contract
let config = normalize_pool_config(&mint_a, &mint_b, ratio_a, ratio_b);
let pool_pda = config.pool_state_pda;

// Smart contract: Uses identical normalization logic
let (pool_pda, _) = Pubkey::find_program_address(&[
    POOL_STATE_SEED_PREFIX,
    config.token_a_mint.as_ref(),
    config.token_b_mint.as_ref(),
    &config.ratio_a_numerator.to_le_bytes(),
    &config.ratio_b_denominator.to_le_bytes(),
], &program_id);
```

## 📚 Best Practices

### **1. Always Normalize First**

```rust
// Do this BEFORE any pool operations
let config = normalize_pool_config(&multiple_mint, &base_mint, ratio_num, ratio_den);

// Use normalized values for all subsequent operations
let pool_pda = config.pool_state_pda;
let token_a = config.token_a_mint;
let token_b = config.token_b_mint;
```

### **2. Test Against Reality, Not Assumptions**

```rust
// Query actual pool state
let pool_state = get_pool_state(&config.pool_state_pda).await?;

// Use actual values for calculations
let output = calculate_swap_output(
    input_amount,
    pool_state.ratio_a_numerator,
    pool_state.ratio_b_denominator,
    is_a_to_b_swap
);
```

### **3. Handle Both Token Orders**

```rust
// Support creating pools with either token order
pub fn create_pool_flexible(
    token_x: &Pubkey,
    token_y: &Pubkey, 
    x_per_y_ratio: u64
) -> Result<PoolConfig> {
    // Normalization handles token ordering automatically
    let config = normalize_pool_config(token_x, token_y, x_per_y_ratio, 1);
    
    // config.token_a_is_the_multiple tells you which token became the "multiple"
    Ok(config)
}
```

### **4. Consistent Ratio Interpretation**

```rust
// Always interpret ratios as: ratio_a_numerator A = ratio_b_denominator B
fn interpret_pool_ratio(pool_state: &PoolState) -> String {
    format!("{}A = {}B", 
           pool_state.ratio_a_numerator, 
           pool_state.ratio_b_denominator)
}
```

## ⚠️ Common Pitfalls

1. **Assuming Token Order**: Never assume which token becomes A or B
2. **Hardcoded Calculations**: Don't hardcode swap formulas based on input order
3. **Ignoring Normalization**: Always normalize before PDA derivation
4. **Test Order Dependency**: Tests should pass regardless of token input order
5. **Multiple Pool Creation**: Check for existing pools using normalized PDAs

## 🎯 Summary

Pool normalization ensures:
- **Consistency**: Same pool regardless of token input order
- **Uniqueness**: No duplicate pools for equivalent configurations  
- **Predictability**: Deterministic PDA generation
- **Mathematical Accuracy**: Preserved ratio relationships after reordering

**Key Takeaway**: Always read actual pool state after normalization and code against those values, not your input assumptions.

---

**Related Documentation:**
- `tests/common/pool_helpers.rs` - Implementation details
- `src/processors/swap.rs` - Smart contract calculation logic
- `tests/32_test_pool_swaps.rs` - Correct testing patterns