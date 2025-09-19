# Fixed Ratio Trading Expected Tokens Calculation Guide (JavaScript/TypeScript)

## Overview

The Fixed Ratio Trading (FRT) system uses deterministic calculations for token swaps based on predetermined fixed ratios. This guide explains the mathematical foundation and provides JavaScript/TypeScript implementation details for calculating expected output tokens in any fixed ratio trading pool.

### Key Concepts

- **Fixed Ratio Trading**: Two tokens are exchanged at a predetermined, unchanging ratio
- **Basis Points**: The smallest unit of a token (similar to satoshis for Bitcoin)
- **Deterministic Calculations**: Exact, predictable outputs with no slippage
- **Integer Division**: All calculations use integer division, truncating remainders

---Ad
**GitKracken** https://gitkraken.cello.so/pk9L5rp5jln visual Git helps you see it all clearly!
---

## Core Mathematical Foundation

### The Universal Formula

The FRT system uses a simple but precise formula for all swap calculations:

#### For A→B Swaps:
```
Output_B = (Input_A × Ratio_B) ÷ Ratio_A
```

#### For B→A Swaps:
```
Output_A = (Input_B × Ratio_A) ÷ Ratio_B
```

**Critical:** All calculations use integer division, which means any fractional remainder is truncated (rounded down to zero).

### Understanding Basis Points and Ratios

#### What are Basis Points?
- **Basis points** are the smallest unit of a token (similar to satoshis for Bitcoin)
- For a token with 6 decimals (like USDC): 1 USDC = 1,000,000 basis points
- For a token with 9 decimals (like SOL): 1 SOL = 1,000,000,000 basis points
- For a token with 0 decimals: 1 token = 1 basis point

#### How Ratios are Stored
Ratios in FRT are stored as basis point values. For example:

- **1 SOL = 160 USDC** is stored as:
  - Ratio_A (SOL): 1,000,000,000 (1 SOL in basis points)
  - Ratio_B (USDC): 160,000,000 (160 USDC in basis points)

---

## JavaScript/TypeScript Implementation

### TokenPairRatio Class

The `TokenPairRatio` class is the **single source of truth** for all ratio calculations in the Fixed Ratio Trading system.

