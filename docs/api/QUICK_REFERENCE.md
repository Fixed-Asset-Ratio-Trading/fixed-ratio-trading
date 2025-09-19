# Fixed Ratio Trading - Quick Reference Guide

## 📋 **Current API Implementation**

---Ad
**GitKracken** https://gitkraken.cello.so/pk9L5rp5jln visual Git helps you see it all clearly!
---


### SystemState Implementation
- **SystemState size**: 83 bytes (includes admin authority fields)
- **Deserialization**: Use `load_from_account()` or `from_account_data_unchecked()`
- **Fields**: `admin_authority`, `pending_admin_authority`, `admin_change_timestamp`

### Current Usage
```rust
// ✅ Production code
let system_state = SystemState::load_from_account(&account, &program_id)?;

// ✅ Test/client code
let system_state = SystemState::from_account_data_unchecked(&account.data)?;
```

---

## 🚀 Quick Start

```javascript
// Program ID
const PROGRAM_ID = new PublicKey("4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn");

// Important: ALL VALUES IN BASIS POINTS!
// 1 SOL = 1_000_000_000 (9 decimals)
// 1 USDC = 1_000_000 (6 decimals)
```

## 📋 Function Quick Reference

### System Management
| Function | Authority | Purpose |
|----------|-----------|---------|
| `process_system_initialize` | Program Authority | One-time setup (sets admin authority) |
| `process_system_pause` | Admin Authority* | Emergency stop |
| `process_system_unpause` | Admin Authority* | Resume operations |
| `process_admin_change` | Admin Authority* | Change admin (72h timelock) |

*Uses configurable admin authority (with upgrade authority fallback)

### Pool Management
| Function | Authority | Fee | Purpose |
|----------|-----------|-----|---------|
| `process_pool_initialize` | Any User | 1.15 SOL | Create pool |
| `process_pool_pause` | Admin Authority* | - | Pause pool operations (bitwise flags) |
| `process_pool_unpause` | Admin Authority* | - | Resume pool operations (bitwise flags) |
| `process_pool_update_fees` | Admin Authority* | - | Update fees |

*Admin Authority (with Program Upgrade Authority fallback)

#### Pool Creation Flags
| Flag | Bit | Value | Purpose |
|------|-----|-------|---------|
| Owner-only swaps | 5 | 32 | Only pool creator can swap |
| Exact exchange required | 6 | 64 | Reject swaps with precision loss |

**Note**: Only bits 5 and 6 can be set during pool creation. Other flags are admin-controlled.

#### Pool Pause Flags
| Flag | Value | Purpose |
|------|-------|---------|
| `PAUSE_FLAG_LIQUIDITY` | 1 | Pause deposits/withdrawals |
| `PAUSE_FLAG_SWAPS` | 2 | Pause swap operations |
| `PAUSE_FLAG_ALL` | 3 | Pause all operations (required for consolidation) |

### Liquidity Operations
| Function | Authority | Default Fee | Purpose |
|----------|-----------|-------------|---------|
| `process_liquidity_deposit` | Any User | 0.013 SOL | Add liquidity |
| `process_liquidity_withdraw` | LP Holder | 0.013 SOL | Remove liquidity |

### Swap Operations
| Function | Authority | Default Fee | Purpose |
|----------|-----------|-------------|---------|
| `process_swap_execute` | Any User** | 0.0002715 SOL | Execute swap |
| `process_swap_set_owner_only` | Admin Authority* | - | Restrict swaps |

*Admin Authority (with Program Upgrade Authority fallback)
**Unless owner-only mode is enabled

### Treasury Operations
| Function | Authority | Purpose |
|----------|-----------|---------|
| `process_treasury_withdraw_fees` | Admin Authority* | Withdraw fees (60-min cooldown after success) |
| `process_treasury_get_info` | Public | View treasury info |
| `process_treasury_donate_sol` | Any User | Support development |
| `process_consolidate_pool_fees` | Admin Authority* | Collect pool fees (security update) |

