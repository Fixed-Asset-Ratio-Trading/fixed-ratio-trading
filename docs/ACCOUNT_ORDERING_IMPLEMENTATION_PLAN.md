# Account Ordering Implementation Plan

## 🎯 **Current vs. Standardized Account Mapping**

This document provides the detailed implementation plan for migrating all `process_*` functions to use the standardized account ordering defined in `ACCOUNT_ORDERING_POLICY.md`.

---

## 📊 **Current Account Analysis**

### **Summary of Current Inconsistencies:**
- **Signer Position**: Authority/signer appears at index 0 in most functions ✅
- **System Program**: Varies between index 8, 5, 3, 10 ❌
- **Rent Sysvar**: Varies between index 10, 6, 4, 12 ❌
- **Pool State**: Varies between index 1, 2, 3 ❌
- **Token Mints**: Inconsistent positioning and order ❌
- **Treasury Accounts**: When present, inconsistent positioning ❌

---

## 🔄 **Function-by-Function Migration Plan**

### **1. Pool Operations (High Priority)**

#### **A. `process_swap` (Current: 12 accounts)**
**Current Order → Standardized Order:**
```rust
// CURRENT (src/processors/swap.rs:186)
0. user_signer (signer)                    → 0 ✅ Authority/User Signer
1. user_input_token_account (writable)     → 10 ✅ User Input Token Account
2. user_output_token_account (writable)    → 11 ✅ User Output Token Account  
3. pool_state_account (writable)           → 4 ✅ Pool State PDA
4. token_a_mint_for_pda_seeds (readable)   → 5 ✅ Token A Mint
5. token_b_mint_for_pda_seeds (readable)   → 6 ✅ Token B Mint
6. pool_token_a_vault_account (writable)   → 7 ✅ Token A Vault PDA
7. pool_token_b_vault_account (writable)   → 8 ✅ Token B Vault PDA
8. system_program_account (readable)       → 1 ✅ System Program
9. token_program_account (readable)        → 9 ✅ SPL Token Program
10. rent_sysvar_account (readable)         → 2 ✅ Rent Sysvar
11. clock_sysvar_account (readable)        → 3 ✅ Clock Sysvar
// NEW: Add Treasury accounts (currently missing)
12. swap_treasury_account (writable)       → 13 ✅ Swap Treasury PDA (or 12 for regular swaps)
```

**🔧 Changes Required:**
- Reorder existing 12 accounts to match standard positions 0-11
- Add missing treasury account at index 12/13
- Update all internal references to use new indices
- Add treasury fee collection logic

#### **B. `process_swap_hft_optimized` (Current: 12 accounts)**
**Same changes as `process_swap` plus:**
- Add HFT treasury account at index 14
- Maintain CU optimizations with new ordering

#### **C. `process_deposit` (Current: 15 accounts)**
**Current Order → Standardized Order:**
```rust
// CURRENT (src/processors/liquidity.rs:130)
0. user_signer (signer)                    → 0 ✅ Authority/User Signer
1. user_source_token_account (writable)    → 10 ✅ User Input Token Account
2. pool_state_account (writable)           → 4 ✅ Pool State PDA
3. token_a_mint_for_pda_seeds (readable)   → 5 ✅ Token A Mint
4. token_b_mint_for_pda_seeds (readable)   → 6 ✅ Token B Mint
5. pool_token_a_vault_account (writable)   → 7 ✅ Token A Vault PDA
6. pool_token_b_vault_account (writable)   → 8 ✅ Token B Vault PDA
7. lp_token_a_mint_account (writable)      → 15 ✅ LP Token A Mint (function-specific)
8. lp_token_b_mint_account (writable)      → 16 ✅ LP Token B Mint (function-specific)
9. user_destination_lp_token_account (writable) → 17 ✅ User LP Token Account (function-specific)
10. system_program_account (readable)      → 1 ✅ System Program
11. token_program_account (readable)       → 9 ✅ SPL Token Program
12. rent_sysvar_account (readable)         → 2 ✅ Rent Sysvar
13. clock_sysvar_account (readable)        → 3 ✅ Clock Sysvar
14. main_treasury_account (writable)       → 12 ✅ Main Treasury PDA
// NEW: Add missing user output token account
11. user_output_token_account (writable)   → 11 ✅ User Output Token Account (not used but standardized)
```

