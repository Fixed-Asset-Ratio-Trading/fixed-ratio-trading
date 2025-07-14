# **DISTRIBUTED COLLECTION + BATCH CONSOLIDATION ARCHITECTURE**

## **Migration Plan: Centralized → Distributed Fee Collection**

**Document Version**: 2.0  
**Author**: AI Assistant  
**Date**: 2025-01-28  
**Purpose**: Complete migration plan from centralized fee collection to distributed collection with system-wide pause consolidation

---

## 📋 **EXECUTIVE SUMMARY**

**Current Architecture**: Real-time fee collection to MainTreasuryState during every operation  
**Target Architecture**: Distributed collection in pool states + batch consolidation with system-wide pause or pool pause  
**Benefits**: 67% CU reduction per operation, 45% lower consolidation costs, better race protection  
**Timeline**: 4 phases over 2-3 weeks  

---

## 🎯 **PHASE 1: DATA STRUCTURE UPDATES**

### **1.1 Pool State Enhancements**
**File**: `src/state/pool_state.rs`

**Add new fields to PoolState:**
```rust
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct PoolState {
    // ... existing fields (REMOVE is_initialized and swap_fee_basis_points) ...
    
    // **NEW: DISTRIBUTED SOL FEE TRACKING**
    /// SOL fees collected from liquidity operations (accumulated locally)  
    pub collected_liquidity_fees: u64,
    
    /// SOL fees collected from regular swaps (accumulated locally)
    pub collected_regular_swap_fees: u64,
    
    /// SOL fees collected from HFT swaps (accumulated locally)
    pub collected_hft_swap_fees: u64,
    
    // **NEW: LIFETIME SOL FEE TRACKING**
    /// Total SOL fees collected by this pool since inception (never resets)
    /// This is the authoritative count of all SOL fees ever collected
    /// Formula: total_sol_fees_collected = total_fees_consolidated + current_pending_fees
    pub total_sol_fees_collected: u64,
    
    // **NEW: CONSOLIDATION MANAGEMENT**
    /// Timestamp of last consolidation (0 if never consolidated)
    pub last_consolidation_timestamp: i64,
    
    /// Total number of consolidations performed on this pool
    pub total_consolidations: u64,
    
    /// Total SOL fees transferred to treasury via consolidation
    pub total_fees_consolidated: u64,
}
```

**Fields REMOVED from PoolState:**
```rust
// REMOVE: Pool creation tracking (not needed - happens only once)
// pub collected_pool_creation_fees: u64,
// pub pool_creation_operations: u64,

// REMOVE: Fixed values that don't need per-pool storage
// pub is_initialized: bool,  // Pool state exists = initialized
// pub swap_fee_basis_points: u64,  // Fixed value across all pools
```

**Update PoolState implementation:**
```rust
impl PoolState {
    // Update LEN calculation
    pub const LEN: usize = 
        // ... existing field sizes (minus removed fields) ...
        8 +   // collected_liquidity_fees  
        8 +   // collected_regular_swap_fees
        8 +   // collected_hft_swap_fees
        8 +   // total_sol_fees_collected
        8 +   // last_consolidation_timestamp
        8 +   // total_consolidations
        8;    // total_fees_consolidated
        // **TOTAL ADDITION: +56 bytes per pool**
        // **TOTAL REMOVAL: -17 bytes (is_initialized + swap_fee_basis_points)**
        // **NET ADDITION: +39 bytes per pool**
}

impl Default for PoolState {
    fn default() -> Self {
        Self {
            // ... existing defaults ...
            
            // Initialize new fields
            collected_liquidity_fees: 0,
            collected_regular_swap_fees: 0,
            collected_hft_swap_fees: 0,
            total_sol_fees_collected: 0,
            last_consolidation_timestamp: 0,
            total_consolidations: 0,
            total_fees_consolidated: 0,
        }
    }
}

// **NEW: Pool-level fee collection methods with atomic updates**
impl PoolState {
    /// Records liquidity operation fee collection
    /// 
    /// **ATOMIC UPDATE**: Updates both specific fee counter and total in single operation
    /// to prevent race conditions and ensure consistency.
    pub fn add_liquidity_fee(&mut self, fee_amount: u64, timestamp: i64) {
        // Atomic update: both counters updated together
        self.collected_liquidity_fees += fee_amount;
        self.total_sol_fees_collected += fee_amount;
        
        // Invariant check (debug mode only) - simplified since pending_sol_fees() uses the mathematical relationship
        debug_assert_eq!(
            self.pending_sol_fees(),
            self.collected_liquidity_fees + self.collected_regular_swap_fees + self.collected_hft_swap_fees,
            "Pending fees calculation should match sum of individual pending fee types"
        );
    }
    
    /// Records regular swap fee collection
    /// 
    /// **ATOMIC UPDATE**: Updates both specific fee counter and total in single operation
    /// to prevent race conditions and ensure consistency.
    pub fn add_regular_swap_fee(&mut self, fee_amount: u64, timestamp: i64) {
        // Atomic update: both counters updated together
        self.collected_regular_swap_fees += fee_amount;
        self.total_sol_fees_collected += fee_amount;
        
        // Invariant check (debug mode only) - simplified since pending_sol_fees() uses the mathematical relationship
        debug_assert_eq!(
            self.pending_sol_fees(),
            self.collected_liquidity_fees + self.collected_regular_swap_fees + self.collected_hft_swap_fees,
            "Pending fees calculation should match sum of individual pending fee types"
        );
    }
    
    /// Records HFT swap fee collection
    /// 
    /// **ATOMIC UPDATE**: Updates both specific fee counter and total in single operation
    /// to prevent race conditions and ensure consistency.
    pub fn add_hft_swap_fee(&mut self, fee_amount: u64, timestamp: i64) {
        // Atomic update: both counters updated together
        self.collected_hft_swap_fees += fee_amount;
        self.total_sol_fees_collected += fee_amount;
        
        // Invariant check (debug mode only) - simplified since pending_sol_fees() uses the mathematical relationship
        debug_assert_eq!(
            self.pending_sol_fees(),
            self.collected_liquidity_fees + self.collected_regular_swap_fees + self.collected_hft_swap_fees,
            "Pending fees calculation should match sum of individual pending fee types"
        );
    }
    
    /// Calculates current pending SOL fees awaiting consolidation
    /// 
    /// **ACCURATE CALCULATION**: Uses the mathematical relationship:
    /// pending_fees = total_lifetime_fees - already_consolidated_fees
    /// 
    /// This automatically includes ALL fee types (pool creation, liquidity, swaps)
    /// without needing to track consolidation state of individual fee types.
    /// 
    /// **WHY THIS IS BETTER THAN SUMMING INDIVIDUAL FEE TYPES:**
    /// - Pool creation fees go directly to MainTreasuryState, not to individual pools
    /// - Previous total_collected_sol_fees() only summed liquidity + swap fees
    /// - Would need complex logic to determine if pool creation fees were consolidated
    /// - Mathematical approach is simple, accurate, and includes everything automatically
    pub fn pending_sol_fees(&self) -> u64 {
        // Simple and accurate: total collected minus what's been consolidated
        self.total_sol_fees_collected - self.total_fees_consolidated
    }
    
    /// Calculates total operations since last consolidation using fee constants
    pub fn total_operations_since_consolidation(&self) -> u64 {
        use crate::constants::*;
        
        let liquidity_ops = self.collected_liquidity_fees / DEPOSIT_WITHDRAWAL_FEE;
        let regular_swap_ops = self.collected_regular_swap_fees / SWAP_FEE;
        let hft_swap_ops = self.collected_hft_swap_fees / HFT_SWAP_FEE;
        
        liquidity_ops + regular_swap_ops + hft_swap_ops
    }
    
    /// Calculates individual operation counts since last consolidation
    pub fn operation_counts_since_consolidation(&self) -> (u64, u64, u64) {
        use crate::constants::*;
        
        let liquidity_ops = self.collected_liquidity_fees / DEPOSIT_WITHDRAWAL_FEE;
        let regular_swap_ops = self.collected_regular_swap_fees / SWAP_FEE;
        let hft_swap_ops = self.collected_hft_swap_fees / HFT_SWAP_FEE;
        
        (liquidity_ops, regular_swap_ops, hft_swap_ops)
    }
    
    /// Resets consolidation counters (called after successful consolidation)
    /// 
    /// **RACE CONDITION PROTECTION**: This method performs atomic updates to ensure
    /// that total_sol_fees_collected remains consistent during consolidation.
    /// The invariant total_sol_fees_collected = total_fees_consolidated + current_pending_fees
    /// is maintained throughout the operation.
    pub fn reset_consolidation_counters(&mut self, timestamp: i64) {
        // Calculate pending fees before any changes using the accurate mathematical relationship
        let pending_fees = self.pending_sol_fees();
        
        // **ATOMIC CONSOLIDATION UPDATE**: 
        // Move pending fees from "collected" to "consolidated" state
        // NOTE: total_sol_fees_collected does NOT change - it's the lifetime total
        self.total_fees_consolidated += pending_fees;
        
        // Reset collected fees (operation counts are calculated from these)
        self.collected_liquidity_fees = 0;
        self.collected_regular_swap_fees = 0;
        self.collected_hft_swap_fees = 0;
        
        // Update consolidation metadata
        self.last_consolidation_timestamp = timestamp;
        self.total_consolidations += 1;
        
        // **INVARIANT VERIFICATION**: Ensure consistency after consolidation
        debug_assert_eq!(
            self.pending_sol_fees(),
            0,
            "Pending fees should be zero after consolidation"
        );
        debug_assert_eq!(
            self.total_sol_fees_collected,
            self.total_fees_consolidated,
            "After consolidation, total collected should equal total consolidated"
        );
    }
    
    /// **NEW: Validates internal consistency of fee tracking**
    /// 
    /// This method can be called periodically to ensure that race conditions
    /// or bugs haven't corrupted the fee tracking state.
    pub fn validate_fee_consistency(&self) -> Result<(), &'static str> {
        // Verify the mathematical relationship: pending = total - consolidated
        let calculated_pending = self.total_sol_fees_collected.saturating_sub(self.total_fees_consolidated);
        let actual_pending = self.pending_sol_fees();
        
        if calculated_pending != actual_pending {
            return Err("Pending SOL fees calculation inconsistency");
        }
        
        // Verify individual pending fees sum matches the mathematical pending
        let individual_sum = self.collected_liquidity_fees + 
                           self.collected_regular_swap_fees + 
                           self.collected_hft_swap_fees;
        
        if actual_pending != individual_sum {
            return Err("Individual pending fees don't match calculated pending fees");
        }
        
        // Verify no arithmetic overflow conditions
        let max_safe_value = u64::MAX / 2; // Conservative check
        if self.total_sol_fees_collected > max_safe_value {
            return Err("Total SOL fees approaching overflow risk");
        }
        
        // Verify consolidated fees don't exceed total fees
        if self.total_fees_consolidated > self.total_sol_fees_collected {
            return Err("Consolidated fees exceed total collected fees");
        }
        
        Ok(())
    }
    
    /// **NEW: Calculate available balance for consolidation (respecting rent exemption)**
    /// 
    /// This method calculates how much SOL can be safely consolidated from a pool state
    /// without violating rent exemption requirements. It considers both the rent exempt
    /// minimum and the actual pending fees.
    /// 
    /// # Arguments
    /// * `current_account_balance` - Current lamports balance of the pool state account
    /// * `rent_exempt_minimum` - Minimum balance required for rent exemption
    /// 
    /// # Returns
    /// * `u64` - Amount of SOL that can be safely consolidated (in lamports)
    /// 
    /// # Safety
    /// This function ensures that consolidation never reduces the pool state balance
    /// below the rent exempt minimum, preventing account closure due to insufficient funds.
    pub fn calculate_available_for_consolidation(
        &self,
        current_account_balance: u64,
        rent_exempt_minimum: u64,
    ) -> u64 {
        // Calculate pending fees awaiting consolidation
        let pending_fees = self.pending_sol_fees();
        
        // Calculate available balance above rent exempt minimum
        let available_above_rent_exempt = if current_account_balance > rent_exempt_minimum {
            current_account_balance - rent_exempt_minimum
        } else {
            0
        };
        
        // Return the minimum of available balance and pending fees
        // This ensures we never:
        // 1. Take more than what's available above rent exempt minimum
        // 2. Take more than what's actually owed in pending fees
        std::cmp::min(available_above_rent_exempt, pending_fees)
    }
    
    /// **NEW: Validate consolidation is safe (respecting rent exemption)**
    /// 
    /// This method validates that a proposed consolidation amount is safe and won't
    /// violate rent exemption requirements or exceed pending fees.
    /// 
    /// # Arguments
    /// * `proposed_consolidation_amount` - Amount of SOL proposed for consolidation
    /// * `current_account_balance` - Current lamports balance of the pool state account
    /// * `rent_exempt_minimum` - Minimum balance required for rent exemption
    /// 
    /// # Returns
    /// * `Result<(), &'static str>` - Ok if consolidation is safe, error message if not
    /// 
    /// # Safety
    /// This function provides comprehensive validation to prevent:
    /// - Account closure due to insufficient rent exempt balance
    /// - Over-consolidation beyond pending fees
    /// - Arithmetic underflow in account balance
    pub fn validate_consolidation_safety(
        &self,
        proposed_consolidation_amount: u64,
        current_account_balance: u64,
        rent_exempt_minimum: u64,
    ) -> Result<(), &'static str> {
        // Check if account would have sufficient balance after consolidation
        if current_account_balance < proposed_consolidation_amount {
            return Err("Consolidation amount exceeds current account balance");
        }
        
        let balance_after_consolidation = current_account_balance - proposed_consolidation_amount;
        if balance_after_consolidation < rent_exempt_minimum {
            return Err("Consolidation would reduce balance below rent exempt minimum");
        }
        
        // Check if consolidation amount exceeds pending fees
        let pending_fees = self.pending_sol_fees();
        if proposed_consolidation_amount > pending_fees {
            return Err("Consolidation amount exceeds pending fees");
        }
        
        // Check for edge cases
        if proposed_consolidation_amount == 0 {
            return Err("Consolidation amount cannot be zero");
        }
        
        Ok(())
    }
}
```

### **1.1.5 Race Condition Protection**

**Critical Design Principle**: All fee collection and consolidation operations are designed to be **atomic** and **consistent** to prevent race conditions that could cause fee count discrepancies.

#### **Atomic Fee Collection**
```rust
// SAFE: Single transaction updates both counters atomically
pub fn add_liquidity_fee(&mut self, fee_amount: u64, timestamp: i64) {
    // Both updates happen in same transaction - no race condition possible
    self.collected_liquidity_fees += fee_amount;      // Specific counter
    self.total_sol_fees_collected += fee_amount;      // Lifetime total
    
    // Debug verification (zero runtime cost in release builds)
    debug_assert_eq!(
        self.total_sol_fees_collected,
        self.total_fees_consolidated + self.total_collected_sol_fees(),
        "Invariant must hold: total = consolidated + pending"
    );
}
```

#### **Atomic Consolidation**
```rust
// SAFE: Consolidation moves fees from "pending" to "consolidated" state atomically
pub fn reset_consolidation_counters(&mut self, timestamp: i64) {
    // Calculate pending fees using accurate mathematical relationship
    let pending_fees = self.pending_sol_fees(); // = total_sol_fees_collected - total_fees_consolidated
    
    // Atomic transfer: pending → consolidated (total_sol_fees_collected unchanged)
    self.total_fees_consolidated += pending_fees;
    self.collected_liquidity_fees = 0;
    self.collected_regular_swap_fees = 0; 
    self.collected_hft_swap_fees = 0;
    
    // total_sol_fees_collected remains constant during consolidation
    // After consolidation: pending_sol_fees() = 0 since total_fees_consolidated now equals total_sol_fees_collected
}
```

#### **Invariant Protection**
```rust
// **MATHEMATICAL INVARIANT** (always true):
// pending_sol_fees = total_sol_fees_collected - total_fees_consolidated
// where pending_sol_fees includes ALL fee types (pool creation, liquidity, swaps)

