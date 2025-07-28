# Decimal Basis Points Refactor Plan
**Critical System-Wide Fix Required**

## 🚨 **Core Problem**

The system inconsistently handles token decimals, mixing **display units** with **basis points** across different components:

- **Basis Points**: Smallest token unit (e.g., 1 USDC = 1,000,000 basis points with 6 decimals)
- **Display Units**: User-friendly units (e.g., 1.5 USDC)

**Result**: Incorrect swap calculations, pool ratios, and liquidity calculations.

## 🚨 **Project Status: Pre-Release**

**CRITICAL**: This project is **not yet released to production**, therefore:
- **Full code replacement is acceptable** - no backward compatibility required
- **Breaking changes are encouraged** - clean implementation over compatibility
- **Fresh devnet environment** - reset devnet means completely fresh start
- **No migration complexity** - direct implementation of proper basis point handling

## 📊 **Current State Analysis**

### **✅ Components Using Basis Points Correctly**
- SPL Token transfers (native Solana requirement)
- User token account balances
- Transaction amounts in blockchain

### **❌ Components Using Display Units Incorrectly**
- `ratio_a_numerator` and `ratio_b_denominator` in pool state
- `total_token_a_liquidity` and `total_token_b_liquidity` in pool state  
- Pool creation calculations
- Swap calculation logic
- Dashboard display calculations
- Test case expected values

## 🎯 **Target Architecture**

### **Universal Rule: All Internal Storage = Basis Points**
```rust
// ✅ CORRECT: Store in basis points
pool_state.ratio_a_numerator = 1_000_000;  // 1.0 token with 6 decimals
pool_state.total_token_a_liquidity = 100_000_000;  // 100.0 tokens with 6 decimals

// ❌ WRONG: Store in display units  
pool_state.ratio_a_numerator = 1;  // Ambiguous without decimal context
```

### **Conversion Points**
- **Input**: Convert user input from display units → basis points (client responsibility)
- **Storage**: Always store basis points (contract assumes this)
- **Calculations**: Always use basis points (contract never converts)
- **Output**: Convert basis points → display units for UI (client responsibility)
- **Decimals**: Fetched from token mint accounts when needed (not stored in PoolState)

### **Decimal Source Strategy**
Token decimals are **available from SPL token mint accounts** and can be queried:
- **Smart Contract**: `Mint::unpack(&mint_account.data.borrow())?.decimals`
- **JavaScript/Dashboard**: `connection.getParsedAccountInfo(mint).value.data.parsed.info.decimals`
- **Timing**: Decimals fetched during pool creation for validation, not stored
- **Benefit**: Keeps PoolState lean, avoids stack size limits

### **One-to-Many Validation Rules**
- **Rule**: "1 Token = X Other Token" where **BOTH** `1` and `X` are whole numbers
- **Valid Example**: `1.000 TS = 10000.000 MST` ✅ (both whole)
- **Invalid Example**: `1.500 TS = 10000.000 MST` ❌ (1.500 not whole)
- **Invalid Example**: `1.000 TS = 10000.500 MST` ❌ (10000.500 not whole)
- **Validation**: `(basis_points % 10^decimals) == 0` for both sides

## 🔧 **Affected Components & Changes Needed**

### **1. Smart Contract Core (`src/`)**

#### **A. Pool State Structure (`src/state/pool_state.rs`)**
```rust
// ✅ NO CHANGES TO POOLSTATE: Keep structure lean to avoid stack size limits
pub struct PoolState {
    // ✅ These are already correct (addresses don't have decimals)
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    
    // 🔧 DOCUMENTATION: These MUST be basis points (external apps responsible for conversion)
    pub ratio_a_numerator: u64,     // ALWAYS basis points
    pub ratio_b_denominator: u64,   // ALWAYS basis points
    
    // 🔧 DOCUMENTATION: These MUST be basis points (external apps responsible for conversion)
    pub total_token_a_liquidity: u64,  // ALWAYS basis points
    pub total_token_b_liquidity: u64,  // ALWAYS basis points
    
    // ❌ NO DECIMAL FIELDS: Decimals fetched from token mint accounts when needed
    // This keeps PoolState size under stack limits and maintains simplicity
}
```