*Uses configurable admin authority (with upgrade authority fallback)

## 🔑 Common PDA Derivations

```javascript
// System State
const [systemStatePDA] = PublicKey.findProgramAddress(
    [Buffer.from("system_state")],
    PROGRAM_ID
);

// Main Treasury
const [mainTreasuryPDA] = PublicKey.findProgramAddress(
    [Buffer.from("main_treasury")],
    PROGRAM_ID
);

// Pool State
const [poolStatePDA] = PublicKey.findProgramAddress(
    [
        Buffer.from("pool_state"),
        tokenAMint.toBuffer(),
        tokenBMint.toBuffer(),
        new BN(ratioA).toArrayLike(Buffer, 'le', 8),
        new BN(ratioB).toArrayLike(Buffer, 'le', 8)
    ],
    PROGRAM_ID
);

// Token Vaults
const [tokenAVaultPDA] = PublicKey.findProgramAddress(
    [Buffer.from("token_a_vault"), poolStatePDA.toBuffer()],
    PROGRAM_ID
);

// LP Token Mints
const [lpTokenAMintPDA] = PublicKey.findProgramAddress(
    [Buffer.from("lp_token_a_mint"), poolStatePDA.toBuffer()],
    PROGRAM_ID
);
```

## 💡 Common Patterns

### Creating a Pool
```javascript
// 1. Convert ratios to basis points
const ratioA = 1.0 * Math.pow(10, tokenADecimals);
const ratioB = 160.0 * Math.pow(10, tokenBDecimals);

// 2. Set pool flags (optional)
const flags = 0; // Standard pool (default)
// const flags = 32; // Owner-only swaps
// const flags = 64; // Exact exchange required (no dust loss)
// const flags = 96; // Both owner-only and exact exchange

// 3. Build instruction
const ix = createInitializePoolInstruction(
    ratioA,
    ratioB,
    flags,
    accounts
);
```

### Depositing Liquidity
```javascript
// Calculate required amounts based on pool ratio
const depositAmount = userAmount; // in basis points
const requiredOtherToken = (depositAmount * ratioB) / ratioA;
```

### Executing a Swap
```javascript
// Calculate expected output
const inputAmount = 1_000_000_000; // 1 SOL
const expectedOutput = (inputAmount * outputRatio) / inputRatio;
const minOutput = expectedOutput * 0.99; // 1% slippage tolerance
```

### Pausing Pool Operations
```javascript
// Pause flag constants
const PAUSE_FLAG_LIQUIDITY = 1;  // Pause deposits/withdrawals
const PAUSE_FLAG_SWAPS = 2;      // Pause swaps
const PAUSE_FLAG_ALL = 3;        // Pause all operations

// Create pause instruction (requires Admin Authority)
const pauseInstruction = createPausePoolInstruction(
    adminAuthority,
    poolStatePDA,
    programDataAccount,
    PAUSE_FLAG_ALL  // Pause all operations
);

// Create unpause instruction
const unpauseInstruction = createUnpausePoolInstruction(
    adminAuthority,
    poolStatePDA,
    programDataAccount,
    PAUSE_FLAG_ALL  // Unpause all operations
);
```

## ⚠️ Important Notes

1. **Basis Points**: All amounts must be in smallest unit
2. **Authority**: Most admin functions use configurable Admin Authority (with upgrade authority fallback)
3. **Fees**: Collected in SOL, configurable per pool
4. **Pausing**: System pause overrides pool pause
5. **Treasury**: Withdrawals subject to fixed 60-minute cooldown after success
6. **SystemState**: Use `load_from_account()` or `from_account_data_unchecked()` for deserialization

## 📞 Support

- **Email**: support@davincicodes.net
- **Fee Modifications**: Case-by-case basis
- **Custom Integrations**: Contact for owner-only swap setup
- **Donations**: Help accelerate feature development