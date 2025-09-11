# Exact Pool Creation Transaction Structure

**File:** `docs/api/EXACT_POOL_CREATION_TRANSACTION_STRUCTURE.md`  
**Purpose:** Provide exact transaction structure for pool creation debugging  
**Source:** Working dashboard implementation (`dashboard/pool-creation.js`)  
**Last Updated:** 2025-01-27  

## 🎯 Overview

This document provides the **exact transaction structure** used by the working dashboard for pool creation. Use this to compare against your stress test service and resolve "Program failed to complete" errors.

## 📋 Complete Transaction Structure

### **Transaction Instructions**

```javascript
// Transaction has exactly 2 instructions:
transaction.instructions = [
    computeBudgetInstruction,    // Instruction 0: Compute Budget
    createPoolInstruction       // Instruction 1: Pool Creation
];
```

### **Instruction 0: Compute Budget**

```javascript
const computeBudgetInstruction = solanaWeb3.ComputeBudgetProgram.setComputeUnitLimit({
    units: 195_000  // Dashboard uses 195,000 CUs
});

// Structure:
{
    programId: "ComputeBudget111111111111111111111111111111",
    keys: [],  // No accounts required
    data: [...] // Program-generated data
}
```

### **Instruction 1: Pool Creation (InitializePool)**

#### **Program ID**
```javascript
const programId = new PublicKey("4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn");
```

#### **Instruction Data Structure**
```javascript
// Total: 18 bytes (1 + 8 + 8 + 1)
const flags = 0; // Standard pool (default)
// const flags = 32; // Owner-only swaps
// const flags = 64; // Exact exchange required
// const flags = 96; // Both owner-only and exact exchange

const instructionData = concatUint8Arrays([
    new Uint8Array([1]),                                           // 1 byte: Discriminator (InitializePool)
    new Uint8Array(new BigUint64Array([BigInt(ratioABasisPoints)]).buffer),  // 8 bytes: ratio_a_numerator (little-endian u64)
    new Uint8Array(new BigUint64Array([BigInt(ratioBBasisPoints)]).buffer),  // 8 bytes: ratio_b_denominator (little-endian u64)
    new Uint8Array([flags])                                        // 1 byte: Pool behavior flags
]);
```

#### **Pool Flags (New in v0.16.x+)**
| Flag | Bit | Value | Description |
|------|-----|-------|-------------|
| Owner-only swaps | 5 | 32 | Only pool creator can swap |
| Exact exchange required | 6 | 64 | Reject swaps with precision loss |

**Important**: Only bits 5 and 6 can be set during pool creation. Use `0` for standard pools that allow all users and permit dust loss.

#### **Account Structure (13 accounts total)**

```javascript
const accountKeys = [
    // 0: User Authority (Signer)
    { 
        pubkey: wallet.publicKey, 
        isSigner: true, 
        isWritable: true 
    },
    
    // 1: System Program
    { 
        pubkey: solanaWeb3.SystemProgram.programId, 
        isSigner: false, 
        isWritable: false 
    },
    
    // 2: System State PDA (for pause validation)
    { 
        pubkey: systemStatePDA[0], 
        isSigner: false, 
        isWritable: false 
    },
    
    // 3: Pool State PDA (to be created)
    { 
        pubkey: poolStatePDA[0], 
        isSigner: false, 
        isWritable: true 
    },
    
    // 4: SPL Token Program
    { 
        pubkey: new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'), 
        isSigner: false, 
        isWritable: false 
    },
    
    // 5: Main Treasury PDA
    { 
        pubkey: mainTreasuryPDA[0], 
        isSigner: false, 
        isWritable: true 
    },
    
    // 6: Rent Sysvar
    { 
        pubkey: solanaWeb3.SYSVAR_RENT_PUBKEY, 
        isSigner: false, 
        isWritable: false 
    },
    
    // 7: Token A Mint (lexicographically smaller)
    { 
        pubkey: tokenAMint, 
        isSigner: false, 
        isWritable: false 
    },
    
    // 8: Token B Mint (lexicographically larger)
    { 
        pubkey: tokenBMint, 
        isSigner: false, 
        isWritable: false 
    },
    
    // 9: Token A Vault PDA (to be created)
    { 
        pubkey: tokenAVaultPDA[0], 
        isSigner: false, 
        isWritable: true 
    },
    
    // 10: Token B Vault PDA (to be created)
    { 
        pubkey: tokenBVaultPDA[0], 
        isSigner: false, 
        isWritable: true 
    },
    
    // 11: LP Token A Mint PDA (to be created)
    { 
        pubkey: lpTokenAMintPDA[0], 
        isSigner: false, 
        isWritable: true 
    },
    
    // 12: LP Token B Mint PDA (to be created)
    { 
        pubkey: lpTokenBMintPDA[0], 
        isSigner: false, 
        isWritable: true 
    }
];
```

## 🔍 PDA Derivation Details

### **Critical: Exact PDA Derivation Order**

