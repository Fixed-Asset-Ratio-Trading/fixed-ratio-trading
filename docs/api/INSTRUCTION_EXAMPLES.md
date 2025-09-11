# Fixed Ratio Trading - Instruction Examples

This guide provides JavaScript/TypeScript code examples for constructing instructions to interact with the Fixed Ratio Trading contract.

## Setup

```javascript
import {
    Connection,
    PublicKey,
    Transaction,
    SystemProgram,
    SYSVAR_RENT_PUBKEY,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import BN from 'bn.js';

const PROGRAM_ID = new PublicKey("4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn");
const connection = new Connection("https://api.mainnet-beta.solana.com");
```

## Instruction Enum

```javascript
// Define instruction types matching the contract (Borsh enum discriminators)
const PoolInstruction = {
    // System Management
    InitializeProgram: 0,        // InitializeProgram { admin_authority }
    InitializePool: 1,           // InitializePool { ratio_a_numerator, ratio_b_denominator, flags }
    Deposit: 2,                  // Deposit { deposit_token_mint, amount }
    Withdraw: 3,                 // Withdraw { withdraw_token_mint, lp_amount_to_burn }
    Swap: 4,                     // Swap { input_token_mint, amount_in, expected_amount_out }
    GetPoolStatePDA: 5,          // GetPoolStatePDA { multiple_token_mint, base_token_mint, multiple_per_base }
    GetTokenVaultPDAs: 6,        // GetTokenVaultPDAs { pool_state_pda }
    GetPoolInfo: 7,              // GetPoolInfo {}
    GetPoolPauseStatus: 8,       // GetPoolPauseStatus {}
    GetLiquidityInfo: 9,         // GetLiquidityInfo {}
    GetFeeInfo: 10,              // GetFeeInfo {}
    GetPoolSolBalance: 11,       // GetPoolSolBalance {}
    PauseSystem: 12,             // PauseSystem { reason_code }
    UnpauseSystem: 13,           // UnpauseSystem
    GetVersion: 14,              // GetVersion
    WithdrawTreasuryFees: 15,    // WithdrawTreasuryFees { amount }
    GetTreasuryInfo: 16,         // GetTreasuryInfo {}
    ConsolidatePoolFees: 17,     // ConsolidatePoolFees { pool_count }
    GetConsolidationStatus: 18,  // GetConsolidationStatus { pool_count }
    PausePool: 19,               // PausePool { reason_code }
    UnpausePool: 20,             // UnpausePool { reason_code }
    SetSwapOwnerOnly: 21,        // SetSwapOwnerOnly { enable_restriction, designated_owner }
    UpdatePoolFees: 22,          // UpdatePoolFees { update_flags, new_liquidity_fee, new_swap_fee }
    DonateSol: 23,               // DonateSol { amount, message }
    ProcessAdminChange: 24,      // ProcessAdminChange { new_admin_authority }
};
```

## System Management Examples

### Initialize Program
```javascript
async function createInitializeProgramInstruction(
    programAuthority: PublicKey,
    adminAuthority: PublicKey, // Admin authority for system operations (pause/unpause, treasury withdrawals)
    programDataAccount: PublicKey
) {
    // Derive PDAs
    const [systemStatePDA] = PublicKey.findProgramAddress(
        [Buffer.from("system_state")],
        PROGRAM_ID
    );
    
    const [mainTreasuryPDA] = PublicKey.findProgramAddress(
        [Buffer.from("main_treasury")],
        PROGRAM_ID
    );
    
    // Serialize instruction data with admin authority
    const instructionData = Buffer.concat([
        Buffer.from([PoolInstruction.InitializeProgram]),
        adminAuthority.toBuffer() // Admin authority pubkey (32 bytes)
    ]);
    
    return new TransactionInstruction({
        keys: [
            { pubkey: programAuthority, isSigner: true, isWritable: true },
            { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
            { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
            { pubkey: systemStatePDA, isSigner: false, isWritable: true },
            { pubkey: mainTreasuryPDA, isSigner: false, isWritable: true },
            { pubkey: programDataAccount, isSigner: false, isWritable: false },
        ],
        programId: PROGRAM_ID,
        data: instructionData,
    });
}
```

