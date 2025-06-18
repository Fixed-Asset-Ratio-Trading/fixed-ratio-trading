# Refactor: Remove Withdrawal Percentage Limits

## Purpose

This refactor aims to remove the withdrawal percentage limits from the Fixed Ratio Trading platform as these limits will be enforced in another contract as part of the deal stage process.

## Affected Components

1. **Security Processor**
2. **Error Types**
3. **Instruction Types**
4. **Main Entry Point**
5. **Tests**
6. **Common Helpers**
7. **Testing Plan**
8. **AI Documentation**
9. **README Documentation**

## Changes Required

### 1. Security Processor

File: `/src/processors/security.rs`

- Remove the `_max_withdrawal_percentage` parameter from the `process_update_security_params` function
- Remove documentation references to withdrawal percentage limits
- Update function comments to remove mentions of future withdrawal limit controls

```diff
- pub fn process_update_security_params(
-     _program_id: &Pubkey,
-     accounts: &[AccountInfo],
-     _max_withdrawal_percentage: Option<u64>,
-     is_paused: Option<bool>,
- ) -> ProgramResult {
+ pub fn process_update_security_params(
+     _program_id: &Pubkey,
+     accounts: &[AccountInfo],
+     is_paused: Option<bool>,
+ ) -> ProgramResult {
```

- Update all documentation that mentions the withdrawal percentage limits
- Remove references to withdrawal limits in the "Future Extensions" section
- Update lines 35, 44, 78, 84 that reference `max_withdrawal_percentage`

### 2. Error Types

File: `/src/error.rs`

- Remove the `WithdrawalTooLarge` error variant:

```diff
-     /// Withdrawal amount exceeds maximum allowed percentage
-     #[error("Withdrawal amount exceeds maximum allowed percentage")]
-     WithdrawalTooLarge,
```

- Update the error code mapping in the `error_code()` method to remove the corresponding code

File: `/src/types/errors.rs`

- Remove the `WithdrawalTooLarge` error variant:

```diff
-     /// Withdrawal amount exceeds maximum allowed percentage
-     WithdrawalTooLarge,
```

- Remove the corresponding display implementation:

```diff
-             PoolError::WithdrawalTooLarge => write!(f, "Withdrawal amount exceeds maximum allowed percentage"),
```

- Update the error code mapping to remove code 1007

### 3. Instruction Types

File: `/src/types/instructions.rs`

- Update the `UpdateSecurityParams` instruction to remove the `max_withdrawal_percentage` field

```diff
/// Updates security parameters for the pool
UpdateSecurityParams {
-     /// Maximum percentage of pool liquidity that can be withdrawn in a single transaction
-     max_withdrawal_percentage: Option<u64>,
      /// Whether to pause pool operations
      is_paused: Option<bool>,
},
```

### 4. Process Instruction Function

File: `/src/lib.rs`

- Update the `process_instruction` function to remove the `max_withdrawal_percentage` parameter when calling the security processor

```diff
PoolInstruction::UpdateSecurityParams {
-     max_withdrawal_percentage,
      is_paused,
- } => process_update_security_params(program_id, accounts, max_withdrawal_percentage, is_paused),
+ } => process_update_security_params(program_id, accounts, is_paused),
```

### 5. Tests

#### File: `/tests/test_security.rs`

- Remove tests related to withdrawal percentage limits validation
- Remove the `max_withdrawal_percentage` parameter from test functions:

```diff
pub fn update_security_params(
    client: &mut BanksClient,
    payer_keypair: &Keypair,
    pool_account: &Pubkey,
    owner: &Keypair,
-   max_withdrawal_percentage: Option<u64>,
    is_paused: Option<bool>,
) -> Result<(), BanksClientError> {
```

- Remove assertions related to withdrawal percentage limits

**Specific test cases to remove/update:**
- Lines 224-236: Remove the `test_invalid_security_params` test that validates withdrawal percentage over 100%
- Update all calls to `update_security_params` to remove the withdrawal percentage parameter

#### File: `/tests/test_utilities.rs`

- Remove the error message test for withdrawal percentage:

```diff
-     assert_eq!(format!("{}", error), "Withdrawal amount exceeds maximum allowed percentage");
```

- Update the `test_pool_error_display` and `test_pool_error_to_program_error` functions to remove `WithdrawalTooLarge` error tests

### 6. Common Helpers

File: `/tests/common/pool_helpers.rs`

