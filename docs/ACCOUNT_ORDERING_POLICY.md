# Account Ordering Policy & Standards

## 📋 **Overview**

This document defines the standardized account ordering for all `process_*` functions in the Fixed-Ratio Trading protocol. Consistent account ordering enables:

- **Predictable Development**: Developers know what to expect at each index
- **Common Helper Functions**: Shared utilities for account array construction
- **Reduced Errors**: Consistent patterns reduce account mismatches
- **Better Testing**: Standardized test helpers across all operations

## 🎯 **Core Principle**

**"Most common accounts appear at the same indices across all functions"**

**PHASE 6 UPDATE**: Ultra-optimized account structure with eliminated rent checks and redundant token mint accounts. Swap functions now require only 10 accounts (down from 14), providing a 29% reduction in account overhead while maintaining all essential security features.

---

## 📊 **Account Ordering by Function Type**

### **🔄 SWAP FUNCTIONS (Phase 6: Ultra-Optimized)**

**Optimized Account Order for `process_swap` and `process_swap_hft_optimized`:**

```rust
0. **Authority/User Signer** (signer, writable) - User authorizing the swap
1. **System Program** (readable) - Solana system program  
2. **Pool State PDA** (writable) - Pool state account
3. **Token A Vault PDA** (writable) - Pool's Token A vault
4. **Token B Vault PDA** (writable) - Pool's Token B vault
5. **SPL Token Program** (readable) - Token program
6. **User Input Token Account** (writable) - User's input token account
7. **User Output Token Account** (writable) - User's output token account
8. **Main Treasury PDA** (writable) - For fee collection
9. **[Function-specific accounts]** - Additional accounts as needed
```

**PHASE 6 SWAP OPTIMIZATIONS:**
- **Eliminated**: Rent and clock sysvar accounts (indices 2-3 removed)
- **Eliminated**: Token A and B mint accounts (indices 5-6 removed)
- **Eliminated**: All rent exemption checks (~500-850 CU savings)
- **Result**: 14 → 10 accounts (29% reduction)
- **Total CU Savings**: ~570-990 CUs per swap (5-8% additional improvement)

### **🏊 STANDARD FUNCTIONS (Phase 5: Optimized)**

**Standard Account Order for liquidity, treasury, and system functions:**

```rust
0. **Authority/User Signer** (signer, writable) - The account authorizing the operation
1. **System Program** (readable) - Core Solana system program  
2. **Rent Sysvar** (readable) - For rent exemption calculations
3. **Clock Sysvar** (readable) - For timestamp operations
4. **Pool State PDA** (writable) - Main pool state data account
5. **Token A Mint** (readable) - Pool's Token A mint (for PDA seeds)
6. **Token B Mint** (readable) - Pool's Token B mint (for PDA seeds)  
7. **Token A Vault PDA** (writable) - Pool's Token A vault account
8. **Token B Vault PDA** (writable) - Pool's Token B vault account
9. **SPL Token Program** (readable) - SPL Token program for token operations
10. **User Input Token Account** (writable) - User's source token account
11. **User Output Token Account** (writable) - User's destination token account
12. **Main Treasury PDA** (writable) - Centralized treasury for all fee collection
13+ **Function-Specific Accounts** - LP token mints, system state, specialized accounts
```

### **Treasury Functions (Phase 7: Ultra-Optimized)**

#### **Treasury Fee Withdrawal** (6 accounts) - 60% reduction from 15 accounts
```
0. Authority/User Signer (signer, writable) - System authority
1. System Program (readable) - Solana system program  
2. Rent Sysvar (readable) - For rent calculations
3. Main Treasury PDA (writable) - Treasury account for withdrawal
4. Destination Account (writable) - Account receiving withdrawn SOL
5. System State Account (readable) - For authority validation
```

#### **Treasury Information Query** (1 account) - 92% reduction from 13 accounts
```
0. Main Treasury PDA (readable) - Treasury account for info query
```

