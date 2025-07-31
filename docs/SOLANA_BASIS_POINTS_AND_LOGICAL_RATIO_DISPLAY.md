# Solana Basis Points and Logical Ratio Display

**File:** `docs/SOLANA_BASIS_POINTS_AND_LOGICAL_RATIO_DISPLAY.md`  
**Purpose:** Document logical ratio display principles for Solana token pairs  
**Audience:** Developers, AI systems, and future maintainers  
**Last Updated:** 2025-01-28

## 🎯 Core Principle

**Users want to see logical, intuitive ratios with the more valuable asset first.**

People do NOT want to see: `1:0.0001`  
People DO want to see: `1:1000`  

Both represent the same mathematical relationship, but one is user-friendly and the other is confusing.

## 📊 The Solana Basis Points Challenge

### Problem: Raw Basis Points vs. Display Values

Solana stores token amounts in **basis points** (smallest indivisible units), but users think in **display units** (whole tokens adjusted for decimals).

#### Example: MST/TS Pool
```
Contract Storage (Basis Points):
- MST Ratio: 1000 basis points
- TS Ratio: 10000 basis points  
- MST Decimals: 0
- TS Decimals: 4

Actual Display Values:
- MST: 1000 ÷ 10^0 = 1000 whole tokens
- TS: 10000 ÷ 10^4 = 1 whole token

Raw Math Result: 1000:1 (TS is more valuable)
Logical Display: "1 TS = 1000 MST" ✅
Bad Display: "1 MST = 0.001 TS" ❌
```

### Why This Happens

The ratio `1000:10000` **may not be as it seems** because:
1. **Numerator (MST)**: 1000 basis points ÷ 0 decimals = 1000 display units
2. **Denominator (TS)**: 10000 basis points ÷ 4 decimals = 1 display unit
3. **Actual ratio**: 1000:1 (MST:TS in display units)
4. **More valuable asset**: TS (1 TS buys 1000 MST)

## 🧠 Logical Display Rules

### Rule 1: More Valuable Asset First
The asset that has higher purchasing power should be displayed first.

```
Examples:
✅ "1 BTC = 43,000 USD" (BTC is more valuable)
✅ "1 SOL = 160 USDT" (SOL is more valuable)  
✅ "1 TS = 1000 MST" (TS is more valuable)

❌ "1 USD = 0.000023 BTC" (confusing fractions)
❌ "1 USDT = 0.00625 SOL" (confusing fractions)
❌ "1 MST = 0.001 TS" (confusing fractions)
```

### Rule 2: Avoid Confusing Fractions
Users should see whole numbers or clean decimals, not tiny fractions.

```
✅ "1:1000" or "2:2039"
❌ "1:0.0001" or "2039:2"
```

### Rule 3: Logical Ordering
Even for complex ratios, maintain logical ordering:

```
✅ "2 ETH = 2039.239 USDT" (ETH more valuable, clean format)
❌ "2039.239 USDT = 2 ETH" (backwards, starts with large number)
```

## 🔢 Technical Implementation Guidelines

### Step 1: Convert Basis Points to Display Units
```javascript
// Always convert basis points to actual display values first
const tokenA_display = tokenA_basisPoints / Math.pow(10, tokenA_decimals);
const tokenB_display = tokenB_basisPoints / Math.pow(10, tokenB_decimals);
```

### Step 2: Determine More Valuable Asset
```javascript
// Calculate exchange rates in both directions
const rate_A_to_B = tokenB_display / tokenA_display;
const rate_B_to_A = tokenA_display / tokenB_display;

// More valuable asset produces rates >= 1 when converting TO it
const A_more_valuable = rate_B_to_A >= 1;
const B_more_valuable = rate_A_to_B >= 1;
```

### Step 3: Display Logically
```javascript
if (rate_A_to_B >= 1) {
    // A is more valuable: "1 A = X B"
    display = `1 ${tokenA} = ${formatClean(rate_A_to_B)} ${tokenB}`;
} else {
    // B is more valuable: "1 B = X A"  
    display = `1 ${tokenB} = ${formatClean(rate_B_to_A)} ${tokenA}`;
}
```