- Update the `update_security_params` helper function to remove the `max_withdrawal_percentage` parameter
- Remove associated documentation

```diff
/// * `payer_keypair` - Payer account keypair
/// * `pool_account` - Pool state account pubkey
/// * `owner` - Pool owner keypair
- /// * `max_withdrawal_percentage` - Maximum withdrawal percentage (optional)
/// * `is_paused` - Whether to pause the pool (optional)
pub async fn update_security_params(
    client: &mut BanksClient,
    payer_keypair: &Keypair,
    pool_account: &Pubkey,
    owner: &Keypair,
-   max_withdrawal_percentage: Option<u64>,
    is_paused: Option<bool>,
) -> Result<(), BanksClientError> {
```

### 7. Testing Plan Updates

File: `/docs/tests/COMPREHENSIVE_TESTING_PLAN.md`

- Remove the planned test LIQ-010 for withdrawal percentage limits:

```diff
- [ ] **LIQ-010** `test_withdrawal_percentage_limit` - Maximum withdrawal percentage check
```

- Update any other test plans that reference withdrawal percentage limits

### 8. AI Documentation Updates

File: `/docs/ai_notes/CodeTestInfoJune15.md`

- Remove reference to "Withdrawal percentage limits" on line 64

File: `/docs/ai_notes/delegate Accounts Notes.md`

- Update the reference to "Fee-Based Limits: Limit withdrawals to percentage of collected fees" on line 80 to clarify this refers to fee-based limits, not pool liquidity percentage limits

### 9. README.md Updates

- Remove any mentions of withdrawal percentage limits from the README.md file 
- Update API documentation sections that reference these parameters

## Testing Strategy

1. **Run Existing Tests**: After making all changes, run the existing test suite to ensure no regressions were introduced:

```bash
cargo test
```

2. **Verify Test Coverage**: Ensure test coverage remains consistent or improves after removing withdrawal percentage limit code:

```bash
cargo tarpaulin --out Xml
```

3. **Manual Verification**: Test the `UpdateSecurityParams` instruction manually to confirm it works correctly with only the `is_paused` parameter.

4. **Error Handling Tests**: Verify that all error-related tests still pass after removing the `WithdrawalTooLarge` error type.

## Additional Considerations

1. **Client SDKs**: Update any JavaScript, Python, or other client SDKs that might be using the withdrawal percentage limits feature.

2. **Documentation**: Review and update all API documentation that references withdrawal percentage limits.

3. **Migration**: This is a non-breaking change since the feature was only partially implemented and reserved for future use, but any existing clients calling the API should be notified of the parameter removal.

4. **Versioning**: Consider if this change warrants a version bump in the protocol to communicate the change clearly to users.

5. **Error Code Reorganization**: After removing error code 1007 (WithdrawalTooLarge), consider whether to reorganize remaining error codes to maintain continuity.

## Detailed File Change Checklist

### Code Files
- [ ] `/src/processors/security.rs` - Remove parameter and documentation
- [ ] `/src/error.rs` - Remove WithdrawalTooLarge error and error code
- [ ] `/src/types/errors.rs` - Remove WithdrawalTooLarge error and display implementation
- [ ] `/src/types/instructions.rs` - Remove max_withdrawal_percentage field
- [ ] `/src/lib.rs` - Update process_instruction call

### Test Files
- [ ] `/tests/test_security.rs` - Remove withdrawal percentage tests and update function signatures
- [ ] `/tests/test_utilities.rs` - Remove WithdrawalTooLarge error tests
- [ ] `/tests/common/pool_helpers.rs` - Update helper function signature

### Documentation Files
- [ ] `/docs/tests/COMPREHENSIVE_TESTING_PLAN.md` - Remove LIQ-010 test plan
- [ ] `/docs/ai_notes/CodeTestInfoJune15.md` - Remove withdrawal percentage limit reference
- [ ] `/docs/ai_notes/delegate Accounts Notes.md` - Clarify fee-based limits context
- [ ] `/README.md` - Remove withdrawal percentage limit references

## Implementation Timeline

1. Code changes: 2-3 days (expanded due to error type removal)
2. Testing and validation: 2 days (additional time for error handling verification)
3. Documentation updates: 1 day

## Approval

This refactoring plan requires approval before implementation begins.

- [ ] Code changes approved
- [ ] Testing strategy approved
- [ ] Documentation updates approved
- [ ] Error type removal approved
