# Fixed Ratio Trading - Quick Reference Guide

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
| `process_system_initialize` | Program Authority | One-time setup |
| `process_system_pause` | Program Authority | Emergency stop |
| `process_system_unpause` | Program Authority | Resume operations |

### Pool Management
| Function | Authority | Fee | Purpose |
|----------|-----------|-----|---------|
| `process_pool_initialize` | Any User | 1.15 SOL | Create pool |
| `process_pool_pause` | Program Authority | - | Pause pool |
| `process_pool_unpause` | Program Authority | - | Resume pool |
| `process_pool_update_fees` | Program Authority* | - | Update fees |

*Contact support@davincicodes.net for fee modifications

### Liquidity Operations
| Function | Authority | Default Fee | Purpose |
|----------|-----------|-------------|---------|
| `process_liquidity_deposit` | Any User | 0.0013 SOL | Add liquidity |
| `process_liquidity_withdraw` | LP Holder | 0.0013 SOL | Remove liquidity |

### Swap Operations
| Function | Authority | Default Fee | Purpose |
|----------|-----------|-------------|---------|
| `process_swap_execute` | Any User** | 0.00002715 SOL | Execute swap |
| `process_swap_set_owner_only` | Program Authority | - | Restrict swaps |

**Unless owner-only mode is enabled

### Treasury Operations
| Function | Authority | Purpose |
|----------|-----------|---------|
| `process_treasury_withdraw_fees` | Program Authority | Withdraw fees (dynamic rate limiting) |
| `process_treasury_get_info` | Public | View treasury info |
| `process_treasury_donate_sol` | Any User | Support development |
| `process_consolidate_pool_fees` | Public | Collect pool fees |

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

## ⚠️ Important Notes

1. **Basis Points**: All amounts must be in smallest unit
2. **Authority**: Most admin functions require Program Upgrade Authority
3. **Fees**: Collected in SOL, configurable per pool
4. **Pausing**: System pause overrides pool pause
5. **Treasury**: Withdrawals subject to dynamic rate limiting

## 📞 Support

- **Email**: support@davincicodes.net
- **Fee Modifications**: Case-by-case basis
- **Custom Integrations**: Contact for owner-only swap setup
- **Donations**: Help accelerate feature development