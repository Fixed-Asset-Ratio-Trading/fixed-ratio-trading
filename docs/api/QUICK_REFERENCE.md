# Fixed Ratio Trading - Quick Reference Guide

## ⚠️ **BREAKING CHANGES v0.16.x+**

**🚨 IMPORTANT**: SystemState structure and deserialization methods have changed in v0.16.x+

### What Changed
- **SystemState size**: 10 bytes → 83 bytes (added admin authority fields)
- **Deserialization**: `try_from_slice()` → `load_from_account()` or `from_account_data_unchecked()`
- **New fields**: `admin_authority`, `pending_admin_authority`, `admin_change_timestamp`

### Migration Required
```rust
// ❌ OLD (v15.x and below - will fail)
let system_state = SystemState::try_from_slice(&account.data)?;

// ✅ NEW (v0.16.x+ - production code)
let system_state = SystemState::load_from_account(&account, &program_id)?;

// ✅ NEW (v0.16.x+ - test/client code)
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

*v0.16.x+: Uses configurable admin authority (with upgrade authority fallback)

### Pool Management
| Function | Authority | Fee | Purpose |
|----------|-----------|-----|---------|
| `process_pool_initialize` | Any User | 1.15 SOL | Create pool |
| `process_pool_pause` | Program Upgrade Authority | - | Pause pool operations (bitwise flags) |
| `process_pool_unpause` | Program Upgrade Authority | - | Resume pool operations (bitwise flags) |
| `process_pool_update_fees` | Program Upgrade Authority | - | Update fees |

#### Pool Pause Flags
| Flag | Value | Purpose |
|------|-------|---------|
| `PAUSE_FLAG_LIQUIDITY` | 1 | Pause deposits/withdrawals |
| `PAUSE_FLAG_SWAPS` | 2 | Pause swap operations |
| `PAUSE_FLAG_ALL` | 3 | Pause all operations (required for consolidation) |

### Liquidity Operations
| Function | Authority | Default Fee | Purpose |
|----------|-----------|-------------|---------|
| `process_liquidity_deposit` | Any User | 0.0013 SOL | Add liquidity |
| `process_liquidity_withdraw` | LP Holder | 0.0013 SOL | Remove liquidity |

### Swap Operations
| Function | Authority | Default Fee | Purpose |
|----------|-----------|-------------|---------|
| `process_swap_execute` | Any User** | 0.00002715 SOL | Execute swap |
| `process_swap_set_owner_only` | Admin Authority* | - | Restrict swaps |

*v0.16.x+: Uses configurable admin authority (with upgrade authority fallback)
**Unless owner-only mode is enabled

### Treasury Operations
| Function | Authority | Purpose |
|----------|-----------|---------|
| `process_treasury_withdraw_fees` | Admin Authority* | Withdraw fees (dynamic rate limiting) |
| `process_treasury_get_info` | Public | View treasury info |
| `process_treasury_donate_sol` | Any User | Support development |
| `process_consolidate_pool_fees` | Admin Authority* | Collect pool fees (security update) |

*v0.16.x+: Uses configurable admin authority (with upgrade authority fallback)

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
        Buffer.from("pool_state_v2"),
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

// 2. Build instruction
const ix = createInitializePoolInstruction(
    ratioA,
    ratioB,
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

// Create pause instruction (requires Program Upgrade Authority)
const pauseInstruction = createPausePoolInstruction(
    programUpgradeAuthority,
    poolStatePDA,
    programDataAccount,
    PAUSE_FLAG_ALL  // Pause all operations
);

// Create unpause instruction
const unpauseInstruction = createUnpausePoolInstruction(
    programUpgradeAuthority,
    poolStatePDA,
    programDataAccount,
    PAUSE_FLAG_ALL  // Unpause all operations
);
```

## ⚠️ Important Notes

1. **Basis Points**: All amounts must be in smallest unit
2. **Authority v0.16.x+**: Most admin functions use configurable Admin Authority (with upgrade authority fallback)
3. **Fees**: Collected in SOL, configurable per pool
4. **Pausing**: System pause overrides pool pause
5. **Treasury**: Withdrawals subject to dynamic rate limiting
6. **Breaking Changes**: v0.16.x+ requires new SystemState deserialization methods
7. **Migration**: Update client code to use `load_from_account()` or `from_account_data_unchecked()`

## 📞 Support

- **Email**: support@davincicodes.net
- **Fee Modifications**: Case-by-case basis
- **Custom Integrations**: Contact for owner-only swap setup
- **Donations**: Help accelerate feature development