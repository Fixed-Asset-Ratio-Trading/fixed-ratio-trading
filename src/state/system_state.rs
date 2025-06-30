//! System-wide state management for global pause functionality
//!
//! This module contains the SystemState struct and related functionality for
//! managing system-wide operations like emergency pause/unpause.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

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
    
    /// Human-readable reason for the system pause
    pub pause_reason: String,
}

impl SystemState {
    /// Account space required for SystemState serialization
    /// 
    /// Calculation:
    /// - authority: 32 bytes (Pubkey)
    /// - is_paused: 1 byte (bool)
    /// - pause_timestamp: 8 bytes (i64)
    /// - pause_reason: 4 bytes (String length) + 200 bytes (content)
    pub const LEN: usize = 32 + 1 + 8 + 4 + 200;
    
    /// Creates a new SystemState with the specified authority.
    /// 
    /// # Arguments
    /// * `authority` - The pubkey authorized to pause/unpause the system and perform all contract operations
    /// 
    /// # Returns
    /// A new SystemState initialized in unpaused state
    pub fn new(authority: Pubkey) -> Self {
        Self {
            authority,
            is_paused: false,
            pause_timestamp: 0,
            pause_reason: String::new(),
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
    
    /// Pauses the system with the specified reason and timestamp.
    /// 
    /// # Arguments
    /// * `reason` - Human-readable reason for the pause
    /// * `timestamp` - Unix timestamp when the pause was initiated
    pub fn pause(&mut self, reason: String, timestamp: i64) {
        self.is_paused = true;
        self.pause_timestamp = timestamp;
        self.pause_reason = reason;
    }
    
    /// Unpauses the system, clearing pause state.
    pub fn unpause(&mut self) {
        self.is_paused = false;
        self.pause_timestamp = 0;
        self.pause_reason.clear();
    }
} 