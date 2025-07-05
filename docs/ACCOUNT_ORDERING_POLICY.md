# Account Ordering Policy & Standards

## 📋 **Overview**

This document defines the standardized account ordering for all `process_*` functions in the Fixed-Ratio Trading protocol. Consistent account ordering enables:

- **Predictable Development**: Developers know what to expect at each index
- **Common Helper Functions**: Shared utilities for account array construction
- **Reduced Errors**: Consistent patterns reduce account mismatches
- **Better Testing**: Standardized test helpers across all operations

## 🎯 **Core Principle**

**"Most common accounts appear at the same indices across all functions"**

Account positions 0-14 are reserved for the most frequently used accounts, with function-specific accounts starting at index 15.

---

## 📊 **Standardized Account Order (Indices 0-14)**

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

### **🏦 Treasury System (12-14)**
```rust
12. **Main Treasury PDA** (writable) - Main treasury for fee collection
13. **Swap Treasury PDA** (writable) - Specialized swap treasury (when needed)
14. **HFT Treasury PDA** (writable) - Specialized HFT treasury (when needed)
```

### **⚙️ Function-Specific (15+)**
```rust
15+ **Function-Specific Accounts** - LP token mints, system state, specialized accounts
```

---

## 🔄 **Account Usage Matrix**

| Function | 0-3 (System) | 4-8 (Pool) | 9-11 (Token) | 12-14 (Treasury) | 15+ (Specific) |
|----------|--------------|------------|--------------|------------------|----------------|
| `process_swap` | ✅ (0,1,2,3) | ✅ (4,5,6,7,8) | ✅ (9,10,11) | ⚠️ (12 only) | - |
| `process_deposit` | ✅ (0,1,2,3) | ✅ (4,5,6,7,8) | ✅ (9,10,11) | ✅ (12) | LP mints (15,16) |
| `process_withdraw` | ✅ (0,1,2,3) | ✅ (4,5,6,7,8) | ✅ (9,10,11) | ✅ (12) | LP mints (15,16) |
| `process_initialize_pool` | ✅ (0,1,2,3) | ✅ (4,5,6,7,8) | ✅ (9) | ✅ (12) | LP mints (15,16) |
| `process_withdraw_treasury_fees` | ✅ (0,1,2) | ❌ | ❌ | ✅ (12) | Dest account (15), System state (16) |
| `process_pause_system` | ✅ (0) | ❌ | ❌ | ❌ | System state (15) |

**Legend:**
- ✅ **Full Usage**: All indices in range used
- ⚠️ **Partial Usage**: Some indices in range used  
- ❌ **Not Used**: Range not applicable to function

---

## 🛠️ **Implementation Guidelines**

### **1. Account Array Construction Pattern**
```rust
pub fn build_standard_accounts(
    authority: &Pubkey,
    pool_config: &PoolConfig,
    user_accounts: &UserAccounts,
    treasury_config: &TreasuryConfig,
) -> Vec<AccountMeta> {
    vec![
        // 0-3: Base System Accounts
        AccountMeta::new(*authority, true),                          // 0: Authority/User Signer
        AccountMeta::new_readonly(system_program::id(), false),      // 1: System Program
        AccountMeta::new_readonly(rent::id(), false),                // 2: Rent Sysvar
        AccountMeta::new_readonly(clock::id(), false),               // 3: Clock Sysvar
        
        // 4-8: Pool Core Accounts  
        AccountMeta::new(pool_config.pool_state_pda, false),         // 4: Pool State PDA
        AccountMeta::new_readonly(pool_config.token_a_mint, false),  // 5: Token A Mint
        AccountMeta::new_readonly(pool_config.token_b_mint, false),  // 6: Token B Mint
        AccountMeta::new(pool_config.token_a_vault_pda, false),      // 7: Token A Vault PDA
        AccountMeta::new(pool_config.token_b_vault_pda, false),      // 8: Token B Vault PDA
        
        // 9-11: Token Operations
        AccountMeta::new_readonly(spl_token::id(), false),           // 9: SPL Token Program
        AccountMeta::new(user_accounts.input_token_account, false),  // 10: User Input Token Account
        AccountMeta::new(user_accounts.output_token_account, false), // 11: User Output Token Account
        
        // 12-14: Treasury System
        AccountMeta::new(treasury_config.main_treasury_pda, false),  // 12: Main Treasury PDA
        AccountMeta::new(treasury_config.swap_treasury_pda, false),  // 13: Swap Treasury PDA
        AccountMeta::new(treasury_config.hft_treasury_pda, false),   // 14: HFT Treasury PDA
    ]
}
```

### **2. Function-Specific Extension Pattern**
```rust
pub fn extend_for_liquidity_operations(
    mut base_accounts: Vec<AccountMeta>,
    lp_config: &LPTokenConfig,
) -> Vec<AccountMeta> {
    // Add function-specific accounts starting at index 15
    base_accounts.extend(vec![
        AccountMeta::new(lp_config.lp_token_a_mint, false),      // 15: LP Token A Mint
        AccountMeta::new(lp_config.lp_token_b_mint, false),      // 16: LP Token B Mint
        AccountMeta::new(lp_config.user_lp_token_account, false), // 17: User LP Token Account
    ]);
    base_accounts
}
```