**🔧 Changes Required:**
- Reorder accounts to match standard positions 0-14
- Move LP token accounts to function-specific positions (15-17)
- Add placeholder for user output token account at index 11
- Update all internal references

#### **D. `process_withdraw` (Current: 15 accounts)**
**Same pattern as `process_deposit` with similar reordering needed.**

#### **E. `process_initialize_pool` (Current: 12 accounts)**
**Current Order → Standardized Order:**
```rust
// CURRENT (src/processors/pool_creation.rs:74)
0. payer (signer, writable)                → 0 ✅ Authority/User Signer
1. pool_state_pda_account (writable)       → 4 ✅ Pool State PDA
2. multiple_token_mint_account (readable)  → 5 ✅ Token A Mint
3. base_token_mint_account (readable)      → 6 ✅ Token B Mint
4. lp_token_a_mint_account (signer, writable) → 15 ✅ LP Token A Mint (function-specific)
5. lp_token_b_mint_account (signer, writable) → 16 ✅ LP Token B Mint (function-specific)
6. token_a_vault_pda_account (writable)    → 7 ✅ Token A Vault PDA
7. token_b_vault_pda_account (writable)    → 8 ✅ Token B Vault PDA
8. system_program_account (readable)       → 1 ✅ System Program
9. token_program_account (readable)        → 9 ✅ SPL Token Program
10. rent_sysvar_account (readable)         → 2 ✅ Rent Sysvar
11. main_treasury_account (writable)       → 12 ✅ Main Treasury PDA
// NEW: Add missing standard accounts
3. clock_sysvar_account (readable)         → 3 ✅ Clock Sysvar
10. user_input_token_account (writable)    → 10 ✅ User Input Token Account (not used but standardized)
11. user_output_token_account (writable)   → 11 ✅ User Output Token Account (not used but standardized)
```

**🔧 Changes Required:**
- Reorder accounts to match standard positions 0-12
- Add missing clock sysvar, user token accounts as placeholders
- Move LP token accounts to function-specific positions (15-16)
- Update all internal references

### **2. Treasury Operations (Medium Priority)**

#### **A. `process_withdraw_treasury_fees` (Current: 6 accounts)**
**Current Order → Standardized Order:**
```rust
// CURRENT (src/processors/treasury.rs:56)
0. authority_account (signer)              → 0 ✅ Authority/User Signer
1. main_treasury_account (writable)        → 12 ✅ Main Treasury PDA
2. destination_account (writable)          → 15 ✅ Destination Account (function-specific)
3. system_program_account (readable)       → 1 ✅ System Program
4. rent_sysvar_account (readable)          → 2 ✅ Rent Sysvar
5. system_state_account (readable)         → 16 ✅ System State (function-specific)
// NEW: Add missing standard accounts (as placeholders)
3. clock_sysvar_account (readable)         → 3 ✅ Clock Sysvar
4-11. [placeholder accounts not used]      → 4-11 [not used]
13-14. [treasury accounts not used]       → 13-14 [not used]
```

**🔧 Changes Required:**
- Reorder accounts to match standard positions where applicable
- Add placeholder accounts for unused standard positions
- Move function-specific accounts to positions 15+

#### **B. `process_consolidate_treasuries` (Current: 4 accounts)**
**Current Order → Standardized Order:**
```rust
// CURRENT (src/processors/treasury.rs:163)
0. main_treasury_account (writable)        → 12 ✅ Main Treasury PDA
1. swap_treasury_account (writable)        → 13 ✅ Swap Treasury PDA
2. hft_treasury_account (writable)         → 14 ✅ HFT Treasury PDA
3. clock_sysvar_account (readable)         → 3 ✅ Clock Sysvar
// NEW: Add missing standard accounts (as placeholders)
0. authority_account (signer)              → 0 ✅ Authority/User Signer (not used but standardized)
1. system_program_account (readable)       → 1 ✅ System Program (not used but standardized)
2. rent_sysvar_account (readable)          → 2 ✅ Rent Sysvar (not used but standardized)
4-11. [placeholder accounts not used]     → 4-11 [not used]
```

