//! Pool State Types and Structures
//! 
//! This module contains all the core state structures for the trading pool,
//! including the main PoolState, delegate management, and related helper types.

use crate::constants::*;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    pubkey::Pubkey,
    rent::Rent,
    program_pack::Pack,
};
use spl_token::state::{Account as TokenAccount, Mint as MintAccount};

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

/// Represents a completed withdrawal for audit tracking.
#[derive(BorshSerialize, BorshDeserialize, Debug, Default, Clone, Copy)]
pub struct WithdrawalRecord {
    pub delegate: Pubkey,
    pub token_mint: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
    pub slot: u64,
}

impl WithdrawalRecord {
    pub fn new(delegate: Pubkey, token_mint: Pubkey, amount: u64, timestamp: i64, slot: u64) -> Self {
        Self {
            delegate,
            token_mint,
            amount,
            timestamp,
            slot,
        }
    }

    pub fn get_packed_len() -> usize {
        32 + // delegate
        32 + // token_mint
        8 +  // amount
        8 +  // timestamp
        8    // slot
    }
}

/// Manages delegate authorization, fee withdrawals, and pool pause requests.
#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
pub struct DelegateManagement {
    pub delegates: [Pubkey; MAX_DELEGATES],
    pub delegate_count: u8,
    pub withdrawal_history: [WithdrawalRecord; 10], // Last 10 withdrawals
    pub withdrawal_history_index: u8,
    pub withdrawal_requests: [WithdrawalRequest; MAX_DELEGATES], // One request per delegate
    pub delegate_wait_times: [u64; MAX_DELEGATES], // Wait time in seconds for each delegate
    pub pool_pause_requests: [PoolPauseRequest; MAX_DELEGATES], // One pause request per delegate
    pub pool_pause_wait_times: [u64; MAX_DELEGATES], // Pool pause wait time in seconds for each delegate (default 72 hours)
}

impl DelegateManagement {
    pub fn new(owner: Pubkey, _current_slot: u64) -> Self {
        let mut delegates = [Pubkey::default(); MAX_DELEGATES];
        delegates[0] = owner; // Owner is the first delegate
        
        Self {
            delegates,
            delegate_count: 1,
            withdrawal_history: [WithdrawalRecord::default(); 10],
            withdrawal_history_index: 0,
            withdrawal_requests: [WithdrawalRequest::default(); MAX_DELEGATES],
            delegate_wait_times: [MIN_WITHDRAWAL_WAIT_TIME; MAX_DELEGATES], // Default to minimum wait time for fee withdrawals
            pool_pause_requests: [PoolPauseRequest::default(); MAX_DELEGATES], // No pending pause requests initially
            pool_pause_wait_times: [259200; MAX_DELEGATES], // Default 72 hours for pool pausing (more deliberation time)
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

    pub fn add_delegate(&mut self, delegate: Pubkey) -> Result<(), crate::PoolError> {
        if self.delegate_count as usize >= MAX_DELEGATES {
            return Err(crate::PoolError::DelegateLimitExceeded);
        }

        // Check if already a delegate
        if self.is_delegate(&delegate) {
            return Err(crate::PoolError::DelegateAlreadyExists { delegate });
        }

        self.delegates[self.delegate_count as usize] = delegate;
        self.delegate_count += 1;
        Ok(())
    }

    pub fn remove_delegate(&mut self, delegate: Pubkey) -> Result<(), crate::PoolError> {
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
                self.withdrawal_requests[i] = self.withdrawal_requests[i + 1];
                self.delegate_wait_times[i] = self.delegate_wait_times[i + 1];
            }
            self.delegates[self.delegate_count as usize - 1] = Pubkey::default();
            self.withdrawal_requests[self.delegate_count as usize - 1] = WithdrawalRequest::default();
            self.delegate_wait_times[self.delegate_count as usize - 1] = MIN_WITHDRAWAL_WAIT_TIME;
            self.delegate_count -= 1;
            Ok(())
        } else {
            Err(crate::PoolError::DelegateNotFound { delegate })
        }
    }

