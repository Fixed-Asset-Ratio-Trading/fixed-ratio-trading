# System Halt & Restart Penalty - Comprehensive Test Plan

## Overview
This document outlines the testing strategy for the system halt (pause) and restart penalty functionality that blocks treasury withdrawals during system pause and applies a 3-day (71-hour) cooling-off period after system restart.

## Critical Requirements Summary
1. **System Halt**: Treasury withdrawals must be blocked when system is paused
2. **Restart Penalty**: 71-hour withdrawal penalty applied when system is re-enabled
3. **Penalty Precedence**: Restart penalty takes precedence over regular rate limiting
4. **Security**: Only program upgrade authority can pause/unpause system

---

## Test Categories

### 1. System Pause (Halt) Functionality Tests

#### 1.1 Basic System Pause Operations
- [ ] **Test**: System can be paused by program upgrade authority
- [ ] **Test**: System pause requires valid program upgrade authority signature
- [ ] **Test**: System pause with invalid authority should fail
- [ ] **Test**: System pause updates SystemState correctly (is_paused = true)
- [ ] **Test**: System pause records timestamp and reason code

#### 1.2 Treasury Withdrawal Blocking During Pause
- [ ] **Test**: Treasury withdrawal fails when system is paused
- [ ] **Test**: Error message clearly indicates system is paused
- [ ] **Test**: Regular operations (swaps, liquidity) also blocked during pause
- [ ] **Test**: System pause validation occurs before authority validation in treasury operations

#### 1.3 System Pause Edge Cases
- [ ] **Test**: Attempting to pause already paused system should fail gracefully
- [ ] **Test**: System pause with different reason codes works correctly
- [ ] **Test**: System pause persists across multiple transaction attempts

### 2. System Unpause (Restart) Functionality Tests

#### 2.1 Basic System Unpause Operations
- [ ] **Test**: System can be unpaused by program upgrade authority
- [ ] **Test**: System unpause requires valid program upgrade authority signature
- [ ] **Test**: System unpause with invalid authority should fail
- [ ] **Test**: System unpause updates SystemState correctly (is_paused = false)
- [ ] **Test**: System unpause requires MainTreasuryState account (4 accounts total)

#### 2.2 Restart Penalty Application
- [ ] **Test**: System unpause applies 71-hour restart penalty to treasury
- [ ] **Test**: `last_withdrawal_timestamp` set to current_time + 71 hours
- [ ] **Test**: `last_update_timestamp` updated to current unpause time
- [ ] **Test**: Treasury state serialization succeeds after penalty application
- [ ] **Test**: Log messages include penalty expiration timestamp

#### 2.3 System Unpause Edge Cases
- [ ] **Test**: Attempting to unpause already unpaused system should fail gracefully
- [ ] **Test**: System unpause logs previous pause duration correctly
- [ ] **Test**: System unpause works with various pause reason codes

### 3. Restart Penalty Validation Tests

#### 3.1 Penalty Period Blocking
- [ ] **Test**: Treasury withdrawal blocked immediately after system restart
- [ ] **Test**: Treasury withdrawal blocked 1 hour after restart
- [ ] **Test**: Treasury withdrawal blocked 24 hours after restart
- [ ] **Test**: Treasury withdrawal blocked 70 hours after restart (just before expiry)
- [ ] **Test**: Error message clearly indicates restart penalty is active

#### 3.2 Penalty Period Error Messages
- [ ] **Test**: Error includes remaining penalty time in seconds
- [ ] **Test**: Error includes remaining penalty time in hours
- [ ] **Test**: Error includes remaining penalty time in days
- [ ] **Test**: Error explains purpose of 3-day cooling-off period

#### 3.3 Time Calculation Accuracy
- [ ] **Test**: `is_blocked_by_restart_penalty()` returns true during penalty
- [ ] **Test**: `restart_penalty_time_remaining()` calculates correctly
- [ ] **Test**: Time calculations handle edge cases (exactly at expiry)

### 4. Penalty Expiration and Normal Operation Resume

#### 4.1 Penalty Expiration
- [ ] **Test**: Treasury withdrawal allowed exactly at 71-hour mark
- [ ] **Test**: Treasury withdrawal allowed 72 hours after restart
- [ ] **Test**: Treasury withdrawal allowed days after restart
- [ ] **Test**: Normal rate limiting resumes after penalty expires

#### 4.2 Post-Penalty Rate Limiting
- [ ] **Test**: Dynamic rate limiting works correctly after penalty expires
- [ ] **Test**: Hourly withdrawal limits calculated correctly post-penalty
- [ ] **Test**: Rolling 60-minute window functions normally post-penalty
- [ ] **Test**: Multiple withdrawals work with normal timing after penalty

### 5. Integration and Interaction Tests

#### 5.1 Multiple Pause/Unpause Cycles
- [ ] **Test**: Second pause/unpause cycle applies new 71-hour penalty
- [ ] **Test**: Previous penalty is overwritten by new restart penalty
- [ ] **Test**: Multiple rapid pause/unpause cycles work correctly
- [ ] **Test**: Long pause periods followed by unpause work correctly

