//! Distributed Fee Collection with Centralized Treasury Consolidation
//! 
//! **DISTRIBUTED COLLECTION ARCHITECTURE**
//! 
//! This module implements a distributed fee collection system where operational fees
//! (liquidity/swaps) are collected to individual pool states and then consolidated
//! to the main treasury in batches. Pool creation fees go directly to the treasury.
//!
//! **FEE COLLECTION PATHS**
//! 
//! - Pool creation fees: Direct to main treasury (one-time operations)
//! - Liquidity/swap fees: Collected to pool states, consolidated in batches
//! - Treasury operations: Direct treasury updates (withdrawals, donations)
//!
//! Key benefits:
//! - Reduced compute units per operation (67% savings for liquidity/swaps)
//! - Distributed storage reduces treasury account contention
//! - Batch consolidation improves efficiency
//! - Complete treasury visibility through consolidation
//! - Optimal fee routing based on operation frequency

use borsh::{BorshDeserialize, BorshSerialize};

/// **MAIN TREASURY: CONSOLIDATION TARGET FOR DISTRIBUTED FEES**
/// 
/// This treasury serves as the consolidation target for fees collected across
/// individual pool states. It provides centralized tracking and management
/// while supporting the distributed collection architecture.
/// 
/// **Fee Collection Sources:**
/// - Pool creation fees: Direct collection (immediate tracking)
/// - Liquidity operation fees: Consolidated from pool states (batch tracking)
/// - Regular swap fees: Consolidated from pool states (batch tracking)
/// - Treasury operations: Direct updates (withdrawals, donations)
/// 
/// **Consolidation Process:**
/// - Pool states accumulate fees and operation counts locally
/// - Periodic consolidation transfers fees and counts to this treasury
/// - Treasury maintains complete historical view of all protocol activity
/// - Real-time balance tracking with distributed fee accumulation
/// 
/// **Consolidation Benefits:**
/// - Reduced CU costs for high-frequency operations
/// - Distributed fee storage prevents treasury account bottlenecks
/// - Batch processing improves overall system efficiency
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Default)]
pub struct MainTreasuryState {
    /// Current SOL balance of the main treasury account (synced with account.lamports())
    pub total_balance: u64,
    
    /// **NEW: Rent-exempt minimum balance requirement**
    pub rent_exempt_minimum: u64,
    
    /// Total SOL fees withdrawn by authority over time
    pub total_withdrawn: u64,
    
    /// **OPERATION COUNTERS** - Updated via direct collection or consolidation
    pub pool_creation_count: u64,      // Direct: Updated immediately on pool creation
    pub liquidity_operation_count: u64, // Consolidated: Updated from pool states
    pub regular_swap_count: u64,        // Consolidated: Updated from pool states
    
    /// **NEW: EXTENDED COUNTERS** - Additional operation tracking
    pub treasury_withdrawal_count: u64,
    pub failed_operation_count: u64,
    
    /// **FEE TOTALS** - Updated via direct collection or consolidation
    pub total_pool_creation_fees: u64, // Direct: Updated immediately on pool creation
    pub total_liquidity_fees: u64,     // Consolidated: Updated from pool states
    pub total_regular_swap_fees: u64,  // Consolidated: Updated from pool states
    
    /// Total swap contract fees collected across all pools
    /// These are fixed SOL fees charged per swap to cover computational costs
    pub total_swap_contract_fees: u64,
    
    /// Last update timestamp (replaces consolidation timestamp)
    pub last_update_timestamp: i64,
    
    /// **NEW: Consolidation tracking**
    /// Number of consolidation operations performed
    pub total_consolidations_performed: u64,
    
    /// **RATE LIMITING: Timestamp of last treasury withdrawal**
    /// Used for rolling 60-minute withdrawal rate limiting
    pub last_withdrawal_timestamp: i64,
    
    /// **DONATION TRACKING: Total number of donations received**
    /// Tracks voluntary SOL contributions to the protocol
    pub donation_count: u64,
    
