# Fixed Ratio Trading Expected Tokens Calculation Guide (C#/.NET)

## Overview

The Fixed Ratio Trading (FRT) system uses deterministic calculations for token swaps based on predetermined fixed ratios. This guide explains the mathematical foundation and provides C#/.NET implementation details for calculating expected output tokens in any fixed ratio trading pool.

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

## C#/.NET Implementation

### Core Calculation Method

```csharp
public static class FRTExpectedTokens
{
    /// <summary>
    /// Calculates expected output tokens for Fixed Ratio Trading swaps
    /// </summary>
    /// <param name="valueIn">Input amount in basis points</param>
    /// <param name="tokenADecimals">Token A decimal places</param>
    /// <param name="tokenBDecimals">Token B decimal places</param>
    /// <param name="tokenARatio">Token A ratio in basis points</param>
    /// <param name="tokenBRatio">Token B ratio in basis points</param>
    /// <param name="aToB">True for A→B swap, false for B→A swap</param>
    /// <returns>Expected output amount in basis points</returns>
    public static ulong Calculate(
        ulong valueIn,
        int tokenADecimals,
        int tokenBDecimals,
        ulong tokenARatio,
        ulong tokenBRatio,
        bool aToB)
    {
        if (aToB)
        {
            // A→B swap: output_B = input_A × ratio_B ÷ ratio_A
            return checked((valueIn * tokenBRatio) / tokenARatio);
        }
        else
        {
            // B→A swap: output_A = input_B × ratio_A ÷ ratio_B
            return checked((valueIn * tokenARatio) / tokenBRatio);
        }
    }

    /// <summary>
    /// Calculates minimum input required to produce at least 1 basis point of output
    /// </summary>
    public static ulong MinimumInputForOutput(
        int tokenADecimals,
        int tokenBDecimals,
        ulong tokenARatio,
        ulong tokenBRatio,
        bool aToB)
    {
        if (aToB)
        {
            // Minimum input_A to get 1 basis point of B
            return (tokenARatio + tokenBRatio - 1) / tokenBRatio; // Ceiling division
        }
        else
        {
            // Minimum input_B to get 1 basis point of A
            return (tokenBRatio + tokenARatio - 1) / tokenARatio; // Ceiling division
        }
    }

    /// <summary>
    /// Converts display amount to basis points
    /// </summary>
    public static ulong DisplayToBasisPoints(decimal displayAmount, int decimals)
    {
        return (ulong)(displayAmount * (decimal)Math.Pow(10, decimals));
    }

    /// <summary>
    /// Converts basis points to display amount
    /// </summary>
    public static decimal BasisPointsToDisplay(ulong basisPoints, int decimals)
    {
        return (decimal)basisPoints / (decimal)Math.Pow(10, decimals);
    }
}
```

### Advanced Implementation with BigInteger

For handling very large numbers and preventing overflow:

```csharp
using System.Numerics;

public static class FRTExpectedTokensAdvanced
{
    public static ulong CalculateWithBigInteger(
        ulong valueIn,
        ulong tokenARatio,
        ulong tokenBRatio,
        bool aToB)
    {
        BigInteger input = new BigInteger(valueIn);
        BigInteger ratioNumerator = aToB ? new BigInteger(tokenBRatio) : new BigInteger(tokenARatio);
        BigInteger ratioDenominator = aToB ? new BigInteger(tokenARatio) : new BigInteger(tokenBRatio);

        BigInteger result = (input * ratioNumerator) / ratioDenominator;
        
        if (result > ulong.MaxValue)
            throw new OverflowException("Calculation result exceeds ulong.MaxValue");
            
        return (ulong)result;
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

**Swap 0.5 SOL to USDC:**

```csharp
// Input values
decimal inputSOL = 0.5m;
ulong inputBasisPoints = FRTExpectedTokens.DisplayToBasisPoints(inputSOL, 9);
// inputBasisPoints = 500,000,000

