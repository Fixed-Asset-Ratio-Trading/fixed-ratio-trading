# lib.rs Refactor Plan

## Overview

Current `lib.rs` is a monolithic file with **4,982 lines** containing all program logic. This refactor will break it into focused, manageable modules that AI systems with small context windows can easily understand and modify.

## 🎯 Goals

1. **Improve Maintainability** - Each module focuses on a single responsibility
2. **Enable AI Collaboration** - Small, focused files fit within AI context windows
3. **Better Organization** - Logical separation of concerns
4. **Preserve Functionality** - No breaking changes to existing API
5. **Enhanced Testing** - Easier to unit test individual modules

## 📁 Proposed Module Structure

```
src/
├── lib.rs                     # Main entry point and re-exports (~150 lines)
├── constants.rs               # All constants and configuration (~100 lines)
├── types/
│   ├── mod.rs                 # Type re-exports
│   ├── pool_state.rs          # PoolState struct and implementation (~200 lines)
│   ├── instructions.rs        # PoolInstruction enum (~300 lines)
│   ├── errors.rs              # PoolError enum and implementations (~200 lines)
│   └── governance.rs          # Delegate and pause-related types (~400 lines)
├── processors/
│   ├── mod.rs                 # Processor re-exports
│   ├── pool_creation.rs       # Pool initialization processors (~800 lines)
│   ├── liquidity.rs           # Deposit/withdraw processors (~600 lines)
│   ├── trading.rs             # Swap processor (~400 lines)
│   ├── fees.rs                # Fee-related processors (~500 lines)
│   ├── governance.rs          # Delegate management processors (~800 lines)
│   ├── security.rs            # Security and pause processors (~400 lines)
│   └── utilities.rs           # Helper and view processors (~300 lines)
└── utils/
    ├── mod.rs                 # Utility re-exports
    ├── validation.rs          # Input validation helpers (~200 lines)
    ├── serialization.rs       # Serialization utilities (~150 lines)
    └── rent.rs                # Rent calculation utilities (~100 lines)
```

## 📋 Detailed Module Breakdown

### 1. **lib.rs** (Main Entry Point)
**Size**: ~150 lines  
**Purpose**: Program entry point and public API

**Contents**:
- Program ID declaration
- Module declarations and re-exports
- Main `process_instruction` function (dispatcher only)
- Public API exports for external use

**Dependencies**: All internal modules

---

### 2. **constants.rs** 
**Size**: ~100 lines  
**Purpose**: Centralized configuration and constants

**Contents**:
- Fee constants (`REGISTRATION_FEE`, `SWAP_FEE`, etc.)
- Delegate system constants (`MAX_DELEGATES`, `MIN_WITHDRAWAL_WAIT_TIME`)
- PDA seed prefixes (`POOL_STATE_SEED_PREFIX`, etc.)
- Swap fee configuration (`MAX_SWAP_FEE_BASIS_POINTS`)
- Buffer constants (`MINIMUM_RENT_BUFFER`)

**Dependencies**: None (pure constants)

---

### 3. **types/mod.rs** (Type Definitions Module)
**Purpose**: All data structures and enums

#### **types/pool_state.rs**
**Size**: ~200 lines  
**Purpose**: Core pool state management

**Contents**:
- `PoolState` struct definition
- `RentRequirements` struct  
- Implementation methods for state management
- Serialization size calculations

#### **types/instructions.rs**
**Size**: ~300 lines  
**Purpose**: Instruction definitions

**Contents**:
- `PoolInstruction` enum with all variants
- Instruction parameter structures
- Borsh serialization derives

#### **types/errors.rs**
**Size**: ~200 lines  
**Purpose**: Error handling

**Contents**:
- `PoolError` enum with all variants
- Error code mappings
- Display implementations
- `From<PoolError>` for `ProgramError`

#### **types/governance.rs**
**Size**: ~400 lines  
**Purpose**: Governance and delegate types

**Contents**:
- `DelegateManagement` struct and all methods
- `WithdrawalRecord` and `WithdrawalRequest` structs
- `PoolPauseRequest` and `PoolPauseReason` types
- All governance-related implementations

---

### 4. **processors/mod.rs** (Instruction Processors)
**Purpose**: Business logic implementation

#### **processors/pool_creation.rs**
**Size**: ~800 lines  
**Purpose**: Pool initialization logic

**Contents**:
- `process_create_pool_state_account` (deprecated)
- `process_initialize_pool_data` (deprecated) 
- `process_initialize_pool` (recommended)
- Pool normalization logic
- Account creation and validation

#### **processors/liquidity.rs**
**Size**: ~600 lines  
**Purpose**: Liquidity management

**Contents**:
- `process_deposit`
- `process_deposit_with_features`
- `process_withdraw`
- LP token minting/burning logic
- Liquidity calculations

#### **processors/trading.rs**
**Size**: ~400 lines  
**Purpose**: Token swapping

