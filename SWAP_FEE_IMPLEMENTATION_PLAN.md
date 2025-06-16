# Swap Fee and Delegate Management Implementation Plan

## Overview
This document outlines the plan to implement pool-specific swap fees with delegate control and streamline the delegate management system.

## Current State
- Swap fees are currently set to 0% by default
- Maximum fee cap is 0.5% (50 basis points)
- Fees can only be adjusted by contract creator
- Separate methods exist for different delegate actions
- No time-based authorization system for fee changes

## Proposed Changes

1. Consolidated Instructions: Reduced the number of methods by combining similar functionality:
* Single RequestDelegateAction for all delegate actions
* Single RevokeAction for all revocation types
* Single SetDelegateTimeLimits for all time limit settings
2. Time-based Authorization:
* Default 72-hour wait time for all delegates
* Configurable per-delegate and per-action wait times (5 minutes to 72 hours)
* Contract creator can revoke changes before activation
3. State Management:
* Added pending actions tracking
* Per-delegate time limits
* Action history for auditing
4. Security and Risk Mitigation:
* Double validation for critical actions
* Rate limiting
* Time manipulation safeguards


### 1. State Changes

#### Pool State Additions
```rust
pub struct PoolState {
    // ... existing fields ...
    pub delegate_time_limits: DelegateTimeLimits,
    pub pending_actions: Vec<PendingDelegateAction>,
}

pub struct DelegateTimeLimits {
    pub fee_change_wait_time: u64,    // Per delegate wait time for fee changes
    pub withdraw_wait_time: u64,      // Per delegate wait time for withdrawals
    pub pause_wait_time: u64,         // Per delegate wait time for pausing
}

pub struct PendingDelegateAction {
    pub delegate: Pubkey,
    pub action_type: DelegateActionType,
    pub request_timestamp: u64,
    pub execution_timestamp: u64,
    pub params: DelegateActionParams,
}

pub enum DelegateActionType {
    FeeChange,
    Withdrawal,
    PoolPause,
}

pub enum DelegateActionParams {
    FeeChange { new_fee_basis_points: u64 },
    Withdrawal { token_mint: Pubkey, amount: u64 },
    PoolPause { duration: u64 },
}
```

### 2. Instruction Consolidation

#### New Consolidated Instructions
```rust
pub enum PoolInstruction {
    // ... existing instructions ...
    
    // New consolidated instructions
    RequestDelegateAction {
        action_type: DelegateActionType,
        params: DelegateActionParams,
    },
    
    ExecuteDelegateAction {
        action_id: u64,  // Unique identifier for the pending action
    },
    
    RevokeAction {
        action_id: u64,
    },
    
    SetDelegateTimeLimits {
        delegate: Pubkey,
        time_limits: DelegateTimeLimits,
    },
}
```

### 3. Removed/Consolidated Instructions
The following instructions will be removed and consolidated:
- `WithdrawFeesToDelegate`
- `RequestFeeWithdrawal`
- `CancelWithdrawalRequest`
- `SetDelegateWaitTime`
- `SetSwapFee`

### 4. Implementation Phases

#### Phase 1: State Structure Updates
1. Add new state structures
2. Implement serialization/deserialization
3. Update pool initialization
4. Add migration logic for existing pools

#### Phase 2: Core Logic Implementation
1. Implement consolidated request system
2. Add time-based authorization logic
3. Implement execution and revocation logic
4. Add validation and security checks

#### Phase 3: Testing and Validation
1. Update existing tests
2. Add new test cases for:
   - Delegate action requests
   - Time-based authorization
   - Revocation system
   - Multiple pending actions
   - Edge cases and security scenarios

### 5. Security Considerations

#### Time Management
- Use Solana's slot-based time system
- Include buffer periods for network latency
- Implement safeguards against time manipulation

#### Authorization
- Double validation for critical actions
- Clear separation of creator and delegate permissions
- Atomic execution of state changes

#### Rate Limiting
- Maximum number of pending actions per delegate
- Cool-down period between similar actions
- Maximum number of actions per time window

### 6. Test Impact

#### Modified Tests
- `test_swap_zero_amount_fails`: Add fee validation
- `test_exchange_token_b_for_token_a`: Include fee calculations
- All delegate-related tests need updates

#### New Test Cases
1. Delegate fee change request flow
2. Time-based authorization tests
3. Revocation scenarios
4. Multiple delegate interaction tests
5. Edge case handling