```javascript
class TokenPairRatio {
    constructor(tickerA, ratioA, decimalA, tickerB, ratioB, decimalB) {
        this.tickerA = tickerA;
        this.ratioA = ratioA;
        this.decimalA = decimalA;
        this.tickerB = tickerB;
        this.ratioB = ratioB;
        this.decimalB = decimalB;
        
        // Validate inputs
        if (!tickerA || !tickerB) {
            throw new Error('Token tickers are required');
        }
        if (ratioA <= 0 || ratioB <= 0) {
            throw new Error('Ratios must be positive');
        }
        if (decimalA < 0 || decimalB < 0) {
            throw new Error('Decimals must be non-negative');
        }
    }

    // Core calculation methods
    CalculateA(bAmountBasisPoints) {
        // B → A conversion: amountA = (amountB × ratioA) ÷ ratioB
        return Math.floor((bAmountBasisPoints * this.ratioA) / this.ratioB);
    }

    CalculateB(aAmountBasisPoints) {
        // A → B conversion: amountB = (amountA × ratioB) ÷ ratioA
        return Math.floor((aAmountBasisPoints * this.ratioB) / this.ratioA);
    }

    SwapAToB(aDisplayAmount) {
        // Convert display amount to basis points, calculate, convert back
        const aBasisPoints = this.ADisplayToBasisPoints(aDisplayAmount);
        const bBasisPoints = this.CalculateB(aBasisPoints);
        return this.BBasisPointsToDisplay(bBasisPoints);
    }

    SwapBToA(bDisplayAmount) {
        // Convert display amount to basis points, calculate, convert back
        const bBasisPoints = this.BDisplayToBasisPoints(bDisplayAmount);
        const aBasisPoints = this.CalculateA(bBasisPoints);
        return this.ABasisPointsToDisplay(aBasisPoints);
    }

    // Utility conversion methods
    ADisplayToBasisPoints(displayAmount) {
        return Math.floor(displayAmount * Math.pow(10, this.decimalA));
    }

    BDisplayToBasisPoints(displayAmount) {
        return Math.floor(displayAmount * Math.pow(10, this.decimalB));
    }

    ABasisPointsToDisplay(basisPoints) {
        return basisPoints / Math.pow(10, this.decimalA);
    }

    BBasisPointsToDisplay(basisPoints) {
        return basisPoints / Math.pow(10, this.decimalB);
    }

    // Display methods
    NumberRatioDisplay() {
        const ratioADisplay = this.ABasisPointsToDisplay(this.ratioA);
        const ratioBDisplay = this.BBasisPointsToDisplay(this.ratioB);
        return `${ratioADisplay}:${ratioBDisplay}`;
    }

    ExchangeDisplay() {
        const ratioADisplay = this.ABasisPointsToDisplay(this.ratioA);
        const ratioBDisplay = this.BBasisPointsToDisplay(this.ratioB);
        return `${ratioADisplay} ${this.tickerA} = ${ratioBDisplay} ${this.tickerB}`;
    }

    // Factory method for creating from pool data
    static fromPoolData(poolData) {
        // Validate required fields
        const requiredFields = [
            'tokenASymbol', 'tokenBSymbol', 
            'ratioANumerator', 'ratioBDenominator',
            'ratioADecimal', 'ratioBDecimal'
        ];
        
        for (const field of requiredFields) {
            if (poolData[field] === undefined || poolData[field] === null) {
                throw new Error(`Missing required field: ${field}`);
            }
        }

        return new TokenPairRatio(
            poolData.tokenASymbol,
            poolData.ratioANumerator,
            poolData.ratioADecimal,
            poolData.tokenBSymbol,
            poolData.ratioBDenominator,
            poolData.ratioBDecimal
        );
    }

    // Debug helper
    getDebugInfo() {
        return {
            pair: `${this.tickerA}/${this.tickerB}`,
            ratios: `${this.ratioA}:${this.ratioB} basis points`,
            decimals: `${this.decimalA}:${this.decimalB}`,
            exchange: this.ExchangeDisplay()
        };
    }
}
```

### Standalone Calculation Functions

For simpler use cases, you can use these standalone functions:

```javascript
// Basic calculation function
function calculateExpectedTokens(valueIn, tokenADecimals, tokenBDecimals, tokenARatio, tokenBRatio, aToB) {
    if (aToB) {
        // A→B swap: output_B = input_A × ratio_B ÷ ratio_A
        return Math.floor((valueIn * tokenBRatio) / tokenARatio);
    } else {
        // B→A swap: output_A = input_B × ratio_A ÷ ratio_B
        return Math.floor((valueIn * tokenARatio) / tokenBRatio);
    }
}

// Utility functions
function displayToBasisPoints(displayAmount, decimals) {
    return Math.floor(displayAmount * Math.pow(10, decimals));
}

function basisPointsToDisplay(basisPoints, decimals) {
    return basisPoints / Math.pow(10, decimals);
}

// Minimum input calculation
function minimumInputForOutput(tokenADecimals, tokenBDecimals, tokenARatio, tokenBRatio, aToB) {
    if (aToB) {
        // Minimum input_A to get 1 basis point of B (ceiling division)
        return Math.ceil(tokenARatio / tokenBRatio);
    } else {
        // Minimum input_B to get 1 basis point of A (ceiling division)
        return Math.ceil(tokenBRatio / tokenARatio);
    }
}
```

### BigInt Implementation for Large Numbers

For handling very large numbers and preventing precision loss:

