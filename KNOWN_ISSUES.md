# Known Issues

## Delegate Test Failures (Discovered During Refactor)

### Issue Description
Two delegate tests are failing due to a serialization/deserialization issue in the program code:
- `test_add_duplicate_delegate_fails`
- `test_add_multiple_delegates`

### Root Cause Analysis
The debug investigation revealed:
1. ✅ **Pool owner auto-addition works correctly** (delegate_count=1, first delegate=owner)
2. ✅ **Delegate addition logic works correctly** (returns success)  
3. ❌ **Serialization issue occurs** when reading pool state after delegate addition
4. ❌ **Error**: "Unexpected length of input" during BorshDeserialize

### Technical Details
- The `add_delegate` function successfully adds delegates
- The `serialize()` call appears to corrupt the pool state data
- This suggests a size calculation mismatch in `PoolState::get_packed_len()` or related structures
- Complex nested structures (`DelegateManagement`, `PoolPauseRequest`, etc.) may have alignment issues

### Impact Assessment  
**🎯 Refactor Status: SUCCESSFUL with Enhanced Test Coverage**

- **Not a regression**: These delegate tests **did not exist** in the original `integration_test.rs` file
- **Improved coverage**: The refactor added comprehensive delegate testing that found real bugs
- **Original functionality**: All tests from the original file pass correctly
- **Modular structure**: Successfully created 6 focused test modules + utilities

### Current Status
- **Pool creation tests**: ✅ Working
- **Swap tests**: ✅ Working  
- **Security tests**: ✅ Working
- **Fee tests**: ✅ Working
- **Utilities tests**: ✅ Working
- **Delegate tests**: ❌ Failing due to program serialization bug (NEW functionality)

### Recommended Actions
1. **Short-term**: Mark delegate tests as "known issues" and continue with working functionality
2. **Medium-term**: Debug the `PoolState` serialization issue in the program code
3. **Long-term**: Fix the underlying serialization bug and enable full delegate testing

### Logging Control Status ✅
The original issue with `RUST_LOG=error cargo test` is **resolved**:
- Logging control works correctly (minimal output with error level)  
- Test failures are due to **new functionality**, not the refactor itself
- All original test functionality works in the modular structure 