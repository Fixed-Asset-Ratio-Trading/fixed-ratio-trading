# Refactor Plan: Delegate Pool Pause/Unpause System (Swap-Only)

> **🎯 FOCUSED REFACTOR**: This plan fixes the delegate pause/unpause system to work properly for individual pool pause controls that only affect swap operations. Deposits and withdrawals continue to function normally during delegate-initiated pool pause. **Since the project is not yet deployed, we can safely remove the old time-based pause system completely.**

## Current Problem Analysis
- **Broken Delegate Pause**: Current `PoolPause` delegate action has flawed implementation with duration requirements
- **Time-Based Auto-Unpause**: Confusing auto-unpause logic based on timestamps that can be removed
- **Missing Pool-Specific Control**: Delegates need ability to pause individual pools, not just system-wide
- **Incorrect Scope**: Pool pause should only affect swaps, not all operations
- **Poor Public Visibility**: Pool pause status not easily readable by users before they attempt swaps

## Desired Design
- **Pause Request**: Delegate requests pool pause → wait period → execute to pause pool swaps indefinitely
- **Unpause Request**: Delegate requests pool unpause → wait period → execute to unpause pool swaps
- **Owner Override**: Pool owner can cancel any pending pause/unpause request
- **Swap-Only Impact**: Pool pause only affects `process_swap()` - deposits and withdrawals continue normally
- **Public Status**: Pool pause status easily queryable by users
- **No Auto-Unpause**: Pool stays paused until manually unpaused (removes confusing time-based logic)
- **No Duration**: Pool pause lasts indefinitely until explicitly unpaused (simplifies system)

---

## 📋 TASK LIST

### **PHASE 1: Remove Old Time-Based Pause System**

#### **Task 1.1: Remove Duration-Based Pause Fields**
- **File**: `src/types/pool_state.rs`
- **Changes**:
  - Remove `pause_end_timestamp: i64` field from `PoolState`
  - Remove `PoolPauseReason` enum (replaced with simple string reason)
  - Remove all time-based pause validation logic
  - Clean up `get_packed_len()` calculations
  ```rust
  // REMOVE these fields from PoolState:
  // pub pause_end_timestamp: i64,  ❌ REMOVE
  // pub pause_reason: PoolPauseReason,  ❌ REMOVE (replace with string)
  ```

#### **Task 1.2: Remove Auto-Unpause Logic**
- **File**: `src/processors/delegate_actions.rs` 
- **Changes**:
  - Remove automatic unpause check in `process_execute_delegate_action()`
  ```rust
  // REMOVE this entire block:
  // if pool_state.is_paused && pool_state.pause_end_timestamp > 0 && clock.unix_timestamp >= pool_state.pause_end_timestamp {
  //     pool_state.is_paused = false;
  //     pool_state.pause_end_timestamp = 0;
  //     ...
  // }  ❌ REMOVE
  ```

#### **Task 1.3: Remove Auto-Unpause from Validation**
- **File**: `src/utils/validation.rs`
- **Changes**:
  - Remove auto-unpause logic from `validate_pool_not_paused()`
  ```rust
  // REMOVE this entire auto-unpause block:
  // if pool_state.is_paused && pool_state.pause_end_timestamp > 0 && current_timestamp >= pool_state.pause_end_timestamp {
  //     pool_state.is_paused = false;
  //     pool_state.pause_end_timestamp = 0;
  //     ...
  // }  ❌ REMOVE
  ```

#### **Task 1.4: Remove Duration Parameters**
- **File**: `src/types/delegate_actions.rs`
- **Changes**:
  - Remove `duration` parameter from `PoolPause` enum variant
  ```rust
  // REMOVE old PoolPause variant:
  // PoolPause {
  //     duration: u64,  ❌ REMOVE
  //     reason: PauseReason,
  // },
  ```

---

### **PHASE 2: Implement New Swap-Only Pause System**