**Contents**:
- `process_swap`
- Exchange rate calculations
- Fee collection during swaps
- Slippage protection

#### **processors/fees.rs**
**Size**: ~500 lines  
**Purpose**: Fee management

**Contents**:
- `process_withdraw_fees`
- `process_withdraw_fees_to_delegate`
- `process_set_swap_fee`
- `process_request_fee_withdrawal`
- `process_cancel_withdrawal_request`
- Fee calculation utilities

#### **processors/governance.rs**
**Size**: ~800 lines  
**Purpose**: Delegate and governance

**Contents**:
- `process_add_delegate`
- `process_remove_delegate`
- `process_set_delegate_wait_time`
- `process_request_pool_pause`
- `process_cancel_pool_pause`
- `process_set_pool_pause_wait_time`
- Delegate authorization logic

#### **processors/security.rs**
**Size**: ~400 lines  
**Purpose**: Security and pausing

**Contents**:
- `process_update_security_params`
- Pool pause enforcement
- Security parameter validation
- Emergency controls

#### **processors/utilities.rs**
**Size**: ~300 lines  
**Purpose**: Helper and view functions

**Contents**:
- `get_pool_state_pda`
- `get_token_vault_pdas`
- `get_pool_info`
- `get_liquidity_info`
- `get_delegate_info`
- `get_fee_info`
- `process_get_withdrawal_history`

---

### 5. **utils/mod.rs** (Utility Functions)
**Purpose**: Shared helper functions

#### **utils/validation.rs**
**Size**: ~200 lines  
**Purpose**: Input validation

**Contents**:
- Account validation helpers
- Parameter range checking
- Token mint validation
- Authorization checks

#### **utils/serialization.rs**
**Size**: ~150 lines  
**Purpose**: Serialization utilities

**Contents**:
- Buffer serialization patterns
- Size calculation helpers
- Borsh serialization utilities
- Account data management

#### **utils/rent.rs**
**Size**: ~100 lines  
**Purpose**: Rent calculations

**Contents**:
- `check_rent_exempt`
- `ensure_rent_exempt`
- Rent requirement calculations
- Account balance validation

---

## 🔄 Migration Strategy

### Phase 1: Create Module Structure
1. Create all module directories and files
2. Move constants to `constants.rs`
3. Set up basic module declarations in `lib.rs`

### Phase 2: Extract Type Definitions
1. Move `PoolState` to `types/pool_state.rs`
2. Move `PoolInstruction` to `types/instructions.rs`
3. Move `PoolError` to `types/errors.rs`
4. Move governance types to `types/governance.rs`

### Phase 3: Extract Processors
1. Move pool creation functions to `processors/pool_creation.rs`
2. Move liquidity functions to `processors/liquidity.rs`
3. Move trading functions to `processors/trading.rs`
4. Move fee functions to `processors/fees.rs`
5. Move governance functions to `processors/governance.rs`
6. Move security functions to `processors/security.rs`
7. Move utility functions to `processors/utilities.rs`

### Phase 4: Extract Utilities
1. Move validation helpers to `utils/validation.rs`
2. Move serialization helpers to `utils/serialization.rs`
3. Move rent helpers to `utils/rent.rs`

### Phase 5: Update Main Entry Point
1. Update `lib.rs` to import and re-export all modules
2. Update `process_instruction` to dispatch to appropriate processors
3. Verify all tests still pass

## 🧪 Testing Strategy

1. **Run all existing tests** after each phase to ensure no regressions
2. **Gradual migration** - move one module at a time
3. **Preserve all public APIs** - no breaking changes to external interface
4. **Verify compilation** after each module extraction

## 📊 Benefits for AI Systems

### **Context Window Optimization**
- **Before**: 4,982 lines in single file (exceeds most AI context limits)
- **After**: Largest module ~800 lines (fits comfortably in context windows)

### **Focused Understanding**
- **AI can focus on specific functionality** without overwhelming context
- **Easier to understand dependencies** between modules
- **Clear separation of concerns** makes modifications safer

### **Collaborative Development**
- **Multiple AI systems** can work on different modules simultaneously
- **Reduced merge conflicts** due to logical separation
- **Easier code review** of smaller, focused changes

## 🎯 Success Criteria

1. ✅ **All existing tests pass** after refactor
2. ✅ **No breaking changes** to public API
3. ✅ **Each module under 1000 lines** for AI compatibility
4. ✅ **Clear module boundaries** with minimal cross-dependencies
5. ✅ **Comprehensive documentation** for each module
6. ✅ **Zero compilation warnings** after refactor

## 🚀 Implementation Priority

1. **High Priority**: `constants.rs`, `types/errors.rs`, `types/instructions.rs`
2. **Medium Priority**: `types/pool_state.rs`, `processors/trading.rs`, `processors/liquidity.rs`
3. **Low Priority**: `utils/*`, `processors/utilities.rs`

This refactor will transform the codebase into a modular, maintainable structure that's perfect for AI collaboration while preserving all existing functionality. 