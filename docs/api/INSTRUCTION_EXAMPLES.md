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
// Define instruction types matching the contract
const PoolInstruction = {
    // System Management
    InitializeProgram: 0,
    PauseSystem: 1,
    UnpauseSystem: 2,
    
    // Pool Management
    InitializePool: 3,
    PausePool: 4,
    UnpausePool: 5,
    UpdatePoolFees: 6,
    
    // Liquidity Operations
    Deposit: 7,
    Withdraw: 8,
    
    // Swap Operations
    Swap: 9,
    SetSwapOwnerOnly: 10,
    
    // Treasury Operations
    WithdrawTreasuryFees: 11,
    GetTreasuryInfo: 12,
    DonateSol: 13,
    ConsolidatePoolFees: 14,
};
```

## System Management Examples

### Initialize Program
```javascript
async function createInitializeProgramInstruction(
    programAuthority: PublicKey,
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
    
    // Serialize instruction data
    const instructionData = Buffer.from([
        PoolInstruction.InitializeProgram
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
    ratioB: BN
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
            Buffer.from("pool_state_v2"),
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
        ratioB.toArrayLike(Buffer, 'le', 8)
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
            Buffer.from("pool_state_v2"),
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
    
    // Create instruction
    const instruction = await createInitializePoolInstruction(
        wallet.publicKey,
        tokenAMint,
        tokenBMint,
        ratioA,
        ratioB
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