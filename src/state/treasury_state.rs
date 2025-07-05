//! Central Treasury State for All Contract Fees
//! 
//! This module contains treasury structures for collecting and tracking
//! all contract fees across the protocol in a centralized manner with
//! separate collection points for different fee types.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Main treasury that consolidates all contract fees across the protocol.
/// 
/// This is the primary treasury where all fees eventually end up. The system
/// authority can withdraw fees from this treasury. Other specialized treasuries
/// feed into this main treasury when counts are requested.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct MainTreasuryState {
    /// System authority that can withdraw fees and manage treasury
    pub authority: Pubkey,
    
    /// Total SOL fees currently in this treasury (after consolidation)
    pub total_balance: u64,
    
    /// Total SOL fees withdrawn by authority over time
    pub total_withdrawn: u64,
    
    /// Comprehensive counters for all fee types (updated on consolidation)
    pub pool_creation_count: u64,
    pub liquidity_operation_count: u64,
    pub regular_swap_count: u64,
    pub hft_swap_count: u64,
    
    /// Total fees collected by category (cumulative)
    pub total_pool_creation_fees: u64,
    pub total_liquidity_fees: u64,
    pub total_regular_swap_fees: u64,
    pub total_hft_swap_fees: u64,
    
    /// Last consolidation timestamp
    pub last_consolidation_timestamp: i64,
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
        8;    // last_consolidation_timestamp

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
            last_consolidation_timestamp: 0,
        }
    }
}

/// Specialized treasury for regular swap fees only.
/// 
/// This treasury collects fees from standard swap operations and gets
/// emptied into the main treasury when consolidation is triggered.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct SwapTreasuryState {
    /// Current balance in this specialized treasury
    pub balance: u64,
    
    /// Number of regular swaps processed
    pub swap_count: u64,
    
    /// Total fees collected since last consolidation
    pub total_collected: u64,
    
    /// Last consolidation timestamp
    pub last_consolidation: i64,
}

impl SwapTreasuryState {
    pub const LEN: usize = 
        8 +   // balance
        8 +   // swap_count
        8 +   // total_collected
        8;    // last_consolidation

    pub fn get_packed_len() -> usize {
        Self::LEN
    }

    pub fn new() -> Self {
        Self {
            balance: 0,
            swap_count: 0,
            total_collected: 0,
            last_consolidation: 0,
        }
    }
    
    /// Adds a regular swap fee to this treasury
    pub fn add_swap_fee(&mut self, fee_amount: u64) {
        self.balance += fee_amount;
        self.swap_count += 1;
        self.total_collected += fee_amount;
    }
    
    /// Empties this treasury and returns the data for main treasury consolidation
    pub fn drain(&mut self) -> (u64, u64, u64) {
        let balance = self.balance;
        let count = self.swap_count;
        let total = self.total_collected;
        
        self.balance = 0;
        self.swap_count = 0;
        self.total_collected = 0;
        
        (balance, count, total)
    }
}

/// Specialized treasury for HFT swap fees only.
/// 
/// This treasury collects fees from HFT-optimized swap operations and gets
/// emptied into the main treasury when consolidation is triggered.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct HftTreasuryState {
    /// Current balance in this specialized treasury
    pub balance: u64,
    
    /// Number of HFT swaps processed
    pub hft_swap_count: u64,
    
    /// Total fees collected since last consolidation
    pub total_collected: u64,
    
    /// Last consolidation timestamp
    pub last_consolidation: i64,
}

impl HftTreasuryState {
    pub const LEN: usize = 
        8 +   // balance
        8 +   // hft_swap_count
        8 +   // total_collected
        8;    // last_consolidation

    pub fn get_packed_len() -> usize {
        Self::LEN
    }

    pub fn new() -> Self {
        Self {
            balance: 0,
            hft_swap_count: 0,
            total_collected: 0,
            last_consolidation: 0,
        }
    }
    
    /// Adds an HFT swap fee to this treasury
    pub fn add_hft_swap_fee(&mut self, fee_amount: u64) {
        self.balance += fee_amount;
        self.hft_swap_count += 1;
        self.total_collected += fee_amount;
    }
    
    /// Empties this treasury and returns the data for main treasury consolidation
    pub fn drain(&mut self) -> (u64, u64, u64) {
        let balance = self.balance;
        let count = self.hft_swap_count;
        let total = self.total_collected;
        
        self.balance = 0;
        self.hft_swap_count = 0;
        self.total_collected = 0;
        
        (balance, count, total)
    }
}

/// Treasury management utilities
impl MainTreasuryState {
    /// Consolidates fees from specialized treasuries into the main treasury
    pub fn consolidate_from_specialized_treasuries(
        &mut self,
        swap_treasury_data: (u64, u64, u64),  // (balance, count, total)
        hft_treasury_data: (u64, u64, u64),   // (balance, count, total)
        timestamp: i64,
    ) {
        let (swap_balance, swap_count, swap_total) = swap_treasury_data;
        let (hft_balance, hft_count, hft_total) = hft_treasury_data;
        
        // Add balances to main treasury
        self.total_balance += swap_balance + hft_balance;
        
        // Update counters
        self.regular_swap_count += swap_count;
        self.hft_swap_count += hft_count;
        
        // Update cumulative totals
        self.total_regular_swap_fees += swap_total;
        self.total_hft_swap_fees += hft_total;
        
        // Update consolidation timestamp
        self.last_consolidation_timestamp = timestamp;
    }
    
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
} 