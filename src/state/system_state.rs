//! System-wide state management for global pause functionality
//!
//! This module contains the SystemState struct and related functionality for
//! managing system-wide operations like emergency pause/unpause.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Result of processing an admin authority change
#[derive(Debug, Clone)]
pub enum AdminChangeResult {
    /// Admin change was initiated with 72-hour timer
    Initiated { 
        new_admin: Pubkey, 
        previous_pending: Option<Pubkey> 
    },
    /// Admin change was completed after timelock
    Completed { 
        old_admin: Pubkey, 
        new_admin: Pubkey 
    },
    /// Pending admin change was cancelled
    Cancelled,
    /// No change needed (same admin as current)
    NoChange,
    /// Change is still pending (timelock not satisfied)
    Pending { 
        pending_admin: Pubkey, 
        remaining_seconds: i64 
    },
}

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
///
///   System-wide state that controls global operations for the entire contract.
/// 
/// This state is separate from individual pool states and provides emergency
/// controls that can override all pool operations when necessary.
/// Only the program upgrade authority can perform system-wide operations.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct SystemState {
    /// Global pause state - when true, all operations are blocked except unpause
    pub is_paused: bool,
    
    /// Unix timestamp when the system was paused
    pub pause_timestamp: i64,
    
    /// Pause reason code for efficient storage (see documentation above for meanings)
    pub pause_reason_code: u8,
    
    /// **ADMIN AUTHORITY SYSTEM WITH 72-HOUR TIMELOCK**
    /// Current admin authority that can perform all admin operations
    pub admin_authority: Pubkey,
    
    /// Pending admin authority change (None if no change pending)
    pub pending_admin_authority: Option<Pubkey>,
    
    /// Timestamp when admin authority change was initiated (0 if no change pending)
    pub admin_change_timestamp: i64,
    
    /// **FUTURE EXPANSION RESERVED SPACE**
    /// Reserved bytes for future features without breaking compatibility
    /// Always initialize to zero and ignore during deserialization
    pub _reserved: [u8; 128],
}

impl SystemState {
    /// Account space required for SystemState serialization
    /// 
    /// **UPDATED CALCULATION WITH ADMIN AUTHORITY SYSTEM + RESERVED SPACE**:
    /// - is_paused: 1 byte (bool)
    /// - pause_timestamp: 8 bytes (i64)
    /// - pause_reason_code: 1 byte (u8)
    /// - admin_authority: 32 bytes (Pubkey)
    /// - pending_admin_authority: 33 bytes (Option<Pubkey> = 1 + 32)
    /// - admin_change_timestamp: 8 bytes (i64)
    /// - _reserved: 128 bytes ([u8; 128])
    /// 
    /// **TOTAL: 211 bytes**
    pub const LEN: usize = 1 + 8 + 1 + 32 + 33 + 8 + 128;
    
    /// Creates a new SystemState in unpaused state with specified admin authority.
    /// 
    /// # Arguments
    /// * `admin_authority` - The initial admin authority pubkey
    /// 
    /// # Returns
    /// A new SystemState initialized in unpaused state (code 0)
    pub fn new(admin_authority: Pubkey) -> Self {
        Self {
            is_paused: false,
            pause_timestamp: 0,
            pause_reason_code: 0, // 0 = No pause active
            admin_authority,
            pending_admin_authority: None,
            admin_change_timestamp: 0,
            _reserved: [0; 128], // Initialize reserved space to zero
        }
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
    
    /// **ADMIN AUTHORITY MANAGEMENT WITH 72-HOUR TIMELOCK**
    
    /// Timelock duration in seconds (72 hours)
    pub const ADMIN_CHANGE_TIMELOCK: i64 = 72 * 60 * 60; // 259,200 seconds
    
    /// Processes admin authority change with automatic completion
    /// 
    /// This unified function handles both initiation and completion of admin changes:
    /// 1. If no change is pending or different admin proposed: starts 72-hour timer
    /// 2. If 72+ hours have passed and pending admin differs from current: completes change
    /// 3. If same admin proposed again: acts as cancellation (clears pending)
    /// 
    /// # Arguments
    /// * `new_admin` - The proposed new admin authority pubkey
    /// * `timestamp` - Current timestamp
    /// 
    /// # Returns
    /// * `Ok(AdminChangeResult)` - Indicates what action was taken
    pub fn process_admin_change(&mut self, new_admin: Pubkey, timestamp: i64) -> Result<AdminChangeResult, String> {
        // Case 1: Same as current admin - cancel any pending change
        if new_admin == self.admin_authority {
            if self.pending_admin_authority.is_some() {
                self.pending_admin_authority = None;
                self.admin_change_timestamp = 0;
                return Ok(AdminChangeResult::Cancelled);
            } else {
                return Ok(AdminChangeResult::NoChange);
            }
        }
        
        // Case 2: Check if we can complete a pending change
        if let Some(pending_admin) = self.pending_admin_authority {
            let time_elapsed = timestamp - self.admin_change_timestamp;
            
            if time_elapsed >= Self::ADMIN_CHANGE_TIMELOCK {
                // Timelock satisfied - complete the change if it's different from current
                if pending_admin != self.admin_authority {
                    let old_admin = self.admin_authority;
                    self.admin_authority = pending_admin;
                    self.pending_admin_authority = None;
                    self.admin_change_timestamp = 0;
                    return Ok(AdminChangeResult::Completed { old_admin, new_admin: pending_admin });
                } else {
                    // Pending admin same as current - just clear pending
                    self.pending_admin_authority = None;
                    self.admin_change_timestamp = 0;
                    return Ok(AdminChangeResult::Cancelled);
                }
            }
            
            // Timelock not satisfied yet - check if proposing different admin
            if pending_admin != new_admin {
                // Different admin proposed - reset timer
                self.pending_admin_authority = Some(new_admin);
                self.admin_change_timestamp = timestamp;
                return Ok(AdminChangeResult::Initiated { 
                    new_admin, 
                    previous_pending: Some(pending_admin) 
                });
            } else {
                // Same pending admin proposed again - no change
                let remaining = Self::ADMIN_CHANGE_TIMELOCK - time_elapsed;
                return Ok(AdminChangeResult::Pending { 
                    pending_admin, 
                    remaining_seconds: remaining 
                });
            }
        } else {
            // Case 3: No pending change - initiate new one
            self.pending_admin_authority = Some(new_admin);
            self.admin_change_timestamp = timestamp;
            return Ok(AdminChangeResult::Initiated { 
                new_admin, 
                previous_pending: None 
            });
        }
    }
    
    /// Checks if the given authority matches the current admin
    pub fn is_admin(&self, authority: &Pubkey) -> bool {
        self.admin_authority == *authority
    }
    
    /// Gets time remaining for pending admin change (0 if no change pending or ready)
    pub fn admin_change_time_remaining(&self, current_timestamp: i64) -> i64 {
        if self.pending_admin_authority.is_none() {
            return 0;
        }
        
        let elapsed = current_timestamp - self.admin_change_timestamp;
        if elapsed >= Self::ADMIN_CHANGE_TIMELOCK {
            return 0; // Ready to complete
        }
        
        Self::ADMIN_CHANGE_TIMELOCK - elapsed
    }
}

impl Default for SystemState {
    fn default() -> Self {
        // Use a placeholder pubkey for default - should be set properly during initialization
        Self::new(Pubkey::default())
    }
} 