## 📋 Examples and Test Cases

### Example 1: BTC/USD Pool
```
Basis Points: BTC(100), USD(4300000)
Decimals: BTC(8), USD(6)
Display Units: BTC(0.000001), USD(4.3)
Rate: 1 BTC = 4,300,000 USD ✅
NOT: 1 USD = 0.0000002326 BTC ❌
```

### Example 2: SOL/USDT Pool
```
Basis Points: SOL(100), USDT(16000)  
Decimals: SOL(9), USDT(6)
Display Units: SOL(0.0000001), USDT(0.016)
Rate: 1 SOL = 160 USDT ✅
NOT: 1 USDT = 0.00625 SOL ❌
```

### Example 3: One-to-Many Flag Pools
```
Basis Points: MST(1000), TS(10000)
Decimals: MST(0), TS(4)
Display Units: MST(1000), TS(1)
Rate: 1 TS = 1000 MST ✅
Flag: ONE_TO_MANY_RATIO set ✅
```

## 🚨 Common Mistakes to Avoid

### ❌ Mistake 1: Using Raw Basis Points
```javascript
// WRONG: Using basis points directly
const ratio = tokenA_basisPoints / tokenB_basisPoints; // 1000/10000 = 0.1
display = `1 ${tokenA} = ${ratio} ${tokenB}`; // "1 MST = 0.1 TS" ❌
```

### ❌ Mistake 2: Ignoring Decimals
```javascript
// WRONG: Not accounting for decimal differences
const ratio = 1000 / 10000; // Ignores that MST(0 decimals) vs TS(4 decimals)
```

### ❌ Mistake 3: Backwards Display
```javascript
// WRONG: Showing less valuable asset first
display = "1000 MST = 1 TS"; // Backwards! ❌
// CORRECT: Show more valuable asset first
display = "1 TS = 1000 MST"; // ✅
```

## 🛠️ Implementation Checklist

For any token pair display implementation:

- [ ] **Convert basis points to display units** using proper decimals
- [ ] **Calculate both direction exchange rates** (A→B and B→A)
- [ ] **Identify more valuable asset** (rate ≥ 1 when converting TO it)
- [ ] **Display more valuable asset first** (base token)
- [ ] **Use clean formatting** (avoid tiny fractions)
- [ ] **Test with real data** from state.json or contract
- [ ] **Verify one-to-many flag handling** for special pools

## 📚 Related Files

### Implementation Files
- `dashboard/utils.js` - Token display utility functions
- `dashboard/dashboard.js` - Main dashboard pool display
- `dashboard/liquidity.js` - Liquidity page display
- `dashboard/swap.js` - Swap page display

### Test Data
- `dashboard/state.json` - Contains ratio_a_actual and ratio_b_actual for verification
- Pool flags and decimal information for testing

### Documentation
- `docs/ONE_TO_MANY_POOL_DISPLAY_RULES.md` - Specific rules for one-to-many pools
- This document - General principles for all pool displays

## 🔄 Validation Process

### Step 1: Manual Verification
Check `state.json` ratio_a_actual and ratio_b_actual values:
- These are calculated display values for verification
- Your UI should match these logical ratios

### Step 2: User Testing
- Does the display make intuitive sense?
- Would a typical user understand the value relationship?
- Are we avoiding confusing fractions?

### Step 3: Edge Case Testing
- Very large ratios (1:1,000,000)
- Very small ratios that should be flipped (0.000001:1 → 1:1,000,000)
- Mixed decimal scenarios (0 decimals vs 18 decimals)

---

## 💡 Summary

**The goal is user-friendly, logical display that makes economic sense.**

Users want to see exchange rates that answer: "How much of the cheaper asset can I get for 1 unit of the expensive asset?"

This is NOT about mathematical precision - it's about **human comprehension and usability**.

---

**Remember: If users see confusing fractions or backwards ratios, the implementation is wrong, regardless of mathematical correctness.**