**Phase 7 Treasury Benefits:**
- **Treasury Withdrawal**: 15 → 6 accounts (60% reduction, ~210-420 CU savings)
- **Treasury Info**: 13 → 1 account (92% reduction, ~420-840 CU savings)  
- **Eliminated**: All placeholder accounts that were unused in treasury operations
- **Simplified**: Client integration with minimal account requirements
- **Maximum Efficiency**: Read-only treasury info requires only the treasury account

---

## 🔄 **Account Usage Matrix (Phase 6 Updated)**

| Function | Accounts | Optimizations | Total Count |
|----------|----------|---------------|-------------|
| `process_swap` | 0,1,2,3,4,5,6,7,8 | **Phase 6: Ultra-optimized** | **10** |
| `process_swap_hft_optimized` | 0,1,2,3,4,5,6,7,8 | **Phase 6: Ultra-optimized** | **10** |
| `process_deposit` | 0,1,2,3,4,5,6,7,8,9,10,11,12,13,14 | Phase 5: Optimized | **15** |
| `process_withdraw` | 0,1,2,3,4,5,6,7,8,9,10,11,12,13,14 | Phase 5: Optimized | **15** |
| `process_initialize_pool` | 0,1,2,3,4,5,6,7,8,9,12,13,14 | Phase 5: Optimized | **15** |
| `process_withdraw_treasury_fees` | 0,1,2,3,12,13,14 | Phase 5: Optimized | **15** |
| `process_get_treasury_info` | 0,1,2,3,12 | Phase 5: Optimized | **13** |
| `process_pause_system` | 0,13 | Phase 5: Optimized | **14** |
| `process_initialize_program` | 0,1,2,12,13 | Phase 5: Optimized | **14** |

## 📈 **Performance Benefits Summary**

### **PHASE 6: SWAP ULTRA-OPTIMIZATION**
- **Account Reduction**: 14 → 10 accounts (29% reduction)
- **Rent Check Elimination**: ~500-850 CU savings per swap
- **Token Mint Removal**: ~50-100 CU savings per swap
- **Sysvar Elimination**: ~50-100 CU savings per swap
- **Total CU Savings**: ~600-1,050 CUs per swap (5-8% improvement)
- **Cumulative Improvement**: 30-35% total CU reduction vs original

### **PHASE 5: GENERAL OPTIMIZATIONS**
- **Treasury Centralization**: 13-19% account reduction across functions
- **Specialized Account Removal**: Eliminated unused treasury accounts
- **Validation Simplification**: Single treasury validation path

## 🔒 **Security Rationale for Phase 6**

**Why Rent Checks Can Be Safely Eliminated:**

1. **Structural Protection**: Pool accounts are created rent-exempt and have no lamport withdrawal mechanisms
2. **No Rent Loss Scenarios**: Protocol design prevents any operations that could drain account lamports
3. **Account Immutability**: Pool account sizes are fixed, preventing rent requirement changes
4. **Program-Only Access**: Only the program can modify pool accounts, preventing external manipulation

**Why Token Mint Accounts Can Be Eliminated:**

1. **Pool State Contains Mints**: Token mint addresses are stored in pool state data
2. **PDA Seed Generation**: Uses pool state data, not separate account references
3. **Validation Redundancy**: Token mint validation is performed via user token accounts
4. **No Direct Mint Operations**: Swap functions don't directly interact with mint accounts

## 🎯 **Implementation Guidelines**

### **For Swap Operations (Phase 6)**
- Use ultra-optimized 10-account structure
- Token mints derived from pool state data
- No rent or clock sysvar accounts needed
- Focus on essential validation only

### **For Other Operations (Phase 5)**
- Use standard 13-15 account structure
- Include rent/clock sysvars for operations requiring them
- Use centralized treasury (index 12) for all fee collection
- Function-specific accounts start at index 13

### **Client Integration**
- Swap clients: Use 10-account arrays
- Other clients: Use 13-15 account arrays as appropriate
- Account builders: Provide separate helpers for swap vs standard operations
- Testing: Update test helpers to match new account structures 