# Comprehensive Testing Plan - Fixed Ratio Trading

File Name : COMPREHENSIVE_TESTING_PLAN.md

## Executive Summary
**Current Coverage:** 29.76% (576/1935 lines covered)  
**Target Coverage:** 85%+ (1,645+ lines covered)  
**Total Tests Implemented:** 68 passing tests  
**Total Tests Needed:** ~42 additional tests  
**Estimated Timeline:** 3-4 weeks

## Testing Philosophy & Bug Fix Policy

### Core Principles:
- **High Priority First**: Critical business logic with 0% coverage
- **Sequential Implementation**: One test at a time with developer approval
- **Continuous Improvement**: Update plan based on discoveries
- **Clear Progress Tracking**: Visible milestones and completion status

### 🔧 CONTRACT BUG FIX POLICY:
**When tests reveal bugs in the contract code, we fix the contract rather than work around issues, since the contract is not yet deployed.**

- ✅ **Fix contract bugs immediately** when discovered during testing
- ✅ **Update all affected tests** after contract fixes
- ✅ **Document fixes** in the testing plan progress notes
- ❌ **No workarounds** - ensure tests verify correct functionality
- ✅ **Test-driven fixes** - let good tests drive better contract code

### 📝 GIT COMMIT MESSAGE FORMAT:
**All test completion commits must follow this exact format:**

```
test: Complete LIQ-XXX <description> - <summary of work done>
```

**Examples:**
- `test: Complete LIQ-004 zero amount deposit validation test - Add validation, implement test, update plan`
- `test: Complete LIQ-005 wrong token deposit validation test - Add test, update plan`
- `test: Complete LIQ-006 insufficient balance deposit validation test - Add test, update plan`

**Format Requirements:**
- Start with `test:` prefix
- Include the test ID (LIQ-XXX, FEE-XXX, etc.)
- Brief description of what the test validates
- Dash separator followed by summary of changes made

## Progress Overview
- Current Coverage: 29.76%
- Target Coverage: 85%+
- Total Tests Running: 68 passing tests
- Tests Completed in Phase 1: 14/20
- Estimated Timeline: 3-4 weeks
- Additional Tests Needed: ~42

## PHASE 1: HIGH PRIORITY TESTS 🚨
*Critical business logic with significant coverage improvements*

### Module 1: Liquidity Management (0% → 80% target)
**Status:** ✅ Complete (9/9 completed) | **Priority:** Critical | **File:** `src/processors/liquidity.rs`

#### Sub-category 1.1: Deposit Operations
- [x] **LIQ-001** `test_basic_deposit_success` - Basic token deposit functionality ✅ **COMPLETED**
  - **🔧 CRITICAL BUG FIXES APPLIED**: 
    1. Fixed `process_instruction` pause checking to use correct pool state account index for each instruction type (was assuming accounts[0] for all instructions)
    2. **MAJOR**: Added missing entrypoint declaration - contract wasn't being called at all
    3. **BUFFER SERIALIZATION WORKAROUND**: Applied known Solana fix for PDA data corruption during invoke_signed operations (same pattern as process_initialize_pool_data)
  - **✅ COMPLETED**: Contract now working (18 tests pass), instruction serialization confirmed working
  - **📚 DOCUMENTATION ADDED**: Comprehensive documentation of Buffer Serialization Workaround for future developers
    - Module-level documentation explaining the PDA data corruption issue
    - Inline documentation with detailed problem/solution explanation
    - References to when and how to use this pattern
- [x] **LIQ-002** `test_deposit_with_features_success` - Advanced deposit with slippage protection ✅ **COMPLETED**
  - **✅ COMPLETED**: Successfully tests `process_deposit_with_features` function
  - **🔧 FEATURES TESTED**: 
    1. Slippage protection with minimum LP token guarantees (10% tolerance tested)
    2. Proper 1:1 LP token minting for fixed-ratio deposits
    3. Custom fee recipient option (currently logs intent)
    4. Enhanced deposit validation and error handling
  - **📊 TEST COVERAGE**: Both success case and slippage protection failure case
  - **🎯 RESULTS**: Deposited 500K tokens → received 500K LP tokens, slippage protection correctly triggers with Custom(2001) error
