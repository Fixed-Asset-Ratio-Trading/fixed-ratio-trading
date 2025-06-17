//! Pool State Types and Structures
//! 
//! This module contains all the core state structures for the trading pool,
//! including the main PoolState, delegate management, and related helper types.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    pubkey::Pubkey,
    sysvar::rent::Rent,
    program_pack::Pack,
};
use spl_token::state::{Account as TokenAccount, Mint as MintAccount};
use crate::error::PoolError;
use crate::constants::*;
use super::delegate_actions::{DelegateTimeLimits, PendingDelegateAction};

/// Enumerated reasons for pool pause requests.
/// 
/// This enum provides structured categorization of pause requests to enable
/// different governance and bonding mechanisms based on the type of issue.
/// Designed for integration with higher-layer governance contracts.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Default)]
pub enum PoolPauseReason {
    #[default]
    /// Dispute over the fixed ratio accuracy or fairness
    RatioDispute,
    /// Insufficient bonding by pool participants
    InsufficientBond,
    /// General security concern requiring investigation
    SecurityConcern,
    /// Governance action or proposal execution
    GovernanceAction,
    /// Manual intervention by authorized delegate
    ManualIntervention,
    /// Emergency response to detected issues
    Emergency,
}

/// Individual pool pause request structure.
/// 
/// Represents a delegate's request to pause pool operations for a specific duration.
/// Designed as a primitive for governance contracts to implement sophisticated
/// dispute resolution, bonding mechanisms, and automated pool management.
/// 
/// # Design Principles:
/// - Separate timing from withdrawal requests for independent governance
/// - Owner cancellation capability for emergency resolution
/// - Structured reasons for automated governance integration
/// - Maximum 72-hour duration to prevent indefinite pausing
/// 
/// # Timing Model:
/// - Request submitted at `request_timestamp`
/// - Becomes active after `wait_time` seconds (1 minute to 72 hours)
/// - Remains active for `duration_seconds` (1 minute to 72 hours)
/// - Can be cancelled by delegate or owner before activation
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, Default)]
pub struct PoolPauseRequest {
    /// Delegate who submitted the pause request
    pub delegate: Pubkey,
    /// Structured reason for the pause request
    pub reason: PoolPauseReason,
    /// Timestamp when the request was submitted (Unix timestamp)
    pub request_timestamp: i64,
    /// Solana slot when the request was submitted (for audit trails)
    pub request_slot: u64,
    /// Wait time before pause becomes active (60 to 259200 seconds)
    pub wait_time: u64,
    /// Duration of the pause once active (60 to 259200 seconds)
    pub duration_seconds: u64,
}

impl PoolPauseRequest {
    /// Create a new pool pause request with validation.
    /// 
    /// # Arguments:
    /// * `delegate` - Pubkey of the requesting delegate
    /// * `reason` - Structured reason for the pause
    /// * `request_timestamp` - Current Unix timestamp
    /// * `request_slot` - Current Solana slot
    /// * `wait_time` - Delay before pause activation (60-259200 seconds)
    /// * `duration_seconds` - Duration of pause (60-259200 seconds)
    /// 
    /// # Validation:
    /// - Wait time must be between 1 minute and 72 hours
    /// - Duration must be between 1 minute and 72 hours
    pub fn new(
        delegate: Pubkey,
        reason: PoolPauseReason,
        request_timestamp: i64,
        request_slot: u64,
        wait_time: u64,
        duration_seconds: u64,
    ) -> Result<Self, crate::PoolError> {
        // Validate wait time (1 minute to 72 hours)
        if wait_time < 60 || wait_time > 259200 {
            return Err(crate::PoolError::InvalidWaitTime { wait_time });
        }
        
        // Validate duration (1 minute to 72 hours)
        if duration_seconds < 60 || duration_seconds > 259200 {
            return Err(crate::PoolError::InvalidWaitTime { wait_time: duration_seconds });
        }

        Ok(Self {
            delegate,
            reason,
            request_timestamp,
            request_slot,
            wait_time,
            duration_seconds,
        })
    }