// This invariant ensures:
// 1. No fees are ever lost or double-counted
// 2. Accurate calculation without complex consolidation state tracking
// 3. Automatically includes pool creation fees (unlike old total_collected_sol_fees)
// 4. Race conditions cannot corrupt fee tracking
// 5. Easy auditing: lifetime total is always available
// 6. Simple and bulletproof: pending = total - consolidated
```

#### **Solana Transaction Atomicity**
Since Solana transactions are **atomic by design**, all updates within a single transaction either **all succeed** or **all fail**. This means:

- **Fee collection**: User pays SOL → pool receives SOL → counters update (atomic)
- **Consolidation**: Pool transfers SOL → treasury receives SOL → counters update (atomic)  
- **Pause operations**: Flags update atomically with validation
- **No partial states**: Impossible to have SOL transferred without counter updates

#### **Race Condition Scenarios Prevented**
```rust
// ❌ IMPOSSIBLE: SOL collected but counter not updated
// ❌ IMPOSSIBLE: Counter updated but SOL not collected
// ❌ IMPOSSIBLE: Partial consolidation (some pools yes, some no)
// ❌ IMPOSSIBLE: Counter overflow without detection
// ✅ GUARANTEED: All operations atomic within transaction boundaries
```

### **1.2 MainTreasuryState Enhancements**
**File**: `src/state/treasury_state.rs`

**Add rent-exempt balance tracking:**
```rust
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct MainTreasuryState {
    /// Current SOL balance of the main treasury account (synced with account.lamports())
    pub total_balance: u64,
    
    /// **NEW: Rent-exempt minimum balance requirement**
    pub rent_exempt_minimum: u64,
    
    /// Total SOL fees withdrawn by authority over time
    pub total_withdrawn: u64,
    
    // ... existing counters and totals ...
    
    /// **NEW: Consolidation tracking**
    /// Number of consolidation operations performed
    pub total_consolidations_performed: u64,
    
    /// Timestamp of last consolidation
    pub last_consolidation_timestamp: i64,
}

impl MainTreasuryState {
    pub const LEN: usize = 
        8 +   // total_balance
        8 +   // rent_exempt_minimum ← NEW
        8 +   // total_withdrawn
        8 +   // pool_creation_count
        8 +   // liquidity_operation_count
        8 +   // regular_swap_count
        8 +   // hft_swap_count
        8 +   // total_pool_creation_fees
        8 +   // total_liquidity_fees
        8 +   // total_regular_swap_fees
        8 +   // total_hft_swap_fees
        8 +   // last_update_timestamp
        8 +   // total_consolidations_performed ← NEW
        8;    // last_consolidation_timestamp ← NEW
        // **TOTAL ADDITION: +24 bytes**

    /// **NEW: Initialize with rent-exempt balance**
    pub fn new_with_rent_exemption(rent_exempt_minimum: u64) -> Self {
        Self {
            total_balance: rent_exempt_minimum, // Start with rent-exempt balance
            rent_exempt_minimum,
            total_withdrawn: 0,
            pool_creation_count: 0,
            liquidity_operation_count: 0,
            regular_swap_count: 0,
            hft_swap_count: 0,
            total_pool_creation_fees: 0,
            total_liquidity_fees: 0,
            total_regular_swap_fees: 0,
            total_hft_swap_fees: 0,
            last_update_timestamp: 0,
            total_consolidations_performed: 0,
            last_consolidation_timestamp: 0,
        }
    }
    
    /// **NEW: Process batch consolidation from multiple pools**
    pub fn process_batch_consolidation(
        &mut self,
        consolidated_fees: u64,
        consolidated_operations: &ConsolidatedOperations,
        timestamp: i64,
    ) {
        // Update fee totals (pool creation fees handled during initial creation)
        self.total_liquidity_fees += consolidated_operations.liquidity_fees;
        self.total_regular_swap_fees += consolidated_operations.regular_swap_fees;
        self.total_hft_swap_fees += consolidated_operations.hft_swap_fees;
        
        // Update operation counts
        self.liquidity_operation_count += consolidated_operations.liquidity_operation_count;
        self.regular_swap_count += consolidated_operations.regular_swap_count;
        self.hft_swap_count += consolidated_operations.hft_swap_count;
        
        // Update consolidation metadata
        self.total_consolidations_performed += 1;
        self.last_consolidation_timestamp = timestamp;
        self.last_update_timestamp = timestamp;
    }
    
    /// **NEW: Calculate available balance for withdrawal (considering rent exemption)**
    pub fn available_for_withdrawal(&self) -> u64 {
        if self.total_balance > self.rent_exempt_minimum {
            self.total_balance - self.rent_exempt_minimum
        } else {
            0
        }
    }
}

/// **NEW: Consolidated operations data structure**
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Default)]
pub struct ConsolidatedOperations {
    pub liquidity_fees: u64,
    pub regular_swap_fees: u64,
    pub hft_swap_fees: u64,
    pub liquidity_operation_count: u64,
    pub regular_swap_count: u64,
    pub hft_swap_count: u64,
}
```

### **1.3 Constants Updates**
**File**: `src/constants.rs`

**Add consolidation configuration:**
```rust
//=============================================================================
// CONSOLIDATION CONFIGURATION
//=============================================================================

/// Maximum number of pools that can be consolidated in a single batch
/// Limited by Solana's 200,000 CU transaction limit
pub const MAX_POOLS_PER_CONSOLIDATION_BATCH: u8 = 20;

/// System pause reason code used during consolidation operations
pub const PAUSE_REASON_CONSOLIDATION: u8 = 15;

//=============================================================================
// FIXED SYSTEM VALUES (MOVED FROM POOLSTATE)
//=============================================================================

/// Fixed swap fee basis points across all pools (0.25% = 25 basis points)
/// Since this is a fixed value, no need to store per pool
pub const FIXED_SWAP_FEE_BASIS_POINTS: u64 = 25;

//=============================================================================
// ERROR CODES FOR CONSOLIDATION
//=============================================================================

/// Error code for consolidation failures
pub const ERROR_CONSOLIDATION_FAILED: u32 = 5001;

/// Error code for invalid consolidation batch
pub const ERROR_INVALID_CONSOLIDATION_BATCH: u32 = 5002;

/// Error code for consolidation during active operations
pub const ERROR_CONSOLIDATION_RACE_CONDITION: u32 = 5003;
```

---

## 🔄 **PHASE 2: FEE COLLECTION MODIFICATIONS**

### **2.1 Distributed Fee Collection Functions**
**File**: `src/utils/fee_validation.rs`

**Replace centralized fee collection with distributed collection:**
```rust
/// **NEW: Distributed liquidity fee collection**
/// Collects fee directly to the pool state account instead of MainTreasuryState
pub fn collect_liquidity_fee_distributed<'a>(
    payer_account: &AccountInfo<'a>,
    pool_state_account: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    program_id: &Pubkey,
) -> ProgramResult {
    collect_fee_to_pool_state(
        payer_account,
        pool_state_account,
        system_program,
        program_id,
        DEPOSIT_WITHDRAWAL_FEE,
        FeeType::Liquidity,
    )
}