- [x] **LIQ-003** `test_deposit_insufficient_tokens_fails` - Insufficient balance error handling ✅ **COMPLETED**
  - **✅ COMPLETED**: Successfully tests insufficient balance error handling
  - **🔧 FEATURES TESTED**:
    1. Proper error handling when attempting to deposit more tokens than available
    2. State consistency after failed deposit attempt
    3. Verification that no LP tokens are minted on failure
  - **📊 TEST COVERAGE**: Error case for insufficient balance in deposit operations
  - **🎯 RESULTS**: Correctly returns InsufficientFunds error, maintains account state integrity
- [x] **LIQ-004** `test_deposit_zero_amount_fails` - Zero amount validation ✅ **COMPLETED**
  - **✅ COMPLETED**: Successfully tests zero amount validation
  - **🔧 FEATURES TESTED**:
    1. Proper error handling when attempting to deposit zero tokens
    2. State consistency after failed deposit attempt
    3. Verification that no LP tokens are minted on failure
  - **📊 TEST COVERAGE**: Error case for zero amount in deposit operations
  - **🎯 RESULTS**: Correctly returns InvalidArgument error, maintains account state integrity
- [x] **LIQ-005** `test_deposit_wrong_token_fails` - Invalid token mint validation ✅ **COMPLETED**
- [x] **LIQ-006** `test_deposit_insufficient_balance_fails` - Insufficient balance validation ✅ **COMPLETED**

#### Sub-category 1.2: Withdrawal Operations  
- [x] **LIQ-007** `test_basic_withdrawal_success` - Basic LP token withdrawal ✅ **COMPLETED**
  - **✅ COMPLETED**: Successfully tests basic withdrawal functionality
  - **🔧 FEATURES TESTED**:
    1. LP token burning during withdrawal
    2. Underlying token transfer back to user
    3. Pool state updates after withdrawal
    4. 1:1 ratio maintained between LP tokens and underlying tokens
  - **📊 TEST COVERAGE**: Full withdrawal flow from deposit to withdrawal
  - **🎯 RESULTS**: Successfully withdrew 1M tokens, verified all state changes
- [x] **LIQ-008** `test_withdrawal_insufficient_lp_fails` - Insufficient LP tokens error ✅ **COMPLETED**
  - **✅ COMPLETED**: Successfully tests insufficient LP token error handling
  - **🔧 FEATURES TESTED**:
    1. Proper error handling when attempting to withdraw more LP tokens than available
    2. State consistency after failed withdrawal attempt
    3. Verification that no LP tokens are burned on failure
    4. Verification that no underlying tokens are transferred
  - **📊 TEST COVERAGE**: Error case for insufficient LP tokens in withdrawal operations
  - **🎯 RESULTS**: Correctly returns InsufficientFunds error, maintains account state integrity
- [x] **LIQ-009** `test_withdrawal_delegate_process` - Two-step withdrawal validation ✅ **COMPLETED**
  - **✅ COMPLETED**: Successfully tests the two-step delegate withdrawal process
  - **🔧 FEATURES TESTED**:
    1. Delegate must request withdrawal through `process_request_delegate_action`
    2. Request enters waiting period for owner review
    3. Owner can cancel withdrawal during waiting period
    4. Only approved withdrawals can be executed
  - **📊 TEST COVERAGE**: Full validation of two-step withdrawal security process
  - **🎯 RESULTS**: Ensures withdrawals follow proper security protocol; all steps and error cases pass as expected

**Milestone 1.1:** ✅ Complete basic deposit/withdrawal functionality (Tests LIQ-001 to LIQ-009)

---

### Module 2: Fee Management (0% → 85% target)
**Status:** ✅ Complete (5/5 completed) | **Priority:** Critical | **File:** `src/processors/fees.rs`