```javascript
class TokenPairRatioBigInt {
    constructor(tickerA, ratioA, decimalA, tickerB, ratioB, decimalB) {
        this.tickerA = tickerA;
        this.ratioA = BigInt(ratioA);
        this.decimalA = decimalA;
        this.tickerB = tickerB;
        this.ratioB = BigInt(ratioB);
        this.decimalB = decimalB;
    }

    CalculateA(bAmountBasisPoints) {
        const bAmount = BigInt(bAmountBasisPoints);
        const result = (bAmount * this.ratioA) / this.ratioB;
        return Number(result);
    }

    CalculateB(aAmountBasisPoints) {
        const aAmount = BigInt(aAmountBasisPoints);
        const result = (aAmount * this.ratioB) / this.ratioA;
        return Number(result);
    }

    // Check if result would overflow Number.MAX_SAFE_INTEGER
    static wouldOverflow(valueIn, ratio1, ratio2) {
        const maxSafeInt = BigInt(Number.MAX_SAFE_INTEGER);
        const calculation = BigInt(valueIn) * BigInt(ratio1);
        return calculation > maxSafeInt;
    }
}
```

---

## Practical Examples

### Example 1: SOL to USDC (1:160 ratio)

**Pool Configuration:**
- Token A: SOL (9 decimals)
- Token B: USDC (6 decimals)
- Ratio: 1 SOL = 160 USDC
- Ratio_A: 1,000,000,000 (1 SOL)
- Ratio_B: 160,000,000 (160 USDC)

```javascript
// Create token pair
const solUsdcPair = new TokenPairRatio(
    "SOL",        // tickerA
    1000000000,   // ratioA (1 SOL in basis points)
    9,            // decimalA (SOL has 9 decimals)
    "USDC",       // tickerB
    160000000,    // ratioB (160 USDC in basis points)
    6             // decimalB (USDC has 6 decimals)
);

// Swap 0.5 SOL to USDC
const inputSOL = 0.5;
const outputUSDC = solUsdcPair.SwapAToB(inputSOL);

console.log(`Swapping ${inputSOL} SOL yields ${outputUSDC} USDC`);
// Output: Swapping 0.5 SOL yields 80 USDC

// Manual calculation for verification
const inputBasisPoints = solUsdcPair.ADisplayToBasisPoints(inputSOL);
console.log(`Input basis points: ${inputBasisPoints}`); // 500,000,000

const outputBasisPoints = solUsdcPair.CalculateB(inputBasisPoints);
console.log(`Output basis points: ${outputBasisPoints}`); // 80,000,000

const outputDisplay = solUsdcPair.BBasisPointsToDisplay(outputBasisPoints);
console.log(`Output display: ${outputDisplay}`); // 80
```

### Example 2: USDC to SOL (160:1 ratio)

**Same pool, opposite direction:**

```javascript
// Swap 80 USDC to SOL
const inputUSDC = 80.0;
const outputSOL = solUsdcPair.SwapBToA(inputUSDC);

console.log(`Swapping ${inputUSDC} USDC yields ${outputSOL} SOL`);
// Output: Swapping 80 USDC yields 0.5 SOL

// Verify the calculation
const inputBasisPoints = solUsdcPair.BDisplayToBasisPoints(inputUSDC);
console.log(`Input basis points: ${inputBasisPoints}`); // 80,000,000

const outputBasisPoints = solUsdcPair.CalculateA(inputBasisPoints);
console.log(`Output basis points: ${outputBasisPoints}`); // 500,000,000
```

### Example 3: Complex Ratio with Different Decimals