#### **B. Pool Creation (`src/processors/pool_creation.rs`)**
```rust
// 🔧 CHANGE: Client sends basis points, contract fetches decimals for validation
fn process_initialize_pool(
    ratio_a_numerator: u64,    // Client-converted basis points
    ratio_b_denominator: u64,  // Client-converted basis points
    accounts: &[AccountInfo],
) -> ProgramResult {
    // Get token mint accounts from instruction accounts
    let token_a_mint_account = &accounts[X];
    let token_b_mint_account = &accounts[Y];
    
    // Fetch decimals from mint accounts (available during pool creation)
    let token_a_mint = Mint::unpack(&token_a_mint_account.data.borrow())?;
    let token_b_mint = Mint::unpack(&token_b_mint_account.data.borrow())?;
    
    let token_a_decimals = token_a_mint.decimals;
    let token_b_decimals = token_b_mint.decimals;
    
    // Validate one-to-many: BOTH sides must be whole numbers
    if !validate_one_to_many(ratio_a_numerator, ratio_b_denominator, token_a_decimals, token_b_decimals) {
        return Err(PoolError::InvalidRatio.into());
    }
    
    // Store basis points directly (no conversion needed)
    pool_state.ratio_a_numerator = ratio_a_numerator;
    pool_state.ratio_b_denominator = ratio_b_denominator;
}

fn validate_one_to_many(
    ratio_a_basis_points: u64,
    ratio_b_basis_points: u64, 
    token_a_decimals: u8,
    token_b_decimals: u8
) -> bool {
    let token_a_factor = 10_u64.pow(token_a_decimals as u32);
    let token_b_factor = 10_u64.pow(token_b_decimals as u32);
    
    // Both ratios must represent whole numbers in display units
    let a_is_whole = (ratio_a_basis_points % token_a_factor) == 0;
    let b_is_whole = (ratio_b_basis_points % token_b_factor) == 0;
    
    a_is_whole && b_is_whole  // "1 Token = X Other Token" where both 1 and X are whole
}
```

#### **C. Swap Logic (`src/processors/swap.rs`)**
```rust
// 🔧 CHANGE: All calculations in basis points (no conversion needed)
fn calculate_swap_output(
    amount_in_basis_points: u64,
    ratio_numerator_basis_points: u64,
    ratio_denominator_basis_points: u64,
) -> u64 {
    // Direct calculation - all values already in basis points
    amount_in_basis_points * ratio_numerator_basis_points / ratio_denominator_basis_points
}
```

#### **D. Liquidity Management (`src/processors/liquidity.rs`)**
```rust
// 🔧 CHANGE: Update liquidity tracking in basis points
fn update_liquidity(
    pool_state: &mut PoolState,
    token_amount_basis_points: u64,
    is_token_a: bool,
    is_deposit: bool,
) {
    if is_token_a {
        if is_deposit {
            pool_state.total_token_a_liquidity += token_amount_basis_points;
        } else {
            pool_state.total_token_a_liquidity -= token_amount_basis_points;
        }
    }
    // Similar for token B...
}
```

### **2. Test Suite (`tests/`)**

#### **A. Test Utilities (`tests/common/`)**
```rust
// 🔧 CHANGE: Helper functions for basis point conversion
pub fn display_to_basis_points(display_amount: f64, decimals: u8) -> u64 {
    (display_amount * 10_f64.powi(decimals as i32)) as u64
}

pub fn basis_points_to_display(basis_points: u64, decimals: u8) -> f64 {
    basis_points as f64 / 10_f64.powi(decimals as i32)
}

// 🔧 CHANGE: Update all test assertions
pub fn assert_swap_calculation(
    input_display: f64,
    expected_output_display: f64,
    input_decimals: u8,
    output_decimals: u8,
    actual_output_basis_points: u64,
) {
    let expected_basis_points = display_to_basis_points(expected_output_display, output_decimals);
    assert_eq!(actual_output_basis_points, expected_basis_points);
}
```

#### **B. Pool Creation Tests (`tests/20_test_pool_creation.rs`)**
```rust
// 🔧 CHANGE: Test client-side conversion and contract validation
#[tokio::test]
async fn test_pool_creation_with_basis_points() {
    // User wants: "1 TS = 10,000 MST" (both whole numbers)
    let ts_ratio_display = 1.0;
    let mst_ratio_display = 10_000.0;
    
    // Mock token mints with 6 decimals each (fetched from mint accounts)
    let ts_decimals = 6;
    let mst_decimals = 6;
    
    // Client converts to basis points before sending
    let ts_ratio_basis_points = (ts_ratio_display * 10_f64.powi(ts_decimals as i32)) as u64;    // 1,000,000
    let mst_ratio_basis_points = (mst_ratio_display * 10_f64.powi(mst_decimals as i32)) as u64;  // 10,000,000,000
    
    // Create pool with basis points (contract validates one-to-many internally)
    create_pool(ts_ratio_basis_points, mst_ratio_basis_points).await;
    
    // Verify storage is in basis points
    let pool_state = get_pool_state().await;
    assert_eq!(pool_state.ratio_a_numerator, 1_000_000);
    assert_eq!(pool_state.ratio_b_denominator, 10_000_000_000);
    
    // Verify one-to-many flag is set correctly
    assert!(pool_state.flags & ONE_TO_MANY_FLAG != 0);
}

#[tokio::test] 
async fn test_invalid_one_to_many_rejected() {
    // User wants: "1.5 TS = 10,000 MST" (1.5 is not whole)
    let ts_ratio_basis_points = 1_500_000;  // 1.5 * 10^6
    let mst_ratio_basis_points = 10_000_000_000;  // 10000.0 * 10^6
    
    // Should fail validation
    let result = create_pool(ts_ratio_basis_points, mst_ratio_basis_points).await;
    assert!(result.is_err());
}
```

