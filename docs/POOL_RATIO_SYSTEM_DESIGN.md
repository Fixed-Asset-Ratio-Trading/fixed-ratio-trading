# Pool Ratio System Design: Universal Base Unit Architecture

## Executive Summary

This document outlines an enhanced pool ratio system that supports both simple integer ratios and advanced fractional ratio support. The design ensures universal interoperability between applications of different complexity levels.

## Current System Analysis

### Existing Structure
```rust
pub struct PoolState {
    pub ratio_a_numerator: u64,      // Currently constrained
    pub ratio_b_denominator: u64,    // Always forced to 1
    pub token_a_is_the_multiple: bool, // RENAMED → token_a_is_first
}
```

### Current Limitations
- `ratio_b_denominator` is hardcoded to 1 in all pool creation logic
- Cannot represent fractional ratios like 1.01 BTC = 1.01 USDT
- Limited to simple integer relationships (2:1, 100:1, 1000:1)
- Redundant field that adds complexity without benefit

## Proposed Enhanced Design

### Enhanced Architecture
```rust
pub struct PoolState {
    // UNIVERSAL: Always stored as base unit relationships (EXISTING FIELDS)
    pub ratio_a_numerator: u64,      // Token A base units (already exists)
    pub ratio_b_denominator: u64,    // Token B base units (already exists)
    
    // COMPATIBILITY: Auto-determined by contract (NEW FIELD - NOT USER INPUT)
    pub one_to_many_ratio: bool,     // true = clean whole number ratios with one token = 1.0 (contract sets automatically)
    
    // REMOVED: token_a_is_the_multiple and token_a_is_first (app-specific display logic)
    
    // ... all other existing fields remain unchanged ...
}
```

### Key Design Principles

1. **Universal Base Unit Storage**: All ratios stored as SPL token base units regardless of application complexity
2. **Compatibility Flagging**: `one_to_many_ratio` indicates whether pool was designed for simple integer display
3. **Cross-Application Interoperability**: Any application can read any pool, with mode awareness
4. **Precision Preservation**: Full SPL token decimal precision maintained (6-9 decimals typical)

## Application Modes

### Mode 1: Integer Applications (`one_to_many_ratio = true`)

**Target Use Case**: Simple trading applications requiring clean integer ratios
**Examples**: "2 USDC per 1 SOL", "1000 DOGE per 1 USDC"

**Validation Rules**:
```rust
// Contract validation for integer mode
if one_to_many_ratio {
    // Verify ratio simplifies to N:1 in display units
    let display_ratio_a = ratio_a_numerator / 10^token_a_decimals;
    let display_ratio_b = ratio_b_denominator / 10^token_b_decimals;
    
    require!(display_ratio_b == 1, "Integer mode requires 1:N or N:1 ratios");
    require!(display_ratio_a > 0, "Ratio must be positive");
    require!(display_ratio_a % 1 == 0, "Ratio must be whole number");
}
```

**Storage Example**:
```rust
// User creates: "1 SOL = 2 USDC" (wants SOL displayed first)
// Token A = SOL, Token B = USDC
// SOL: 9 decimals, USDC: 6 decimals
PoolState {
    ratio_a_numerator: 1_000_000_000,  // 1.0 SOL in base units  
    ratio_b_denominator: 2_000_000,    // 2.0 USDC in base units
    one_to_many_ratio: true,
    token_a_is_first: true,            // Display as "1 SOL = 2 USDC" (SOL first)
}
```

### Mode 2: Advanced Applications (`one_to_many_ratio = false`)

**Target Use Case**: Professional trading applications requiring precise fractional ratios
**Examples**: "1.01 USDT per 1 BTC", "0.5 ETH per 1 BTC"