#### Sub-category 2.1: Fee Withdrawal
- [x] **FEE-001** `test_withdraw_fees_success` - Basic fee withdrawal by owner ✅ **COMPLETED**
  - **🔧 CRITICAL BUG FIXES APPLIED**: 
    1. Fixed `process_instruction` pause checking to use correct pool state account index for each instruction type (was assuming accounts[0] for all instructions)
    2. **MAJOR**: Added missing entrypoint declaration - contract wasn't being called at all
    3. **BUFFER SERIALIZATION WORKAROUND**: Applied known Solana fix for PDA data corruption during invoke_signed operations (same pattern as process_initialize_pool_data)
  - **✅ COMPLETED**: Contract now working (18 tests pass), instruction serialization confirmed working
  - **📚 DOCUMENTATION ADDED**: Comprehensive documentation of Buffer Serialization Workaround for future developers
- [x] **FEE-002** `test_withdraw_fees_unauthorized_fails` - Non-owner fee withdrawal rejection ✅ **COMPLETED**
  - **✅ COMPLETED**: Successfully tests unauthorized fee withdrawal prevention
  - **🔧 FEATURES TESTED**: 
    1. Owner authorization verification
    2. Proper error handling for unauthorized attempts
    3. Transaction rejection with appropriate error
    4. State protection from unauthorized modifications
- [x] **FEE-003** `test_withdraw_fees_insufficient_balance` - Insufficient fee balance handling ✅ **COMPLETED**
  - **✅ COMPLETED**: Successfully tests behavior when withdrawing from a pool with only rent-exempt balance
  - **🔧 FEATURES TESTED**:
    1. Rent-exempt minimum balance protection
    2. Error handling for insufficient fee scenarios
    3. Proper transaction behavior with error code verification
    4. Balance preservation when no excess fees available
- [x] **FEE-004** `test_withdraw_fees_both_tokens` - Withdrawal of both token types ✅ **COMPLETED**
  - **✅ COMPLETED**: Successfully tests withdrawal of both token types through delegate system
  - **🔧 FEATURES TESTED**:
    1. Token A and Token B fee withdrawal
    2. Delegate authorization and validation
    3. Fee collection state tracking
    4. Proper token transfers and balance updates
    5. Pool state consistency after withdrawals
  - **🎯 RESULTS**: Successfully withdrew both token types with proper state updates
- [x] **FEE-005** `test_withdraw_fees_zero_balance` - No fees available scenario ✅ **COMPLETED**
  - **✅ COMPLETED**: Successfully tests behavior when pool has exactly rent-exempt minimum balance
  - **🔧 FEATURES TESTED**:
    1. Transaction success with no transfer when at rent-exempt minimum
    2. Balance preservation for both pool and owner accounts
    3. Proper handling of zero-fee state
    4. State consistency after attempted withdrawal
  - **🎯 RESULTS**: Successfully verified no-fee withdrawal behavior

**Milestone 1.2:** ✅ Complete - Fee management functionality (5/5 tests completed)

---

### Module 3: Client SDK (partial → 90% target)
**Status:** 🟡 In Progress | **Priority:** Critical | **File:** `src/client_sdk.rs`

#### Sub-category 3.1: Client Initialization & Core Methods
- [ ] **SDK-001** `test_pool_client_new` - PoolClient initialization
- [ ] **SDK-002** `test_derive_pool_addresses` - PDA derivation accuracy
- [ ] **SDK-003** `test_create_pool_instruction` - Pool creation instruction building
- [ ] **SDK-004** `test_get_pool_state_success` - Pool state retrieval
- [ ] **SDK-005** `test_get_pool_state_not_found` - Non-existent pool handling

**Milestone 1.3:** ✅ Complete core SDK functionality (Tests SDK-001 to SDK-005)

---

## PHASE 2: MEDIUM PRIORITY TESTS 🔶
*Important features with partial or missing coverage*

### Module 4: Consolidated Delegate Management (0% → 85% target)
**Status:** 🔴 Not Started | **Priority:** High | **File:** `src/processors/delegates.rs`

#### Sub-category 4.1: Action Request & Execution
- [ ] **DEL-001** `test_request_delegate_action_fee_change` - Fee change request
  - Test requesting fee change with valid parameters
  - Verify action is properly recorded with correct wait time
  - Ensure fee is not changed until execution
  - Validate fee change within allowed range (0-0.5%)