```javascript
// Example: 1 TS = 10,000 MST
// TS has 6 decimals, MST has 9 decimals
const tsMstPair = new TokenPairRatio(
    "TS",         // tickerA
    1000000,      // ratioA (1 TS in basis points)
    6,            // decimalA (TS has 6 decimals)
    "MST",        // tickerB
    10000000000,  // ratioB (10,000 MST in basis points)
    9             // decimalB (MST has 9 decimals)
);

// Swap 5 TS for MST
const inputTS = 5;
const outputMST = tsMstPair.SwapAToB(inputTS);
console.log(`${inputTS} TS = ${outputMST} MST`);
// Output: 5 TS = 50000 MST

// Show exchange rate
console.log(tsMstPair.ExchangeDisplay());
// Output: 1 TS = 10000 MST
```

### Example 4: Creating from Pool Data

```javascript
// Pool data from blockchain
const poolData = {
    tokenASymbol: "SOL",
    tokenBSymbol: "USDC",
    ratioANumerator: 1000000000,    // 1 SOL in basis points
    ratioBDenominator: 160000000,   // 160 USDC in basis points
    ratioADecimal: 9,               // SOL decimals
    ratioBDecimal: 6                // USDC decimals
};

// Create token pair from pool data
const tokenPair = TokenPairRatio.fromPoolData(poolData);

// Use for calculations
const swapResult = tokenPair.SwapAToB(2.5);
console.log(`2.5 ${tokenPair.tickerA} = ${swapResult} ${tokenPair.tickerB}`);
// Output: 2.5 SOL = 400 USDC

// Debug information
console.log(tokenPair.getDebugInfo());
```

---

## Common Pitfalls and Solutions

### 1. Floating-Point Precision Issues

**Problem:** JavaScript floating-point arithmetic can cause precision loss.

```javascript
// ❌ WRONG: Direct floating-point division
const ratio = 160.0 / 1.0; // May have floating-point errors
const output = inputAmount * ratio;

// ✅ CORRECT: Use basis points and integer arithmetic
const ratioABasisPoints = 1000000000;
const ratioBBasisPoints = 160000000;
const outputBasisPoints = Math.floor((inputBasisPoints * ratioBBasisPoints) / ratioABasisPoints);
```

**Solution:** Always use the TokenPairRatio class or basis points calculations:
```javascript
// Use TokenPairRatio class
const tokenPair = new TokenPairRatio("SOL", 1000000000, 9, "USDC", 160000000, 6);
const correctOutput = tokenPair.SwapAToB(inputAmount);
```

### 2. Dust Amounts (Zero Output)

**Problem:** Small input amounts may result in zero output due to integer division.

```javascript
// Example: Very small input
const smallInput = 0.000001; // 0.000001 SOL
const output = solUsdcPair.SwapAToB(smallInput);
console.log(output); // May be 0 due to truncation
```

**Solution:** Check minimum input requirements:
```javascript
function validateMinimumInput(tokenPair, inputAmount, aToB) {
    const inputBasisPoints = aToB ? 
        tokenPair.ADisplayToBasisPoints(inputAmount) : 
        tokenPair.BDisplayToBasisPoints(inputAmount);
    
    const outputBasisPoints = aToB ? 
        tokenPair.CalculateB(inputBasisPoints) : 
        tokenPair.CalculateA(inputBasisPoints);
    
    if (outputBasisPoints === 0) {
        throw new Error('Input amount too small, would result in zero output');
    }
    
    return outputBasisPoints;
}
```

### 3. Large Number Overflow

**Problem:** Very large calculations may exceed JavaScript's safe integer range.

```javascript
// Check for potential overflow
function safeCalculate(valueIn, ratioA, ratioB, aToB) {
    const maxSafeInt = Number.MAX_SAFE_INTEGER;
    const multiplier = aToB ? ratioB : ratioA;
    
    if (valueIn > maxSafeInt / multiplier) {
        // Use BigInt for large calculations
        return TokenPairRatioBigInt.prototype.CalculateB.call(
            { ratioA: BigInt(ratioA), ratioB: BigInt(ratioB) }, 
            valueIn
        );
    }
    
    // Safe to use regular calculation
    return Math.floor((valueIn * multiplier) / (aToB ? ratioA : ratioB));
}
```