    /// **DONATION TRACKING: Total SOL donated to the protocol**
    /// Sum of all voluntary donations in lamports
    pub total_donations: u64,
}

/// **NEW: Consolidated operations data structure**
/// Used for batch consolidation processing from multiple pools
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Default)]
pub struct ConsolidatedOperations {
    pub liquidity_fees: u64,
    pub regular_swap_fees: u64,
    pub liquidity_operation_count: u64,
    pub regular_swap_count: u64,
}



impl MainTreasuryState {
    pub const LEN: usize = 
        8 +   // total_balance
        8 +   // rent_exempt_minimum ← NEW
        8 +   // total_withdrawn
        8 +   // pool_creation_count
        8 +   // liquidity_operation_count
        8 +   // regular_swap_count
        8 +   // treasury_withdrawal_count ← NEW
        8 +   // failed_operation_count ← NEW
        8 +   // total_pool_creation_fees
        8 +   // total_liquidity_fees
        8 +   // total_regular_swap_fees
        8 +   // total_swap_contract_fees ← NEW
        8 +   // last_update_timestamp
        8 +   // total_consolidations_performed ← NEW
        8 +   // last_withdrawal_timestamp ← NEW (for rate limiting)
        8 +   // donation_count ← NEW
        8;    // total_donations ← NEW
        // **TOTAL ADDITION: +40 bytes** (includes 16 bytes for donation tracking)
        // Authority removed: 32 bytes saved, validation handled through SystemState

    pub fn get_packed_len() -> usize {
        Self::LEN
    }

    pub fn new() -> Self {
        Self {
            total_balance: 0,
            rent_exempt_minimum: 0,
            total_withdrawn: 0,
            pool_creation_count: 0,
            liquidity_operation_count: 0,
            regular_swap_count: 0,
            treasury_withdrawal_count: 0,
            failed_operation_count: 0,
            total_pool_creation_fees: 0,
            total_liquidity_fees: 0,
            total_regular_swap_fees: 0,
            total_swap_contract_fees: 0,
            last_update_timestamp: 0,
            total_consolidations_performed: 0,
            last_withdrawal_timestamp: 0,
            donation_count: 0,
            total_donations: 0,
        }
    }
    
    /// **NEW: Initialize with rent-exempt balance**
    pub fn new_with_rent_exemption(rent_exempt_minimum: u64) -> Self {
        Self {
            total_balance: rent_exempt_minimum, // Start with rent-exempt balance
            rent_exempt_minimum,
            total_withdrawn: 0,
            pool_creation_count: 0,
            liquidity_operation_count: 0,
            regular_swap_count: 0,
            treasury_withdrawal_count: 0,
            failed_operation_count: 0,
            total_pool_creation_fees: 0,
            total_liquidity_fees: 0,
            total_regular_swap_fees: 0,
            total_swap_contract_fees: 0,
            last_update_timestamp: 0,
            total_consolidations_performed: 0,
            last_withdrawal_timestamp: 0,
            donation_count: 0,
            total_donations: 0,
        }
    }
    
    /// **DIRECT COLLECTION: Pool creation fee tracking**
    /// Records a pool creation fee collected directly to treasury
    pub fn add_pool_creation_fee(&mut self, fee_amount: u64, timestamp: i64) {
        self.pool_creation_count += 1;
        self.total_pool_creation_fees += fee_amount;
        self.last_update_timestamp = timestamp;
    }
    
    /// **CONSOLIDATION: Liquidity operation fee tracking**
    /// Records liquidity operation fees consolidated from pool states
    /// Note: This is called during consolidation, not during individual operations
    pub fn add_liquidity_fee(&mut self, fee_amount: u64, timestamp: i64) {
        self.liquidity_operation_count += 1;
        self.total_liquidity_fees += fee_amount;
        self.last_update_timestamp = timestamp;
    }
    