- [ ] **DEL-002** `test_request_delegate_action_withdrawal` - Withdrawal request
  - Test requesting fee withdrawal with valid amount
  - Verify withdrawal request is properly recorded
  - Ensure funds are not moved until execution
  - Validate withdrawal amount against available balance

- [ ] **DEL-003** `test_request_delegate_action_pool_pause` - Pool pause request
  - Test requesting pool pause with valid duration
  - Verify pause request is properly recorded
  - Ensure pool remains active until execution
  - Validate pause duration within allowed range

- [ ] **DEL-004** `test_execute_delegate_action_success` - Action execution
  - Test executing each type of delegate action
  - Verify wait time is enforced before execution
  - Ensure state changes are applied correctly
  - Validate action history is updated

#### Sub-category 4.2: Action Revocation & Time Limits
- [ ] **DEL-005** `test_revoke_action_success` - Action revocation
  - Test revoking pending actions by owner
  - Verify action is properly removed from pending list
  - Ensure state remains unchanged after revocation
  - Validate action history records revocation

- [ ] **DEL-006** `test_set_delegate_time_limits` - Time limit configuration
  - Test setting custom wait times for each action type
  - Verify limits are within allowed range
  - Ensure limits are applied per-delegate
  - Validate default limits for new delegates

#### Sub-category 4.3: Security & Validation
- [ ] **DEL-007** `test_unauthorized_action_request_fails` - Authorization checks
  - Test action requests from non-delegates
  - Verify unauthorized requests are rejected
  - Ensure proper error codes are returned
  - Validate no state changes occur

- [ ] **DEL-008** `test_early_execution_prevention` - Wait time enforcement
  - Test executing actions before wait time
  - Verify early execution attempts fail
  - Ensure proper error codes are returned
  - Validate wait time calculation accuracy

- [ ] **DEL-009** `test_rate_limiting_enforcement` - Rate limiting
  - Test rapid successive action requests
  - Verify maximum pending actions limit
  - Ensure proper action queuing
  - Validate action counting logic

#### Sub-category 4.4: Edge Cases & Error Handling
- [ ] **DEL-010** `test_invalid_action_parameters` - Parameter validation
  - Test invalid fee rates
  - Test invalid withdrawal amounts
  - Test invalid pause durations
  - Verify proper error handling

- [ ] **DEL-011** `test_concurrent_action_handling` - Concurrency handling
  - Test multiple pending actions
  - Test executing actions in order
  - Test revoking while pending execution
  - Verify state consistency

**Milestone 4.1:** ✅ Complete consolidated delegate management (Tests DEL-001 to DEL-011)

---

### Module 5: Swap Fee Management (10.6% → 80% target)
**Status:** 🔴 Not Started | **Priority:** High | **File:** `src/processors/swap.rs`

#### Sub-category 5.1: Fee Change Through Delegate Actions
- [ ] **SWAP-001** `test_fee_change_request_success` - Fee change request flow
  - Test requesting fee change through delegate action
  - Verify fee change request is properly recorded
  - Ensure fee remains unchanged during wait time
  - Validate new fee after execution

- [ ] **SWAP-002** `test_fee_change_validation` - Fee validation
  - Test fee changes within allowed range
  - Test fee changes exceeding maximum
  - Test zero fee setting
  - Verify proper error handling

- [ ] **SWAP-003** `test_fee_change_authorization` - Authorization checks
  - Test fee changes from authorized delegates
  - Test unauthorized fee change attempts
  - Test owner override capabilities
  - Verify proper permission enforcement

- [ ] **SWAP-004** `test_fee_change_timing` - Timing controls
  - Test fee change wait time enforcement
  - Test multiple fee changes in succession
  - Test fee change authorization timing
  - Verify timing calculation accuracy

#### Sub-category 5.2: Fee Collection & Distribution
- [ ] **SWAP-005** `test_fee_collection_accuracy` - Fee calculation
  - Test fee collection on swaps
  - Verify fee amount calculation accuracy
  - Test fee accumulation over multiple swaps
  - Validate fee balance tracking

