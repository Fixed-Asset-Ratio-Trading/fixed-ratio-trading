# Fixed Ratio Trading - Swap Amount Calculation Guide

**File:** `docs/SWAP_CALCULATION_GUIDE.md`  
**Purpose:** Complete guide for calculating exact swap amounts for any pool ratio  
**Last Updated:** 2025-01-28

## 📋 Table of Contents

1. [Overview](#overview)
2. [Understanding Basis Points](#understanding-basis-points)
3. [Core Swap Formula](#core-swap-formula)
4. [Token Decimals](#token-decimals)
5. [Calculation Steps](#calculation-steps)
6. [Implementation Examples](#implementation-examples)
7. [Special Cases](#special-cases)
8. [Common Pitfalls](#common-pitfalls)
9. [Testing Your Calculations](#testing-your-calculations)

## 🎯 Overview

Fixed Ratio Trading pools use a deterministic swap formula that guarantees exact output amounts based on the pool's fixed ratio. This document explains how to calculate swap amounts correctly for any pool configuration.

### Key Concepts
- **Fixed Ratio**: Pools maintain a constant exchange ratio (e.g., 1 TokenA = 10 TokenB)
- **Basis Points**: All amounts in the smart contract are stored as integers (no decimals)
- **Token Decimals**: Each token has a decimal precision that affects display vs storage
- **No Slippage**: Fixed ratios mean no price impact or slippage

## 🔢 Understanding Basis Points

### What are Basis Points?
Basis points are the smallest indivisible units of a token. They represent the actual integer values stored on the blockchain.

**Examples:**
- 1 SOL (9 decimals) = 1,000,000,000 basis points
- 1 USDC (6 decimals) = 1,000,000 basis points  
- 1 Token (0 decimals) = 1 basis point

### Conversion Formulas

```javascript
// Display Units → Basis Points
basisPoints = displayAmount × 10^decimals

// Basis Points → Display Units
displayAmount = basisPoints ÷ 10^decimals
```

## 📐 Core Swap Formula

The fundamental swap formula for Fixed Ratio Trading is:

### Token A → Token B
```
outputB = (inputA × ratioB) ÷ ratioA
```

### Token B → Token A
```
outputA = (inputB × ratioA) ÷ ratioB
```

**Important:** All values (input, output, ratioA, ratioB) must be in basis points!

## 🔤 Token Decimals

Token decimals determine how basis points translate to display units:

| Token | Decimals | 1 Display Unit | Basis Points |
|-------|----------|----------------|--------------|
| SOL | 9 | 1.0 SOL | 1,000,000,000 |
| USDC | 6 | 1.0 USDC | 1,000,000 |
| BTC | 8 | 1.0 BTC | 100,000,000 |
| Custom | 0 | 1 Token | 1 |

## 📝 Calculation Steps

### Step 1: Gather Pool Data
```javascript
// From pool state
const ratioA_basisPoints = pool.ratioANumerator;    // e.g., 1000
const ratioB_basisPoints = pool.ratioBDenominator;  // e.g., 10000
const tokenA_decimals = pool.ratioADecimal;         // e.g., 0
const tokenB_decimals = pool.ratioBDecimal;         // e.g., 4
```

### Step 2: Convert User Input to Basis Points
```javascript
// User wants to swap 5.0 TokenA
const userInput_display = 5.0;
const userInput_basisPoints = userInput_display * Math.pow(10, tokenA_decimals);
// Result: 5 basis points (if decimals = 0)
```

### Step 3: Calculate Output in Basis Points
```javascript
// Swap A → B
const output_basisPoints = Math.floor(
  (userInput_basisPoints * ratioB_basisPoints) / ratioA_basisPoints
);
```

### Step 4: Convert Output to Display Units
```javascript
const output_display = output_basisPoints / Math.pow(10, tokenB_decimals);
// Result: 0.05 TokenB (if 50000 basis points with 4 decimals)
```

## 💻 Implementation Examples

### Example 1: Simple Integer Ratio (MST/TS Pool)
```javascript
// Pool Configuration
const pool = {
  ratioANumerator: 1000,      // MST ratio (0 decimals)
  ratioBDenominator: 10000,   // TS ratio (4 decimals)
  ratioADecimal: 0,           // MST has 0 decimals
  ratioBDecimal: 4            // TS has 4 decimals
};

// User swaps 500 MST → TS
const input_MST_display = 500;
const input_MST_basisPoints = 500 * Math.pow(10, 0) = 500;

// Calculate output
const output_TS_basisPoints = Math.floor((500 * 10000) / 1000) = 5000;
const output_TS_display = 5000 / Math.pow(10, 4) = 0.5;

// Result: 500 MST → 0.5 TS
```

### Example 2: High Decimal Tokens (SOL/USDT Pool)
```javascript
// Pool Configuration  
const pool = {
  ratioANumerator: 100,         // SOL ratio (9 decimals)
  ratioBDenominator: 16000,     // USDT ratio (6 decimals)
  ratioADecimal: 9,             // SOL has 9 decimals
  ratioBDecimal: 6              // USDT has 6 decimals
};

// User swaps 0.1 SOL → USDT
const input_SOL_display = 0.1;
const input_SOL_basisPoints = 0.1 * Math.pow(10, 9) = 100000000;

// Calculate output
const output_USDT_basisPoints = Math.floor((100000000 * 16000) / 100) = 16000000000;
const output_USDT_display = 16000000000 / Math.pow(10, 6) = 16000;

// Result: 0.1 SOL → 16,000 USDT
```

### Example 3: Using the TokenPairRatio Class
```javascript
// Recommended approach using the centralized class
const ratio = new TokenPairRatio(
  'MST',    // Token A symbol
  1000,     // Token A ratio basis points
  0,        // Token A decimals
  'TS',     // Token B symbol  
  10000,    // Token B ratio basis points
  4         // Token B decimals
);

// Swap 500 MST → TS
const output_TS = ratio.SwapAToB(500);  // Returns 0.5

// Swap 1 TS → MST
const output_MST = ratio.SwapBToA(1);   // Returns 1000
```

## 🎯 Special Cases

### One-to-Many Pools
Pools with the one-to-many flag set follow the same calculation rules but have special display requirements:

```javascript
// Check if pool is one-to-many
const isOneToMany = (pool.flags & 1) !== 0;

if (isOneToMany) {
  // Display must show whole numbers only
  // Example: "1 TS = 1000 MST" not "1 MST = 0.001 TS"
}
```

### Zero Decimal Tokens
Tokens with 0 decimals have a 1:1 ratio between display and basis points:

```javascript
// For 0 decimal tokens
displayAmount === basisPoints
```

### Very Large Numbers
Use BigInt for calculations that might overflow JavaScript's number precision:

```javascript
// For very large amounts
const output = Number(
  (BigInt(input) * BigInt(ratioB)) / BigInt(ratioA)
);
```

## ⚠️ Common Pitfalls

### 1. Mixing Display Units with Basis Points
```javascript
// ❌ WRONG - Mixing units
const output = (5.0 * ratioB) / ratioA;  // 5.0 is display, ratios are basis points

// ✅ CORRECT - All basis points
const input_bp = 5.0 * Math.pow(10, decimals);
const output_bp = (input_bp * ratioB) / ratioA;
```

### 2. Forgetting to Floor Results
```javascript
// ❌ WRONG - Fractional basis points don't exist
const output = (input * ratioB) / ratioA;  // Might be 1234.56

// ✅ CORRECT - Always floor to integer
const output = Math.floor((input * ratioB) / ratioA);  // 1234
```

### 3. Using Wrong Decimal Values
```javascript
// ❌ WRONG - Assuming all tokens have 6 decimals
const basisPoints = amount * 1000000;

// ✅ CORRECT - Use actual token decimals
const basisPoints = amount * Math.pow(10, token.decimals);
```

### 4. Rounding Errors in Display
```javascript
// ❌ WRONG - Too many decimal places can show rounding errors
display = 1.0000000000000002;  

// ✅ CORRECT - Limit display precision
display = parseFloat(amount.toFixed(6));
```

## 🧪 Testing Your Calculations

### Test Case 1: Verify Reversibility
```javascript
// Swapping A→B→A should return (almost) original amount
const startA = 100;
const resultB = swapAToB(startA);
const resultA = swapBToA(resultB);
// resultA should be very close to startA (may lose 1-2 basis points to rounding)
```

### Test Case 2: Compare with Smart Contract
```javascript
// Your calculation should match the contract exactly
const contractFormula = (amountIn * ratioOut) / ratioIn;
const yourFormula = calculateSwap(amountIn, ratioIn, ratioOut);
assert(contractFormula === yourFormula);
```

### Test Case 3: Edge Cases
```javascript
// Test with:
// - Minimum amounts (1 basis point)
// - Maximum safe integers
// - Tokens with different decimal places
// - One-to-many ratios
```

## 📚 Reference Implementation

The dashboard uses these key files for swap calculations:

1. **`utils.js`** - `TokenPairRatio` class (recommended approach)
2. **`swap.js`** - `calculateSwapOutputEnhanced()` function
3. **`data-service.js`** - Pool data parsing

### Quick Reference Code
```javascript
// Complete swap calculation function
function calculateSwapAmount(
  inputAmount_display,
  inputDecimals,
  outputDecimals,
  ratioIn_basisPoints,
  ratioOut_basisPoints
) {
  // Convert display to basis points
  const input_bp = Math.round(inputAmount_display * Math.pow(10, inputDecimals));
  
  // Calculate output in basis points
  const output_bp = Math.floor((input_bp * ratioOut_basisPoints) / ratioIn_basisPoints);
  
  // Convert back to display units
  const output_display = output_bp / Math.pow(10, outputDecimals);
  
  return output_display;
}
```

## 🔗 Related Documentation

- [One-to-Many Pool Display Rules](./codepolicy/ONE_TO_MANY_POOL_DISPLAY_RULES.md)
- [Display Standards Best Practices](./codepolicy/DISPLAY_STANDARDS_BEST_PRACTICES.md)
- [Browser Debugging Guide](./codepolicy/BROWSER_DEBUGGING_GUIDE.md)

---

**Remember:** The key to accurate swap calculations is maintaining consistency between basis points and display units throughout the entire calculation process. Always test your calculations against known good values before using in production!