### Pause System
```javascript
function createPauseSystemInstruction(
    systemAuthority: PublicKey,
    reasonCode: number,
    programDataAccount: PublicKey
) {
    const [systemStatePDA] = PublicKey.findProgramAddress(
        [Buffer.from("system_state")],
        PROGRAM_ID
    );
    
    const instructionData = Buffer.concat([
        Buffer.from([PoolInstruction.PauseSystem]),
        Buffer.from([reasonCode])
    ]);
    
    return new TransactionInstruction({
        keys: [
            { pubkey: systemAuthority, isSigner: true, isWritable: true },
            { pubkey: systemStatePDA, isSigner: false, isWritable: true },
            { pubkey: programDataAccount, isSigner: false, isWritable: false },
        ],
        programId: PROGRAM_ID,
        data: instructionData,
    });
}
```

## Pool Management Examples

### Initialize Pool
```javascript
async function createInitializePoolInstruction(
    userAuthority: PublicKey,
    tokenAMint: PublicKey,
    tokenBMint: PublicKey,
    ratioA: BN,
    ratioB: BN,
    flags: number = 0
) {
    // Normalize token order
    const [mintA, mintB] = tokenAMint.toBuffer() < tokenBMint.toBuffer() 
        ? [tokenAMint, tokenBMint]
        : [tokenBMint, tokenAMint];
    
    // Derive all PDAs
    const [systemStatePDA] = PublicKey.findProgramAddress(
        [Buffer.from("system_state")],
        PROGRAM_ID
    );
    
    const [poolStatePDA] = PublicKey.findProgramAddress(
        [
            Buffer.from("pool_state"),
            mintA.toBuffer(),
            mintB.toBuffer(),
            ratioA.toArrayLike(Buffer, 'le', 8),
            ratioB.toArrayLike(Buffer, 'le', 8)
        ],
        PROGRAM_ID
    );
    
    const [tokenAVaultPDA] = PublicKey.findProgramAddress(
        [Buffer.from("token_a_vault"), poolStatePDA.toBuffer()],
        PROGRAM_ID
    );
    
    const [tokenBVaultPDA] = PublicKey.findProgramAddress(
        [Buffer.from("token_b_vault"), poolStatePDA.toBuffer()],
        PROGRAM_ID
    );
    
    const [lpTokenAMintPDA] = PublicKey.findProgramAddress(
        [Buffer.from("lp_token_a_mint"), poolStatePDA.toBuffer()],
        PROGRAM_ID
    );
    
    const [lpTokenBMintPDA] = PublicKey.findProgramAddress(
        [Buffer.from("lp_token_b_mint"), poolStatePDA.toBuffer()],
        PROGRAM_ID
    );
    
    const [mainTreasuryPDA] = PublicKey.findProgramAddress(
        [Buffer.from("main_treasury")],
        PROGRAM_ID
    );
    
    // Serialize instruction data
    const instructionData = Buffer.concat([
        Buffer.from([PoolInstruction.InitializePool]),
        ratioA.toArrayLike(Buffer, 'le', 8),
        ratioB.toArrayLike(Buffer, 'le', 8),
        Buffer.from([flags])
    ]);
    
    return new TransactionInstruction({
        keys: [
            { pubkey: userAuthority, isSigner: true, isWritable: true },
            { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
            { pubkey: systemStatePDA, isSigner: false, isWritable: false },
            { pubkey: poolStatePDA, isSigner: false, isWritable: true },
            { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
            { pubkey: mainTreasuryPDA, isSigner: false, isWritable: true },
            { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
            { pubkey: tokenAMint, isSigner: false, isWritable: false },
            { pubkey: tokenBMint, isSigner: false, isWritable: false },
            { pubkey: tokenAVaultPDA, isSigner: false, isWritable: true },
            { pubkey: tokenBVaultPDA, isSigner: false, isWritable: true },
            { pubkey: lpTokenAMintPDA, isSigner: false, isWritable: true },
            { pubkey: lpTokenBMintPDA, isSigner: false, isWritable: true },
        ],
        programId: PROGRAM_ID,
        data: instructionData,
    });
}
```