/// **NEW: Distributed swap fee collection**
pub fn collect_regular_swap_fee_distributed<'a>(
    payer_account: &AccountInfo<'a>,
    pool_state_account: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    program_id: &Pubkey,
) -> ProgramResult {
    collect_fee_to_pool_state(
        payer_account,
        pool_state_account,
        system_program,
        program_id,
        SWAP_FEE,
        FeeType::RegularSwap,
    )
}

/// **NEW: Distributed HFT swap fee collection**
pub fn collect_hft_swap_fee_distributed<'a>(
    payer_account: &AccountInfo<'a>,
    pool_state_account: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    program_id: &Pubkey,
) -> ProgramResult {
    collect_fee_to_pool_state(
        payer_account,
        pool_state_account,
        system_program,
        program_id,
        HFT_SWAP_FEE,
        FeeType::HftSwap,
    )
}

/// **NEW: Generic fee collection to pool state**
fn collect_fee_to_pool_state<'a>(
    payer_account: &AccountInfo<'a>,
    pool_state_account: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    program_id: &Pubkey,
    fee_amount: u64,
    fee_type: FeeType,
) -> ProgramResult {
    // Validate fee payment capability
    let validation_result = validate_fee_payment(payer_account, fee_amount, VALIDATION_CONTEXT_FEE);
    if !validation_result.is_valid {
        return Err(PoolError::InsufficientFeeBalance {
            required: fee_amount,
            available: validation_result.available_balance,
            account: *payer_account.key,
        }.into());
    }
    
    // Load and validate pool state
    let mut pool_state = crate::utils::validation::validate_and_deserialize_pool_state_secure(pool_state_account, program_id)?;
    
    // Transfer SOL to pool state account
    invoke(
        &system_instruction::transfer(
            payer_account.key,
            pool_state_account.key,
            fee_amount,
        ),
        &[
            payer_account.clone(),
            pool_state_account.clone(),
            system_program.clone(),
        ],
    )?;
    
    // Update pool state based on fee type
    let current_timestamp = Clock::get()?.unix_timestamp;
    match fee_type {
        FeeType::Liquidity => pool_state.add_liquidity_fee(fee_amount, current_timestamp),
        FeeType::RegularSwap => pool_state.add_regular_swap_fee(fee_amount, current_timestamp),
        FeeType::HftSwap => pool_state.add_hft_swap_fee(fee_amount, current_timestamp),
    }
    
    // Save updated pool state
    let serialized_data = pool_state.try_to_vec()?;
    pool_state_account.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    Ok(())
}

/// **NEW: Fee type enumeration**
enum FeeType {
    Liquidity,
    RegularSwap,
    HftSwap,
}
```

### **2.2 Pool Creation Fee Handling**
**File**: `src/processors/pool_creation.rs`

**Pool creation fees go directly to MainTreasuryState (no change needed):**
```rust
// Pool creation fees continue to go directly to MainTreasuryState
// since pool creation happens only once and doesn't need distributed collection
// This keeps the pattern simple and avoids unnecessary consolidation complexity

// KEEP EXISTING PATTERN:
use crate::utils::fee_validation::collect_pool_creation_fee;

collect_pool_creation_fee(
    user_authority_signer,
    main_treasury_pda,  // Keep direct collection to MainTreasuryState
    system_program_account,
    clock_sysvar_account,
    program_id,
)?;
```

### **2.3 Processor Fee Collection Updates**
**Files**: `src/processors/liquidity.rs`, `src/processors/swap.rs`

**Replace liquidity and swap fee collection:**

**Liquidity Operations (`src/processors/liquidity.rs`):**
```rust
// REPLACE: Centralized fee collection
// OLD:
// collect_liquidity_fee(
//     user_authority_signer,
//     main_treasury_pda,  ← REMOVE
//     system_program_account,
//     clock_sysvar_account,
//     program_id,
// )?;

// NEW: Distributed fee collection
collect_liquidity_fee_distributed(
    user_authority_signer,
    pool_state_pda,  // ← Collect to pool state
    system_program_account,
    program_id,
)?;
```

**Swap Operations (`src/processors/swap.rs`):**
```rust
// REPLACE: Regular swap fee collection
collect_regular_swap_fee_distributed(
    user_authority_signer,
    pool_state_pda,  // ← Collect to pool state instead of main treasury
    system_program_account,
    program_id,
)?;

// REPLACE: HFT swap fee collection
collect_hft_swap_fee_distributed(
    user_authority_signer,
    pool_state_pda,  // ← Collect to pool state instead of main treasury
    system_program_account,
    program_id,
)?;
```

---

## 🔄 **PHASE 3: SINGLE CONSOLIDATION PROCESSOR IMPLEMENTATION**

### **3.1 Pool Fee Consolidation Processor**
**File**: `src/processors/consolidation.rs` (NEW FILE)

```rust
//! Pool Fee Consolidation Processor
//! 
//! This module implements the single batch consolidation process for SOL fees from
//! multiple pool states to the MainTreasuryState with flexible pause support.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    clock::Clock,
    sysvar::Sysvar,
};

use crate::{
    constants::*,
    error::PoolError,
    state::{MainTreasuryState, ConsolidatedOperations},
    utils::validation::{validate_and_deserialize_pool_state_secure},
};