    /// **CONSOLIDATION: Swap contract fee tracking**
    /// 
    /// Records swap contract fees consolidated from pool states.
    /// These are fixed SOL fees charged per swap to cover computational costs.
    /// Note: This is called during consolidation, not during individual operations
    /// 
    /// # Arguments  
    /// * `fee_amount` - The swap contract fee amount in lamports
    /// * `timestamp` - Timestamp of the fee collection
    pub fn add_swap_contract_fee(&mut self, fee_amount: u64, timestamp: i64) {
        self.regular_swap_count += 1;  // Increment the operation count
        self.total_swap_contract_fees += fee_amount;
        self.last_update_timestamp = timestamp;
        
        // Also update the legacy regular_swap_fees for backward compatibility
        // TODO: Remove this after migration period
        self.total_regular_swap_fees += fee_amount;
    }

    /// Adds a regular swap fee to the treasury (legacy method)
    /// 
    /// **DEPRECATED**: Use add_swap_contract_fee instead for new code.
    /// This method is maintained for backward compatibility only.
    pub fn add_regular_swap_fee(&mut self, fee_amount: u64, timestamp: i64) {
        // Delegate to the new method to ensure consistency
        self.add_swap_contract_fee(fee_amount, timestamp);
    }
    
    /// **NEW: Records a treasury withdrawal operation**
    pub fn add_treasury_withdrawal(&mut self, withdrawal_amount: u64, timestamp: i64) {
        self.treasury_withdrawal_count += 1;
        self.total_withdrawn += withdrawal_amount;
        self.last_update_timestamp = timestamp;
        self.last_withdrawal_timestamp = timestamp;
    }
    
    /// **NEW: Records a failed operation for debugging and analytics**
    pub fn add_failed_operation(&mut self, timestamp: i64) {
        self.failed_operation_count += 1;
        self.last_update_timestamp = timestamp;
    }
    
    /// **DONATION TRACKING: Records a voluntary donation to the treasury**
    /// 
    /// This function tracks donations separately from fees to provide transparency
    /// about voluntary contributions vs mandatory protocol fees.
    /// 
    /// # Arguments
    /// * `donation_amount` - The donation amount in lamports
    /// * `timestamp` - Timestamp of the donation
    pub fn add_donation(&mut self, donation_amount: u64, timestamp: i64) {
        self.donation_count += 1;
        self.total_donations += donation_amount;
        self.last_update_timestamp = timestamp;
    }
    
    /// **NEW: Calculate total successful operations across all types**
    pub fn total_successful_operations(&self) -> u64 {
        self.pool_creation_count + 
        self.liquidity_operation_count + 
        self.regular_swap_count + 
        self.treasury_withdrawal_count +
        self.total_consolidations_performed +
        self.donation_count
    }
    
    /// **NEW: Calculate success rate (successful vs failed operations)**
    pub fn success_rate_percentage(&self) -> f64 {
        let total_operations = self.total_successful_operations() + self.failed_operation_count;
        if total_operations == 0 {
            100.0 // No operations yet, consider 100% success rate
        } else {
            (self.total_successful_operations() as f64 / total_operations as f64) * 100.0
        }
    }
    
    /// **NEW: Calculate average fees per operation type**
    pub fn average_pool_creation_fee(&self) -> f64 {
        if self.pool_creation_count == 0 {
            0.0
        } else {
            self.total_pool_creation_fees as f64 / self.pool_creation_count as f64
        }
    }
    
    pub fn average_liquidity_fee(&self) -> f64 {
        if self.liquidity_operation_count == 0 {
            0.0
        } else {
            self.total_liquidity_fees as f64 / self.liquidity_operation_count as f64
        }
    }
    
    pub fn average_swap_fee(&self) -> f64 {
        if self.regular_swap_count == 0 {
            0.0
        } else {
            self.total_regular_swap_fees as f64 / self.regular_swap_count as f64
        }
    }
    