#### **Task 2.1: Replace Delegate Action Types**
- **File**: `src/types/delegate_actions.rs`
- **Changes**:
  - Replace `PoolPause` with `PausePoolSwaps` and `UnpausePoolSwaps` in `DelegateActionType` enum
  - Replace old `PoolPause` params with new variants in `DelegateActionParams`
  - Remove duration-based pause parameters completely
  - Add reason field for transparency
  ```rust
  #[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
  pub enum DelegateActionType {
      // ... existing actions ...
      
      /// Pause swap operations for this specific pool (deposits/withdrawals continue)
      PausePoolSwaps,
      
      /// Unpause swap operations for this specific pool  
      UnpausePoolSwaps,
      
      // ❌ REMOVE: PoolPause, (old time-based version)
  }
  
  #[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
  pub enum DelegateActionParams {
      // ... existing params ...
      
      PausePoolSwaps {
          reason: String, // Human-readable reason for pause (max 200 chars)
      },
      
      UnpausePoolSwaps,
      
      // ❌ REMOVE: PoolPause { duration, reason }, (old time-based version)
  }
  ```

#### **Task 2.2: Update Pool State Types**
- **File**: `src/types/pool_state.rs`
- **Changes**:
  - Add `swaps_paused` field to distinguish from system pause
  - Add `swaps_pause_requested_by` field to track who initiated current pause
  - Add `swaps_pause_initiated_timestamp` for audit trails
  - Add `swaps_pause_reason` for transparency
  ```rust
  #[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
  pub struct PoolState {
      // ... existing fields ...
      
      // Pool-specific swap pause controls (separate from system pause)
      pub swaps_paused: bool,
      pub swaps_pause_requested_by: Option<Pubkey>,
      pub swaps_pause_initiated_timestamp: i64,
      pub swaps_pause_reason: Option<String>,
      
      // ❌ REMOVE: pause_end_timestamp: i64,
      // ❌ REMOVE: pause_reason: PoolPauseReason,
  }
  ```

#### **Task 2.3: Update Instruction Types**
- **File**: `src/types/instructions.rs`
- **Changes**:
  - Add new query instruction for pool pause status
  ```rust
  pub enum PoolInstruction {
      // ... existing instructions ...
      
      /// Get current pool pause status (publicly readable)
      /// Returns swap pause status, deposit/withdrawal status, and pause details
      GetPoolPauseStatus,
  }
  ```

---

### **PHASE 3: Core Processing Logic**