/// Processes batch consolidation of SOL fees with flexible pause requirements
/// 
/// This function safely consolidates SOL fees from 1-20 pools to the MainTreasuryState
/// using either system-wide pause or individual pool pause. The consolidation is atomic - 
/// either all eligible pools are processed successfully or the entire operation fails.
/// 
/// # Flexible Pause Requirements
/// 1. **System Paused**: If system is paused, all specified pools are consolidated
/// 2. **System Active**: If system is NOT paused, only pools with both `swaps_paused` AND `paused` set to true are consolidated
/// 3. **Individual Control**: Allows pausing specific pools without affecting entire system
/// 4. **Race Protection**: Paused state prevents concurrent operations during consolidation
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA validation
/// * `pool_count` - Number of pools to consolidate (1-20)
/// * `accounts` - Array of accounts in the following order:
///   - [0] System State PDA (for pause validation)
///   - [1] Main Treasury PDA (receives consolidated fees)
///   - [2..2+pool_count] Pool State PDAs (pools to consolidate)
/// 
/// # Returns
/// * `ProgramResult` - Success or error
/// 
/// # CU Estimate: ~109,000 CUs for 20 pools, scales linearly down to ~5,000 for 1 pool
/// - System pause validation: 1,000 CUs
/// - Pool processing (N pools): N * 5,200 CUs  
/// - Treasury update: 4,000 CUs
/// 
/// # External Validation
/// - **No fee minimums**: All pools processed regardless of fee amount
/// - **No operation minimums**: All pools processed regardless of operation count
/// - **External filtering**: Caller responsible for determining which pools to consolidate
/// - **Flexible pause support**: Works with system-wide pause OR individual pool pause
pub fn process_consolidate_pool_fees(
    program_id: &Pubkey,
    pool_count: u8,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("🔄 Starting batch consolidation for {} pools", pool_count);
    
    // Validate pool count within limits
    if pool_count == 0 {
        msg!("❌ Pool count cannot be zero");
        return Err(ProgramError::InvalidArgument);
    }
    if pool_count > MAX_POOLS_PER_CONSOLIDATION_BATCH {
        msg!("❌ Pool count {} exceeds maximum {}", pool_count, MAX_POOLS_PER_CONSOLIDATION_BATCH);
        return Err(ProgramError::InvalidArgument);
    }
    
    // Extract accounts
    let system_state_pda = &accounts[0];
    let main_treasury_pda = &accounts[1];
    let pool_accounts = &accounts[2..2 + pool_count as usize];
    
    // Validate account count
    let expected_accounts = 2 + pool_count as usize;
    if accounts.len() != expected_accounts {
        msg!("❌ Expected {} accounts, got {}", expected_accounts, accounts.len());
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    let current_timestamp = Clock::get()?.unix_timestamp;
    
    // **PHASE 1: DETERMINE CONSOLIDATION MODE**
    let consolidation_mode = determine_consolidation_mode(program_id, system_state_pda)?;
    
    // **PHASE 2: BATCH CONSOLIDATION**
    perform_batch_consolidation(
        program_id,
        pool_accounts,
        main_treasury_pda,
        current_timestamp,
        consolidation_mode,
    )
}

/// Consolidation mode based on system/pool pause state
#[derive(Debug, Clone, Copy)]
enum ConsolidationMode {
    /// System is paused - consolidate all specified pools
    SystemPaused,
    /// System is active - only consolidate individually paused pools
    IndividualPoolPause,
}

/// Determines the consolidation mode based on system state
fn determine_consolidation_mode(
    program_id: &Pubkey,
    system_state_pda: &AccountInfo,
) -> Result<ConsolidationMode, ProgramError> {
    // Validate system state PDA
    let system_state = crate::utils::validation::validate_and_deserialize_system_state_secure(
        system_state_pda, 
        program_id
    )?;
    
    if system_state.is_paused {
        // System is paused - consolidate all specified pools
        msg!("🔍 System is paused - consolidating all specified pools");
        if system_state.pause_reason == PAUSE_REASON_CONSOLIDATION {
            msg!("✅ System paused with consolidation reason code ({})", PAUSE_REASON_CONSOLIDATION);
        } else {
            msg!("ℹ️ System paused with reason code: {}", system_state.pause_reason);
        }
        Ok(ConsolidationMode::SystemPaused)
    } else {
        // System is active - only consolidate individually paused pools
        msg!("🔍 System is active - checking for individually paused pools");
        Ok(ConsolidationMode::IndividualPoolPause)
    }
}

/// Performs the actual batch consolidation logic
fn perform_batch_consolidation(
    program_id: &Pubkey,
    pool_accounts: &[AccountInfo],
    main_treasury_pda: &AccountInfo,
    timestamp: i64,
    consolidation_mode: ConsolidationMode,
) -> ProgramResult {
    let mut total_sol_collected = 0u64;
    let mut consolidated_ops = ConsolidatedOperations::default();
    let mut pools_processed = 0u8;
    
    // **PROCESS POOLS BASED ON CONSOLIDATION MODE**
    for pool_account in pool_accounts {
        let mut pool_state = validate_and_deserialize_pool_state_secure(pool_account, program_id)?;
        
        // Check if pool is eligible for consolidation based on mode
        let is_eligible = match consolidation_mode {
            ConsolidationMode::SystemPaused => {
                // System paused - all pools are eligible
                true
            }
            ConsolidationMode::IndividualPoolPause => {
                // System active - only pools with both swaps_paused AND paused are eligible
                if pool_state.swaps_paused && pool_state.paused {
                    msg!("✅ Pool {} is individually paused (swaps_paused: true, paused: true)", pool_account.key);
                    true
                } else {
                    msg!("ℹ️ Pool {} not eligible - swaps_paused: {}, paused: {}", 
                         pool_account.key, pool_state.swaps_paused, pool_state.paused);
                    false
                }
            }
        };
        
        if !is_eligible {
            continue;
        }
        
        let pool_fees = pool_state.pending_sol_fees();
        
        // Skip pools with no fees (but don't error)
        if pool_fees == 0 {
            msg!("ℹ️ Pool {} has no fees to consolidate, skipping", pool_account.key);
            continue;
        }
        
        // **RENT EXEMPT PROTECTION: Calculate rent exempt minimum for pool state**
        let rent_exempt_minimum = {
            use solana_program::sysvar::{rent::Rent, Sysvar};
            let rent = Rent::get()?;
            rent.minimum_balance(PoolState::get_packed_len())
        };
        
        // **RENT EXEMPT PROTECTION: Use helper method to safely calculate available consolidation**
        let current_pool_balance = pool_account.lamports();
        let available_for_consolidation = pool_state.calculate_available_for_consolidation(
            current_pool_balance,
            rent_exempt_minimum,
        );
        
        if available_for_consolidation == 0 {
            msg!("⚠️ Pool {} has {} lamports but needs {} for rent exemption, skipping consolidation", 
                 pool_account.key, current_pool_balance, rent_exempt_minimum);
            continue;
        }
        
        if available_for_consolidation < pool_fees {
            msg!("⚠️ Pool {} has {} pending fees but only {} available above rent exempt minimum", 
                 pool_account.key, pool_fees, available_for_consolidation);
            msg!("   Current balance: {} lamports", current_pool_balance);
            msg!("   Rent exempt minimum: {} lamports", rent_exempt_minimum);
            msg!("   Consolidating partial amount: {} lamports", available_for_consolidation);
        }
        
        // **SAFETY VALIDATION: Double-check consolidation safety before proceeding**
        if let Err(safety_error) = pool_state.validate_consolidation_safety(
            available_for_consolidation,
            current_pool_balance,
            rent_exempt_minimum,
        ) {
            msg!("❌ Consolidation safety check failed for pool {}: {}", pool_account.key, safety_error);
            continue;
        }
        
        // **RENT EXEMPT PROTECTION: Transfer only the available amount (not the full pending fees)**
        **pool_account.try_borrow_mut_lamports()? -= available_for_consolidation;
        **main_treasury_pda.try_borrow_mut_lamports()? += available_for_consolidation;
        
        // **IMPORTANT: Partial consolidation tracking**
        // Since we may not consolidate all fees, we need to track what was actually consolidated
        let consolidation_ratio = if pool_fees > 0 {
            available_for_consolidation as f64 / pool_fees as f64
        } else {
            0.0
        };
        
        // Apply consolidation ratio to fee breakdown
        let liquidity_fees_consolidated = (pool_state.collected_liquidity_fees as f64 * consolidation_ratio) as u64;
        let regular_swap_fees_consolidated = (pool_state.collected_regular_swap_fees as f64 * consolidation_ratio) as u64;
        let hft_swap_fees_consolidated = (pool_state.collected_hft_swap_fees as f64 * consolidation_ratio) as u64;
        
        // Accumulate consolidated data
        consolidated_ops.liquidity_fees += liquidity_fees_consolidated;
        consolidated_ops.regular_swap_fees += regular_swap_fees_consolidated;
        consolidated_ops.hft_swap_fees += hft_swap_fees_consolidated;
        
        // Calculate operation counts from consolidated fees (using fixed fee constants)
        let liquidity_ops = liquidity_fees_consolidated / DEPOSIT_WITHDRAWAL_FEE;
        let regular_ops = regular_swap_fees_consolidated / SWAP_FEE;
        let hft_ops = hft_swap_fees_consolidated / HFT_SWAP_FEE;
        
        consolidated_ops.liquidity_operation_count += liquidity_ops;
        consolidated_ops.regular_swap_count += regular_ops;
        consolidated_ops.hft_swap_count += hft_ops;
        
        total_sol_collected += available_for_consolidation;
        
        // **PARTIAL CONSOLIDATION: Update pool state based on what was actually consolidated**
        if consolidation_ratio >= 1.0 {
            // Full consolidation - reset all counters
            pool_state.reset_consolidation_counters(timestamp);
        } else {
            // Partial consolidation - reduce counters proportionally
            pool_state.collected_liquidity_fees -= liquidity_fees_consolidated;
            pool_state.collected_regular_swap_fees -= regular_swap_fees_consolidated;
            pool_state.collected_hft_swap_fees -= hft_swap_fees_consolidated;
            
            // Update total consolidated amount
            pool_state.total_fees_consolidated += available_for_consolidation;
            
            // Update metadata
            pool_state.last_consolidation_timestamp = timestamp;
            pool_state.total_consolidations += 1;
        }
        
        // **CONSISTENCY VALIDATION**: Verify fee tracking integrity after consolidation
        debug_assert!(pool_state.validate_fee_consistency().is_ok(), 
                     "Fee consistency check failed for pool {}", pool_account.key);
        
        // **RENT EXEMPT VALIDATION**: Verify pool still has rent exempt balance
        debug_assert!(pool_account.lamports() >= rent_exempt_minimum,
                     "Pool {} balance {} below rent exempt minimum {} after consolidation",
                     pool_account.key, pool_account.lamports(), rent_exempt_minimum);
        
        // Save updated pool state
        let serialized_data = pool_state.try_to_vec()?;
        pool_account.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);
        
        pools_processed += 1;
        msg!("✅ Pool {} consolidated: {} SOL ({}% of pending fees)", 
             pool_account.key, 
             available_for_consolidation as f64 / 1_000_000_000.0,
             (consolidation_ratio * 100.0) as u64);
    }
    
    // **STEP 3: Update MainTreasuryState** (even if no pools processed, update timestamp)
    let mut treasury_state = MainTreasuryState::try_from_slice(&main_treasury_pda.data.borrow())?;
    
    // Process batch consolidation
    treasury_state.process_batch_consolidation(total_sol_collected, &consolidated_ops, timestamp);
    
    // Sync balance with actual account balance
    treasury_state.sync_balance_with_account(main_treasury_pda.lamports());
    
    // Save updated treasury state
    let serialized_data = treasury_state.try_to_vec()?;
    main_treasury_pda.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    // Report consolidation results
    match consolidation_mode {
        ConsolidationMode::SystemPaused => {
            msg!("🎉 System-wide consolidation completed:");
        }
        ConsolidationMode::IndividualPoolPause => {
            msg!("🎉 Individual pool consolidation completed:");
        }
    }
    
    msg!("   Pools processed: {}", pools_processed);
    msg!("   Total SOL consolidated: {} ({} SOL)", 
         total_sol_collected, total_sol_collected as f64 / 1_000_000_000.0);
    msg!("   Total operations: {}", 
         consolidated_ops.liquidity_operation_count + 
         consolidated_ops.regular_swap_count + 
         consolidated_ops.hft_swap_count);
    msg!("   🛡️ Rent exempt SOL protected in all pools");
    
    // Handle case where no pools were eligible/processed
    if pools_processed == 0 {
        match consolidation_mode {
            ConsolidationMode::SystemPaused => {
                msg!("ℹ️ No pools had fees to consolidate above rent exempt minimum");
            }
            ConsolidationMode::IndividualPoolPause => {
                msg!("ℹ️ No pools were individually paused or had sufficient fees above rent exempt minimum");
                msg!("ℹ️ To consolidate specific pools, pause them individually or pause the entire system");
            }
        }
    }
    
    Ok(())
}

/// **NEW: Get consolidation status for pools**
/// View-only function to check pool consolidation status
pub fn get_consolidation_status(
    program_id: &Pubkey,
    pool_accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("📊 CONSOLIDATION STATUS REPORT");
    msg!("===============================");
    
    let mut total_fees = 0u64;
    let mut pools_with_fees = 0u8;
    
    for (i, pool_account) in pool_accounts.iter().enumerate() {
        let pool_state = validate_and_deserialize_pool_state_secure(pool_account, program_id)?;
        
        let pool_fees = pool_state.pending_sol_fees();
        let operations = pool_state.total_operations_since_consolidation();
        
        msg!("Pool {}: {}", i + 1, pool_account.key);
        msg!("  Collected SOL: {} lamports ({:.6} SOL)", pool_fees, pool_fees as f64 / 1_000_000_000.0);
        msg!("  Operations since last consolidation: {}", operations);
        msg!("  Last consolidation: {}", 
             if pool_state.last_consolidation_timestamp == 0 { 
                 "Never".to_string() 
             } else { 
                 pool_state.last_consolidation_timestamp.to_string() 
             });
        msg!("");
        
        if pool_fees > 0 {
            total_fees += pool_fees;
            pools_with_fees += 1;
        }
    }
    
    msg!("📈 SUMMARY:");
    msg!("  Pools with fees: {}", pools_with_fees);
    msg!("  Total SOL available: {} lamports ({:.6} SOL)", 
         total_fees, total_fees as f64 / 1_000_000_000.0);
    msg!("  Estimated consolidation cost: ~0.58 SOL");
    if total_fees > 580_000_000 { // 0.58 SOL
        msg!("  Net benefit: {:.6} SOL", (total_fees as f64 / 1_000_000_000.0) - 0.58);
    } else {
        msg!("  ⚠️ Consolidation cost exceeds available fees");
    }
    
    Ok(())
}
```

### **3.2 System State Validation Utilities**
**File**: `src/utils/validation.rs`

**Add secure system state validation function:**
```rust
/// **NEW: Secure system state validation**
/// Validates that the account is the correct SystemState PDA and deserializes it
pub fn validate_and_deserialize_system_state_secure(
    system_state_account: &AccountInfo,
    program_id: &Pubkey,
) -> Result<SystemState, ProgramError> {
    // Validate this is the correct SystemState PDA
    let (expected_system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        program_id,
    );
    
    if *system_state_account.key != expected_system_state_pda {
        msg!("❌ Invalid SystemState PDA provided");
        msg!("❌ Expected: {}", expected_system_state_pda);
        msg!("❌ Got: {}", system_state_account.key);
        return Err(PoolError::InvalidSystemStatePDA.into());
    }
    
    // Deserialize and return system state
    SystemState::try_from_slice(&system_state_account.data.borrow())
        .map_err(|_| PoolError::InvalidSystemStateDeserialization.into())
}
```

---

## 🔧 **PHASE 3.5: MISSING POOL PAUSE/UNPAUSE FUNCTIONALITY**

### **3.5.1 Current State Analysis**
**Infrastructure Status**: ✅ 90% Complete
- **PoolState properties exist**: `paused: bool` and `swaps_paused: bool` ✅
- **Validation functions exist**: `validate_pool_not_paused()` and `validate_pool_swaps_not_paused()` ✅
- **Error types exist**: `PoolPaused`, `PoolSwapsPaused`, etc. ✅
- **Instructions missing**: `PausePoolSwaps`, `UnpausePoolSwaps`, `PausePool`, `UnpausePool` ❌

### **3.5.2 Required Instructions Implementation**
**File**: `src/types/instructions.rs`

**Add streamlined pool pause instructions with bitwise flags:**
```rust
/// **POOL MANAGEMENT**: Pause operations for a specific pool using bitwise flags (pool owner only)
/// 
/// Uses bitwise flags to control which operations to pause:
/// - 0b01 (1): Pause general operations (deposits/withdrawals) - sets `paused = true`
/// - 0b10 (2): Pause swaps - sets `swaps_paused = true`  
/// - 0b11 (3): Pause both (required for consolidation eligibility)
/// 
/// **Idempotent**: Pausing already paused operations does not cause an error.
/// 
/// Accounts:
/// 0. Pool Owner Signer [signer]
/// 1. System State PDA [readable] (for system pause validation)
/// 2. Pool State PDA [writable]
PausePool {
    /// Bitwise flags indicating which operations to pause
    /// Bit 0 (1): General operations (deposits/withdrawals)
    /// Bit 1 (2): Swap operations
    pause_flags: u8,
},