    /// **BALANCE SYNCHRONIZATION**
    /// Synchronizes internal balance tracking with actual account balance
    pub fn sync_balance_with_account(&mut self, account_lamports: u64) {
        self.total_balance = account_lamports;
    }
    
    /// **NEW: Process batch consolidation from multiple pools**
    pub fn batch_consolidation(
        &mut self,
        _consolidated_fees: u64,
        consolidated_operations: &ConsolidatedOperations,
        timestamp: i64,
    ) {
        // Update fee totals (pool creation fees handled during initial creation)
        self.total_liquidity_fees += consolidated_operations.liquidity_fees;
        self.total_regular_swap_fees += consolidated_operations.regular_swap_fees;
        self.total_swap_contract_fees += consolidated_operations.regular_swap_fees; // Same fees, different tracking
        
        // Update operation counts
        self.liquidity_operation_count += consolidated_operations.liquidity_operation_count;
        self.regular_swap_count += consolidated_operations.regular_swap_count;
        
        // Update consolidation metadata
        self.total_consolidations_performed += 1;
        self.last_update_timestamp = timestamp;
    }
    
    /// **DYNAMIC RATE LIMITING: Calculate current hourly withdrawal rate limit**
    /// 
    /// Calculates the appropriate hourly rate limit based on available treasury balance
    /// using a dynamic scaling system that ensures the treasury can be drained within 48 hours.
    /// 
    /// **Hourly Rate Limits by Treasury Balance:**
    /// - 27 SOL treasury → ~25 SOL available → **10 SOL/hour** (Tier 1 - Base)
    /// - 500 SOL treasury → ~498 SOL available → **100 SOL/hour** (Tier 2 - 10x)
    /// - 5,000 SOL treasury → ~4,998 SOL available → **1,000 SOL/hour** (Tier 3 - 100x)
    /// 
    /// **Scaling Tiers:**
    /// - Tier 1: ≤480 SOL available → 10 SOL/hour (base rate)
    /// - Tier 2: ≤4,800 SOL available → 100 SOL/hour (10x multiplier)
    /// - Tier 3: ≤48,000 SOL available → 1,000 SOL/hour (100x multiplier)
    /// - Each tier scales by 10x when 48-hour drain threshold is exceeded
    /// 
    /// # Returns
    /// * `u64` - Current hourly rate limit in lamports
    pub fn calculate_current_hourly_rate_limit(&self) -> u64 {
        use crate::constants::{
            TREASURY_BASE_HOURLY_RATE, 
            TREASURY_MAX_DRAIN_TIME_HOURS, 
            TREASURY_RATE_SCALING_MULTIPLIER
        };
        
        let available_balance = self.available_for_withdrawal();
        let mut current_rate = TREASURY_BASE_HOURLY_RATE; // Start with 10 SOL/hour
        
        // Scale up rate by 10x whenever 48-hour drain time would be exceeded
        while available_balance > (TREASURY_MAX_DRAIN_TIME_HOURS * current_rate) {
            current_rate = current_rate.saturating_mul(TREASURY_RATE_SCALING_MULTIPLIER);
            
            // Safety check to prevent infinite loop (though practically impossible)
            if current_rate == 0 || current_rate == u64::MAX {
                break;
            }
        }
        
        current_rate
    }
    
    /// **RATE LIMITING: Dynamic hourly rate limit with non-cumulative 60-min cooldown**
    /// 
    /// Validates that the requested withdrawal amount doesn't exceed the dynamically calculated
    /// hourly rate limit. The 60-minute cooldown timer starts only after successful withdrawals;
    /// failed attempts (exceeding hourly limit or within cooldown) do not extend or reset cooldown.
    /// 
    /// # Arguments
    /// * `withdrawal_amount` - Amount to be withdrawn in lamports
    /// * `current_timestamp` - Current timestamp for rate limit window calculation
    /// 
    /// # Returns
    /// * `Ok(())` - If withdrawal is within rate limit
    /// * `Err(&'static str)` - If withdrawal exceeds rate limit with error message
    pub fn validate_withdrawal_rate_limit(&self, withdrawal_amount: u64, current_timestamp: i64) -> Result<(), &'static str> {
        use crate::constants::TREASURY_WITHDRAWAL_RATE_LIMIT_WINDOW;
        