```javascript
// 1. System State PDA
const systemStatePDA = await PublicKey.findProgramAddress(
    [new TextEncoder().encode('system_state')],
    programId
);

// 2. Main Treasury PDA  
const mainTreasuryPDA = await PublicKey.findProgramAddress(
    [new TextEncoder().encode('main_treasury')],
    programId
);

// 3. Pool State PDA (CRITICAL: Uses normalized tokens and basis points)
const poolStatePDA = await PublicKey.findProgramAddress([
    new TextEncoder().encode('pool_state'),
    tokenAMint.toBuffer(),  // Normalized Token A (lexicographically smaller)
    tokenBMint.toBuffer(),  // Normalized Token B (lexicographically larger)
    new Uint8Array(new BigUint64Array([BigInt(finalRatioABasisPoints)]).buffer),  // ratio_a_numerator
    new Uint8Array(new BigUint64Array([BigInt(finalRatioBBasisPoints)]).buffer)   // ratio_b_denominator
], programId);

// 4. Token A Vault PDA
const tokenAVaultPDA = await PublicKey.findProgramAddress(
    [new TextEncoder().encode('token_a_vault'), poolStatePDA[0].toBuffer()],
    programId
);

// 5. Token B Vault PDA
const tokenBVaultPDA = await PublicKey.findProgramAddress(
    [new TextEncoder().encode('token_b_vault'), poolStatePDA[0].toBuffer()],
    programId
);

// 6. LP Token A Mint PDA
const lpTokenAMintPDA = await PublicKey.findProgramAddress(
    [new TextEncoder().encode('lp_token_a_mint'), poolStatePDA[0].toBuffer()],
    programId
);

// 7. LP Token B Mint PDA
const lpTokenBMintPDA = await PublicKey.findProgramAddress(
    [new TextEncoder().encode('lp_token_b_mint'), poolStatePDA[0].toBuffer()],
    programId
);
```

## 💡 Critical Success Factors

### **1. Token Normalization**

```javascript
// CRITICAL: Tokens must be normalized lexicographically
function normalizeTokenOrder(mintA, mintB) {
    const bytesA = mintA.toBytes();
    const bytesB = mintB.toBytes();
    
    let aLessThanB = false;
    for (let i = 0; i < 32; i++) {
        if (bytesA[i] < bytesB[i]) { aLessThanB = true; break; }
        if (bytesA[i] > bytesB[i]) { aLessThanB = false; break; }
    }
    
    return aLessThanB ? { tokenA: mintA, tokenB: mintB } : { tokenA: mintB, tokenB: mintA };
}
```

### **2. Basis Points Conversion**

```javascript
// CRITICAL: Each token uses its own decimal precision
function displayToBasisPoints(displayAmount, decimals) {
    return Math.round(displayAmount * Math.pow(10, decimals));
}

// Example: 1 SOL = 160 USDC
const solDecimals = 9;
const usdcDecimals = 6;

const ratioANumerator = displayToBasisPoints(1.0, solDecimals);     // 1,000,000,000
const ratioBDenominator = displayToBasisPoints(160.0, usdcDecimals); // 160,000,000
```

### **3. Instruction Data Verification**

```javascript
// Verify instruction data matches PDA seeds
const ratioABytes = new Uint8Array(new BigUint64Array([BigInt(finalRatioABasisPoints)]).buffer);
const ratioBBytes = new Uint8Array(new BigUint64Array([BigInt(finalRatioBBasisPoints)]).buffer);

console.log('PDA Seeds:', {
    ratioA: Array.from(ratioABytes),
    ratioB: Array.from(ratioBBytes)
});

console.log('Instruction Data:', {
    ratioA: Array.from(instructionData.slice(1, 9)),
    ratioB: Array.from(instructionData.slice(9, 17))
});

// These MUST match exactly
```

## 🔧 Transaction Building Code

### **Complete Working Implementation**