### Update Pool Fees
```javascript
function createUpdatePoolFeesInstruction(
    programAuthority: PublicKey,
    poolStatePDA: PublicKey,
    programDataAccount: PublicKey,
    updateFlags: number,
    newLiquidityFee: BN,
    newSwapFee: BN
) {
    const [systemStatePDA] = PublicKey.findProgramAddress(
        [Buffer.from("system_state")],
        PROGRAM_ID
    );
    
    const instructionData = Buffer.concat([
        Buffer.from([PoolInstruction.UpdatePoolFees]),
        Buffer.from([updateFlags]),
        newLiquidityFee.toArrayLike(Buffer, 'le', 8),
        newSwapFee.toArrayLike(Buffer, 'le', 8)
    ]);
    
    return new TransactionInstruction({
        keys: [
            { pubkey: programAuthority, isSigner: true, isWritable: true },
            { pubkey: systemStatePDA, isSigner: false, isWritable: false },
            { pubkey: poolStatePDA, isSigner: false, isWritable: true },
            { pubkey: programDataAccount, isSigner: false, isWritable: false },
        ],
        programId: PROGRAM_ID,
        data: instructionData,
    });
}
```

### Pause Pool Operations
```javascript
// Import Borsh for proper serialization
import { serialize } from 'borsh';

// Pause flag constants
const PAUSE_FLAG_LIQUIDITY = 1;  // 0b01 - Pause deposits/withdrawals
const PAUSE_FLAG_SWAPS = 2;      // 0b10 - Pause swaps  
const PAUSE_FLAG_ALL = 3;        // 0b11 - Pause both operations

function createPausePoolInstruction(
    adminAuthority: PublicKey,
    poolStatePDA: PublicKey,
    programDataAccount: PublicKey,
    pauseFlags: number
) {
    const [systemStatePDA] = PublicKey.findProgramAddress(
        [Buffer.from("system_state")],
        PROGRAM_ID
    );
    
    // Create PoolInstruction::PausePool using Borsh serialization
    const pausePoolInstruction = {
        pausePool: {
            pause_flags: pauseFlags
        }
    };
    
    // Serialize using Borsh (matches working test implementation)
    const instructionData = serialize(PoolInstructionSchema, pausePoolInstruction);
    
    return new TransactionInstruction({
        keys: [
            { pubkey: adminAuthority, isSigner: true, isWritable: true },
            { pubkey: systemStatePDA, isSigner: false, isWritable: true },     // ⚠️ WRITABLE
            { pubkey: poolStatePDA, isSigner: false, isWritable: true },       // ⚠️ WRITABLE
            { pubkey: programDataAccount, isSigner: false, isWritable: false },
        ],
        programId: PROGRAM_ID,
        data: instructionData,
    });
}

// Usage examples:
// Pause only liquidity operations (deposits/withdrawals)
const pauseLiquidityInstruction = createPausePoolInstruction(
    adminAuthority,
    poolStatePDA,
    programDataAccount,
    PAUSE_FLAG_LIQUIDITY
);

// Pause only swap operations
const pauseSwapsInstruction = createPausePoolInstruction(
    adminAuthority,
    poolStatePDA,
    programDataAccount,
    PAUSE_FLAG_SWAPS
);

// Pause all operations (required for consolidation eligibility)
const pauseAllInstruction = createPausePoolInstruction(
    adminAuthority,
    poolStatePDA,
    programDataAccount,
    PAUSE_FLAG_ALL
);
```

