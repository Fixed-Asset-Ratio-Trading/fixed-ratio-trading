//! Central Treasury State for All Contract Fees
//! 
//! **PHASE 3: CENTRALIZED TREASURY ARCHITECTURE**
//! 
//! This module implements a centralized treasury system where all fees are collected
//! directly into the main treasury with real-time counter updates. This eliminates
//! the complexity of specialized treasuries and consolidation race conditions.
//!
//! Key improvements:
//! - Single treasury for all fee types
//! - Real-time counter updates
//! - No consolidation needed
//! - Simplified architecture
//! - Single source of truth for all balances

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// **PHASE 3: CENTRALIZED MAIN TREASURY**
/// 
/// This is the single treasury that collects ALL contract fees directly.
/// All fee types are tracked in real-time with immediate counter updates.
/// No specialized treasuries or consolidation operations are needed.
/// 
/// **Real-time Tracking:**
/// - Pool creation fees: Collected and counted immediately
/// - Liquidity operation fees: Collected and counted immediately  
/// - Regular swap fees: Collected and counted immediately
/// - HFT swap fees: Collected and counted immediately
/// 
/// **Single Source of Truth:**
/// - total_balance: Always reflects actual account balance
/// - All counters: Updated immediately on fee collection
/// - All totals: Updated immediately on fee collection
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct MainTreasuryState {
    /// System authority that can withdraw fees and manage treasury
    pub authority: Pubkey,
    
    /// Current SOL balance of the main treasury account (synced with account.lamports())
    pub total_balance: u64,
    
    /// Total SOL fees withdrawn by authority over time
    pub total_withdrawn: u64,
    
    /// **PHASE 3: REAL-TIME COUNTERS** - Updated immediately on fee collection
    pub pool_creation_count: u64,
    pub liquidity_operation_count: u64,
    pub regular_swap_count: u64,
    pub hft_swap_count: u64,
    
    /// **PHASE 3: REAL-TIME TOTALS** - Updated immediately on fee collection
    pub total_pool_creation_fees: u64,
    pub total_liquidity_fees: u64,
    pub total_regular_swap_fees: u64,
    pub total_hft_swap_fees: u64,
    
    /// Last update timestamp (replaces consolidation timestamp)
    pub last_update_timestamp: i64,
}

impl MainTreasuryState {
    pub const LEN: usize = 
        32 +  // authority
        8 +   // total_balance
        8 +   // total_withdrawn
        8 +   // pool_creation_count
        8 +   // liquidity_operation_count
        8 +   // regular_swap_count
        8 +   // hft_swap_count
        8 +   // total_pool_creation_fees
        8 +   // total_liquidity_fees
        8 +   // total_regular_swap_fees
        8 +   // total_hft_swap_fees
        8;    // last_update_timestamp

    pub fn get_packed_len() -> usize {
        Self::LEN
    }

    pub fn new(authority: Pubkey) -> Self {
        Self {
            authority,
            total_balance: 0,
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
        }
    }
    
    /// **PHASE 3: REAL-TIME FEE TRACKING**
    /// Records a pool creation fee immediately when collected
    pub fn add_pool_creation_fee(&mut self, fee_amount: u64, timestamp: i64) {
        self.pool_creation_count += 1;
        self.total_pool_creation_fees += fee_amount;
        self.last_update_timestamp = timestamp;
    }
    
    /// **PHASE 3: REAL-TIME FEE TRACKING**
    /// Records a liquidity operation fee immediately when collected
    pub fn add_liquidity_fee(&mut self, fee_amount: u64, timestamp: i64) {
        self.liquidity_operation_count += 1;
        self.total_liquidity_fees += fee_amount;
        self.last_update_timestamp = timestamp;
    }
    
    /// **PHASE 3: REAL-TIME FEE TRACKING**
    /// Records a regular swap fee immediately when collected
    pub fn add_regular_swap_fee(&mut self, fee_amount: u64, timestamp: i64) {
        self.regular_swap_count += 1;
        self.total_regular_swap_fees += fee_amount;
        self.last_update_timestamp = timestamp;
    }
    
    /// **PHASE 3: REAL-TIME FEE TRACKING**
    /// Records an HFT swap fee immediately when collected
    pub fn add_hft_swap_fee(&mut self, fee_amount: u64, timestamp: i64) {
        self.hft_swap_count += 1;
        self.total_hft_swap_fees += fee_amount;
        self.last_update_timestamp = timestamp;
    }
    
    /// **PHASE 3: SIMPLIFIED BALANCE SYNC**
    /// Updates total_balance to match actual account balance
    pub fn sync_balance_with_account(&mut self, account_lamports: u64) {
        self.total_balance = account_lamports;
    }
}

/// **PHASE 3: TREASURY MANAGEMENT UTILITIES**
impl MainTreasuryState {
    /// Validates authority for treasury operations
    pub fn validate_authority(&self, authority: &Pubkey) -> bool {
        self.authority == *authority
    }
    
    /// Calculates available balance for withdrawal (total - minimum for rent)
    pub fn available_for_withdrawal(&self, minimum_balance: u64) -> u64 {
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
    
    /// **PHASE 3: ANALYTICS METHODS**
    /// Calculates total fees collected across all categories
    pub fn total_fees_collected(&self) -> u64 {
        self.total_pool_creation_fees +
        self.total_liquidity_fees +
        self.total_regular_swap_fees +
        self.total_hft_swap_fees
    }
    
    /// **PHASE 3: ANALYTICS METHODS**
    /// Calculates total operations processed across all categories
    pub fn total_operations_processed(&self) -> u64 {
        self.pool_creation_count +
        self.liquidity_operation_count +
        self.regular_swap_count +
        self.hft_swap_count
    }
    
    /// **PHASE 3: ANALYTICS METHODS**
    /// Calculates average fee per operation (if any operations have been processed)
    pub fn average_fee_per_operation(&self) -> f64 {
        let total_ops = self.total_operations_processed();
        if total_ops > 0 {
            self.total_fees_collected() as f64 / total_ops as f64
        } else {
            0.0
        }
    }
}

// ============================================================================
// PHASE 3: SPECIALIZED TREASURY STRUCTURES REMOVED
// ============================================================================
// 
// The following structures have been removed in Phase 3:
// - SwapTreasuryState: No longer needed, fees go directly to main treasury
// - HftTreasuryState: No longer needed, fees go directly to main treasury
// 
// Benefits of removal:
// - Eliminates consolidation race conditions
// - Simplifies architecture significantly
// - Provides real-time fee tracking
// - Reduces compute unit usage
// - Single source of truth for all treasury data
// 
// Migration path:
// - All existing specialized treasury accounts can be closed
// - Fees are collected directly into main treasury
// - Real-time counters replace consolidation-based tracking
// ============================================================================ 