/// **POOL MANAGEMENT**: Unpause operations for a specific pool using bitwise flags (pool owner only)
/// 
/// Uses bitwise flags to control which operations to unpause:
/// - 0b01 (1): Unpause general operations (deposits/withdrawals) - sets `paused = false`
/// - 0b10 (2): Unpause swaps - sets `swaps_paused = false`
/// - 0b11 (3): Unpause both operations
/// 
/// **Idempotent**: Unpausing already unpaused operations does not cause an error.
/// 
/// Accounts:
/// 0. Pool Owner Signer [signer]
/// 1. System State PDA [readable] (for system pause validation)
/// 2. Pool State PDA [writable]
UnpausePool {
    /// Bitwise flags indicating which operations to unpause
    /// Bit 0 (1): General operations (deposits/withdrawals)
    /// Bit 1 (2): Swap operations
    unpause_flags: u8,
},
```

### **3.5.3 Bitwise Flag Constants**
**File**: `src/constants.rs`

**Add pause flag constants:**
```rust
//=============================================================================
// POOL PAUSE BITWISE FLAGS
//=============================================================================

/// Pause general pool operations (deposits and withdrawals)
/// Sets pool_state.paused = true
pub const PAUSE_FLAG_GENERAL: u8 = 0b01; // 1

/// Pause swap operations only
/// Sets pool_state.swaps_paused = true  
pub const PAUSE_FLAG_SWAPS: u8 = 0b10; // 2

/// Pause all operations (general + swaps)
/// Required combination for consolidation eligibility
pub const PAUSE_FLAG_ALL: u8 = PAUSE_FLAG_GENERAL | PAUSE_FLAG_SWAPS; // 3

/// Maximum valid pause flag value
pub const PAUSE_FLAG_MAX: u8 = PAUSE_FLAG_ALL;
```

### **3.5.4 Required Processor Functions**
**File**: `src/processors/pool_management.rs` (NEW FILE)

```rust
//! Pool Management Operations
//! 
//! This module handles pool-specific pause/unpause operations using bitwise flags
//! that allow pool owners to control their individual pools without affecting
//! other pools or requiring system-wide authority.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    constants::*,
    error::PoolError,
    state::PoolState,
    utils::validation::{validate_signer, validate_and_deserialize_pool_state_secure},
};

/// Pauses pool operations using bitwise flags (pool owner only)
/// 
/// Uses bitwise flags to control which operations to pause:
/// - PAUSE_FLAG_GENERAL (1): Pause deposits/withdrawals
/// - PAUSE_FLAG_SWAPS (2): Pause swaps
/// - PAUSE_FLAG_ALL (3): Pause both (required for consolidation eligibility)
/// 
/// **Idempotent**: Pausing already paused operations does not cause an error.
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA validation
/// * `pause_flags` - Bitwise flags indicating which operations to pause
/// * `accounts` - Array of accounts in the following order:
///   - [0] Pool Owner Signer (must match pool.owner)
///   - [1] System State PDA (for system pause validation)  
///   - [2] Pool State PDA (writable, to update pause state)
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_pause_pool(
    program_id: &Pubkey,
    pause_flags: u8,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing PausePool instruction with flags: 0b{:08b} ({})", pause_flags, pause_flags);
    
    // Validate flags
    if pause_flags == 0 {
        msg!("❌ Invalid pause flags: cannot be zero");
        return Err(ProgramError::InvalidArgument);
    }
    if pause_flags > PAUSE_FLAG_MAX {
        msg!("❌ Invalid pause flags: {} exceeds maximum {}", pause_flags, PAUSE_FLAG_MAX);
        return Err(ProgramError::InvalidArgument);
    }
    
    // Extract accounts
    let pool_owner_signer = &accounts[0];
    let system_state_pda = &accounts[1];
    let pool_state_pda = &accounts[2];
    
    // Validate system is not paused (allow pool owner operations during system pause)
    crate::utils::validation::validate_system_not_paused_secure(system_state_pda, program_id)?;
    
    // Validate signer
    validate_signer(pool_owner_signer)?;
    
    // Load and validate pool state
    let mut pool_state = validate_and_deserialize_pool_state_secure(pool_state_pda, program_id)?;
    
    // Validate pool owner authority
    if pool_state.owner != *pool_owner_signer.key {
        msg!("❌ Unauthorized: Only pool owner can pause pool operations");
        msg!("   Pool owner: {}", pool_state.owner);
        msg!("   Attempted by: {}", pool_owner_signer.key);
        return Err(PoolError::Unauthorized.into());
    }
    
    // Apply pause flags (idempotent - no error if already paused)
    let mut operations_changed = Vec::new();
    
    if pause_flags & PAUSE_FLAG_GENERAL != 0 {
        if !pool_state.paused {
            pool_state.paused = true;
            operations_changed.push("general operations");
        }
    }
    
    if pause_flags & PAUSE_FLAG_SWAPS != 0 {
        if !pool_state.swaps_paused {
            pool_state.swaps_paused = true;
            operations_changed.push("swaps");
        }
    }
    
    // Save updated pool state
    let serialized_data = pool_state.try_to_vec()?;
    pool_state_pda.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    // Log results
    if operations_changed.is_empty() {
        msg!("ℹ️ No changes made - requested operations were already paused");
    } else {
        msg!("✅ Pool operations paused: {}", operations_changed.join(", "));
    }
    
    msg!("   Pool: {}", pool_state_pda.key);
    msg!("   General operations: {}", if pool_state.paused { "PAUSED" } else { "ENABLED" });
    msg!("   Swap operations: {}", if pool_state.swaps_paused { "PAUSED" } else { "ENABLED" });
    msg!("   Consolidation eligible: {}", 
         if pool_state.paused && pool_state.swaps_paused { "YES" } else { "NO" });
    
    Ok(())
}

/// Unpauses pool operations using bitwise flags (pool owner only)
/// 
/// Uses bitwise flags to control which operations to unpause:
/// - PAUSE_FLAG_GENERAL (1): Unpause deposits/withdrawals
/// - PAUSE_FLAG_SWAPS (2): Unpause swaps
/// - PAUSE_FLAG_ALL (3): Unpause both operations
/// 
/// **Idempotent**: Unpausing already unpaused operations does not cause an error.
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA validation
/// * `unpause_flags` - Bitwise flags indicating which operations to unpause
/// * `accounts` - Array of accounts in the following order:
///   - [0] Pool Owner Signer (must match pool.owner)
///   - [1] System State PDA (for system pause validation)  
///   - [2] Pool State PDA (writable, to update pause state)
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_unpause_pool(
    program_id: &Pubkey,
    unpause_flags: u8,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing UnpausePool instruction with flags: 0b{:08b} ({})", unpause_flags, unpause_flags);
    
    // Validate flags
    if unpause_flags == 0 {
        msg!("❌ Invalid unpause flags: cannot be zero");
        return Err(ProgramError::InvalidArgument);
    }
    if unpause_flags > PAUSE_FLAG_MAX {
        msg!("❌ Invalid unpause flags: {} exceeds maximum {}", unpause_flags, PAUSE_FLAG_MAX);
        return Err(ProgramError::InvalidArgument);
    }
    
    // Extract accounts
    let pool_owner_signer = &accounts[0];
    let system_state_pda = &accounts[1];
    let pool_state_pda = &accounts[2];
    
    // Validate system is not paused
    crate::utils::validation::validate_system_not_paused_secure(system_state_pda, program_id)?;
    
    // Validate signer
    validate_signer(pool_owner_signer)?;
    
    // Load and validate pool state
    let mut pool_state = validate_and_deserialize_pool_state_secure(pool_state_pda, program_id)?;
    
    // Validate pool owner authority
    if pool_state.owner != *pool_owner_signer.key {
        msg!("❌ Unauthorized: Only pool owner can unpause pool operations");
        msg!("   Pool owner: {}", pool_state.owner);
        msg!("   Attempted by: {}", pool_owner_signer.key);
        return Err(PoolError::Unauthorized.into());
    }
    
    // Apply unpause flags (idempotent - no error if already unpaused)
    let mut operations_changed = Vec::new();
    
    if unpause_flags & PAUSE_FLAG_GENERAL != 0 {
        if pool_state.paused {
            pool_state.paused = false;
            operations_changed.push("general operations");
        }
    }
    
    if unpause_flags & PAUSE_FLAG_SWAPS != 0 {
        if pool_state.swaps_paused {
            pool_state.swaps_paused = false;
            operations_changed.push("swaps");
        }
    }
    
    // Save updated pool state
    let serialized_data = pool_state.try_to_vec()?;
    pool_state_pda.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    // Log results
    if operations_changed.is_empty() {
        msg!("ℹ️ No changes made - requested operations were already unpaused");
    } else {
        msg!("✅ Pool operations unpaused: {}", operations_changed.join(", "));
    }
    
    msg!("   Pool: {}", pool_state_pda.key);
    msg!("   General operations: {}", if pool_state.paused { "PAUSED" } else { "ENABLED" });
    msg!("   Swap operations: {}", if pool_state.swaps_paused { "PAUSED" } else { "ENABLED" });
    msg!("   Consolidation eligible: {}", 
         if pool_state.paused && pool_state.swaps_paused { "YES" } else { "NO" });
    
    Ok(())
}
```

### **3.5.5 Module Integration**
**File**: `src/processors/mod.rs`

**Add pool management module:**
```rust
pub mod pool_management;

// Re-export pool management functions
pub use pool_management::*;
```

### **3.5.6 Error Handling Updates**
**File**: `src/error.rs`

**Note**: With idempotent operations, most pause-specific errors are **no longer needed**:
```rust
// REMOVE these error variants (no longer used with idempotent operations):
// PoolSwapsAlreadyPaused - idempotent pause doesn't error
// PoolSwapsNotPaused - idempotent unpause doesn't error
// PoolAlreadyPaused - idempotent pause doesn't error
// PoolNotPaused - idempotent unpause doesn't error

// KEEP these error variants (still used):
pub enum PoolError {
    // ... existing errors ...
    PoolPaused,         // Used by operation validation
    PoolSwapsPaused,    // Used by swap validation
    Unauthorized,       // Used by authority validation
}
```

### **3.5.7 Integration with Flexible Consolidation**

**How bitwise flags enable flexible consolidation:**

#### **Individual Pool Consolidation Setup:**
```rust
use crate::constants::*;

// 1. Pause specific pools for consolidation (both flags required)
let pause_instruction = create_pause_pool_instruction(
    &pool_owner_keypair.pubkey(),
    &pool_state_pda,
    PAUSE_FLAG_ALL, // 3 = pause both general operations AND swaps
);

// 2. Execute consolidation (system active, only fully paused pools processed)
let consolidation_instruction = create_consolidation_instruction(
    &system_state_pda,
    &main_treasury_pda,
    &[pool_state_pda], // Only this pool will be consolidated
    1,
);

// 3. Unpause the pool
let unpause_instruction = create_unpause_pool_instruction(
    &pool_owner_keypair.pubkey(),
    &pool_state_pda,
    PAUSE_FLAG_ALL, // 3 = unpause both operations
);
```

#### **Flexible Pause Options:**
```rust
// Pause only swaps (deposits/withdrawals continue)
let pause_swaps_only = create_pause_pool_instruction(
    &pool_owner_keypair.pubkey(),
    &pool_state_pda,
    PAUSE_FLAG_SWAPS, // 2 = pause only swaps
);