// Calculate expected output
ulong outputBasisPoints = FRTExpectedTokens.Calculate(
    inputBasisPoints,    // 500,000,000
    9,                   // SOL decimals
    6,                   // USDC decimals
    1000000000,          // SOL ratio (1 SOL)
    160000000,           // USDC ratio (160 USDC)
    true                 // A→B swap
);
// outputBasisPoints = 80,000,000

// Convert to display amount
decimal outputUSDC = FRTExpectedTokens.BasisPointsToDisplay(outputBasisPoints, 6);
// outputUSDC = 80.0

Console.WriteLine($"Swapping {inputSOL} SOL yields {outputUSDC} USDC");
```

### Example 2: USDC to SOL (160:1 ratio)

**Same pool, opposite direction:**

```csharp
// Input values
decimal inputUSDC = 80.0m;
ulong inputBasisPoints = FRTExpectedTokens.DisplayToBasisPoints(inputUSDC, 6);
// inputBasisPoints = 80,000,000

// Calculate expected output
ulong outputBasisPoints = FRTExpectedTokens.Calculate(
    inputBasisPoints,    // 80,000,000
    9,                   // SOL decimals
    6,                   // USDC decimals
    1000000000,          // SOL ratio (1 SOL)
    160000000,           // USDC ratio (160 USDC)
    false                // B→A swap
);
// outputBasisPoints = 500,000,000

// Convert to display amount
decimal outputSOL = FRTExpectedTokens.BasisPointsToDisplay(outputBasisPoints, 9);
// outputSOL = 0.5

Console.WriteLine($"Swapping {inputUSDC} USDC yields {outputSOL} SOL");
```

### Example 3: Tokens with Different Decimals

**Pool Configuration:**
- Token A: ABC (9 decimals)
- Token B: XYZ (2 decimals)
- Ratio: 1 ABC = 1 XYZ (1:1)
- Ratio_A: 1,000,000,000 (1 ABC)
- Ratio_B: 100 (1 XYZ)

```csharp
// Swap 0.5 ABC to XYZ
decimal inputABC = 0.5m;
ulong inputBasisPoints = FRTExpectedTokens.DisplayToBasisPoints(inputABC, 9);
// inputBasisPoints = 500,000,000

ulong outputBasisPoints = FRTExpectedTokens.Calculate(
    inputBasisPoints, 9, 2, 1000000000, 100, true);
// outputBasisPoints = 50

decimal outputXYZ = FRTExpectedTokens.BasisPointsToDisplay(outputBasisPoints, 2);
// outputXYZ = 0.50

Console.WriteLine($"Swapping {inputABC} ABC yields {outputXYZ} XYZ");
```

---

## Common Pitfalls and Solutions

### 1. Dust Amounts (Zero Output)

**Problem:** Small input amounts may result in zero output due to integer division.

**Example:**
```csharp
// Pool: 1 ABC (9 decimals) = 1000 XYZ (0 decimals)
ulong inputBasisPoints = 999999; // 0.000999999 ABC
ulong output = FRTExpectedTokens.Calculate(999999, 9, 0, 1000000000, 1000, true);
// output = 0 (dust eliminated)
```

**Solution:** Use `MinimumInputForOutput` to calculate the minimum viable trade:
```csharp
ulong minInput = FRTExpectedTokens.MinimumInputForOutput(9, 0, 1000000000, 1000, true);
Console.WriteLine($"Minimum input required: {minInput} basis points");

if (inputBasisPoints < minInput)
{
    throw new InvalidOperationException("Input amount too small, would result in zero output");
}
```

### 2. Overflow with Large Numbers

**Problem:** Multiplication of large numbers can cause overflow.

**Solution:** Use checked arithmetic or BigInteger:
```csharp
// Option 1: Checked arithmetic (throws on overflow)
try
{
    return checked((valueIn * tokenBRatio) / tokenARatio);
}
catch (OverflowException)
{
    // Fall back to BigInteger calculation
    return FRTExpectedTokensAdvanced.CalculateWithBigInteger(valueIn, tokenARatio, tokenBRatio, aToB);
}