### Unpause Pool Operations
```javascript
function createUnpausePoolInstruction(
    adminAuthority: PublicKey,
    poolStatePDA: PublicKey,
    programDataAccount: PublicKey,
    unpauseFlags: number
) {
    const [systemStatePDA] = PublicKey.findProgramAddress(
        [Buffer.from("system_state")],
        PROGRAM_ID
    );
    
    // Create PoolInstruction::UnpausePool using Borsh serialization
    const unpausePoolInstruction = {
        unpausePool: {
            unpause_flags: unpauseFlags
        }
    };
    
    // Serialize using Borsh (matches working test implementation)
    const instructionData = serialize(PoolInstructionSchema, unpausePoolInstruction);
    
    return new TransactionInstruction({
        keys: [
            { pubkey: adminAuthority, isSigner: true, isWritable: true },
            { pubkey: systemStatePDA, isSigner: false, isWritable: true },     // ⚠️ WRITABLE
            { pubkey: poolStatePDA, isSigner: false, isWritable: true },       // ⚠️ WRITABLE
            { pubkey: programDataAccount, isSigner: false, isWritable: false },
        ],
        programId: PROGRAM_ID,
        data: instructionData,
    });
}

// Usage examples:
// Unpause only liquidity operations
const unpauseLiquidityInstruction = createUnpausePoolInstruction(
    adminAuthority,
    poolStatePDA,
    programDataAccount,
    PAUSE_FLAG_LIQUIDITY
);

// Unpause only swap operations
const unpauseSwapsInstruction = createUnpausePoolInstruction(
    adminAuthority,
    poolStatePDA,
    programDataAccount,
    PAUSE_FLAG_SWAPS
);

// Unpause all operations
const unpauseAllInstruction = createUnpausePoolInstruction(
    adminAuthority,
    poolStatePDA,
    programDataAccount,
    PAUSE_FLAG_ALL
);
```

### Set Swap Owner Only
```javascript
function createSetSwapOwnerOnlyInstruction(
    adminAuthority: PublicKey,
    poolStatePDA: PublicKey,
    programDataAccount: PublicKey,
    enableRestriction: boolean,
    designatedOwner: PublicKey
) {
    const [systemStatePDA] = PublicKey.findProgramAddress(
        [Buffer.from("system_state")],
        PROGRAM_ID
    );
    
    // Serialize instruction data
    const instructionData = Buffer.concat([
        Buffer.from([21]), // SetSwapOwnerOnly discriminator
        Buffer.from([enableRestriction ? 1 : 0]), // boolean as u8
        designatedOwner.toBuffer() // Pubkey (32 bytes)
    ]);
    
    return new TransactionInstruction({
        keys: [
            { pubkey: adminAuthority, isSigner: true, isWritable: false }, // ⚠️ READ-ONLY
            { pubkey: systemStatePDA, isSigner: false, isWritable: false },
            { pubkey: poolStatePDA, isSigner: false, isWritable: true },
            { pubkey: programDataAccount, isSigner: false, isWritable: false },
        ],
        programId: PROGRAM_ID,
        data: instructionData,
    });
}

// Usage examples:
// Enable owner-only restrictions with custom designated owner
const enableOwnerOnlyInstruction = createSetSwapOwnerOnlyInstruction(
    adminAuthority,
    poolStatePDA,
    programDataAccount,
    true, // enable restriction
    customContractPubkey // designated owner
);

// Disable owner-only restrictions (designated owner parameter ignored when disabling)
const disableOwnerOnlyInstruction = createSetSwapOwnerOnlyInstruction(
    adminAuthority,
    poolStatePDA,
    programDataAccount,
    false, // disable restriction
    PublicKey.default // ignored when disabling
);
```

## Liquidity Operations Examples