```javascript
async function createExactPoolTransaction(tokenA, tokenB, exchangeRatio, wallet, connection) {
    const programId = new PublicKey("4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn");
    
    // 1. Get token decimals
    const tokenADecimals = await getTokenDecimals(tokenA.mint, connection);
    const tokenBDecimals = await getTokenDecimals(tokenB.mint, connection);
    
    // 2. Create PublicKeys
    const tokenAMint = new PublicKey(tokenA.mint);
    const tokenBMint = new PublicKey(tokenB.mint);
    
    // 3. Normalize tokens lexicographically
    const { tokenA: normalizedTokenA, tokenB: normalizedTokenB, swapped } = normalizeTokenOrder(tokenAMint, tokenBMint);
    
    // 4. Calculate basis points (adjust for swapping)
    let tokenADisplay, tokenBDisplay;
    if (swapped) {
        tokenADisplay = exchangeRatio;  // User's TokenB becomes normalized TokenA
        tokenBDisplay = 1.0;            // User's TokenA becomes normalized TokenB
    } else {
        tokenADisplay = 1.0;            // User's TokenA stays as normalized TokenA
        tokenBDisplay = exchangeRatio;  // User's TokenB stays as normalized TokenB
    }
    
    const normalizedTokenADecimals = await getTokenDecimals(normalizedTokenA.toString(), connection);
    const normalizedTokenBDecimals = await getTokenDecimals(normalizedTokenB.toString(), connection);
    
    const finalRatioABasisPoints = displayToBasisPoints(tokenADisplay, normalizedTokenADecimals);
    const finalRatioBBasisPoints = displayToBasisPoints(tokenBDisplay, normalizedTokenBDecimals);
    
    // 5. Derive all PDAs (exact order from working dashboard)
    const systemStatePDA = await PublicKey.findProgramAddress(
        [new TextEncoder().encode('system_state')],
        programId
    );
    
    const mainTreasuryPDA = await PublicKey.findProgramAddress(
        [new TextEncoder().encode('main_treasury')],
        programId
    );
    
    const poolStatePDA = await PublicKey.findProgramAddress([
        new TextEncoder().encode('pool_state'),
        normalizedTokenA.toBuffer(),
        normalizedTokenB.toBuffer(),
        new Uint8Array(new BigUint64Array([BigInt(finalRatioABasisPoints)]).buffer),
        new Uint8Array(new BigUint64Array([BigInt(finalRatioBBasisPoints)]).buffer)
    ], programId);
    
    const tokenAVaultPDA = await PublicKey.findProgramAddress(
        [new TextEncoder().encode('token_a_vault'), poolStatePDA[0].toBuffer()],
        programId
    );
    
    const tokenBVaultPDA = await PublicKey.findProgramAddress(
        [new TextEncoder().encode('token_b_vault'), poolStatePDA[0].toBuffer()],
        programId
    );
    
    const lpTokenAMintPDA = await PublicKey.findProgramAddress(
        [new TextEncoder().encode('lp_token_a_mint'), poolStatePDA[0].toBuffer()],
        programId
    );
    
    const lpTokenBMintPDA = await PublicKey.findProgramAddress(
        [new TextEncoder().encode('lp_token_b_mint'), poolStatePDA[0].toBuffer()],
        programId
    );
    
    // 6. Create instruction data (exact format)
    const discriminator = new Uint8Array([1]);
    const ratioABytes = new Uint8Array(new BigUint64Array([BigInt(finalRatioABasisPoints)]).buffer);
    const ratioBBytes = new Uint8Array(new BigUint64Array([BigInt(finalRatioBBasisPoints)]).buffer);
    
    const instructionData = concatUint8Arrays([discriminator, ratioABytes, ratioBBytes]);
    
    // 7. Create account keys (exact order)
    const accountKeys = [
        { pubkey: wallet.publicKey, isSigner: true, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        { pubkey: systemStatePDA[0], isSigner: false, isWritable: false },
        { pubkey: poolStatePDA[0], isSigner: false, isWritable: true },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: mainTreasuryPDA[0], isSigner: false, isWritable: true },
        { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
        { pubkey: normalizedTokenA, isSigner: false, isWritable: false },
        { pubkey: normalizedTokenB, isSigner: false, isWritable: false },
        { pubkey: tokenAVaultPDA[0], isSigner: false, isWritable: true },
        { pubkey: tokenBVaultPDA[0], isSigner: false, isWritable: true },
        { pubkey: lpTokenAMintPDA[0], isSigner: false, isWritable: true },
        { pubkey: lpTokenBMintPDA[0], isSigner: false, isWritable: true }
    ];
    
    // 8. Create instructions
    const computeBudgetInstruction = ComputeBudgetProgram.setComputeUnitLimit({
        units: 195_000
    });
    
    const createPoolInstruction = new TransactionInstruction({
        keys: accountKeys,
        programId: programId,
        data: instructionData
    });
    
    // 9. Build transaction
    const transaction = new Transaction()
        .add(computeBudgetInstruction)
        .add(createPoolInstruction);
    
    return transaction;
}
```

## 🔍 Debugging Checklist

Compare your stress test service against this exact structure:

### **1. Account Count**
- [ ] Exactly 13 accounts in instruction 1
- [ ] Accounts in exact order shown above

### **2. Account Properties**
- [ ] Correct signer flags (only account 0 is signer)
- [ ] Correct writable flags (accounts 0, 3, 5, 9, 10, 11, 12 are writable)

### **3. Instruction Data**
- [ ] Exactly 18 bytes total
- [ ] First byte is `1` (discriminator)
- [ ] Next 8 bytes are ratio_a_numerator (little-endian u64)
- [ ] Next 8 bytes are ratio_b_denominator (little-endian u64)
- [ ] Last byte is flags (u8) - use `0` for standard pools

### **4. PDA Derivation**
- [ ] Pool State PDA uses normalized tokens and basis points
- [ ] All other PDAs derive from Pool State PDA
- [ ] Byte arrays match between PDA seeds and instruction data

### **5. Token Normalization**
- [ ] Tokens ordered lexicographically (byte-wise comparison)
- [ ] Ratios adjusted if tokens were swapped during normalization

### **6. Basis Points Calculation**
- [ ] Each token uses its own decimal precision
- [ ] Display amounts converted to basis points correctly

This exact structure is proven to work in the dashboard. Any deviation likely causes the "Program failed to complete" error.

---

**Note:** This structure is extracted from the working dashboard implementation and represents the exact transaction format that successfully creates pools.