// Option 2: Pre-check for potential overflow
if (valueIn > ulong.MaxValue / tokenBRatio)
{
    return FRTExpectedTokensAdvanced.CalculateWithBigInteger(valueIn, tokenARatio, tokenBRatio, aToB);
}
```

### 3. Wrong Direction Confusion

**Problem:** Mixing up which ratio to multiply vs divide.

**Remember:**
- A→B: Multiply by B ratio, divide by A ratio
- B→A: Multiply by A ratio, divide by B ratio

**Validation:**
```csharp
public static void ValidateCalculation(ulong input, ulong output, bool aToB, 
    ulong ratioA, ulong ratioB)
{
    // Reverse calculation should yield original input (within rounding)
    ulong reverseOutput = Calculate(output, 0, 0, ratioA, ratioB, !aToB);
    
    if (Math.Abs((long)input - (long)reverseOutput) > 1)
    {
        throw new InvalidOperationException("Calculation validation failed");
    }
}
```

---

## Testing Guidelines

### Unit Test Examples

```csharp
[TestClass]
public class FRTExpectedTokensTests
{
    [TestMethod]
    public void TestSimpleOneToOneRatio()
    {
        // Arrange
        ulong input = 1000000; // 1 token (6 decimals)
        ulong ratioA = 1000000; // 1 token A
        ulong ratioB = 1000000; // 1 token B
        
        // Act
        ulong output = FRTExpectedTokens.Calculate(input, 6, 6, ratioA, ratioB, true);
        
        // Assert
        Assert.AreEqual(1000000UL, output);
    }

    [TestMethod]
    public void TestDifferentDecimals()
    {
        // Arrange: 1 token A (9 decimals) = 1000 token B (6 decimals)
        ulong input = 1000000000; // 1 A
        ulong ratioA = 1000000000; // 1 A
        ulong ratioB = 1000000000; // 1000 B (in basis points)
        
        // Act
        ulong output = FRTExpectedTokens.Calculate(input, 9, 6, ratioA, ratioB, true);
        
        // Assert
        Assert.AreEqual(1000000000UL, output); // 1000 B
    }

    [TestMethod]
    public void TestFractionalAmounts()
    {
        // Arrange: 3:2 ratio
        ulong input = 1500000; // 1.5 A (6 decimals)
        ulong ratioA = 3000000; // 3 A
        ulong ratioB = 2000000; // 2 B
        
        // Act
        ulong output = FRTExpectedTokens.Calculate(input, 6, 6, ratioA, ratioB, true);
        
        // Assert
        Assert.AreEqual(1000000UL, output); // 1.0 B
    }