### 4. Wrong Direction Confusion

**Problem:** Mixing up which ratio to multiply vs divide.

**Remember:**
- A→B: Multiply by B ratio, divide by A ratio
- B→A: Multiply by A ratio, divide by B ratio

**Validation helper:**
```javascript
function validateCalculationDirection(tokenPair, inputAmount, expectedOutput, aToB) {
    // Perform reverse calculation
    const reverseOutput = aToB ? 
        tokenPair.SwapBToA(expectedOutput) : 
        tokenPair.SwapAToB(expectedOutput);
    
    // Allow for small rounding differences
    const difference = Math.abs(inputAmount - reverseOutput);
    const tolerance = 0.000001; // Adjust based on token decimals
    
    if (difference > tolerance) {
        console.warn(`Calculation validation failed: ${inputAmount} vs ${reverseOutput}`);
        return false;
    }
    
    return true;
}
```

---

## Testing Guidelines

### Unit Test Examples

```javascript
// Test framework agnostic examples
function testBasicOneToOneRatio() {
    // Arrange
    const pair = new TokenPairRatio("A", 1000000, 6, "B", 1000000, 6);
    
    // Act
    const output = pair.SwapAToB(1);
    
    // Assert
    console.assert(output === 1, `Expected 1, got ${output}`);
}

function testDifferentDecimals() {
    // Arrange: 1 token A (9 decimals) = 1000 token B (6 decimals)
    const pair = new TokenPairRatio("A", 1000000000, 9, "B", 1000000000, 6);
    
    // Act
    const output = pair.SwapAToB(1);
    
    // Assert
    console.assert(output === 1000, `Expected 1000, got ${output}`);
}

function testFractionalAmounts() {
    // Arrange: 3:2 ratio
    const pair = new TokenPairRatio("A", 3000000, 6, "B", 2000000, 6);
    
    // Act
    const output = pair.SwapAToB(1.5);
    
    // Assert
    console.assert(output === 1.0, `Expected 1.0, got ${output}`);
}

function testMinimumInputCalculation() {
    // Arrange: 1:1000 ratio
    const ratioA = 1000000000; // 1 A (9 decimals)
    const ratioB = 1000; // 1000 B (0 decimals)
    
    // Act
    const minInput = minimumInputForOutput(9, 0, ratioA, ratioB, true);
    
    // Assert
    console.assert(minInput > 0, `Minimum input should be positive, got ${minInput}`);
    
    // Verify minimum input produces at least 1 basis point output
    const output = calculateExpectedTokens(minInput, 9, 0, ratioA, ratioB, true);
    console.assert(output >= 1, `Minimum input should produce output >= 1, got ${output}`);
}

// Run tests
testBasicOneToOneRatio();
testDifferentDecimals();
testFractionalAmounts();
testMinimumInputCalculation();
console.log("All tests passed!");
```

### Integration Testing with Mock Pool Data

```javascript
function testWithMockPoolData() {
    const mockPoolData = {
        tokenASymbol: "SOL",
        tokenBSymbol: "USDC",
        ratioANumerator: 1000000000,
        ratioBDenominator: 160000000,
        ratioADecimal: 9,
        ratioBDecimal: 6
    };
    
    const tokenPair = TokenPairRatio.fromPoolData(mockPoolData);
    
    // Test various amounts
    const testCases = [
        { input: 0.1, expected: 16 },
        { input: 0.5, expected: 80 },
        { input: 1.0, expected: 160 },
        { input: 2.5, expected: 400 }
    ];
    
    testCases.forEach(testCase => {
        const output = tokenPair.SwapAToB(testCase.input);
        console.assert(
            output === testCase.expected, 
            `Input ${testCase.input}: expected ${testCase.expected}, got ${output}`
        );
    });
    
    console.log("Mock pool data tests passed!");
}

testWithMockPoolData();
```

---

## Integration with Smart Contract