#### **Task 3.1: Replace Delegate Action Processor Logic**
- **File**: `src/processors/delegate_actions.rs`
- **Changes**:
  - Replace `process_request_delegate_action()`:
    - Remove old `PoolPause` handling completely
    - Implement `PausePoolSwaps` and `UnpausePoolSwaps` action types
    - Validate pause requests (can't pause if swaps already paused, etc.)
    - Validate unpause requests (can't unpause if swaps not paused, etc.)
    - Store reason for pause for transparency
  - Replace `process_execute_delegate_action()`:
    - Remove old duration-based pause execution completely
    - Implement `PausePoolSwaps` execution (set `swaps_paused = true`, no end timestamp)
    - Implement `UnpausePoolSwaps` execution (set `swaps_paused = false`, clear pause data)
    - Update pool state tracking fields with delegate info
    - Remove auto-unpause check (Phase 1 removal)
  - Replace `validate_action_params()`:
    - Remove old `PoolPause` validation completely
    - Add validation for new `PausePoolSwaps` and `UnpausePoolSwaps` types
    - Validate reason string length (max 200 chars)

#### **Task 3.2: Update Swap Processor for Pool Pause**
- **File**: `src/processors/swap.rs`
- **Changes**:
  - Add pool-specific pause validation to `process_swap()`
  ```rust
  pub fn process_swap(
      program_id: &Pubkey,
      accounts: &[AccountInfo],
      input_token_mint: Pubkey,
      amount_in: u64,
      minimum_amount_out: u64,
  ) -> ProgramResult {
      // ✅ SYSTEM PAUSE: Check system-wide pause first (existing)
      crate::utils::validation::validate_system_not_paused_safe(accounts, 14)?;
      
      // ✅ POOL SWAP PAUSE: Check pool-specific swap pause (NEW)
      validate_pool_swaps_not_paused(accounts)?;
      
      // ... rest of swap logic unchanged ...
  }
  
  /// Validates that pool swaps are not paused (granular pool check)
  fn validate_pool_swaps_not_paused(accounts: &[AccountInfo]) -> ProgramResult {
      let pool_state_account = &accounts[2]; // Pool state PDA
      let pool_state_data = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
      
      if pool_state_data.swaps_paused {
          msg!("Pool swaps are currently paused by delegate");
          msg!("Paused by: {:?}", pool_state_data.swaps_pause_requested_by);
          msg!("Paused at: {}", pool_state_data.swaps_pause_initiated_timestamp);
          if let Some(reason) = &pool_state_data.swaps_pause_reason {
              msg!("Reason: {}", reason);
          }
          msg!("Note: Deposits and withdrawals are still available");
          return Err(PoolError::PoolSwapsPaused.into());
      }
      
      Ok(())
  }
  ```

#### **Task 3.3: Enhanced Withdrawal with Automatic Slippage Protection**
- **File**: `src/processors/liquidity.rs`
- **Changes**:
  - Enhance `process_withdraw()` with automatic swap pause for large withdrawals
  - Add slippage protection logic to prevent front-running
  ```rust
  pub fn process_withdraw(/* ... */) -> ProgramResult {
      // ✅ SYSTEM PAUSE: Only check system-wide pause (existing)
      crate::utils::validation::validate_system_not_paused_safe(accounts, 14)?;
      
      // ✅ NO POOL SWAP PAUSE CHECK: Withdrawals work regardless of delegate pool swap pause
      
      // 🛡️ AUTOMATIC SLIPPAGE PROTECTION: Check if we should temporarily pause swaps
      let pool_state_account = &accounts[3];
      let mut pool_state_data = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
      
      // Determine if this withdrawal needs swap protection
      let needs_protection = should_protect_withdrawal_from_slippage(
          lp_amount_to_burn,
          &pool_state_data,
      )?;
      
      if needs_protection {
          // Temporarily pause swaps to protect this withdrawal
          initiate_withdrawal_protection(&mut pool_state_data, user_signer.key)?;
          msg!("🛡️ Temporarily pausing swaps to protect large withdrawal from slippage");
      }
      
      // Perform the withdrawal safely (no concurrent swaps can interfere)
      let withdrawal_result = execute_withdrawal_logic(
          &mut pool_state_data,
          lp_amount_to_burn,
          withdraw_token_mint,
          /* other params */
      );
      
      // Always re-enable swaps after withdrawal (regardless of success/failure)
      if needs_protection {
          complete_withdrawal_protection(&mut pool_state_data)?;
          msg!("🔓 Re-enabling swaps after withdrawal completion");
      }
      
      // Save updated pool state
      pool_state_data.serialize(&mut *pool_state_account.data.borrow_mut())?;
      
      withdrawal_result
  }
  
  /// Determines if a withdrawal needs protection from swap interference
  fn should_protect_withdrawal_from_slippage(
      lp_amount_to_burn: u64,
      pool_state: &PoolState,
  ) -> Result<bool, ProgramError> {
      // Calculate withdrawal as percentage of total pool liquidity
      let total_lp_supply = pool_state.total_token_a_liquidity + pool_state.total_token_b_liquidity;
      if total_lp_supply == 0 {
          return Ok(false); // Empty pool, no protection needed
      }
      
      let withdrawal_percentage = (lp_amount_to_burn * 100) / total_lp_supply;
      
      // Protect withdrawals >5% of total pool to prevent slippage/front-running
      const LARGE_WITHDRAWAL_THRESHOLD: u64 = 5;
      
      if withdrawal_percentage >= LARGE_WITHDRAWAL_THRESHOLD {
          msg!("Large withdrawal detected: {}% of pool. Enabling slippage protection.", withdrawal_percentage);
          return Ok(true);
      }
      
      // Also check if swaps are already paused by delegates (don't interfere)
      if pool_state.swaps_paused {
          msg!("Swaps already paused by delegate - no additional protection needed");
          return Ok(false);
      }
      
      Ok(false)
  }
  
  /// Temporarily pause swaps to protect withdrawal from slippage
  fn initiate_withdrawal_protection(
      pool_state: &mut PoolState,
      withdrawer: &Pubkey,
  ) -> ProgramResult {
      // Only pause if not already paused by delegates
      if !pool_state.swaps_paused {
          pool_state.swaps_paused = true;
          pool_state.swaps_pause_requested_by = Some(*withdrawer);
          pool_state.swaps_pause_initiated_timestamp = Clock::get()?.unix_timestamp;
          pool_state.swaps_pause_reason = Some("Automatic slippage protection during large withdrawal".to_string());
          
          // Mark this as a temporary withdrawal protection pause
          pool_state.withdrawal_protection_active = true;
      }
      
      Ok(())
  }
  
  /// Re-enable swaps after withdrawal protection
  fn complete_withdrawal_protection(pool_state: &mut PoolState) -> ProgramResult {
      // Only unpause if this was our withdrawal protection pause
      if pool_state.withdrawal_protection_active {
          pool_state.swaps_paused = false;
          pool_state.swaps_pause_requested_by = None;
          pool_state.swaps_pause_reason = None;
          pool_state.withdrawal_protection_active = false;
          
          msg!("Withdrawal protection completed - swaps re-enabled");
      }
      
      Ok(())
  }

#### **Task 3.4: Update Pool State for Withdrawal Protection**
- **File**: `src/types/pool_state.rs`
- **Changes**:
  - Add withdrawal protection tracking field
  ```rust
  #[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
  pub struct PoolState {
      // ... existing fields ...
      
      // Pool-specific swap pause controls (separate from system pause)
      pub swaps_paused: bool,
      pub swaps_pause_requested_by: Option<Pubkey>,
      pub swaps_pause_initiated_timestamp: i64,
      pub swaps_pause_reason: Option<String>,
      
      // Automatic withdrawal protection
      pub withdrawal_protection_active: bool,
      
      // ❌ REMOVE: pause_end_timestamp: i64,
      // ❌ REMOVE: pause_reason: PoolPauseReason,
  }
  ```

#### **Task 3.5: Keep Deposit Operations Unchanged**
- **File**: `src/processors/liquidity.rs`
- **Changes**:
  - Ensure `process_deposit()` does NOT check pool swap pause
  - Only system pause validation remains (existing behavior)
  ```rust
  pub fn process_deposit(/* ... */) -> ProgramResult {
      // ✅ SYSTEM PAUSE: Only check system-wide pause (existing)
      crate::utils::validation::validate_system_not_paused_safe(accounts, 14)?;
      
      // ✅ NO POOL SWAP PAUSE CHECK: Deposits work regardless of pool swap pause
      // ✅ NO WITHDRAWAL PROTECTION CHECK: Deposits work during withdrawal protection
      
      // ... existing deposit logic unchanged ...
  }
  ```

#### **Task 3.6: Update Action Cancellation**
- **File**: `src/processors/delegate_actions.rs`
- **Changes**:
  - Update `process_revoke_action()`:
    - Allow pool owner to cancel any pending pool pause/unpause request
    - Allow delegate to cancel their own requests
    - Add specific error messages for pause/unpause cancellations

---

### **PHASE 4: Public Status and Transparency**

#### **Task 4.1: Add Pool Pause Status Query**
- **File**: `src/processors/utilities.rs`
- **Changes**:
  ```rust
  /// Returns current pool pause status - publicly accessible
  pub fn get_pool_pause_status(accounts: &[AccountInfo]) -> ProgramResult {
      let pool_state_account = &accounts[0];
      let pool_state_data = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
      
      // Log comprehensive pause status for public visibility
      msg!("=== POOL STATUS ===");
      msg!("Swaps: {}", if pool_state_data.swaps_paused { "PAUSED" } else { "ENABLED" });
      msg!("Deposits: ENABLED");  // Always enabled (only system pause affects)
      msg!("Withdrawals: ENABLED"); // Always enabled (only system pause affects)
      
      if pool_state_data.swaps_paused {
          msg!("=== PAUSE DETAILS ===");
          msg!("Paused by: {:?}", pool_state_data.swaps_pause_requested_by);
          msg!("Paused at: {}", pool_state_data.swaps_pause_initiated_timestamp);
          if let Some(reason) = &pool_state_data.swaps_pause_reason {
              msg!("Reason: {}", reason);
          }
          msg!("Note: No auto-unpause - requires manual unpause action");
      }
      
      msg!("==================");
      
      Ok(())
  }
  
  /// Enhanced pool info that includes pause status
  pub fn get_pool_info(accounts: &[AccountInfo]) -> ProgramResult {
      let pool_state_account = &accounts[0];
      let pool_state_data = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
      
      // Standard pool info
      msg!("Pool token A liquidity: {}", pool_state_data.total_token_a_liquidity);
      msg!("Pool token B liquidity: {}", pool_state_data.total_token_b_liquidity);
      msg!("Pool ratio: {}", pool_state_data.ratio_primary_per_base);
      
      // Operations status for user guidance
      msg!("Current operations status:");
      msg!("  Swaps: {}", if pool_state_data.swaps_paused { "PAUSED (delegate)" } else { "ENABLED" });
      msg!("  Deposits: ENABLED");
      msg!("  Withdrawals: ENABLED");
      
      Ok(())
  }
  ```

#### **Task 4.2: Update Utilities for New Action Types**
- **File**: `src/processors/utilities.rs`
- **Changes**:
  - Update `get_action_wait_time()` to handle new action types
  ```rust
  pub fn get_action_wait_time(pool_state: &PoolState, delegate: &Pubkey, action_type: &DelegateActionType) -> Option<u64> {
      if let Some(time_limits) = pool_state.delegate_management.get_delegate_time_limits(delegate) {
          match action_type {
              DelegateActionType::FeeChange => Some(time_limits.fee_change_wait_time),
              DelegateActionType::Withdrawal => Some(time_limits.withdraw_wait_time),
              DelegateActionType::PausePoolSwaps => Some(time_limits.pause_wait_time),
              DelegateActionType::UnpausePoolSwaps => Some(time_limits.pause_wait_time),
              // ❌ REMOVE: DelegateActionType::PoolPause => ...
          }
      } else {
          None
      }
  }
  ```

### **PHASE 5: Error Handling**

#### **Task 5.1: Add Pool-Specific Error Types**
- **File**: `src/error.rs`
- **Changes**:
  ```rust
  #[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
  pub enum PoolError {
      // ... existing errors ...
      
      #[error("Pool swaps are currently paused by delegate")]
      PoolSwapsPaused = 4000,
      
      #[error("Pool swaps are already paused")]
      PoolSwapsAlreadyPaused = 4001,
      
      #[error("Pool swaps are not currently paused")]
      PoolSwapsNotPaused = 4002,
      
      #[error("Invalid pause reason (must be 1-200 characters)")]
      InvalidPauseReason = 4003,
  }
  ```

---

### **PHASE 6: Remove Old Test Infrastructure and Add New Tests**

#### **Task 6.1: Remove Old Duration-Based Tests**
- **File**: `tests/test_delegates.rs`
- **Changes**:
  - **COMPLETELY REMOVE** `test_request_delegate_action_pool_pause()` function
  - **COMPLETELY REMOVE** all duration-based test constants:
    ```rust
    // ❌ REMOVE these constants:
    // const MIN_PAUSE_DURATION: u64 = 60;
    // const VALID_PAUSE_SHORT: u64 = 3600;
    // const VALID_PAUSE_MEDIUM: u64 = 7200;
    // const INVALID_TOO_SHORT: u64 = 30;
    // const INVALID_TOO_LONG: u64 = 300000;
    ```
  - **REMOVE** all auto-unpause validation in existing tests
  - **REMOVE** all `pause_end_timestamp` assertions in tests
  - **REMOVE** duration validation logic from tests

#### **Task 6.2: Remove Old Test Helper Functions**
- **File**: `tests/test_delegates.rs`
- **Changes**:
  - **REMOVE** any helper functions that test duration-based pause functionality
  - **REMOVE** functions that validate `pause_end_timestamp`
  - **REMOVE** auto-unpause test scenarios

#### **Task 6.3: Document New Pool Pause Tests**
- **File**: `docs/tests/COMPREHENSIVE_TESTING_PLAN.md`
  - **Changes**:
  - Add new test module section: **Module 11: Pool-Specific Swap Pause (0% → 90% target)**
  - Document five comprehensive tests following UTIL-002 format:
    - **POOL-PAUSE-001**: `test_delegate_pause_swaps_only` - Delegate pause affects only swaps, not deposits/withdrawals
    - **POOL-PAUSE-002**: `test_pool_pause_status_query` - Public pause status visibility and transparency
    - **POOL-PAUSE-003**: `test_delegate_unpause_cycle` - Complete pause/unpause cycle with manual controls
    - **POOL-PAUSE-004**: `test_indefinite_pause_no_auto_unpause` - Indefinite pause without auto-unpause
    - **POOL-PAUSE-005**: `test_pause_reason_validation` - Pause reason string validation and error handling
  - Each test documented with:
    - **🔧 FEATURES TO TEST** section with numbered technical specifications (minimum 6 per test)
    - **📊 EXPECTED OUTCOMES** section with bullet-pointed results (minimum 5 per test)
    - Swap-only pause validation (deposits/withdrawals unaffected)
    - Delegate action wait time enforcement
    - Manual control validation (no auto-unpause)
    - Public status query functionality
    - Error handling for invalid pause reasons
    - Integration with existing delegate management system

#### **Task 6.4: Update Existing Delegate Tests**
- **File**: `tests/test_delegates.rs`
- **Changes**:
  - Update `test_execute_delegate_action_success()`:
    - Remove old `PoolPause` execution tests completely
    - Add new `PausePoolSwaps` and `UnpausePoolSwaps` execution tests
    - Remove duration and `pause_end_timestamp` validations
    - Verify only swaps are affected, not deposits/withdrawals
    - Test wait time enforcement for new action types

#### **Task 6.5: Document Automatic Withdrawal Protection Tests**
- **File**: `docs/tests/COMPREHENSIVE_TESTING_PLAN.md`
- **Changes**:
  - Add new test module section: **Module 12: Automatic Withdrawal Protection (0% → 90% target)**
  - Document six comprehensive tests following UTIL-002 format:
    - **WITHDRAWAL-PROTECTION-001**: `test_large_withdrawal_automatic_protection` - Large withdrawal automatic protection (>5% threshold)
    - **WITHDRAWAL-PROTECTION-002**: `test_small_withdrawal_no_protection` - Small withdrawal no protection (<5% threshold)
    - **WITHDRAWAL-PROTECTION-003**: `test_withdrawal_protection_with_delegate_pause` - Integration with delegate pause system
    - **WITHDRAWAL-PROTECTION-004**: `test_withdrawal_failure_cleanup` - Withdrawal failure cleanup verification
    - **WITHDRAWAL-PROTECTION-005**: `test_concurrent_withdrawals_protection` - Concurrent withdrawal protection handling
    - **WITHDRAWAL-PROTECTION-006**: `test_withdrawal_protection_status_visibility` - Protection status visibility and transparency
  - Each test documented with:
    - **🔧 FEATURES TO TEST** section with numbered technical specifications (minimum 6 per test)
    - **📊 EXPECTED OUTCOMES** section with bullet-pointed results (minimum 5 per test)
    - Integration testing with delegate pause system
    - Error handling and cleanup verification
    - Status visibility and transparency testing
    - Slippage protection and MEV prevention validation
    - Automatic cleanup on success and failure scenarios

---

### **PHASE 7: Cleanup and Finalization**

#### **Task 7.1: Remove Legacy Pause Constants**
- **File**: `src/constants.rs` 
- **Changes**:
  - Remove any duration-related constants for pool pause
  - Keep only wait-time related constants

#### **Task 7.2: Update Documentation Comments**
- **Files**: All modified source files
- **Changes**:
  - Remove references to "auto-unpause" and "duration" in doc comments
  - Update comments to reflect indefinite pause until manual unpause
  - Document that only swaps are affected by pool pause

#### **Task 7.3: Final State Cleanup**
- **File**: `src/processors/pool_creation.rs`
- **Changes**:
  - Remove initialization of `pause_end_timestamp` field
  - Initialize new `swaps_paused` and related fields

---

## 📊 **SUCCESS METRICS**

- [ ] **Old System Completely Removed**: No duration-based pause code remains
- [ ] **No Auto-Unpause**: Pool stays paused until manually unpaused  
- [ ] **Delegates can request pool swap pause** with time delay
- [ ] **Delegates can request pool swap unpause** with time delay  
- [ ] **Pool owner can cancel** any pending pause/unpause request
- [ ] **Pool swap pause only affects** `process_swap()` operations
- [ ] **Deposits work normally** during pool swap pause
- [ ] **Withdrawals work normally** during pool swap pause
- [ ] **Pool pause status is publicly queryable** via `GetPoolPauseStatus`
- [ ] **Automatic Withdrawal Protection**: Large withdrawals (>5%) automatically pause swaps temporarily
- [ ] **Slippage Prevention**: No front-running or interference during protected withdrawals
- [ ] **Automatic Cleanup**: Swaps re-enabled after withdrawal completion (success or failure)
- [ ] **Old duration-based tests completely removed** and replaced
- [ ] **New test coverage** for swap-only pause and withdrawal protection
- [ ] **Clear error messages** distinguish pool pause from system pause

## 🎯 **KEY DESIGN PRINCIPLES**

1. **Complete Legacy Removal**: All time-based, duration-based pause code completely removed
2. **Pool-Specific Control**: Each pool can be paused independently by its delegates
3. **Swap-Only Impact**: Pool pause only affects swaps, never deposits or withdrawals  
4. **Time-Delayed Governance**: All delegate actions require wait period before execution
5. **Owner Override**: Pool owner can always cancel pending requests for emergency control
6. **Public Transparency**: Pool pause status easily readable by users before trading
7. **Manual Control**: No automatic state changes - all pause/unpause must be explicitly executed
8. **Indefinite Pause**: Pool stays paused until manually unpaused (no auto-unpause)
9. **Clear Separation**: Pool pause distinct from system pause (both can coexist)
10. **Automatic Slippage Protection**: Large withdrawals automatically pause swaps temporarily
11. **MEV Prevention**: No front-running or sandwich attacks during protected withdrawals
12. **Fail-Safe Design**: Protection cleanup happens regardless of withdrawal success/failure

## ⚠️ **IMPLEMENTATION NOTES**

- **Breaking Changes OK**: Since project not deployed, we can safely remove old time-based system
- **Scope**: This is pool-specific pause, not system-wide pause (system pause already works)
- **Operations**: Only swaps are affected by pool pause - deposits/withdrawals always work
- **Transparency**: Users can check pool pause status before attempting swaps
- **Coexistence**: Pool pause and system pause are independent (system pause overrides pool pause)
- **Delegate Control**: Delegates control pool-specific pause, not system-wide pause
- **Simplification**: Removing duration/auto-unpause makes system much simpler and more predictable

This focused refactor **completely removes the confusing time-based pause system** and replaces it with a simple, predictable **swap-only pause** that lasts until manually unpaused, while ensuring deposits/withdrawals are never affected by pool-level pause controls. 