    [TestMethod]
    public void TestMinimumInputCalculation()
    {
        // Arrange: 1:1000 ratio
        ulong ratioA = 1000000000; // 1 A (9 decimals)
        ulong ratioB = 1000; // 1000 B (0 decimals)
        
        // Act
        ulong minInput = FRTExpectedTokens.MinimumInputForOutput(9, 0, ratioA, ratioB, true);
        
        // Assert
        Assert.IsTrue(minInput > 0);
        
        // Verify minimum input produces at least 1 basis point output
        ulong output = FRTExpectedTokens.Calculate(minInput, 9, 0, ratioA, ratioB, true);
        Assert.IsTrue(output >= 1);
    }
}
```

---

## Integration with Smart Contract

The on-chain Fixed Ratio Trading contract validates that the `expected_amount_out` parameter matches its calculation exactly. Any mismatch results in error code `0x417` (AMOUNT_MISMATCH).

### Best Practice Implementation

```csharp
public class FRTSwapService
{
    public async Task<TransactionResult> ExecuteSwapAsync(
        Wallet wallet,
        string poolId,
        SwapDirection direction,
        decimal inputAmount,
        PoolData poolData)
    {
        // Convert input to basis points
        int inputDecimals = direction == SwapDirection.AToB ? 
            poolData.TokenADecimals : poolData.TokenBDecimals;
        ulong inputBasisPoints = FRTExpectedTokens.DisplayToBasisPoints(inputAmount, inputDecimals);

        // Calculate expected output
        ulong expectedOutputBasisPoints = FRTExpectedTokens.Calculate(
            inputBasisPoints,
            poolData.TokenADecimals,
            poolData.TokenBDecimals,
            poolData.RatioANumerator,
            poolData.RatioBDenominator,
            direction == SwapDirection.AToB
        );

        // Validate minimum output
        if (expectedOutputBasisPoints == 0)
        {
            throw new InvalidOperationException("Input amount too small, would result in zero output");
        }

        // Submit swap transaction with exact expected amount
        return await SubmitSwapTransactionAsync(
            wallet, 
            poolId, 
            direction, 
            inputBasisPoints, 
            expectedOutputBasisPoints
        );
    }
}
```

---

## Debugging and Troubleshooting

### Debugging Checklist

1. **Log All Values:** When debugging, log ratios, decimals, and intermediate calculations
2. **Check Units:** Ensure all values are in basis points, not whole tokens
3. **Validate Ratios:** Confirm ratios match the pool's intended exchange rate
4. **Test Edge Cases:** Try minimum amounts, maximum amounts, and amounts that should produce dust

### Debug Helper Methods

```csharp
public static class FRTDebugHelpers
{
    public static void LogCalculationDetails(
        ulong valueIn,
        int tokenADecimals,
        int tokenBDecimals,
        ulong tokenARatio,
        ulong tokenBRatio,
        bool aToB)
    {
        Console.WriteLine("=== FRT Calculation Debug ===");
        Console.WriteLine($"Input: {valueIn} basis points");
        Console.WriteLine($"Token A Decimals: {tokenADecimals}");
        Console.WriteLine($"Token B Decimals: {tokenBDecimals}");
        Console.WriteLine($"Ratio A: {tokenARatio} basis points");
        Console.WriteLine($"Ratio B: {tokenBRatio} basis points");
        Console.WriteLine($"Direction: {(aToB ? "A→B" : "B→A")}");
        
        ulong output = FRTExpectedTokens.Calculate(
            valueIn, tokenADecimals, tokenBDecimals, 
            tokenARatio, tokenBRatio, aToB);
            
        Console.WriteLine($"Output: {output} basis points");
        
        // Show display amounts
        decimal inputDisplay = FRTExpectedTokens.BasisPointsToDisplay(
            valueIn, aToB ? tokenADecimals : tokenBDecimals);
        decimal outputDisplay = FRTExpectedTokens.BasisPointsToDisplay(
            output, aToB ? tokenBDecimals : tokenADecimals);
            
        Console.WriteLine($"Display: {inputDisplay} → {outputDisplay}");
        Console.WriteLine("=============================");
    }
}
```

---

## Summary

The FRTExpectedTokens calculation is deterministic and straightforward:
- Use the correct formula based on swap direction
- Work entirely in basis points
- Apply integer division (truncating remainders)
- Match the contract's calculation exactly for successful swaps

This approach ensures predictable, slippage-free trading with no surprises or failed transactions due to calculation mismatches.

### Key Takeaways for C#/.NET Developers

1. **Always use the `FRTExpectedTokens.Calculate` method** - don't implement calculations manually
2. **Handle overflow scenarios** with checked arithmetic or BigInteger
3. **Validate minimum inputs** to prevent zero-output transactions
4. **Test thoroughly** with various decimal combinations and edge cases
5. **Log calculations** during debugging for easier troubleshooting