    pub fn get_packed_len() -> usize {
        32 + // delegate (Pubkey)
        1 +  // reason (PoolPauseReason enum)
        8 +  // request_timestamp (i64)
        8 +  // request_slot (u64)
        8 +  // wait_time (u64)
        8    // duration_seconds (u64)
    }
    
    /// Check if the pause request is ready to become active.
    /// 
    /// # Arguments:
    /// * `current_timestamp` - Current Unix timestamp for comparison
    /// 
    /// # Returns:
    /// - `true` if enough time has passed since request submission
    /// - `false` if still within the wait period
    pub fn is_ready_to_activate(&self, current_timestamp: i64) -> bool {
        current_timestamp >= self.request_timestamp + self.wait_time as i64
    }
    
    /// Check if the pause is currently active.
    /// 
    /// # Arguments:
    /// * `current_timestamp` - Current Unix timestamp for comparison
    /// 
    /// # Returns:
    /// - `true` if pause is active (past wait time, within duration)
    /// - `false` if pause hasn't started or has expired
    pub fn is_active(&self, current_timestamp: i64) -> bool {
        let activation_time = self.request_timestamp + self.wait_time as i64;
        let expiration_time = activation_time + self.duration_seconds as i64;
        
        current_timestamp >= activation_time && current_timestamp < expiration_time
    }
    
    /// Check if the pause has expired.
    /// 
    /// # Arguments:
    /// * `current_timestamp` - Current Unix timestamp for comparison
    /// 
    /// # Returns:
    /// - `true` if pause duration has fully elapsed
    /// - `false` if pause is still pending or active
    pub fn is_expired(&self, current_timestamp: i64) -> bool {
        let expiration_time = self.request_timestamp + self.wait_time as i64 + self.duration_seconds as i64;
        current_timestamp >= expiration_time
    }
}

/// Represents a withdrawal request with time delay for enhanced security.
#[derive(BorshSerialize, BorshDeserialize, Debug, Default, Clone, Copy)]
pub struct WithdrawalRequest {
    pub delegate: Pubkey,
    pub token_mint: Pubkey,
    pub amount: u64,
    pub request_timestamp: i64,
    pub request_slot: u64,
    pub wait_time: u64, // Wait time in seconds
}

impl WithdrawalRequest {
    pub fn new(delegate: Pubkey, token_mint: Pubkey, amount: u64, request_timestamp: i64, request_slot: u64, wait_time: u64) -> Self {
        Self {
            delegate,
            token_mint,
            amount,
            request_timestamp,
            request_slot,
            wait_time,
        }
    }

    pub fn get_packed_len() -> usize {
        32 + // delegate
        32 + // token_mint
        8 +  // amount
        8 +  // request_timestamp
        8 +  // request_slot
        8    // wait_time
    }
}

/// Manages delegate authorization, actions, and time limits
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct DelegateManagement {
    /// Array of authorized delegates
    pub delegates: [Pubkey; MAX_DELEGATES],
    /// Number of active delegates
    pub delegate_count: u8,
    /// Time limits for different actions per delegate
    pub time_limits: [DelegateTimeLimits; MAX_DELEGATES],
    /// Pending actions from delegates
    pub pending_actions: Vec<PendingDelegateAction>, // Allow multiple actions per delegate
    /// Number of pending actions
    pub pending_action_count: u8,
    /// Next action ID to assign
    pub next_action_id: u64,
    /// History of completed actions
    pub action_history: Vec<PendingDelegateAction>, // Keep last 10 completed actions
    /// Index for circular action history buffer
    pub action_history_index: u8,
}

impl Default for DelegateManagement {
    fn default() -> Self {
        Self {
            delegates: [Pubkey::default(); MAX_DELEGATES],
            delegate_count: 0,
            time_limits: [DelegateTimeLimits::default(); MAX_DELEGATES],
            pending_actions: Vec::with_capacity(MAX_DELEGATES * 2),
            pending_action_count: 0,
            next_action_id: 1,
            action_history: Vec::with_capacity(10),
            action_history_index: 0,
        }
    }
}