        // **FIRST CHECK: System restart penalty (71 hours)**
        // This takes precedence over all other rate limiting and cooldown checks
        if self.is_blocked_by_restart_penalty(current_timestamp) {
            return Err("Withdrawal blocked: system restart penalty active (3-day cooling-off period)");
        }
        
        // Calculate current dynamic rate limit based on available balance
        let current_hourly_limit = self.calculate_current_hourly_rate_limit();
        
        // If this is the first withdrawal ever, check against current rate limit
        if self.last_withdrawal_timestamp == 0 {
            if withdrawal_amount > current_hourly_limit {
                return Err("Withdrawal amount exceeds current hourly rate limit");
            }
            return Ok(());
        }
        
        // Calculate time since last successful withdrawal
        let time_since_last_withdrawal = current_timestamp - self.last_withdrawal_timestamp;
        
        // If more than 60 minutes have passed since last successful withdrawal, check against current rate limit
        if time_since_last_withdrawal >= TREASURY_WITHDRAWAL_RATE_LIMIT_WINDOW {
            if withdrawal_amount > current_hourly_limit {
                return Err("Withdrawal amount exceeds current hourly rate limit");
            }
            return Ok(());
        }
        
        // Within 60-minute window: reject any withdrawal request
        // This enforces the "1 withdrawal per hour" rule regardless of amount
        Err("Rate limit exceeded: withdrawals are limited to once per hour")
    }
    
    /// **RATE LIMITING: Get time remaining until next withdrawal is allowed**
    /// 
    /// # Arguments
    /// * `current_timestamp` - Current timestamp for calculation
    /// 
    /// # Returns
    /// * `0` - If withdrawal is allowed now
    /// * `seconds` - Number of seconds until next withdrawal is allowed
    pub fn time_until_next_withdrawal_allowed(&self, current_timestamp: i64) -> i64 {
        use crate::constants::TREASURY_WITHDRAWAL_RATE_LIMIT_WINDOW;
        
        if self.last_withdrawal_timestamp == 0 {
            return 0; // First withdrawal ever, no waiting
        }
        
        let time_since_last_withdrawal = current_timestamp - self.last_withdrawal_timestamp;
        
        if time_since_last_withdrawal >= TREASURY_WITHDRAWAL_RATE_LIMIT_WINDOW {
            return 0; // Rate limit window has passed
        }
        
        TREASURY_WITHDRAWAL_RATE_LIMIT_WINDOW - time_since_last_withdrawal
    }
    
    /// **SYSTEM RESTART PENALTY: Apply withdrawal penalty when system is re-enabled**
    /// 
    /// Sets the last withdrawal timestamp to 71 hours in the future to prevent
    /// withdrawals for 3 days after system restart. This security measure prevents
    /// immediate fund drainage after system maintenance or emergency halts.
    /// 
    /// # Arguments
    /// * `current_timestamp` - Current timestamp when system is being re-enabled
    pub fn apply_system_restart_penalty(&mut self, current_timestamp: i64) {
        use crate::constants::TREASURY_SYSTEM_RESTART_PENALTY_SECONDS;
        
        // Set last withdrawal timestamp 71 hours into the future
        self.last_withdrawal_timestamp = current_timestamp + TREASURY_SYSTEM_RESTART_PENALTY_SECONDS;
        
        // Also update the general timestamp for tracking
        self.last_update_timestamp = current_timestamp;
    }
    
    /// **SYSTEM RESTART PENALTY: Check if withdrawal is blocked due to system restart penalty**
    /// 
    /// # Arguments
    /// * `current_timestamp` - Current timestamp for penalty check
    /// 
    /// # Returns
    /// * `true` - If withdrawal is blocked due to restart penalty
    /// * `false` - If restart penalty period has expired
    pub fn is_blocked_by_restart_penalty(&self, current_timestamp: i64) -> bool {
        // If last withdrawal timestamp is in the future, we're in penalty period
        self.last_withdrawal_timestamp > current_timestamp
    }
    
    /// **SYSTEM RESTART PENALTY: Get remaining restart penalty time**
    /// 
    /// # Arguments
    /// * `current_timestamp` - Current timestamp for calculation
    /// 
    /// # Returns
    /// * `0` - If no penalty is active
    /// * `seconds` - Number of seconds remaining in penalty period
    pub fn restart_penalty_time_remaining(&self, current_timestamp: i64) -> i64 {
        if self.last_withdrawal_timestamp > current_timestamp {
            self.last_withdrawal_timestamp - current_timestamp
        } else {
            0
        }
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

impl MainTreasuryState {
    /// Calculate available balance for withdrawal with explicit minimum balance
    pub fn available_for_withdrawal_with_minimum(&self, minimum_balance: u64) -> u64 {
        if self.total_balance > minimum_balance {
            self.total_balance - minimum_balance
        } else {
            0
        }
    }
    
    /// Records a withdrawal by the authority
    pub fn record_withdrawal(&mut self, amount: u64) -> Result<(), &'static str> {
        if amount > self.total_balance {
            return Err("Insufficient balance");
        }
        
        self.total_balance -= amount;
        self.total_withdrawn += amount;
        Ok(())
    }
    
    /// **ANALYTICS: Total fees calculation**
    /// Calculates total fees collected across all categories
    pub fn total_fees_collected(&self) -> u64 {
        self.total_pool_creation_fees +
        self.total_liquidity_fees +
        self.total_regular_swap_fees
    }
    
    /// **ANALYTICS: Total operations calculation**
    /// Calculates total operations processed across all categories
    pub fn total_operations_processed(&self) -> u64 {
        self.pool_creation_count +
        self.liquidity_operation_count +
        self.regular_swap_count
    }
    
    /// **ANALYTICS: Average fee calculation**
    /// Calculates average fee per operation (if any operations have been processed)
    pub fn average_fee_per_operation(&self) -> f64 {
        let total_ops = self.total_operations_processed();
        if total_ops > 0 {
            self.total_fees_collected() as f64 / total_ops as f64
        } else {
            0.0
        }
    }

    /// Records consolidated fee operations from pool states
    /// 
    /// This function processes consolidated operations from pool fee collection,
    /// updating the treasury's tracking of all fee types and operation counts.
    pub fn record_consolidated_operations(&mut self, consolidated_operations: &ConsolidatedOperations, timestamp: i64) {
        // Add all fee types
        self.total_liquidity_fees += consolidated_operations.liquidity_fees;
        self.total_regular_swap_fees += consolidated_operations.regular_swap_fees;
        self.total_swap_contract_fees += consolidated_operations.regular_swap_fees; // Same fees, different tracking
        
        // Update operation counts using correct field names
        self.liquidity_operation_count += consolidated_operations.liquidity_operation_count;
        self.regular_swap_count += consolidated_operations.regular_swap_count;
        
        // Update metadata
        self.last_update_timestamp = timestamp;
        self.total_consolidations_performed += 1;
    }
}

// ============================================================================
// SPECIALIZED TREASURY STRUCTURES 
// ============================================================================
// Benefits of removal:
// - Eliminates consolidation race conditions
// - Simplifies architecture significantly
// - Provides real-time fee tracking
// - Single source of truth for all treasury operations
// - Reduces compute unit usage for fee operations
// 
// Migration notes:
// - All specialized treasury accounts can be closed
// - All fees now route directly to main treasury
// - Real-time tracking eliminates need for consolidation delays
// ============================================================================ 