**Validation Rules**:
```rust
// Contract validation for advanced mode
if !one_to_many_ratio {
    require!(ratio_a_numerator > 0, "Numerator must be positive");
    require!(ratio_b_denominator > 0, "Denominator must be positive");
    // No additional constraints - full flexibility
    // token_a_is_first is ignored - advanced apps handle their own display logic
}
```

**Storage Example**:
```rust
// User creates: "1 BTC = 1.01 USDT" 
// Token A = BTC, Token B = USDT
// BTC: 8 decimals, USDT: 6 decimals
PoolState {
    ratio_a_numerator: 100_000_000,    // 1.0 BTC in base units
    ratio_b_denominator: 1_010_000,    // 1.01 USDT in base units
    one_to_many_ratio: false,
    token_a_is_first: true,            // Ignored by contract in advanced mode only for UI
}
```

## Universal Pool Reading

All applications read pools the same way since data is consistently stored as base units:

```rust
fn get_precise_exchange_rate(pool: &PoolState, token_a_decimals: u8, token_b_decimals: u8) -> f64 {
    // Universal function - works with ANY pool type
    let rate_base_units = pool.ratio_a_numerator as f64 / pool.ratio_b_denominator as f64;
    
    // Adjust for token decimal differences to get display rate
    let decimal_adjustment = 10_f64.powi(token_b_decimals as i32 - token_a_decimals as i32);
    rate_base_units * decimal_adjustment
}

// Applications handle their own filtering and display logic:
// - Integer apps: filter by pool.one_to_many_ratio == true in their UI
// - Advanced apps: can use any pool, choose display complexity
// - All apps: use pool.token_a_is_first for display order preference
```

## Swap Calculation Impact

### Current Swap Logic
```rust
// Current implementation (unchanged)
let amount_out = if input_is_token_a {
    amount_in * pool_state.ratio_b_denominator / pool_state.ratio_a_numerator
} else {
    amount_in * pool_state.ratio_a_numerator / pool_state.ratio_b_denominator
};
```

### Impact Analysis
- **No changes required** to core swap logic
- Base unit storage ensures mathematical precision
- Both application modes use identical calculation paths
- Existing tests remain valid

## Pool Creation Workflow

### Current Pool Creation Parameters (Existing)
```rust
// Current InitializePool instruction
InitializePool {
    multiple_per_base: u64,              // Single ratio value (normalized to base units) 
    pool_authority_bump_seed: u8,        // Pool PDA bump seed
    multiple_token_vault_bump_seed: u8,  // Multiple token vault bump seed
    base_token_vault_bump_seed: u8,      // Base token vault bump seed
}

// Current accounts (11 total):
// 0. Payer (signer, writable) 
// 1. Pool State PDA (writable)
// 2. Multiple Token Mint (readable)
// 3. Base Token Mint (readable)
// 4. LP Token A Mint (signer, writable)
// 5. LP Token B Mint (signer, writable)
// 6. Token A Vault PDA (writable)
// 7. Token B Vault PDA (writable)
// 8. System Program (readable)
// 9. SPL Token Program (readable)
// 10. Rent Sysvar (readable)
```

### Enhanced Pool Creation Parameters (Updated)
```rust
// Enhanced InitializePool instruction  
InitializePool {
    ratio_a_numerator: u64,              // Token A base units (EXISTS - replaces multiple_per_base in instruction)
    ratio_b_denominator: u64,            // Token B base units (EXISTS - was hardcoded to 1 in creation logic)
    token_a_is_first: bool,              // Display preference flag (RENAMED from token_a_is_the_multiple with opposite meaning)
    pool_authority_bump_seed: u8,        // Pool PDA bump seed (UNCHANGED)
    token_a_vault_bump_seed: u8,         // Token A vault bump seed (RENAMED from multiple_token_vault_bump_seed)
    token_b_vault_bump_seed: u8,         // Token B vault bump seed (RENAMED from base_token_vault_bump_seed)
}

// NOTE: one_to_many_ratio is NOT a parameter - it's automatically determined by the contract
```