**🔧 Changes Required:**
- Add placeholder accounts for unused standard positions
- Move treasury accounts to their standardized positions
- Update internal references

### **3. System Control Operations (Low Priority)**

#### **A. `process_initialize_program` (Current: 7 accounts)**
**Current Order → Standardized Order:**
```rust
// CURRENT (src/processors/system_pause.rs:56)
0. system_authority_account (signer, writable) → 0 ✅ Authority/User Signer
1. system_state_account (writable)          → 15 ✅ System State (function-specific)
2. main_treasury_account (writable)         → 12 ✅ Main Treasury PDA
3. swap_treasury_account (writable)         → 13 ✅ Swap Treasury PDA
4. hft_treasury_account (writable)          → 14 ✅ HFT Treasury PDA
5. system_program_account (readable)        → 1 ✅ System Program
6. rent_sysvar_account (readable)           → 2 ✅ Rent Sysvar
// NEW: Add missing standard accounts (as placeholders)
3. clock_sysvar_account (readable)          → 3 ✅ Clock Sysvar
4-11. [placeholder accounts not used]      → 4-11 [not used]
```

**🔧 Changes Required:**
- Reorder accounts to match standard positions
- Add placeholder accounts for unused standard positions
- Move system state to function-specific position

#### **B. `process_pause_system` & `process_unpause_system` (Current: 2 accounts)**
**Current Order → Standardized Order:**
```rust
// CURRENT (src/processors/system_pause.rs:245)
0. authority_account (signer)              → 0 ✅ Authority/User Signer
1. system_state_account (writable)         → 15 ✅ System State (function-specific)
// NEW: Add missing standard accounts (as placeholders)
1. system_program_account (readable)       → 1 ✅ System Program (not used but standardized)
2. rent_sysvar_account (readable)          → 2 ✅ Rent Sysvar (not used but standardized)
3. clock_sysvar_account (readable)         → 3 ✅ Clock Sysvar (not used but standardized)
4-14. [placeholder accounts not used]     → 4-14 [not used]
```

**🔧 Changes Required:**
- Add placeholder accounts for unused standard positions
- Move system state to function-specific position

#### **C. `process_set_swap_fee` (Current: 2 accounts)**
**Current Order → Standardized Order:**
```rust
// CURRENT (src/processors/swap.rs:910)
0. owner (signer)                          → 0 ✅ Authority/User Signer
1. pool_state (writable)                   → 4 ✅ Pool State PDA
// NEW: Add missing standard accounts (as placeholders)
1. system_program_account (readable)       → 1 ✅ System Program (not used but standardized)
2. rent_sysvar_account (readable)          → 2 ✅ Rent Sysvar (not used but standardized)
3. clock_sysvar_account (readable)         → 3 ✅ Clock Sysvar (not used but standardized)
5-14. [placeholder accounts not used]     → 5-14 [not used]
```

**🔧 Changes Required:**
- Add placeholder accounts for unused standard positions
- Maintain pool state at standardized position

---

## 📋 **Migration Steps**

### **Phase 1: High Priority Functions (Pool Operations)**
1. **Implement account reordering for swap functions**
   - Update `process_swap` and `process_swap_hft_optimized`
   - Add treasury accounts and fee collection
   - Update all internal index references

2. **Implement account reordering for liquidity functions**
   - Update `process_deposit` and `process_withdraw`
   - Reorder accounts to match standard positions
   - Move LP token accounts to function-specific positions

3. **Implement account reordering for pool creation**
   - Update `process_initialize_pool`
   - Add missing standard accounts as placeholders
   - Move LP token accounts to function-specific positions

### **Phase 2: Medium Priority Functions (Treasury Operations)**
1. **Update treasury management functions**
   - Reorder accounts in `process_withdraw_treasury_fees`
   - Add placeholder accounts for unused standard positions
   - Update `process_consolidate_treasuries`

### **Phase 3: Low Priority Functions (System Control)**
1. **Update system control functions**
   - Reorder accounts in `process_initialize_program`
   - Add placeholder accounts to pause/unpause functions
   - Update `process_set_swap_fee`

### **Phase 4: Testing & Validation**
1. **Update all test files** to use new account ordering
2. **Create standardized test helpers** using the new ordering
3. **Validate all functions** work correctly with new ordering
4. **Update documentation** and API references