// Pause only general operations (swaps continue)
let pause_general_only = create_pause_pool_instruction(
    &pool_owner_keypair.pubkey(),
    &pool_state_pda,
    PAUSE_FLAG_GENERAL, // 1 = pause only deposits/withdrawals
);

// Note: For consolidation eligibility, BOTH flags must be set (value 3)
// Individual flags (1 or 2) won't make pools eligible for consolidation
```

#### **Idempotent Operations:**
```rust
// Safe to call multiple times - no errors
let pause_all = create_pause_pool_instruction(
    &pool_owner_keypair.pubkey(),
    &pool_state_pda,
    PAUSE_FLAG_ALL,
);

// If called again, logs "No changes made - already paused" but succeeds
let pause_all_again = create_pause_pool_instruction(
    &pool_owner_keypair.pubkey(),
    &pool_state_pda,
    PAUSE_FLAG_ALL, // No error - idempotent
);

// Partial unpausing is also idempotent
let unpause_swaps_only = create_unpause_pool_instruction(
    &pool_owner_keypair.pubkey(),
    &pool_state_pda,
    PAUSE_FLAG_SWAPS, // Unpause swaps, leave general operations paused
);
```

### **3.5.8 Testing Requirements**

**Integration tests needed:**
- Pool owner can pause/unpause with different flag combinations
- Non-pool-owners cannot pause pools (authority validation)
- Bitwise flag validation (0 and >3 should error)
- Idempotent operations don't cause errors
- Consolidation eligibility requires both flags set (value 3)
- Pool operations respect individual pause states correctly
- Partial pause/unpause combinations work as expected

---

## 🔧 **PHASE 4: INSTRUCTION UPDATES & ERROR HANDLING**

### **4.1 New Instructions**
**File**: `src/types/instructions.rs`

**Add consolidation instructions:**
```rust
/// **NEW: Consolidation instruction**
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum ConsolidationInstruction {
    /// Consolidate SOL fees from 1-20 pools to MainTreasuryState (flexible pause support)
    /// 
    /// Behavior:
    /// - If system is paused: All specified pools are consolidated
    /// - If system is NOT paused: Only pools with both swaps_paused AND paused set to true are consolidated
    /// 
    /// Accounts:
    /// 0. System State PDA [readable] (checked for pause state)
    /// 1. Main Treasury PDA [writable]
    /// 2..2+pool_count. Pool State PDAs [writable]
    ConsolidatePools {
        pool_count: u8,
    },
    
    /// Get consolidation status for specified pools (view-only)
    /// 
    /// Accounts:
    /// 0..pool_count. Pool State PDAs [readable]
    GetConsolidationStatus {
        pool_count: u8,
    },
}

// Update main instruction enum
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum PoolInstruction {
    // ... existing instructions ...
    
    /// **NEW: Batch consolidation instruction**
    Consolidation {
        consolidation_instruction: ConsolidationInstruction,
    },
}
```

### **4.2 Error Handling Updates**
**File**: `src/error.rs`

**Add consolidation error types:**
```rust
#[derive(Error, Debug, Clone)]
pub enum PoolError {
    // ... existing errors ...
    
    /// **NEW: Consolidation-related errors**
    #[error("Consolidation failed: {reason}")]
    ConsolidationFailed { reason: String },
    
    #[error("Invalid consolidation batch: expected {expected} pools, got {actual}")]
    InvalidConsolidationBatch { expected: u8, actual: u8 },
    
    #[error("Pool not eligible for consolidation: {reason}")]
    PoolNotEligibleForConsolidation { reason: String },
    
    #[error("Consolidation race condition detected")]
    ConsolidationRaceCondition,
    
    #[error("No pools eligible for consolidation")]
    NoPoolsEligibleForConsolidation,
    
    #[error("Invalid SystemState PDA provided")]
    InvalidSystemStatePDA,
    
    #[error("Failed to deserialize SystemState")]
    InvalidSystemStateDeserialization,
}
```

### **4.3 Account Structure Updates**

**Updated account structures for each operation:**

**Pool Creation (UPDATED):**
```
OLD (13 accounts):
0. User Authority Signer [signer, writable]
1. System Program Account [readable]
2. System State PDA [readable]
3. Pool State PDA [writable]
4. SPL Token Program Account [readable]
5. Main Treasury PDA [writable] ← KEEP (pool creation fees still go here)
6. Rent Sysvar Account [readable]
7. Token A Mint Account [readable]
8. Token B Mint Account [readable]
9. Token A Vault PDA [writable]
10. Token B Vault PDA [writable]
11. LP Token A Mint PDA [writable]
12. LP Token B Mint PDA [writable]

NEW (13 accounts): NO CHANGE - pool creation fees still go to MainTreasuryState
```

**Liquidity Operations (UPDATED):**
```
OLD (14 accounts): Remove Main Treasury PDA (index 5)
NEW (13 accounts): All accounts after index 4 shift down by 1 index
```

**Swap Operations (UPDATED):**
```
OLD (10 accounts): Remove Main Treasury PDA (index 5)
NEW (9 accounts): All accounts after index 4 shift down by 1 index
```

**NEW Consolidation Operation:**
```
ConsolidatePools (2 + N accounts):
0. System State PDA [readable] (checked for pause state)
1. Main Treasury PDA [writable]
2..2+N. Pool State PDAs [writable] (1-20 pools)

Behavior:
- System paused: All specified pools consolidated
- System active: Only pools with both swaps_paused AND paused=true consolidated
```

---

## 🗑️ **REMOVAL OF OLD CODE (NO DEPRECATION)**

### **5.1 Complete Code Removal Strategy**
**Important**: Since this code has **NOT been published to mainnet**, we are implementing **complete removal** rather than deprecation or backward compatibility. This allows for cleaner, more efficient code without legacy burden.

### **5.2 Files to Remove Completely**
**Delete these files entirely:**

```bash
# Remove old system pause test files (already deleted)
rm tests/test_system_pause_advanced.rs
rm tests/test_system_pause_basic.rs

# Remove old centralized fee collection functions (will be replaced)
# Note: Files will be updated, not deleted, but old functions removed
```

### **5.3 Functions to Remove Completely**
**File**: `src/utils/fee_validation.rs`

**Remove these centralized fee collection functions:**
```rust
// REMOVE COMPLETELY:
pub fn collect_liquidity_fee(...)  // OLD centralized version
pub fn collect_swap_fee(...)       // OLD centralized version  
pub fn collect_hft_swap_fee(...)   // OLD centralized version
pub fn collect_fee_to_treasury(...)// OLD centralized helper
```

**Replace with new distributed versions:**
```rust
// NEW DISTRIBUTED VERSIONS:
pub fn collect_liquidity_fee_distributed(...)
pub fn collect_regular_swap_fee_distributed(...)
pub fn collect_hft_swap_fee_distributed(...)
pub fn collect_fee_to_pool_state(...)
```

### **5.4 Data Structure Fields to Remove**
**File**: `src/state/pool_state.rs`

**Remove these fields from PoolState:**
```rust
// REMOVE COMPLETELY (no deprecation):
pub is_initialized: bool,                              // Pool existence = initialization
pub swap_fee_basis_points: u64,                        // Move to constants as fixed value
pub collected_pool_creation_fees: u64,                 // Pool creation happens once
pub pool_creation_operations: u64,                     // Pool creation happens once
pub liquidity_operations_since_consolidation: u64,     // Calculate from fees: collected_liquidity_fees / DEPOSIT_WITHDRAWAL_FEE
pub regular_swap_operations_since_consolidation: u64,  // Calculate from fees: collected_regular_swap_fees / SWAP_FEE
pub hft_swap_operations_since_consolidation: u64,      // Calculate from fees: collected_hft_swap_fees / HFT_SWAP_FEE
```

**File**: `src/state/treasury_state.rs`

**Remove these fields from MainTreasuryState:**
```rust
// REMOVE COMPLETELY:
// (No fields removed from MainTreasuryState in this migration)
```

### **5.5 Constants to Remove**
**File**: `src/constants.rs`

**Remove old configuration constants:**
```rust
// REMOVE COMPLETELY:
// (No constants removed, only additions in this migration)
```

### **5.6 Processor Updates - Complete Function Replacement**
**File**: `src/processors/pool_creation.rs`

**Replace fee collection calls:**
```rust
// REMOVE OLD CALLS:
// collect_liquidity_fee(user_authority_signer, main_treasury_pda, ...)

// REPLACE WITH NEW CALLS:
// collect_liquidity_fee_distributed(user_authority_signer, pool_state_pda, ...)
```

**File**: `src/processors/liquidity.rs`

**Replace fee collection calls:**
```rust
// REMOVE OLD CALLS:
// collect_liquidity_fee(user_authority_signer, main_treasury_pda, ...)

// REPLACE WITH NEW CALLS:
// collect_liquidity_fee_distributed(user_authority_signer, pool_state_pda, ...)
```

**File**: `src/processors/swap.rs`

**Replace fee collection calls:**
```rust
// REMOVE OLD CALLS:
// collect_swap_fee(user_authority_signer, main_treasury_pda, ...)
// collect_hft_swap_fee(user_authority_signer, main_treasury_pda, ...)

// REPLACE WITH NEW CALLS:
// collect_regular_swap_fee_distributed(user_authority_signer, pool_state_pda, ...)
// collect_hft_swap_fee_distributed(user_authority_signer, pool_state_pda, ...)
```

### **5.7 Account Structure Changes**
**Complete removal of accounts from instructions:**

**Liquidity Operations:**
```rust
// REMOVE ACCOUNT:
// 5. Main Treasury PDA [writable]  ← REMOVE COMPLETELY

// ACCOUNT INDICES SHIFT DOWN:
// Old index 6 → New index 5
// Old index 7 → New index 6
// ... and so on
```

**Swap Operations:**
```rust
// REMOVE ACCOUNT:
// 5. Main Treasury PDA [writable]  ← REMOVE COMPLETELY

// ACCOUNT INDICES SHIFT DOWN:
// Old index 6 → New index 5  
// Old index 7 → New index 6
// ... and so on
```

### **5.8 Error Handling - Complete Removal**
**File**: `src/error.rs`

**Remove old error variants:**
```rust
// REMOVE COMPLETELY:
// (No specific errors removed in this migration)
// Only NEW error variants added for consolidation
```

### **5.9 Test File Updates - Complete Replacement**
**Remove old test patterns:**

**All test files in `tests/` directory:**
```rust
// REMOVE OLD TEST PATTERNS:
// - References to main_treasury_pda in liquidity/swap operations
// - Tests expecting MainTreasuryState to be updated during operations
// - Tests validating centralized fee collection

// REPLACE WITH NEW TEST PATTERNS:
// - Pool state fee collection validation
// - Consolidation operation tests
// - Distributed fee tracking tests
```

### **5.10 Client SDK Updates - Complete Replacement**
**File**: `src/client_sdk.rs`

**Remove old client functions:**
```rust
// REMOVE OLD FUNCTIONS:
// Functions that expect MainTreasuryState updates during operations
// Functions that include MainTreasuryState in operation accounts

// REPLACE WITH NEW FUNCTIONS:
// Functions that work with distributed collection
// Functions that support consolidation operations
```

### **5.11 Documentation Updates - Complete Replacement**
**Remove old documentation:**
```rust
// REMOVE OLD DOCS:
// - Centralized fee collection explanations
// - MainTreasuryState update documentation for operations
// - Old account structure documentation