// Accounts remain the same (11 total):
// 0. Payer (signer, writable) 
// 1. Pool State PDA (writable)
// 2. Token A Mint (readable) - RENAMED from Multiple Token Mint
// 3. Token B Mint (readable) - RENAMED from Base Token Mint  
// 4. LP Token A Mint (signer, writable)
// 5. LP Token B Mint (signer, writable)
// 6. Token A Vault PDA (writable)
// 7. Token B Vault PDA (writable)
// 8. System Program (readable)
// 9. SPL Token Program (readable)
// 10. Rent Sysvar (readable)
```

### Parameter Changes Summary
**Modified Parameters:**
- `multiple_per_base: u64` → `ratio_a_numerator: u64` (semantic rename - field already exists)
- `multiple_token_vault_bump_seed: u8` → `token_a_vault_bump_seed: u8` (semantic rename)
- `base_token_vault_bump_seed: u8` → `token_b_vault_bump_seed: u8` (semantic rename)

**Removed Parameters:**
- `token_a_is_first: bool` (removed - apps handle display preferences)

**Unchanged Parameters:**
- `ratio_a_numerator: u64` (already exists - just exposing in instruction)
- `ratio_b_denominator: u64` (already exists - was hardcoded to 1 in creation logic)
- `pool_authority_bump_seed: u8` (no change)
- All account parameters (no change)

**Auto-Determined Field (Not a Parameter):**
- `one_to_many_ratio: bool` - Contract automatically sets to true if:
  - Both ratios are positive whole numbers (no fractions)
  - One token has exactly 1.0 ratio
  - Both ratios are greater than zero

**Removed PoolState Fields:**
- `token_a_is_the_multiple: bool` (removed - app-specific logic)

### Contract Validation Rules
```rust
// Step 1: Basic validation for all pools
require!(ratio_a_numerator > 0, "Numerator must be positive");
require!(ratio_b_denominator > 0, "Denominator must be positive");

// Step 2: Auto-determine if pool has clean one-to-many ratio
let has_one_to_many = check_one_to_many_ratio(
    ratio_a_numerator, 
    ratio_b_denominator, 
    token_a_decimals, 
    token_b_decimals
);

// Step 3: Set one_to_many_ratio flag automatically
pool_state.one_to_many_ratio = has_one_to_many;