#### **C. Swap Tests (`tests/32_test_pool_swaps.rs`)**
```rust
// 🔧 CHANGE: All test calculations use basis points
#[tokio::test]
async fn test_swap_with_decimals() {
    // Setup: 1 TS (6 decimals) = 10,000 MST (6 decimals)
    let input_amount_display = 1.5; // User wants to swap 1.5 TS
    let input_amount_basis_points = display_to_basis_points(input_amount_display, 6); // 1,500,000
    
    let result = perform_swap(input_amount_basis_points).await;
    
    // Expected: 1.5 * 10,000 = 15,000 MST = 15,000,000,000 basis points
    let expected_output_basis_points = display_to_basis_points(15_000.0, 6);
    assert_eq!(result.output_amount, expected_output_basis_points);
}
```

### **3. Dashboard (`dashboard/`)**

#### **A. Pool Creation (`dashboard/pool-creation.js`)**
```javascript
// 🔧 CHANGE: Client converts display units to basis points and validates one-to-many
async function createPool() {
    const primaryRatio = parseFloat(document.getElementById('primary-ratio').value); // 1.0
    const baseRatio = parseFloat(document.getElementById('base-ratio').value);       // 10000.0
    
    // Get token decimals from mint metadata
    const primaryDecimals = await getTokenDecimals(primaryTokenMint);
    const baseDecimals = await getTokenDecimals(baseTokenMint);
    
    // Convert to basis points
    const primaryRatioBasisPoints = primaryRatio * Math.pow(10, primaryDecimals);
    const baseRatioBasisPoints = baseRatio * Math.pow(10, baseDecimals);
    
    // Validate one-to-many locally (both sides must be whole numbers)
    if (!isOneToManyRatio(primaryRatioBasisPoints, baseRatioBasisPoints, primaryDecimals, baseDecimals)) {
        throw new Error("Invalid ratio: Both sides must be whole numbers (e.g., 1 Token = 10000 Other Token)");
    }
    
    // Send basis points to contract (contract will re-validate)
    await initializePool(primaryRatioBasisPoints, baseRatioBasisPoints);
}

function isOneToManyRatio(ratioABasisPoints, ratioBBasisPoints, decimalsA, decimalsB) {
    const factorA = Math.pow(10, decimalsA);
    const factorB = Math.pow(10, decimalsB);
    
    // Both ratios must represent whole numbers in display units
    const aIsWhole = (ratioABasisPoints % factorA) === 0;
    const bIsWhole = (ratioBBasisPoints % factorB) === 0;
    
    return aIsWhole && bIsWhole;
}

async function getTokenDecimals(mintAddress) {
    const mintInfo = await connection.getParsedAccountInfo(new PublicKey(mintAddress));
    return mintInfo.value.data.parsed.info.decimals;
}
```

#### **B. Swap Interface (`dashboard/swap.js`)**
```javascript
// 🔧 CHANGE: Handle all conversions properly
function calculateSwapOutput(inputAmountDisplay, fromToken, toToken, poolData) {
    // Convert input to basis points
    const inputBasisPoints = inputAmountDisplay * Math.pow(10, fromToken.decimals);
    
    // Get pool ratios (already in basis points from contract)
    const numerator = poolData.ratioANumerator;    // basis points
    const denominator = poolData.ratioBDenominator; // basis points
    
    // Calculate in basis points
    const outputBasisPoints = inputBasisPoints * numerator / denominator;
    
    // Convert back to display units
    const outputDisplay = outputBasisPoints / Math.pow(10, toToken.decimals);
    
    return outputDisplay;
}
```

#### **C. Display Utilities (`dashboard/utils.js`)**
```javascript
// 🔧 ADD: Basis point conversion utilities
window.TokenUtils = {
    displayToBasisPoints(displayAmount, decimals) {
        return Math.floor(displayAmount * Math.pow(10, decimals));
    },
    
    basisPointsToDisplay(basisPoints, decimals) {
        return basisPoints / Math.pow(10, decimals);
    },
    
    formatTokenAmount(basisPoints, decimals, precision = 6) {
        const displayAmount = this.basisPointsToDisplay(basisPoints, decimals);
        return displayAmount.toFixed(precision);
    }
};
```