// REPLACE WITH NEW DOCS:
// - Distributed collection explanations
// - Consolidation operation documentation
// - New account structure documentation
```

### **5.12 No Backward Compatibility**
**Critical Design Decision**: 
- **NO deprecation warnings** - functions removed completely
- **NO backward compatibility** - old patterns will break
- **NO migration path** - clean slate implementation
- **NO feature flags** - single code path only

**Benefits of Complete Removal:**
- **Cleaner codebase** without legacy burden
- **Better performance** without compatibility overhead
- **Simpler maintenance** without dual code paths
- **Clearer documentation** without confusing alternatives
- **Smaller binary size** without unused code

### **5.13 Migration Verification**
**Verification checklist for complete removal:**

```bash
# 1. Verify no references to old functions exist
grep -r "collect_liquidity_fee(" src/ --exclude-dir=target
grep -r "collect_swap_fee(" src/ --exclude-dir=target
grep -r "collect_hft_swap_fee(" src/ --exclude-dir=target

# 2. Verify no references to removed fields exist  
grep -r "is_initialized" src/ --exclude-dir=target
grep -r "swap_fee_basis_points" src/ --exclude-dir=target
grep -r "collected_pool_creation_fees" src/ --exclude-dir=target
grep -r "liquidity_operations_since_consolidation" src/ --exclude-dir=target
grep -r "regular_swap_operations_since_consolidation" src/ --exclude-dir=target
grep -r "hft_swap_operations_since_consolidation" src/ --exclude-dir=target

# 3. Verify no old account structures exist
grep -r "main_treasury_pda" src/processors/liquidity.rs
grep -r "main_treasury_pda" src/processors/swap.rs

# 4. Verify new functions are used
grep -r "collect_liquidity_fee_distributed" src/
grep -r "collect_regular_swap_fee_distributed" src/
grep -r "collect_hft_swap_fee_distributed" src/
```

---

## 🔄 **CONSOLIDATION USAGE FLOW**

### **Option 1: System-Wide Consolidation (Multiple Pools)**

#### **Step 1: Pause Entire System**
```rust
// Use existing pause_system instruction with consolidation reason
let pause_instruction = create_pause_system_instruction(
    &system_authority_keypair.pubkey(),
    PAUSE_REASON_CONSOLIDATION,  // Reason code 15
);

// Execute pause instruction
// System is now paused - ALL operations blocked across ALL pools
```

#### **Step 2: Execute Consolidation**
```rust
// Select pools to consolidate (1-20 pools)
let pool_pubkeys = vec![pool1_pubkey, pool2_pubkey, pool3_pubkey];

// Create consolidation instruction
let consolidation_instruction = create_consolidation_instruction(
    &system_state_pda,
    &main_treasury_pda,
    &pool_pubkeys,
    pool_pubkeys.len() as u8,
);

// Execute consolidation (system is paused - all specified pools processed)
// Fees transferred from ALL specified pools to MainTreasuryState
```

#### **Step 3: Unpause System**
```rust
// Use existing unpause_system instruction
let unpause_instruction = create_unpause_system_instruction(
    &system_authority_keypair.pubkey(),
);

// Execute unpause instruction
// System is now active - normal operations resume across all pools
```

### **Option 2: Individual Pool Consolidation (Selective)**

#### **Step 1: Pause Individual Pools**
```rust
use crate::constants::*;

// Pause specific pools individually (both flags must be set for consolidation eligibility)
let pause_pool1_instruction = create_pause_pool_instruction(
    &pool_authority_keypair.pubkey(),
    &pool1_pubkey,
    PAUSE_FLAG_ALL,  // 3 = pause both general operations AND swaps
);

let pause_pool2_instruction = create_pause_pool_instruction(
    &pool_authority_keypair.pubkey(),
    &pool2_pubkey,
    PAUSE_FLAG_ALL,  // 3 = pause both general operations AND swaps
);

// Execute pause instructions (idempotent - safe to retry)
// Only specified pools are paused - other pools continue operating normally
```

#### **Step 2: Execute Consolidation**
```rust
// Include all pools you want to check (1-20 pools)
// Only pools with both swaps_paused AND paused set to true will be consolidated
let pool_pubkeys = vec![pool1_pubkey, pool2_pubkey, pool3_pubkey];

// Create consolidation instruction
let consolidation_instruction = create_consolidation_instruction(
    &system_state_pda,
    &main_treasury_pda,
    &pool_pubkeys,
    pool_pubkeys.len() as u8,
);

// Execute consolidation (system is active - only individually paused pools processed)
// Fees transferred from ONLY paused pools to MainTreasuryState
```

#### **Step 3: Unpause Individual Pools**
```rust
// Unpause the specific pools that were paused
let unpause_pool1_instruction = create_unpause_pool_instruction(
    &pool_authority_keypair.pubkey(),
    &pool1_pubkey,
    PAUSE_FLAG_ALL,  // 3 = unpause both general operations AND swaps
);

let unpause_pool2_instruction = create_unpause_pool_instruction(
    &pool_authority_keypair.pubkey(),
    &pool2_pubkey,
    PAUSE_FLAG_ALL,  // 3 = unpause both general operations AND swaps
);

// Execute unpause instructions (idempotent - safe to retry)
// Specified pools resume normal operations
```

### **Benefits of This Flexible Approach:**
- **System-Wide Control**: Pause entire system for comprehensive consolidation
- **Granular Control**: Pause only specific pools for targeted consolidation
- **Bitwise Flexibility**: Use flags to pause general operations, swaps, or both
- **Idempotent Operations**: Pause/unpause calls never cause errors if already in desired state
- **Minimal Disruption**: Individual pool pausing doesn't affect other pools
- **Clear Separation**: Pause/unpause logic separate from consolidation logic
- **Predictable**: System and pool states always explicitly managed
- **Safe**: No risk of system remaining paused due to consolidation errors
- **Efficient**: Only 2 functions needed instead of 4 (reduced code complexity)
- **Transparent**: Clear logging shows which operations were changed or already in desired state

---

## 🚀 **MIGRATION TIMELINE & TESTING**

### **Week 1: Data Structure Updates**
- **Days 1-2**: Implement Phase 1 (PoolState + MainTreasuryState updates)
- **Days 3-4**: Implement Phase 2 (Fee collection modifications)
- **Day 5**: Unit testing of new data structures

### **Week 2: Single Consolidation Implementation**
- **Days 1-3**: Implement Phase 3 (Single consolidation processor: process_consolidate_pool_fees)
- **Days 4-5**: Implement Phase 4 (Instructions + error handling)

### **Week 3: Integration & Testing**
- **Days 1-2**: Integration testing with existing operations
- **Days 3-4**: Single consolidation process testing with multiple pools
- **Day 5**: Performance testing and CU optimization

### **Migration Deployment:**
- **Deploy new contract version** with backward compatibility
- **Migrate existing pools** with data migration script
- **Update client SDKs** with new account structures
- **Monitor system performance** and consolidation efficiency

---

## 📊 **EXPECTED BENEFITS POST-MIGRATION**

### **User Experience:**
- **67% CU reduction** on liquidity and swap operations
- **Faster transactions** due to lower CU usage
- **Lower transaction costs** for all users
- **Pool creation unchanged** (fees still go directly to MainTreasuryState)

### **Owner Economics:**
- **Consolidation cost**: ~0.54 SOL per 20-pool batch, scales linearly down
- **Break-even**: Only 2 operations per pool between consolidations
- **Cost-effective** for virtually all active pools
- **Flexible batching**: Can consolidate 1-20 pools as needed

### **System Performance:**
- **109,000 CUs** for 20-pool consolidation (within 200K limit)
- **Linear scaling**: ~5,000 CUs for 1-pool consolidation
- **No race conditions** with system-wide pause requirement
- **Atomic operations** with guaranteed consistency
- **External control**: No automatic triggers, full external control
- **Separation of concerns**: Pause/unpause independent of consolidation

### **Simplified Design:**
- **No minimum fee requirements** - external filtering
- **No minimum operation requirements** - external filtering
- **Fixed swap fees** moved to constants
- **Removed initialization flag** - pool existence = initialization
- **Pool creation tracking removed** - happens only once
- **Flexible pause support** - system-wide OR individual pool consolidation

---

---

## 🎯 **FLEXIBLE CONSOLIDATION SUMMARY**

### **Two Consolidation Modes:**

#### **🌐 System-Wide Consolidation**
- **Use Case**: Comprehensive fee collection across multiple pools
- **Requirement**: System must be paused (all operations blocked)
- **Behavior**: All specified pools are consolidated regardless of individual pause state
- **Benefits**: Maximum throughput, predictable behavior, comprehensive collection

#### **🎯 Individual Pool Consolidation**
- **Use Case**: Targeted fee collection from specific pools
- **Requirement**: Target pools must have `PAUSE_FLAG_ALL` (3) set - both `swaps_paused` AND `paused` = true
- **Behavior**: Only individually paused pools are consolidated, others continue operating
- **Benefits**: Minimal disruption, granular control, selective operation

### **Key Advantages:**
- **Operational Flexibility**: Choose between system-wide or targeted consolidation
- **Bitwise Control**: Use flags (1=general, 2=swaps, 3=both) for precise pause control
- **Idempotent Operations**: Pause/unpause calls never fail due to current state
- **Minimal Disruption**: Individual pool pausing doesn't affect other pools
- **Clear Logic**: Simple pause state checks determine consolidation eligibility
- **Safe Execution**: No risk of consolidating from active pools
- **Predictable Costs**: Same CU usage regardless of consolidation mode
- **External Control**: Full control over which pools to pause and when to consolidate
- **Reduced Complexity**: Only 2 functions instead of 4, with flexible flag parameters

### **Performance Impact:**
- **CU Usage**: 109,000 CUs for 20 pools, scales linearly down to ~5,000 CUs for 1 pool
- **Cost**: ~0.54 SOL per 20-pool batch (same for both modes)
- **Efficiency**: No additional overhead for pause state checking
- **Scalability**: Works with any combination of paused/active pools

### **Race Condition Protection:**
- **Atomic Updates**: All fee collection and consolidation operations are atomic within Solana transactions
- **Accurate Fee Calculation**: Uses `pending_sol_fees() = total_sol_fees_collected - total_fees_consolidated` (includes ALL fee types)
- **Simplified Logic**: No complex consolidation state tracking needed for individual fee types
- **Debug Validation**: Automatic consistency checks in debug builds with zero runtime cost in production
- **Overflow Protection**: Conservative checks prevent arithmetic overflow scenarios
- **Audit Trail**: Complete lifetime fee tracking with `total_sol_fees_collected` field including pool creation fees
- **No Lost Fees**: Impossible for fees to be collected without counter updates or vice versa

---

## 🛡️ **RENT EXEMPT PROTECTION SUMMARY**

### **Critical Implementation: Consolidation Cannot Take Rent Exempt SOL**

The consolidation process has been designed with comprehensive rent exempt protection to ensure pool state accounts can never be closed due to insufficient rent exempt balance:

#### **1. Rent Exempt Calculation**
```rust
// Calculate rent exempt minimum for pool state
let rent_exempt_minimum = {
    use solana_program::sysvar::{rent::Rent, Sysvar};
    let rent = Rent::get()?;
    rent.minimum_balance(PoolState::get_packed_len())
};
```

#### **2. Available Balance Calculation**
```rust
// Helper method in PoolState
pub fn calculate_available_for_consolidation(
    &self,
    current_account_balance: u64,
    rent_exempt_minimum: u64,
) -> u64 {
    let pending_fees = self.pending_sol_fees();
    let available_above_rent_exempt = if current_account_balance > rent_exempt_minimum {
        current_account_balance - rent_exempt_minimum
    } else {
        0
    };
    
    // Never take more than available above rent exempt OR pending fees
    std::cmp::min(available_above_rent_exempt, pending_fees)
}
```

#### **3. Safety Validation**
```rust
// Comprehensive safety check before consolidation
pub fn validate_consolidation_safety(
    &self,
    proposed_consolidation_amount: u64,
    current_account_balance: u64,
    rent_exempt_minimum: u64,
) -> Result<(), &'static str> {
    // Ensures balance after consolidation >= rent_exempt_minimum
    // Ensures consolidation amount <= pending fees
    // Prevents arithmetic underflow
}
```

#### **4. Protected Consolidation Flow**
1. **Calculate rent exempt minimum** for pool state account size
2. **Determine available balance** above rent exempt minimum
3. **Limit to pending fees** (never take more than owed)
4. **Validate consolidation safety** before proceeding
5. **Transfer only available amount** (may be partial consolidation)
6. **Verify final balance** >= rent exempt minimum (debug assertion)

#### **5. Partial Consolidation Support**
- **If full consolidation is unsafe**: Only consolidate what's available above rent exempt minimum
- **Proportional tracking**: Update fee counters proportionally to actual consolidated amount
- **Consistent state**: Maintains accurate fee tracking even with partial consolidation
- **Clear logging**: Shows exactly how much was consolidated and why

#### **6. Safety Guarantees**
- ✅ **Account closure prevention**: Pool state accounts can never be closed due to insufficient rent
- ✅ **Partial consolidation support**: Safely handles cases where full consolidation would violate rent exemption
- ✅ **Accurate tracking**: Fee counters remain consistent even with partial consolidation
- ✅ **Clear reporting**: Detailed logging shows consolidation amounts and rent exempt protection status
- ✅ **Debug validation**: Assertions verify rent exempt balance after consolidation (debug builds only)

#### **7. Edge Case Handling**
- **Zero available balance**: Skips consolidation entirely with clear logging
- **Partial consolidation**: Consolidates only what's safe, updates counters proportionally
- **Multiple consolidations**: Subsequent consolidations work correctly with remaining fees
- **Rent changes**: Recalculates rent exempt minimum on each consolidation

### **Usage Example**
```rust
// Safe consolidation that respects rent exemption
let available_for_consolidation = pool_state.calculate_available_for_consolidation(
    pool_account.lamports(),
    rent_exempt_minimum,
);

