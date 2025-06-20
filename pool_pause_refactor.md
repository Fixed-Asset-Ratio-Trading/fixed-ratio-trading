# Refactor Plan: Delegate Pause/Unpause System

> **⚠️ BREAKING CHANGE REFACTOR**: This plan completely removes the old `PoolPause` system and replaces it with proper `PausePool`/`UnpausePool` functionality. No backward compatibility will be maintained since the contract has not been deployed yet.

## Current Problem Analysis
- **Flawed Design**: Current `PoolPause` requires fixed duration and auto-unpauses
- **Missing Functionality**: No delegate-initiated unpause capability
- **Inconsistent UX**: Owner can pause/unpause instantly, but delegates have broken system

## Desired Design
- **Pause Request**: Delegate requests pause → wait period → execute to pause indefinitely
- **Unpause Request**: Delegate requests unpause → wait period → execute to unpause
- **Owner Override**: Pool owner can cancel any pending pause/unpause request
- **No Auto-Unpause**: Pool stays paused until manually unpaused

---

## 📋 TASK LIST

### **PHASE 1: Core Type System Changes**

#### **Task 1.1: Replace Delegate Action Types**
- **File**: `src/types/delegate_actions.rs`
- **Changes**:
  - Replace `PoolPause` with `PausePool` and `UnpausePool` in `DelegateActionType` enum
  - Replace old `PoolPause` params with new `PausePool` and `UnpausePool` variants in `DelegateActionParams`
  - Remove duration-based pause parameters completely
  - Update `Default` implementations

#### **Task 1.2: Update Pool State Types**
- **File**: `src/types/pool_state.rs`
- **Changes**:
  - Add `pause_requested_by` field to track who initiated current pause
  - Add `pause_initiated_timestamp` for audit trails
  - Consider adding `pause_history` for governance transparency

#### **Task 1.3: Update Instruction Types**
- **File**: `src/types/instructions.rs`
- **Changes**:
  - Ensure `RequestDelegateAction` handles new action types
  - Update instruction serialization/deserialization

---

### **PHASE 2: Core Processing Logic**