    pub fn add_withdrawal_record(&mut self, record: WithdrawalRecord) {
        let index = self.withdrawal_history_index as usize;
        self.withdrawal_history[index] = record;
        self.withdrawal_history_index = (self.withdrawal_history_index + 1) % 10;
    }

    pub fn get_packed_len() -> usize {
        // Use exact calculation - Borsh serializes structs precisely
        (32 * MAX_DELEGATES) + // delegates: [Pubkey; MAX_DELEGATES]
        1 +  // delegate_count: u8
        (WithdrawalRecord::get_packed_len() * 10) + // withdrawal_history: [WithdrawalRecord; 10]
        1 +  // withdrawal_history_index: u8
        (WithdrawalRequest::get_packed_len() * MAX_DELEGATES) + // withdrawal_requests: [WithdrawalRequest; MAX_DELEGATES]
        (8 * MAX_DELEGATES) + // delegate_wait_times: [u64; MAX_DELEGATES]
        (PoolPauseRequest::get_packed_len() * MAX_DELEGATES) + // pool_pause_requests: [PoolPauseRequest; MAX_DELEGATES]
        (8 * MAX_DELEGATES) // pool_pause_wait_times: [u64; MAX_DELEGATES]
    }

    pub fn set_delegate_wait_time(&mut self, delegate: &Pubkey, wait_time: u64) -> Result<(), crate::PoolError> {
        if wait_time < MIN_WITHDRAWAL_WAIT_TIME || wait_time > MAX_WITHDRAWAL_WAIT_TIME {
            return Err(crate::PoolError::InvalidWaitTime { wait_time });
        }

        if let Some(index) = self.get_delegate_index(delegate) {
            self.delegate_wait_times[index] = wait_time;
            Ok(())
        } else {
            Err(crate::PoolError::DelegateNotFound { delegate: *delegate })
        }
    }

    pub fn get_delegate_wait_time(&self, delegate: &Pubkey) -> Option<u64> {
        self.get_delegate_index(delegate).map(|index| self.delegate_wait_times[index])
    }

    pub fn create_withdrawal_request(&mut self, delegate: &Pubkey, token_mint: Pubkey, amount: u64, timestamp: i64, slot: u64) -> Result<(), crate::PoolError> {
        if let Some(index) = self.get_delegate_index(delegate) {
            // Check if there's already a pending request
            if self.withdrawal_requests[index].delegate != Pubkey::default() {
                return Err(crate::PoolError::PendingWithdrawalExists);
            }

            let wait_time = self.delegate_wait_times[index];
            self.withdrawal_requests[index] = WithdrawalRequest::new(
                *delegate,
                token_mint,
                amount,
                timestamp,
                slot,
                wait_time,
            );
            Ok(())
        } else {
            Err(crate::PoolError::DelegateNotFound { delegate: *delegate })
        }
    }

    pub fn cancel_withdrawal_request(&mut self, delegate: &Pubkey) -> Result<(), crate::PoolError> {
        if let Some(index) = self.get_delegate_index(delegate) {
            self.withdrawal_requests[index] = WithdrawalRequest::default();
            Ok(())
        } else {
            Err(crate::PoolError::DelegateNotFound { delegate: *delegate })
        }
    }

    pub fn get_withdrawal_request(&self, delegate: &Pubkey) -> Option<&WithdrawalRequest> {
        self.get_delegate_index(delegate).map(|index| &self.withdrawal_requests[index])
    }

    pub fn is_withdrawal_ready(&self, delegate: &Pubkey, current_timestamp: i64) -> Result<bool, crate::PoolError> {
        if let Some(request) = self.get_withdrawal_request(delegate) {
            if request.delegate == Pubkey::default() {
                return Err(crate::PoolError::NoPendingWithdrawal);
            }

            let elapsed_time = current_timestamp - request.request_timestamp;
            Ok(elapsed_time >= request.wait_time as i64)
        } else {
            Err(crate::PoolError::DelegateNotFound { delegate: *delegate })
        }
    }
    