### 7. Migration Strategy

#### For Existing Pools
1. Default all delegate time limits to 72 hours
2. Initialize empty pending actions list
3. Preserve existing fee settings
4. Add version tracking for future upgrades

#### For New Pools
1. Initialize with new state structure
2. Set default time limits during creation
3. Enable all new features by default

## Timeline and Dependencies

### Week 1: Foundation
- State structure updates
- Basic validation logic
- Initial test framework updates

### Week 2: Core Implementation
- Request system implementation
- Time-based authorization
- Execution and revocation logic

### Week 3: Testing and Refinement
- Complete test coverage
- Security auditing
- Documentation updates

### Week 4: Migration and Deployment
- Migration script development
- Deployment planning
- Final testing and validation

## Suggested Additional Improvements

1. **Action Batching**
   - Allow multiple delegate actions to be requested/executed in a single transaction
   - Reduce transaction fees and improve UX

2. **Delegate Tiers**
   - Implement different permission levels for delegates
   - Allow fine-grained control over delegate capabilities

3. **Analytics Integration**
   - Track delegate action history
   - Provide insights into fee changes and their impacts

4. **Emergency Controls**
   - Add emergency pause mechanism for all delegate actions
   - Implement circuit breaker for rapid fee changes

## Risks and Mitigations

### Technical Risks
1. **State Bloat**
   - Mitigation: Implement cleanup of executed/expired actions
   - Regular state compaction

2. **Time Manipulation**
   - Mitigation: Use multiple time sources
   - Implement safety bounds

3. **Transaction Ordering**
   - Mitigation: Clear action sequencing
   - Atomic state updates

### Business Risks
1. **User Experience**
   - Mitigation: Clear error messages
   - Intuitive action flow
   - Comprehensive documentation

2. **Delegate Management**
   - Mitigation: Clear delegation hierarchy
   - Transparent permission system

## Success Metrics

1. **Technical Metrics**
   - Successful action execution rate
   - Average action processing time
   - Error rate and types

2. **Business Metrics**
   - Delegate action distribution
   - Fee change frequency and impact
   - Pool performance correlation

## Documentation Requirements

1. **Technical Documentation**
   - Architecture overview
   - State transition diagrams
   - Security model

2. **User Documentation**
   - Delegate guide
   - Action request/execution flow
   - Time limit management

3. **Integration Guide**
   - API documentation
   - Example implementations
   - Common patterns

## Testing Documentation Updates

### COMPREHENSIVE_TESTING_PLAN Updates Required

#### Methods to Remove from Testing
1. Individual Delegate Action Tests:
   - `test_withdraw_fees_to_delegate`
   - `test_request_fee_withdrawal`
   - `test_cancel_withdrawal_request`
   - `test_set_delegate_wait_time`
   - `test_set_swap_fee`

#### New Methods to Add for Testing
1. Consolidated Action Tests:
   - `test_request_delegate_action_fee_change`
   - `test_request_delegate_action_withdrawal`
   - `test_request_delegate_action_pool_pause`
   - `test_execute_delegate_action`
   - `test_revoke_action`
   - `test_set_delegate_time_limits`

2. Time-based Authorization Tests:
   - `test_delegate_action_wait_time_enforcement`
   - `test_delegate_action_early_execution_prevention`
   - `test_creator_revocation_rights`
   - `test_delegate_time_limits_boundaries`

3. State Management Tests:
   - `test_pending_actions_tracking`
   - `test_action_history_recording`
   - `test_multiple_pending_actions`
   - `test_delegate_specific_time_limits`

4. Security and Validation Tests:
   - `test_double_validation_critical_actions`
   - `test_rate_limiting_enforcement`
   - `test_time_manipulation_prevention`
   - `test_concurrent_action_handling`

5. Migration Tests:
   - `test_existing_pool_migration`
   - `test_default_time_limits_application`
   - `test_preserved_fee_settings`

#### Test Coverage Requirements
- Each new consolidated instruction must have ≥95% code coverage
- All edge cases and error conditions must be tested
- Integration tests must verify complete action workflows
- Performance impact tests for state changes

#### Test Documentation Updates
- Update test descriptions to reflect new consolidated approach
- Add detailed test scenarios for time-based authorization
- Document expected behavior for all error conditions
- Include examples of correct usage patterns 