impl DelegateManagement {
    pub fn new(owner: Pubkey, _current_slot: u64) -> Self {
        let mut delegates = [Pubkey::default(); MAX_DELEGATES];
        delegates[0] = owner; // Owner is the first delegate
        
        Self {
            delegates,
            delegate_count: 1,
            time_limits: [DelegateTimeLimits::default(); MAX_DELEGATES],
            pending_actions: Vec::with_capacity(MAX_DELEGATES * 2),
            pending_action_count: 0,
            next_action_id: 1,
            action_history: Vec::with_capacity(10),
            action_history_index: 0,
        }
    }

    pub fn get_delegate_index(&self, pubkey: &Pubkey) -> Option<usize> {
        for i in 0..self.delegate_count as usize {
            if self.delegates[i] == *pubkey {
                return Some(i);
            }
        }
        None
    }

    pub fn is_delegate(&self, pubkey: &Pubkey) -> bool {
        self.get_delegate_index(pubkey).is_some()
    }

    pub fn add_delegate(&mut self, delegate: Pubkey) -> Result<(), PoolError> {
        if self.delegate_count as usize >= MAX_DELEGATES {
            return Err(PoolError::DelegateLimitExceeded);
        }

        // Check if already a delegate
        if self.is_delegate(&delegate) {
            return Err(PoolError::DelegateAlreadyExists { delegate });
        }

        self.delegates[self.delegate_count as usize] = delegate;
        self.delegate_count += 1;
        Ok(())
    }

    pub fn remove_delegate(&mut self, delegate: Pubkey) -> Result<(), PoolError> {
        let mut found_index = None;
        for i in 0..self.delegate_count as usize {
            if self.delegates[i] == delegate {
                found_index = Some(i);
                break;
            }
        }

        if let Some(index) = found_index {
            // Shift remaining delegates
            for i in index..(self.delegate_count as usize - 1) {
                self.delegates[i] = self.delegates[i + 1];
                self.time_limits[i] = self.time_limits[i + 1];
            }
            self.delegates[self.delegate_count as usize - 1] = Pubkey::default();
            self.time_limits[self.delegate_count as usize - 1] = DelegateTimeLimits::default();
            self.delegate_count -= 1;
            Ok(())
        } else {
            Err(PoolError::DelegateNotFound { delegate })
        }
    }

    pub fn add_pending_action(
        &mut self,
        action: PendingDelegateAction,
    ) -> Result<u64, PoolError> {
        if self.pending_action_count as usize >= MAX_DELEGATES * 2 {
            return Err(PoolError::TooManyPendingActions);
        }

        // Assign next action ID and increment
        let action_id = self.next_action_id;
        self.next_action_id = self.next_action_id.checked_add(1)
            .ok_or(PoolError::ArithmeticOverflow)?;

        // Store the action
        self.pending_actions.push(action);
        self.pending_action_count += 1;

        Ok(action_id)
    }

    pub fn get_pending_action(&self, action_id: u64) -> Option<&PendingDelegateAction> {
        self.pending_actions.iter()
            .find(|action| action.action_id == action_id)
    }

    pub fn remove_pending_action(&mut self, action_id: u64) -> Result<PendingDelegateAction, PoolError> {
        let position = self.pending_actions.iter()
            .position(|action| action.action_id == action_id)
            .ok_or(PoolError::ActionNotFound)?;

        // Remove and return the action
        let action = self.pending_actions.remove(position);
        self.pending_action_count -= 1;

        // Add to history
        self.add_to_history(action.clone());

        Ok(action)
    }

    fn add_to_history(&mut self, action: PendingDelegateAction) {
        if self.action_history.len() >= 10 {
            self.action_history.remove(0);
        }
        self.action_history.push(action);
    }

    pub fn get_delegate_time_limits(&self, delegate: &Pubkey) -> Option<&DelegateTimeLimits> {
        self.get_delegate_index(delegate)
            .map(|index| &self.time_limits[index])
    }