    // **POOL PAUSE REQUEST MANAGEMENT METHODS**
    
    /// Set pool pause wait time for a specific delegate.
    /// 
    /// Configures the delay period between when a delegate requests a pool pause
    /// and when it becomes effective. This is separate from withdrawal wait times
    /// to allow independent governance parameter tuning.
    /// 
    /// # Arguments:
    /// * `delegate` - The delegate's public key
    /// * `wait_time` - Wait time in seconds (60 to 259200 = 1 minute to 72 hours)
    /// 
    /// # Returns:
    /// - `Ok(())` if successful
    /// - `PoolError::InvalidWaitTime` if wait time is out of range
    /// - `PoolError::DelegateNotFound` if delegate is not authorized
    pub fn set_pool_pause_wait_time(&mut self, delegate: &Pubkey, wait_time: u64) -> Result<(), crate::PoolError> {
        // Validate wait time (1 minute to 72 hours)
        if wait_time < 60 || wait_time > 259200 {
            return Err(crate::PoolError::InvalidWaitTime { wait_time });
        }

        if let Some(index) = self.get_delegate_index(delegate) {
            self.pool_pause_wait_times[index] = wait_time;
            Ok(())
        } else {
            Err(crate::PoolError::DelegateNotFound { delegate: *delegate })
        }
    }
    
    /// Get pool pause wait time for a specific delegate.
    /// 
    /// # Arguments:
    /// * `delegate` - The delegate's public key
    /// 
    /// # Returns:
    /// - `Some(wait_time)` if delegate exists
    /// - `None` if delegate is not found
    pub fn get_pool_pause_wait_time(&self, delegate: &Pubkey) -> Option<u64> {
        self.get_delegate_index(delegate).map(|index| self.pool_pause_wait_times[index])
    }
    
    /// Create a pool pause request for a specific delegate.
    /// 
    /// Submits a request to pause pool operations for a delegate-defined duration.
    /// The pause will become active after the delegate's configured wait time.
    /// 
    /// # Arguments:
    /// * `delegate` - The requesting delegate's public key
    /// * `reason` - Structured reason for the pause request
    /// * `duration_seconds` - Duration of pause once active (60 to 259200 seconds)
    /// * `timestamp` - Current Unix timestamp
    /// * `slot` - Current Solana slot for audit trails
    /// 
    /// # Returns:
    /// - `Ok(())` if successful
    /// - `PoolError::DelegateNotFound` if delegate is not authorized
    /// - `PoolError::PendingWithdrawalExists` if delegate already has active pause request
    /// - `PoolError::InvalidWaitTime` if duration is out of range
    pub fn create_pool_pause_request(
        &mut self, 
        delegate: &Pubkey, 
        reason: PoolPauseReason,
        duration_seconds: u64,
        timestamp: i64, 
        slot: u64
    ) -> Result<(), crate::PoolError> {
        if let Some(index) = self.get_delegate_index(delegate) {
            // Check if there's already a pending request (delegate != default means active request)
            if self.pool_pause_requests[index].delegate != Pubkey::default() {
                return Err(crate::PoolError::PendingWithdrawalExists);
            }

            let wait_time = self.pool_pause_wait_times[index];
            let pause_request = PoolPauseRequest::new(
                *delegate,
                reason,
                timestamp,
                slot,
                wait_time,
                duration_seconds,
            )?;
            
            self.pool_pause_requests[index] = pause_request;
            Ok(())
        } else {
            Err(crate::PoolError::DelegateNotFound { delegate: *delegate })
        }
    }
    