- [ ] **SWAP-006** `test_fee_withdrawal_through_action` - Fee withdrawal
  - Test fee withdrawal through delegate action
  - Verify withdrawal amount validation
  - Test partial vs full withdrawals
  - Validate balance updates

**Milestone 5.1:** ✅ Complete swap fee management (Tests SWAP-001 to SWAP-006)

---

## PHASE 3: LOW PRIORITY TESTS 🔹
*Edge cases, error handling, and comprehensive coverage*

### Module 6: Validation Functions (0% → 85% target)
**Status:** 🔴 Not Started | **Priority:** Low | **File:** `src/utils/validation.rs`

#### Sub-category 6.1: Account Validation
- [ ] **VAL-001** `test_validate_account_owner_success` - Correct owner validation
- [ ] **VAL-002** `test_validate_account_owner_fails` - Wrong owner rejection
- [ ] **VAL-003** `test_validate_signer_success` - Signer validation
- [ ] **VAL-004** `test_validate_writable_success` - Writable account check

#### Sub-category 6.2: Business Logic Validation
- [ ] **VAL-005** `test_validate_swap_fee_success` - Valid fee range
- [ ] **VAL-006** `test_validate_non_zero_amount_success` - Non-zero validation
- [ ] **VAL-007** `test_validate_different_tokens_success` - Token differentiation
- [ ] **VAL-008** `test_validate_wait_time_success` - Wait time validation
- [ ] **VAL-009** `test_validate_pool_initialized_success` - Pool state validation
- [ ] **VAL-010** `test_validate_pool_not_paused_success` - Pause state validation

**Milestone 3.1:** ✅ Complete validation function coverage (Tests VAL-001 to VAL-010)

---

## Testing Infrastructure & Utilities

### Common Test Patterns
Each test will follow this structure:
```rust
#[tokio::test]
async fn test_name() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup test environment
    // 2. Create necessary accounts/mints
    // 3. Execute the function under test
    // 4. Assert expected outcomes
    // 5. Verify state changes
    Ok(())
}
```

### Success Criteria Per Test
- ✅ Compiles without warnings
- ✅ Passes all assertions
- ✅ Follows established test patterns
- ✅ Includes comprehensive error cases
- ✅ Maintains test isolation

---

## Milestone Tracking

### Phase 1 Milestones
- [x] **M1.1** - Liquidity Management Complete (9/9 tests completed) ✅
- [x] **M1.2** - Fee Management Complete (5/5 tests completed) ✅
- [ ] **M1.3** - Client SDK Complete (0/5 tests completed)
- [ ] **🎯 Phase 1 Complete** - All high priority tests passing (14/20 completed)

### Phase 2 Milestones
- [ ] **M2.1** - Consolidated Delegate Management Complete (11 tests)
- [ ] **M2.2** - Swap Fee Management Complete (6 tests)
- [ ] **🎯 Phase 2 Complete** - All medium priority tests passing

### Phase 3 Milestones
- [ ] **M3.1** - Validation Functions Complete (10 tests)
- [ ] **🎯 Phase 3 Complete** - All low priority tests passing

### Final Milestone
- [ ] **🏆 PROJECT COMPLETE** - Target 85%+ coverage achieved

---

## Continuous Improvement Process

### After Each Test Implementation:
1. **Run Coverage Analysis**: `cargo tarpaulin --verbose`
2. **Identify Additional Test Cases**: Review uncovered edge cases
3. **Update This Plan**: Add new tests if critical gaps discovered
4. **Developer Review**: Wait for approval before proceeding

### Quality Gates:
- Each test must pass individually
- No regression in existing tests
- Coverage increase measurable after each test
- Code review for test quality and completeness

---

## Risk Assessment

### High Risk Areas (Require Extra Attention):
- **Financial calculations** in liquidity management
- **Authorization checks** in fee and delegate operations  
- **State consistency** across concurrent operations
- **Error handling** for invalid inputs

### Mitigation Strategies:
- Comprehensive boundary testing
- Negative test cases for all validations
- State verification after each operation
- Integration tests for critical workflows

---

*Last Updated: 2025-06-18 (FEE-005 completed)*  
*Next Review: After each completed milestone* 