### Deposit Liquidity
```javascript
function createDepositInstruction(
    userAuthority: PublicKey,
    poolStatePDA: PublicKey,
    depositAmount: BN,
    depositTokenMint: PublicKey,
    userTokenAccount: PublicKey,
    poolTokenVault: PublicKey,
    otherTokenVault: PublicKey,
    lpTokenMint: PublicKey,
    userLpAccount: PublicKey
) {
    const [systemStatePDA] = PublicKey.findProgramAddress(
        [Buffer.from("system_state")],
        PROGRAM_ID
    );
    
    const [mainTreasuryPDA] = PublicKey.findProgramAddress(
        [Buffer.from("main_treasury")],
        PROGRAM_ID
    );
    
    const instructionData = Buffer.concat([
        Buffer.from([PoolInstruction.Deposit]),
        depositTokenMint.toBuffer(),
        depositAmount.toArrayLike(Buffer, 'le', 8)
    ]);
    
    return new TransactionInstruction({
        keys: [
            { pubkey: userAuthority, isSigner: true, isWritable: true },
            { pubkey: systemStatePDA, isSigner: false, isWritable: false },
            { pubkey: poolStatePDA, isSigner: false, isWritable: true },
            { pubkey: userTokenAccount, isSigner: false, isWritable: true },
            { pubkey: poolTokenVault, isSigner: false, isWritable: true },
            { pubkey: otherTokenVault, isSigner: false, isWritable: true },
            { pubkey: lpTokenMint, isSigner: false, isWritable: true },
            { pubkey: userLpAccount, isSigner: false, isWritable: true },
            { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
            { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
            { pubkey: mainTreasuryPDA, isSigner: false, isWritable: true },
            { pubkey: depositTokenMint, isSigner: false, isWritable: false },
        ],
        programId: PROGRAM_ID,
        data: instructionData,
    });
}
```

## Swap Operations Examples

### Execute Swap
```javascript
function createSwapInstruction(
    userAuthority: PublicKey,
    poolStatePDA: PublicKey,
    amountIn: BN,
    expectedAmountOut: BN,
    inputTokenMint: PublicKey,
    userInputAccount: PublicKey,
    userOutputAccount: PublicKey,
    poolInputVault: PublicKey,
    poolOutputVault: PublicKey
) {
    const [systemStatePDA] = PublicKey.findProgramAddress(
        [Buffer.from("system_state")],
        PROGRAM_ID
    );
    
    const [mainTreasuryPDA] = PublicKey.findProgramAddress(
        [Buffer.from("main_treasury")],
        PROGRAM_ID
    );
    
    const instructionData = Buffer.concat([
        Buffer.from([PoolInstruction.Swap]),
        inputTokenMint.toBuffer(),
        amountIn.toArrayLike(Buffer, 'le', 8),
        expectedAmountOut.toArrayLike(Buffer, 'le', 8)
    ]);
    
    return new TransactionInstruction({
        keys: [
            { pubkey: userAuthority, isSigner: true, isWritable: true },
            { pubkey: systemStatePDA, isSigner: false, isWritable: false },
            { pubkey: poolStatePDA, isSigner: false, isWritable: true },
            { pubkey: userInputAccount, isSigner: false, isWritable: true },
            { pubkey: userOutputAccount, isSigner: false, isWritable: true },
            { pubkey: poolInputVault, isSigner: false, isWritable: true },
            { pubkey: poolOutputVault, isSigner: false, isWritable: true },
            { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
            { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
            { pubkey: mainTreasuryPDA, isSigner: false, isWritable: true },
            { pubkey: inputTokenMint, isSigner: false, isWritable: false },
        ],
        programId: PROGRAM_ID,
        data: instructionData,
    });
}
```

## Treasury Operations Examples