    /// Cancel a pending pool pause request for a specific delegate.
    /// 
    /// Removes a pool pause request before it becomes active. Can be called by
    /// the requesting delegate or the pool owner.
    /// 
    /// # Arguments:
    /// * `delegate` - The delegate's public key
    /// 
    /// # Returns:
    /// - `Ok(())` if successful
    /// - `PoolError::DelegateNotFound` if delegate is not authorized
    /// - `PoolError::NoPendingWithdrawal` if no pause request exists
    pub fn cancel_pool_pause_request(&mut self, delegate: &Pubkey) -> Result<(), crate::PoolError> {
        if let Some(index) = self.get_delegate_index(delegate) {
            if self.pool_pause_requests[index].delegate == Pubkey::default() {
                return Err(crate::PoolError::NoPendingWithdrawal);
            }
            
            self.pool_pause_requests[index] = PoolPauseRequest::default();
            Ok(())
        } else {
            Err(crate::PoolError::DelegateNotFound { delegate: *delegate })
        }
    }
    
    /// Get the pool pause request for a specific delegate.
    /// 
    /// # Arguments:
    /// * `delegate` - The delegate's public key
    /// 
    /// # Returns:
    /// - `Some(&PoolPauseRequest)` if a request exists
    /// - `None` if no request exists or delegate not found
    pub fn get_pool_pause_request(&self, delegate: &Pubkey) -> Option<&PoolPauseRequest> {
        self.get_delegate_index(delegate)
            .and_then(|index| {
                if self.pool_pause_requests[index].delegate != Pubkey::default() {
                    Some(&self.pool_pause_requests[index])
                } else {
                    None
                }
            })
    }
    
    /// Check if any pool pause is currently active.
    /// 
    /// Iterates through all delegate pause requests to determine if any
    /// pause is currently in effect. This is used to enforce pool pausing.
    /// 
    /// # Arguments:
    /// * `current_timestamp` - Current Unix timestamp for comparison
    /// 
    /// # Returns:
    /// - `true` if any delegate has an active pause
    /// - `false` if no pauses are currently active
    pub fn is_pool_paused_by_delegates(&self, current_timestamp: i64) -> bool {
        for i in 0..self.delegate_count as usize {
            let request = &self.pool_pause_requests[i];
            if request.delegate != Pubkey::default() && request.is_active(current_timestamp) {
                return true;
            }
        }
        false
    }
    
    /// Get information about the currently active pool pause, if any.
    /// 
    /// Returns details about the first active pool pause found, including
    /// the delegate responsible and the reason for the pause.
    /// 
    /// # Arguments:
    /// * `current_timestamp` - Current Unix timestamp for comparison
    /// 
    /// # Returns:
    /// - `Some((delegate, reason))` if a pause is active
    /// - `None` if no pause is currently active
    pub fn get_active_pool_pause_info(&self, current_timestamp: i64) -> Option<(Pubkey, PoolPauseReason)> {
        for i in 0..self.delegate_count as usize {
            let request = &self.pool_pause_requests[i];
            if request.delegate != Pubkey::default() && request.is_active(current_timestamp) {
                return Some((request.delegate, request.reason.clone()));
            }
        }
        None
    }
    
    /// Clean up expired pool pause requests.
    /// 
    /// Removes pause requests that have expired to keep the state clean.
    /// Should be called periodically to prevent state bloat.
    /// 
    /// # Arguments:
    /// * `current_timestamp` - Current Unix timestamp for comparison
    /// 
    /// # Returns:
    /// - Number of expired requests cleaned up
    pub fn cleanup_expired_pool_pause_requests(&mut self, current_timestamp: i64) -> u8 {
        let mut cleaned_count = 0;
        
        for i in 0..self.delegate_count as usize {
            let request = &self.pool_pause_requests[i];
            if request.delegate != Pubkey::default() && request.is_expired(current_timestamp) {
                self.pool_pause_requests[i] = PoolPauseRequest::default();
                cleaned_count += 1;
            }
        }
        
        cleaned_count
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
#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
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
    pub delegate_management: DelegateManagement,
    pub collected_fees_token_a: u64,
    pub collected_fees_token_b: u64,
    pub total_fees_withdrawn_token_a: u64,
    pub total_fees_withdrawn_token_b: u64,
    pub swap_fee_basis_points: u64, // Fee in basis points (0-50, representing 0%-0.5%)
    pub collected_sol_fees: u64, // Track collected SOL fees
    pub total_sol_fees_withdrawn: u64, // Track total SOL fees withdrawn
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