---

## 🔧 **Implementation Utilities**

### **Standard Account Builder Function**
```rust
pub fn build_standard_accounts(
    authority: &Pubkey,
    config: &StandardAccountConfig,
) -> Vec<AccountMeta> {
    let mut accounts = Vec::with_capacity(18);
    
    // 0-3: Base System Accounts
    accounts.push(AccountMeta::new(*authority, true));                    // 0: Authority/User Signer
    accounts.push(AccountMeta::new_readonly(system_program::id(), false)); // 1: System Program
    accounts.push(AccountMeta::new_readonly(rent::id(), false));          // 2: Rent Sysvar
    accounts.push(AccountMeta::new_readonly(clock::id(), false));         // 3: Clock Sysvar
    
    // 4-8: Pool Core Accounts (optional)
    if let Some(pool_config) = &config.pool_config {
        accounts.push(AccountMeta::new(pool_config.pool_state_pda, false));     // 4: Pool State PDA
        accounts.push(AccountMeta::new_readonly(pool_config.token_a_mint, false)); // 5: Token A Mint
        accounts.push(AccountMeta::new_readonly(pool_config.token_b_mint, false)); // 6: Token B Mint
        accounts.push(AccountMeta::new(pool_config.token_a_vault_pda, false));  // 7: Token A Vault PDA
        accounts.push(AccountMeta::new(pool_config.token_b_vault_pda, false));  // 8: Token B Vault PDA
    } else {
        // Add placeholder accounts for unused positions
        for _ in 4..=8 {
            accounts.push(AccountMeta::new_readonly(Pubkey::default(), false));
        }
    }
    
    // 9-11: Token Operations (optional)
    if let Some(token_config) = &config.token_config {
        accounts.push(AccountMeta::new_readonly(spl_token::id(), false));             // 9: SPL Token Program
        accounts.push(AccountMeta::new(token_config.user_input_token_account, false)); // 10: User Input Token Account
        accounts.push(AccountMeta::new(token_config.user_output_token_account, false)); // 11: User Output Token Account
    } else {
        // Add placeholder accounts for unused positions
        for _ in 9..=11 {
            accounts.push(AccountMeta::new_readonly(Pubkey::default(), false));
        }
    }
    
    // 12-14: Treasury System (optional)
    if let Some(treasury_config) = &config.treasury_config {
        accounts.push(AccountMeta::new(treasury_config.main_treasury_pda, false));  // 12: Main Treasury PDA
        accounts.push(AccountMeta::new(treasury_config.swap_treasury_pda, false));  // 13: Swap Treasury PDA
        accounts.push(AccountMeta::new(treasury_config.hft_treasury_pda, false));   // 14: HFT Treasury PDA
    } else {
        // Add placeholder accounts for unused positions
        for _ in 12..=14 {
            accounts.push(AccountMeta::new_readonly(Pubkey::default(), false));
        }
    }
    
    accounts
}
```

### **Account Validation Helper**
```rust
pub fn validate_standard_accounts(accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() < 15 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // Validate required positions
    if !accounts[0].is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    if *accounts[1].key != system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    
    if *accounts[2].key != rent::id() {
        return Err(ProgramError::InvalidAccountData);
    }
    
    if *accounts[3].key != clock::id() {
        return Err(ProgramError::InvalidAccountData);
    }
    
    Ok(())
}
```

---

## ⚠️ **Breaking Changes Warning**

**This migration will require updates to:**
- All client SDKs and applications
- All test files and test utilities
- All documentation and examples
- Any external integrations

**Migration Period:**
- Implement new standardized functions alongside existing ones
- Add deprecation warnings to old functions
- Provide migration guide for external developers
- Remove old functions after sufficient migration period

---

## 🎯 **Success Criteria**

1. **All 11 process functions** follow the standardized account ordering
2. **All tests pass** with the new account ordering
3. **Common account builder functions** work across all operations
4. **Developer documentation** is updated and comprehensive
5. **No functional regressions** in any operation
6. **Performance maintained** or improved (especially for HFT operations)

This standardization will significantly improve the developer experience and enable powerful shared utilities across the entire Fixed-Ratio Trading protocol. 