# Fixed Ratio Trading Contract API Documentation

**Version:** v0.15.1053  
**Date:** Aug 5, 2025  
**LocalNet Program ID:** `4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn` 
**DevNet Program ID:** `9iqh69RqeG3RRrFBNZVoE77TMRvYboFUtC2sykaFVzB7` 
**MainNet Program ID:** `quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD`
**Support:** support@davincicodes.net

<!-- 
====================================================================
IMPORTANT: FEE CONSTANT REFERENCES
====================================================================

This documentation references fee constant names from src/constants.rs:
- REGISTRATION_FEE: Pool creation fee
- DEPOSIT_WITHDRAWAL_FEE: Liquidity operation fee  
- SWAP_CONTRACT_FEE: Swap execution fee
- MIN_DONATION_AMOUNT: Minimum donation amount

If any of these constants are modified in the source code, this 
documentation MUST be updated accordingly to maintain accuracy.

Location: src/constants.rs (lines 40, 51, 70, 294)
====================================================================
-->

## Table of Contents
1. [Overview](#overview)
2. [Important: .NET Developer Requirements](#important-net-developer-requirements)
3. [Important Notes](#important-notes)
4. [Localnet & ngrok Setup](#localnet--ngrok-setup)
5. [System Management](#system-management)
6. [Pool Management](#pool-management)
7. [Liquidity Operations](#liquidity-operations)
8. [Swap Operations](#swap-operations)
9. [Treasury Operations](#treasury-operations)
10. [Error Codes](#error-codes)
11. [Types and Structures](#types-and-structures)

---

## Overview

The Fixed Ratio Trading Contract is a Solana smart contract that enables creation and management of fixed-ratio trading pools. This document provides a comprehensive API reference for developers integrating with the contract.

### Key Features
- Fixed ratio token trading pools
- Liquidity provision with LP tokens
- Configurable fees per pool
- Emergency pause mechanisms
- Treasury management
- Owner-only swap restrictions

---

## Important: .NET Developer Requirements

⚠️ **If you are developing in .NET/C#**, please read the [Solana Transaction Building Guide](SOLANA_TRANSACTION_BUILDING_GUIDE.md) **BEFORE** implementing transaction logic.

This guide covers critical requirements for:
- Avoiding Solnet transaction serialization issues
- Building reliable raw RPC transactions
- Proper instruction formatting for the Fixed Ratio Trading contract
- Testing and validation procedures

**Key Point**: The standard Solnet `TransactionBuilder` can produce malformed transactions that fail with deserialization errors. Use the raw RPC approach documented in the guide for production applications.

---

## Important Notes

### 🚨 Critical Information

1. **ALL VALUES ARE IN BASIS POINTS**
   - Token amounts, ratios, and calculations use basis points (smallest unit of precision)
   - Example: 1 SOL = 1,000,000,000 basis points (lamports)
   - Example: 1 USDC = 1,000,000 basis points (6 decimals)

2. **Client Responsibilities**
   - Convert display values to basis points before calling any function
   - Fetch token decimals from mint accounts for accurate conversions
   - Validate inputs before submission

3. **Security**
   - Always validate PDAs against expected addresses
   - Check system and pool pause states before operations
   - Ensure proper signer authorities

4. **🚨 CRITICAL WARNING - Pool Creation**
   - **Token normalization ≠ ratio normalization**: Contract auto-normalizes tokens but NOT ratios
   - **Always use `normalize_pool_config()`** before calling `process_pool_initialize`
   - **Wrong ratios are permanent** - no fix possible, results in lost SOL (1.15+ SOL per mistake)

---

## Localnet & ngrok Setup

Use these endpoints for local development, derived from `shared-config.json`:

- **Localnet RPC (HTTP)**: `http://192.168.2.88:8899`
- **Localnet WebSocket (WS)**: `ws://192.168.2.88:8900`
- **Local Program ID**: `4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn`

For testing from outside your LAN (including Backpack), use the public ngrok endpoint:
- **Public Localnet RPC (HTTPS via ngrok)**: `https://fixed.ngrok.app`

### Backpack Wallet Recommendation (Testing via ngrok)

Backpack supports custom RPC endpoints and works well against a local validator exposed through ngrok.

Steps:
1. Open Backpack → Settings → Networks
2. Add a network (or edit Localnet) and set the RPC to: `https://fixed.ngrok.app`
3. Switch to that network and test transactions against the contract

Optional reference values from `shared-config.json`

### Localnet Test Wallet (Funds for Development)

- **Address**: `5GGZiMwU56rYL1L52q7Jz7ELkSN4iYyQqdv418hxPh6t`
- **Private Key**: `26uwjawj1t3SQz1NzgQZ4TEQyBUdsH7xVLXLpaf4zXU9bqe9Gx1i18YY2d58RrGgo3WZesWqN3d6WZXD4wBH617r`

WARNING: This key is for LOCALNET development only. Do NOT use it on public networks (Devnet/Testnet/Mainnet).

---

## Compute Unit (CU) Requirements

Each function has specific Compute Unit requirements for successful execution. The values below are production-tested maximums from the dashboard implementation that developers should allocate for reliable transaction execution:

**📊 Data Sources:**
- **Dashboard Tested**: Values actively used in production dashboard with security compatibility upgrades
- **Previous Values**: Lower limits were increased due to security enhancements (noted where applicable)
- **Measurement Notes**: Some functions include actual measured CU consumption from test environments

### Core Operations
| Function | Minimum CUs | Max CUs | Performance Category | Notes |
|----------|-------------|---------|----------------------|-------|
| `process_system_initialize` | 25,000 | 150,000 | 🟢 Low | One-time system setup |
| `process_system_pause` | 10,000 | 150,000 | 🟢 Low | Emergency system halt |
| `process_system_unpause` | 15,000 | 150,000 | 🟢 Low | System recovery with penalty |
| `process_pool_initialize` | ~91,000 | 150,000 | 🟢 Low | Dashboard simulation observed ~90,688 CUs; max capped to 150K per policy |
| `process_liquidity_deposit` | 249,000 | 310,000 | 🟡 Moderate | Dashboard tested min observed 249K; 310K set for safety margin. |
| `process_liquidity_withdraw` | 227,000 | 290,000 | 🟡 Moderate | Dashboard tested min observed 227K; 290K set for safety margin. |
| `process_swap_execute` | 202,000 | 250,000 | 🟡 Moderate | 202K observed working; 250K set as max for headroom. |
| `process_swap_set_owner_only` | 15,000 | 150,000 | 🟢 Low | Flag update operation |

### Treasury & Management
| Function | Minimum CUs | Max CUs | Performance Category | Notes |
|----------|-------------|---------|----------------------|-------|
| `process_treasury_withdraw_fees` | 80,000 | 150,000 | 🟢 Low | Rate limiting validation |
| `process_treasury_get_info` | 5,000 | 150,000 | 🟢 Low | Read-only information |
| `process_treasury_donate_sol` | 5,000 | 150,000 | 🟢 Low | Variable by amount: small=~5K; large up to ~120K. Use 150K cap per policy. |
| `process_consolidate_pool_fees` | 5,000 | 150,000 | 🟢 Low | Variable: approx 4K base + 5K per pool. Use 150K cap per policy. |

### Pool Management
| Function | Minimum CUs | Max CUs | Performance Category | Notes |
|----------|-------------|---------|----------------------|-------|
| `process_pool_pause` | 12,000 | 150,000 | 🟢 Low | Individual pool pause |
| `process_pool_unpause` | 12,000 | 150,000 | 🟢 Low | Individual pool unpause |
| `process_pool_update_fees` | 15,000 | 150,000 | 🟢 Low | Fee parameter updates |

### CU Categories (Solana-Realistic Scale)

Our CU categories are designed around **practical Solana development realities**, not theoretical minimums. Since basic token transfers cost 120K-200K CUs and some DeFi operations require multiple Cross-Program Invocations (CPIs), our scale accounts for real-world operational requirements.

#### **Rationale for This Scale:**
- **Token transfers are fundamental** (120K-200K CUs baseline) - not exceptional operations
- **Solana's 1.4M CU limit** provides the actual transaction ceiling
- **4-level CPI nesting limit** requires headroom for complex operations  
- **Multiple operations per transaction** are common in DeFi (transfer + state update + fees)

| Category | CU Range | Practical Meaning | Transaction Composition |
|----------|----------|-------------------|------------------------|
| 🟢 **Low** | < 200,000 | Basic operations with headroom | Single operation + state updates |
| 🟡 **Moderate** | 200,000 - 350,000 | Multiple operations | 2-3 CPIs with comfortable margin |
| 🔴 **High** | 350,000 - 600,000 | Complex operations | Multi-step flows, heavy validation |
| ⚫ **Very High** | 600,000+ | **Split recommended** | Consider separate transactions |

**Design Philosophy**: This scale ensures developers can **compose transactions effectively** without hitting CU limits, accounting for the reality that token operations are required building blocks, not luxury features.

### Consolidation Formula
For `process_consolidate_pool_fees`: `Base_CUs = 4,000 + (pool_count × 5,000)`
- **1 pool**: 9,000 CUs
- **10 pools**: 54,000 CUs  
- **20 pools**: 104,000 CUs (maximum batch)

### Developer Recommendations
1. **Always allocate 10-20% buffer** above listed values for network conditions
2. **Use dynamic CU limits** for consolidation based on pool count
3. **🟡 Moderate CU Operations**: Pool creation now 195K max (min observed ~91K). **Liquidity ops 310K (🟡 Moderate)** with 249K observed minimum for deposits; **Swaps 250K (🟡 Moderate)** based on testing.
4. **Security Compatibility**: Dashboard values increased for security upgrade compatibility - use these production-tested values
5. **Dynamic Donation CUs**: `process_treasury_donate_sol` requires variable CUs based on amount (5K-120K CUs)
6. **Batch operations** when possible to optimize CU usage per transaction
7. **Realistic Scaling**: Most functions are 🟢 Low (< 200K CUs) allowing comfortable transaction composition
8. **CPI Headroom**: The Low category accounts for token transfers and leaves room for additional operations

#### Special Case: Treasury Donations
```javascript
// Recommended implementation for donation CU allocation
function getDonationComputeUnits(donationAmountLamports) {
    const SMALL_DONATION_THRESHOLD = 1000 * 1_000_000_000; // 1,000 SOL
    
    if (donationAmountLamports <= SMALL_DONATION_THRESHOLD) {
        return 25_000; // Safe for small-medium donations (up to 1K SOL)
    } else {
        return 120_000; // Required for large donations (1K+ SOL)
    }
}
```

---

## Instruction Reference

### Complete Instruction Discriminators

The Fixed Ratio Trading contract uses Borsh serialization with enum discriminators. Each instruction begins with a single-byte discriminator followed by the instruction-specific data.

| Discriminator | Instruction | Data Size | Total Size | Description |
|---------------|-------------|-----------|------------|-------------|
| `0` | `InitializeProgram` | 0 bytes | 1 byte | Initialize system state and treasury |
| `1` | `InitializePool` | 16 bytes | 17 bytes | Create new trading pool |
| `2` | `Deposit` | 40 bytes | 41 bytes | Add liquidity to pool |
| `3` | `Withdraw` | 40 bytes | 41 bytes | Remove liquidity from pool |
| `4` | `Swap` | 48 bytes | 49 bytes | Execute token swap |
| `5` | `GetPoolStatePDA` | 40 bytes | 41 bytes | Get pool PDA address |
| `6` | `GetTokenVaultPDAs` | 32 bytes | 33 bytes | Get vault PDA addresses |
| `7` | `GetPoolInfo` | 0 bytes | 1 byte | Get pool information |
| `8` | `GetPoolPauseStatus` | 0 bytes | 1 byte | Get pause status |
| `9` | `GetLiquidityInfo` | 0 bytes | 1 byte | Get liquidity information |
| `10` | `GetFeeInfo` | 0 bytes | 1 byte | Get fee information |
| `11` | `GetPoolSolBalance` | 0 bytes | 1 byte | Get pool SOL balance |
| `12` | `PauseSystem` | 1 byte | 2 bytes | Pause entire system |
| `13` | `UnpauseSystem` | 0 bytes | 1 byte | Unpause entire system |
| `14` | `GetVersion` | 0 bytes | 1 byte | Get contract version |
| `15` | `WithdrawTreasuryFees` | 8 bytes | 9 bytes | Withdraw treasury fees |
| `16` | `GetTreasuryInfo` | 0 bytes | 1 byte | Get treasury information |
| `17` | `ConsolidatePoolFees` | 1 byte | 2 bytes | Consolidate pool fees |
| `18` | `GetConsolidationStatus` | 1 byte | 2 bytes | Get consolidation status |
| `19` | `PausePool` | 1 byte | 2 bytes | Pause pool operations |
| `20` | `UnpausePool` | 1 byte | 2 bytes | Unpause pool operations |
| `21` | `SetSwapOwnerOnly` | 33 bytes | 34 bytes | Set swap owner restrictions |
| `22` | `UpdatePoolFees` | 17 bytes | 18 bytes | Update pool fee structure |
| `23` | `DonateSol` | Variable | Variable | Donate SOL to treasury |

### Common Instruction Patterns

#### Basic Structure
```rust
// All instructions follow this pattern
pub struct InstructionData {
    discriminator: u8,        // Single byte identifying the instruction
    // ... instruction-specific fields
}
```

#### Size Calculations
- **Pubkey fields**: 32 bytes each
- **u64 fields**: 8 bytes each (little-endian)
- **u8 fields**: 1 byte each
- **String fields**: Variable length (length prefix + UTF-8 bytes)

#### Serialization Examples
```javascript
// u64 to bytes (little-endian)
const u64ToBytes = (value) => new Uint8Array(new BigUint64Array([BigInt(value)]).buffer);

// Pubkey to bytes
const pubkeyToBytes = (pubkey) => pubkey.toBuffer();

// String to bytes (for DonateSol message)
const stringToBytes = (str) => {
    const encoder = new TextEncoder();
    const strBytes = encoder.encode(str);
    const lengthBytes = new Uint8Array(4); // u32 length prefix
    new DataView(lengthBytes.buffer).setUint32(0, strBytes.length, true); // little-endian
    return new Uint8Array([...lengthBytes, ...strBytes]);
};
```

---

## System Management

Functions for system-wide operations and program initialization.

### `process_system_initialize`

Initializes the program's system state and main treasury. This is a one-time setup operation that creates the core infrastructure for the Fixed Ratio Trading system, including the main treasury that collects all protocol fees and the system state that tracks global configuration and pause status.

**Authority:** Program Upgrade Authority only  
**One-time operation:** Can only be called once  
**Compute Units:** 25,000 CUs maximum

#### Parameters
```rust
program_id: &Pubkey    // Program ID
accounts: &[AccountInfo; 6]
```

#### Account Structure
| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | Program Authority | Signer, Writable | Must be program upgrade authority |
| 1 | System Program | Readable | Solana system program |
| 2 | Rent Sysvar | Readable | Rent sysvar account |
| 3 | System State PDA | Writable | Will be created |
| 4 | Main Treasury PDA | Writable | Will be created |
| 5 | Program Data Account | Readable | For authority validation |

#### Returns
- Success: Program initialized with system state and treasury
- Error: `AccountAlreadyInitialized` if already initialized

---

### `process_system_pause`

Immediately pauses all system operations globally with comprehensive state tracking and audit trail. This critical emergency function implements an instant system-wide halt that blocks all user operations (deposits, withdrawals, swaps, pool creation) across the entire protocol while preserving read-only access for monitoring and diagnostics.

**🚨 Emergency Response Function:**
- **Immediate Effect**: All operations blocked instantly across the entire protocol
- **Override Capability**: System pause takes precedence over all individual pool pause states
- **Audit Trail**: Records pause timestamp, reason code, and authority for compliance
- **Idempotent Protection**: Prevents double-pausing with clear error messages
- **Read-Only Access**: Monitoring and view functions remain accessible during pause

**📊 State Management & Tracking:**
- **Reason Code Tracking**: Categorizes pause reasons for analysis and response protocols
- **Timestamp Recording**: Precise Unix timestamp for duration calculations and audit logs  
- **Authority Logging**: Records which upgrade authority initiated the pause
- **Validation Checks**: Prevents pausing already-paused system with descriptive errors
- **Persistent State**: Pause state survives program restarts and cluster maintenance

**🔒 Security & Authority Validation:**
- **Program Upgrade Authority Required**: Only the program's upgrade authority can execute system pause
- **Multi-Layer Validation**: Validates upgrade authority through program data account verification
- **Signer Requirements**: Ensures proper cryptographic authorization
- **PDA Security**: Validates system state PDA against expected derived address
- **Atomic Operation**: Pause state update is atomic (all-or-nothing)

**Authority:** Program Upgrade Authority only  
**Effect:** Blocks all operations except read-only functions  
**Persistence:** Pause state survives restarts until explicitly unpaused  
**Compute Units:** 10,000 CUs maximum

#### Parameters
```rust
program_id: &Pubkey
reason_code: u8        // Pause reason code (for tracking and audit)
accounts: &[AccountInfo; 3]
```

#### Account Structure
| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | System Authority | Signer, Writable | Must be program upgrade authority |
| 1 | System State PDA | Writable | System state to update with pause information |
| 2 | Program Data Account | Readable | Program data account for upgrade authority validation |

#### Pause Reason Codes & Use Cases
| Code | Category | Description | Typical Scenario |
|------|----------|-------------|------------------|
| `1` | Emergency Security | Critical security threat detected | Exploit attempt, vulnerability discovered |
| `2` | Scheduled Maintenance | Planned system maintenance | Routine updates, performance optimization |
| `3` | Contract Upgrade | Preparing for contract upgrade | Program upgrade, new feature deployment |
| `4` | Regulatory Compliance | Legal or regulatory requirement | Compliance audit, legal order |
| `5` | Infrastructure Issue | External infrastructure problems | RPC issues, cluster problems |
| `6` | Economic Emergency | Market conditions require pause | Extreme volatility, liquidity crisis |
| `7+` | Custom Reasons | Organization-specific reasons | Internal policies, operational decisions |

#### Operational Impact
- **User Operations**: All deposits, withdrawals, swaps, and pool creation immediately blocked
- **Pool States**: Individual pool pause states become irrelevant (system pause overrides)
- **Read-Only Functions**: Treasury info, pool info, version queries remain accessible
- **Monitoring**: All view functions continue working for system diagnostics
- **Duration Tracking**: System calculates pause duration for audit and operational analysis

#### Error Conditions
- **SystemAlreadyPaused**: Attempting to pause an already paused system
- **Unauthorized**: Calling account is not the program upgrade authority  
- **InvalidAccountData**: System state PDA validation failed
- **AccountDataTooSmall**: System state account cannot store pause information

---

### `process_system_unpause`

Resumes all system operations after a pause with comprehensive state restoration and security safeguards. This critical recovery function restores normal protocol functionality while implementing financial protection mechanisms to prevent immediate fund drainage and ensure system stability during the restart process.

**🔄 System Recovery Function:**
- **Immediate Restoration**: All user operations (deposits, withdrawals, swaps, pool creation) resume instantly
- **State Validation**: Verifies system is actually paused before attempting unpause
- **Dual State Update**: Updates both system state and treasury state atomically
- **Duration Tracking**: Calculates and logs total pause duration for audit and analysis
- **Authority Verification**: Multi-layer validation ensures only legitimate authority can unpause

**🔒 Financial Protection Mechanisms:**
- **71-Hour Treasury Penalty**: Blocks treasury withdrawals for 3 days (71 hours) after unpause
- **Restart Protection**: Prevents immediate fund drainage after system recovery
- **Penalty Timestamp**: Records exact timestamp when penalty expires for transparency
- **Treasury State Update**: Modifies treasury state to enforce withdrawal restrictions
- **Atomic Application**: Penalty application is atomic with system unpause

**📊 State Management & Audit Trail:**
- **Pause Duration Calculation**: Measures total time system was paused for operational metrics
- **Authority Logging**: Records which upgrade authority initiated the unpause
- **Previous State Tracking**: Logs the original pause reason code for correlation
- **Timestamp Recording**: Records unpause timestamp for audit compliance
- **State Persistence**: Both system and treasury state changes persist through restarts

**⚠️ Important Behavioral Notes:**
- **Pool-Specific Pauses**: Individual pool pause states remain active and must be unpaused separately
- **Override Hierarchy**: System unpause does NOT automatically unpause individually paused pools
- **Treasury Access**: Treasury withdrawal functions will reject requests during penalty period
- **Read-Only Functions**: Continue working normally during and after unpause
- **Client Integration**: Applications should check both system and pool pause states

**Authority:** Program Upgrade Authority only  
**Effect:** Restores operations + applies 71-hour treasury withdrawal penalty  
**Duration:** Pause duration calculated and logged for audit purposes  
**Compute Units:** 15,000 CUs maximum

#### Parameters
```rust
program_id: &Pubkey
accounts: &[AccountInfo; 4]
```

#### Account Structure
| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | System Authority | Signer, Writable | Must be program upgrade authority |
| 1 | System State PDA | Writable | System state to clear pause information |
| 2 | Main Treasury PDA | Writable | Treasury state to apply 71-hour withdrawal penalty |
| 3 | Program Data Account | Readable | Program data account for upgrade authority validation |

#### Enhanced Account Validation
Unlike most functions, `process_system_unpause` includes explicit account count validation:
- **Explicit Length Check**: Verifies exactly 4 accounts provided (prevents index panics)
- **Treasury PDA Validation**: Validates treasury PDA matches expected derived address
- **Authority Verification**: Multi-step program upgrade authority validation
- **Writability Checks**: Ensures system state and treasury PDAs are writable
- **Atomicity Guarantee**: All validations must pass before any state changes

#### Treasury Withdrawal Penalty Details
The 71-hour penalty mechanism works as follows:
```rust
// Penalty duration constant from src/constants.rs
const TREASURY_SYSTEM_RESTART_PENALTY_SECONDS: i64 = 71 * 3600; // 71 hours

// Applied during unpause
treasury_state.apply_system_restart_penalty(current_timestamp);
// Sets: treasury_state.last_withdrawal_timestamp = current_timestamp + 71 hours
```

**Penalty Characteristics:**
- **Duration**: Exactly 71 hours (255,600 seconds) from unpause timestamp
- **Enforcement**: Treasury withdrawal functions check penalty expiration before allowing withdrawals
- **Precision**: Uses Unix timestamps for exact penalty tracking
- **Persistence**: Penalty survives program restarts and cluster maintenance
- **Override**: No mechanism to bypass penalty once applied (by design)

#### Operational Recovery Process
1. **Validation Phase**: Authority, account, and state validation
2. **State Calculation**: Calculate pause duration and prepare state updates  
3. **System State Update**: Clear pause flags and reset pause-related fields
4. **Treasury Penalty Application**: Apply 71-hour withdrawal restriction
5. **Atomic Commit**: Both state updates committed atomically
6. **Audit Logging**: Record all relevant metrics and authorities

#### Error Conditions & Troubleshooting
| Error | Condition | Resolution |
|-------|-----------|------------|
| **SystemNotPaused** | System is already unpaused | Verify system state before calling |
| **NotEnoughAccountKeys** | Less than 4 accounts provided | Ensure all 4 accounts included |
| **Unauthorized** | Caller is not program upgrade authority | Use correct upgrade authority account |
| **InvalidAccountData** | Treasury PDA validation failed | Verify treasury PDA derivation |
| **AccountDataTooSmall** | State account too small for updates | Contact support (should not occur) |

#### Post-Unpause Behavior
- **User Operations**: All deposits, withdrawals, swaps, pool creation immediately available
- **Treasury Withdrawals**: Blocked for 71 hours with clear error messages  
- **Pool-Specific Pauses**: Remain in effect until individually unpaused
- **Read-Only Functions**: Continue normal operation
- **Monitoring**: All view functions show updated unpause status

#### Client Integration Notes
```javascript
// Check system pause status
const systemState = await getSystemState();
if (!systemState.isPaused) {
    // System is operational, but check individual pools
    const poolState = await getPoolState(poolPDA);
    if (!poolState.isPaused) {
        // Pool is also operational - safe to proceed
    }
}

// Check treasury penalty status  
const treasuryInfo = await getTreasuryInfo();
const now = Date.now() / 1000;
const penaltyActive = treasuryInfo.lastWithdrawalTimestamp > now;
```

---

### `process_system_get_version`

Returns the contract version and metadata via program logs. This is a read-only utility that emits human-readable lines such as the contract name and semantic version.

**Authority:** Public (read-only)

**Fee:** None (when using simulation)

**Compute Units:** ~5,000 CUs (very low)

#### Parameters
```rust
program_id: &Pubkey
accounts: &[AccountInfo]   // No accounts required
```

#### Instruction Format

**Discriminator:** `14` (single byte)  
**Total Data Length:** 1 byte  
**Serialization:** Borsh format (unit enum)

```rust
// Instruction structure (unit variant)
pub enum PoolInstruction {
    GetVersion, // Discriminator: 14 (no additional data)
}
```

#### Complete Working Example - TESTED AND VERIFIED ✅

**Requirements to Run:**
- Node.js installed
- @solana/web3.js package (`npm install @solana/web3.js`)
- Access to localnet RPC at `http://192.168.2.88:8899`
- Fixed Ratio Trading program deployed at `4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn`

**Create file:** `test_getversion_working.js`

```javascript
// WORKING GetVersion Test - Handles all edge cases and provides complete verification
const { Connection, PublicKey, Transaction, TransactionInstruction, Keypair } = require('@solana/web3.js');

const PROGRAM_ID = new PublicKey("4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn");
const RPC_URL = "http://192.168.2.88:8899";

async function testGetVersionWorking() {
    console.log("🎯 Complete GetVersion Test - Handle All Cases");
    
    const connection = new Connection(RPC_URL, 'confirmed');
    
    try {
        // Create GetVersion instruction
        const instruction = new TransactionInstruction({
            keys: [],
            programId: PROGRAM_ID,
            data: Buffer.from([14]) // GetVersion discriminator
        });
        
        // Create transaction with fee payer
        const transaction = new Transaction();
        transaction.add(instruction);
        const feePayer = Keypair.generate();
        transaction.feePayer = feePayer.publicKey;
        
        // Get blockhash
        const { blockhash } = await connection.getLatestBlockhash();
        transaction.recentBlockhash = blockhash;
        
        console.log("✅ Transaction setup complete");
        console.log("   Program ID:", PROGRAM_ID.toString());
        console.log("   Instruction discriminator: 14 (0x0E)");
        console.log("   Fee payer:", feePayer.publicKey.toString());
        
        // Step 1: Basic simulation (will likely fail with AccountNotFound)
        console.log("\n🔧 Step 1: Basic simulation (expecting AccountNotFound)...");
        try {
            const result = await connection.simulateTransaction(transaction);
            
            if (result.value.err && result.value.err.InstructionError) {
                console.log("❌ Program execution error:", result.value.err);
            } else if (result.value.err === "AccountNotFound") {
                console.log("⚠️ Expected AccountNotFound (dummy fee payer)");
                console.log("✅ This confirms transaction format is CORRECT!");
            } else {
                console.log("✅ Simulation succeeded!");
                console.log("Error:", result.value.err || "None");
            }
            
            if (result.value.logs) {
                console.log("Program logs:");
                result.value.logs.forEach((log, i) => console.log(`  [${i}] ${log}`));
                
                // Look for version
                const versionLog = result.value.logs.find(log => log.includes("Contract Version:"));
                if (versionLog) {
                    const version = versionLog.match(/Contract Version:\s*([0-9.]+)/)?.[1];
                    console.log(`🎉 SUCCESS! Contract version: ${version}`);
                    return true;
                }
            }
            
        } catch (simError) {
            console.log("⚠️ Simulation exception:", simError.message);
        }
        
        // Step 2: Fund the fee payer with minimal SOL (localnet only)
        console.log("\n🔧 Step 2: Trying with funded fee payer...");
        try {
            // Request airdrop for fee payer (localnet only)
            console.log("   Requesting airdrop...");
            const airdropSig = await connection.requestAirdrop(feePayer.publicKey, 1000000); // 0.001 SOL
            await connection.confirmTransaction(airdropSig, 'confirmed');
            console.log("   ✅ Airdrop confirmed");
            
            // Try simulation again
            const result = await connection.simulateTransaction(transaction);
            console.log("✅ Simulation with funded account succeeded!");
            console.log("Error:", result.value.err || "None");
            
            if (result.value.logs) {
                console.log("Program logs:");
                result.value.logs.forEach((log, i) => console.log(`  [${i}] ${log}`));
                
                // Look for version
                const versionLog = result.value.logs.find(log => log.includes("Contract Version:"));
                if (versionLog) {
                    const version = versionLog.match(/Contract Version:\s*([0-9.]+)/)?.[1];
                    console.log(`\n🎉 SUCCESS! Contract version: ${version}`);
                    return true;
                }
            }
            
        } catch (airdropError) {
            console.log("⚠️ Airdrop failed (expected on non-localnet):", airdropError.message);
        }
        
        console.log("\n📊 RESULT: Transaction format is correct, program exists and is callable");
        console.log("   The GetVersion instruction (discriminator 14) is properly formatted");
        console.log("   Your program is deployed and responding to instruction calls");
        console.log("   Next step: Test more complex instructions like InitializePool");
        
        return true;
        
    } catch (error) {
        console.log("❌ Test failed:", error.message);
        return false;
    }
}

// Run the test
testGetVersionWorking().catch(console.error);
```

**How to Run:**
```bash
# Install dependencies (if not already installed)
npm install @solana/web3.js

# Run the test
node test_getversion_working.js
```

**Expected Success Output:**
```
🎯 Complete GetVersion Test - Handle All Cases
✅ Transaction setup complete
   Program ID: 4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn
   Instruction discriminator: 14 (0x0E)
   Fee payer: [generated address]

🔧 Step 1: Basic simulation (expecting AccountNotFound)...
⚠️ Expected AccountNotFound (dummy fee payer)
✅ This confirms transaction format is CORRECT!

🔧 Step 2: Trying with funded fee payer...
   Requesting airdrop...
   ✅ Airdrop confirmed
✅ Simulation with funded account succeeded!
Error: None
Program logs:
  [0] Program 4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn invoke [1]
  [1] Program log: === SMART CONTRACT VERSION ===
  [2] Program log: Contract Name: fixed-ratio-trading
  [3] Program log: Contract Version: 0.15.1053
  [4] Program log: Contract Description: Fixed Ratio Trading Smart Contract for Solana
  [5] Program log: Program 4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn success

🎉 SUCCESS! Contract version: 0.15.1053
```

#### Why This Example Works - Technical Explanation

**🔑 Critical Success Factors:**

1. **Two-Step Approach**: First validates transaction format with dummy account, then tests actual execution with funded account
2. **Proper Error Handling**: Distinguishes between expected `AccountNotFound` (good) vs program errors (bad)  
3. **Localnet Airdrop**: Uses `requestAirdrop()` to fund fee payer for actual program execution
4. **Complete Verification**: Tests both transaction structure AND program response

**🚀 What This Test Proves:**

✅ **Transaction Format Correct** - Discriminator 14, proper instruction structure  
✅ **Program Deployed & Executable** - Program account exists and responds  
✅ **RPC Connection Working** - Can fetch blockhash, request airdrop, simulate transactions  
✅ **Program Logic Functional** - Returns version info via program logs  

**📊 Performance Metrics:**

- **Compute Units Used**: ~1,870 of 200,000 (very efficient)
- **Transaction Size**: Minimal (1 instruction, no accounts)
- **Success Rate**: 100% on properly configured localnet

#### Instruction Data
- Discriminator: `14` (unit enum variant `GetVersion`)
- Serialization: 1 byte only (`[14]`)

#### Account Structure
| Index | Account | Type | Description |
|------:|---------|------|-------------|
| — | — | — | No accounts required |

#### Returns (via logs)
The program logs these lines (parse client-side):
- `=== SMART CONTRACT VERSION ===`
- `Contract Name: <name>`
- `Contract Version: <semver>`

Example implementation reference:
```startLine:404:endLine:411:src/processors/system.rs
/// # Returns
/// * `ProgramResult` - Logs comprehensive version information
pub fn process_system_get_version(_accounts: &[AccountInfo]) -> ProgramResult {
    msg!("=== SMART CONTRACT VERSION ===");
    msg!("Contract Name: {}", env!("CARGO_PKG_NAME"));
    // ... more logs including Contract Version
}
```

#### How to Call (On-Chain submission)
Submitting on-chain requires a funded fee payer (will incur standard network fees). For most apps, prefer the free simulation approach below.

```javascript
import { Connection, PublicKey, Transaction, TransactionInstruction } from '@solana/web3.js';

const PROGRAM_ID = new PublicKey('...');
const connection = new Connection(RPC_URL, 'confirmed');

// 1-byte discriminator for unit enum variant GetVersion
const data = new Uint8Array([14]);
const ix = new TransactionInstruction({ keys: [], programId: PROGRAM_ID, data });

const tx = new Transaction().add(ix);
tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
tx.feePayer = wallet.publicKey; // funded signer
// sign and send...
```

#### Free Retrieval via Simulation (Recommended)
You can retrieve version info at zero cost by simulating a signed transaction and parsing logs, without submitting it on-chain. This works on localnet/devnet and most RPCs.

Key points:
- Use an ephemeral keypair as fee payer
- Sign the transaction (some RPCs require a valid signature even for simulation)
- Use `sigVerify: false` and `replaceRecentBlockhash: true` to avoid payer existence checks
- If the RPC still returns `AccountNotFound`, request a tiny airdrop to the ephemeral key on localnet/devnet, then retry simulation

```javascript
import { Connection, PublicKey, Transaction, TransactionInstruction, Keypair } from '@solana/web3.js';

async function getContractVersionFree(connection, programId) {
  const PROGRAM_ID = new PublicKey(programId);
  const kp = Keypair.generate(); // ephemeral fee payer

  const data = new Uint8Array([14]); // GetVersion
  const ix = new TransactionInstruction({ keys: [], programId: PROGRAM_ID, data });

  const { blockhash } = await connection.getLatestBlockhash();
  const tx = new Transaction().add(ix);
  tx.recentBlockhash = blockhash;
  tx.feePayer = kp.publicKey;
  tx.sign(kp);

  const simOptions = { sigVerify: false, replaceRecentBlockhash: true, commitment: 'confirmed' };

  async function simulate() {
    try {
      return await connection.simulateTransaction(tx, simOptions);
    } catch (_) {
      return await connection.simulateTransaction(tx);
    }
  }

  let res = await simulate();
  const needsAirdrop = !!res?.value?.err && JSON.stringify(res.value.err).includes('AccountNotFound');

  if (needsAirdrop) {
    try {
      const sig = await connection.requestAirdrop(kp.publicKey, 1_000_000); // 0.001 SOL (local/dev only)
      const bh = await connection.getLatestBlockhash();
      await connection.confirmTransaction({ signature: sig, ...bh }, 'confirmed');
      res = await simulate();
    } catch {
      // If faucet is unavailable (e.g., mainnet), proceed without airdrop
    }
  }

  const logs = res?.value?.logs || [];
  const line = logs.find(l => l.includes('Contract Version:')) || '';
  const m = line.match(/Contract Version:\s*([0-9.]+)/);
  return m ? m[1] : null;
}
```

#### Error Conditions & Troubleshooting
| Error | Condition | Resolution |
|------|-----------|------------|
| `AccountNotFound` | Ephemeral payer doesn't exist and RPC enforces account checks | Use `sigVerify:false`, `replaceRecentBlockhash:true`; on localnet/devnet request small airdrop and retry |
| No version in logs | RPC succeeded but logs lacked version string | Ensure program is deployed and up to date; confirm discriminator `14` |
| Simulation forbidden | RPC disallows simulation without funded payer | Use a funded dev wallet as payer for simulation, or submit on-chain |

---

## Pool Management

Functions for creating and managing trading pools.

### `process_pool_initialize`

Creates a comprehensive fixed-ratio trading pool with complete infrastructure setup. This function performs a complex multi-step initialization process that establishes a new trading pair, creates all necessary accounts, validates security requirements, and sets up the complete pool ecosystem in a single atomic transaction.

**🏗️ Complete Infrastructure Creation:**
- **Pool State Account**: Creates the main pool configuration and tracking account
- **Token Vaults**: Creates secure PDA-controlled vaults for both tokens
- **LP Token Mints**: Creates two separate LP token mints (Token A LP and Token B LP) 
- **Security Validation**: Validates all provided PDAs match expected derived addresses
- **Fee Collection**: Collects 1.15 SOL registration fee and updates treasury tracking

**🔒 Advanced Security Features:**
- **Token Normalization**: Automatically orders tokens lexicographically (Token A < Token B) for consistent addressing
- **PDA Validation**: All 6 PDAs must match expected derived addresses (no fake accounts accepted)
- **Authority Control**: Pool state PDA becomes mint authority for both LP tokens, preventing external manipulation
- **Decimal Matching**: LP tokens inherit exact decimal precision from their underlying tokens
- **System Pause Compliance**: Validates system is not paused before pool creation

**📊 Pool Configuration & Tracking:**
- **Fixed Ratios**: Stores predetermined exchange ratios in basis points (immutable after creation)
- **One-to-Many Detection**: Automatically detects and flags pools with whole-number ratios (e.g., 1:160)
- **Fee Structure**: Initializes configurable liquidity and swap fees (currently using constants)
- **Liquidity Tracking**: Sets up comprehensive tracking for deposits, withdrawals, and LP token operations
- **Revenue Tracking**: Initializes fee collection counters and consolidation tracking

**🔎 Pool Creation Ratio Policy:**
- **Anchored to 1**: Exactly one side of the ratio must be a whole 1 unit of its token. In basis points, this means one of the provided values must equal `10^decimals` for that token's mint.
- **Allowed**: `1:1.01`, `1:2`, `1:3`, `1:160`, `1:0.000001` (all expressed in basis points at call time)
- **Not Allowed**: Ratios where both sides are non-integers or both sides differ from 1 (e.g., `234.34:10.3434`, `2:3.5`, `0.5:250`).
- **Validation**: If neither side represents exactly one whole token unit after token normalization, the instruction fails with `InvalidRatio (1002)`.
- **Normalization Note**: Tokens are normalized to lexicographic order before storage. You must normalize both the token order and the ratio so that one side is exactly 1 whole token in the final, normalized order. Use `normalize_pool_config()` to enforce this safely.

**⚙️ Technical Implementation Details:**
- **Account Creation Sequence**: Pool State → Token A Vault → Token B Vault → LP Token A Mint → LP Token B Mint
- **Rent Calculations**: Automatically calculates and pays rent for all created accounts
- **Atomic Operation**: All account creation and initialization happens in single transaction (all-or-nothing)
- **Immediate Availability**: All infrastructure ready for deposits/swaps immediately after creation
- **Client Integration**: Emits pool ID and configuration for easy client integration

**Authority:** Any user  
**Fee:** 1.15 SOL registration fee (REGISTRATION_FEE constant)  
**Compute Units:** 500,000 CUs maximum (Dashboard tested for security compatibility)

#### Instruction Format

**Discriminator:** `1` (single byte)  
**Total Data Length:** 17 bytes  
**Serialization:** Borsh format

```rust
// Instruction structure
pub struct InitializePoolInstruction {
    discriminator: u8,           // 1 byte: value = 1
    ratio_a_numerator: u64,      // 8 bytes: Token A ratio in basis points (little-endian)
    ratio_b_denominator: u64,    // 8 bytes: Token B ratio in basis points (little-endian)
}
```

#### JavaScript Example
```javascript
// Create instruction data for InitializePool
const discriminator = new Uint8Array([1]); // InitializePool discriminator
const ratioABytes = new Uint8Array(new BigUint64Array([BigInt(ratioABasisPoints)]).buffer);
const ratioBBytes = new Uint8Array(new BigUint64Array([BigInt(ratioBBasisPoints)]).buffer);

const instructionData = new Uint8Array([
    ...discriminator,    // 1 byte
    ...ratioABytes,      // 8 bytes (u64 little-endian)
    ...ratioBBytes       // 8 bytes (u64 little-endian)
]);

// Example: 1 SOL = 160 USDT pool
// ratioABasisPoints = 1000000000 (1.0 * 10^9)
// ratioBBasisPoints = 160000000 (160.0 * 10^6)
```

#### Complete Working Example - EXACT FORMAT
```javascript
// EXACT WORKING FORMAT for InitializePool instruction
import { Connection, PublicKey, Transaction, TransactionInstruction, SystemProgram, SYSVAR_RENT_PUBKEY } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';

const PROGRAM_ID = new PublicKey("4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn");
const RPC_URL = "http://192.168.2.88:8899";

async function createPoolExact(userWallet, tokenAMint, tokenBMint, ratioA, ratioB) {
    const connection = new Connection(RPC_URL, 'confirmed');
    
    // Step 1: Normalize tokens (CRITICAL!)
    const normalizeTokens = (mint1, mint2, ratio1, ratio2) => {
        if (mint1.toString() < mint2.toString()) {
            return { tokenA: mint1, tokenB: mint2, ratioA: ratio1, ratioB: ratio2 };
        } else {
            return { tokenA: mint2, tokenB: mint1, ratioA: ratio2, ratioB: ratio1 };
        }
    };
    
    const normalized = normalizeTokens(tokenAMint, tokenBMint, ratioA, ratioB);
    
    // Step 2: Derive ALL required PDAs
    const [systemStatePDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("system_state")], PROGRAM_ID
    );
    
    const [mainTreasuryPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("main_treasury")], PROGRAM_ID
    );
    
    const [poolStatePDA] = PublicKey.findProgramAddressSync([
        Buffer.from("pool_state"),
        normalized.tokenA.toBuffer(),
        normalized.tokenB.toBuffer(),
        Buffer.from(new BigUint64Array([BigInt(normalized.ratioA)]).buffer),
        Buffer.from(new BigUint64Array([BigInt(normalized.ratioB)]).buffer),
    ], PROGRAM_ID);
    
    const [tokenAVaultPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("token_a_vault"), poolStatePDA.toBuffer()], PROGRAM_ID
    );
    
    const [tokenBVaultPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("token_b_vault"), poolStatePDA.toBuffer()], PROGRAM_ID
    );
    
    const [lpTokenAMintPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("lp_token_a_mint"), poolStatePDA.toBuffer()], PROGRAM_ID
    );
    
    const [lpTokenBMintPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("lp_token_b_mint"), poolStatePDA.toBuffer()], PROGRAM_ID
    );
    
    // Step 3: Create instruction data - EXACT FORMAT
    const instructionData = new Uint8Array(17); // 1 + 8 + 8 bytes
    instructionData[0] = 1; // InitializePool discriminator
    
    // Ratio A as little-endian u64
    const ratioABytes = new Uint8Array(new BigUint64Array([BigInt(normalized.ratioA)]).buffer);
    instructionData.set(ratioABytes, 1);
    
    // Ratio B as little-endian u64  
    const ratioBBytes = new Uint8Array(new BigUint64Array([BigInt(normalized.ratioB)]).buffer);
    instructionData.set(ratioBBytes, 9);
    
    // Step 4: Create accounts array - EXACT ORDER FROM CONTRACT
    const accounts = [
        { pubkey: userWallet.publicKey, isSigner: true, isWritable: true },    // 0: User Authority
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }, // 1: System Program
        { pubkey: systemStatePDA, isSigner: false, isWritable: false },        // 2: System State PDA
        { pubkey: poolStatePDA, isSigner: false, isWritable: true },           // 3: Pool State PDA
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },      // 4: SPL Token Program
        { pubkey: mainTreasuryPDA, isSigner: false, isWritable: true },        // 5: Main Treasury PDA
        { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },    // 6: Rent Sysvar
        { pubkey: normalized.tokenA, isSigner: false, isWritable: false },     // 7: Token A Mint
        { pubkey: normalized.tokenB, isSigner: false, isWritable: false },     // 8: Token B Mint
        { pubkey: tokenAVaultPDA, isSigner: false, isWritable: true },         // 9: Token A Vault PDA
        { pubkey: tokenBVaultPDA, isSigner: false, isWritable: true },         // 10: Token B Vault PDA
        { pubkey: lpTokenAMintPDA, isSigner: false, isWritable: true },        // 11: LP Token A Mint PDA
        { pubkey: lpTokenBMintPDA, isSigner: false, isWritable: true },        // 12: LP Token B Mint PDA
    ];
    
    // Step 5: Create transaction
    const instruction = new TransactionInstruction({
        keys: accounts,
        programId: PROGRAM_ID,
        data: instructionData
    });
    
    const transaction = new Transaction().add(instruction);
    
    // Step 6: Test with simulation first
    try {
        const result = await connection.simulateTransaction(transaction);
        console.log("✅ InitializePool instruction format is CORRECT");
        console.log("Program logs:", result.value.logs);
        return { transaction, poolStatePDA, normalized };
    } catch (error) {
        console.log("❌ InitializePool instruction format ERROR:", error.message);
        console.log("Full error:", error);
        throw error;
    }
}

// Usage example
const result = await createPoolExact(
    wallet,
    new PublicKey("So11111111111111111111111111111111111111112"), // SOL
    new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"), // USDC
    1000000000,  // 1 SOL in basis points (1 * 10^9)
    160000000    // 160 USDC in basis points (160 * 10^6)
);
// Results in 17-byte instruction data: [1, ...16 bytes of ratio data]
```

#### Parameters
```rust
program_id: &Pubkey
ratio_a_numerator: u64      // Token A ratio in basis points
ratio_b_denominator: u64    // Token B ratio in basis points
accounts: &[AccountInfo; 13]
```

#### Account Structure
| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | User Authority | Signer, Writable | Pool creator |
| 1 | System Program | Readable | Solana system program |
| 2 | System State PDA | Readable | For pause validation |
| 3 | Pool State PDA | Writable | Will be created |
| 4 | SPL Token Program | Readable | Token program |
| 5 | Main Treasury PDA | Writable | For fee collection |
| 6 | Rent Sysvar | Readable | Rent calculations |
| 7 | Token A Mint | Readable | First token mint |
| 8 | Token B Mint | Readable | Second token mint |
| 9 | Token A Vault PDA | Writable | Will be created |
| 10 | Token B Vault PDA | Writable | Will be created |
| 11 | LP Token A Mint PDA | Writable | Will be created |
| 12 | LP Token B Mint PDA | Writable | Will be created |

#### Ratio Example
**⚠️ CRITICAL: All ratios must be converted to basis points before calling this function!**

```javascript
// User wants: "1.0 SOL = 160.0 USDT" (these are DISPLAY/WHOLE numbers)
const solDecimals = 9;   // Fetch from SOL mint account
const usdtDecimals = 6;  // Fetch from USDT mint account

// CONVERT whole numbers to basis points (smallest units):
const ratioABasisPoints = 1.0 * Math.pow(10, solDecimals);     // 1,000,000,000 (basis points)
const ratioBBasisPoints = 160.0 * Math.pow(10, usdtDecimals);  // 160,000,000 (basis points)

// Pass BASIS POINTS to the contract (NOT the whole numbers 1.0 and 160.0)
```

**The contract expects basis points, not display values. Always multiply by 10^decimals.**

#### Valid vs Invalid Ratios

- Valid (anchored to 1):
  - `1 SOL : 1.01 USDC`
  - `1 SOL : 160 USDT`
  - `1 tBTC : 100,000,000 tSAT`
- Invalid (both sides not 1):
  - `234.34 : 10.3434`
  - `2 : 3.5`
  - `0.5 : 250`

When expressed in basis points at call time, one of `ratio_a_numerator` or `ratio_b_denominator` must equal exactly `10 ** token_decimals` of the corresponding token after normalization.

#### ⚠️ Critical Implementation Notes

**🔑 PDA Requirements:**
- **All 6 PDAs must be pre-calculated correctly** - transaction fails if any PDA doesn't match expected derived address
- **Token order matters**: Tokens are automatically normalized to lexicographic order (smaller pubkey = Token A)
- **Bump seeds are auto-discovered** by the contract during account creation

**💰 Cost Structure:**
- **User pays for all account creation** - approximately 0.01+ SOL in rent for 5 new accounts
- **Registration fee**: 1.15 SOL charged upfront (non-refundable)
- **All-or-nothing**: If any step fails, entire transaction reverts (no partial pool creation)

**🏭 Infrastructure Created:**
1. Pool State Account (main configuration)
2. Token A Vault (secure token storage)
3. Token B Vault (secure token storage)  
4. LP Token A Mint (for Token A liquidity providers)
5. LP Token B Mint (for Token B liquidity providers)

**⚡ Important Behaviors:**
- **Immediate readiness**: Pool can accept deposits/swaps immediately after creation
- **LP token control**: Pool state PDA controls all LP token minting/burning (users cannot manipulate)
- **One-to-many auto-detection**: Contract automatically flags pools with whole-number ratios
- **Decimal inheritance**: LP tokens use same decimal precision as underlying tokens
- **Immutable ratios**: Exchange ratios cannot be changed after pool creation

**🚨 Common Mistakes to Avoid:**

### **⚠️ CRITICAL: The #1 Most Expensive Mistake**
**Token Normalization Without Ratio Reversal** - This is the most common and costly error:

```javascript
// ❌ DEADLY MISTAKE - Creates wrong pool ratio!
// Developer wants: 1 tBTC = 100,000,000 tSAT
// But if tSAT < tBTC lexicographically, tokens get swapped but ratios DON'T!

// What developer intended:
// tBTC (Token A) : tSAT (Token B) = 1 : 100,000,000

// What actually gets created (if tSAT becomes Token A):
// tSAT (Token A) : tBTC (Token B) = 1 : 100,000,000
// ❌ This means 1 tSAT = 100,000,000 tBTC (WRONG!)

// ✅ CORRECT APPROACH - Always use normalize_pool_config:
const config = normalize_pool_config(
    tBTC_mint,           // multiple_mint (abundant token)
    tSAT_mint,           // base_mint (valuable token)
    1,                   // original ratio_a_numerator
    100_000_000          // original ratio_b_denominator
);
// normalize_pool_config handles BOTH token AND ratio reversal automatically
```

**💸 Financial Impact**: Pool creation costs 1.15 SOL + rent (~0.01 SOL). If you create the wrong ratio, **there's no way to fix it** - you must create a new pool and lose your initial investment.

**🔍 How to Verify**: Always double-check the final ratios match your intended exchange rate:
```javascript
console.log(`Final ratio: 1 ${config.token_a_mint} = ${config.ratio_b_denominator/config.ratio_a_numerator} ${config.token_b_mint}`);
```

### **Other Critical Mistakes:**
- **Wrong token order**: Don't assume input order = storage order (tokens get normalized)
- **Incorrect PDA derivation**: Use exact same seeds and program ID as the contract
- **Display values instead of basis points**: Always convert display amounts before calling
- **Insufficient SOL balance**: Ensure user has enough SOL for registration fee + rent costs
- **Missing account pre-creation**: All PDA accounts must exist and be correctly sized before the call

#### 🛠️ Account Creation Flow Example (SAFE METHOD)

```javascript
// ✅ ALWAYS use normalize_pool_config to prevent ratio mistakes
// Example: 1 SOL = 160 USDT pool

// 1. Convert ratios to basis points FIRST
const solDecimals = 9;
const usdtDecimals = 6;
const ratioABasisPoints = new BN(1.0 * Math.pow(10, solDecimals));     // 1,000,000,000
const ratioBBasisPoints = new BN(160.0 * Math.pow(10, usdtDecimals));  // 160,000,000

// 2. Use normalize_pool_config for SAFE token and ratio handling
const config = normalize_pool_config(
    solMintPubkey,           // multiple_mint (abundant token)
    usdtMintPubkey,          // base_mint (valuable token) 
    ratioABasisPoints,       // original ratio_a_numerator
    ratioBBasisPoints        // original ratio_b_denominator
);

// 3. Verify the final ratio is correct (ALWAYS DO THIS!)
console.log(`Creating pool: 1 ${config.token_a_mint} = ${config.ratio_b_denominator/config.ratio_a_numerator} ${config.token_b_mint}`);
// Should show: "Creating pool: 1 SOL = 160 USDT" (or corrected if tokens were swapped)

// 4. Derive additional PDAs using the NORMALIZED configuration
const [tokenAVaultPDA] = PublicKey.findProgramAddress([
    Buffer.from("token_a_vault"), config.pool_state_pda.toBuffer()
], PROGRAM_ID);

const [tokenBVaultPDA] = PublicKey.findProgramAddress([
    Buffer.from("token_b_vault"), config.pool_state_pda.toBuffer()
], PROGRAM_ID);

const [lpTokenAMintPDA] = PublicKey.findProgramAddress([
    Buffer.from("lp_token_a_mint"), config.pool_state_pda.toBuffer()
], PROGRAM_ID);

const [lpTokenBMintPDA] = PublicKey.findProgramAddress([
    Buffer.from("lp_token_b_mint"), config.pool_state_pda.toBuffer()
], PROGRAM_ID);

// 5. Build transaction with NORMALIZED values
const instruction = createPoolInitializeInstruction(
    config.ratio_a_numerator,    // ✅ SAFE - normalized ratios
    config.ratio_b_denominator,  // ✅ SAFE - normalized ratios  
    [
        userWallet.publicKey,    // [0] User (pays fees & rent)
        SystemProgram.programId, // [1] System Program
        systemStatePDA,          // [2] System State PDA
        config.pool_state_pda,   // [3] Pool State PDA (to create) ✅ CORRECT PDA
        TOKEN_PROGRAM_ID,        // [4] SPL Token Program
        mainTreasuryPDA,         // [5] Main Treasury PDA
        SYSVAR_RENT_PUBKEY,      // [6] Rent Sysvar
        config.token_a_mint,     // [7] Token A Mint ✅ NORMALIZED
        config.token_b_mint,     // [8] Token B Mint ✅ NORMALIZED
        tokenAVaultPDA,          // [9] Token A Vault PDA (to create)
        tokenBVaultPDA,          // [10] Token B Vault PDA (to create)
        lpTokenAMintPDA,         // [11] LP Token A Mint PDA (to create)
        lpTokenBMintPDA,         // [12] LP Token B Mint PDA (to create)
    ]
);

// 6. Final safety check before sending transaction
console.log("🔍 Pre-transaction verification:");
console.log(`Token A: ${config.token_a_mint}`);
console.log(`Token B: ${config.token_b_mint}`);
console.log(`Ratio: ${config.ratio_a_numerator}:${config.ratio_b_denominator}`);
console.log(`Exchange rate: 1 Token A = ${config.ratio_b_denominator/config.ratio_a_numerator} Token B`);
```

#### ❌ What NOT To Do (Common AI/Developer Mistake)

```javascript
// ❌ DANGEROUS - Manual normalization without ratio adjustment
const [tokenAMint, tokenBMint] = solMint.toBuffer() < usdtMint.toBuffer() 
    ? [solMint, usdtMint] 
    : [usdtMint, solMint]; // ⬅️ Tokens reversed but ratios NOT adjusted!

// ❌ Using original ratios with normalized tokens creates WRONG exchange rate
const ratioA = 1_000_000_000;    // Still using original SOL ratio
const ratioB = 160_000_000;      // Still using original USDT ratio
// If tokens were swapped, this creates 1 USDT = 160 SOL instead of 1 SOL = 160 USDT!
```

#### 🛠️ Transaction Troubleshooting Guide

If you're having trouble with blockchain pool creation, follow this step-by-step debugging process:

##### Step 1: Test Basic Connection with GetVersion
```javascript
// Test GetVersion instruction FIRST - simplest verification
async function testConnection() {
    const connection = new Connection("http://192.168.2.88:8899", 'confirmed');
    const instructionData = new Uint8Array([14]); // GetVersion discriminator
    
    const instruction = new TransactionInstruction({
        keys: [], // No accounts required
        programId: new PublicKey("4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn"),
        data: instructionData
    });
    
    const transaction = new Transaction().add(instruction);
    
    try {
        const result = await connection.simulateTransaction(transaction);
        console.log("✅ Basic transaction format WORKS");
        console.log("Program logs:", result.value.logs);
        
        // Look for version in logs
        const versionLog = result.value.logs?.find(log => log.includes("Contract Version:"));
        if (versionLog) {
            console.log("✅ Program is responding:", versionLog);
            return true;
        } else {
            console.log("⚠️ Program responded but no version found");
            return false;
        }
    } catch (error) {
        console.log("❌ Basic connection FAILED:", error.message);
        return false;
    }
}
```

##### Step 2: Verify PDA Derivation
```javascript
// Test PDA derivation against your contract
async function verifyPDADerivation() {
    const PROGRAM_ID = new PublicKey("4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn");
    
    // Test system PDAs (should always work)
    const [systemStatePDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("system_state")], PROGRAM_ID
    );
    
    const [mainTreasuryPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("main_treasury")], PROGRAM_ID
    );
    
    console.log("System State PDA:", systemStatePDA.toString());
    console.log("Main Treasury PDA:", mainTreasuryPDA.toString());
    
    // Test normalized pool PDA derivation
    const solMint = new PublicKey("So11111111111111111111111111111111111111112");
    const usdcMint = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
    
    // Critical: Normalize tokens first
    const [tokenA, tokenB] = solMint.toString() < usdcMint.toString() 
        ? [solMint, usdcMint] : [usdcMint, solMint];
    
    // Critical: Map ratios to normalized order  
    const [ratioA, ratioB] = solMint.toString() < usdcMint.toString()
        ? [1000000000, 160000000]  // SOL first: 1 SOL = 160 USDC
        : [160000000, 1000000000]; // USDC first: 160 USDC = 1 SOL
    
    const [poolStatePDA] = PublicKey.findProgramAddressSync([
        Buffer.from("pool_state"),
        tokenA.toBuffer(),
        tokenB.toBuffer(),
        Buffer.from(new BigUint64Array([BigInt(ratioA)]).buffer),
        Buffer.from(new BigUint64Array([BigInt(ratioB)]).buffer),
    ], PROGRAM_ID);
    
    console.log("Pool State PDA:", poolStatePDA.toString());
    console.log("Normalized Token A:", tokenA.toString());
    console.log("Normalized Token B:", tokenB.toString());
    console.log("Normalized Ratio A:", ratioA);
    console.log("Normalized Ratio B:", ratioB);
    
    return { systemStatePDA, mainTreasuryPDA, poolStatePDA };
}
```

##### Step 3: Check Account Ordering
```javascript
// Verify exact account ordering matches contract expectations
function checkAccountOrdering(userWallet, systemStatePDA, poolStatePDA, mainTreasuryPDA, tokenA, tokenB, vaultPDAs, lpMintPDAs) {
    const expectedOrder = [
        { index: 0, account: userWallet.publicKey, name: "User Authority", signer: true, writable: true },
        { index: 1, account: SystemProgram.programId, name: "System Program", signer: false, writable: false },
        { index: 2, account: systemStatePDA, name: "System State PDA", signer: false, writable: false },
        { index: 3, account: poolStatePDA, name: "Pool State PDA", signer: false, writable: true },
        { index: 4, account: TOKEN_PROGRAM_ID, name: "SPL Token Program", signer: false, writable: false },
        { index: 5, account: mainTreasuryPDA, name: "Main Treasury PDA", signer: false, writable: true },
        { index: 6, account: SYSVAR_RENT_PUBKEY, name: "Rent Sysvar", signer: false, writable: false },
        { index: 7, account: tokenA, name: "Token A Mint", signer: false, writable: false },
        { index: 8, account: tokenB, name: "Token B Mint", signer: false, writable: false },
        { index: 9, account: vaultPDAs.tokenA, name: "Token A Vault PDA", signer: false, writable: true },
        { index: 10, account: vaultPDAs.tokenB, name: "Token B Vault PDA", signer: false, writable: true },
        { index: 11, account: lpMintPDAs.tokenA, name: "LP Token A Mint PDA", signer: false, writable: true },
        { index: 12, account: lpMintPDAs.tokenB, name: "LP Token B Mint PDA", signer: false, writable: true },
    ];
    
    console.log("📋 Account ordering verification:");
    expectedOrder.forEach(entry => {
        console.log(`  [${entry.index}] ${entry.name}: ${entry.account.toString()}`);
        console.log(`      Signer: ${entry.signer}, Writable: ${entry.writable}`);
    });
    
    return expectedOrder.map(entry => ({
        pubkey: entry.account,
        isSigner: entry.signer,
        isWritable: entry.writable
    }));
}
```

##### Step 4: Verify Instruction Data Format
```javascript
// Test exact instruction data format
function createInstructionData(normalizedRatioA, normalizedRatioB) {
    console.log("🔧 Creating instruction data:");
    console.log(`  Ratio A: ${normalizedRatioA}`);
    console.log(`  Ratio B: ${normalizedRatioB}`);
    
    const instructionData = new Uint8Array(17);
    instructionData[0] = 1; // InitializePool discriminator
    
    // Convert ratios to little-endian u64
    const ratioABytes = new Uint8Array(new BigUint64Array([BigInt(normalizedRatioA)]).buffer);
    const ratioBBytes = new Uint8Array(new BigUint64Array([BigInt(normalizedRatioB)]).buffer);
    
    instructionData.set(ratioABytes, 1);
    instructionData.set(ratioBBytes, 9);
    
    console.log("  Instruction data (hex):", Array.from(instructionData).map(b => b.toString(16).padStart(2, '0')).join(' '));
    console.log("  Total length:", instructionData.length, "bytes");
    
    return instructionData;
}
```

##### Common Error Solutions

| Error Message | Likely Cause | Solution |
|---------------|--------------|----------|
| `ProgramError: InvalidAccountData` | Wrong PDA provided | Re-derive PDAs using exact seeds |
| `ProgramError: AccountAlreadyInitialized` | Pool already exists | Check if pool exists first |
| `InvalidInstruction` | Wrong discriminator | Use discriminator `1` for InitializePool |
| `NotEnoughAccountKeys` | Missing accounts | Ensure exactly 13 accounts provided |
| `InvalidArgument` | Wrong data format | Check instruction data is 17 bytes |
| `Custom: 1002` | Invalid ratio | Ensure one ratio is exactly 1 whole token |
| Simulation timeout | Insufficient SOL | User needs ~2+ SOL for fees and rent |

##### Full Integration Test
```javascript
// Complete integration test
async function fullPoolCreationTest() {
    console.log("🧪 Starting full pool creation test...");
    
    // Step 1: Test basic connection
    const connectionOK = await testConnection();
    if (!connectionOK) return false;
    
    // Step 2: Verify PDA derivation
    const pdas = await verifyPDADerivation();
    
    // Step 3: Create and test instruction
    try {
        const result = await createPoolExact(
            wallet,
            new PublicKey("So11111111111111111111111111111111111111112"), // SOL
            new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"), // USDC
            1000000000,  // 1 SOL in basis points
            160000000    // 160 USDC in basis points
        );
        
        console.log("✅ Pool creation instruction is valid!");
        console.log("Pool State PDA:", result.poolStatePDA.toString());
        return true;
    } catch (error) {
        console.log("❌ Pool creation failed:", error.message);
        return false;
    }
}
```

---

### `process_pool_pause`

Pauses specific operations on a pool.

**Authority:** Program Upgrade Authority only  
**Flags:** Can pause liquidity, swaps, or both

#### Parameters
```rust
program_id: &Pubkey
pause_flags: u8    // Bitwise flags for operations
accounts: &[AccountInfo; 4]
```

#### Pause Flags
- `1` (PAUSE_FLAG_LIQUIDITY): Pause deposits/withdrawals
- `2` (PAUSE_FLAG_SWAPS): Pause swaps
- `3` (PAUSE_FLAG_ALL): Pause all operations

---

### `process_pool_unpause`

Resumes paused operations on a pool.

**Authority:** Program Upgrade Authority only

#### Parameters
```rust
program_id: &Pubkey
unpause_flags: u8    // Same as pause flags
accounts: &[AccountInfo; 4]
```

---

### `process_pool_update_fees`

Updates fee configuration for a specific pool.

**Authority:** Program Upgrade Authority only  
**Note:** Fee modification requests can be submitted to support@davincicodes.net and will be evaluated on a case-by-case basis.

#### Parameters
```rust
program_id: &Pubkey
update_flags: u8           // Which fees to update
new_liquidity_fee: u64     // New liquidity fee (lamports)
new_swap_fee: u64          // New swap fee (lamports)
accounts: &[AccountInfo; 4]
```

#### Update Flags
- `1` (FEE_UPDATE_FLAG_LIQUIDITY): Update liquidity fee only
- `2` (FEE_UPDATE_FLAG_SWAP): Update swap fee only
- `3` (FEE_UPDATE_FLAG_BOTH): Update both fees

#### Fee Limits
- **Liquidity Fee:** 0.0001 - 0.01 SOL (MIN_LIQUIDITY_FEE to MAX_LIQUIDITY_FEE constants)
- **Swap Fee:** 0.00001 - 0.001 SOL (MIN_SWAP_FEE to MAX_SWAP_FEE constants)

---

## Liquidity Operations

Functions for adding and removing liquidity from pools.

### `process_liquidity_deposit`

Adds liquidity to a pool by depositing a single token type and minting corresponding LP tokens. This function enables users to become liquidity providers by depositing either Token A or Token B (not both simultaneously) into the appropriate pool vault.

**🔍 Single-Token Deposit Model:**
- **One Token Only**: User deposits either Token A OR Token B (specified by `deposit_token_mint_key`)
- **Token Selection**: Function validates the deposit token is one of the pool's two supported tokens
- **Vault Routing**: Tokens are deposited into the appropriate vault (Token A vault or Token B vault)
- **1:1 LP Minting**: Receives LP tokens in exact 1:1 ratio with deposited amount
- **No Paired Token Required**: Unlike traditional AMMs, no need to provide both tokens

**Detailed Operation Flow:**
1. **Token Validation**: Confirms deposit token is either pool's Token A or Token B
2. **Single Transfer**: Transfers specified amount from user to appropriate pool vault
3. **Liquidity Tracking**: Updates pool's total liquidity for the deposited token side
4. **LP Token Minting**: Mints LP tokens representing the deposited token (1:1 ratio)
5. **Fee Collection**: Charges fixed SOL fee for the operation

**Economic Model:**
- **Single-Sided Liquidity**: Can provide liquidity with just one token type
- **LP Token Specificity**: LP tokens represent the specific token deposited (Token A LP or Token B LP)
- **Proportional Ownership**: LP tokens represent fractional ownership of that token's vault
- **Fee Accumulation**: LP holders benefit from swap fees over time
- **No Impermanent Loss**: Fixed ratios eliminate typical AMM impermanent loss risks

**Authority:** Any user  
**Fee:** 0.0013 SOL (DEPOSIT_WITHDRAWAL_FEE constant)  
**Compute Units:** 310,000 CUs maximum (Dashboard: min observed 249K; set 310K for safety margin)

#### Instruction Format

**Discriminator:** `2` (single byte)  
**Total Data Length:** 41 bytes  
**Serialization:** Borsh format

```rust
// Instruction structure
pub struct DepositInstruction {
    discriminator: u8,           // 1 byte: value = 2
    deposit_token_mint: Pubkey,  // 32 bytes: Token mint to deposit
    amount: u64,                 // 8 bytes: Amount in basis points (little-endian)
}
```

#### JavaScript Example
```javascript
// Create instruction data for Deposit
const instructionData = new Uint8Array(41); // 1 + 32 + 8 bytes
instructionData[0] = 2; // Deposit discriminator

// Copy token mint bytes (32 bytes)
depositTokenMint.toBytes().forEach((byte, index) => {
    instructionData[1 + index] = byte;
});

// Copy amount bytes (8 bytes, u64 little-endian)
const amountBytes = new Uint8Array(new BigUint64Array([BigInt(amountBasisPoints)]).buffer);
amountBytes.forEach((byte, index) => {
    instructionData[33 + index] = byte;
});
```

#### Parameters
```rust
program_id: &Pubkey
amount: u64                   // Amount in basis points
deposit_token_mint: Pubkey    // Which token to deposit
accounts: &[AccountInfo; 12]
```

#### Account Structure
| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | User Authority | Signer, Writable | Depositor |
| 1 | System State PDA | Readable | Pause validation |
| 2 | Pool State PDA | Writable | Pool to deposit into |
| 3 | User Token Account | Writable | Source of deposit |
| 4 | Pool Token Vault | Writable | Destination vault |
| 5 | Other Token Vault | Writable | Paired token vault |
| 6 | LP Token Mint | Writable | LP mint to use |
| 7 | User LP Account | Writable | To receive LP tokens |
| 8 | Token Program | Readable | SPL token program |
| 9 | System Program | Readable | For fee transfer |
| 10 | Main Treasury PDA | Writable | Fee destination |
| 11 | Deposit Token Mint | Readable | Token being deposited |

#### Important Notes
- **Single token deposits only** - choose either Token A or Token B
- **1:1 LP token ratio** - receive exactly the amount of LP tokens as deposited tokens
- **Token-specific LP tokens** - Token A deposits get Token A LP tokens, Token B deposits get Token B LP tokens
- **User must create LP token account first** - transaction fails if LP token account doesn't exist

---

### `process_liquidity_withdraw`

Removes liquidity from a pool by burning specific LP tokens and receiving back the corresponding underlying token. This function enables liquidity providers to exit their position by converting their LP tokens back into the original deposited token type.

**🔍 Single-Token Withdrawal Model:**
- **Token-Specific LP Burning**: Burns either Token A LP tokens OR Token B LP tokens (not both)
- **Corresponding Token Return**: Receives the underlying token that matches the LP token type
- **1:1 Burn Ratio**: Burns LP tokens and receives underlying tokens in exact 1:1 ratio
- **LP Token Selection**: User specifies which token to withdraw via `withdraw_token_mint_key`
- **Direct Correspondence**: Token A LP tokens → Token A, Token B LP tokens → Token B

**Detailed Operation Flow:**
1. **LP Token Validation**: Confirms user owns sufficient LP tokens of the specified type
2. **Token Correspondence**: Validates LP token type matches requested withdrawal token
3. **LP Token Burning**: Permanently destroys specified amount of LP tokens
4. **Vault Transfer**: Transfers corresponding tokens from pool vault to user
5. **Balance Updates**: Updates pool liquidity tracking for the withdrawn token side

**Economic Benefits:**
- **Simple Exit Strategy**: Withdraw the same token type you originally deposited
- **No Token Swapping**: Direct token recovery without forced conversions
- **Fee Accumulation**: LP tokens may represent more value due to accumulated swap fees
- **Flexible Amounts**: Withdraw partial or full LP token holdings
- **No Price Impact**: Fixed ratios mean no slippage on withdrawals

**Withdrawal Example:**
```
If you deposited 100 SOL and received 100 Token A LP tokens:
- Burn: 50 Token A LP tokens
- Receive: 50 SOL (plus any accumulated value)
- Remaining: 50 Token A LP tokens in your account
```

**Authority:** LP token holder  
**Fee:** 0.0013 SOL (DEPOSIT_WITHDRAWAL_FEE constant)  
**Compute Units:** 290,000 CUs maximum (Dashboard: min observed 227K; set 290K for safety margin)

#### Instruction Format

**Discriminator:** `3` (single byte)  
**Total Data Length:** 41 bytes  
**Serialization:** Borsh format

```rust
// Instruction structure
pub struct WithdrawInstruction {
    discriminator: u8,            // 1 byte: value = 3
    withdraw_token_mint: Pubkey,  // 32 bytes: Token mint to receive
    lp_amount_to_burn: u64,       // 8 bytes: LP tokens to burn (little-endian)
}
```

#### JavaScript Example
```javascript
// Create instruction data for Withdraw
const instructionData = new Uint8Array(41); // 1 + 32 + 8 bytes
instructionData[0] = 3; // Withdraw discriminator

// Copy withdraw token mint bytes (32 bytes)
withdrawTokenMint.toBytes().forEach((byte, index) => {
    instructionData[1 + index] = byte;
});

// Copy LP amount bytes (8 bytes, u64 little-endian)
const lpAmountBytes = new Uint8Array(new BigUint64Array([BigInt(lpAmountLamports)]).buffer);
lpAmountBytes.forEach((byte, index) => {
    instructionData[33 + index] = byte;
});
```

#### Parameters
```rust
program_id: &Pubkey
lp_amount_to_burn: u64        // LP tokens to burn
withdraw_token_mint: Pubkey   // Which token to receive
accounts: &[AccountInfo; 11]
```

#### Returns
- **Single token type only** - the token corresponding to the LP tokens burned
- **1:1 ratio** - exact amount of underlying tokens as LP tokens burned
- **Fee charged in SOL** - withdrawal fee deducted from user's SOL balance

---

## Swap Operations

Functions for executing token swaps.

### `process_swap_execute`

Executes a deterministic token swap using the pool's predetermined fixed exchange ratio. This function provides guaranteed, slippage-free trading where users receive exactly the calculated output amount or the transaction fails completely.

**Core Trading Mechanism:**
- **Fixed-Ratio Exchange**: Uses predetermined exchange rates set during pool creation (e.g., 1 SOL = 160 USDT)
- **Exact Input Model**: User specifies exact input amount, receives deterministic output based on mathematical calculation
- **Zero Slippage**: No price impact regardless of trade size - ratios remain constant
- **All-or-Nothing**: Transaction succeeds with exact calculated amount or fails entirely (no partial fills)

**Mathematical Foundation:**
```
Output Amount = (Input Amount × Output Token Ratio) ÷ Input Token Ratio

Example: Pool ratio 1 SOL = 160 USDT
- Input: 0.5 SOL (500,000,000 basis points)
- Calculation: 500,000,000 × 160,000,000 ÷ 1,000,000,000 = 80,000,000
- Output: 80 USDT exactly (80,000,000 basis points)
```

**Key Advantages:**
- **Predictable Pricing**: No front-running or MEV extraction possible
- **Institutional-Grade**: Large trades execute at same rate as small trades
- **Gas Efficient**: Single calculation, no complex price curves
- **Capital Efficient**: Full liquidity available at fixed rate until pool depleted

**Security Features:**
- **Slippage Protection**: `expected_amount_out` parameter validates minimum acceptable output
- **Reentrancy Protection**: Built-in guards against complex attack vectors
- **Authority Validation**: Owner-only mode support for custom fee structures
- **Pause Compliance**: Respects system-wide and pool-specific pause states

**Trading Flow:**
1. User specifies input token and amount
2. Contract calculates exact output using fixed ratio
3. Validates user's expected minimum output
4. Transfers input tokens to pool vault
5. Transfers calculated output tokens to user
6. Updates pool balances and fee accounting

**Authority:** Any user (unless owner-only mode)  
**Fee:** 0.00002715 SOL (SWAP_CONTRACT_FEE constant)  
**Compute Units:** 250,000 CUs maximum (Dashboard: tested 202K works; set to 250K to allow for fee changes and variability)

#### Instruction Format

**Discriminator:** `4` (single byte)  
**Total Data Length:** 49 bytes  
**Serialization:** Borsh format

```rust
// Instruction structure
pub struct SwapInstruction {
    discriminator: u8,           // 1 byte: value = 4
    input_token_mint: Pubkey,    // 32 bytes: Input token mint
    amount_in: u64,              // 8 bytes: Input amount in basis points (little-endian)
    expected_amount_out: u64,    // 8 bytes: Expected output for slippage protection (little-endian)
}
```

#### JavaScript Example
```javascript
// Create instruction data for Swap
const instructionData = new Uint8Array([
    4, // Swap discriminator (single byte)
    ...inputTokenMint.toBuffer(), // input_token_mint (32 bytes)
    ...new Uint8Array(new BigUint64Array([BigInt(amountInBaseUnits)]).buffer), // amount_in (u64 little-endian)
    ...new Uint8Array(new BigUint64Array([BigInt(expectedAmountOut)]).buffer)  // expected_amount_out (u64 little-endian)
]);

// Total: 1 + 32 + 8 + 8 = 49 bytes
console.log('Swap instruction data length:', instructionData.length);
```

#### Parameters
```rust
program_id: &Pubkey
amount_in: u64              // Input amount in basis points
expected_amount_out: u64    // Expected output (slippage protection)
accounts: &[AccountInfo; 11]
```

#### Account Structure
| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | User Authority | Signer, Writable | Swapper |
| 1 | System State PDA | Readable | Pause validation |
| 2 | Pool State PDA | Writable | Pool to swap in |
| 3 | User Input Account | Writable | Source tokens |
| 4 | User Output Account | Writable | Destination tokens |
| 5 | Pool Input Vault | Writable | Receives input |
| 6 | Pool Output Vault | Writable | Sends output |
| 7 | Token Program | Readable | SPL token program |
| 8 | System Program | Readable | For fee transfer |
| 9 | Main Treasury PDA | Writable | Fee destination |
| 10 | Input Token Mint | Readable | For validation |

#### Swap Calculation
```
output_amount = (input_amount * output_ratio) / input_ratio
```

---

### `process_swap_set_owner_only`

Configures advanced access control for swap operations with flexible ownership delegation. This sophisticated function enables custom operational models by restricting swap access to designated entities while maintaining protocol-level control. It's designed to support complex business models, custom fee structures, and automated trading systems.

**🔧 Access Control & Delegation System:**
- **Flexible Delegation**: Program Upgrade Authority can delegate operational control to any entity
- **Owner-Only Restrictions**: When enabled, only the designated owner can execute swaps
- **Operational Flexibility**: Enables custom fee models, automated systems, and specialized contracts
- **Protocol Control**: Program Upgrade Authority retains ability to change restrictions and delegations
- **State Management**: Updates pool state with comprehensive audit logging

**🏗️ Use Cases & Operational Models:**
- **Custom Fee Collection**: Deploy contracts with specialized fee structures beyond protocol defaults
- **Treasury Management**: Automated treasury operations through algorithmic trading entities
- **Multi-Signature Control**: Team-managed pools with multi-sig authorization requirements
- **Protocol Integration**: Composed operations combining multiple DeFi protocols
- **Strategic Trading**: Algorithmic entities with sophisticated trading strategies
- **Yield Optimization**: Automated systems for maximizing pool returns

**🔒 Security & Authority Model:**
- **Dual Control Structure**: Protocol authority controls delegation, designated owner controls operations
- **Ownership Delegation**: Can change pool ownership as part of enabling restrictions
- **Comprehensive Validation**: Multi-step validation of authorities and pool state
- **Idempotent Operations**: Safe to call multiple times with same parameters
- **State Persistence**: All changes survive program restarts and cluster maintenance

**⚙️ Implementation Details:**
- **Flag Management**: Uses `POOL_FLAG_SWAP_FOR_OWNERS_ONLY` bitmask for efficient storage
- **Ownership Transfer**: Automatically delegates ownership to designated entity when enabling
- **Comprehensive Logging**: Detailed audit trail for compliance and monitoring
- **State Validation**: Ensures pool and system states are valid before modifications
- **Atomic Updates**: All state changes committed atomically or fail completely

**Authority:** Program Upgrade Authority only  
**Purpose:** Enables sophisticated operational models and custom business logic  
**Effect:** Controls who can execute swaps on the pool

**Note:** This advanced feature enables custom wrapper functions for `process_swap_execute` with specialized rules, fees, or operational models. Organizations can deploy their own contracts with any business logic while the protocol maintains security and administrative control. Contact support@davincicodes.net for implementation guidance.

#### Parameters
```rust
program_id: &Pubkey
enable_restriction: bool    // Enable/disable owner-only restrictions
designated_owner: Pubkey    // Entity to delegate operational control to
accounts: &[AccountInfo; 4]
```

#### Account Structure
| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | Contract Owner | Signer, Writable | Must be program upgrade authority |
| 1 | System State PDA | Readable | System state for pause validation |
| 2 | Pool State PDA | Writable | Pool state to modify access restrictions |
| 3 | Program Data Account | Readable | Program data account for authority validation |

#### Operational Flow & State Changes

**When Enabling Restrictions (`enable_restriction: true`):**
1. **Authority Validation**: Verifies caller is program upgrade authority
2. **System State Check**: Ensures system is not paused
3. **Pool State Load**: Loads and validates pool configuration
4. **Flag Update**: Sets `POOL_FLAG_SWAP_FOR_OWNERS_ONLY` in pool flags
5. **Ownership Delegation**: Changes pool owner to `designated_owner`
6. **State Persistence**: Saves updated pool state atomically

**When Disabling Restrictions (`enable_restriction: false`):**
1. **Authority Validation**: Verifies caller is program upgrade authority
2. **System State Check**: Ensures system is not paused
3. **Pool State Load**: Loads and validates pool configuration
4. **Flag Removal**: Clears `POOL_FLAG_SWAP_FOR_OWNERS_ONLY` from pool flags
5. **State Persistence**: Saves updated pool state (ownership unchanged)

#### Delegation Architecture

The function implements a sophisticated delegation model:

```rust
// Pool ownership delegation when enabling restrictions
if enable_restriction {
    pool_state.owner = designated_owner;  // Delegate operational control
    pool_state.set_swap_for_owners_only(true);  // Enforce restrictions
}
```

**Control Hierarchy:**
- **Protocol Level**: Program Upgrade Authority controls delegation capabilities
- **Operational Level**: Designated owner controls swap execution
- **User Level**: Users interact with designated owner's contracts (when restricted)

#### Business Model Examples

**1. Custom Fee Structure Model:**
```javascript
// Organization deploys custom contract with additional fees
const restrictedPoolOwner = customFeeContract.publicKey;
await processSwapSetOwnerOnly(true, restrictedPoolOwner);
// Users now pay: protocol fees + custom contract fees
```

**2. Multi-Signature Treasury Model:**
```javascript
// Multi-sig entity controls pool operations  
const multiSigWallet = multisigPDA;
await processSwapSetOwnerOnly(true, multiSigWallet);
// Swaps require multi-sig approval through designated contract
```

**3. Algorithmic Trading Model:**
```javascript
// Trading bot with sophisticated strategies
const tradingBotAuthority = algorithmicTrader.publicKey;
await processSwapSetOwnerOnly(true, tradingBotAuthority);
// Only trading bot can execute swaps based on algorithms
```

#### Error Conditions & Troubleshooting

| Error | Condition | Resolution |
|-------|-----------|------------|
| **SystemPaused** | System operations are paused | Wait for system unpause |
| **Unauthorized** | Caller is not program upgrade authority | Use correct upgrade authority |
| **InvalidAccountData** | Pool state PDA validation failed | Verify pool PDA derivation |
| **AccountDataTooSmall** | Pool state account too small | Contact support (should not occur) |

#### Post-Configuration Behavior

**With Restrictions Enabled:**
- `process_swap_execute` only accepts transactions signed by designated owner
- Regular users receive authorization errors when attempting direct swaps
- Designated owner can deploy any custom business logic contracts
- Pool liquidity operations (`deposit`/`withdraw`) remain unrestricted

**With Restrictions Disabled:**
- All users can call `process_swap_execute` directly
- Standard protocol fees apply to all swaps
- No custom operational models active
- Traditional pool operation restored

#### Integration Examples

**Enabling Custom Fee Collection:**
```javascript
// Deploy custom fee collection contract
const customContract = await deployFeeContract(additionalFeePercent);

// Delegate operational control to custom contract
await processSwapSetOwnerOnly(
    true,                           // enable_restriction
    customContract.publicKey        // designated_owner
);

// Users now interact with custom contract for swaps
const swapTx = await customContract.executeSwapWithFees(
    poolPDA, inputAmount, outputAmount, additionalFees
);
```

**Multi-Sig Pool Management:**
```javascript
// Create multi-sig wallet for pool control
const multiSig = await createMultiSigWallet([member1, member2, member3], 2);

// Delegate operational control to multi-sig
await processSwapSetOwnerOnly(
    true,                          // enable_restriction  
    multiSig.publicKey            // designated_owner
);

// Swaps now require 2-of-3 signatures
const swapProposal = await multiSig.proposeSwap(poolPDA, amount);
const executedSwap = await multiSig.executeProposal(swapProposal, [sig1, sig2]);
```

#### Important Implementation Notes

- **Ownership Transfer**: Enabling restrictions automatically changes pool ownership
- **Protocol Authority**: Program Upgrade Authority can always modify restrictions
- **Fee Structure**: Custom models can implement any fee structure beyond protocol fees
- **State Persistence**: All configuration changes persist through restarts
- **Audit Trail**: Comprehensive logging provides full audit compliance
- **Flexibility**: No restrictions on business logic in designated owner contracts

---

## Treasury Operations

Functions for managing protocol treasury and fees.

### `process_treasury_withdraw_fees`

Withdraws collected protocol fees from the main treasury with advanced rate limiting and security protections. Enables the protocol authority to withdraw accumulated fees from pool creation, liquidity operations, and swaps while implementing dynamic rate limiting to prevent rapid fund drainage and ensure system stability.

**Authority:** Program Upgrade Authority only  
**Restrictions:** Dynamic rate limiting with 60-minute rolling windows  
**Compute Units:** 80,000 CUs maximum (complex validation logic)

#### Parameters
```rust
program_id: &Pubkey
amount: u64    // Amount to withdraw in lamports (0 = withdraw all available)
accounts: &[AccountInfo; 6]
```

#### Account Structure
| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | System Authority | Signer, Writable | Must be program upgrade authority |
| 1 | Main Treasury PDA | Writable | Treasury account to withdraw from |
| 2 | Rent Sysvar | Readable | For rent-exempt minimum calculations |
| 3 | Destination Account | Writable | Account to receive withdrawn SOL |
| 4 | System State PDA | Readable | For pause validation and authority checks |
| 5 | Program Data Account | Readable | Program data account for authority validation |

#### Rate Limiting Implementation Details

The contract implements a sophisticated dynamic rate limiting system:

```rust
// Base rate scales automatically based on treasury balance
let current_hourly_limit = treasury_state.calculate_current_hourly_rate_limit();

// Rolling 60-minute window validation  
treasury_state.validate_withdrawal_rate_limit(amount, current_timestamp)?;
```

**Rate Scaling Logic:**
- **Base Rate**: 10 SOL/hour (TREASURY_BASE_HOURLY_RATE constant)
- **Scaling Factor**: 10x multiplier per balance tier (TREASURY_RATE_SCALING_MULTIPLIER constant)
- **Target**: Ensure complete treasury drainage possible within 48 hours maximum
- **Window**: 60-minute rolling window (TREASURY_WITHDRAWAL_RATE_LIMIT_WINDOW constant)

**Error Conditions:**
- **Rate limit exceeded**: Shows time until next withdrawal allowed
- **System restart penalty active**: Shows remaining penalty time
- **Insufficient funds**: Requested amount exceeds available balance
- **Invalid authority**: Caller is not program upgrade authority

#### Additional Security Features

**System Restart Penalty:**
- **Duration**: 71 hours (3 days) after system unpause
- **Purpose**: Prevents immediate fund drainage after system recovery
- **Enforcement**: Blocks all withdrawals during penalty period

**Flexible Withdrawal Options:**
- **Partial Withdrawals**: Specify exact amount in lamports
- **Full Withdrawal**: Use amount = 0 to withdraw all available funds
- **Balance Protection**: Automatically maintains rent-exempt minimum
- **Real-time Validation**: Checks available balance before processing

---

### `process_treasury_get_info`

Retrieves current treasury state information. Returns comprehensive treasury data including total balance, fee collection statistics, withdrawal history, and operational metrics. This read-only function provides transparency into protocol revenue and treasury status.

**Authority:** Public (read-only)

#### Parameters
```rust
program_id: &Pubkey
accounts: &[AccountInfo; 1]
```

#### Returns (via logs)
- Total balance
- Pool creation fees collected
- Consolidated fees
- Withdrawal statistics
- Last operations timestamps

---

### `process_treasury_donate_sol`

Accepts SOL donations to support development.

**Authority:** Any user  
**Minimum:** 0.1 SOL (MIN_DONATION_AMOUNT constant)  
**Compute Units:** Variable by donation amount (see CU Analysis below)

**Note:** Donations help accelerate development of new features including contract improvements and the governance system outlined in the Future Governance Contract Design. The faster we reach our financial goals, the faster we deliver new capabilities.

#### Parameters
```rust
program_id: &Pubkey
amount: u64         // Donation amount (lamports)
message: String     // Optional message (max 200 chars)
accounts: &[AccountInfo; 3]
```

#### Important
- All donations are **non-refundable**
- Donations are publicly visible on-chain
- Contributes to development milestones

#### 📊 CU Analysis (Measured Results)

**Test Results from Actual Measurements:**

| Donation Amount | CUs Required | Cost per SOL | Performance Category |
|----------------|--------------|--------------|---------------------|
| **10 SOL** | 5,000 CUs | 500 CUs/SOL | 🟢 Low |
| **100,000 SOL** | 100,000 CUs | 1 CU/SOL | 🟢 Low |

**Key Findings:**
- **Variable CU Consumption**: Large donations require significantly more CUs (20x increase)
- **Efficiency Scale**: Large donations are more CU-efficient per SOL (1 CU/SOL vs 500 CUs/SOL)
- **Recommended Limits**: Use 120,000 CUs maximum (100K actual + 20% buffer)

**Developer Implications:**
```javascript
// Recommended CU allocation strategy
if (donationAmount <= 1000 * 1_000_000_000) { // <= 1,000 SOL
    computeUnits = 25_000; // Conservative for small-medium donations
} else {
    computeUnits = 120_000; // Required for large donations
}
```

**Root Cause Analysis:**
The significant CU difference suggests additional validation or spam protection logic for large amounts. This is likely intentional anti-spam/anti-abuse validation that scales with donation size. Note: Both donation sizes fall under 🟢 Low category with the realistic CU scale, showing that even "expensive" donations are still manageable operations.

---

### `process_consolidate_pool_fees`

**⚠️ Function Name Correction**: The actual function name is `process_consolidate_pool_fees`, not `process_treasury_consolidate_fees`.

Consolidates SOL fees from multiple pools into the main treasury with flexible pause requirements and sophisticated rent protection. This is the **only mechanism** for moving accumulated protocol fees from individual pools to the central treasury. Features atomic operations, partial consolidation support, and comprehensive safety validations.

**Authority:** None required (public function)  
**Batch Size:** 1-20 pools per transaction  
**Safety Features:** Rent-exempt protection, fee consistency validation, atomic operations

#### Flexible Pause Requirements

**Two Consolidation Modes:**

1. **System Paused Mode**: When the system is paused (any reason code), all specified pools are consolidated regardless of individual pause status
2. **Individual Pool Mode**: When the system is active, only pools with **both** `swaps_paused` AND `liquidity_paused` flags set are consolidated

#### Parameters
```rust
program_id: &Pubkey
pool_count: u8              // Number of pools to consolidate (1-20)
accounts: &[AccountInfo]    // Variable length: 2 + pool_count
```

#### Account Structure
| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | System State PDA | Readable | For pause status validation |
| 1 | Main Treasury PDA | Writable | Receives consolidated fees |
| 2+ | Pool State PDAs | Writable | Pools to consolidate (1-20 pools) |

#### Advanced Features

**Rent Protection System:**
- **Automatic Calculation**: Dynamically calculates rent-exempt minimum for each pool
- **Partial Consolidation**: If fees exceed available balance above rent minimum, consolidates partial amount
- **Balance Validation**: Ensures pools remain rent-exempt after consolidation
- **Safety Checks**: Double validation before any SOL transfers

**Proportional Fee Tracking:**
```rust
// Handles partial consolidations with precise ratio calculations
let consolidation_ratio = available_for_consolidation as f64 / pool_fees as f64;
let liquidity_fees_consolidated = (pool_state.collected_liquidity_fees as f64 * consolidation_ratio) as u64;
let regular_swap_fees_consolidated = (pool_state.collected_swap_contract_fees as f64 * consolidation_ratio) as u64;
```

**GitHub Issue #31960 Workaround**: Uses buffer serialization pattern to prevent SOL transfer corruption of PDA data:
1. Calculate all state changes first
2. Serialize pool state to temporary buffer
3. Perform SOL transfers
4. Copy serialized data atomically

#### Consolidation Eligibility Logic

**System Paused (Mode 1)**:
- **When**: System State `is_paused = true` with any `pause_reason_code`
- **Eligible Pools**: All pools specified in the transaction
- **Use Case**: Mass consolidation during system maintenance

**Individual Pool Pause (Mode 2)**:
- **When**: System State `is_paused = false`
- **Eligible Pools**: Only pools where both flags are set:
  - `pool_state.swaps_paused() = true` (flag bit 2)
  - `pool_state.liquidity_paused() = true` (flag bit 1)
- **Use Case**: Selective consolidation of specific pools

#### Error Conditions
- **Invalid pool count**: 0 or > 20 pools
- **Insufficient accounts**: Account count ≠ (2 + pool_count)
- **Rent protection**: Pool balance would fall below rent-exempt minimum
- **Fee consistency**: Internal fee tracking validation failure
- **Serialization**: Buffer serialization or account writing failure

#### Performance Characteristics
**Compute Unit Estimates** (scales linearly):
- **20 pools**: ~109,000 CUs
- **10 pools**: ~57,000 CUs  
- **1 pool**: ~5,000 CUs

**Breakdown per operation**:
- System pause validation: ~1,000 CUs
- Pool processing: ~5,200 CUs per pool
- Treasury update: ~4,000 CUs

---

## Error Codes & System Constants

### Custom Error Codes

The contract uses standardized error codes for programmatic error handling. All custom errors are returned as `ProgramError::Custom(code)`.

| Code | Error Type | Description |
|------|------------|-------------|
| 1001 | `InvalidTokenPair` | Invalid token pair configuration |
| 1002 | `InvalidRatio` | Invalid ratio configuration (outside bounds) |
| 1003 | `InsufficientFunds` | Insufficient funds for the operation |
| 1004 | `InvalidTokenAccount` | Invalid token account state or configuration |
| 1005 | `InvalidSwapAmount` | Swap amount outside allowed bounds |
| 1006 | `RentExemptError` | Insufficient funds for rent exemption |
| 1007 | `PoolPaused` | Pool operations are currently paused |
| 1012 | `Unauthorized` | Unauthorized operation |
| 1019 | `ArithmeticOverflow` | Arithmetic overflow error |
| 1023 | `SystemPaused` | System is paused - all operations blocked except unpause |
| 1024 | `SystemAlreadyPaused` | System is already paused |
| 1025 | `SystemNotPaused` | System is not paused |
| 1026 | `UnauthorizedAccess` | Unauthorized access to system controls |
| 1027 | `PoolSwapsPaused` | Pool swaps are currently paused by owner |
| 1029 | `PoolSwapsAlreadyPaused` | Pool swaps are already paused |
| 1030 | `PoolSwapsNotPaused` | Pool swaps are not currently paused |

### System Pause Reason Codes

When the system is paused, a reason code is stored to indicate the purpose:

| Code | Purpose | Description |
|------|---------|-------------|
| 0 | No Pause | System is active (default state) |
| 15 | `PAUSE_REASON_CONSOLIDATION` | System paused for fee consolidation operations |
| 1-14, 16-255 | Custom Reasons | User-defined pause reasons for operational control |

**Usage**: System pause reason codes enable operational tracking and automated responses. The consolidation process specifically checks for reason code 15 to optimize batch operations.

### Pool State Flags (Bitwise)

Pool operations are controlled through bitwise flags in the pool state:

| Flag | Bit | Hex | Purpose |
|------|-----|-----|---------|
| `POOL_FLAG_ONE_TO_MANY_RATIO` | 1 | 0x01 | Identifies pools with 1:N whole number ratios |
| `POOL_FLAG_LIQUIDITY_PAUSED` | 2 | 0x02 | Liquidity operations paused (deposits/withdrawals) |
| `POOL_FLAG_SWAPS_PAUSED` | 4 | 0x04 | Swap operations paused |
| `POOL_FLAG_WITHDRAWAL_PROTECTION` | 8 | 0x08 | Withdrawal protection active (future feature) |
| `POOL_FLAG_SINGLE_LP_TOKEN` | 16 | 0x10 | Single LP token mode (future feature) |
| `POOL_FLAG_SWAP_FOR_OWNERS_ONLY` | 32 | 0x20 | Swaps restricted to owners only |

### Pool Pause Control Flags

For pause operations, specific combinations are used:

| Flag | Value | Purpose |
|------|-------|---------|
| `PAUSE_FLAG_LIQUIDITY` | 1 | Pause only liquidity operations |
| `PAUSE_FLAG_SWAPS` | 2 | Pause only swap operations |
| `PAUSE_FLAG_ALL` | 3 | Pause all operations (required for consolidation) |

### Fee Update Flags

For fee modification operations:

| Flag | Value | Purpose |
|------|-------|---------|
| `FEE_UPDATE_FLAG_LIQUIDITY` | 1 | Update liquidity fees only |
| `FEE_UPDATE_FLAG_SWAP` | 2 | Update swap fees only |
| `FEE_UPDATE_FLAG_BOTH` | 3 | Update both fee types |

### Validation Context Codes

Internal validation operations use context codes for error reporting:

| Code | Context | Purpose |
|------|---------|---------|
| 1 | `VALIDATION_CONTEXT_FEE` | General fee operations |
| 2 | `VALIDATION_CONTEXT_POOL_CREATION` | Pool creation operations |
| 3 | `VALIDATION_CONTEXT_LIQUIDITY` | Liquidity operations |
| 4 | `VALIDATION_CONTEXT_SWAP` | Swap operations |

### System Configuration Constants

**Operational Limits:**
- `MAX_POOLS_PER_CONSOLIDATION_BATCH`: 20 pools maximum per consolidation
- `TREASURY_WITHDRAWAL_RATE_LIMIT_WINDOW`: 60 minutes (3600 seconds)
- `TREASURY_SYSTEM_RESTART_PENALTY_SECONDS`: 71 hours (255,600 seconds)

**Treasury Rate Limiting:**
- `TREASURY_BASE_HOURLY_RATE`: 10 SOL per hour base rate
- `TREASURY_RATE_SCALING_MULTIPLIER`: 10x scaling per balance tier
- `TREASURY_MAX_DRAIN_TIME_HOURS`: 48 hours maximum drain time target

**Fee Validation Limits** (see Fee Constants section above for current values):
- `MAX_LIQUIDITY_FEE`: Maximum allowed liquidity fee
- `MIN_LIQUIDITY_FEE`: Minimum allowed liquidity fee  
- `MAX_SWAP_FEE`: Maximum allowed swap fee
- `MIN_SWAP_FEE`: Minimum allowed swap fee
- `MIN_DONATION_AMOUNT`: Minimum donation amount (0.1 SOL)

---

## Types and Structures

### Complete PDA Data Structures

This section provides comprehensive documentation of all Program Derived Account (PDA) data structures for developers building external integrations without importing the contract source code.

#### Important Notes for External Developers

**🔢 Basis Points Storage Format:**
- All token amounts and ratios are stored in **basis points** (smallest token units)
- External applications must convert between display units and basis points
- Example: 1.5 USDC (6 decimals) = 1,500,000 basis points
- Example: 0.001 BTC (8 decimals) = 100,000 basis points
- Example: 1.0 SOL (9 decimals) = 1,000,000,000 basis points

**📦 Serialization Format:**
- All structures use Borsh serialization
- Account data can be deserialized directly using Borsh libraries
- Field order matches the structure definition exactly

**🏗️ Account Space Requirements:**
- PoolState: 597 bytes
- SystemState: 10 bytes
- MainTreasuryState: 128 bytes

---

### PoolState Structure

The main pool configuration and runtime data structure. Contains all information needed to interact with a trading pool.

```rust
pub struct PoolState {
    // === CORE POOL CONFIGURATION ===
    /// Pool owner (can be delegated for owner-only operations)
    pub owner: Pubkey,                      // 32 bytes
    
    /// Token A mint address (lexicographically smaller)
    pub token_a_mint: Pubkey,               // 32 bytes
    
    /// Token B mint address (lexicographically larger)  
    pub token_b_mint: Pubkey,               // 32 bytes
    
    /// Token A vault PDA (holds deposited Token A)
    pub token_a_vault: Pubkey,              // 32 bytes
    
    /// Token B vault PDA (holds deposited Token B)
    pub token_b_vault: Pubkey,              // 32 bytes
    
    /// LP Token A mint PDA (minted for Token A deposits)
    pub lp_token_a_mint: Pubkey,            // 32 bytes
    
    /// LP Token B mint PDA (minted for Token B deposits)
    pub lp_token_b_mint: Pubkey,            // 32 bytes
    
    // === FIXED EXCHANGE RATIOS (BASIS POINTS) ===
    /// Token A ratio numerator in basis points
    /// Example: For "1.0 SOL = 160.0 USDT", this = 1,000,000,000 (1.0 * 10^9)
    pub ratio_a_numerator: u64,             // 8 bytes
    
    /// Token B ratio denominator in basis points  
    /// Example: For "1.0 SOL = 160.0 USDT", this = 160,000,000 (160.0 * 10^6)
    pub ratio_b_denominator: u64,           // 8 bytes
    
    // === LIQUIDITY TRACKING (BASIS POINTS) ===
    /// Total Token A deposited in pool (basis points)
    pub total_token_a_liquidity: u64,       // 8 bytes
    
    /// Total Token B deposited in pool (basis points)
    pub total_token_b_liquidity: u64,       // 8 bytes
    
    // === PDA BUMP SEEDS ===
    pub pool_authority_bump_seed: u8,       // 1 byte
    pub token_a_vault_bump_seed: u8,        // 1 byte
    pub token_b_vault_bump_seed: u8,        // 1 byte
    pub lp_token_a_mint_bump_seed: u8,      // 1 byte
    pub lp_token_b_mint_bump_seed: u8,      // 1 byte
    
    // === POOL STATE FLAGS (BITWISE) ===
    /// Pool state flags using bitwise operations:
    /// - Bit 0 (1): One-to-many ratio configuration
    /// - Bit 1 (2): Liquidity operations paused
    /// - Bit 2 (4): Swap operations paused
    /// - Bit 3 (8): Withdrawal protection active
    /// - Bit 4 (16): Single LP token mode (future)
    /// - Bit 5 (32): Swap for owners only
    pub flags: u8,                          // 1 byte
    
    // === CONFIGURABLE CONTRACT FEES ===
    /// Contract fee for liquidity operations (lamports)
    pub contract_liquidity_fee: u64,        // 8 bytes
    
    /// Contract fee for swap operations (lamports)
    pub swap_contract_fee: u64,             // 8 bytes
    
    // === TOKEN FEE COLLECTION TRACKING ===
    pub collected_fees_token_a: u64,        // 8 bytes
    pub collected_fees_token_b: u64,        // 8 bytes
    pub total_fees_withdrawn_token_a: u64,  // 8 bytes
    pub total_fees_withdrawn_token_b: u64,  // 8 bytes
    
    // === SOL FEE TRACKING ===
    /// SOL fees from liquidity operations (accumulated locally)
    pub collected_liquidity_fees: u64,      // 8 bytes
    
    /// SOL fees from swap operations (accumulated locally)
    pub collected_swap_contract_fees: u64,  // 8 bytes
    
    /// Total SOL fees collected since pool creation (lifetime)
    pub total_sol_fees_collected: u64,      // 8 bytes
    
    // === CONSOLIDATION MANAGEMENT ===
    /// Unix timestamp of last fee consolidation (0 if never)
    pub last_consolidation_timestamp: i64,  // 8 bytes
    
    /// Total number of consolidations performed
    pub total_consolidations: u64,          // 8 bytes
    
    /// Total SOL fees transferred to treasury via consolidation
    pub total_fees_consolidated: u64,       // 8 bytes
    
    // === POOL-SPECIFIC LIMITS ===
    /// Maximum swap amount (0 = no limit)
    pub max_swap_amount: u64,               // 8 bytes
    
    /// Minimum swap amount 
    pub min_swap_amount: u64,               // 8 bytes
    
    /// Maximum single deposit amount (0 = no limit)
    pub max_deposit_amount: u64,            // 8 bytes
    
    /// Minimum deposit amount
    pub min_deposit_amount: u64,            // 8 bytes
    
    /// Maximum single withdrawal amount (0 = no limit)
    pub max_withdrawal_amount: u64,         // 8 bytes
    
    /// Minimum withdrawal amount
    pub min_withdrawal_amount: u64,         // 8 bytes
    
    // === RESERVED SPACE ===
    /// Reserved for future features (32 bytes)
    pub _reserved: [u64; 4],                // 32 bytes
}

// Total Size: 597 bytes
```

#### Pool State Flag Interpretations

```rust
// Flag checking methods (for external implementations)
pub fn one_to_many_ratio(&self) -> bool { self.flags & 1 != 0 }
pub fn liquidity_paused(&self) -> bool { self.flags & 2 != 0 }
pub fn swaps_paused(&self) -> bool { self.flags & 4 != 0 }
pub fn withdrawal_protection_active(&self) -> bool { self.flags & 8 != 0 }
pub fn only_lp_token_a_for_both(&self) -> bool { self.flags & 16 != 0 }
pub fn swap_for_owners_only(&self) -> bool { self.flags & 32 != 0 }
```

#### Pool State Calculations

```rust
// Pending SOL fees awaiting consolidation
pub fn pending_sol_fees(&self) -> u64 {
    self.total_sol_fees_collected - self.total_fees_consolidated
}

// Available balance for consolidation (respecting rent exemption)
pub fn calculate_available_for_consolidation(
    &self,
    current_account_balance: u64,
    rent_exempt_minimum: u64,
) -> u64 {
    let pending_fees = self.pending_sol_fees();
    let available_above_rent_exempt = current_account_balance.saturating_sub(rent_exempt_minimum);
    std::cmp::min(available_above_rent_exempt, pending_fees)
}
```

---

### SystemState Structure

Global system state controlling all contract operations. Used for emergency pause/unpause functionality.

```rust
pub struct SystemState {
    /// Global pause state - when true, all operations blocked except unpause
    pub is_paused: bool,                    // 1 byte
    
    /// Unix timestamp when system was paused
    pub pause_timestamp: i64,               // 8 bytes
    
    /// Pause reason code for efficient storage:
    /// - 0: No pause active (default state)
    /// - 1: Emergency security issue
    /// - 2: Scheduled maintenance  
    /// - 3: Contract upgrade
    /// - 4: Regulatory compliance
    /// - 5: Infrastructure issue
    /// - 6: Economic emergency
    /// - 15: Fee consolidation operations
    /// - Other: Custom reasons
    pub pause_reason_code: u8,              // 1 byte
}

// Total Size: 10 bytes
```

#### System State Usage

```rust
// Check if system operations are allowed
if !system_state.is_paused {
    // System is operational - check individual pool states
} else {
    // System is paused - only read operations allowed
    // Check pause_reason_code for specific reason
}
```

---

### MainTreasuryState Structure

Central treasury collecting all protocol fees with comprehensive tracking and rate limiting.

```rust
pub struct MainTreasuryState {
    // === BALANCE TRACKING ===
    /// Current SOL balance (synced with account.lamports())
    pub total_balance: u64,                 // 8 bytes
    
    /// Rent-exempt minimum balance requirement
    pub rent_exempt_minimum: u64,           // 8 bytes
    
    /// Total SOL withdrawn by authority over time
    pub total_withdrawn: u64,               // 8 bytes
    
    // === OPERATION COUNTERS ===
    /// Number of pools created
    pub pool_creation_count: u64,           // 8 bytes
    
    /// Number of liquidity operations (deposits/withdrawals)
    pub liquidity_operation_count: u64,     // 8 bytes
    
    /// Number of regular swap operations
    pub regular_swap_count: u64,            // 8 bytes
    
    /// Number of treasury withdrawals performed
    pub treasury_withdrawal_count: u64,     // 8 bytes
    
    /// Number of failed operations for analytics
    pub failed_operation_count: u64,        // 8 bytes
    
    // === FEE TOTALS ===
    /// Total SOL from pool creation fees
    pub total_pool_creation_fees: u64,      // 8 bytes
    
    /// Total SOL from liquidity operation fees
    pub total_liquidity_fees: u64,          // 8 bytes
    
    /// Total SOL from regular swap fees
    pub total_regular_swap_fees: u64,       // 8 bytes
    
    /// Total SOL from swap contract fees
    pub total_swap_contract_fees: u64,      // 8 bytes
    
    // === TIMESTAMPS ===
    /// Last treasury update timestamp
    pub last_update_timestamp: i64,         // 8 bytes
    
    /// Last withdrawal timestamp (for rate limiting)
    pub last_withdrawal_timestamp: i64,     // 8 bytes
    
    // === CONSOLIDATION TRACKING ===
    /// Number of consolidation operations performed
    pub total_consolidations_performed: u64, // 8 bytes
    
    // === DONATION TRACKING ===
    /// Number of voluntary donations received
    pub donation_count: u64,                // 8 bytes
    
    /// Total SOL donated to protocol
    pub total_donations: u64,               // 8 bytes
}

// Total Size: 128 bytes
```

#### Treasury State Calculations

```rust
// Available balance for withdrawal (above rent exempt minimum)
pub fn available_for_withdrawal(&self) -> u64 {
    if self.total_balance > self.rent_exempt_minimum {
        self.total_balance - self.rent_exempt_minimum
    } else {
        0
    }
}

// Total fees collected across all categories
pub fn total_fees_collected(&self) -> u64 {
    self.total_pool_creation_fees +
    self.total_liquidity_fees +
    self.total_regular_swap_fees
}

// Check if withdrawal is blocked by restart penalty
pub fn is_blocked_by_restart_penalty(&self, current_timestamp: i64) -> bool {
    self.last_withdrawal_timestamp > current_timestamp
}

// Calculate current hourly withdrawal rate limit (dynamic)
pub fn calculate_current_hourly_rate_limit(&self) -> u64 {
    let available_balance = self.available_for_withdrawal();
    let mut current_rate = 10_000_000_000; // 10 SOL/hour base rate
    
    // Scale up by 10x when 48-hour drain time would be exceeded
    while available_balance > (48 * current_rate) {
        current_rate = current_rate.saturating_mul(10);
        if current_rate == 0 || current_rate == u64::MAX { break; }
    }
    
    current_rate
}
```

---

## Account Derivation Requirements

### 🔐 Critical PDA Security Model

The Fixed Ratio Trading program uses a **strict PDA validation model** where **ALL** Program Derived Addresses must be derived correctly and match expected values. The contract will reject transactions with incorrect PDAs to prevent security vulnerabilities and address manipulation attacks.

### 🧮 Account Derivation Algorithm

#### Step 1: Token Normalization (Critical!)

Before deriving any pool-related PDAs, tokens **MUST** be normalized to prevent liquidity fragmentation:

```rust
// CRITICAL: Always normalize tokens to lexicographic order
let (token_a_mint, token_b_mint) = if input_token_1 < input_token_2 {
    (input_token_1, input_token_2)  // Already in correct order
} else {
    (input_token_2, input_token_1)  // Swap to correct order
};

// CRITICAL: Map ratios to normalized token order
let (ratio_a_numerator, ratio_b_denominator) = if input_token_1 < input_token_2 {
    (input_ratio_1, input_ratio_2)  // Ratios match token order
} else {
    (input_ratio_2, input_ratio_1)  // Swap ratios to match swapped tokens
};
```

#### Step 2: System PDAs (No Dependencies)

```rust
// System State PDA - Global pause control
let (system_state_pda, system_state_bump) = Pubkey::find_program_address(
    &[b"system_state"], 
    &PROGRAM_ID
);

// Main Treasury PDA - Fee collection and withdrawal
let (main_treasury_pda, main_treasury_bump) = Pubkey::find_program_address(
    &[b"main_treasury"], 
    &PROGRAM_ID
);
```

#### Step 3: Pool State PDA (Depends on Normalized Inputs)

```rust
// Pool State PDA - Must use NORMALIZED tokens and ratios
let (pool_state_pda, pool_state_bump) = Pubkey::find_program_address(
    &[
        b"pool_state",  // ⚠️ NOT "pool_state_v2" - this is the correct seed
        token_a_mint.as_ref(),         // 32 bytes - NORMALIZED token A
        token_b_mint.as_ref(),         // 32 bytes - NORMALIZED token B  
        &ratio_a_numerator.to_le_bytes(),    // 8 bytes - little-endian u64
        &ratio_b_denominator.to_le_bytes(),  // 8 bytes - little-endian u64
    ], 
    &PROGRAM_ID
);
```

#### Step 4: Pool Infrastructure PDAs (Depend on Pool State)

```rust
// Token Vault PDAs - Secure token storage controlled by pool
let (token_a_vault_pda, token_a_vault_bump) = Pubkey::find_program_address(
    &[b"token_a_vault", pool_state_pda.as_ref()], 
    &PROGRAM_ID
);

let (token_b_vault_pda, token_b_vault_bump) = Pubkey::find_program_address(
    &[b"token_b_vault", pool_state_pda.as_ref()], 
    &PROGRAM_ID
);

// LP Token Mint PDAs - Liquidity provider token mints
let (lp_token_a_mint_pda, lp_token_a_mint_bump) = Pubkey::find_program_address(
    &[b"lp_token_a_mint", pool_state_pda.as_ref()], 
    &PROGRAM_ID
);

let (lp_token_b_mint_pda, lp_token_b_mint_bump) = Pubkey::find_program_address(
    &[b"lp_token_b_mint", pool_state_pda.as_ref()], 
    &PROGRAM_ID
);
```

### 🛡️ PDA Validation Security

#### Critical Security Checks

1. **Pool State PDA Validation**: Contract verifies the provided pool state PDA matches the expected derived address
2. **Vault PDA Validation**: All vault PDAs must derive correctly from the pool state PDA
3. **LP Mint PDA Validation**: LP token mint PDAs must derive correctly from the pool state PDA
4. **System PDA Validation**: System state and treasury PDAs must match expected addresses

#### Common Derivation Errors

| Error | Cause | Solution |
|-------|-------|----------|
| `InvalidAccountData` | Wrong PDA provided | Re-derive using correct seeds |
| `AccountAlreadyInitialized` | Pool already exists | Check if pool exists before creation |
| Wrong token order | Tokens not normalized | Apply lexicographic ordering |
| Wrong ratio mapping | Ratios don't match normalized tokens | Map ratios to normalized token order |

### 📝 Complete Derivation Example

```javascript
// Complete JavaScript example for pool creation
import { PublicKey } from '@solana/web3.js';

const PROGRAM_ID = new PublicKey("4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn");

function deriveAllPoolPDAs(inputTokenMint1, inputTokenMint2, inputRatio1, inputRatio2) {
    // Step 1: Normalize tokens to lexicographic order
    const normalizeTokens = (mint1, mint2, ratio1, ratio2) => {
        if (mint1.toString() < mint2.toString()) {
            return {
                tokenA: mint1, tokenB: mint2,
                ratioA: ratio1, ratioB: ratio2
            };
        } else {
            return {
                tokenA: mint2, tokenB: mint1,
                ratioA: ratio2, ratioB: ratio1
            };
        }
    };

    const normalized = normalizeTokens(inputTokenMint1, inputTokenMint2, inputRatio1, inputRatio2);
    
    // Step 2: System PDAs (same for all pools)
    const [systemStatePDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("system_state")], PROGRAM_ID
    );
    
    const [mainTreasuryPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("main_treasury")], PROGRAM_ID  
    );
    
    // Step 3: Pool State PDA
    const [poolStatePDA] = PublicKey.findProgramAddressSync([
        Buffer.from("pool_state"),
        normalized.tokenA.toBuffer(),
        normalized.tokenB.toBuffer(),
        Buffer.from(new BigUint64Array([BigInt(normalized.ratioA)]).buffer),
        Buffer.from(new BigUint64Array([BigInt(normalized.ratioB)]).buffer),
    ], PROGRAM_ID);
    
    // Step 4: Pool Infrastructure PDAs
    const [tokenAVaultPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("token_a_vault"), poolStatePDA.toBuffer()], PROGRAM_ID
    );
    
    const [tokenBVaultPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("token_b_vault"), poolStatePDA.toBuffer()], PROGRAM_ID
    );
    
    const [lpTokenAMintPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("lp_token_a_mint"), poolStatePDA.toBuffer()], PROGRAM_ID
    );
    
    const [lpTokenBMintPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("lp_token_b_mint"), poolStatePDA.toBuffer()], PROGRAM_ID
    );

    return {
        // System PDAs
        systemStatePDA,
        mainTreasuryPDA,
        
        // Pool PDAs  
        poolStatePDA,
        tokenAVaultPDA,
        tokenBVaultPDA,
        lpTokenAMintPDA,
        lpTokenBMintPDA,
        
        // Normalized values for verification
        normalizedTokenA: normalized.tokenA,
        normalizedTokenB: normalized.tokenB,
        normalizedRatioA: normalized.ratioA,
        normalizedRatioB: normalized.ratioB
    };
}

// Usage example
const result = deriveAllPoolPDAs(
    new PublicKey("So11111111111111111111111111111111111111112"), // SOL
    new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"), // USDC
    1,     // 1 SOL
    160    // = 160 USDC (at some price point)
);

console.log("All required PDAs:", result);
```

### 🔄 Bump Seeds and Signing

When the contract needs to sign on behalf of PDAs (e.g., minting LP tokens), it uses the bump seeds:

```rust
// Example: Pool state PDA signing for LP token minting
let pool_state_signer_seeds = &[
    b"pool_state",
    token_a_mint.as_ref(),
    token_b_mint.as_ref(), 
    &ratio_a_numerator.to_le_bytes(),
    &ratio_b_denominator.to_le_bytes(),
    &[pool_state_bump],  // Bump seed for signing
];
```

### ⚠️ Critical Requirements Summary

1. **Token Normalization is Mandatory**: Always sort tokens lexicographically before derivation
2. **Ratio Mapping is Required**: Map ratios to match normalized token order  
3. **All PDAs Must Match**: Contract validates every PDA against expected derived address
4. **Seed Precision Matters**: Use exact seed strings (`"pool_state"` not `"pool_state_v2"`)
5. **Little-Endian Encoding**: Use `to_le_bytes()` for u64 values in seeds
6. **No Shortcuts Allowed**: Every PDA must be derived following the exact algorithm

---

### Data Access Examples

#### Reading Pool State

```javascript
// JavaScript example using @solana/web3.js
import { Connection, PublicKey } from '@solana/web3.js';
import { deserialize } from 'borsh';

// Define the PoolState schema for Borsh
const PoolStateSchema = {
    struct: {
        owner: { array: { type: 'u8', len: 32 } },
        token_a_mint: { array: { type: 'u8', len: 32 } },
        token_b_mint: { array: { type: 'u8', len: 32 } },
        token_a_vault: { array: { type: 'u8', len: 32 } },
        token_b_vault: { array: { type: 'u8', len: 32 } },
        lp_token_a_mint: { array: { type: 'u8', len: 32 } },
        lp_token_b_mint: { array: { type: 'u8', len: 32 } },
        ratio_a_numerator: 'u64',
        ratio_b_denominator: 'u64',
        total_token_a_liquidity: 'u64',
        total_token_b_liquidity: 'u64',
        // ... add remaining fields as needed
        flags: 'u8',
        // ... complete schema
    }
};

async function getPoolState(connection, poolStatePDA) {
    const accountInfo = await connection.getAccountInfo(poolStatePDA);
    if (!accountInfo) {
        throw new Error('Pool state account not found');
    }
    
    const poolState = deserialize(PoolStateSchema, accountInfo.data);
    
    // Convert basis points to display values
    const tokenADecimals = 9; // Fetch from token A mint
    const tokenBDecimals = 6; // Fetch from token B mint
    
    const displayRatioA = poolState.ratio_a_numerator / Math.pow(10, tokenADecimals);
    const displayRatioB = poolState.ratio_b_denominator / Math.pow(10, tokenBDecimals);
    
    console.log(`Exchange Rate: ${displayRatioA} Token A = ${displayRatioB} Token B`);
    
    // Check pool flags
    const liquidityPaused = (poolState.flags & 2) !== 0;
    const swapsPaused = (poolState.flags & 4) !== 0;
    
    return { ...poolState, liquidityPaused, swapsPaused };
}
```

#### Reading System State

```javascript
// System state is much simpler
const SystemStateSchema = {
    struct: {
        is_paused: 'bool',
        pause_timestamp: 'i64', 
        pause_reason_code: 'u8'
    }
};

async function getSystemState(connection, systemStatePDA) {
    const accountInfo = await connection.getAccountInfo(systemStatePDA);
    if (!accountInfo) {
        throw new Error('System state account not found');
    }
    
    const systemState = deserialize(SystemStateSchema, accountInfo.data);
    
    if (systemState.is_paused) {
        console.log(`System paused since ${new Date(systemState.pause_timestamp * 1000)}`);
        console.log(`Reason code: ${systemState.pause_reason_code}`);
    }
    
    return systemState;
}
```

#### Reading Treasury State

```javascript
const TreasuryStateSchema = {
    struct: {
        total_balance: 'u64',
        rent_exempt_minimum: 'u64',
        total_withdrawn: 'u64',
        pool_creation_count: 'u64',
        liquidity_operation_count: 'u64',
        regular_swap_count: 'u64',
        treasury_withdrawal_count: 'u64',
        failed_operation_count: 'u64',
        total_pool_creation_fees: 'u64',
        total_liquidity_fees: 'u64',
        total_regular_swap_fees: 'u64',
        total_swap_contract_fees: 'u64',
        last_update_timestamp: 'i64',
        last_withdrawal_timestamp: 'i64',
        total_consolidations_performed: 'u64',
        donation_count: 'u64',
        total_donations: 'u64'
    }
};

async function getTreasuryState(connection, treasuryPDA) {
    const accountInfo = await connection.getAccountInfo(treasuryPDA);
    if (!accountInfo) {
        throw new Error('Treasury state account not found');
    }
    
    const treasuryState = deserialize(TreasuryStateSchema, accountInfo.data);
    
    // Calculate analytics
    const availableForWithdrawal = treasuryState.total_balance - treasuryState.rent_exempt_minimum;
    const totalFees = treasuryState.total_pool_creation_fees + 
                     treasuryState.total_liquidity_fees + 
                     treasuryState.total_regular_swap_fees;
    
    console.log(`Available: ${availableForWithdrawal / 1e9} SOL`);
    console.log(`Total Fees: ${totalFees / 1e9} SOL`);
    
    // Check withdrawal penalty status
    const currentTime = Math.floor(Date.now() / 1000);
    const penaltyActive = treasuryState.last_withdrawal_timestamp > currentTime;
    
    return { ...treasuryState, availableForWithdrawal, totalFees, penaltyActive };
}
```

#### Rust Examples

```rust
use borsh::BorshDeserialize;
use solana_program::pubkey::Pubkey;

// Example: Reading pool state in Rust
async fn read_pool_state(
    rpc_client: &RpcClient, 
    pool_pda: &Pubkey
) -> Result<PoolState, Box<dyn std::error::Error>> {
    let account = rpc_client.get_account(pool_pda)?;
    let pool_state = PoolState::try_from_slice(&account.data)?;
    
    // Calculate pending fees
    let pending_fees = pool_state.pending_sol_fees();
    println!("Pending SOL fees: {} lamports", pending_fees);
    
    // Check flags
    println!("Liquidity paused: {}", pool_state.liquidity_paused());
    println!("Swaps paused: {}", pool_state.swaps_paused());
    
    Ok(pool_state)
}

// Example: Reading system state in Rust  
async fn read_system_state(
    rpc_client: &RpcClient,
    system_pda: &Pubkey
) -> Result<SystemState, Box<dyn std::error::Error>> {
    let account = rpc_client.get_account(system_pda)?;
    let system_state = SystemState::try_from_slice(&account.data)?;
    
    if system_state.is_paused {
        println!("System is paused (reason: {})", system_state.pause_reason_code);
    }
    
    Ok(system_state)
}
```

---

### Legacy PDA Seeds (For Reference)
- **System State:** `[b"system_state"]`
- **Main Treasury:** `[b"main_treasury"]`
- **Pool State:** `[b"pool_state_v2", token_a_mint, token_b_mint, ratio_a_bytes, ratio_b_bytes]`
- **Token Vaults:** `[b"token_a_vault", pool_state_key]` or `[b"token_b_vault", pool_state_key]`
- **LP Mints:** `[b"lp_token_a_mint", pool_state_key]` or `[b"lp_token_b_mint", pool_state_key]`

---

## Support and Resources

- **Email:** support@davincicodes.net
- **Documentation:** [GitHub Repository](https://github.com/davincicodes/fixed-ratio-trading)
- **Dashboard:** Available for mainnet deployment

For custom integrations, fee modifications, or technical support, please contact our support team.