fn check_one_to_many_ratio(
    ratio_a_numerator: u64,
    ratio_b_denominator: u64, 
    token_a_decimals: u8,
    token_b_decimals: u8
) -> bool {
    let token_a_decimal_factor = 10_u64.pow(token_a_decimals as u32);
    let token_b_decimal_factor = 10_u64.pow(token_b_decimals as u32);
    
    // Check if both ratios represent whole numbers (no fractional parts)
    let a_is_whole = (ratio_a_numerator % token_a_decimal_factor) == 0;
    let b_is_whole = (ratio_b_denominator % token_b_decimal_factor) == 0;
    
    // Convert to display units
    let display_ratio_a = ratio_a_numerator / token_a_decimal_factor;
    let display_ratio_b = ratio_b_denominator / token_b_decimal_factor;
    
    // Check if both are greater than zero, whole numbers, and one equals exactly 1
    let both_positive = display_ratio_a > 0 && display_ratio_b > 0;
    let one_equals_one = display_ratio_a == 1 || display_ratio_b == 1;
    
    a_is_whole && b_is_whole && both_positive && one_equals_one
}
```

### Application Usage Examples

**One-to-Many Pool (Your Integer App's Target)**:
```rust
// App converts "1 SOL = 2 USDC" to base units
create_pool(
    1_000_000_000,  // 1.0 SOL in base units (SOL: 9 decimals)
    2_000_000,      // 2.0 USDC in base units (USDC: 6 decimals)
)
// Contract automatically sets one_to_many_ratio = true (both whole numbers: 1.0 SOL, 2.0 USDC)
```

**Many-to-One Pool (Also One-to-Many)**:
```rust
// App converts "1000 DOGE = 1 USDC" to base units
create_pool(
    1_000_000_000,  // 1000.0 DOGE in base units (DOGE: 6 decimals)
    1_000_000,      // 1.0 USDC in base units (USDC: 6 decimals)
)
// Contract automatically sets one_to_many_ratio = true (both whole numbers: 1000.0 DOGE, 1.0 USDC)
```

**Fractional Pool (Not One-to-Many)**:
```rust
// App converts "1 BTC = 1.01 USDT" to base units  
create_pool(
    100_000_000,    // 1.0 BTC in base units (BTC: 8 decimals)
    1_010_000,      // 1.01 USDT in base units (USDT: 6 decimals)
)
// Contract automatically sets one_to_many_ratio = false (1.01 is not a whole number)
```

**Another Non-Qualifying Example**:
```rust
// App converts "0.5 BTC = 1 ETH" to base units
create_pool(
    50_000_000,     // 0.5 BTC in base units (BTC: 8 decimals)
    1_000_000_000,  // 1.0 ETH in base units (ETH: 9 decimals)  
)
// Contract automatically sets one_to_many_ratio = false (0.5 is not a whole number)
```

**Benefits of Auto-Detection**:
- Your app can filter `one_to_many_ratio = true` pools for clean display
- Contract determines this objectively based on actual ratios
- Apps handle their own display preferences (which token to show first)

## Implementation Strategy

### Phase 1: Core Implementation
```rust
// Update PoolState structure
pub struct PoolState {
    pub ratio_a_numerator: u64,         // EXISTING - no change needed
    pub ratio_b_denominator: u64,       // EXISTING - no change needed
    pub one_to_many_ratio: bool,         // ADD - auto-determined by contract (not user parameter)
    // REMOVE: token_a_is_the_multiple (app-specific logic)
    // ... all other existing fields unchanged
}

// Update seed prefix (remove version suffix)
pub const POOL_STATE_SEED_PREFIX: &[u8] = b"pool_state"; // Was: b"pool_state_v2"
```

### Phase 2: Update Instructions and Client Code
```rust
// Update InitializePool instruction parameters
InitializePool {
    ratio_a_numerator: u64,              // Renamed from multiple_per_base  
    ratio_b_denominator: u64,            // Expose existing field (was hardcoded to 1)
    pool_authority_bump_seed: u8,        // Unchanged
    token_a_vault_bump_seed: u8,         // Renamed from multiple_token_vault_bump_seed
    token_b_vault_bump_seed: u8,         // Renamed from base_token_vault_bump_seed
}
// Note: one_to_many_ratio is NOT a parameter - auto-determined by contract logic
// Note: Display preferences are handled by individual apps, not the contract
```

// Update PDA derivation in all files
let seeds = [
    b"pool_state",                       // Remove "_v2" suffix
    token_a_mint.as_ref(),
    token_b_mint.as_ref(), 
    &ratio_a_numerator.to_le_bytes(),
    &ratio_b_denominator.to_le_bytes(),
];
```

### Phase 3: Application Development
```rust
// Your integer app: Filter pools where one_to_many_ratio = true
// Advanced apps: Can use any pool regardless of one_to_many_ratio
// All apps: Implement their own display preferences (which token to show first)
// Universal: Use same get_precise_exchange_rate() function
```

### Code Changes Required
**Constants**: Update `POOL_STATE_SEED_PREFIX` to remove "_v2"
**Types**: 
- ADD `one_to_many_ratio: bool` field to PoolState (auto-determined by contract)
- REMOVE `token_a_is_the_multiple: bool` field from PoolState 
- UPDATE get_packed_len() to reflect field changes
**Instructions**: Update InitializePool parameters (remove token_a_is_first parameter)
**Validation**: Add one-to-many ratio detection logic
**Client SDK**: Update parameter handling and PDA derivation (remove "_v2")
**Dashboard**: Update pool creation UI and PDA derivation (remove "_v2")
**Tests**: Update all test cases for new parameters
**Logic**: Implement automatic one-to-many detection (check if either token equals 1.0)
**Apps**: Each app implements its own display preferences for token ordering

