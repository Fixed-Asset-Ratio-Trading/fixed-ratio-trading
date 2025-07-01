# Migration Guide

This document provides guidance for migrating from legacy patterns to modern implementations and maintaining backward compatibility.

## Table of Contents
- [Single-Instruction Pool Initialization](#single-instruction-pool-initialization)
- [Client SDK Migration](#client-sdk-migration)
- [Legacy Pattern Support](#legacy-pattern-support)
- [Migration Strategies](#migration-strategies)
- [Compatibility Matrix](#compatibility-matrix)

## Single-Instruction Pool Initialization

### **NEW: Single-Instruction Pool Initialization**
We've implemented a major improvement that replaces the deprecated two-instruction pattern with a modern single-instruction approach:

#### **Before (Deprecated):**
```rust
// Required 2 separate transactions
let create_ix = PoolInstruction::CreatePoolStateAccount { /* ... */ };
let init_ix = PoolInstruction::InitializePoolData { /* ... */ };
// Send transaction 1, then transaction 2
```

#### **After (Recommended):**
```rust
// Single atomic transaction
let init_ix = PoolInstruction::InitializePool { /* ... */ };
// Send single transaction ✅
```

**Benefits:**
- ✅ **Atomic Operation**: All-or-nothing execution prevents partial states
- ✅ **Simpler Integration**: Single instruction call vs. two separate transactions  
- ✅ **Better UX**: Reduces transaction costs and complexity for users
- ✅ **Eliminates Race Conditions**: No possibility of partial pool creation
- ✅ **Future-Proof**: Uses modern Solana best practices

## Client SDK Migration

### **NEW: Client SDK**
Added a comprehensive client SDK that simplifies integration:

```rust
use fixed_ratio_trading::client_sdk::*;

// Create pool client
let pool_client = PoolClient::new(program_id);

// Configure pool
let pool_config = PoolConfig::new(usdc_mint, sol_mint, 1000)?; // 1000:1 ratio

// Get pool creation instruction (single atomic operation)
let create_ix = pool_client.create_pool_instruction(&payer, &pool_config, &lp_a_mint, &lp_b_mint)?;

// Add liquidity 
let deposit_ix = pool_client.deposit_instruction(&user, &pool_config, &usdc_mint, 1000000, &user_source, &user_lp)?;
```

**SDK Features:**
- 🔧 **Automatic PDA Derivation**: No manual address calculation needed
- 🔧 **Account Preparation**: Automatically prepares all required accounts
- 🔧 **Type Safety**: Strongly typed interfaces prevent common mistakes
- 🔧 **Error Handling**: Clear error messages and validation
- 🔧 **Testing Support**: Built-in utilities for testing and debugging

### **NEW: Enhanced Deposit Instructions**
Added `DepositWithFeatures` for advanced use cases:

```rust
let enhanced_deposit_ix = pool_client.deposit_with_features_instruction(
    &user,
    &pool_config,
    &token_mint,
    amount,
    minimum_lp_tokens_out, // Slippage protection
    custom_fee_recipient,   // Optional custom fee recipient
    &user_source,
    &user_lp,
)?;
```

**Features:**
- 🛡️ **Slippage Protection**: Minimum LP token guarantees
- 💰 **Custom Fee Recipients**: Flexible fee distribution
- 🔍 **Enhanced Validation**: Additional checks and error handling

### **NEW: PDA Helper Utilities**
Simplified PDA derivation with helper instructions:

```rust
// Get pool state PDA
let get_pda_ix = PoolInstruction::GetPoolStatePDA {
    multiple_token_mint: usdc_mint,
    base_token_mint: sol_mint,
    multiple_per_base: 1000,
};

// Get token vault PDAs
let get_vaults_ix = PoolInstruction::GetTokenVaultPDAs {
    pool_state_pda: pool_pda,
};
```

### **NEW: Test-Specific View Instructions**
Easy access to pool state data for testing and debugging:

```rust
// Get comprehensive pool information
let pool_info_ix = PoolInstruction::GetPoolInfo;

// Get liquidity information
let liquidity_ix = PoolInstruction::GetLiquidityInfo;

// Get fee information
let fee_ix = PoolInstruction::GetFeeInfo;
```

**Provides:**
- 📊 **Pool State**: Comprehensive pool configuration data
- 💧 **Liquidity Info**: Token balances, exchange rates, TVL
- 💸 **Fee Info**: Fee rates, collected fees, available balances

## Legacy Pattern Support

### Backward Compatibility

The legacy two-instruction pattern is still supported but marked as deprecated:

```rust
// ⚠️ DEPRECATED - Still works but not recommended
PoolInstruction::CreatePoolStateAccount { /* ... */ }
PoolInstruction::InitializePoolData { /* ... */ }

// ✅ RECOMMENDED - Use this instead
PoolInstruction::InitializePool { /* ... */ }
```

### Legacy Instruction Support

#### CreatePoolStateAccount (Deprecated)
```rust
pub enum PoolInstruction {
    #[deprecated(since = "1.1.0", note = "Use InitializePool instead")]
    CreatePoolStateAccount {
        multiple_per_base: u64,
        pool_authority_bump_seed: u8,
        multiple_token_vault_bump_seed: u8,
        base_token_vault_bump_seed: u8,
    },
    // ... other instructions
}
```

#### InitializePoolData (Deprecated)
```rust
pub enum PoolInstruction {
    #[deprecated(since = "1.1.0", note = "Use InitializePool instead")]
    InitializePoolData {
        lp_token_mint_a_bump_seed: u8,
        lp_token_mint_b_bump_seed: u8,
    },
    // ... other instructions
}
```

### Legacy Error Handling

Legacy error codes are maintained for backward compatibility:

```rust
pub enum PoolError {
    #[deprecated]
    LegacyAccountCreationFailed,
    #[deprecated]
    LegacyInitializationFailed,
    // ... modern error codes
}
```

## Migration Strategies

### For New Projects
Use the new single-instruction pattern and client SDK:

```rust
// 1. Use the client SDK
use fixed_ratio_trading::client_sdk::*;

// 2. Create pool with single instruction
let pool_client = PoolClient::new(program_id);
let config = PoolConfig::new(multiple_mint, base_mint, ratio)?;
let create_ix = pool_client.create_pool_instruction(&payer, &config, &lp_a, &lp_b)?;

// 3. Use helper utilities
let deposit_ix = pool_client.deposit_instruction(/* ... */)?;
```

### For Existing Projects

#### Option 1: Gradual Migration
You can migrate incrementally:

1. **Immediate**: Existing code continues to work with deprecated instructions
2. **Gradual**: Replace two-instruction calls with single-instruction calls
3. **Full**: Adopt client SDK for new features

#### Option 2: Direct Migration

1. **Replace Instruction Patterns**:
```rust
// Old pattern
let create_ix = PoolInstruction::CreatePoolStateAccount { /* ... */ };
let init_ix = PoolInstruction::InitializePoolData { /* ... */ };

// New pattern
let init_ix = PoolInstruction::InitializePool { /* ... */ };
```

2. **Adopt Client SDK**:
```rust
// Old manual PDA derivation
let (pool_pda, bump) = Pubkey::find_program_address(seeds, &program_id);

// New SDK approach
let pool_client = PoolClient::new(program_id);
let addresses = pool_client.derive_pool_addresses(&config);
```

3. **Update Error Handling**:
```rust
// Old error handling
match error {
    PoolError::LegacyAccountCreationFailed => { /* ... */ },
    // ...
}

// New error handling  
match error {
    PoolError::InvalidPoolConfiguration => { /* ... */ },
    // ...
}
```

### Testing Migration

Run both legacy and modern patterns in tests:

```bash
# Run all tests including legacy compatibility
cargo test

# Run only new pattern tests
cargo test test_initialize_pool_new_pattern
cargo test test_helper_functions_new_pattern

# Run legacy pattern tests
cargo test test_legacy_pool_creation
cargo test test_backward_compatibility

# Run with output
cargo test -- --nocapture
```

### Migration Checklist

#### Before Migration
- [ ] Review current integration points
- [ ] Identify dependencies on legacy patterns
- [ ] Plan migration timeline
- [ ] Backup current implementation

#### During Migration
- [ ] Update instruction calls to single-instruction pattern
- [ ] Integrate client SDK
- [ ] Update PDA derivation logic
- [ ] Update error handling
- [ ] Update tests

#### After Migration
- [ ] Test all functionality thoroughly
- [ ] Verify performance improvements
- [ ] Update documentation
- [ ] Remove deprecated code references

## Compatibility Matrix

### Instruction Compatibility

| Instruction | Legacy Support | New Pattern | Recommended |
|-------------|----------------|-------------|-------------|
| CreatePoolStateAccount | ✅ Supported | ❌ N/A | ❌ Deprecated |
| InitializePoolData | ✅ Supported | ❌ N/A | ❌ Deprecated |
| InitializePool | ❌ N/A | ✅ Supported | ✅ Recommended |
| Deposit | ✅ Supported | ✅ Enhanced | ✅ Both |
| Withdraw | ✅ Supported | ✅ Enhanced | ✅ Both |
| Swap | ✅ Supported | ✅ Enhanced | ✅ Both |

### Client Integration

| Feature | Legacy API | Client SDK | Notes |
|---------|------------|------------|-------|
| Manual PDA Derivation | ✅ Required | ✅ Optional | SDK handles automatically |
| Account Preparation | ✅ Manual | ✅ Automatic | SDK simplifies integration |
| Error Handling | ✅ Basic | ✅ Enhanced | Better error messages |
| Type Safety | ❌ Limited | ✅ Full | Strongly typed interfaces |
| Testing Utilities | ❌ None | ✅ Built-in | Helper functions included |

### Version Compatibility

| Version | Legacy Pattern | New Pattern | Client SDK |
|---------|----------------|-------------|------------|
| 1.0.x | ✅ Primary | ❌ N/A | ❌ N/A |
| 1.1.x | ⚠️ Deprecated | ✅ Primary | ✅ Available |
| 1.2.x+ | ⚠️ Deprecated | ✅ Primary | ✅ Enhanced |

### Performance Comparison

| Metric | Legacy Pattern | New Pattern | Improvement |
|--------|----------------|-------------|-------------|
| Transactions Required | 2 | 1 | 50% reduction |
| Gas Cost | ~0.002 SOL | ~0.001 SOL | 50% reduction |
| Complexity | High | Low | Simplified |
| Error Risk | Medium | Low | Atomic operations |
| Integration Time | Hours | Minutes | SDK automation |

## Support and Troubleshooting

### Common Migration Issues

1. **PDA Derivation Changes**
   - **Issue**: Manual PDA derivation no longer matches
   - **Solution**: Use client SDK or update derivation logic

2. **Account Structure Changes**
   - **Issue**: Account layout expectations
   - **Solution**: Use latest account structures and validation

3. **Error Code Changes**
   - **Issue**: Different error codes returned
   - **Solution**: Update error handling to use new error codes

### Getting Help

- 📖 **Documentation**: Check updated documentation in `/docs`
- 🧪 **Examples**: Review test files for migration patterns
- 🐛 **Issues**: Report migration issues on GitHub
- 💬 **Community**: Ask for help in Discord

### Migration Timeline

We recommend the following migration timeline:

- **Phase 1 (Immediate)**: Start using new pattern for new features
- **Phase 2 (1-2 months)**: Migrate existing critical functionality
- **Phase 3 (3-6 months)**: Complete migration and remove legacy code
- **Phase 4 (6+ months)**: Full client SDK adoption 