## 📋 **Implementation Order** 
*(Full Replacement Approach - No Backward Compatibility)*

### **Phase 1: Smart Contract Core** *(Complete Replacement)*
1. ✅ **Replace** existing pool creation logic with basis point assumption
2. ✅ **Replace** swap calculations with corrected basis point logic  
3. ✅ **Replace** liquidity tracking with basis point handling
4. ✅ **Replace** one-to-many validation with corrected logic
5. ✅ **Remove** any existing decimal conversion code from contract

### **Phase 2: Test Suite** *(Complete Rewrite)*
1. ✅ **Replace** all test utilities with basis point conversion functions
2. ✅ **Rewrite** pool creation tests with basis point inputs
3. ✅ **Rewrite** swap calculation tests with corrected expectations
4. ✅ **Rewrite** liquidity management tests with basis point values
5. ✅ **Test in fresh devnet** environment after reset

### **Phase 3: Dashboard** *(Complete Overhaul)*
1. ✅ **Replace** pool creation form with decimal detection and conversion
2. ✅ **Replace** swap interface with proper basis point handling
3. ✅ **Replace** liquidity interface with corrected calculations
4. ✅ **Replace** all display utilities with consistent formatting
5. ✅ **Remove** old conversion logic completely

### **Phase 4: Validation** *(Fresh Testing)*
1. ✅ **Create** new end-to-end test scenarios
2. ✅ **Validate** cross-component consistency from scratch
3. ✅ **Test** with fresh pools using corrected logic

## 🧪 **Testing Strategy**
*(Fresh Implementation - No Legacy Compatibility)*

### **Test Cases to Validate**
```rust
// Example comprehensive test - fresh implementation
#[tokio::test]
async fn test_end_to_end_basis_point_consistency() {
    // Create pool: 1.0 USDC (6 decimals) = 1000.0 BONK (9 decimals)
    // Client converts to basis points: 1_000_000 and 1_000_000_000_000
    let pool = create_pool_basis_points(1_000_000, 1_000_000_000_000).await;
    
    // Swap: 5.5 USDC → ? BONK (client converts 5.5 * 10^6 = 5_500_000)
    let input_usdc_basis_points = 5_500_000;
    let result = swap_tokens_basis_points(input_usdc_basis_points).await;
    
    // Expected: 5_500_000 * 1_000_000_000_000 / 1_000_000 = 5_500_000_000_000
    assert_eq!(result.output_amount_basis_points, 5_500_000_000_000);
    
    // Verify no legacy conversion logic exists
    assert_no_legacy_decimal_handling();
}
```

## 📖 **Documentation Requirements**

### **For Developers**
- Clear basis point conversion examples
- Token decimal handling best practices
- Common pitfalls and how to avoid them

### **For Users**  
- Transparent conversion explanations
- Precision limitations disclosure
- Expected vs actual amount clarifications

## 🚦 **Migration Strategy**

### **🚨 No Backward Compatibility Required**
**IMPORTANT**: This project is **not yet released**, therefore:
- **No backward compatibility needed** - old code will be completely replaced
- **No migration scripts required** - devnet reset provides fresh environment
- **No version detection needed** - fresh start with new basis point logic
- **No existing pools to concern** - devnet reset means clean slate
- **Breaking changes are acceptable** - no production users to impact

### **Deployment Plan**
1. **Replace existing contract logic** with basis point implementation
2. **Update all test cases** to use basis point values
3. **Replace dashboard conversion logic** completely
4. **Reset devnet environment** for fresh start with corrected logic
5. **Deploy clean implementation** without compatibility layers

### **Development Strategy**
- **Full replacement approach** - no need to maintain old logic
- **Clean slate implementation** - devnet reset removes any existing decimal handling inconsistencies
- **Direct testing** - validate new logic in fresh devnet environment
- **Simplified codebase** - no complex compatibility layers or migration code

---

## ⚠️ **Risks & Mitigations**

### **High Risk**
- **Math precision errors**: Use checked arithmetic throughout
- **Decimal overflow**: Validate max values for each operation  
- **Display inconsistencies**: Comprehensive UI testing required

### **Medium Risk**
- **Complete code replacement**: Systematic testing of all components
- **User confusion**: Clear documentation and examples (no legacy users to worry about)
- **Performance impact**: Benchmark new conversion operations

### **Low Risk**
- **Code complexity**: Clean implementation without legacy compatibility layers
- **Maintenance overhead**: Simplified codebase without backward compatibility
- **Breaking changes**: Acceptable since project is pre-release

---

This refactor is **critical for system integrity** and will resolve the swap calculation bugs permanently. 