    pub fn set_delegate_time_limits(
        &mut self,
        delegate: &Pubkey,
        new_limits: DelegateTimeLimits,
    ) -> Result<(), PoolError> {
        let index = self.get_delegate_index(delegate)
            .ok_or(PoolError::DelegateNotFound { delegate: *delegate })?;
        
        self.time_limits[index] = new_limits;
        Ok(())
    }

    /// Calculate the maximum packed length the structure may occupy when
    /// serialized.  For on-chain accounts we must allocate enough space for
    /// the *largest* possible variant so that the account does not need to be
    /// re-allocated later (which Solana does not allow).
    ///
    /// Capacity assumptions:
    /// * `MAX_DELEGATES` is the hard limit for delegates (currently 3).
    /// * Each delegate may have **two** concurrent pending actions, giving
    ///   `MAX_PENDING_ACTIONS = MAX_DELEGATES * 2`.
    /// * A circular history keeps the **last 10** completed actions.
    ///
    /// These values are aligned with the expectations of the test-suite and
    /// governance design docs.
    pub fn get_packed_len() -> usize {
        // Fixed-size fields --------------------------------------------------
        let delegates_size = 32 * MAX_DELEGATES;        // [Pubkey; MAX_DELEGATES]
        let delegate_count_size = 1;                    // u8
        let time_limits_size = 24 * MAX_DELEGATES;      // 3 * u64 per delegate
        let pending_action_count_size = 1;              // u8
        let next_action_id_size = 8;                    // u64
        let action_history_index_size = 1;              // u8

        // Variable-length fields -------------------------------------------
        // For vectors we include 4-byte length prefix plus the maximum number
        // of elements we plan to store.
        const MAX_PENDING_ACTIONS_PER_DELEGATE: usize = 2;
        const ACTION_HISTORY_CAPACITY: usize = 10;

        let max_pending_actions = MAX_DELEGATES * MAX_PENDING_ACTIONS_PER_DELEGATE;
        let pending_actions_size = 4 + PendingDelegateAction::get_packed_len() * max_pending_actions;

        let action_history_size = 4 + PendingDelegateAction::get_packed_len() * ACTION_HISTORY_CAPACITY;

        delegates_size
            + delegate_count_size
            + time_limits_size
            + pending_actions_size
            + pending_action_count_size
            + next_action_id_size
            + action_history_size
            + action_history_index_size
    }
}

/// Tracks rent requirements for pool accounts to ensure rent exemption.
#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
pub struct RentRequirements {
    pub last_update_slot: u64,
    pub rent_exempt_minimum: u64,
    pub pool_state_rent: u64,
    pub token_vault_rent: u64,
    pub lp_mint_rent: u64,
}

impl RentRequirements {
    pub fn new(rent: &Rent) -> Self {
        Self {
            last_update_slot: 0,
            rent_exempt_minimum: rent.minimum_balance(0),
            pool_state_rent: rent.minimum_balance(PoolState::get_packed_len()),
            token_vault_rent: rent.minimum_balance(TokenAccount::LEN),
            lp_mint_rent: rent.minimum_balance(MintAccount::LEN),
        }
    }

    pub fn update_if_needed(&mut self, rent: &Rent, current_slot: u64) -> bool {
        // Update rent requirements if they've changed or if it's been a while
        let needs_update = self.last_update_slot == 0 || 
                          current_slot - self.last_update_slot > 1000 || // Update every ~1000 slots
                          self.pool_state_rent != rent.minimum_balance(PoolState::get_packed_len()) ||
                          self.token_vault_rent != rent.minimum_balance(TokenAccount::LEN) ||
                          self.lp_mint_rent != rent.minimum_balance(MintAccount::LEN);

        if needs_update {
            self.pool_state_rent = rent.minimum_balance(PoolState::get_packed_len());
            self.token_vault_rent = rent.minimum_balance(TokenAccount::LEN);
            self.lp_mint_rent = rent.minimum_balance(MintAccount::LEN);
            self.last_update_slot = current_slot;
        }

        needs_update
    }