#### **Task 2.1: Replace Delegate Action Processor Logic**
- **File**: `src/processors/delegate_actions.rs`
- **Changes**:
  - Replace `process_request_delegate_action()`:
    - Remove old `PoolPause` handling completely
    - Implement `PausePool` and `UnpausePool` action types
    - Validate pause requests (can't pause if already paused, etc.)
    - Validate unpause requests (can't unpause if not paused, etc.)
  - Replace `process_execute_delegate_action()`:
    - Remove old duration-based pause execution
    - Implement `PausePool` execution (set `is_paused = true`, no end timestamp)
    - Implement `UnpausePool` execution (set `is_paused = false`, clear pause data)
    - Update pool state tracking fields
  - Replace `validate_action_params()`:
    - Remove old `PoolPause` validation completely
    - Add validation for new `PausePool` and `UnpausePool` types

#### **Task 2.2: Update Action Cancellation**
- **File**: `src/processors/delegate_actions.rs`
- **Changes**:
  - Update `process_revoke_action()`:
    - Allow pool owner to cancel any pause/unpause request
    - Allow delegate to cancel their own requests
    - Add specific error messages for pause/unpause cancellations

#### **Task 2.3: Replace Pool State Validation**
- **File**: `src/utils/validation.rs`
- **Changes**:
  - Replace `validate_pool_not_paused()`:
    - Remove all auto-unpause logic completely
    - Pool stays paused until explicitly unpaused via delegate action
    - Add clear logging about pause state and who initiated it

---

### **PHASE 3: Security and Access Control**

#### **Task 3.1: Enhanced Authorization**
- **File**: `src/processors/delegate_actions.rs`
- **Changes**:
  - Add validation to prevent conflicting pause/unpause requests
  - Ensure only one pause/unpause action per delegate at a time
  - Add owner override capabilities for emergency situations

#### **Task 3.2: State Consistency Checks**
- **File**: `src/utils/validation.rs`
- **Changes**:
  - Add `validate_pause_request_logic()`:
    - Can't request pause if already paused
    - Can't request unpause if not paused
    - Can't have multiple conflicting requests
  - Add comprehensive state validation for pause transitions

---

### **PHASE 4: Testing Infrastructure**

#### **Task 4.1: Replace Existing Tests**
- **File**: `tests/test_delegates.rs`
- **Changes**:
  - Replace `test_request_delegate_action_pool_pause()`:
    - Remove all old `PoolPause` tests completely
    - Implement new `PausePool` action type tests
    - Implement new `UnpausePool` action type tests
  - Replace `test_execute_delegate_action_success()`:
    - Remove old duration-based pause execution tests
    - Test execution of new pause/unpause actions
    - Verify state changes are correct
    - Test wait time enforcement

#### **Task 4.2: New Comprehensive Pause/Unpause Tests**
- **File**: `tests/test_delegates.rs`
- **Changes**:
  - Add `test_delegate_pause_unpause_cycle()`:
    - Request pause → wait → execute pause → verify paused
    - Request unpause → wait → execute unpause → verify unpaused
  - Add `test_owner_cancels_pause_request()`:
    - Delegate requests pause → owner cancels → verify cannot execute
  - Add `test_conflicting_pause_requests()`:
    - Test multiple delegates trying to pause/unpause
    - Verify proper error handling

#### **Task 4.3: State Validation Tests**
- **File**: `tests/test_delegates.rs`
- **Changes**:
  - Add `test_pause_state_validation()`:
    - Test can't pause if already paused
    - Test can't unpause if not paused
    - Test proper error messages
  - Add `test_pause_unpause_edge_cases()`:
    - Test rapid pause/unpause requests
    - Test owner vs delegate pause conflicts

---

### **PHASE 5: Documentation and Examples**

#### **Task 5.1: Update API Documentation**
- **File**: `docs/` directory
- **Changes**:
  - Create `PAUSE_UNPAUSE_GUIDE.md`:
    - Explain new pause/unpause workflow
    - Show example transactions
    - Document wait times and governance process
  - Update existing delegate documentation

#### **Task 5.2: Update Code Comments**
- **Files**: All modified source files
- **Changes**:
  - Add comprehensive doc comments for new functions
  - Update existing comments to reflect new behavior
  - Add examples in doc comments

#### **Task 5.3: Update README**
- **File**: `README.md`
- **Changes**:
  - Document new pause/unpause functionality
  - Add examples of delegate governance workflow
  - Update feature list

---

### **PHASE 6: Code Cleanup and Removal**

#### **Task 6.1: Remove Old Pause Code**
- **File**: `src/types/delegate_actions.rs`
- **Changes**:
  - Remove all references to old `PoolPause` enum variant
  - Remove old `PoolPause` parameters from `DelegateActionParams`
  - Clean up any unused imports or dependencies

#### **Task 6.2: Clean Up Processing Logic**
- **File**: `src/processors/delegate_actions.rs`
- **Changes**:
  - Remove all old `PoolPause` handling code completely
  - Remove duration-based pause logic and calculations
  - Remove auto-unpause mechanisms
  - Clean up any unused helper functions

#### **Task 6.3: Remove Old Test Infrastructure**
- **File**: `tests/test_delegates.rs`
- **Changes**:
  - Remove all test constants related to old pause system (duration constants, etc.)
  - Remove any helper functions specific to old `PoolPause` testing
  - Clean up test setup code that supported old pause functionality
  - Remove old test documentation and comments

---

### **PHASE 7: Error Handling and Edge Cases**

#### **Task 7.1: Enhanced Error Messages**
- **File**: `src/error.rs`
- **Changes**:
  - Add specific error types:
    - `CannotPauseAlreadyPaused`
    - `CannotUnpauseNotPaused`
    - `ConflictingPauseRequest`
    - `PauseRequestCancelled`

#### **Task 7.2: Edge Case Handling**
- **Files**: Various processor files
- **Changes**:
  - Handle delegate removal while pause request pending
  - Handle owner transfer during pause governance
  - Handle multiple delegates requesting conflicting actions
  - Remove any edge case handling for old `PoolPause` system

---

### **PHASE 8: Integration and End-to-End Testing**

#### **Task 8.1: Integration Test Suite**
- **File**: `tests/test_integration.rs` (new)
- **Changes**:
  - Create full workflow tests:
    - Complete pause/unpause cycles
    - Owner cancellation scenarios
    - Multi-delegate governance scenarios
  - Test interaction with other pool operations

#### **Task 8.2: Security Testing**
- **File**: `tests/test_security.rs`
- **Changes**:
  - Test pause/unpause during various pool states
  - Test attack vectors (rapid requests, etc.)
  - Verify proper access control

---

## 🔄 **EXECUTION ORDER**

1. **Phase 1**: Core type system changes (foundation)
2. **Phase 2**: Processing logic implementation
3. **Phase 3**: Security and validation
4. **Phase 4**: Basic testing
5. **Phase 5**: Documentation
6. **Phase 6**: Code cleanup and removal
7. **Phase 7**: Error handling
8. **Phase 8**: Integration testing

## 📊 **METRICS FOR SUCCESS**

- [ ] Delegates can request pause with time delay
- [ ] Delegates can request unpause with time delay  
- [ ] Pool owner can cancel any pending pause/unpause request
- [ ] No auto-unpause (manual control only)
- [ ] Old `PoolPause` system completely removed
- [ ] Comprehensive test coverage (>95%)
- [ ] Clear documentation and examples
- [ ] Clean, maintainable codebase with no legacy code

## ⚠️ **RISK CONSIDERATIONS**

- **State Complexity**: Track pause initiator and timestamps carefully
- **Race Conditions**: Handle multiple simultaneous pause/unpause requests
- **Code Cleanup**: Ensure all old pause system references are completely removed
- **Governance Conflicts**: Handle owner vs delegate authority clearly
- **Testing Coverage**: Ensure all old test cases are properly replaced with new functionality

## 🎯 **KEY DESIGN PRINCIPLES**

1. **Time-Delayed Governance**: All delegate actions require wait period before execution
2. **Owner Override**: Pool owner can always cancel pending requests for emergency control
3. **Manual Control**: No automatic state changes - all pause/unpause must be explicitly requested and executed
4. **Clear State Tracking**: Always know who initiated pause and when
5. **Clean Architecture**: Complete removal of legacy pause system for maintainable codebase
6. **Comprehensive Validation**: Prevent invalid state transitions and conflicting requests

This plan provides a complete roadmap for implementing proper pause/unpause functionality while maintaining the security model of time-delayed governance with owner override capabilities. 