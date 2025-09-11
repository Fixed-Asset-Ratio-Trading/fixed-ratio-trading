//! System-wide state management for global pause functionality
//!
//! This module contains the SystemState struct and related functionality for
//! managing system-wide operations like emergency pause/unpause.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

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
///   System-wide state that controls global operations for the entire contract.
/// 
/// This state is separate from individual pool states and provides emergency
/// controls that can override all pool operations when necessary.
/// Only the admin authority (stored in this state) can perform system-wide operations.
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
}

impl SystemState {
    /// Account space required for SystemState serialization
    /// 
    /// **UPDATED CALCULATION WITH ADMIN AUTHORITY SYSTEM**:
    /// - is_paused: 1 byte (bool)
    /// - pause_timestamp: 8 bytes (i64)
    /// - pause_reason_code: 1 byte (u8)
    /// - admin_authority: 32 bytes (Pubkey)
    /// - pending_admin_authority: 33 bytes (Option<Pubkey> = 1 + 32)
    /// - admin_change_timestamp: 8 bytes (i64)
    /// 
    /// **TOTAL: 83 bytes**
    pub const LEN: usize = 1 + 8 + 1 + 32 + 33 + 8; // 83 bytes - exact calculation
    
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

    /// **CENTRALIZED DESERIALIZATION** - Robust loading from account data
    /// 
    /// This method provides a single, reliable way to load SystemState from any account,
    /// handling size variations gracefully and including security validation.
    /// 
    /// **Benefits:**
    /// - Tolerant of account size changes (handles trailing bytes)
    /// - Validates PDA security automatically
    /// - Single point of maintenance for deserialization logic
    /// - Prevents "Not all bytes read" errors
    /// 
    /// # Arguments
    /// * `account` - The SystemState account to load from
    /// * `program_id` - Program ID for PDA validation
    /// 
    /// # Returns
    /// * `Result<SystemState, ProgramError>` - Loaded state or error
    /// 
    /// # Security
    /// Validates that the account is the correct SystemState PDA before deserializing
    pub fn load_from_account(
        account: &AccountInfo,
        program_id: &Pubkey,
    ) -> Result<Self, ProgramError> {
        // 🔒 SECURITY: Validate this is the correct SystemState PDA
        let (expected_system_state_pda, _) = Pubkey::find_program_address(
            &[crate::constants::SYSTEM_STATE_SEED_PREFIX], // b"system_state"
            program_id,
        );
        
        if *account.key != expected_system_state_pda {
            msg!("🚨 SECURITY: Invalid SystemState PDA provided");
            msg!("Expected: {}, Provided: {}", expected_system_state_pda, account.key);
            return Err(ProgramError::InvalidAccountData);
        }
        
        // 🔧 TOLERANT DESERIALIZATION: Handles account size variations
        let account_data = account.data.borrow();
        msg!("🔍 SystemState loading: account has {} bytes", account_data.len());
        
        // Handle empty/uninitialized accounts
        if account_data.is_empty() {
            msg!("❌ SystemState account is empty");
            return Err(ProgramError::UninitializedAccount);
        }
        
        // Deserialize with tolerance for trailing bytes
        Self::deserialize(&mut &account_data[..])
            .map_err(|e| {
                msg!("❌ SystemState deserialization failed: {:?}", e);
                ProgramError::InvalidAccountData
            })
    }

    /// **TEST-FRIENDLY DESERIALIZATION** - For use in test environments only
    /// 
    /// This method provides tolerant deserialization without PDA validation,
    /// suitable for test environments where account setup may vary.
    /// 
    /// **⚠️ WARNING: DO NOT USE IN PRODUCTION CODE**
    /// This method skips security validation and should only be used in tests.
    /// 
    /// # Arguments
    /// * `data` - Raw account data to deserialize
    /// 
    /// # Returns
    /// * `Result<SystemState, ProgramError>` - Loaded state or error
    pub fn from_account_data_unchecked(data: &[u8]) -> Result<Self, ProgramError> {
        if data.is_empty() {
            return Err(ProgramError::UninitializedAccount);
        }
        
        // Tolerant deserialization for test environments
        Self::deserialize(&mut &data[..])
            .map_err(|_| ProgramError::InvalidAccountData)
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
        // Check if we can complete a pending change (same pending admin after 72+ hours)
        if let Some(pending_admin) = self.pending_admin_authority {
            let time_elapsed = timestamp - self.admin_change_timestamp;
            
            if pending_admin == new_admin && time_elapsed >= Self::ADMIN_CHANGE_TIMELOCK {
                // Complete the change: same pending admin requested after timelock period
                if pending_admin != self.admin_authority {
                    let old_admin = self.admin_authority;
                    self.admin_authority = pending_admin;
                    self.pending_admin_authority = None;
                    self.admin_change_timestamp = 0;
                    return Ok(AdminChangeResult::Completed { old_admin, new_admin: pending_admin });
                } else {
                    // Pending admin same as current admin - just clear pending state
                    self.pending_admin_authority = None;
                    self.admin_change_timestamp = 0;
                    return Ok(AdminChangeResult::Cancelled);
                }
            }
        }
        
        // For all other cases: Set pending admin and reset timestamp
        // This includes:
        // - New admin change (no pending change)
        // - Different admin than pending (reset timer)  
        // - Same admin as current (still set pending state)
        // - Same pending admin but < 72 hours (reset timer)
        
        let previous_pending = self.pending_admin_authority;
        self.pending_admin_authority = Some(new_admin);
        self.admin_change_timestamp = timestamp;
        
        if previous_pending.is_some() && previous_pending != Some(new_admin) {
            // Different admin proposed - reset timer
            return Ok(AdminChangeResult::Initiated { 
                new_admin, 
                previous_pending 
            });
        } else if previous_pending.is_none() {
            // No previous pending change - initiate new one
            return Ok(AdminChangeResult::Initiated { 
                new_admin, 
                previous_pending: None 
            });
        } else {
            // Same pending admin proposed again but < 72 hours - reset timer
            let remaining = Self::ADMIN_CHANGE_TIMELOCK;
            return Ok(AdminChangeResult::Pending { 
                pending_admin: new_admin, 
                remaining_seconds: remaining 
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