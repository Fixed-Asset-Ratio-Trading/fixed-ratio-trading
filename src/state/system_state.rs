//! System-wide state management for global pause functionality
//!
//! This module contains the SystemState struct and related functionality for
//! managing system-wide operations like emergency pause/unpause.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// **PAUSE REASON CODES** (Documentation Only - Not Part of Smart Contract Logic)
/// 
/// These standardized codes are used for efficient storage. Client applications
/// should map these codes to human-readable text for display purposes.
/// 
/// **Standard Pause Codes:**
/// - 0: No pause active (default state)
/// - 1: Temporary consolidation of funds across pools  
/// - 2: Contract upgrade in progress
/// - 3: Critical security issue detected
/// - 4: Routine maintenance and debugging
/// - 5: Emergency halt due to unexpected behavior
/// - 6: Governance action or vote in progress
/// - 7: Technical issues with external dependencies
/// - 8: Compliance or regulatory requirements
/// - 9: Testing or development activities
/// - 10: Oracle or price feed issues
/// - 11: Liquidity management operations
/// - 12: Network congestion or high fees
/// - 13: Token economic rebalancing
/// - 14: External audit in progress
/// - 15: Scheduled system maintenance
/// - 255: Custom reason (see external documentation)

/// System-wide state that controls global operations for the entire contract.
/// 
/// This state is separate from individual pool states and provides emergency
/// controls that can override all pool operations when necessary.
/// Only the contract authority can perform system-wide operations.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct SystemState {
    /// Authority that can pause/unpause the entire system and perform contract operations
    pub authority: Pubkey,
    
    /// Global pause state - when true, all operations are blocked except unpause
    pub is_paused: bool,
    
    /// Unix timestamp when the system was paused
    pub pause_timestamp: i64,
    
    /// Pause reason code for efficient storage (see documentation above for meanings)
    pub pause_reason_code: u8,
}

impl SystemState {
    /// Account space required for SystemState serialization
    /// 
    /// **ULTRA-OPTIMIZED CALCULATION** (203 bytes saved vs String version):
    /// - authority: 32 bytes (Pubkey)
    /// - is_paused: 1 byte (bool)
    /// - pause_timestamp: 8 bytes (i64)
    /// - pause_reason_code: 1 byte (u8) [was 204 bytes for String]
    /// 
    /// **TOTAL: 42 bytes** (vs 245 bytes previously - **83% reduction!**)
    pub const LEN: usize = 32 + 1 + 8 + 1;
    
    /// Creates a new SystemState with the specified authority.
    /// 
    /// # Arguments
    /// * `authority` - The pubkey authorized to pause/unpause the system and perform all contract operations
    /// 
    /// # Returns
    /// A new SystemState initialized in unpaused state (code 0)
    pub fn new(authority: Pubkey) -> Self {
        Self {
            authority,
            is_paused: false,
            pause_timestamp: 0,
            pause_reason_code: 0, // 0 = No pause active
        }
    }
    
    /// Validates that the provided pubkey has authority to modify system state.
    /// 
    /// # Arguments
    /// * `authority` - The pubkey to validate
    /// 
    /// # Returns
    /// * `true` if the pubkey matches the system authority, `false` otherwise
    pub fn validate_authority(&self, authority: &Pubkey) -> bool {
        self.authority == *authority
    }
    
    /// Pauses the system with the specified reason code and timestamp.
    /// 
    /// # Arguments
    /// * `reason_code` - Pause reason code (see documentation above)
    /// * `timestamp` - Unix timestamp when the pause was initiated
    pub fn pause(&mut self, reason_code: u8, timestamp: i64) {
        self.is_paused = true;
        self.pause_timestamp = timestamp;
        self.pause_reason_code = reason_code;
    }
    
    /// Unpauses the system, clearing pause state.
    pub fn unpause(&mut self) {
        self.is_paused = false;
        self.pause_timestamp = 0;
        self.pause_reason_code = 0; // 0 = No pause active
    }
} 