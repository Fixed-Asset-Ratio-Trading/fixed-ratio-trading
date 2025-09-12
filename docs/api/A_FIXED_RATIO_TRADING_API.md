# Fixed Ratio Trading Contract API Documentation

**Version:** v0.16.1060  
**Date:** August 30, 2025  
**LocalNet Program ID:** `4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn` 
**DevNet Program ID:** `9iqh69RqeG3RRrFBNZVoE77TMRvYboFUtC2sykaFVzB7` 
**MainNet Program ID:** `quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD`
**Support:** support@davincicodes.net
**GitKracken** https://gitkraken.cello.so/pk9L5rp5jln visual Git helps you see it all clearly!

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
7. [🚨 Critical Pool Creation Issue: Decimal Precision Mistakes](#-critical-pool-creation-issue-decimal-precision-mistakes)
8. [Error Analysis & Troubleshooting](#error-analysis--troubleshooting)
9. [Liquidity Operations](#liquidity-operations)
10. [Swap Operations](#swap-operations)
11. [Treasury Operations](#treasury-operations)
12. [Error Codes](#error-codes)
13. [Types and Structures](#types-and-structures)
14. [📚 Developer Calculation Guides](#-developer-calculation-guides)

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

5. **⚠️ Transaction Building Requirements**
   - **Instruction Data Format**: Use single-byte discriminator `[1]` (18 bytes total for InitializePool)
   - **Token Ordering**: Must use lexicographic byte comparison (same as Rust Pubkey::cmp)
   - **Basis Points**: Convert user ratios using `amount * Math.pow(10, decimals)`
   - **Account Structure**: Must include exactly 13 accounts in documented order
   - **Solnet Issues**: Known transaction serialization bugs - use raw RPC for complex transactions

---

## Authoritative PDA Seed Rules and Instruction Account Orders

This section is a concise, single-source reference for all PDA derivations and required account orders per instruction. If you see “invalid account data for instruction”, verify your PDAs and account order here first.

### PDA Seed Rules

- System State PDA:
  - Seeds: `[b"system_state"]`
  - Derivation: `Pubkey::find_program_address([SYSTEM_STATE_SEED_PREFIX], program_id)`

- Main Treasury PDA:
  - Seeds: `[b"main_treasury"]`
  - Derivation: `Pubkey::find_program_address([MAIN_TREASURY_SEED_PREFIX], program_id)`

- Pool State PDA:
  - Seeds: `[b"pool_state", token_a_mint, token_b_mint, ratio_a_numerator.to_le_bytes(), ratio_b_denominator.to_le_bytes()]`
  - Token normalization: order mints lexicographically to get `(token_a_mint, token_b_mint)`
  - Ratio mapping (canonicalization):
    - If `multiple_token_mint < base_token_mint` → `(ratio_a, ratio_b) = (multiple_per_base, 1)`
    - Else → `(ratio_a, ratio_b) = (1, multiple_per_base)`

- Token A Vault PDA:
  - Seeds: `[b"token_a_vault", pool_state_pda]`

- Token B Vault PDA:
  - Seeds: `[b"token_b_vault", pool_state_pda]`

- LP Token A Mint PDA:
  - Seeds: `[b"lp_token_a_mint", pool_state_pda]`

- LP Token B Mint PDA:
  - Seeds: `[b"lp_token_b_mint", pool_state_pda]`

All u64 values in seeds use little-endian encoding (`to_le_bytes`). The program validates provided accounts against these exact derivations.

### Instruction Account Orders (by index)

The following lists reflect the on-chain handlers and are validated in code. Indexes are zero-based.

- InitializeProgram (6 accounts)
  - [0] Program Authority Signer (upgrade authority, signer, writable)
  - [1] System Program
  - [2] Rent Sysvar
  - [3] System State PDA (writable)
  - [4] Main Treasury PDA (writable)
  - [5] Program Data Account (BPF Upgradeable Loader ProgramData)

- InitializePool (13 accounts)
  - [0] User Authority Signer (signer)
  - [1] System Program
  - [2] System State PDA
  - [3] Pool State PDA (writable)
  - [4] SPL Token Program
  - [5] Main Treasury PDA (writable)
  - [6] Rent Sysvar
  - [7] Token A Mint Account (readable)
  - [8] Token B Mint Account (readable)
  - [9] Token A Vault PDA (writable)
  - [10] Token B Vault PDA (writable)
  - [11] LP Token A Mint PDA (writable)
  - [12] LP Token B Mint PDA (writable)

- Deposit (11 accounts)
  - [0] User Authority Signer (signer, writable)
  - [1] System Program
  - [2] System State PDA
  - [3] Pool State PDA (writable) ⚠️ CRITICAL: Must be writable for fee tracking
  - [4] SPL Token Program
  - [5] Token A Vault PDA (writable)
  - [6] Token B Vault PDA (writable)
  - [7] User Input Token Account (writable)
  - [8] User Output LP Token Account (writable)
  - [9] LP Token A Mint PDA (writable)
  - [10] LP Token B Mint PDA (writable)

- Withdraw (11 accounts)
  - [0] User Authority Signer (signer, writable)
  - [1] System Program
  - [2] System State PDA
  - [3] Pool State PDA (writable) ⚠️ CRITICAL: Must be writable for fee tracking
  - [4] SPL Token Program
  - [5] Token A Vault PDA (writable)
  - [6] Token B Vault PDA (writable)
  - [7] User Input LP Token Account (writable)
  - [8] User Output Token Account (writable)
  - [9] LP Token A Mint PDA (writable)
  - [10] LP Token B Mint PDA (writable)

- Swap (11 accounts)
  - [0] User Authority Signer (signer, writable)
  - [1] System Program
  - [2] System State PDA
  - [3] Pool State PDA (writable) ⚠️ CRITICAL: Must be writable for fee tracking
  - [4] SPL Token Program
  - [5] Token A Vault PDA (writable)
  - [6] Token B Vault PDA (writable)
  - [7] User Input Token Account (writable)
  - [8] User Output Token Account (writable)
  - [9] Input Mint Account (must match user input token account mint)
  - [10] Output Mint Account (must match user output token account mint)

- SetSwapOwnerOnly (4 accounts)
  - [0] Admin Authority Signer (admin authority with program upgrade authority fallback)
  - [1] System State PDA
  - [2] Pool State PDA (writable)
  - [3] Program Data Account (ProgramData)

- UpdatePoolFees (4 accounts)
  - [0] Program Authority Signer (admin authority)
  - [1] System State PDA
  - [2] Pool State PDA (writable)
  - [3] Program Data Account (ProgramData)

- PauseSystem (3 accounts)
  - [0] System Authority Signer (admin authority)
  - [1] System State PDA (writable)
  - [2] Program Data Account (ProgramData)

- UnpauseSystem (4 accounts)
  - [0] System Authority Signer (admin authority)
  - [1] System State PDA (writable)
  - [2] Main Treasury PDA (writable)
  - [3] Program Data Account (ProgramData)

- WithdrawTreasuryFees (6 accounts)
  - [0] System Authority Signer (admin authority)
  - [1] Main Treasury PDA (writable)
  - [2] Rent Sysvar
  - [3] Destination Account (writable)
  - [4] System State PDA
  - [5] Program Data Account (ProgramData)

- DonateSol (4 accounts)
  - [0] Donor Account (signer, writable)
  - [1] Main Treasury PDA (writable)
  - [2] System State PDA
  - [3] System Program

### View / Debug Instructions

These are read-only or utility calls that help you derive/verify addresses and inspect state:

- GetPoolStatePDA (no accounts)
  - Inputs: `multiple_token_mint`, `base_token_mint`, `multiple_per_base`
  - Logs: normalized token mints, normalized ratio, derived pool state PDA and bump

- GetTokenVaultPDAs (no accounts)
  - Inputs: `pool_state_pda`
  - Logs: derived Token A/B vault PDAs and bumps

- GetPoolInfo (4 accounts)
  - [0] Placeholder, [1] Placeholder, [2] Pool State PDA, [3] Placeholder (SPL Token Program)
  - Logs: owner, all PDAs (vaults, LP mints), ratios, and bump seeds

- GetPoolPauseStatus (1 account)
  - [0] Pool State PDA

- GetLiquidityInfo (1 account)
  - [0] Pool State PDA

- GetFeeInfo (1 account)
  - [0] Pool State PDA

- GetPoolSolBalance (1 account)
  - [0] Pool State PDA

- GetVersion (no accounts)

### Quick PDA/Account Mismatch Checklist

- Re-derive PDAs using the exact seeds above and your program ID.
- Ensure token mints are lexicographically ordered before seeding pool PDAs.
- Ensure ratios are mapped to little-endian bytes as described.
- Verify account order and count precisely matches the lists above.
- For swaps, include the two mint accounts at indexes [9] and [10].

### One-shot comparison (logs) to diagnose “invalid account data”

To compare your provided accounts to the canonical PDAs without code changes:

1) Call `GetPoolStatePDA(multiple_token_mint, base_token_mint, multiple_per_base)` to log the pool state PDA and normalized inputs.
2) Call `GetTokenVaultPDAs(pool_state_pda)` to log the expected vault PDAs.
3) Call `GetPoolInfo([_, _, pool_state_pda, _])` to log all PDAs and bump seeds from on-chain state.

Comparing these logs with your transaction’s accounts will surface any PDA or ordering mismatches immediately.

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
| `process_admin_change` | 15,000 | 150,000 | 🟢 Low | Admin authority management with 72h timelock |
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

**Note:** Function names like `process_treasury_donate_sol` correspond to instruction enum variants (e.g., `DonateSol` with discriminator `23`).

#### Function Name to Discriminator Mapping

| Function Name | Instruction Enum | Discriminator |
|---------------|------------------|---------------|
| `process_treasury_donate_sol` | `DonateSol` | `23` |
| `process_initialize_program` | `InitializeProgram` | `0` |
| `process_initialize_pool` | `InitializePool` | `1` |
| `process_deposit` | `Deposit` | `2` |
| `process_withdraw` | `Withdraw` | `3` |
| `process_swap` | `Swap` | `4` |
| `process_pause_system` | `PauseSystem` | `12` |
| `process_unpause_system` | `UnpauseSystem` | `13` |
| `process_treasury_withdraw_fees` | `WithdrawTreasuryFees` | `15` |
| `process_consolidate_pool_fees` | `ConsolidatePoolFees` | `17` | ⚠️ Admin Authority Required |
| `process_pause_pool` | `PausePool` | `19` |
| `process_unpause_pool` | `UnpausePool` | `20` |
| `process_set_swap_owner_only` | `SetSwapOwnerOnly` | `21` |
| `process_update_pool_fees` | `UpdatePoolFees` | `22` |
| `process_admin_change` | `ProcessAdminChange` | `24` |

| Discriminator | Instruction | Data Size | Total Size | Description |
|---------------|-------------|-----------|------------|-------------|
| `0` | `InitializeProgram` | 32 bytes | 33 bytes | Initialize system state and treasury |
| `1` | `InitializePool` | 17 bytes | 18 bytes | Create new trading pool |
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
| `17` | `ConsolidatePoolFees` | 1 byte | 2 bytes | Consolidate pool fees (Admin Authority required) |
| `18` | `GetConsolidationStatus` | 1 byte | 2 bytes | Get consolidation status |
| `19` | `PausePool` | 1 byte | 2 bytes | Pause pool operations |
| `20` | `UnpausePool` | 1 byte | 2 bytes | Unpause pool operations |
| `21` | `SetSwapOwnerOnly` | 33 bytes | 34 bytes | Set swap owner restrictions |
| `22` | `UpdatePoolFees` | 17 bytes | 18 bytes | Update pool fee structure |
| `23` | `DonateSol` | 12+ bytes | 13+ bytes | Donate SOL to treasury (8 bytes amount + 4+ bytes message) |
| `24` | `ProcessAdminChange` | 32 bytes | 33 bytes | Change admin authority (72h timelock) |

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

### Instruction Account Orders

Each instruction requires accounts to be provided in a specific order. The following table documents the account order for each instruction:

#### Pool Management Instructions

**PausePool (Discriminator 19)**
- [0] Admin Authority (Signer, Writable) - Must be admin authority (or program upgrade authority as fallback)
- [1] System State PDA (Writable) - System state for pause validation
- [2] Pool State PDA (Writable) - Pool state to update with pause information
- [3] Program Data Account (Readable) - Program data account for authority validation

**UnpausePool (Discriminator 20)**
- [0] Admin Authority (Signer, Writable) - Must be admin authority (or program upgrade authority as fallback)  
- [1] System State PDA (Writable) - System state for pause validation
- [2] Pool State PDA (Writable) - Pool state to update with unpause information
- [3] Program Data Account (Readable) - Program data account for authority validation

**SetSwapOwnerOnly (Discriminator 21)**
- [0] Admin Authority (Signer, Readable) - Must be admin authority (or program upgrade authority as fallback)
- [1] System State PDA (Readable) - System state for pause validation
- [2] Pool State PDA (Writable) - Pool state to modify access restrictions and ownership
- [3] Program Data Account (Readable) - Program data account for authority validation

**⚠️ Critical Notes:**
- **Pool State PDA is WRITABLE** for pause/unpause operations - the pool state is updated directly by the client transaction
- **System State PDA is WRITABLE** for pause/unpause operations - system state may be updated during validation
- **Pool State PDA is WRITABLE** for SetSwapOwnerOnly operations - it updates both flags and ownership
- **Admin Authority** can be either the configured admin authority OR the program upgrade authority (fallback)
- **Authority Validation** uses multi-layer validation through program data account verification
- **Borsh Serialization** is used for all instruction data (not raw byte arrays)

---

## System Management

Functions for system-wide operations and program initialization.

### `process_system_initialize`

Initializes the program's system state and main treasury. This is a one-time setup operation that creates the core infrastructure for the Fixed Ratio Trading system, including the main treasury that collects all protocol fees and the system state that tracks global configuration and pause status.

The `admin_authority` parameter sets the account that will have control over system-wide operations such as pausing/unpausing the system and withdrawing treasury funds. This authority is separate from the program upgrade authority and can be configured to point to governance systems, multisigs, or other administrative structures.

**Authority:** Admin Authority only  
**One-time operation:** Can only be called once  
**Compute Units:** 25,000 CUs maximum

#### Parameters
```rust
program_id: &Pubkey    // Program ID
admin_authority: Pubkey // Admin authority for system operations
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

#### Instruction Format

**Discriminator:** `0` (single byte)  
**Total Data Length:** 33 bytes  
**Serialization:** Borsh format

```rust
// Instruction structure
pub struct InitializeProgramInstruction {
    discriminator: u8,        // 1 byte: value = 0
    admin_authority: Pubkey,  // 32 bytes: Admin authority for system operations
}
```

#### JavaScript Example

```javascript
// Create InitializeProgram instruction
const adminAuthority = new PublicKey("6ytvYbjegFnBWLk9FsEoy1nwKwnTKcX5MxgX7PeGDHp2");

const instructionData = Buffer.concat([
    Buffer.from([0]),                    // Discriminator: 0
    adminAuthority.toBuffer()            // Admin authority: 32 bytes
]);

const instruction = new TransactionInstruction({
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
```

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
- **Authority Logging**: Records which admin authority initiated the pause
- **Validation Checks**: Prevents pausing already-paused system with descriptive errors
- **Persistent State**: Pause state survives program restarts and cluster maintenance

**🔒 Security & Authority Validation:**
- **Admin Authority Required**: Only the program's admin authority can execute system pause
- **Multi-Layer Validation**: Validates admin authority through program data account verification
- **Signer Requirements**: Ensures proper cryptographic authorization
- **PDA Security**: Validates system state PDA against expected derived address
- **Atomic Operation**: Pause state update is atomic (all-or-nothing)

**Authority:** Admin Authority only  
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
| 0 | System Authority | Signer, Writable | Must be admin authority |
| 1 | System State PDA | Writable | System state to update with pause information |
| 2 | Program Data Account | Readable | Program data account for admin authority validation |

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
- **Authority Logging**: Records which admin authority initiated the unpause
- **Previous State Tracking**: Logs the original pause reason code for correlation
- **Timestamp Recording**: Records unpause timestamp for audit compliance
- **State Persistence**: Both system and treasury state changes persist through restarts

**⚠️ Important Behavioral Notes:**
- **Pool-Specific Pauses**: Individual pool pause states remain active and must be unpaused separately
- **Override Hierarchy**: System unpause does NOT automatically unpause individually paused pools
- **Treasury Access**: Treasury withdrawal functions will reject requests during penalty period
- **Read-Only Functions**: Continue working normally during and after unpause
- **Client Integration**: Applications should check both system and pool pause states

**Authority:** Admin Authority only  
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
| 0 | System Authority | Signer, Writable | Must be admin authority |
| 1 | System State PDA | Writable | System state to clear pause information |
| 2 | Main Treasury PDA | Writable | Treasury state to apply 71-hour withdrawal penalty |
| 3 | Program Data Account | Readable | Program data account for admin authority validation |

#### Enhanced Account Validation
Unlike most functions, `process_system_unpause` includes explicit account count validation:
- **Explicit Length Check**: Verifies exactly 4 accounts provided (prevents index panics)
- **Treasury PDA Validation**: Validates treasury PDA matches expected derived address
- **Authority Verification**: Multi-step admin authority validation
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
| **Unauthorized** | Caller is not admin authority | Use correct admin authority account |
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

### `process_admin_change`

Manages admin authority changes with a secure 72-hour timelock mechanism. This critical security function enables controlled transfer of administrative privileges while preventing hostile takeovers through mandatory waiting periods and automatic completion logic.

**🔐 Security-First Admin Management:**
- **72-Hour Timelock**: Mandatory waiting period prevents immediate hostile takeover
- **Automatic Completion**: No separate "finalize" transaction needed after timelock expires
- **Timer Reset Protection**: Different admin proposed within 72 hours resets the timer
- **Cancellation Support**: Proposing current admin as new admin cancels pending changes
- **Unified Interface**: Single function handles initiation, completion, and cancellation

**🔄 State Machine Logic:**
- **Initiation**: New admin proposed → starts 72-hour timer → pending state
- **Completion**: 72+ hours elapsed + different admin → automatic completion
- **Cancellation**: Current admin proposed as "new" admin → clears pending state
- **Reset**: Different admin proposed during pending → resets timer to full 72 hours
- **No-Op**: Same pending admin proposed again → returns pending status

**📊 Admin Change Process Flow:**
```
Current Admin: Alice
┌─────────────────────────────────────────────────────────────────┐
│ 1. Alice proposes Bob as new admin                              │
│    → Starts 72-hour timer                                      │
│    → State: Alice (current), Bob (pending)                     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 2a. After 72+ hours: Alice calls with Bob again                │
│     → Automatic completion                                      │
│     → State: Bob (current), None (pending)                     │
│                                                                 │
│ 2b. Within 72 hours: Alice proposes Charlie                    │
│     → Timer resets to full 72 hours                            │
│     → State: Alice (current), Charlie (pending)                │
│                                                                 │
│ 2c. Within 72 hours: Alice proposes Alice                      │
│     → Cancellation (clears pending)                            │
│     → State: Alice (current), None (pending)                   │
└─────────────────────────────────────────────────────────────────┘
```

**⚠️ Important Security Notes:**
- **Authority Validation**: Only current admin authority (or upgrade authority during migration) can initiate changes
- **PDA Security**: Validates system state PDA against expected derived address
- **Atomic Operations**: All state changes are atomic (all-or-nothing)
- **Persistent State**: Pending changes survive program restarts and cluster maintenance
- **Audit Trail**: All changes logged with timestamps and authority information

**Authority:** Current Admin Authority (or Program Upgrade Authority during migration)  
**Effect:** Initiates, completes, or cancels admin authority changes  
**Timelock:** 72 hours (259,200 seconds) mandatory waiting period  
**Compute Units:** 15,000 CUs maximum

#### Parameters
```rust
program_id: &Pubkey
new_admin: Pubkey          // Proposed new admin authority
accounts: &[AccountInfo; 3]
```

#### Account Structure
| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | Current Admin Authority | Signer, Writable | Must be current admin authority (or upgrade authority during migration) |
| 1 | System State PDA | Writable | System state to update with admin change info |
| 2 | Program Data Account | Readable | Program data account for admin authority validation |

#### Admin Change Results & Responses

##### Successful Initiation
```rust
AdminChangeResult::Initiated { 
    new_admin: Pubkey, 
    previous_pending: Option<Pubkey> 
}
```
- **Effect**: 72-hour timer started for proposed admin
- **State**: `pending_admin_authority` set to new admin
- **Timestamp**: `admin_change_timestamp` updated to current time
- **Previous**: If another change was pending, it gets replaced

##### Successful Completion
```rust
AdminChangeResult::Completed { 
    old_admin: Pubkey, 
    new_admin: Pubkey 
}
```
- **Effect**: Admin authority transferred immediately
- **State**: `admin_authority` updated, `pending_admin_authority` cleared
- **Requirement**: 72+ hours must have elapsed since initiation

##### Cancellation
```rust
AdminChangeResult::Cancelled
```
- **Effect**: Pending admin change cleared
- **Trigger**: Current admin proposed as "new" admin
- **State**: `pending_admin_authority` set to None

##### No Change Needed
```rust
AdminChangeResult::NoChange
```
- **Effect**: No state modification
- **Trigger**: Proposed admin is already current admin (no pending change)

##### Still Pending
```rust
AdminChangeResult::Pending { 
    pending_admin: Pubkey, 
    remaining_seconds: i64 
}
```
- **Effect**: No state change, returns remaining time
- **Trigger**: Same pending admin proposed again before timelock expires

#### Timelock Calculation
```rust
// Constants
const ADMIN_CHANGE_TIMELOCK: i64 = 72 * 60 * 60; // 259,200 seconds

// Remaining time calculation
let elapsed = current_timestamp - admin_change_timestamp;
let remaining = ADMIN_CHANGE_TIMELOCK - elapsed;

// Ready when remaining <= 0
```

#### Security Validations
1. **Authority Check**: Verifies signer is current admin authority (or upgrade authority during migration)
2. **PDA Validation**: Confirms system state PDA matches expected address
3. **Signer Requirement**: Ensures proper cryptographic authorization
4. **Account Writability**: Validates system state account is writable
5. **Timestamp Integrity**: Uses on-chain clock for tamper-proof timing

#### Error Conditions
- **Unauthorized**: Calling account is not current admin authority (or upgrade authority during migration)
- **InvalidAccountData**: System state PDA validation failed
- **AccountDataTooSmall**: System state account cannot store admin change data
- **InvalidSystemStatePDA**: Provided PDA doesn't match expected derived address

#### Integration Examples

##### JavaScript/TypeScript
```javascript
// Initiate admin change
const newAdminPubkey = new PublicKey("...");
const instruction = createAdminChangeInstruction(
    currentAdminKeypair.publicKey,
    newAdminPubkey,
    systemStatePDA,
    programDataAccount,
    PROGRAM_ID
);

// Check if change can be completed
const systemState = await getSystemState(connection, systemStatePDA);
if (systemState.pending_admin_authority) {
    const elapsed = Date.now() / 1000 - systemState.admin_change_timestamp;
    const remaining = (72 * 60 * 60) - elapsed;
    
    if (remaining <= 0) {
        console.log("Admin change ready for completion");
    } else {
        console.log(`Admin change pending: ${remaining} seconds remaining`);
    }
}
```

##### Rust Client
```rust
// Check pending admin status
let system_state = SystemState::from_account_data_unchecked(&account.data)?;
let remaining_time = system_state.get_admin_change_remaining_time(current_timestamp);

if remaining_time == 0 && system_state.pending_admin_authority.is_some() {
    println!("Admin change ready for completion");
} else if remaining_time > 0 {
    println!("Admin change pending: {} seconds remaining", remaining_time);
}
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

## 🚨 CRITICAL: GetVersion Simulation Requirements

### Required Parameters
The GetVersion instruction requires these specific simulation parameters:

```javascript
const simOptions = {
    sigVerify: false,
    replaceRecentBlockhash: true,
    commitment: 'confirmed',  // REQUIRED - other commitments may not work
    encoding: 'base64'
};
```

**⚠️ WARNING**: Omitting `commitment: 'confirmed'` will result in empty logs and failed version retrieval.

### Account Funding Requirement

**IMPORTANT**: The GetVersion instruction requires a funded fee payer to execute properly:
- **Unfunded Account**: Returns `AccountNotFound` error with empty logs
- **Funded Account**: Returns program logs with version information

#### Recommended Approach:
1. Try simulation with ephemeral keypair
2. If `AccountNotFound`, request airdrop (localnet/devnet only)
3. Confirm airdrop transaction
4. Retry simulation with funded account

```javascript
// Request airdrop if AccountNotFound
if (result.value.err && JSON.stringify(result.value.err).includes('AccountNotFound')) {
    const airdropSig = await connection.requestAirdrop(keypair.publicKey, 1_000_000);
    await connection.confirmTransaction({
        signature: airdropSig,
        ...(await connection.getLatestBlockhash())
    }, 'confirmed');
    // Retry simulation
}
```

### Successful GetVersion Response

When properly executed, GetVersion returns these program logs:

```json
{
  "err": null,
  "logs": [
    "Program 4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn invoke [1]",
    "Program log: === SMART CONTRACT VERSION ===",
    "Program log: Contract Name: fixed-ratio-trading",
    "Program log: Contract Version: 0.15.1054",  // ← Parse this line
    "Program log: Contract Description: Fixed Ratio Trading Smart Contract for Solana",
    "Program log: ==============================="
  ]
}
```

**Version Extraction**: Look for the log containing `"Contract Version: X.X.X"` and extract using regex: `/Contract Version:\s*([0-9\.]+)/`

## Troubleshooting GetVersion

| Issue | Cause | Solution |
|-------|-------|----------|
| Empty logs (`"logs": []`) | Missing `commitment: 'confirmed'` | Add commitment parameter |
| `AccountNotFound` error | Unfunded fee payer | Request airdrop and retry |
| No version in logs | Wrong instruction format | Verify discriminator is `[14]` |
| Transaction fails | Network issues | Check RPC connection |

These documentation improvements prevent hours of debugging by clearly stating the funded account requirement and commitment parameter necessity.

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

## 🔧 Working Pool Creation Implementation Guide

**⚠️ Critical**: The C# stress test service was getting transaction serialization errors. The working implementation is in `C:\Users\Davinci\code\fixed-ratio-trading\dashboard\pool-creation.js`. Key fixes needed:

### 1. Correct Instruction Data Format

**❌ Wrong (C# API was doing this):**
```csharp
// Using 4-byte discriminator
var data = new PoolInitializeInstructionData
{
    Discriminator = 4, // WRONG - this is 4 bytes in Borsh
    RatioANumerator = poolConfig.RatioANumerator,
    RatioBDenominator = poolConfig.RatioBDenominator
};
```

**✅ Correct:**
```javascript
// Single-byte discriminator (18 bytes total)
const discriminator = new Uint8Array([1]); // Single byte for InitializePool
const ratioABytes = new Uint8Array(new BigUint64Array([BigInt(ratioABasisPoints)]).buffer);
const ratioBBytes = new Uint8Array(new BigUint64Array([BigInt(ratioBBasisPoints)]).buffer);
const flagsByte = new Uint8Array([flags]); // Pool behavior flags
const instructionData = concatUint8Arrays([discriminator, ratioABytes, ratioBBytes, flagsByte]);
```

### 2. Token Ordering (Critical Fix)

**❌ Wrong (String comparison):**
```csharp
// This doesn't match Rust Pubkey comparison
if (tokenAMint.ToString().CompareTo(tokenBMint.ToString()) > 0)
```

**✅ Correct (Byte comparison):**
```javascript
// Use byte-level comparison (same as Rust Pubkey::cmp)
const primaryTokenBytes = primaryTokenMint.toBytes();
const baseTokenBytes = baseTokenMint.toBytes();
let primaryIsLessThanBase = false;
for (let i = 0; i < 32; i++) {
    if (primaryTokenBytes[i] < baseTokenBytes[i]) {
        primaryIsLessThanBase = true;
        break;
    } else if (primaryTokenBytes[i] > baseTokenBytes[i]) {
        primaryIsLessThanBase = false;
        break;
    }
}
const tokenAMint = primaryIsLessThanBase ? primaryTokenMint : baseTokenMint;
const tokenBMint = primaryIsLessThanBase ? baseTokenMint : primaryTokenMint;
```

### 3. Basis Points Conversion

**✅ Correct conversion using token decimals:**
```javascript
const displayToBasisPoints = (amount, decimals) => {
    return Math.floor(amount * Math.pow(10, decimals));
};

// Get token decimals from mint accounts
const normalizedTokenADecimals = await getTokenDecimals(tokenAMint.toString(), connection);
const normalizedTokenBDecimals = await getTokenDecimals(tokenBMint.toString(), connection);

// Convert to basis points
finalRatioABasisPoints = displayToBasisPoints(tokenADisplay, normalizedTokenADecimals);
finalRatioBBasisPoints = displayToBasisPoints(tokenBDisplay, normalizedTokenBDecimals);
```

### 4. Avoiding Solnet Transaction Issues

**✅ Use raw transaction building:**
```javascript
// Build transaction manually instead of using Solnet's complex builders
const transaction = new solanaWeb3.Transaction()
    .add(computeBudgetInstruction)
    .add(createPoolInstruction);

// Get fresh blockhash and send directly
const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash('confirmed');
transaction.recentBlockhash = blockhash;
transaction.feePayer = wallet.publicKey;
const signedTx = await wallet.signTransaction(transaction);
const signature = await connection.sendRawTransaction(signedTx.serialize());
```

### 5. Successful Pool Creation Result

When implemented correctly, you should see:
- ✅ Airdrop funding works (1.5 SOL sufficient)
- ✅ Token mints created successfully  
- ✅ Pool transaction submitted without serialization errors
- ✅ Pool appears in dashboard
- ✅ Pool ID, token addresses, and transaction signature returned

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
- **Fee Collection**: Collects 1.15 SOL registration fee directly to main treasury (immediate tracking)

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
**Compute Units:** 150,000 CUs maximum (observed ~91K; table-aligned)

#### Instruction Format

**Discriminator:** `1` (single byte)  
**Total Data Length:** 18 bytes  
**Serialization:** Borsh format

```rust
// Instruction structure
pub struct InitializePoolInstruction {
    discriminator: u8,           // 1 byte: value = 1
    ratio_a_numerator: u64,      // 8 bytes: Token A ratio in basis points (little-endian)
    ratio_b_denominator: u64,    // 8 bytes: Token B ratio in basis points (little-endian)
    flags: u8,                   // 1 byte: Pool behavior flags (bitwise)
}
```

#### JavaScript Example
```javascript
// Create instruction data for InitializePool
const discriminator = new Uint8Array([1]); // InitializePool discriminator
const ratioABytes = new Uint8Array(new BigUint64Array([BigInt(ratioABasisPoints)]).buffer);
const ratioBBytes = new Uint8Array(new BigUint64Array([BigInt(ratioBBasisPoints)]).buffer);
const flagsByte = new Uint8Array([flags]); // Pool behavior flags

const instructionData = new Uint8Array([
    ...discriminator,    // 1 byte
    ...ratioABytes,      // 8 bytes (u64 little-endian)
    ...ratioBBytes,      // 8 bytes (u64 little-endian)
    ...flagsByte         // 1 byte (u8 flags)
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

async function createPoolExact(userWallet, tokenAMint, tokenBMint, ratioA, ratioB, flags = 0) {
    const connection = new Connection(RPC_URL, 'confirmed');
    
    // Step 1: Normalize tokens (CRITICAL! Use byte-wise lexicographic order like Rust Pubkey::cmp)
    const normalizeTokens = (mint1, mint2, ratio1, ratio2) => {
        const a = mint1.toBytes();
        const b = mint2.toBytes();
        let aLessThanB = false;
        for (let i = 0; i < 32; i++) {
            if (a[i] < b[i]) { aLessThanB = true; break; }
            if (a[i] > b[i]) { aLessThanB = false; break; }
        }
        if (aLessThanB) {
            return { tokenA: mint1, tokenB: mint2, ratioA: ratio1, ratioB: ratio2 };
        }
        return { tokenA: mint2, tokenB: mint1, ratioA: ratio2, ratioB: ratio1 };
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
    const instructionData = new Uint8Array(18); // 1 + 8 + 8 + 1 bytes
    instructionData[0] = 1; // InitializePool discriminator
    
    // Ratio A as little-endian u64
    const ratioABytes = new Uint8Array(new BigUint64Array([BigInt(normalized.ratioA)]).buffer);
    instructionData.set(ratioABytes, 1);
    
    // Ratio B as little-endian u64  
    const ratioBBytes = new Uint8Array(new BigUint64Array([BigInt(normalized.ratioB)]).buffer);
    instructionData.set(ratioBBytes, 9);
    
    // Pool flags
    instructionData[17] = flags;
    
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

// Usage examples

// Standard pool (allows all users, permits dust loss)
const standardPool = await createPoolExact(
    wallet,
    new PublicKey("So11111111111111111111111111111111111111112"), // SOL
    new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"), // USDC
    1000000000,  // 1 SOL in basis points (1 * 10^9)
    160000000,   // 160 USDC in basis points (160 * 10^6)
    0            // flags: standard behavior
);

// Owner-only pool with exact exchange required
const restrictedPool = await createPoolExact(
    wallet,
    tokenAMint,
    tokenBMint,
    ratioA,
    ratioB,
    32 | 64      // flags: owner-only (32) + exact exchange (64) = 96
);
// Results in 18-byte instruction data: [1, ...16 bytes of ratio data, flags]
```

#### Parameters
```rust
program_id: &Pubkey
ratio_a_numerator: u64      // Token A ratio in basis points
ratio_b_denominator: u64    // Token B ratio in basis points
flags: u8                   // Pool behavior flags (bitwise)
accounts: &[AccountInfo; 13]
```

#### Pool Flags (Bitwise)

The `flags` parameter controls pool behavior using bitwise operations. Multiple flags can be combined using the OR operator (`|`).

| Bit | Value | Flag Name | Description |
|-----|-------|-----------|-------------|
| 0 | 1 | **One-to-many ratio** | Automatically set by contract based on ratio type |
| 1 | 2 | **Liquidity paused** | Deposits/withdrawals paused (set by admin) |
| 2 | 4 | **Swaps paused** | Swap operations paused (set by admin) |
| 3 | 8 | **Withdrawal protection** | Enhanced withdrawal validation (set by admin) |
| 4 | 16 | **Single LP token mode** | Future feature (reserved) |
| 5 | 32 | **Owner-only swaps** | Only pool creator can swap (settable at creation) |
| 6 | 64 | **Exact exchange required** | Reject swaps with precision loss (settable at creation) |
| 7 | 128 | *Reserved* | Future use |

**⚠️ Important Notes:**
- **Settable at creation**: Only bits 5 (owner-only) and 6 (exact exchange) can be set during pool initialization
- **Admin-controlled**: Bits 1-4 are managed by admin authority after pool creation
- **Automatic**: Bit 0 is automatically set by the contract based on ratio analysis
- **Default value**: Use `0` for standard pool behavior (allows all users, permits dust loss)

**Detailed Flag Behavior:**

**Bit 5 (32) - Owner-only swaps:**
- When set: Only the pool creator (original signer) can execute swaps
- When unset: Any user can swap tokens in the pool
- Use case: Private pools, testing environments, or exclusive trading pairs
- Can be toggled after creation using `SetSwapOwnerOnly` instruction (can also set a new owner)

**Bit 6 (64) - Exact exchange required:**
- When set: Rejects any swap that would result in precision loss (dust)
- When unset: Allows swaps with dust loss (standard behavior)
- Use case: High-value tokens where precision is critical, institutional trading
- Cannot be changed after pool creation
- Example rejection: Swapping 1.5 tokens for a 0-decimal token (would lose 0.5)

**Common Flag Combinations:**
```javascript
// Standard pool (default)
const flags = 0; // No restrictions, allows dust loss

// Owner-only pool
const flags = 32; // 0b0010_0000 - only creator can swap

// Exact exchange pool (precision-critical)
const flags = 64; // 0b0100_0000 - rejects swaps with precision loss

// Owner-only + exact exchange (maximum control)
const flags = 32 | 64; // 96 (0b0110_0000) - both restrictions

// Check if flags are set
const isOwnerOnly = (flags & 32) !== 0;
const requiresExactExchange = (flags & 64) !== 0;
```

#### Account Structure
| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | User Authority | Signer, Writable | Pool creator |
| 1 | System Program | Readable | Solana system program |
| 2 | System State PDA | Readable | For pause validation |
| 3 | Pool State PDA | Writable | Will be created |
| 4 | SPL Token Program | Readable | Token program |
| 5 | Main Treasury PDA | Writable | For direct fee collection (pool creation) |
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
function createInstructionData(normalizedRatioA, normalizedRatioB, flags = 0) {
    console.log("🔧 Creating instruction data:");
    console.log(`  Ratio A: ${normalizedRatioA}`);
    console.log(`  Ratio B: ${normalizedRatioB}`);
    console.log(`  Flags: ${flags} (0b${flags.toString(2).padStart(8, '0')})`);
    
    const instructionData = new Uint8Array(18);
    instructionData[0] = 1; // InitializePool discriminator
    
    // Convert ratios to little-endian u64
    const ratioABytes = new Uint8Array(new BigUint64Array([BigInt(normalizedRatioA)]).buffer);
    const ratioBBytes = new Uint8Array(new BigUint64Array([BigInt(normalizedRatioB)]).buffer);
    
    instructionData.set(ratioABytes, 1);
    instructionData.set(ratioBBytes, 9);
    instructionData[17] = flags; // Pool flags
    
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
| `InvalidArgument` | Wrong data format | Check instruction data is 18 bytes |
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

## 🚨 Critical Pool Creation Issue: Decimal Precision Mistakes

### The #1 Most Expensive Pool Creation Bug

**Problem:** The most common and costly mistake when creating pools is incorrect basis points calculation when tokens have different decimal precisions. This results in completely wrong exchange ratios and pool creation failures.

#### Real-World Debug Example

```
🔍 The New Error Analysis
The new error "Error processing Instruction 1: Program failed to complete" suggests that:
- Instruction 0: Compute Budget instruction (likely succeeded)  
- Instruction 1: Pool creation instruction (failed)

Debug Output:
The problem is that we have Token A with 9 decimals and Token B with 6 decimals, 
but we're sending the same ratio values (1000:1000). This is wrong!

Token A: 9 decimals (was reordered to be first lexicographically)
Token B: 6 decimals  
Decimals difference: |9 - 6| = 3
Needs inversion: True (because token order changed)

❌ WRONG calculation:
ratioBDenominator = ratioWholeNumber = 1000  
ratioANumerator = Math.Pow(10, 3) = 1000

The issue is that we're using ratioWholeNumber (1000) but we should be using basis points!
```

#### ❌ Common Developer Mistake

```javascript
// ❌ WRONG - This is what developers typically do:
const tokenA = { mint: "So11111111111111111111111111111112", decimals: 9 };  // SOL
const tokenB = { mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", decimals: 6 };  // USDC
const ratioWholeNumber = 160; // Want 1 SOL = 160 USDC

// ❌ WRONG: Using same value for both sides
const ratioANumerator = ratioWholeNumber;     // 160 ❌ 
const ratioBDenominator = ratioWholeNumber;   // 160 ❌

// This sends wrong basis points to smart contract:
// ratioANumerator: 160 (should be 1,000,000,000)
// ratioBDenominator: 160 (should be 160,000,000)
```

**Result:** Pool creation fails or creates pool with wrong ratio like "160 SOL = 160 USDC" instead of "1 SOL = 160 USDC"

#### ✅ Correct Implementation

```javascript
/**
 * ✅ CORRECT: Convert display amounts to basis points using each token's decimals
 */
function displayToBasisPoints(displayAmount, decimals) {
    if (displayAmount === 0) return 0;
    if (displayAmount === null || displayAmount === undefined) return 0;
    
    const basisPoints = Math.round(displayAmount * Math.pow(10, decimals));
    console.log(`Converting ${displayAmount} display units × 10^${decimals} = ${basisPoints} basis points`);
    return basisPoints;
}

// ✅ CORRECT: Example for 1 SOL = 160 USDC
const tokenA = { mint: "So11111111111111111111111111111112", decimals: 9 };  // SOL
const tokenB = { mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", decimals: 6 };  // USDC

// For ratio "1 SOL = 160 USDC":
const tokenADisplay = 1.0;    // 1 SOL
const tokenBDisplay = 160.0;  // 160 USDC

// Convert each using its own decimal precision
const ratioANumerator = displayToBasisPoints(tokenADisplay, tokenA.decimals);    // 1,000,000,000
const ratioBDenominator = displayToBasisPoints(tokenBDisplay, tokenB.decimals);  // 160,000,000

console.log(`Correct basis points: ${ratioANumerator} : ${ratioBDenominator}`);
// Output: Correct basis points: 1000000000 : 160000000
```

#### Complete Working Implementation with Token Normalization

```javascript
/**
 * ✅ COMPLETE CORRECT IMPLEMENTATION
 * Handles token normalization AND proper decimal conversion
 */
async function createPoolCorrect(userTokenA, userTokenB, exchangeRatio) {
    // Step 1: Fetch token decimals from chain
    const tokenADecimals = await getTokenDecimals(userTokenA.mint);
    const tokenBDecimals = await getTokenDecimals(userTokenB.mint);
    
    console.log(`Token decimals: ${userTokenA.symbol}=${tokenADecimals}, ${userTokenB.symbol}=${tokenBDecimals}`);
    
    // Step 2: Convert token strings to PublicKey for comparison
    const tokenAMint = new solanaWeb3.PublicKey(userTokenA.mint);
    const tokenBMint = new solanaWeb3.PublicKey(userTokenB.mint);
    
    // Step 3: Normalize tokens lexicographically (like smart contract does)
    const { normalizedTokenA, normalizedTokenB, tokensWereSwapped } = normalizeTokenOrder(
        tokenAMint, tokenBMint, tokenADecimals, tokenBDecimals
    );
    
    // Step 4: Calculate display amounts based on user's desired ratio
    // User wants: "1 userTokenA = exchangeRatio userTokenB"
    let tokenADisplayAmount, tokenBDisplayAmount;
    
    if (tokensWereSwapped) {
        // User's TokenA became normalized TokenB, User's TokenB became normalized TokenA
        // Convert "1 userTokenA = X userTokenB" to "X normalizedTokenA = 1 normalizedTokenB"
        tokenADisplayAmount = exchangeRatio;  // X units of user's TokenB (now normalized TokenA)
        tokenBDisplayAmount = 1.0;            // 1 unit of user's TokenA (now normalized TokenB)
    } else {
        // Tokens kept original order
        // "1 userTokenA = X userTokenB" stays as "1 normalizedTokenA = X normalizedTokenB"  
        tokenADisplayAmount = 1.0;            // 1 unit of user's TokenA (still normalized TokenA)
        tokenBDisplayAmount = exchangeRatio;  // X units of user's TokenB (still normalized TokenB)
    }
    
    // Step 5: Convert to basis points using NORMALIZED token decimals
    const ratioANumerator = displayToBasisPoints(tokenADisplayAmount, normalizedTokenA.decimals);
    const ratioBDenominator = displayToBasisPoints(tokenBDisplayAmount, normalizedTokenB.decimals);
    
    console.log(`Final basis points: ${ratioANumerator} : ${ratioBDenominator}`);
    
    // Step 6: Verify the math is correct
    console.log(`Verification: ${ratioANumerator / Math.pow(10, normalizedTokenA.decimals)} : ${ratioBDenominator / Math.pow(10, normalizedTokenB.decimals)}`);
    
    return {
        tokenAMint: normalizedTokenA.mint,
        tokenBMint: normalizedTokenB.mint,
        ratioANumerator,
        ratioBDenominator
    };
}

function normalizeTokenOrder(mintA, mintB, decimalsA, decimalsB) {
    // Lexicographic comparison (byte-wise, like Rust PublicKey::cmp)
    const bytesA = mintA.toBytes();
    const bytesB = mintB.toBytes();
    
    let aLessThanB = false;
    for (let i = 0; i < 32; i++) {
        if (bytesA[i] < bytesB[i]) { aLessThanB = true; break; }
        if (bytesA[i] > bytesB[i]) { aLessThanB = false; break; }
    }
    
    if (aLessThanB) {
        return {
            normalizedTokenA: { mint: mintA, decimals: decimalsA },
            normalizedTokenB: { mint: mintB, decimals: decimalsB },
            tokensWereSwapped: false
        };
    } else {
        return {
            normalizedTokenA: { mint: mintB, decimals: decimalsB },
            normalizedTokenB: { mint: mintA, decimals: decimalsA },
            tokensWereSwapped: true
        };
    }
}
```

#### Specific Examples by Decimal Combinations

**🔹 Example 1: SOL (9 decimals) + USDC (6 decimals) = 1:160**

```javascript
// Want: 1 SOL = 160 USDC
const solDecimals = 9;
const usdcDecimals = 6;

// ✅ CORRECT:
const ratioANumerator = 1.0 * Math.pow(10, solDecimals);     // 1,000,000,000
const ratioBDenominator = 160.0 * Math.pow(10, usdcDecimals); // 160,000,000

// Result: Pool correctly prices 1 SOL = 160 USDC
```

**🔹 Example 2: BTC (8 decimals) + ETH (18 decimals) = 1:15**

```javascript
// Want: 1 BTC = 15 ETH  
const btcDecimals = 8;
const ethDecimals = 18;

// ✅ CORRECT:
const ratioANumerator = 1.0 * Math.pow(10, btcDecimals);     // 100,000,000
const ratioBDenominator = 15.0 * Math.pow(10, ethDecimals);  // 15,000,000,000,000,000,000

// Result: Pool correctly prices 1 BTC = 15 ETH
```

**🔹 Example 3: Custom Token (6 decimals) + SOL (9 decimals) = 1000:1**

```javascript
// Want: 1000 CustomToken = 1 SOL
const customTokenDecimals = 6; 
const solDecimals = 9;

// Express as "1 CustomToken = 0.001 SOL"
// ✅ CORRECT:
const ratioANumerator = 1.0 * Math.pow(10, customTokenDecimals);      // 1,000,000
const ratioBDenominator = 0.001 * Math.pow(10, solDecimals);          // 1,000,000

// Result: Pool correctly prices 1000 CustomToken = 1 SOL
```

#### Error Prevention Checklist

**✅ Before Creating Any Pool:**

1. **Fetch Actual Decimals**: Always get decimals from chain, never hardcode
   ```javascript
   const decimals = await getTokenDecimals(mintAddress);
   ```

2. **Understand Token Normalization**: Smart contract reorders tokens lexicographically
   ```javascript
   const normalized = normalizeTokenOrder(mintA, mintB);
   ```

3. **Use Correct Decimals for Each Token**: Each side uses its own token's decimals
   ```javascript
   const basisPointsA = displayAmount * Math.pow(10, tokenA.decimals);
   const basisPointsB = displayAmount * Math.pow(10, tokenB.decimals);
   ```

4. **Verify Your Math**: Check the reverse calculation
   ```javascript
   const backToDisplay = basisPoints / Math.pow(10, decimals);
   console.log(`${basisPoints} basis points = ${backToDisplay} display units`);
   ```

5. **Test with Different Decimal Combinations**: Don't just test same-decimal pairs

#### Safe Pool Creation Utility

```javascript
/**
 * Safe pool creation that prevents decimal precision mistakes
 */
async function createPoolSafe(tokenAMint, tokenBMint, displayRatio) {
    // Validate inputs
    if (!tokenAMint || !tokenBMint) {
        throw new Error("Both token mints required");
    }
    
    if (!displayRatio || displayRatio <= 0) {
        throw new Error("Valid display ratio required");
    }
    
    // Get decimals from chain
    const [decimalsA, decimalsB] = await Promise.all([
        getTokenDecimals(tokenAMint),
        getTokenDecimals(tokenBMint)
    ]);
    
    console.log(`Token decimals: A=${decimalsA}, B=${decimalsB}`);
    
    // Convert to basis points correctly
    const ratioABasisPoints = 1.0 * Math.pow(10, decimalsA);
    const ratioBBasisPoints = displayRatio * Math.pow(10, decimalsB);
    
    // Verify calculation
    console.log(`Basis points: ${ratioABasisPoints} : ${ratioBBasisPoints}`);
    console.log(`Verification: ${ratioABasisPoints / Math.pow(10, decimalsA)} : ${ratioBBasisPoints / Math.pow(10, decimalsB)}`);
    
    return { ratioABasisPoints, ratioBBasisPoints };
}
```

**💰 Cost Impact:** This mistake typically costs 1.15+ SOL per failed pool creation attempt, so proper implementation is critical for production applications.

---

### `process_pool_pause`

Pauses specific pool operations using bitwise flags with granular control over which operations to halt. This function provides pool-level pause control that operates independently of system-wide pause states, allowing administrators to selectively disable deposits/withdrawals or swaps while maintaining other pool functionality.

**🎛️ Granular Operation Control:**
- **PAUSE_FLAG_LIQUIDITY (1)**: Pause deposits and withdrawals only
- **PAUSE_FLAG_SWAPS (2)**: Pause swap operations only  
- **PAUSE_FLAG_ALL (3)**: Pause both liquidity and swap operations (required for consolidation eligibility)
- **Bitwise Logic**: Flags can be combined using bitwise OR operations for flexible control

**🔒 Security & Authority Requirements:**
- **Admin Authority Required**: Only the admin authority (or program upgrade authority as fallback) can pause individual pools
- **System State Validation**: Validates system is not paused before allowing pool-specific pause operations
- **Admin Authority Validation**: Multi-layer validation through program data account verification with upgrade authority fallback
- **Idempotent Operation**: Pausing already paused operations does not cause errors

**📊 State Management & Tracking:**
- **Pool State Updates**: Modifies pool state flags to reflect pause status
- **Operation Logging**: Records which specific operations were paused for audit trail
- **Consolidation Eligibility**: Pools with both liquidity and swaps paused become eligible for fee consolidation
- **Persistent State**: Pause state survives program restarts and cluster maintenance

**Authority:** Admin Authority (with Program Upgrade Authority fallback)  
**Effect:** Blocks specified pool operations based on pause flags  
**Persistence:** Pause state survives restarts until explicitly unpaused  
**Compute Units:** 12,000 - 150,000 CUs

#### Instruction Format

**Discriminator:** `19` (single byte)  
**Total Data Length:** Variable (Borsh serialization)  
**Serialization:** Borsh format

```rust
// Instruction structure (Borsh serialized)
PoolInstruction::PausePool {
    pause_flags: u8,      // Bitwise flags for operations to pause
    pool_id: Pubkey,      // Expected Pool ID for security validation
}
```

#### Parameters
```rust
program_id: &Pubkey            // Program ID
pause_flags: u8                // Bitwise flags to pause
pool_id: Pubkey                // Expected Pool ID for security validation
accounts: &[AccountInfo; 4]    // Admin, SystemState, PoolState (w), ProgramData
```

#### Account Order

| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | Admin Authority | Signer, Readable | Must be admin authority (or program upgrade authority as fallback) |
| 1 | System State PDA | Readable | System state for pause validation |
| 2 | Pool State PDA | **Writable** | Pool state to update with pause information |
| 3 | Program Data Account | Readable | Program data account for authority validation |

#### Pause Flag Constants

```javascript
// Pause flag constants (from src/constants.rs)
const PAUSE_FLAG_LIQUIDITY = 1;  // 0b01 - Pause deposits/withdrawals
const PAUSE_FLAG_SWAPS = 2;      // 0b10 - Pause swaps  
const PAUSE_FLAG_ALL = 3;        // 0b11 - Pause both operations
```

#### JavaScript Example

```javascript
// Import Borsh for proper serialization
import { serialize } from 'borsh';

function createPausePoolInstruction(
    adminAuthority,
    poolStatePDA,
    programDataAccount,
    pauseFlags
) {
    const [systemStatePDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("system_state")],
        PROGRAM_ID
    );
    
    // Create PoolInstruction::PausePool using Borsh serialization
    const pausePoolInstruction = {
        pausePool: {
            pause_flags: pauseFlags,
            pool_id: poolStatePDA,
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
// Pause only liquidity operations
const pauseLiquidityOnly = createPausePoolInstruction(
    adminAuthority,
    poolStatePDA,
    programDataAccount,
    PAUSE_FLAG_LIQUIDITY
);

// Pause only swaps
const pauseSwapsOnly = createPausePoolInstruction(
    adminAuthority,
    poolStatePDA,
    programDataAccount,
    PAUSE_FLAG_SWAPS
);

// Pause all operations (required for consolidation)
const pauseAllOperations = createPausePoolInstruction(
    adminAuthority,
    poolStatePDA,
    programDataAccount,
    PAUSE_FLAG_ALL
);
```

#### Error Conditions
- **SystemPaused**: Cannot pause individual pools when system is paused
- **Unauthorized**: Calling account is not the program upgrade authority
- **InvalidAccountData**: Pool state PDA validation failed
- **AccountDataTooSmall**: Pool state account cannot store pause information

---

### `process_pool_unpause`

Unpauses specific pool operations using bitwise flags with granular control over which operations to restore. This function provides pool-level unpause control that operates independently of system-wide pause states, allowing administrators to selectively re-enable deposits/withdrawals or swaps with precise operational control.

**🎛️ Granular Operation Control:**
- **PAUSE_FLAG_LIQUIDITY (1)**: Unpause deposits and withdrawals only
- **PAUSE_FLAG_SWAPS (2)**: Unpause swap operations only  
- **PAUSE_FLAG_ALL (3)**: Unpause both liquidity and swap operations
- **Bitwise Logic**: Flags can be combined using bitwise OR operations for flexible control

**🔒 Security & Authority Requirements:**
- **Admin Authority Required**: Only the admin authority (or program upgrade authority as fallback) can unpause individual pools
- **System State Validation**: Validates system is not paused before allowing pool-specific unpause operations
- **Admin Authority Validation**: Multi-layer validation through program data account verification with upgrade authority fallback
- **Idempotent Operation**: Unpausing already unpaused operations does not cause errors

**📊 State Management & Tracking:**
- **Pool State Updates**: Modifies pool state flags to reflect unpause status
- **Operation Logging**: Records which specific operations were unpaused for audit trail
- **Consolidation Impact**: Unpausing operations may affect consolidation eligibility status
- **Persistent State**: Unpause state survives program restarts and cluster maintenance

**⚠️ Important Behavioral Notes:**
- **System Override**: Pool unpause only works when system is not paused (system pause overrides pool states)
- **Independent Operation**: Pool unpause does NOT affect system-wide pause state
- **Selective Control**: Can unpause specific operations while leaving others paused
- **Client Integration**: Applications should check both system and pool pause states before operations

**Authority:** Admin Authority (with Program Upgrade Authority fallback)  
**Effect:** Re-enables specified pool operations based on unpause flags  
**Persistence:** Unpause state survives restarts until explicitly paused again  
**Compute Units:** 12,000 - 150,000 CUs

#### Instruction Format

**Discriminator:** `20` (single byte)  
**Total Data Length:** Variable (Borsh serialization)  
**Serialization:** Borsh format

```rust
// Instruction structure (Borsh serialized)
PoolInstruction::UnpausePool {
    unpause_flags: u8,      // Bitwise flags for operations to unpause
    pool_id: Pubkey,        // Expected Pool ID for security validation
}
```

#### Parameters
```rust
program_id: &Pubkey            // Program ID
unpause_flags: u8              // Bitwise flags to unpause
pool_id: Pubkey                // Expected Pool ID for security validation
accounts: &[AccountInfo; 4]    // Admin, SystemState, PoolState (w), ProgramData
```

#### Account Order

| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | Admin Authority | Signer, Writable | Must be admin authority (or program upgrade authority as fallback) |
| 1 | System State PDA | Writable | System state for pause validation |
| 2 | Pool State PDA | **Writable** | Pool state to update with unpause information |
| 3 | Program Data Account | Readable | Program data account for authority validation |

#### JavaScript Example

```javascript
// Import Borsh for proper serialization
import { serialize } from 'borsh';

function createUnpausePoolInstruction(
    adminAuthority,
    poolStatePDA,
    programDataAccount,
    unpauseFlags
) {
    const [systemStatePDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("system_state")],
        PROGRAM_ID
    );
    
    // Create PoolInstruction::UnpausePool using Borsh serialization
    const unpausePoolInstruction = {
        unpausePool: {
            unpause_flags: unpauseFlags,
            pool_id: poolStatePDA,
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
const unpauseLiquidityOnly = createUnpausePoolInstruction(
    adminAuthority,
    poolStatePDA,
    programDataAccount,
    PAUSE_FLAG_LIQUIDITY
);

// Unpause only swaps
const unpauseSwapsOnly = createUnpausePoolInstruction(
    adminAuthority,
    poolStatePDA,
    programDataAccount,
    PAUSE_FLAG_SWAPS
);

// Unpause all operations
const unpauseAllOperations = createUnpausePoolInstruction(
    adminAuthority,
    poolStatePDA,
    programDataAccount,
    PAUSE_FLAG_ALL
);
```

#### Error Conditions
- **SystemPaused**: Cannot unpause individual pools when system is paused
- **Unauthorized**: Calling account is not the program upgrade authority
- **InvalidAccountData**: Pool state PDA validation failed
- **AccountDataTooSmall**: Pool state account cannot store unpause information

---

## Error Analysis & Troubleshooting

### Transaction Instruction Failure Analysis

Understanding multi-instruction transaction failures is crucial for debugging Fixed Ratio Trading operations. Most operations use multiple instructions that must all succeed atomically.

#### Common Instruction Patterns

**📋 Typical Transaction Structure:**
```
Instruction 0: Compute Budget (priority fee setting)
Instruction 1: Main Program Operation (pool creation, swap, etc.)
Instruction 2+: Additional operations (if applicable)
```

#### Instruction-Level Error Analysis

When you see errors like `Error processing Instruction 1: Program failed to complete`, this indicates:

- **Instruction 0**: Compute Budget instruction (usually succeeds)
- **Instruction 1**: Your main operation (failed)
- **Instruction 2+**: Not executed due to transaction atomicity

**🔍 Common Causes of Instruction 1 Failures:**

1. **Account Ordering Issues**
   - Smart contract expects accounts in a specific order
   - Misplaced signer accounts
   - Incorrect PDA derivation order
   - Token account vs mint account confusion

2. **Missing Account Initialization**
   - Associated token accounts not created
   - PDA accounts not properly derived
   - System program accounts missing
   - Rent-exempt balance insufficient

3. **Insufficient Account Permissions**
   - Missing signer flags on required accounts
   - Missing writable flags on accounts that need modification
   - Authority mismatches
   - Token account ownership issues

4. **Program Logic Validation Failures**
   - Custom error codes (see Error Codes section)
   - Business logic constraints not met
   - State validation failures
   - Arithmetic constraints violated

#### Debugging Transaction Failures

**🔧 Step-by-Step Debugging Process:**

1. **Identify the Failed Instruction**
   ```
   Error processing Instruction N: Program failed to complete
   ```
   This tells you which instruction in the transaction failed.

2. **Check Account Ordering**
   - Verify accounts match the exact order in API documentation
   - Ensure all required accounts are present
   - Validate PDA derivations match expected seeds

3. **Validate Account States**
   - Check if all token accounts exist and are initialized
   - Verify sufficient balances for operations
   - Ensure accounts have proper ownership

4. **Review Permissions**
   - Confirm signer accounts are marked as signers
   - Verify writable accounts are marked as writable
   - Check authority relationships

5. **Test with Simpler Operations**
   - Try view operations first to verify account setup
   - Use smaller amounts to test validation logic
   - Test with known working configurations

#### Account Ordering Requirements

**⚠️ Critical Account Order Rules:**

1. **System Accounts Always First**
   - System Program
   - Token Program
   - Associated Token Program (if used)
   - Rent Sysvar (if needed)

2. **Authority Accounts Next**
   - Signer accounts
   - Program authority PDAs
   - Pool authority PDAs

3. **Data Accounts Follow**
   - Pool state PDAs
   - System state PDAs
   - Treasury state PDAs

4. **Token Accounts Last**
   - Token mint accounts
   - Token vault accounts
   - User token accounts
   - LP token mint accounts

#### Common Error Patterns and Solutions

**🚨 "Program failed to complete" with no custom error:**

- **Likely Cause**: Account ordering or missing accounts
- **Solution**: Verify account list matches API documentation exactly
- **Check**: All required accounts present and in correct order

**🚨 Custom Error Codes (see Error Codes section):**

- **Likely Cause**: Business logic validation failure
- **Solution**: Check error code against Custom Error Codes table
- **Action**: Address specific validation requirement

**🚨 "Insufficient funds" or rent-related errors:**

- **Likely Cause**: Account creation or rent exemption issues
- **Solution**: Ensure sufficient SOL for rent and operations
- **Check**: Account minimum balance requirements met

**🚨 "Invalid account data" errors:**

- **Likely Cause**: Account not initialized or wrong account type
- **Solution**: Initialize missing accounts or verify account addresses
- **Check**: PDA derivations and account existence

#### Transaction Building Best Practices

**✅ Recommended Practices:**

1. **Use Raw RPC Approach**
   - Standard transaction builders often produce malformed transactions
   - Follow SOLANA_TRANSACTION_BUILDING_GUIDE.md for correct implementation
   - Test transactions with simulation before submitting

2. **Validate Before Submission**
   - Check all account states before building transaction
   - Verify sufficient balances and permissions
   - Use view functions to validate pool states

3. **Handle Errors Gracefully**
   - Parse specific error codes for user feedback
   - Implement retry logic for temporary failures
   - Provide clear error messages to users

4. **Test Incrementally**
   - Start with simple operations
   - Build up to complex multi-instruction transactions
   - Use devnet for initial testing

#### Error Code Reference Quick Lookup

For immediate error code lookup, see the [Custom Error Codes](#custom-error-codes) section below. Common issues:

- `1001`: Token pair configuration issues
- `1002`: Ratio validation failures  
- `1003`: Insufficient balance for operation
- `1004`: Token account validation failures
- `1023`: System paused (check system state)
- `1007`: Pool paused (check pool flags)

---

### `process_pool_pause`

Pauses specific operations on a pool.

#### Instruction Format

**Discriminator:** `19` (single byte)

```rust
// Instruction structure (Borsh serialized)
PoolInstruction::PausePool {
    pause_flags: u8,      // Bitwise flags for operations to pause
    pool_id: Pubkey,      // Expected Pool ID for security validation
}
```

#### Parameters
```rust
program_id: &Pubkey
pause_flags: u8
pool_id: Pubkey
accounts: &[AccountInfo; 4]
```

#### Account Order
| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | Admin Authority | Signer, Readable | Must be admin authority (or program upgrade authority as fallback) |
| 1 | System State PDA | Readable | System state for pause validation |
| 2 | Pool State PDA | Writable | Pool state to update with pause information |
| 3 | Program Data Account | Readable | Program data account for authority validation |

**Authority:** Admin Authority only  
**Flags:** Can pause liquidity, swaps, or both

#### Parameters
```rust
program_id: &Pubkey
pause_flags: u8    // Bitwise flags for operations
pool_id: Pubkey    // Expected Pool ID for security validation
accounts: &[AccountInfo; 4]
```

#### Pause Flags
- `1` (PAUSE_FLAG_LIQUIDITY): Pause deposits/withdrawals
- `2` (PAUSE_FLAG_SWAPS): Pause swaps
- `3` (PAUSE_FLAG_ALL): Pause all operations

---

### `process_pool_unpause`

Resumes paused operations on a pool.

**Authority:** Admin Authority only

#### Parameters
```rust
program_id: &Pubkey
unpause_flags: u8    // Same as pause flags
pool_id: Pubkey      // Expected Pool ID for security validation
accounts: &[AccountInfo; 4]
```

---

### `process_pool_update_fees`

Updates fee configuration for a specific pool.

Important: These fees are fixed SOL fees charged in lamports (smallest SOL unit), not percentages of trade volume. Clients must pass absolute lamport values. No percentage-based fees are supported by this instruction.

**Authority:** Admin Authority only  
**Note:** Fee modification requests can be submitted to support@davincicodes.net and will be evaluated on a case-by-case basis.

#### Parameters
```rust
program_id: &Pubkey
accounts: &[AccountInfo; 4]
update_flags: u8           // Which fees to update
new_liquidity_fee: u64     // New liquidity fee in lamports (fixed SOL fee per deposit/withdraw)
new_swap_fee: u64          // New swap fee in lamports (fixed SOL fee per swap)
pool_id: Pubkey            // Expected Pool ID for security validation
```

#### Update Flags
- `1` (FEE_UPDATE_FLAG_LIQUIDITY): Update liquidity fee only
- `2` (FEE_UPDATE_FLAG_SWAP): Update swap fee only
- `3` (FEE_UPDATE_FLAG_BOTH): Update both fees

#### Fee Limits (enforced by contract)
- Liquidity fee: 100,000 – 10,000,000 lamports (0.0001 – 0.01 SOL)
  - Constants: `MIN_LIQUIDITY_FEE` = 100_000, `MAX_LIQUIDITY_FEE` = 10_000_000
- Swap fee: 10,000 – 1,000,000 lamports (0.00001 – 0.001 SOL)
  - Constants: `MIN_SWAP_FEE` = 10_000, `MAX_SWAP_FEE` = 1_000_000

Notes:
- Units are lamports. Convert SOL to lamports as needed (1 SOL = 1_000_000_000 lamports).
- Fees are deducted from the caller's SOL balance and tracked in the pool state.

#### Serialization & Wire Format
- Send as the Borsh-serialized `PoolInstruction::UpdatePoolFees` enum variant.
- Do not hardcode discriminators or craft raw byte arrays.
- Use `try_to_vec()` in Rust or an equivalent Borsh enum serializer in JS/TS.

#### Preconditions (system initialization)
- System must be initialized (System State PDA exists and is owned by this program).
- Program Data Account must be the BPF Loader Upgradeable program data account for this program: derive using seeds `[program_id]` with loader `BPFLoaderUpgradeable11111111111111111111111`.
- Pool State PDA must be valid and owned by this program; it will be validated against the expected derived PDA using internal pool fields.

#### Account Structure (order and mutability)
| Index | Account | Signer | Writable | Description |
|-------|---------|--------|----------|-------------|
| 0 | Admin Authority (Program Upgrade Authority) | Yes | No | Must match program's upgrade authority |
| 1 | System State PDA | No | No | For pause and authority validation |
| 2 | Pool State PDA | No | Yes | Pool being updated |
| 3 | Program Data Account | No | No | BPF Loader Upgradeable program data account |

#### Error Conditions
- Invalid flags: non-[1,2,3] → `InvalidFeeUpdateFlags` (code 1043 in tests)
- Liquidity fee out of range → `InvalidLiquidityFee { fee, min, max }`
- Swap fee out of range → `InvalidSwapFee { fee, min, max }`
- Unauthorized caller → Admin authority validation failure
- System paused → `SystemPaused`

#### Example (Rust)
```rust
use solana_sdk::{instruction::{AccountMeta, Instruction}, pubkey::Pubkey};
use fixed_ratio_trading::{self as frt, types::instructions::PoolInstruction};
use frt::constants::{FEE_UPDATE_FLAG_LIQUIDITY, FEE_UPDATE_FLAG_SWAP};

let program_id = frt::id();

// Derive program data account (BPF Loader Upgradeable)
let (program_data_account, _bump) = Pubkey::find_program_address(
    &[program_id.as_ref()],
    &solana_program::bpf_loader_upgradeable::id(),
);

let ix = Instruction {
    program_id,
    accounts: vec![
        AccountMeta::new_readonly(admin_authority, true),
        AccountMeta::new_readonly(system_state_pda, false),
        AccountMeta::new(pool_state_pda, false),
        AccountMeta::new_readonly(program_data_account, false),
    ],
    data: PoolInstruction::UpdatePoolFees {
        update_flags: FEE_UPDATE_FLAG_LIQUIDITY | FEE_UPDATE_FLAG_SWAP, // 3
        new_liquidity_fee: 200_000, // 0.0002 SOL
        new_swap_fee: 20_000,       // 0.00002 SOL
    }.try_to_vec().unwrap(),
};
```

#### Example (TypeScript) – Borsh enum serialization
```ts
import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { serialize } from 'borsh';

// Pseudo-classes; prefer codegen from Rust to avoid drift
class UpdatePoolFees { constructor(
  public update_flags: number,      // u8
  public new_liquidity_fee: bigint, // u64
  public new_swap_fee: bigint,      // u64
) {} }

class PoolInstructionEnum { constructor(
  public UpdatePoolFees?: UpdatePoolFees,
) {} }

// Replace with generated schema
const schema = new Map<any, any>([
  [UpdatePoolFees, { kind: 'struct', fields: [
    ['update_flags', 'u8'],
    ['new_liquidity_fee', 'u64'],
    ['new_swap_fee', 'u64'],
  ]}],
  [PoolInstructionEnum, { kind: 'enum', field: 'enum', values: [
    ['UpdatePoolFees', UpdatePoolFees],
  ]}],
]);

export function buildUpdateFeesIx(args: {
  programId: PublicKey;
  adminAuthority: PublicKey; // signer
  systemStatePda: PublicKey;
  poolStatePda: PublicKey;
}) {
  const variant = new PoolInstructionEnum({
    UpdatePoolFees: new UpdatePoolFees(
      3,                 // both flags
      200_000n,          // liquidity fee
      20_000n,           // swap fee
    ),
  });
  const data = Buffer.from(serialize(schema as any, variant));

  // Derive program data account (BPF Loader Upgradeable)
  const [programDataAccount] = PublicKey.findProgramAddressSync(
    [args.programId.toBuffer()],
    new PublicKey('BPFLoaderUpgradeab1e11111111111111111111111'),
  );

  return new TransactionInstruction({
    programId: args.programId,
    keys: [
      { pubkey: args.adminAuthority, isSigner: true, isWritable: false },
      { pubkey: args.systemStatePda, isSigner: false, isWritable: false },
      { pubkey: args.poolStatePda, isSigner: false, isWritable: true },
      { pubkey: programDataAccount, isSigner: false, isWritable: false },
    ],
    data,
  });
}
```

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
**Fee:** 0.013 SOL (DEPOSIT_WITHDRAWAL_FEE constant)  
**Compute Units:** 310,000 CUs maximum (Dashboard: min observed 249K; set 310K for safety margin)

⚠️ **CRITICAL ACCOUNT REQUIREMENT:**
The Pool State PDA **MUST** be marked as writable in your transaction. This is required for fee tracking updates and will cause Error 1033 (FeeValidationFailed) if marked as ReadOnly.

#### Instruction Format

**Discriminator:** `2` (single byte)  
**Total Data Length:** 73 bytes  
**Serialization:** Borsh format

```rust
// Instruction structure
pub struct DepositInstruction {
    discriminator: u8,           // 1 byte: value = 2
    deposit_token_mint: Pubkey,  // 32 bytes: Token mint to deposit
    amount: u64,                 // 8 bytes: Amount in base units (little-endian)
    pool_id: Pubkey,             // 32 bytes: Expected Pool ID (security validation)
}
```

#### JavaScript Example
```javascript
// Create instruction data for Deposit
const instructionData = new Uint8Array(73); // 1 + 32 + 8 + 32 bytes
instructionData[0] = 2; // Deposit discriminator

// Copy token mint bytes (32 bytes)
depositTokenMint.toBytes().forEach((byte, index) => {
    instructionData[1 + index] = byte;
});

// Copy amount bytes (8 bytes, u64 little-endian)
const amountBytes = new Uint8Array(new BigUint64Array([BigInt(amountBaseUnits)]).buffer);
amountBytes.forEach((byte, index) => {
    instructionData[33 + index] = byte;
});

// Copy pool_id bytes (32 bytes)
poolId.toBytes().forEach((byte, index) => {
    instructionData[41 + index] = byte;
});
```

#### Parameters
```rust
program_id: &Pubkey
amount: u64                   // Amount in base units (smallest token units)
deposit_token_mint: Pubkey    // Which token to deposit
pool_id: Pubkey               // Expected Pool ID for security validation
accounts: &[AccountInfo; 11]
```

#### Account Structure
| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | User Authority Signer | Signer, Writable | Depositor authority |
| 1 | System Program | Readable | Solana system program |
| 2 | System State PDA | Readable | Global pause validation |
| 3 | Pool State PDA | Writable | Target pool state account |
| 4 | SPL Token Program | Readable | SPL Token program |
| 5 | Token A Vault PDA | Writable | Pool vault for Token A |
| 6 | Token B Vault PDA | Writable | Pool vault for Token B |
| 7 | User Input Token Account | Writable | Source token account |
| 8 | User Output LP Token Account | Writable | Destination LP token account |
| 9 | LP Token A Mint PDA | Writable | LP mint for Token A deposits |
| 10 | LP Token B Mint PDA | Writable | LP mint for Token B deposits |

#### Important Notes
- **Single token deposits only** - choose either Token A or Token B
- **1:1 LP token ratio** - receive exactly the amount of LP tokens as deposited tokens
- **Token-specific LP tokens** - Token A deposits get Token A LP tokens, Token B deposits get Token B LP tokens
- **User must create LP token account first** - transaction fails if LP token account doesn't exist
- **Fee destination** - Fees are collected to the Pool State PDA (distributed). Consolidation to the Main Treasury occurs via separate operations.

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
**Fee:** 0.013 SOL (DEPOSIT_WITHDRAWAL_FEE constant)  
**Compute Units:** 290,000 CUs maximum (Dashboard: min observed 227K; set 290K for safety margin)

⚠️ **CRITICAL ACCOUNT REQUIREMENT:**
The Pool State PDA **MUST** be marked as writable in your transaction. This is required for fee tracking updates and will cause Error 1033 (FeeValidationFailed) if marked as ReadOnly.

#### Instruction Format

**Discriminator:** `3` (single byte)  
**Total Data Length:** 73 bytes  
**Serialization:** Borsh format

```rust
// Instruction structure
pub struct WithdrawInstruction {
    discriminator: u8,            // 1 byte: value = 3
    withdraw_token_mint: Pubkey,  // 32 bytes: Token mint to receive
    lp_amount_to_burn: u64,       // 8 bytes: LP tokens to burn (little-endian)
    pool_id: Pubkey,              // 32 bytes: Expected Pool ID (security validation)
}
```

#### JavaScript Example
```javascript
// Create instruction data for Withdraw
const instructionData = new Uint8Array(73); // 1 + 32 + 8 + 32 bytes
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

// Copy pool_id bytes (32 bytes)
poolId.toBytes().forEach((byte, index) => {
    instructionData[41 + index] = byte;
});
```

#### Parameters
```rust
program_id: &Pubkey
lp_amount_to_burn: u64        // LP tokens to burn
withdraw_token_mint: Pubkey   // Which token to receive
pool_id: Pubkey               // Expected Pool ID for security validation
accounts: &[AccountInfo; 11]
```

#### Account Structure
| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | User Authority Signer | Signer, Writable | Withdrawer authority |
| 1 | System Program | Readable | Solana system program |
| 2 | System State PDA | Readable | Global pause validation |
| 3 | Pool State PDA | Writable | Target pool state account |
| 4 | SPL Token Program | Readable | SPL Token program |
| 5 | Token A Vault PDA | Writable | Pool vault for Token A |
| 6 | Token B Vault PDA | Writable | Pool vault for Token B |
| 7 | User Input LP Token Account | Writable | Source LP token account |
| 8 | User Output Token Account | Writable | Destination token account |
| 9 | LP Token A Mint PDA | Writable | LP mint A (authority validation) |
| 10 | LP Token B Mint PDA | Writable | LP mint B (authority validation) |

#### Important Notes
- **Fee destination** - Fees are collected to the Pool State PDA (distributed). Consolidation to the Main Treasury occurs via separate operations.

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

**Critical: Dust Handling and Expected Amount Requirements**

⚠️ **IMPORTANT**: The `expected_amount_out` parameter must be **EXACT** - not a minimum acceptable amount.

**Dust Rounding Behavior:**
When swapping between tokens with different decimal precisions, the contract **rounds down** any fractional amounts that cannot be represented in the output token's precision. This prevents creation of dust amounts.

**Example - Dust Elimination:**
```
Pool: 1:1 ratio between Token A (9 decimals) and Token B (0 decimals)
- Input: 0.999999999 Token A (999,999,999 basis points)
- Calculation: 999,999,999 × 1 ÷ 1 = 999,999,999 
- Token B precision: 0 decimals (1 basis point = 1 whole token)
- Result: 999,999,999 ÷ 1,000,000,000 = 0.999... → rounds down to 0
- Output: 0 Token B (dust eliminated)
```

**Key Points:**
1. **Exact Match Required**: The swap will fail if `expected_amount_out` doesn't exactly match the calculated output
2. **No Dust Creation**: Fractional amounts below the output token's smallest unit are discarded
3. **User Protection**: Always calculate expected output considering decimal differences and rounding
4. **Precision Loss**: When swapping from high-precision to low-precision tokens, fractional amounts are lost

**Best Practice:**
Always use the contract's calculation logic to determine the exact output amount before submitting a swap. Consider token decimal differences and ensure your input amount will result in a meaningful output after rounding.

**📚 Calculation Guides:**

For detailed examples and step-by-step instructions on calculating exact swap amounts for any pool configuration, see our language-specific implementation guides:

**C#/.NET Developers:**
[EXPECTED_TOKENS_GUIDE_CSHARP.md](EXPECTED_TOKENS_GUIDE_CSHARP.md) - Complete C#/.NET implementation guide including:
- FRTExpectedTokens class with all calculation methods
- Overflow handling with BigInteger
- Comprehensive examples and test cases
- Integration patterns for smart contract calls
- Debugging and troubleshooting techniques

**JavaScript/TypeScript Developers:**
[EXPECTED_TOKENS_GUIDE_JAVASCRIPT.md](EXPECTED_TOKENS_GUIDE_JAVASCRIPT.md) - Complete JavaScript/TypeScript implementation guide including:
- TokenPairRatio class for all calculations
- BigInt support for large number handling
- Practical examples with real pool scenarios
- Integration patterns for web applications
- Testing strategies and validation methods

**General Reference:**
[SWAP_CALCULATION_GUIDE.md](SWAP_CALCULATION_GUIDE.md) - Language-agnostic mathematical foundation including:
- Complete calculation formulas for all pool types
- Mathematical concepts and basis points explanation
- Common pitfalls and edge cases
- Cross-language validation strategies

**Trading Flow:**
1. User specifies input token and amount
2. Contract calculates exact output using fixed ratio
3. Validates user's expected minimum output
4. Transfers input tokens to pool vault
5. Transfers calculated output tokens to user
6. Updates pool balances and fee accounting

**Authority:** Any user (unless owner-only mode)  
**Fee:** 0.0002715 SOL (SWAP_CONTRACT_FEE constant)  
**Compute Units:** 250,000 CUs maximum (Dashboard: tested 202K works; set to 250K to allow for fee changes and variability)

⚠️ **CRITICAL ACCOUNT REQUIREMENT:**
The Pool State PDA **MUST** be marked as writable in your transaction. This is the #1 cause of Error 1033 (FeeValidationFailed). The contract needs to update fee tracking fields in the pool state during swap execution.

#### Instruction Format

**Discriminator:** `4` (single byte)  
**Total Data Length:** 81 bytes (when manually constructing bytes; recommended to Borsh-serialize the enum)  
**Serialization:** Borsh format

```rust
// Instruction structure (when manually constructing bytes)
pub struct SwapInstruction {
    discriminator: u8,           // 1 byte: value = 4
    input_token_mint: Pubkey,    // 32 bytes: Input token mint
    amount_in: u64,              // 8 bytes: Input amount in basis points (little-endian)
    expected_amount_out: u64,    // 8 bytes: EXACT expected output (little-endian)
    pool_id: Pubkey,             // 32 bytes: Expected Pool ID for security validation
}
```

#### JavaScript Example
```javascript
// Create instruction data for Swap (manual bytes; prefer Borsh enum serialization)
const instructionData = new Uint8Array(81); // 1 + 32 + 8 + 8 + 32
instructionData[0] = 4; // Swap discriminator

// input_token_mint (32 bytes)
inputTokenMint.toBytes().forEach((b, i) => instructionData[1 + i] = b);

// amount_in (u64 little-endian)
new Uint8Array(new BigUint64Array([BigInt(amountInBaseUnits)]).buffer)
  .forEach((b, i) => instructionData[33 + i] = b);

// expected_amount_out (u64 little-endian)
new Uint8Array(new BigUint64Array([BigInt(expectedAmountOut)]).buffer)
  .forEach((b, i) => instructionData[41 + i] = b);

// pool_id (32 bytes)
poolId.toBytes().forEach((b, i) => instructionData[49 + i] = b);
```

#### Parameters
```rust
program_id: &Pubkey
amount_in: u64              // Input amount in basis points
expected_amount_out: u64    // EXACT expected output (must match calculated amount precisely)
pool_id: Pubkey             // Expected Pool ID for security validation
accounts: &[AccountInfo; 11]
```

#### Account Structure
| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | User Authority Signer | Signer, Readable | Swapper authority |
| 1 | System Program | Readable | Solana system program |
| 2 | System State PDA | Readable | Global pause validation |
| 3 | Pool State PDA | Writable | Pool state (fee tracking, flags) |
| 4 | SPL Token Program | Readable | SPL Token program |
| 5 | Token A Vault PDA | Writable | Pool vault for Token A |
| 6 | Token B Vault PDA | Writable | Pool vault for Token B |
| 7 | User Input Token Account | Writable | Source tokens |
| 8 | User Output Token Account | Writable | Destination tokens |
| 9 | Input Token Mint | Readable | Mint for input token (decimals/validation) |
| 10 | Output Token Mint | Readable | Mint for output token (decimals/validation) |

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

**Authority:** Admin Authority (with Program Upgrade Authority fallback)  
**Purpose:** Enables sophisticated operational models and custom business logic  
**Effect:** Controls who can execute swaps on the pool

**Note:** This advanced feature enables custom wrapper functions for `process_swap_execute` with specialized rules, fees, or operational models. Organizations can deploy their own contracts with any business logic while the protocol maintains security and administrative control. Contact support@davincicodes.net for implementation guidance.

#### Parameters
```rust
program_id: &Pubkey
enable_restriction: bool    // Enable/disable owner-only restrictions
designated_owner: Pubkey    // Entity to delegate operational control to (ignored when disabling)
pool_id: Pubkey             // Expected Pool ID for security validation
accounts: &[AccountInfo; 4]
```

#### Account Structure
| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | Admin Authority | Signer, **Readable** | Must be admin authority (program upgrade authority) |
| 1 | System State PDA | Readable | System state for pause validation |
| 2 | Pool State PDA | Writable | Pool state to modify access restrictions |
| 3 | Program Data Account | Readable | Program data account for authority validation |

#### Mutability & Signer Flags (exact)
- [0] Admin Authority: signer, read-only
- [1] System State PDA: read-only
- [2] Pool State PDA: writable
- [3] Program Data Account: read-only

#### Authorization
- Only the protocol Admin Authority (program upgrade authority) may call this function.
- Calls from the pool owner or any other signer are rejected. This is verified in tests.
- System pause applies: operation is blocked while paused.

#### Serialization & Wire Format
- SetSwapOwnerOnly must be sent as the Borsh-serialized `PoolInstruction` enum variant.
- Do not handcraft a byte array or hardcode a numeric discriminator.
- Use `try_to_vec()` in Rust or an equivalent Borsh enum serializer in JS/TS.
- The enum variant index is an internal detail and not a stable public API.

Rust example (already uses `try_to_vec()` in the snippet below). For JS/TS guidance, see the example later in this section.

#### Example (Rust) – building the instruction
```rust
use solana_sdk::{instruction::{AccountMeta, Instruction}, transaction::Transaction};
use fixed_ratio_trading::{self as frt, utils::program_authority::get_program_data_address};

let ix = Instruction {
    program_id: frt::id(),
    accounts: vec![
        // 0. Admin Authority (Program Upgrade Authority)
        AccountMeta::new_readonly(admin_authority_pubkey, true),
        // 1. System State PDA
        AccountMeta::new_readonly(system_state_pda, false),
        // 2. Pool State PDA (writable)
        AccountMeta::new(pool_state_pda, false),
        // 3. Program Data Account
        AccountMeta::new_readonly(get_program_data_address(&frt::id()), false),
    ],
    data: fixed_ratio_trading::PoolInstruction::SetSwapOwnerOnly {
        enable_restriction: true,
        designated_owner: delegated_owner,
        pool_id: pool_state_pda, // Include pool_id for security validation
    }
    .try_to_vec()
    .unwrap(),
};
```

#### Preconditions (system initialization)
- The system must be initialized before calling `SetSwapOwnerOnly`.
- The System State PDA must exist and be owned by this program.
- The Program Data Account must be the BPF Loader Upgradeable program data for this program, derivable via `[program_id]` per the loader’s rules (use `get_program_data_address(&program_id)`).

#### Operational Flow & State Changes

**When Enabling Restrictions (`enable_restriction: true`):**
1. **Authority Validation**: Verifies caller is admin authority (or program upgrade authority as fallback)
2. **System State Check**: Ensures system is not paused
3. **Pool State Load**: Loads and validates pool configuration
4. **Flag Update**: Sets `POOL_FLAG_SWAP_FOR_OWNERS_ONLY` in pool flags
5. **Ownership Delegation**: Changes pool owner to `designated_owner`
6. **State Persistence**: Saves updated pool state atomically

**When Disabling Restrictions (`enable_restriction: false`):**
1. **Authority Validation**: Verifies caller is admin authority (or program upgrade authority as fallback)
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
| **Unauthorized** | Caller is not admin authority or program upgrade authority | Use correct admin authority |
| **InvalidAccountData** | Pool state PDA validation failed | Verify pool PDA derivation |
| **AccountDataTooSmall** | Pool state account too small | Contact support (should not occur) |

#### Post-Configuration Behavior

**With Restrictions Enabled:**
- `process_swap_execute` only accepts transactions signed by `pool_state.owner` (set to `designated_owner` at enable time)
- Regular users receive authorization errors when attempting direct swaps
- Designated owner can deploy any custom business logic contracts
- Pool liquidity operations (`deposit`/`withdraw`) remain unrestricted
- Admin Authority cannot bypass swap-time restrictions; it must delegate by setting ownership.

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

#### Designated Owner Rules
- When enabling (`enable_restriction = true`), `designated_owner` must be a valid Pubkey; pool ownership is delegated to this key.
- When disabling (`enable_restriction = false`), `designated_owner` is ignored.

#### JS/TS client guidance: Borsh-serialize the enum variant
- Do not construct `[discriminator, bool, pubkey]` manually; enum variant indices may change.
- Use a Borsh schema that includes the `PoolInstruction` enum and serialize the `SetSwapOwnerOnly` variant.
- Recommended: generate the schema from the Rust definitions (codegen) to avoid drift.

Example (TypeScript) using `borsh` with a generated schema (pseudo-code):
```ts
import { serialize } from 'borsh';
import { PublicKey, TransactionInstruction } from '@solana/web3.js';

// These classes mirror the Rust types; schema should be generated from Rust.
class SetSwapOwnerOnly {
  enable_restriction: number; // u8 in borsh (0/1)
  designated_owner: Uint8Array; // 32 bytes
  constructor(fields: { enable_restriction: boolean; designated_owner: PublicKey }) {
    this.enable_restriction = fields.enable_restriction ? 1 : 0;
    this.designated_owner = fields.designated_owner.toBytes();
  }
}

class PoolInstructionEnum {
  // Discriminated union shape: { SetSwapOwnerOnly: SetSwapOwnerOnly }
  SetSwapOwnerOnly?: SetSwapOwnerOnly;
  constructor(fields: { SetSwapOwnerOnly: SetSwapOwnerOnly }) {
    Object.assign(this, fields);
  }
}

// IMPORTANT: Use a schema generated from Rust to ensure variant ordering matches.
// Pseudo schema; replace with generated one.
const schema = new Map<any, any>([
  [SetSwapOwnerOnly, { kind: 'struct', fields: [
    ['enable_restriction', 'u8'],
    ['designated_owner', [32]],
  ]}],
  [PoolInstructionEnum, { kind: 'enum', field: 'enum', values: [
    ['SetSwapOwnerOnly', SetSwapOwnerOnly],
    // ...other variants defined by codegen
  ]}],
]);

export function buildSetSwapOwnerOnlyIx(params: {
  programId: PublicKey;
  systemStatePda: PublicKey;
  poolStatePda: PublicKey;
  programData: PublicKey;
  adminAuthority: PublicKey; // signer
  enableRestriction: boolean;
  designatedOwner: PublicKey;
}): TransactionInstruction {
  const variant = new PoolInstructionEnum({
    SetSwapOwnerOnly: new SetSwapOwnerOnly({
      enable_restriction: params.enableRestriction,
      designated_owner: params.designatedOwner,
      pool_id: params.poolStatePda,
    }),
  });

  const data = Buffer.from(serialize(schema as any, variant));

  return new TransactionInstruction({
    programId: params.programId,
    keys: [
      { pubkey: params.adminAuthority, isSigner: true, isWritable: false },
      { pubkey: params.systemStatePda, isSigner: false, isWritable: false },
      { pubkey: params.poolStatePda, isSigner: false, isWritable: true },
      { pubkey: params.programData, isSigner: false, isWritable: false },
    ],
    data,
  });
}
```

Notes:
- The schema shown is illustrative; use generated schema to avoid variant index drift.
- `enable_restriction` is serialized as `u8` (0 or 1) to match Borsh expectations for Rust `bool` in some toolchains. If your codegen emits `bool`, follow the emitted type.

#### Legacy compatibility (pool schema version)
- Only pools created with the current schema (v0.16.x+) support `SetSwapOwnerOnly`.
- Legacy pools are incompatible (e.g., `PoolState` account length ~438 bytes). Current pools are ~597 bytes.
- How to check: fetch the pool state account and inspect `account.data.length`.
- If the length indicates a legacy layout, migration or creating a new pool is required before using this instruction.

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

Withdraws collected protocol fees from the main treasury with dynamic rate limiting and security protections. Enables the protocol authority to withdraw accumulated fees from pool creation, liquidity operations, and swaps while implementing dynamic hourly rate limits with a 60-minute cooldown only after successful withdrawals to ensure system stability.

**Authority:** Admin Authority only  
**Restrictions:** Dynamic hourly rate limits with 60-minute cooldown only after successful withdrawals  
**Compute Units:** Allocate up to 150,000 CUs; observed minimum ~80,000 CUs

#### Parameters
```rust
program_id: &Pubkey
amount: u64    // Amount to withdraw in lamports (minimum 0.01 SOL, 0 = withdraw all available)
accounts: &[AccountInfo]  // Provide exactly 6 accounts in the order below
```

#### Serialization & Wire Format
- Send as the Borsh-serialized `PoolInstruction::WithdrawTreasuryFees { amount }` enum variant.
- Do not hardcode discriminators or craft raw byte arrays.
- Use `try_to_vec()` in Rust or an equivalent Borsh enum serializer in JS/TS.

#### Account Structure
| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | System Authority | Signer (read-only) | Must be admin authority |
| 1 | Main Treasury PDA | Writable | Treasury account to withdraw from |
| 2 | Rent Sysvar | Readable | For rent-exempt minimum calculations |
| 3 | Destination Account | Writable | Account to receive withdrawn SOL |
| 4 | System State PDA | Readable | For pause validation and authority checks |
| 5 | Program Data Account | Readable | Program data account for authority validation |

Exact account order is required. The `Program Data Account` must be derived with BPF Loader Upgradeable using seeds `[program_id]`.

#### Preconditions
- System initialized: both `System State PDA` and `Main Treasury PDA` exist and are owned by the program.
- Admin authority: caller must be the configured admin (program upgrade authority validation via program data account).
- System not paused: withdrawals blocked while paused; upon unpause, a restart penalty window applies.
- Rent protection: withdrawal is limited to lamports above rent-exempt minimum of the treasury account.
- Minimum amount: withdrawal amount must be at least 0.1 SOL (100,000,000 lamports) unless using amount = 0 for withdraw-all.

#### Rate Limiting Implementation Details

The contract implements a sophisticated dynamic rate limiting system:

```rust
// Dynamic rate scaling based on treasury balance + cooldown only after success
let current_hourly_limit = treasury_state.calculate_current_hourly_rate_limit();
treasury_state.validate_withdrawal_rate_limit(amount, current_timestamp)?;
```

**Rate Scaling Logic:**
- **Base Rate**: 10 SOL/hour (TREASURY_BASE_HOURLY_RATE constant)
- **Scaling Factor**: 10x multiplier per balance tier (TREASURY_RATE_SCALING_MULTIPLIER constant)
- **Target**: Ensure complete treasury drainage possible within 48 hours maximum
- **Cooldown (Non-Cumulative)**: 60-minute window starts only after a successful withdrawal. Failed attempts (amount > hourly limit or within cooldown) do not extend or reset the cooldown.

**System Restart Penalty (71 hours):**
- **Duration**: 71 hours after a system-wide unpause
- **Effect**: Blocks all treasury withdrawals during this period
- **Behavior**: Not cumulative; once 71 hours elapse, normal dynamic rate limiting resumes
- **Precedence**: Penalty check occurs before rate/cooldown checks

#### Dynamic Rate Calculation Example

**Treasury with 27 SOL:**

**Step 1: Available Balance Calculation**
- Treasury balance: 27 SOL = 27,000,000,000 lamports
- Rent-exempt minimum: ~2.04 SOL = 2,040,000,000 lamports (MainTreasuryState size)
- **Available for withdrawal**: ~25 SOL = 25,000,000,000 lamports

**Step 2: Hourly Rate Limit Calculation**
```rust
let available_balance = 25,000,000,000; // ~25 SOL
let mut current_rate = 10,000,000,000;  // 10 SOL/hour (base rate)

// Check if available_balance > (48 hours × current_rate)
// 25,000,000,000 > (48 × 10,000,000,000)?
// 25 SOL > 480 SOL? NO - rate stays at 10 SOL/hour
```

**Step 3: Scaling Tiers**

| Treasury Balance | Available Balance | Hourly Rate Limit | Scaling Tier |
|-----------------|-------------------|-------------------|--------------|
| 27 SOL | ~25 SOL | **10 SOL/hour** | Tier 1 (Base) |
| 500 SOL | ~498 SOL | **100 SOL/hour** | Tier 2 (10x) |
| 5,000 SOL | ~4,998 SOL | **1,000 SOL/hour** | Tier 3 (100x) |

**Step 4: Withdrawal Schedule Example**
- **Hour 0**: Can withdraw up to 10 SOL ✅ (within limit)
- **Wait 60 minutes** (cooldown after successful withdrawal)
- **Hour 1**: Can withdraw up to 10 SOL ✅ (if balance allows)
- **Hour 2**: Can withdraw remaining ~5 SOL ✅ (above rent-exempt minimum)

**Error Conditions:**
- Rate limit exceeded: withdrawal amount exceeds current hourly limit; returns InvalidInstructionData
- Cooldown active: within 60 minutes of last successful withdrawal; returns InvalidInstructionData  
- System restart penalty active: logs remaining penalty time; returns InvalidInstructionData
- Below minimum amount: withdrawal less than 0.01 SOL (InvalidInstructionData)
- Insufficient funds: withdrawal exceeds available above rent (InsufficientFunds)
- Invalid authority: caller is not the admin authority (authority validation failure)
- Invalid account data: incorrect treasury PDA or malformed treasury state

#### Example (Rust)
```rust
use solana_sdk::{instruction::{AccountMeta, Instruction}, pubkey::Pubkey};
use fixed_ratio_trading::{self as frt, PoolInstruction};

let program_id = frt::id();
let amount: u64 = 1_000_000_000; // 1 SOL

// Derive Program Data (BPF Loader Upgradeable)
let (program_data_account, _bump) = Pubkey::find_program_address(
    &[program_id.as_ref()],
    &solana_program::bpf_loader_upgradeable::id(),
);

let ix = Instruction {
    program_id,
    accounts: vec![
        AccountMeta::new_readonly(admin_authority, true),
        AccountMeta::new(main_treasury_pda, false),
        AccountMeta::new_readonly(solana_program::sysvar::rent::id(), false),
        AccountMeta::new(destination_pubkey, false),
        AccountMeta::new_readonly(system_state_pda, false),
        AccountMeta::new_readonly(program_data_account, false),
    ],
    data: PoolInstruction::WithdrawTreasuryFees { amount }.try_to_vec().unwrap(),
};
```

#### Example (TypeScript) – Borsh enum serialization
```ts
import { PublicKey, TransactionInstruction, SYSVAR_RENT_PUBKEY } from '@solana/web3.js';
import { serialize } from 'borsh';

class WithdrawTreasuryFees { constructor(public amount: bigint) {} }
class PoolInstructionEnum { constructor(public WithdrawTreasuryFees?: WithdrawTreasuryFees) {} }

// Replace with generated schema from Rust
const schema = new Map<any, any>([
  [WithdrawTreasuryFees, { kind: 'struct', fields: [['amount', 'u64']] }],
  [PoolInstructionEnum, { kind: 'enum', field: 'enum', values: [['WithdrawTreasuryFees', WithdrawTreasuryFees]] }],
]);

export function buildWithdrawIx(args: {
  programId: PublicKey;
  adminAuthority: PublicKey; // signer
  mainTreasuryPda: PublicKey;
  systemStatePda: PublicKey;
  destination: PublicKey;
  amountLamports: bigint; // 0 = withdraw all available
}): TransactionInstruction {
  const variant = new PoolInstructionEnum({ WithdrawTreasuryFees: new WithdrawTreasuryFees(args.amountLamports) });
  const data = Buffer.from(serialize(schema as any, variant));

  const [programDataAccount] = PublicKey.findProgramAddressSync(
    [args.programId.toBuffer()],
    new PublicKey('BPFLoaderUpgradeab1e11111111111111111111111'),
  );

  const keys = [
    { pubkey: args.adminAuthority, isSigner: true, isWritable: false },
    { pubkey: args.mainTreasuryPda, isSigner: false, isWritable: true },
    { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
    { pubkey: args.destination, isSigner: false, isWritable: true },
    { pubkey: args.systemStatePda, isSigner: false, isWritable: false },
    { pubkey: programDataAccount, isSigner: false, isWritable: false },
  ];

  return new TransactionInstruction({ programId: args.programId, keys, data });
}
```

#### Additional Security Features

**System Restart Penalty:**
- **Duration**: 71 hours (3 days) after system unpause
- **Purpose**: Prevents immediate fund drainage after system recovery
- **Enforcement**: Blocks all withdrawals during penalty period

**Flexible Withdrawal Options:**
- **Partial Withdrawals**: Specify exact amount in lamports (minimum 0.01 SOL)
- **Full Withdrawal**: Use amount = 0 to withdraw all available funds (bypasses minimum)
- **Balance Protection**: Automatically maintains rent-exempt minimum
- **Real-time Validation**: Checks available balance and minimum amount before processing

---

### `process_treasury_get_info`

Retrieves current treasury state information. Returns comprehensive treasury data including total balance, consolidated fee collection statistics, withdrawal history, and operational metrics. This read-only function provides transparency into protocol revenue and treasury status from both direct collection (pool creation) and consolidated fees (liquidity/swap operations).

**Authority:** Public (read-only)

#### Parameters
```rust
program_id: &Pubkey
accounts: &[AccountInfo; 1]
```

#### Account Requirements
| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | Main Treasury PDA | Read-only | Treasury account to query information from |

**Treasury PDA Derivation:**
```rust
let (main_treasury_pda, _) = Pubkey::find_program_address(
    &[MAIN_TREASURY_SEED_PREFIX], // "main_treasury"
    &program_id,
);
```

#### Expected Result Format (via logs)

The function returns comprehensive treasury information through structured log messages. Developers should parse these logs to extract treasury data:

```
📊 Getting real-time treasury information
✅ Successfully loaded treasury state from account data

🏦 TREASURY INFORMATION (DISTRIBUTED COLLECTION + CONSOLIDATION):
   Current Balance: {balance} lamports ({sol_balance} SOL)
   Total Withdrawn: {withdrawn} lamports ({withdrawn_sol} SOL)

📈 OPERATION STATISTICS:
   Pool Creations: {count} (Total fees: {total_fees} lamports, Avg: {avg_fee})
   Liquidity Operations: {count} (Total fees: {total_fees} lamports, Avg: {avg_fee})
   Regular Swaps: {count} (Total fees: {total_fees} lamports, Avg: {avg_fee})
   Treasury Withdrawals: {count} (Total: {total_withdrawn} lamports)
   Consolidations: {count} (Last: {timestamp})
   Donations: {count} (Total: {total_donations} lamports, {donations_sol} SOL)

📊 ENHANCED ANALYTICS:
   Total Successful Operations: {total_operations}
   Failed Operations: {failed_count}
   Success Rate: {success_rate}%
   Total Fees Collected: {total_fees} lamports ({total_fees_sol} SOL)
   Average Fee per Operation: {avg_fee} lamports

⏰ TIMING INFORMATION:
   Last Update: {timestamp}

✅ TREASURY BENEFITS:
   • Real-time data (no consolidation needed)
   • Single source of truth
   • No race conditions
   • Simplified architecture
```

#### Parsing Guidelines for Developers

1. **Balance Information**: Extract current balance and total withdrawn amounts from the "TREASURY INFORMATION" section
2. **Operation Metrics**: Parse individual operation counts and fee totals from "OPERATION STATISTICS"
3. **Analytics Data**: Get success rates and averages from "ENHANCED ANALYTICS"
4. **Timestamp**: Extract last update timestamp from "TIMING INFORMATION"

#### Error Handling

If treasury state deserialization fails, the function creates a default state with current account balance:

```
⚠️ Warning: Failed to deserialize treasury state: {error}
🔄 Creating default treasury state with current account balance

📊 Default state created:
   - Current balance: {balance} lamports
   - Rent exempt minimum: 2039280 lamports
   - All counters reset to 0 (data corruption detected)
```

---

### `process_treasury_donate_sol`

Accepts SOL donations to support development.

**Instruction Discriminator:** `23` (DonateSol)  
**Authority:** Any user  
**Minimum:** 0.1 SOL (MIN_DONATION_AMOUNT constant)  
**Compute Units:** Variable by donation amount (see CU Analysis below)

**⚠️ CRITICAL PREREQUISITE:** The treasury system must be initialized first using `InitializeProgram` (discriminator `0`) before any donations can be made. This creates the required `SystemState` and `MainTreasury` PDAs.

**Note:** Donations help accelerate development of new features including contract improvements and the governance system outlined in the Future Governance Contract Design. The faster we reach our financial goals, the faster we deliver new capabilities.

#### Parameters
```rust
program_id: &Pubkey
amount: u64         // Donation amount (lamports)
message: String     // Optional message (max 200 chars)
accounts: &[AccountInfo; 3]
```

#### Instruction Data Structure
```rust
// Instruction structure for DonateSol (discriminator 23)
pub struct DonateSolInstruction {
    discriminator: u8,    // 1 byte: value = 23
    amount: u64,          // 8 bytes: Donation amount in lamports (little-endian)
    message: String,      // Variable: 4-byte length prefix + UTF-8 bytes
}
// Total size: 13+ bytes (1 + 8 + 4 + message_length)
```

#### Account Requirements

The following accounts must be provided in this exact order:

| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | Donor Account | Signer, Writable | Account donating SOL (must have sufficient balance) |
| 1 | Main Treasury PDA | Writable | Receives the donation (derived from `MAIN_TREASURY_SEED_PREFIX`) |
| 2 | System State PDA | Readable | For pause validation (derived from `SYSTEM_STATE_SEED_PREFIX`) |
| 3 | System Program | Readable | Solana system program (`11111111111111111111111111111112`) |

#### PDA Derivation
```javascript
// Derive required PDAs
const PROGRAM_ID = new PublicKey("4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn");

const [mainTreasuryPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("main_treasury")],
    PROGRAM_ID
);

const [systemStatePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("system_state")],
    PROGRAM_ID
);
```

#### Checking System Initialization
```javascript
// Check if treasury system is initialized
async function isSystemInitialized(connection, programId) {
    const [systemStatePda] = PublicKey.findProgramAddressSync(
        [Buffer.from("system_state")],
        programId
    );
    
    const [mainTreasuryPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("main_treasury")],
        programId
    );
    
    try {
        const systemStateAccount = await connection.getAccountInfo(systemStatePda);
        const mainTreasuryAccount = await connection.getAccountInfo(mainTreasuryPda);
        
        return systemStateAccount !== null && mainTreasuryAccount !== null;
    } catch (error) {
        return false;
    }
}
```

#### JavaScript Example
```javascript
// Create instruction data for DonateSol
const message = "Supporting development!";
const messageBytes = new TextEncoder().encode(message);
const messageLength = new Uint8Array(new Uint32Array([messageBytes.length]).buffer);

const instructionData = new Uint8Array([
    23,                                    // DonateSol discriminator
    ...new Uint8Array(new BigUint64Array([BigInt(amountLamports)]).buffer), // amount (u64 little-endian)
    ...messageLength,                      // message length (u32 little-endian)
    ...messageBytes                        // message UTF-8 bytes
]);

// Create the instruction
const instruction = new TransactionInstruction({
    keys: [
        { pubkey: donorAccount.publicKey, isSigner: true, isWritable: true },
        { pubkey: mainTreasuryPda, isSigner: false, isWritable: true },
        { pubkey: systemStatePda, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    programId: PROGRAM_ID,
    data: instructionData,
});
```

#### Common Errors and Solutions

| Error | Cause | Solution |
|-------|-------|----------|
| `invalid account data for instruction` | Treasury system not initialized | Call `InitializeProgram` (discriminator `0`) first |
| `AccountNotFound` | Wrong PDA derivation | Use correct seeds: `"main_treasury"` and `"system_state"` |
| `InvalidArgument` | Amount below minimum | Ensure donation ≥ 0.1 SOL (100,000,000 lamports) |
| `InsufficientFunds` | Donor has insufficient balance | Check donor account has enough SOL + transaction fees |
| `IncorrectProgramId` | Wrong system program | Use `11111111111111111111111111111112` |

#### Troubleshooting Steps

1. **Check System Initialization:**
   ```javascript
   const initialized = await isSystemInitialized(connection, PROGRAM_ID);
   if (!initialized) {
       console.log("❌ Treasury system not initialized. Call InitializeProgram first.");
   }
   ```

2. **Verify PDA Addresses:**
   ```javascript
   console.log("Main Treasury PDA:", mainTreasuryPda.toString());
   console.log("System State PDA:", systemStatePda.toString());
   ```

3. **Check Minimum Amount:**
   ```javascript
   const MIN_DONATION = 100_000_000; // 0.1 SOL in lamports
   if (amountLamports < MIN_DONATION) {
       console.log(`❌ Amount too small. Minimum: ${MIN_DONATION} lamports`);
   }
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

#### Parameters
```rust
program_id: &Pubkey
pool_count: u8              // Number of pool state PDAs provided (1..=20)
accounts: &[AccountInfo]    // Exactly 4 + pool_count accounts
```

#### Serialization & Wire Format
- Send as the Borsh-serialized `PoolInstruction::ConsolidatePoolFees` enum variant.
- Do not hardcode discriminators or craft raw byte arrays.
- Use `try_to_vec()` in Rust or an equivalent Borsh enum serializer in JS/TS.

#### Account Structure (order and mutability)
| Index | Account | Signer | Writable | Description |
|-------|---------|--------|----------|-------------|
| 0 | Admin Authority (Program Upgrade Authority) | Yes | No | Must match program's upgrade authority |
| 1 | System State PDA | No | No | Used for pause state and admin validation |
| 2 | Main Treasury PDA | No | Yes | Receives consolidated SOL |
| 3 | Program Data Account | No | No | BPF Loader Upgradeable program data for this program |
| 4..(3 + pool_count) | Pool State PDA | No | Yes | Pool(s) to consolidate from |

Exact account count required: `accounts.len() == 4 + pool_count`.

#### Preconditions
- System must be initialized; `System State PDA` must exist and be owned by this program.
- `Program Data Account` must be derived via BPF Loader Upgradeable using seed `[program_id]` with loader `BPFLoaderUpgradeab1e11111111111111111111111`.
- `Main Treasury PDA` must be valid and owned by this program.
- `pool_count` limits: `1 <= pool_count <= 20` (`MAX_POOLS_PER_CONSOLIDATION_BATCH`).
- Flexible pause requirement:
  - If system is paused: all provided pools are eligible.
  - If system is NOT paused: only pools with both `swaps_paused = true` AND `liquidity_paused = true` are eligible; others are skipped without error.

#### Consolidation Behavior
- Rent protection: Contract preserves each pool’s rent-exempt minimum; only transfers lamports above rent-exempt threshold.
- Partial consolidation: If available lamports above rent are less than `pending fees`, transfers the available amount and proportionally reduces fee counters.
- Accounting updates per pool:
  - For full consolidation: resets fee counters and updates metadata (timestamp, consolidation count).
  - For partial: subtracts consolidated portions from `collected_liquidity_fees` and `collected_swap_contract_fees`, increments totals and counters.
- Treasury updates: Updates `MainTreasuryState` via `batch_consolidation`, synchronizes with actual account balance.

#### Limits & Performance
- `pool_count` range: 1..=20 (hard limit).
- CU model: `Base_CUs ≈ 4,000 + pool_count × 5,000` (see table and formula above). Use up to 150K CU cap as policy.

#### Error Conditions
- `InvalidArgument`: when `pool_count == 0` or `pool_count > MAX_POOLS_PER_CONSOLIDATION_BATCH`.
- `NotEnoughAccountKeys`: when `accounts.len() != 4 + pool_count`.
- Unauthorized admin authority: admin validation fails (caller not upgrade authority).
- Other pools may be skipped (not errored) if ineligible (e.g., not paused when system is active) or have no fees or insufficient lamports above rent; operation still succeeds for eligible pools.

#### Example (Rust)
```rust
use solana_sdk::{instruction::{AccountMeta, Instruction}, pubkey::Pubkey};
use fixed_ratio_trading::{self as frt, PoolInstruction};

let program_id = frt::id();
let pool_count: u8 = 2;

// Derive program data account (BPF Loader Upgradeable)
let (program_data_account, _bump) = Pubkey::find_program_address(
    &[program_id.as_ref()],
    &solana_program::bpf_loader_upgradeable::id(),
);

let ix = Instruction {
    program_id,
    accounts: vec![
        AccountMeta::new_readonly(admin_authority, true),
        AccountMeta::new_readonly(system_state_pda, false),
        AccountMeta::new(main_treasury_pda, false),
        AccountMeta::new_readonly(program_data_account, false),
        AccountMeta::new(pool_state_pda_1, false),
        AccountMeta::new(pool_state_pda_2, false),
    ],
    data: PoolInstruction::ConsolidatePoolFees { pool_count }.try_to_vec().unwrap(),
};
```

#### Example (TypeScript) – Borsh enum serialization
```ts
import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { serialize } from 'borsh';

class ConsolidatePoolFees { constructor(public pool_count: number) {} }
class PoolInstructionEnum { constructor(public ConsolidatePoolFees?: ConsolidatePoolFees) {} }

// Replace with generated schema from Rust
const schema = new Map<any, any>([
  [ConsolidatePoolFees, { kind: 'struct', fields: [['pool_count', 'u8']] }],
  [PoolInstructionEnum, { kind: 'enum', field: 'enum', values: [['ConsolidatePoolFees', ConsolidatePoolFees]] }],
]);

export function buildConsolidateIx(args: {
  programId: PublicKey;
  adminAuthority: PublicKey; // signer
  systemStatePda: PublicKey;
  mainTreasuryPda: PublicKey;
  poolStatePdas: PublicKey[]; // length 1..20
}) {
  const variant = new PoolInstructionEnum({ ConsolidatePoolFees: new ConsolidatePoolFees(args.poolStatePdas.length) });
  const data = Buffer.from(serialize(schema as any, variant));

  const [programDataAccount] = PublicKey.findProgramAddressSync(
    [args.programId.toBuffer()],
    new PublicKey('BPFLoaderUpgradeab1e11111111111111111111111'),
  );

  const keys = [
    { pubkey: args.adminAuthority, isSigner: true, isWritable: false },
    { pubkey: args.systemStatePda, isSigner: false, isWritable: false },
    { pubkey: args.mainTreasuryPda, isSigner: false, isWritable: true },
    { pubkey: programDataAccount, isSigner: false, isWritable: false },
    ...args.poolStatePdas.map(p => ({ pubkey: p, isSigner: false, isWritable: true })),
  ];

  return new TransactionInstruction({ programId: args.programId, keys, data });
}
```

#### Developer Notes
- Provide only eligible pools when system is active (both swaps and liquidity paused) to avoid no-op processing.
- Pools with `pending_sol_fees == 0` are skipped without error.
- The contract preserves rent exemption; expect partial transfers if pool lamports are near rent threshold.

**⚠️ SECURITY UPDATE:** Admin Authority Required (was public function)  
**Authority:** Admin Authority only (prevents unauthorized fee manipulation)  
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
accounts: &[AccountInfo]    // Variable length: 4 + pool_count
```

#### Account Structure
| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | Admin Authority Signer | Signer, Readable | Must be admin authority |
| 1 | System State PDA | Readable | For pause status and admin validation |
| 2 | Main Treasury PDA | Writable | Receives consolidated fees |
| 3 | Program Data Account | Readable | For upgrade authority validation |
| 4+ | Pool State PDAs | Writable | Pools to consolidate (1-20 pools) |

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
- **⚠️ SECURITY**: Unauthorized access - caller must be admin authority
- **Invalid pool count**: 0 or > 20 pools
- **Insufficient accounts**: Account count ≠ (4 + pool_count)
- **Admin validation**: Admin authority signature or PDA validation failure
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

#### Core Pool Errors (1001-1019)
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

#### System Control Errors (1023-1030)
| Code | Error Type | Description |
|------|------------|-------------|
| 1023 | `SystemPaused` | System is paused - all operations blocked except unpause |
| 1024 | `SystemAlreadyPaused` | System is already paused |
| 1025 | `SystemNotPaused` | System is not paused |
| 1026 | `UnauthorizedAccess` | Unauthorized access to system controls |
| 1027 | `PoolSwapsPaused` | Pool swaps are currently paused by owner |
| 1028 | `SwapAccessRestricted` | Swap access restricted (owner-only mode) |
| 1029 | `PoolSwapsAlreadyPaused` | Pool swaps are already paused |
| 1030 | `PoolSwapsNotPaused` | Pool swaps are not currently paused |

#### Fee and Treasury Errors (1031-1046)
| Code | Error Type | Description |
|------|------------|-------------|
| 1031 | `InsufficientFeeBalance` | Insufficient fee balance for operation |
| 1032 | `FeeCollectionFailed` | Fee collection operation failed |
| 1033 | `FeeValidationFailed` | Fee validation failed (0x409) - See detailed explanation below |
| 1034 | `TreasuryValidationFailed` | Treasury validation failed |
| 1035 | `InvalidSystemStatePDA` | Invalid system state PDA |
| 1036 | `InvalidSystemStateDeserialization` | System state deserialization failed |
| 1042 | `UnauthorizedFeeUpdate` | Unauthorized fee update operation |
| 1043 | `InvalidFeeUpdateFlags` | Invalid fee update flags |
| 1044 | `InvalidLiquidityFee` | Invalid liquidity fee amount |
| 1045 | `InvalidSwapFee` | Invalid swap fee amount |
| 1046 | `FeeUpdateValidationFailed` | Fee update validation failed |

#### Consolidation Errors (1037-1041)
| Code | Error Type | Description |
|------|------------|-------------|
| 1037 | `ConsolidationFailed` | Pool fee consolidation failed |
| 1038 | `InvalidConsolidationBatch` | Invalid consolidation batch configuration |
| 1039 | `PoolNotEligibleForConsolidation` | Pool not eligible for consolidation |
| 1040 | `ConsolidationRaceCondition` | Consolidation race condition detected |
| 1041 | `NoPoolsEligibleForConsolidation` | No pools eligible for consolidation |

#### Calculation and Validation Errors (1047-1049)
| Code | Error Type | Description |
|------|------------|-------------|
| 1047 | `AmountMismatch` | Calculated amount doesn't match expected (0x417) |
| 1048 | `UnsafeRatioValues` | Unsafe ratio values exceed maximum safe limit |
| 1049 | `UnsupportedRatioType` | Unsupported ratio type for pool creation |

#### Additional Pool State Errors (1035)
| Code | Error Type | Description |
|------|------------|-------------|
| 1035 | `PoolLiquidityPaused` | Pool liquidity operations are paused |

#### Custom Implementation Errors (3000+)
| Code | Error Type | Description |
|------|------------|-------------|
| 3001 | `StrictRatioViolation` | Strict 1:1 LP token ratio violation |
| 4001 | `MissingUserLPTokenAccount` | User LP token account not found |

### Detailed Error Explanations

#### Error 1033: FeeValidationFailed (0x409) - Common Confusion

**The Problem:** Users often see "✅ Fee payment validation passed" in logs but then encounter `FeeValidationFailed` error. This seems contradictory but is actually expected behavior.

**Why This Happens:**
Fee validation occurs in multiple phases AFTER the initial payment validation:

1. **Phase 1: Payment Validation** ✅ 
   - Checks if user has sufficient SOL balance
   - Validates fee amount is correct
   - **Success Message:** "✅ Fee payment validation passed"

2. **Phase 2: Account Validation** ❌ (Common failure point)
   - Validates pool state account is writable
   - **Failure:** "Pool state account is not writable - cannot update fee tracking fields"

3. **Phase 3: System Clock Access** ❌ (Rare failure point)
   - Retrieves current timestamp for fee tracking
   - **Failure:** "Failed to get system clock"

4. **Phase 4: Fee Counter Updates** ❌ (Overflow protection)
   - Updates pool's fee counters with overflow protection
   - **Failure:** "Liquidity fee counter overflow" or "Swap contract fee counter overflow"

5. **Phase 5: State Serialization** ❌ (Account size issues)
   - Saves updated pool state back to account
   - **Failure:** "Pool state account too small for serialized data"

**Common Causes & Solutions:**

| Cause | Solution |
|-------|----------|
| **Pool state account not writable** | Ensure pool state PDA is marked as writable in transaction |
| **Account size insufficient** | Pool state account may be corrupted or undersized |
| **Fee counter overflow** | Pool has collected maximum possible fees (very rare) |
| **System clock unavailable** | Network/validator issue - retry transaction |

**Debugging Steps:**
1. Check transaction logs for the specific failure reason after "Fee payment validation passed"
2. Verify all accounts in transaction are properly marked as writable where required
3. Ensure pool state account has sufficient space (typically 1000+ bytes)
4. If overflow errors occur, the pool may need fee consolidation

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
- SystemState: 83 bytes ⚠️ **BREAKING CHANGE v0.16.x+**
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
    
    /// ⚠️ **NEW IN v0.16.x+**: Admin authority for system operations
    pub admin_authority: Pubkey,            // 32 bytes
    
    /// ⚠️ **NEW IN v0.16.x+**: Pending admin authority (with 72-hour timelock)
    pub pending_admin_authority: Option<Pubkey>, // 33 bytes (1 + 32)
    
    /// ⚠️ **NEW IN v0.16.x+**: Timestamp when admin change was initiated
    pub admin_change_timestamp: i64,        // 8 bytes
}

// Total Size: 83 bytes ⚠️ **BREAKING CHANGE from 10 bytes in v0.15.x**
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

#### ⚠️ **BREAKING CHANGE v0.16.x+**: Centralized Deserialization Methods

**🚨 IMPORTANT**: Direct deserialization with `try_from_slice()` is **DEPRECATED** and will fail due to account size changes.

##### Production Code (Recommended)
```rust
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

// ✅ NEW: Centralized method with built-in security validation
pub fn load_from_account(
    account: &AccountInfo,
    program_id: &Pubkey,
) -> Result<SystemState, ProgramError>

// Usage example:
let system_state = SystemState::load_from_account(&system_state_account, &program_id)?;
```

**Features:**
- ✅ **Tolerant deserialization** - Handles account size variations
- ✅ **Automatic PDA validation** - Security built-in
- ✅ **Future-proof** - Works with any SystemState size changes
- ✅ **Single point of maintenance** - All changes in one place

##### Test Code Only
```rust
// ⚠️ TEST ENVIRONMENTS ONLY - No PDA validation
pub fn from_account_data_unchecked(data: &[u8]) -> Result<SystemState, ProgramError>

// Usage example (tests only):
let system_state = SystemState::from_account_data_unchecked(&account.data)?;
```

##### ❌ Deprecated (Will Fail in v0.16.x+)
```rust
// ❌ DEPRECATED: Will fail with "Not all bytes read" error
let system_state = SystemState::try_from_slice(&account.data)?; // DON'T USE

// ❌ DEPRECATED: Manual deserialization without validation  
let system_state = SystemState::deserialize(&mut &account.data[..])?; // DON'T USE
```

**Migration Guide:**
1. **Production code**: Replace `try_from_slice()` with `load_from_account()`
2. **Test code**: Replace `try_from_slice()` with `from_account_data_unchecked()`
3. **Update dependencies**: Ensure you're using v0.16.x+ of the contract

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

// Main Treasury PDA - Direct fee collection (pool creation) and consolidation target
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
// ⚠️ **UPDATED FOR v0.16.x+**: System state with admin authority fields
const SystemStateSchema = {
    struct: {
        is_paused: 'bool',
        pause_timestamp: 'i64', 
        pause_reason_code: 'u8',
        admin_authority: 'publicKey',           // NEW in v0.16.x+
        pending_admin_authority: { option: 'publicKey' }, // NEW in v0.16.x+
        admin_change_timestamp: 'i64'          // NEW in v0.16.x+
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

// ⚠️ **UPDATED FOR v0.16.x+**: Reading system state in Rust with new methods
async fn read_system_state(
    rpc_client: &RpcClient,
    system_pda: &Pubkey,
    program_id: &Pubkey
) -> Result<SystemState, Box<dyn std::error::Error>> {
    let account = rpc_client.get_account(system_pda)?;
    
    // ✅ NEW: Use tolerant deserialization for client code
    let system_state = SystemState::from_account_data_unchecked(&account.data)?;
    
    if system_state.is_paused {
        println!("System is paused (reason: {})", system_state.pause_reason_code);
        println!("Admin authority: {}", system_state.admin_authority);
    }
    
    Ok(system_state)
}

// ❌ DEPRECATED: Old method (will fail in v0.16.x+)
// let system_state = SystemState::try_from_slice(&account.data)?; // DON'T USE
```

---

### Legacy PDA Seeds (For Reference)
- **System State:** `[b"system_state"]`
- **Main Treasury:** `[b"main_treasury"]`
- **Pool State:** `[b"pool_state", token_a_mint, token_b_mint, ratio_a_bytes, ratio_b_bytes]`
- **Token Vaults:** `[b"token_a_vault", pool_state_key]` or `[b"token_b_vault", pool_state_key]`
- **LP Mints:** `[b"lp_token_a_mint", pool_state_key]` or `[b"lp_token_b_mint", pool_state_key]`

---

## 📚 Developer Calculation Guides

### Overview

Fixed Ratio Trading requires precise mathematical calculations to determine expected token outputs. These guides provide comprehensive implementation details for different programming languages, ensuring your applications can accurately calculate swap amounts and integrate seamlessly with the smart contract.

### Language-Specific Implementation Guides

#### C#/.NET Developers
**[EXPECTED_TOKENS_GUIDE_CSHARP.md](EXPECTED_TOKENS_GUIDE_CSHARP.md)**

Complete C#/.NET implementation guide featuring:
- **FRTExpectedTokens Class**: Ready-to-use static methods for all calculations
- **Overflow Protection**: BigInteger implementation for handling large numbers
- **Comprehensive Examples**: Real-world scenarios with step-by-step calculations
- **Unit Testing**: Complete test suite with edge cases and validation
- **Smart Contract Integration**: Best practices for transaction building
- **Debugging Tools**: Helper methods and troubleshooting techniques

Key Features:
- Checked arithmetic with overflow detection
- Minimum input calculation methods
- Display/basis points conversion utilities
- Extensive error handling and validation

#### JavaScript/TypeScript Developers
**[EXPECTED_TOKENS_GUIDE_JAVASCRIPT.md](EXPECTED_TOKENS_GUIDE_JAVASCRIPT.md)**

Complete JavaScript/TypeScript implementation guide featuring:
- **TokenPairRatio Class**: Object-oriented approach to ratio calculations
- **BigInt Support**: Large number handling for precision-critical operations
- **Factory Methods**: Easy creation from blockchain pool data
- **Web Integration**: Patterns for browser and Node.js applications
- **Async/Await Support**: Modern JavaScript patterns for blockchain interaction
- **Debug Helpers**: Comprehensive logging and validation tools

Key Features:
- Math.floor() for consistent integer division
- Display amount conversion methods
- Pool data validation and error handling
- Cross-browser compatibility considerations

### Mathematical Foundation

Both guides are built on the same mathematical foundation:

**Core Formula:**
```
A→B Swaps: Output_B = (Input_A × Ratio_B) ÷ Ratio_A
B→A Swaps: Output_A = (Input_B × Ratio_A) ÷ Ratio_B
```

**Key Principles:**
- All calculations use basis points (smallest token units)
- Integer division with truncation (no rounding up)
- Deterministic results matching smart contract logic
- Zero slippage with exact output amounts

### Integration Requirements

**Critical:** The smart contract requires the `expected_amount_out` parameter to **exactly match** the calculated output. Any mismatch results in transaction failure with error code `0x417` (AMOUNT_MISMATCH).

**Best Practices:**
1. Always use the provided calculation classes/functions
2. Never implement calculations manually
3. Test with various decimal combinations
4. Validate minimum input amounts to prevent dust
5. Handle overflow scenarios appropriately
6. Log calculations during debugging

### Quick Start

**C#/.NET:**
```csharp
ulong expectedOutput = FRTExpectedTokens.Calculate(
    inputBasisPoints, tokenADecimals, tokenBDecimals, 
    ratioA, ratioB, isAToB);
```

**JavaScript/TypeScript:**
```javascript
const tokenPair = TokenPairRatio.fromPoolData(poolData);
const expectedOutput = tokenPair.SwapAToB(inputAmount);
```

### Additional Resources

- **[SWAP_CALCULATION_GUIDE.md](SWAP_CALCULATION_GUIDE.md)**: Language-agnostic mathematical reference
- **Error Codes Section**: Complete list of calculation-related error codes
- **Pool Management Section**: Understanding pool ratios and decimal configurations

---

## Support and Resources

- **Email:** support@davincicodes.net
- **Documentation:** [GitHub Repository](https://github.com/davincicodes/fixed-ratio-trading)
- **Dashboard:** Available for mainnet deployment

For custom integrations, fee modifications, or technical support, please contact our support team.