# Account Ordering Policy & Standards

## 📋 **Overview**

This document defines the standardized account ordering for all `process_*` functions in the Fixed-Ratio Trading protocol. Consistent account ordering enables:

- **Predictable Development**: Developers know what to expect at each index
- **Common Helper Functions**: Shared utilities for account array construction
- **Reduced Errors**: Consistent patterns reduce account mismatches
- **Better Testing**: Standardized test helpers across all operations

## 🎯 **Core Principle**

**"Most common accounts appear at the same indices across all functions"**

**PHASE 5 UPDATE**: After Phase 3 centralization, account positions 0-12 are reserved for the most frequently used accounts, with function-specific accounts starting at index 13. This optimization reduces account counts and improves performance.

---

## 📊 **Standardized Account Order (Phase 5: Optimized)**

### **🔧 Base System Accounts (0-3)**
```rust
0. **Authority/User Signer** (signer, writable) - The account authorizing the operation
1. **System Program** (readable) - Core Solana system program  
2. **Rent Sysvar** (readable) - For rent exemption calculations
3. **Clock Sysvar** (readable) - For timestamp operations
```

### **🏊 Pool Core Accounts (4-8)**
```rust
4. **Pool State PDA** (writable) - Main pool state data account
5. **Token A Mint** (readable) - Pool's Token A mint (for PDA seeds)
6. **Token B Mint** (readable) - Pool's Token B mint (for PDA seeds)  
7. **Token A Vault PDA** (writable) - Pool's Token A vault account
8. **Token B Vault PDA** (writable) - Pool's Token B vault account
```

### **💰 Token Operations (9-11)**
```rust
9. **SPL Token Program** (readable) - SPL Token program for token operations
10. **User Input Token Account** (writable) - User's source token account
11. **User Output Token Account** (writable) - User's destination token account
```

### **🏦 Treasury System (12) - PHASE 5 OPTIMIZED**
```rust
12. **Main Treasury PDA** (writable) - Centralized treasury for all fee collection
// REMOVED: Specialized treasury accounts (swap and HFT) - Phase 3 centralization
```

### **⚙️ Function-Specific (13+) - PHASE 5 OPTIMIZED**
```rust
13+ **Function-Specific Accounts** - LP token mints, system state, specialized accounts
```

**PHASE 5 OPTIMIZATION BENEFITS:**
- **Reduced Account Counts**: Most functions use 13-15 accounts instead of 15-17
- **Eliminated Complexity**: No specialized treasury management needed
- **Performance Improvement**: 70-300 CUs saved per transaction
- **Simplified Validation**: Only main treasury needs validation

---

## 🔄 **Account Usage Matrix (Phase 5 Updated)**

| Function | 0-3 (System) | 4-8 (Pool) | 9-11 (Token) | 12 (Treasury) | 13+ (Specific) | Total Accounts |
|----------|--------------|------------|--------------|---------------|----------------|----------------|
| `process_swap` | ✅ (0,1,2,3) | ✅ (4,5,6,7,8) | ✅ (9,10,11) | ✅ (12) | - | **14** |
| `process_swap_hft_optimized` | ✅ (0,1,2,3) | ✅ (4,5,6,7,8) | ✅ (9,10,11) | ✅ (12) | HFT treasury (13) | **14** |
| `process_deposit` | ✅ (0,1,2,3) | ✅ (4,5,6,7,8) | ✅ (9,10,11) | ✅ (12) | LP mints (13,14) | **15** |
| `process_withdraw` | ✅ (0,1,2,3) | ✅ (4,5,6,7,8) | ✅ (9,10,11) | ✅ (12) | LP mints (13,14) | **15** |
| `process_initialize_pool` | ✅ (0,1,2,3) | ✅ (4,5,6,7,8) | ✅ (9) | ✅ (12) | LP mints (13,14) | **15** |
| `process_withdraw_treasury_fees` | ✅ (0,1,2) | ❌ | ❌ | ✅ (12) | Dest (13), System (14) | **15** |
| `process_get_treasury_info` | ✅ (0,1,2,3) | ❌ | ❌ | ✅ (12) | - | **13** |
| `process_pause_system` | ✅ (0) | ❌ | ❌ | ❌ | System state (13) | **14** |
| `process_initialize_program` | ✅ (0,1,2) | ❌ | ❌ | ✅ (12) | System state (13) | **14** |

**Legend:**
- ✅ **Full Usage**: All indices in range used
- ⚠️ **Partial Usage**: Some indices in range used  
- ❌ **Not Used**: Range not applicable to function

**PHASE 5 ACCOUNT COUNT REDUCTIONS:**
- Treasury operations: 17 → 15 accounts (13% reduction)
- Swap operations: 15 → 14 accounts (7% reduction)
- Pool creation: 17 → 15 accounts (12% reduction)
- System operations: 16 → 13-14 accounts (13-19% reduction) 