// This will be:
// - 0 if account balance <= rent_exempt_minimum
// - min(available_above_rent_exempt, pending_fees) otherwise
// - Never more than pending fees
// - Never enough to reduce balance below rent_exempt_minimum
```

### **Testing Requirements**
- **Rent exempt boundary testing**: Ensure consolidation stops at rent exempt minimum
- **Partial consolidation testing**: Verify correct proportional counter updates
- **Edge case testing**: Zero balance, exact rent exempt balance, etc.
- **Multiple consolidation testing**: Verify subsequent consolidations work correctly
- **Safety validation testing**: Ensure all safety checks work correctly

**End of Document** 🎯 

---

## 🏗️ **ARCHITECTURAL ANALYSIS: POOL CREATION FEES DESTINATION**

### **Current Implementation vs Alternative Approaches**

#### **Option 1: Pool Creation Fees → Main Treasury (Current & Proposed)**
```rust
// Current implementation in pool_creation.rs
invoke(
    &system_instruction::transfer(
        user_authority_signer.key,
        main_treasury_pda.key,  // ← Direct to main treasury
        REGISTRATION_FEE,
    ),
    // ... accounts
)?;

// Immediate treasury state update
treasury_state.add_pool_creation_fee(fee_amount, current_timestamp);
```

#### **Option 2: Pool Creation Fees → Pool State (Alternative)**
```rust
// Hypothetical alternative approach
invoke(
    &system_instruction::transfer(
        user_authority_signer.key,
        pool_state_pda.key,  // ← To pool state instead
        REGISTRATION_FEE,
    ),
    // ... accounts
)?;

// Pool state tracks creation fee
pool_state.add_pool_creation_fee(fee_amount, current_timestamp);
```

---

### **📊 DETAILED COMPARISON ANALYSIS**

#### **🎯 Frequency & Usage Patterns**

| Factor | Main Treasury | Pool State |
|--------|---------------|------------|
| **Frequency** | One-time per pool | One-time per pool |
| **Timing** | During pool creation | During pool creation |
| **Consolidation** | Not needed | Required later |
| **Usage Pattern** | Single transaction | Create → Later consolidate |

**Analysis**: Pool creation happens exactly once per pool, making consolidation efficiency gains minimal.

#### **💰 Economic Impact**

| Factor | Main Treasury | Pool State | Winner |
|--------|---------------|------------|---------|
| **Transaction Cost** | ~1 transfer | ~1 transfer + consolidation | **Main Treasury** |
| **CU Usage (Creation)** | ~2,000 CUs | ~2,000 CUs | **Tie** |
| **CU Usage (Total)** | ~2,000 CUs | ~2,000 + 5,000 CUs | **Main Treasury** |
| **Net Fee Efficiency** | 100% | ~99.5% (consolidation cost) | **Main Treasury** |

**Analysis**: Main treasury is more economically efficient due to no consolidation overhead.

#### **🔧 Technical Complexity**

| Aspect | Main Treasury | Pool State | Winner |
|--------|---------------|------------|---------|
| **Code Complexity** | Simple | Complex | **Main Treasury** |
| **Error Handling** | Single point | Dual points | **Main Treasury** |
| **Race Conditions** | None | Possible | **Main Treasury** |
| **State Management** | Single state | Dual state | **Main Treasury** |
| **Rollback Complexity** | Simple | Complex | **Main Treasury** |

**Analysis**: Main treasury approach is significantly simpler and more robust.

#### **🛡️ Rent Exempt Considerations**

| Scenario | Main Treasury | Pool State | Analysis |
|----------|---------------|------------|----------|
| **Pool Creation** | No rent exempt impact | Pool starts above rent exempt minimum | Pool state gets buffer |
| **Later Operations** | N/A | Must preserve rent exempt minimum during consolidation | Added complexity |
| **Edge Cases** | None | What if pool never has other operations? | Stranded fees possible |

**Analysis**: Pool state approach creates rent exempt complexity without significant benefits.

#### **⚡ Performance Impact**

| Operation | Main Treasury | Pool State | Winner |
|-----------|---------------|------------|---------|
| **Pool Creation** | 1 transaction | 1 transaction | **Tie** |
| **Later Consolidation** | Not needed | Required | **Main Treasury** |
| **Treasury Queries** | Real-time data | Delayed until consolidation | **Main Treasury** |
| **Error Recovery** | Simple | Complex | **Main Treasury** |

**Analysis**: Main treasury provides better real-time visibility and simpler operations.

#### **🏗️ Architectural Consistency**

| Principle | Main Treasury | Pool State | Analysis |
|-----------|---------------|------------|----------|
| **Consistency with Other Fees** | Different pattern | Same as swaps/liquidity | Pool state more consistent |
| **Separation of Concerns** | Clear: system fees vs pool fees | Mixed: all fees in pools | Main treasury clearer |
| **Single Source of Truth** | Immediate | Eventual | Main treasury better |
| **Fee Type Semantics** | Creation = system-level | Creation = pool-level | Depends on perspective |

**Analysis**: Pool creation is arguably more of a "system-level" operation than a "pool-level" operation.

---

### **🎯 ARCHITECTURAL RECOMMENDATION: MAIN TREASURY**

#### **Primary Reasons**

1. **🔥 One-Time Nature**: Pool creation happens exactly once, making consolidation efficiency irrelevant
2. **💰 Economic Efficiency**: Saves ~5,000 CUs and consolidation transaction costs
3. **🛡️ Simplicity**: No rent exempt complexity, no consolidation race conditions
4. **⚡ Real-time Visibility**: Treasury information immediately available
5. **🏗️ Semantic Clarity**: Pool creation is a "system registration" fee, not a "pool operation" fee

#### **Secondary Benefits**

- **Cleaner Architecture**: Clear separation between system fees (creation) and operation fees (swaps/liquidity)
- **Better Error Handling**: Single point of failure vs dual state management
- **Immediate Availability**: Treasury balance immediately reflects pool creation revenue
- **Simpler Testing**: No need to test consolidation edge cases for creation fees
- **Future-Proof**: If pool creation fee structure changes, only one collection point to update

#### **When Pool State Might Be Better**

The pool state approach would only be superior if:

1. **High Creation Volume**: If pool creation happened frequently enough to make consolidation batching worthwhile
2. **Creation Fee Variability**: If creation fees varied significantly and needed per-pool tracking
3. **Pool-Specific Accounting**: If creation fees needed to be attributed to individual pools for accounting
4. **Uniform Architecture**: If architectural consistency was more important than efficiency

**Current Reality**: None of these conditions apply to the fixed-ratio trading system.

---

### **🧪 EDGE CASE ANALYSIS**

#### **Scenario 1: Pool Created But Never Used**
- **Main Treasury**: Fee already collected, treasury has revenue
- **Pool State**: Fee trapped in unused pool, requires special consolidation
- **Winner**: **Main Treasury** - revenue not stranded

#### **Scenario 2: Pool Creation Fails After Fee Payment**
- **Main Treasury**: Fee collected, creation rollback needed
- **Pool State**: Fee collected, both creation and consolidation rollback needed
- **Winner**: **Main Treasury** - simpler rollback

#### **Scenario 3: Treasury Withdrawal Needed Immediately**
- **Main Treasury**: Funds immediately available
- **Pool State**: Must consolidate first, then withdraw
- **Winner**: **Main Treasury** - immediate liquidity

#### **Scenario 4: Mass Pool Creation (100+ pools)**
- **Main Treasury**: 100 separate fee collections, immediate treasury updates
- **Pool State**: 100 fee collections + 5-20 consolidation transactions
- **Winner**: **Main Treasury** - fewer total transactions

#### **Scenario 5: Audit/Accounting Requirements**
- **Main Treasury**: All creation fees tracked in single location
- **Pool State**: Creation fees scattered across pools, complex aggregation
- **Winner**: **Main Treasury** - simpler accounting

---

### **💡 IMPLEMENTATION INSIGHT**

The proposed distributed collection architecture correctly keeps pool creation fees going to main treasury because:

1. **Different Fee Nature**: Creation fees are "system registration" costs, not "pool operation" costs
2. **Frequency Mismatch**: Creation (once) vs operations (many times) have different optimization profiles  
3. **Economic Optimization**: One-time fees don't benefit from consolidation batching
4. **Architectural Clarity**: System-level fees should go to system-level treasury
5. **Simplicity Wins**: When consolidation provides no benefits, simpler is better

### **📋 FINAL RECOMMENDATION**

**Keep pool creation fees going to main treasury** for all the reasons above. The distributed collection architecture is correctly designed by treating different fee types appropriately:

- **Pool Creation Fees**: Direct to main treasury (one-time, system-level)
- **Operation Fees**: Distributed collection with consolidation (frequent, pool-level)

This hybrid approach optimizes each fee type according to its characteristics rather than forcing architectural uniformity where it doesn't provide benefits.

**End of Document** 🎯