## Edge Cases and Validations

### Ratio Precision Limits
```rust
// Maximum precision bounded by u64 limits and token decimals
// Typical case: 18 total decimal places available (u64 max ≈ 18 digits)
// Per-token: 6-9 decimals common, leaves 9-12 digits for ratio precision
```

### Zero Ratio Prevention
```rust
fn validate_ratio(ratio_a_numerator: u64, ratio_b_denominator: u64) -> Result<(), PoolError> {
    require!(ratio_a_numerator > 0, "Numerator cannot be zero");
    require!(ratio_b_denominator > 0, "Denominator cannot be zero");
    Ok(())
}
```

### Overflow Protection
```rust
fn safe_swap_calculation(amount_in: u64, ratio_num: u64, ratio_den: u64) -> Result<u64, PoolError> {
    amount_in
        .checked_mul(ratio_num)
        .ok_or(PoolError::ArithmeticOverflow)?
        .checked_div(ratio_den)
        .ok_or(PoolError::ArithmeticOverflow)
}
```

## Benefits Analysis

### Technical Benefits
1. **Universal Compatibility**: Any app can read any pool
2. **Precision Preservation**: Full SPL token decimal support
3. **Clean Architecture**: Single storage format, multiple interpretation modes
4. **Future-Proof**: Supports unknown future ratio requirements

### Business Benefits
1. **Market Expansion**: Simple apps for mass market, advanced apps for professionals
2. **Liquidity Sharing**: Pools usable across application types
3. **Developer Adoption**: Clear APIs for different complexity levels
4. **Ecosystem Growth**: Foundation for diverse trading applications

## Design Decisions Made

### Technical Decisions
1. ✅ **PDA Derivation**: Keep current seeds (Option A) for maximum interoperability
2. ✅ **Seed Prefix**: Remove "_v2" version suffix for clean, permanent prefix

### PDA Seed Strategy (Final)
```rust
// Final PDA derivation (no mode flags in seeds)
let seeds = [
    "pool_state",              // Clean prefix (removed _v2)
    token_a_mint,              // Lexicographically first token
    token_b_mint,              // Lexicographically second token  
    ratio_a_numerator,         // Token A base units
    ratio_b_denominator,       // Token B base units
];

// Result: Maximum interoperability
// - Integer apps and advanced apps share same pools
// - No liquidity fragmentation by application type
// - Future app types can use existing pools
```

### Remaining Questions
1. **Gas Optimization**: Any concerns about additional field storage costs?
2. **Field Layout**: Optimal positioning of new fields in PoolState structure?
3. **Use Case Coverage**: Does this handle all anticipated ratio requirements?
4. **Security**: Are ratio constraints sufficient to prevent edge cases?

## Implementation Priority

### Critical Path
1. ✅ Design validation (this document)
2. 🔄 Add `one_to_many_ratio` field to PoolState (auto-determined by contract)
3. 🔄 Remove `token_a_is_the_multiple` field from PoolState
4. 🔄 Update pool creation with auto-detection validation logic
5. 🔄 Update client SDKs and PDA derivation (remove "_v2" suffix)
6. 🔄 Update dashboard for new parameters (no mode selection - contract auto-determines)
7. 🔄 Comprehensive testing suite for enhanced validation

### Success Metrics
- Successful creation and operation of integer ratio pools
- Successful creation and operation of fractional ratio pools  
- Cross-application pool usage demonstrated
- Developer adoption of both modes

---

**Document Status**: Draft for Review  
**Next Action**: Technical validation and implementation planning 