### **3. Account Validation Helper**
```rust
pub fn validate_standard_accounts(accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() < 15 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // Validate standard account positions
    validate_signer(&accounts[0], "Authority/User")?;           // Index 0
    validate_program_id(&accounts[1], &system_program::id())?;  // Index 1
    validate_sysvar(&accounts[2], &rent::id())?;                // Index 2
    validate_sysvar(&accounts[3], &clock::id())?;               // Index 3
    validate_writable(&accounts[4], "Pool State PDA")?;         // Index 4
    validate_program_id(&accounts[9], &spl_token::id())?;       // Index 9
    
    Ok(())
}
```

---

## 📝 **Migration Strategy**

### **Phase 1: Create Standard Account Builders**
1. Create `StandardAccountBuilder` trait and implementations
2. Add validation helpers for standard positions
3. Create test utilities using standard ordering

### **Phase 2: Update Process Functions**
1. **High Priority**: Update swap and liquidity functions (most used)
2. **Medium Priority**: Update pool creation and treasury functions  
3. **Low Priority**: Update system control functions

### **Phase 3: Deprecate Old Patterns**
1. Mark old account ordering as deprecated with compiler warnings
2. Update all tests to use standard ordering
3. Update documentation and examples

---

## 🧪 **Testing Benefits**

### **Common Test Helper Functions**
```rust
pub struct StandardTestAccounts {
    pub authority: Keypair,
    pub pool_config: PoolConfig,
    pub user_accounts: UserAccounts,
    pub treasury_config: TreasuryConfig,
}

impl StandardTestAccounts {
    pub fn build_swap_instruction(&self, amount: u64) -> Instruction {
        let accounts = build_standard_accounts(
            &self.authority.pubkey(),
            &self.pool_config,
            &self.user_accounts,
            &self.treasury_config,
        );
        
        Instruction {
            program_id: PROGRAM_ID,
            accounts: accounts[0..12].to_vec(), // Swap uses indices 0-11
            data: PoolInstruction::Swap { /* ... */ }.try_to_vec().unwrap(),
        }
    }
    
    pub fn build_deposit_instruction(&self, amount: u64) -> Instruction {
        let mut accounts = build_standard_accounts(/* ... */);
        accounts = extend_for_liquidity_operations(accounts, &self.lp_config);
        
        Instruction {
            program_id: PROGRAM_ID,
            accounts: accounts[0..18].to_vec(), // Deposit uses indices 0-17
            data: PoolInstruction::Deposit { /* ... */ }.try_to_vec().unwrap(),
        }
    }
}
```

### **Reduced Test Code Duplication**
```rust
// Before: Different account construction for each test
let swap_accounts = vec![user, input_account, output_account, pool_state, /*...*/];
let deposit_accounts = vec![user, source_account, pool_state, token_a_mint, /*...*/];

// After: Consistent pattern
let test_accounts = StandardTestAccounts::new(/* ... */);
let swap_instruction = test_accounts.build_swap_instruction(1000);
let deposit_instruction = test_accounts.build_deposit_instruction(500);
```

---

## 🚀 **Developer Benefits**

### **1. Predictable Patterns**
```rust
// Developers know that index 0 is always the signer
if !accounts[0].is_signer {
    return Err(ProgramError::MissingRequiredSignature);
}

// Index 4 is always Pool State PDA (when applicable)
let pool_state = PoolState::deserialize(&accounts[4].data.borrow())?;

// Index 9 is always SPL Token Program (when applicable)  
if *accounts[9].key != spl_token::id() {
    return Err(ProgramError::IncorrectProgramId);
}
```

### **2. Shared Validation Logic**
```rust
pub fn validate_pool_operation_accounts(accounts: &[AccountInfo]) -> ProgramResult {
    validate_standard_accounts(accounts)?;
    
    // Pool-specific validations for indices 4-8
    validate_pool_state(&accounts[4])?;
    validate_token_mints(&accounts[5], &accounts[6])?;
    validate_token_vaults(&accounts[7], &accounts[8])?;
    
    Ok(())
}
```

### **3. Documentation Clarity**
All functions follow the same pattern, making it easier to:
- Read and understand account requirements
- Copy patterns between functions  
- Generate documentation automatically
- Create tooling and SDKs

---

## ⚠️ **Important Notes**

### **Backwards Compatibility**
- This standardization will require updating all existing `process_*` functions
- Current account ordering will be deprecated with migration period
- All tests and documentation must be updated

### **Function-Specific Considerations**
- **System Control Functions**: May only use indices 0-3 (authority + sysvars)
- **Treasury Functions**: May skip pool accounts (4-8) and use treasury accounts (12-14)
- **Pool Operations**: Use most/all standard indices (0-14)

### **Account Validation**
- Each function should validate that required accounts are provided
- Unused account indices should be clearly documented
- Optional accounts should be handled gracefully

---

## 📚 **Examples**

### **Before (Current Inconsistent Ordering)**
```rust
// process_swap: user, input, output, pool_state, token_a_mint, token_b_mint, ...
// process_deposit: user, source, pool_state, token_a_mint, token_b_mint, token_a_vault, ...  
// process_withdraw: user, lp_account, dest_account, pool_state, token_a_mint, ...
```

### **After (Standardized Ordering)**
```rust
// ALL functions: authority, system_program, rent, clock, pool_state, token_a_mint, token_b_mint, token_a_vault, token_b_vault, spl_token, user_input, user_output, main_treasury, [function-specific...]
```

This standardization will significantly improve the developer experience and enable powerful common utilities for account management across the entire protocol. 