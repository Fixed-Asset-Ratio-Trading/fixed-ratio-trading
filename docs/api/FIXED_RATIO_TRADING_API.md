# Fixed Ratio Trading Contract API Documentation

**Version:** 1.0  
**Date:** December 2024  
**Program ID:** `4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn`  
**Support:** support@davincicodes.net

## Table of Contents
1. [Overview](#overview)
2. [Important Notes](#important-notes)
3. [System Management](#system-management)
4. [Pool Management](#pool-management)
5. [Liquidity Operations](#liquidity-operations)
6. [Swap Operations](#swap-operations)
7. [Treasury Operations](#treasury-operations)
8. [Error Codes](#error-codes)
9. [Types and Structures](#types-and-structures)

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

---

## System Management

Functions for system-wide operations and program initialization.

### `process_system_initialize`

Initializes the program's system state and main treasury.

**Authority:** Program Upgrade Authority only  
**One-time operation:** Can only be called once

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

Pauses all system operations globally.

**Authority:** Program Upgrade Authority only  
**Effect:** Blocks all operations except read-only functions

#### Parameters
```rust
program_id: &Pubkey
reason_code: u8        // Pause reason code (for tracking)
accounts: &[AccountInfo; 3]
```

#### Account Structure
| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | System Authority | Signer, Writable | Must be program upgrade authority |
| 1 | System State PDA | Writable | System state to update |
| 2 | Program Data Account | Readable | For authority validation |

#### Reason Codes
- `1`: Emergency security pause
- `2`: Scheduled maintenance
- `3`: Contract upgrade preparation
- `4+`: Custom reasons

---

### `process_system_unpause`

Resumes all system operations after a pause.

**Authority:** Program Upgrade Authority only  
**Effect:** Applies 3-day treasury withdrawal penalty

#### Parameters
```rust
program_id: &Pubkey
accounts: &[AccountInfo; 4]
```

#### Account Structure
| Index | Account | Type | Description |
|-------|---------|------|-------------|
| 0 | System Authority | Signer, Writable | Must be program upgrade authority |
| 1 | System State PDA | Writable | System state to update |
| 2 | Main Treasury PDA | Writable | To apply restart penalty |
| 3 | Program Data Account | Readable | For authority validation |

#### Important Notes
- Treasury withdrawals blocked for 72 hours after unpause
- Pool-specific pauses remain in effect

---

## Pool Management

Functions for creating and managing trading pools.

### `process_pool_initialize`

Creates a new fixed-ratio trading pool.

**Authority:** Any user  
**Fee:** 0.05 SOL registration fee

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
```javascript
// User wants: "1.0 SOL = 160.0 USDT"
const solDecimals = 9;
const usdtDecimals = 6;

const ratioABasisPoints = 1.0 * Math.pow(10, solDecimals);   // 1,000,000,000
const ratioBBasisPoints = 160.0 * Math.pow(10, usdtDecimals); // 160,000,000
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
- **Liquidity Fee:** 0.001 - 0.01 SOL
- **Swap Fee:** 0.0001 - 0.001 SOL

---

## Liquidity Operations

Functions for adding and removing liquidity from pools.

### `process_liquidity_deposit`

Adds liquidity to a pool and mints LP tokens.

**Authority:** Any user  
**Fee:** Configurable per pool (default 0.003 SOL)

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
- Must deposit in pool's fixed ratio
- Both tokens required for initial deposit
- LP tokens represent proportional pool ownership

---

### `process_liquidity_withdraw`

Removes liquidity by burning LP tokens.

**Authority:** LP token holder  
**Fee:** Configurable per pool (default 0.003 SOL)

#### Parameters
```rust
program_id: &Pubkey
lp_amount_to_burn: u64        // LP tokens to burn
withdraw_token_mint: Pubkey   // Which token to receive
accounts: &[AccountInfo; 11]
```

#### Returns
- Proportional share of both pool tokens
- Fees deducted from withdrawal

---

## Swap Operations

Functions for executing token swaps.

### `process_swap_execute`

Executes a fixed-ratio token swap.

**Authority:** Any user (unless owner-only mode)  
**Fee:** Configurable per pool (default 0.0003 SOL)

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

Configures owner-only swap restrictions.

**Authority:** Program Upgrade Authority only  
**Purpose:** Enables custom wrapper contracts

**Note:** If you want to create wrapper functions for `process_swap_execute` with custom rules or additional fees, you can request this flag be set. This allows only your designated account to call swaps through a future governance contract. For more information, contact support@davincicodes.net.

#### Parameters
```rust
program_id: &Pubkey
enable_restriction: bool    // Enable/disable restriction
designated_owner: Pubkey    // Authorized swap caller
accounts: &[AccountInfo; 4]
```

---

## Treasury Operations

Functions for managing protocol treasury and fees.

### `process_treasury_withdraw_fees`

Withdraws collected fees from the main treasury.

**Authority:** Program Upgrade Authority only  
**Restrictions:** Limited to 1st-3rd of each month (GMT)

#### Parameters
```rust
program_id: &Pubkey
amount: u64    // Amount to withdraw (lamports)
accounts: &[AccountInfo; 5]
```

#### Withdrawal Windows
- **Regular:** 1st-3rd of each month (GMT)
- **Blocked:** 72 hours after system unpause
- **Emergency:** Contact support for exceptions

---

### `process_treasury_get_info`

Retrieves current treasury state information.

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
**Minimum:** 0.001 SOL

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

---

### `process_treasury_consolidate_fees`

Consolidates fees from multiple pools to the treasury.

**Authority:** Program Upgrade Authority only  
**Requirement:** Pools must be fully paused

#### Parameters
```rust
program_id: &Pubkey
pool_count: u8    // Number of pools (max 10)
accounts: &[AccountInfo]    // Dynamic based on pool_count
```

#### Account Structure
- Index 0: Authority (Signer)
- Index 1: System State PDA
- Index 2: Main Treasury PDA
- Index 3+: Pool State PDAs (3 per pool)

---

## Error Codes

Common error codes returned by the contract:

| Code | Name | Description |
|------|------|-------------|
| 6000 | `InvalidInstruction` | Unknown instruction |
| 6001 | `InvalidAccountData` | Account data validation failed |
| 6002 | `Unauthorized` | Insufficient authority |
| 6003 | `InsufficientFunds` | Insufficient balance |
| 6004 | `PoolNotFound` | Pool doesn't exist |
| 6005 | `PoolPaused` | Pool operations paused |
| 6006 | `SystemPaused` | System-wide pause active |
| 6007 | `InvalidRatio` | Invalid pool ratio |
| 6008 | `SlippageExceeded` | Output less than expected |
| 6009 | `InvalidFeeAmount` | Fee outside valid range |

---

## Types and Structures

### Pool State
```rust
pub struct PoolState {
    pub owner: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub token_a_vault: Pubkey,
    pub token_b_vault: Pubkey,
    pub lp_token_a_mint: Pubkey,
    pub lp_token_b_mint: Pubkey,
    pub ratio_a_numerator: u64,
    pub ratio_b_denominator: u64,
    pub total_token_a_liquidity: u64,
    pub total_token_b_liquidity: u64,
    pub flags: u8,
    pub contract_liquidity_fee: u64,
    pub swap_contract_fee: u64,
    // ... additional fields
}
```

### PDA Seeds
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