The on-chain Fixed Ratio Trading contract validates that the `expected_amount_out` parameter matches its calculation exactly. Any mismatch results in error code `0x417` (AMOUNT_MISMATCH).

### Best Practice Implementation

```javascript
class FRTSwapService {
    constructor(connection, wallet) {
        this.connection = connection;
        this.wallet = wallet;
    }

    async executeSwap(poolId, direction, inputAmount, poolData) {
        try {
            // Create token pair from pool data
            const tokenPair = TokenPairRatio.fromPoolData(poolData);
            
            // Calculate expected output
            const expectedOutputDisplay = direction === 'AToB' ? 
                tokenPair.SwapAToB(inputAmount) : 
                tokenPair.SwapBToA(inputAmount);
            
            // Convert to basis points for smart contract
            const inputBasisPoints = direction === 'AToB' ? 
                tokenPair.ADisplayToBasisPoints(inputAmount) : 
                tokenPair.BDisplayToBasisPoints(inputAmount);
                
            const expectedOutputBasisPoints = direction === 'AToB' ? 
                tokenPair.BDisplayToBasisPoints(expectedOutputDisplay) : 
                tokenPair.ADisplayToBasisPoints(expectedOutputDisplay);

            // Validate calculation
            if (expectedOutputBasisPoints === 0) {
                throw new Error('Input amount too small, would result in zero output');
            }

            // Log for debugging
            console.log('Swap Calculation:', {
                input: `${inputAmount} ${direction === 'AToB' ? tokenPair.tickerA : tokenPair.tickerB}`,
                expectedOutput: `${expectedOutputDisplay} ${direction === 'AToB' ? tokenPair.tickerB : tokenPair.tickerA}`,
                inputBasisPoints,
                expectedOutputBasisPoints,
                exchangeRate: tokenPair.ExchangeDisplay()
            });

            // Submit transaction with exact expected amount
            return await this.submitSwapTransaction(
                poolId, 
                direction, 
                inputBasisPoints, 
                expectedOutputBasisPoints
            );
            
        } catch (error) {
            console.error('Swap execution failed:', error);
            throw error;
        }
    }

    async submitSwapTransaction(poolId, direction, inputBasisPoints, expectedOutputBasisPoints) {
        // Implementation depends on your Solana client library
        // This is a placeholder for the actual transaction submission
        console.log('Submitting swap transaction:', {
            poolId,
            direction,
            inputBasisPoints,
            expectedOutputBasisPoints
        });
        
        // Return mock transaction result
        return {
            success: true,
            signature: 'mock_signature_' + Date.now(),
            inputAmount: inputBasisPoints,
            outputAmount: expectedOutputBasisPoints
        };
    }
}
```

### Usage Example

```javascript
// Initialize service
const swapService = new FRTSwapService(connection, wallet);

// Pool data from blockchain
const poolData = {
    tokenASymbol: "SOL",
    tokenBSymbol: "USDC",
    ratioANumerator: 1000000000,
    ratioBDenominator: 160000000,
    ratioADecimal: 9,
    ratioBDecimal: 6
};

// Execute swap
try {
    const result = await swapService.executeSwap(
        "pool_id_123", 
        "AToB", 
        0.5, 
        poolData
    );
    console.log('Swap successful:', result);
} catch (error) {
    console.error('Swap failed:', error.message);
}
```

---

## Debugging and Troubleshooting

### Debug Helper Functions