    pub fn get_total_required_rent(&self) -> u64 {
        self.pool_state_rent + 
        (2 * self.token_vault_rent) + // Two token vaults
        (2 * self.lp_mint_rent) + // Two LP mints
        MINIMUM_RENT_BUFFER // Additional buffer
    }

    pub fn get_packed_len() -> usize {
        8 + // last_update_slot
        8 + // rent_exempt_minimum
        8 + // pool_state_rent
        8 + // token_vault_rent
        8   // lp_mint_rent
    }
}

/// Main pool state containing all configuration and runtime data.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct PoolState {
    pub owner: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub token_a_vault: Pubkey,
    pub token_b_vault: Pubkey,
    pub lp_token_a_mint: Pubkey,
    pub lp_token_b_mint: Pubkey,
    pub ratio_a_numerator: u64,
    pub ratio_b_denominator: u64,
    pub total_token_a_liquidity: u64,
    pub total_token_b_liquidity: u64,
    pub pool_authority_bump_seed: u8,
    pub token_a_vault_bump_seed: u8,
    pub token_b_vault_bump_seed: u8,
    pub is_initialized: bool,
    pub rent_requirements: RentRequirements,
    pub is_paused: bool,
    pub pause_end_timestamp: i64,  // Unix timestamp when pause ends (0 if not paused)
    pub pause_reason: PoolPauseReason,  // Reason for current pause
    pub delegate_management: DelegateManagement,
    pub collected_fees_token_a: u64,
    pub collected_fees_token_b: u64,
    pub total_fees_withdrawn_token_a: u64,
    pub total_fees_withdrawn_token_b: u64,
    pub swap_fee_basis_points: u64,
    pub collected_sol_fees: u64,
    pub total_sol_fees_withdrawn: u64,
}

impl Default for PoolState {
    fn default() -> Self {
        Self {
            owner: Pubkey::default(),
            token_a_mint: Pubkey::default(),
            token_b_mint: Pubkey::default(),
            token_a_vault: Pubkey::default(),
            token_b_vault: Pubkey::default(),
            lp_token_a_mint: Pubkey::default(),
            lp_token_b_mint: Pubkey::default(),
            ratio_a_numerator: 0,
            ratio_b_denominator: 0,
            total_token_a_liquidity: 0,
            total_token_b_liquidity: 0,
            pool_authority_bump_seed: 0,
            token_a_vault_bump_seed: 0,
            token_b_vault_bump_seed: 0,
            is_initialized: false,
            rent_requirements: RentRequirements::default(),
            is_paused: false,
            pause_end_timestamp: 0,
            pause_reason: PoolPauseReason::default(),
            delegate_management: DelegateManagement::default(),
            collected_fees_token_a: 0,
            collected_fees_token_b: 0,
            total_fees_withdrawn_token_a: 0,
            total_fees_withdrawn_token_b: 0,
            swap_fee_basis_points: 0,
            collected_sol_fees: 0,
            total_sol_fees_withdrawn: 0,
        }
    }
}

impl PoolState {
    pub fn get_packed_len() -> usize {
        32 + // owner
        32 + // token_a_mint
        32 + // token_b_mint
        32 + // token_a_vault
        32 + // token_b_vault
        32 + // lp_token_a_mint
        32 + // lp_token_b_mint
        8 +  // ratio_a_numerator
        8 +  // ratio_b_denominator
        8 +  // total_token_a_liquidity
        8 +  // total_token_b_liquidity
        1 +  // pool_authority_bump_seed
        1 +  // token_a_vault_bump_seed
        1 +  // token_b_vault_bump_seed
        1 +  // is_initialized
        RentRequirements::get_packed_len() + // rent_requirements
        1 +  // is_paused
        8 +  // pause_end_timestamp
        1 +  // pause_reason (enum)
        DelegateManagement::get_packed_len() + // delegate_management
        8 +  // collected_fees_token_a
        8 +  // collected_fees_token_b
        8 +  // total_fees_withdrawn_token_a
        8 +  // total_fees_withdrawn_token_b
        8 +  // swap_fee_basis_points
        8 +  // collected_sol_fees
        8    // total_sol_fees_withdrawn
    }
} 