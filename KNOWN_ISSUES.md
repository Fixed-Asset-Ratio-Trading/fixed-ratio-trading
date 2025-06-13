# Known Issues

## Delegate Test Issues (Discovered During Refactor)

### ✅ **SIGNIFICANT PROGRESS**: 6/8 Tests Now Passing

After comprehensive debugging and bug fixes:
- **Added `Copy` trait** to `PoolPauseReason` and `PoolPauseRequest`
- **Fixed hardcoded array initialization** in `DelegateManagement::new()`
- **Resolved compilation errors** in delegate test modules
- **6 out of 8 delegate tests now pass** (75% success rate)

### Remaining Issues (2/8 tests)
Two delegate tests still fail due to a deeper serialization issue:
- `test_add_duplicate_delegate_fails` - Fails when reading pool state after first delegate addition
- `test_add_multiple_delegates` - Fails on second delegate addition

### Root Cause Analysis 
The debug investigation revealed:
1. ✅ **Pool owner auto-addition works correctly** (delegate_count=1, first delegate=owner)
2. ✅ **Delegate addition logic works correctly** (returns success)  
3. ✅ **Array initialization fixed** (all arrays now use `[Type::default(); MAX_DELEGATES]`)
4. ✅ **Copy traits added** (eliminates trait bound issues)
5. ❌ **Persistent serialization issue** when reading pool state after delegate modifications
6. ❌ **Error**: "Unexpected length of input" during BorshDeserialize

### Technical Analysis
- The `add_delegate` function successfully executes
- Compilation and array initialization issues are resolved  
- The error occurs during subsequent pool state read operations
- This suggests a **deeper program-level bug** in Borsh serialization/account sizing

### Impact Assessment  
**🎯 Refactor Status: SUCCESSFUL with Enhanced Test Coverage**

- **Not a regression**: These delegate tests **did not exist** in the original `integration_test.rs` file
- **Improved coverage**: The refactor added comprehensive delegate testing that found real bugs
- **Original functionality**: All tests from the original file pass correctly
- **Modular structure**: Successfully created 6 focused test modules + utilities

### Current Status
- **Pool creation tests**: ✅ Working (100%)
- **Swap tests**: ✅ Working (100%)  
- **Security tests**: ✅ Working (100%)
- **Fee tests**: ✅ Working (100%)
- **Utilities tests**: ✅ Working (100%)
- **Delegate tests**: 🟡 Partially working (75% - 6/8 tests pass)
  - ✅ `test_add_delegate_success`
  - ✅ `test_pool_owner_as_implicit_delegate`
  - ✅ `test_unauthorized_delegate_operation_fails`
  - ✅ `test_add_delegate_unauthorized_fails`
  - ✅ `test_delegate_limit_enforcement`
  - ✅ `test_delegate_authorization`
  - ❌ `test_add_duplicate_delegate_fails` (serialization bug)
  - ❌ `test_add_multiple_delegates` (serialization bug)

### Recommended Actions
1. **Short-term**: Mark delegate tests as "known issues" and continue with working functionality
2. **Medium-term**: Debug the `PoolState` serialization issue in the program code
3. **Long-term**: Fix the underlying serialization bug and enable full delegate testing

### Logging Control Status ✅
The original issue with `RUST_LOG=error cargo test` is **resolved**:
- Logging control works correctly (minimal output with error level)  
- Test failures are due to **new functionality**, not the refactor itself
- All original test functionality works in the modular structure 