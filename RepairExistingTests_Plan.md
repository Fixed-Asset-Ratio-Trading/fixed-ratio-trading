# Test Repair Plan

## Phase 1: Test Assessment and Triage

### 1. Identify Working Tests
- [x] Run all existing tests and catalog which ones pass
- [x] Document any patterns in working tests
- [x] Create list of completely broken tests
- [ ] Decision point: If no tests work, evaluate whether to proceed with changes

#### Initial Assessment Results (2024-03-21):
1. **Current Status**: Some tests are working
   - Working Tests:
     - test_id
     - test_pool_owner_as_implicit_delegate
     - test_unauthorized_delegate_operation_fails
     - test_delegate_limit_enforcement
     - test_add_delegate_unauthorized_fails
   - Failing Tests:
     - test_delegate_request_fee_withdrawal_success (InvalidAccountData)
     - test_add_duplicate_delegate_fails (Not all bytes read)
     - test_add_multiple_delegates (Not all bytes read)
     - test_add_delegate_success (Pool owner should be able to add delegates)
     - test_delegate_authorization (Delegate addition should succeed)

2. **Main Issues Found**:
   - ~~Private type visibility issues (PoolInstruction, PoolError)~~ ✓ FIXED
   - ~~Missing imports for delegate action types~~ ✓ FIXED
   - New issues discovered:
     - ~~Missing variant `RequestFeeWithdrawal` in `PoolInstruction` enum~~ ✓ FIXED
     - ~~Error handling issues with Option/Result types in async functions~~ ✓ FIXED
     - ~~Ambiguous glob re-exports in lib.rs~~ ✓ FIXED
     - Data serialization/deserialization issues ("Not all bytes read")
     - Invalid account data errors in delegate operations

3. **Required Fixes**:
   a. ~~Update PoolInstruction enum to include RequestFeeWithdrawal variant or update tests to use new format~~ ✓ FIXED
   b. ~~Fix error handling in test functions (convert Option to Result properly)~~ ✓ FIXED
   c. ~~Clean up ambiguous re-exports in lib.rs~~ ✓ FIXED
   d. Fix data serialization/deserialization in delegate management
   e. Fix account validation in delegate operations

4. **Critical Decision Point**: Based on these findings, we should:
   1. Fix the data serialization issues in delegate management
   2. Fix the account validation in delegate operations
   3. Re-run tests after each fix to ensure progress

### 2. Test Categorization
- [ ] Group tests by functionality:
  - [ ] Core pool operations
  - [ ] Fee-related operations
  - [ ] Delegate management
  - [ ] Time-based authorization
  - [ ] State management

### 3. Dependencies Analysis
- [ ] Map test dependencies
- [ ] Identify shared setup code
- [ ] Document common failure points

## Phase 2: Systematic Test Repair

### 1. Core Pool Operations
- [ ] Comment out all failing tests
- [ ] Restore basic pool creation tests
- [ ] Restore token swap tests
- [ ] Restore pool state validation tests

### 2. Fee Management Tests
- [ ] Comment out complex fee scenarios
- [ ] Restore basic fee setting tests
- [ ] Restore fee calculation tests
- [ ] Restore fee collection tests

### 3. Delegate Management Tests
- [ ] Comment out all delegate tests
- [ ] Restore basic delegate assignment tests
- [ ] Restore delegate permission tests
- [ ] Restore delegate action tests

### 4. Time-based Authorization Tests
- [ ] Comment out all time-based tests
- [ ] Restore basic wait time tests
- [ ] Restore authorization period tests
- [ ] Restore time validation tests

## Phase 3: Integration Testing

### 1. Combined Operations
- [ ] Restore tests combining multiple operations
- [ ] Validate state transitions
- [ ] Test error conditions

### 2. Edge Cases
- [ ] Restore boundary condition tests
- [ ] Test maximum/minimum values
- [ ] Test invalid inputs

## Decision Points

### Critical Go/No-Go Checkpoints
1. [ ] **Initial Assessment Complete**
   - If no tests work: Consider rolling back changes
   - If some tests work: Document working patterns

2. [ ] **Core Operations Verified**
   - Must have basic pool operations working
   - Must have token swaps functioning

3. [ ] **Fee System Validation**
   - Basic fee operations must work
   - Fee calculations must be accurate

4. [ ] **Delegate System Check**
   - Basic delegate operations must function
   - Permissions must work correctly

## Progress Tracking

### Currently Working Tests
```
(To be filled in during assessment phase)
```

### Currently Broken Tests
```
(To be filled in during assessment phase)
```

### Test Repair Progress
- Total Tests: TBD
- Working Tests: 0
- Commented Out Tests: 0
- Repaired Tests: 0

## Rollback Criteria
1. If no tests are working after initial assessment
2. If core pool operations cannot be restored
3. If fee calculations show persistent errors
4. If delegate system fundamentally broken

## Success Criteria
1. All core pool operations tests passing
2. Basic fee management tests working
3. Essential delegate operations verified
4. No critical security vulnerabilities
5. At least 80% of original test coverage restored

## Next Steps
1. Run complete test suite and document current state
2. Begin commenting out failing tests
3. Focus on restoring core functionality first
4. Regular progress reviews and adjustments 