#### 5.2 Penalty vs Regular Rate Limiting Precedence
- [ ] **Test**: Restart penalty blocks withdrawals even if regular rate limit would allow
- [ ] **Test**: Restart penalty error shown instead of rate limit error during penalty
- [ ] **Test**: After penalty expires, regular rate limiting takes precedence
- [ ] **Test**: Penalty time calculation ignores regular rate limit timestamps

#### 5.3 Dynamic Rate Calculation During Penalty
- [ ] **Test**: Dynamic rate calculation works during penalty period
- [ ] **Test**: Treasury balance changes don't affect penalty duration
- [ ] **Test**: Rate limit info shown in error messages during penalty
- [ ] **Test**: Penalty and rate limit details both logged appropriately

### 6. Edge Cases and Boundary Conditions

#### 6.1 Timestamp Edge Cases
- [ ] **Test**: System restart at timestamp 0 (fallback case)
- [ ] **Test**: System restart with very large timestamps
- [ ] **Test**: Penalty calculation handles timestamp overflow safely
- [ ] **Test**: Time zone and daylight saving time don't affect penalty

#### 6.2 Account State Edge Cases
- [ ] **Test**: Treasury account with insufficient space for serialization
- [ ] **Test**: Corrupted treasury state during unpause
- [ ] **Test**: Treasury state consistency after failed unpause attempt
- [ ] **Test**: Concurrent access during unpause operation

#### 6.3 First-Time Operations
- [ ] **Test**: First system pause on fresh deployment
- [ ] **Test**: First treasury withdrawal attempt with restart penalty
- [ ] **Test**: Treasury with `last_withdrawal_timestamp = 0` during penalty

### 7. Security and Authority Tests

#### 7.1 Authority Validation
- [ ] **Test**: Only program upgrade authority can trigger system pause
- [ ] **Test**: Only program upgrade authority can trigger system unpause
- [ ] **Test**: Invalid signers cannot pause system
- [ ] **Test**: Invalid signers cannot unpause system
- [ ] **Test**: PDA validation for treasury account during unpause

#### 7.2 Attack Vector Prevention
- [ ] **Test**: Cannot bypass restart penalty with different withdrawal amounts
- [ ] **Test**: Cannot bypass restart penalty with rapid withdrawal attempts
- [ ] **Test**: Cannot manipulate penalty timing through system state corruption
- [ ] **Test**: Cannot use fake treasury accounts during unpause

### 8. Performance and Gas Usage Tests

#### 8.1 Transaction Efficiency
- [ ] **Test**: System pause transaction uses minimal compute units
- [ ] **Test**: System unpause transaction (with penalty) uses reasonable compute units
- [ ] **Test**: Treasury withdrawal validation with penalty check is efficient
- [ ] **Test**: Error message generation doesn't consume excessive compute

#### 8.2 State Storage Optimization
- [ ] **Test**: No unnecessary data stored in penalty application
- [ ] **Test**: Treasury state size remains within expected bounds
- [ ] **Test**: Serialization/deserialization performance acceptable

---

## Test Data Scenarios

### Realistic Treasury Balances
- [ ] Small treasury: 100 SOL (10 SOL/hour rate)
- [ ] Medium treasury: 2000 SOL (100 SOL/hour rate)  
- [ ] Large treasury: 25000 SOL (1000 SOL/hour rate)
- [ ] Very large treasury: 250000 SOL (10000 SOL/hour rate)

### Time-based Scenarios
- [ ] Immediate withdrawal after restart (should fail)
- [ ] Withdrawal at 1 hour, 6 hours, 12 hours, 24 hours, 48 hours, 70 hours
- [ ] Withdrawal exactly at 71-hour mark (should succeed)
- [ ] Withdrawal well after penalty expires (should succeed)

### Authority Scenarios
- [ ] Valid program upgrade authority
- [ ] Invalid authority attempts
- [ ] Missing authority signatures
- [ ] Wrong PDA accounts

---

## Success Criteria

### Functional Requirements
✅ **Critical**: System pause completely blocks treasury withdrawals
✅ **Critical**: System unpause applies exactly 71-hour penalty
✅ **Critical**: Penalty takes precedence over all other rate limiting
✅ **Critical**: Normal operations resume after penalty expires

### Security Requirements
✅ **Critical**: Only authorized entities can pause/unpause system
✅ **Critical**: Cannot bypass penalty through any mechanism
✅ **Critical**: Penalty duration cannot be manipulated

### User Experience Requirements
✅ **Important**: Clear error messages during penalty period
✅ **Important**: Accurate time remaining calculations
✅ **Important**: Informative logging during pause/unpause operations

### Performance Requirements
✅ **Important**: Reasonable transaction costs for pause/unpause
✅ **Important**: Efficient penalty validation on each withdrawal attempt

---

## Testing Environment Setup

### Test Foundation Requirements
**All tests MUST use the Enhanced Test Foundation located at:**
```
/Users/davinci/code/fixed-ratio-trading/tests/common/enhanced_test_foundation.rs
```