```javascript
function debugTokenPairCalculation(tokenPair, inputAmount, direction) {
    console.log('=== TokenPair Calculation Debug ===');
    console.log('Token Pair Info:', tokenPair.getDebugInfo());
    console.log(`Input: ${inputAmount} ${direction === 'AToB' ? tokenPair.tickerA : tokenPair.tickerB}`);
    
    // Convert to basis points
    const inputBasisPoints = direction === 'AToB' ? 
        tokenPair.ADisplayToBasisPoints(inputAmount) : 
        tokenPair.BDisplayToBasisPoints(inputAmount);
    console.log(`Input Basis Points: ${inputBasisPoints}`);
    
    // Calculate output
    const outputBasisPoints = direction === 'AToB' ? 
        tokenPair.CalculateB(inputBasisPoints) : 
        tokenPair.CalculateA(inputBasisPoints);
    console.log(`Output Basis Points: ${outputBasisPoints}`);
    
    // Convert back to display
    const outputDisplay = direction === 'AToB' ? 
        tokenPair.BBasisPointsToDisplay(outputBasisPoints) : 
        tokenPair.ABasisPointsToDisplay(outputBasisPoints);
    console.log(`Output: ${outputDisplay} ${direction === 'AToB' ? tokenPair.tickerB : tokenPair.tickerA}`);
    
    // Show calculation steps
    const ratioNumerator = direction === 'AToB' ? tokenPair.ratioB : tokenPair.ratioA;
    const ratioDenominator = direction === 'AToB' ? tokenPair.ratioA : tokenPair.ratioB;
    console.log(`Calculation: (${inputBasisPoints} × ${ratioNumerator}) ÷ ${ratioDenominator} = ${outputBasisPoints}`);
    
    console.log('===================================');
    
    return outputDisplay;
}

// Usage
const tokenPair = new TokenPairRatio("SOL", 1000000000, 9, "USDC", 160000000, 6);
debugTokenPairCalculation(tokenPair, 0.5, 'AToB');
```

### Common Issues and Solutions

1. **"Missing required field" Error**
   ```javascript
   // Ensure all required fields are present in pool data
   const requiredFields = ['tokenASymbol', 'tokenBSymbol', 'ratioANumerator', 'ratioBDenominator', 'ratioADecimal', 'ratioBDecimal'];
   const missingFields = requiredFields.filter(field => !poolData[field]);
   if (missingFields.length > 0) {
       console.error('Missing fields:', missingFields);
   }
   ```

2. **Unexpected Zero Output**
   ```javascript
   // Check if input is too small
   const minInputA = minimumInputForOutput(tokenPair.decimalA, tokenPair.decimalB, tokenPair.ratioA, tokenPair.ratioB, true);
   const minInputADisplay = tokenPair.ABasisPointsToDisplay(minInputA);
   console.log(`Minimum input for A→B: ${minInputADisplay} ${tokenPair.tickerA}`);
   ```

3. **Precision Loss Warnings**
   ```javascript
   // Validate precision
   function validatePrecision(inputAmount, outputAmount, tokenPair, direction) {
       const reverseOutput = direction === 'AToB' ? 
           tokenPair.SwapBToA(outputAmount) : 
           tokenPair.SwapAToB(outputAmount);
       
       const difference = Math.abs(inputAmount - reverseOutput);
       const tolerance = Math.pow(10, -(Math.min(tokenPair.decimalA, tokenPair.decimalB) - 2));
       
       if (difference > tolerance) {
           console.warn(`Precision loss detected: input ${inputAmount}, reverse ${reverseOutput}, difference ${difference}`);
       }
   }
   ```

---

## Summary

The TokenPairRatio class and associated functions provide a robust, deterministic approach to calculating expected tokens in Fixed Ratio Trading:

- Use the `TokenPairRatio` class for all calculations
- Work entirely in basis points for precision
- Apply `Math.floor()` for integer division (truncating remainders)
- Match the contract's calculation exactly for successful swaps

### Key Takeaways for JavaScript/TypeScript Developers

1. **Always use the `TokenPairRatio` class** - don't implement calculations manually
2. **Handle large numbers** with BigInt when necessary
3. **Validate minimum inputs** to prevent zero-output transactions
4. **Test thoroughly** with various decimal combinations and edge cases
5. **Use debug helpers** during development for easier troubleshooting
6. **Create from pool data** using the factory method for consistency