### Donate SOL
```javascript
function createDonateSolInstruction(
    donor: PublicKey,
    amount: BN,
    message: string
) {
    const [systemStatePDA] = PublicKey.findProgramAddress(
        [Buffer.from("system_state")],
        PROGRAM_ID
    );
    
    const [mainTreasuryPDA] = PublicKey.findProgramAddress(
        [Buffer.from("main_treasury")],
        PROGRAM_ID
    );
    
    // Encode message (max 200 chars)
    const messageBuffer = Buffer.from(message.slice(0, 200), 'utf8');
    const messageLengthBuffer = Buffer.alloc(4);
    messageLengthBuffer.writeUInt32LE(messageBuffer.length, 0);
    
    const instructionData = Buffer.concat([
        Buffer.from([PoolInstruction.DonateSol]),
        amount.toArrayLike(Buffer, 'le', 8),
        messageLengthBuffer,
        messageBuffer
    ]);
    
    return new TransactionInstruction({
        keys: [
            { pubkey: donor, isSigner: true, isWritable: true },
            { pubkey: mainTreasuryPDA, isSigner: false, isWritable: true },
            { pubkey: systemStatePDA, isSigner: false, isWritable: false },
            { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        ],
        programId: PROGRAM_ID,
        data: instructionData,
    });
}
```

## Helper Functions

### Calculate Pool PDA
```javascript
function derivePoolPDA(
    tokenAMint: PublicKey,
    tokenBMint: PublicKey,
    ratioA: BN,
    ratioB: BN
): [PublicKey, number] {
    // Normalize token order
    const [mintA, mintB] = tokenAMint.toBuffer() < tokenBMint.toBuffer() 
        ? [tokenAMint, tokenBMint]
        : [tokenBMint, tokenAMint];
    
    return PublicKey.findProgramAddressSync(
        [
            Buffer.from("pool_state"),
            mintA.toBuffer(),
            mintB.toBuffer(),
            ratioA.toArrayLike(Buffer, 'le', 8),
            ratioB.toArrayLike(Buffer, 'le', 8)
        ],
        PROGRAM_ID
    );
}
```

### Convert Display Amount to Basis Points
```javascript
function toBasisPoints(amount: number, decimals: number): BN {
    return new BN(amount * Math.pow(10, decimals));
}

// Examples
const oneSol = toBasisPoints(1.0, 9);      // 1,000,000,000
const oneUsdc = toBasisPoints(1.0, 6);     // 1,000,000
```

## Transaction Building Example

```javascript
async function executePoolCreation() {
    const wallet = /* your wallet */;
    const tokenAMint = new PublicKey("...");
    const tokenBMint = new PublicKey("...");
    
    // Get token decimals
    const tokenAInfo = await connection.getParsedAccountInfo(tokenAMint);
    const tokenBInfo = await connection.getParsedAccountInfo(tokenBMint);
    const decimalsA = tokenAInfo.value?.data.parsed.info.decimals || 9;
    const decimalsB = tokenBInfo.value?.data.parsed.info.decimals || 6;
    
    // Set pool ratio (e.g., 1 SOL = 160 USDC)
    const ratioA = toBasisPoints(1.0, decimalsA);
    const ratioB = toBasisPoints(160.0, decimalsB);
    
    // Create instruction (with optional flags)
    const flags = 0; // Standard pool (default)
    // const flags = 32; // Owner-only swaps
    // const flags = 64; // Exact exchange required
    // const flags = 96; // Both owner-only and exact exchange
    
    const instruction = await createInitializePoolInstruction(
        wallet.publicKey,
        tokenAMint,
        tokenBMint,
        ratioA,
        ratioB,
        flags
    );
    
    // Build and send transaction
    const transaction = new Transaction().add(instruction);
    const signature = await sendAndConfirmTransaction(
        connection,
        transaction,
        [wallet]
    );
    
    console.log("Pool created:", signature);
}
```

## Error Handling

```javascript
try {
    const signature = await sendAndConfirmTransaction(connection, transaction, signers);
} catch (error) {
    if (error.logs) {
        // Parse custom error codes
        const errorCode = parseErrorCode(error.logs);
        switch (errorCode) {
            case 6006:
                console.error("System is paused");
                break;
            case 6008:
                console.error("Slippage exceeded");
                break;
            default:
                console.error("Transaction failed:", error);
        }
    }
}
```

## Notes

1. Always verify PDAs match expected addresses
2. Handle all amounts in basis points
3. Check pause states before operations
4. Include proper slippage tolerance for swaps
5. Monitor transaction logs for detailed error messages