This ensures:
- ✅ **Actual Solana Environment**: Tests run against real contract code in live Solana environment
- ✅ **No Mock/Smoke Tests**: All tests interact with deployed program functionality
- ✅ **Standardized Test Structure**: Consistent test setup and teardown across all tests
- ✅ **Multi-Pool Capability**: Support for complex scenarios with multiple pools
- ✅ **Shared Resources**: Common system state, treasury, and token program access

### Required Test Configuration Pattern
**All tests MUST follow the standardized TEST CONFIGURATION pattern:**

```rust
#[serial]
async fn test_system_halt_functionality() -> TestResult {
    // ============================================================================
    // 🎯 TEST CONFIGURATION - MODIFY THESE VALUES TO CHANGE TEST BEHAVIOR
    // ============================================================================
    
    // Debug Configuration
    const ENABLE_DEBUG_LOGGING: bool = false; // Set to true for verbose Solana runtime logs
    
    // System State Configuration
    const INITIAL_SYSTEM_PAUSED: bool = false;     // Initial system pause state
    const PAUSE_REASON_CODE: u8 = 1;               // Reason code for system pause
    const TEST_EMERGENCY_PAUSE: bool = false;      // Test emergency vs normal pause
    
    // Treasury Configuration
    const TREASURY_BALANCE_SOL: u64 = 2000;        // Treasury balance in SOL
    const EXPECTED_HOURLY_RATE: u64 = 100;         // Expected withdrawal rate (SOL/hour)
    const WITHDRAWAL_AMOUNT_SOL: u64 = 50;         // Test withdrawal amount
    
    // Authority Configuration
    const USE_VALID_AUTHORITY: bool = true;        // Use valid program upgrade authority
    const TEST_INVALID_AUTHORITY: bool = false;    // Test with invalid authority
    
    // Time Configuration
    const RESTART_PENALTY_HOURS: u64 = 71;        // 3-day cooling period in hours
    const TEST_TIME_OFFSET_HOURS: u64 = 0;        // Time offset for penalty testing
    
    // Verification Configuration
    const VERIFY_ERROR_MESSAGES: bool = true;     // Verify specific error message content
    const VERIFY_LOG_MESSAGES: bool = true;       // Verify system log messages
    const VERIFY_STATE_CHANGES: bool = true;      // Verify SystemState updates
    
    // ============================================================================
    // 🧪 TEST SETUP AND EXECUTION
    // ============================================================================
    
    println!("🧪 TEST: System Halt Functionality");
    println!("====================================");
    println!("🎯 PURPOSE: Verify system pause blocks treasury withdrawals");
    println!("🔍 SCENARIO: Pausing system and attempting treasury operations");
    println!("✅ EXPECTED: All treasury withdrawals blocked during pause");
    
    // ... test implementation using EnhancedTestFoundation
}
```

### Required Test Accounts
- Program upgrade authority keypair (via EnhancedTestFoundation)
- Invalid authority keypair (for negative testing)
- Main treasury PDA (managed by foundation)
- System state PDA (managed by foundation)
- Test destination accounts (created by foundation)

### Live Solana Environment Requirements
- **Real Contract Execution**: Tests must execute actual program instructions
- **Actual PDAs**: Use real Program Derived Addresses, not mock accounts
- **Live Token Operations**: Real SPL token transfers and account management
- **Authentic Authority Validation**: Test real signature and authority checking
- **Genuine State Persistence**: Verify actual account state changes persist

### Configuration-Driven Edge Case Testing
Each test must provide clear configuration constants that allow:
- **Quick Parameter Changes**: Modify test behavior without code restructuring
- **Edge Case Exploration**: Easy testing of boundary conditions
- **Scenario Variants**: Multiple test configurations from single test function
- **Debug Control**: Toggle debug logging and verification levels
- **Time Manipulation**: Configure time-based scenarios (penalty periods, etc.)

### Test Structure Standards
All tests must include:
1. **Configuration Section**: Clear constants at top with 🎯 TEST CONFIGURATION header
2. **Setup Section**: Test purpose and scenario description with 🧪 TEST SETUP header
3. **Enhanced Foundation Usage**: Use `create_enhanced_test_foundation()` for setup
4. **Real Environment Verification**: Confirm tests run against actual Solana program
5. **State Validation**: Verify actual account state changes, not mock responses
6. **Cleanup**: Proper resource cleanup using foundation's cleanup methods

---

## Questions for Refinement

1. **Scope**: Are there any additional edge cases or scenarios you want tested?

2. **Priorities**: Which test categories are most critical vs nice-to-have?

3. **Performance**: What are acceptable compute unit limits for pause/unpause operations?

4. **Error Handling**: Are there specific error messages or codes you want standardized?

5. **Integration**: Should we test integration with specific client applications or just the core functionality?

6. **Automation**: Do you want automated stress testing (e.g., rapid pause/unpause cycles)?

7. **Monitoring**: Should tests include verification of specific log message formats for monitoring?

Please review this test plan and let me know:
- Which tests should be prioritized
- Any missing scenarios or edge cases
- Specific requirements for error messages or logging
- Performance expectations or constraints
- Any modifications to the testing approach