use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Types of actions that delegates can request
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq)]
pub enum DelegateActionType {
    /// Change the swap fee rate
    FeeChange,
    /// Withdraw accumulated fees
    Withdrawal,
    /// Pause pool operations
    PoolPause,
}

impl Default for DelegateActionType {
    fn default() -> Self {
        Self::FeeChange
    }
}

/// Parameters for different delegate actions
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum DelegateActionParams {
    /// Fee change parameters
    FeeChange {
        /// New fee in basis points (0-50 = 0%-0.5%)
        new_fee_basis_points: u64,
    },
    /// Withdrawal parameters
    Withdrawal {
        /// Token mint to withdraw
        token_mint: Pubkey,
        /// Amount to withdraw
        amount: u64,
    },
    /// Pool pause parameters
    PoolPause {
        /// Duration of the pause in seconds
        duration: u64,
        /// Reason for pausing
        reason: PauseReason,
    },
}

impl Default for DelegateActionParams {
    fn default() -> Self {
        Self::FeeChange {
            new_fee_basis_points: 0,
        }
    }
}

/// Reasons for pausing the pool
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Default)]
pub enum PauseReason {
    #[default]
    /// Dispute over the fixed ratio accuracy or fairness
    RatioDispute,
    /// Security concern requiring investigation
    SecurityConcern,
    /// Governance action or proposal execution
    GovernanceAction,
    /// Manual intervention by authorized delegate
    ManualIntervention,
    /// Emergency response to detected issues
    Emergency,
}

/// Represents a pending delegate action
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Default)]
pub struct PendingDelegateAction {
    /// The delegate requesting the action
    pub delegate: Pubkey,
    /// Type of action being requested
    pub action_type: DelegateActionType,
    /// Unix timestamp when the request was made
    pub request_timestamp: i64,
    /// Unix timestamp when the action can be executed
    pub execution_timestamp: i64,
    /// Action-specific parameters
    pub params: DelegateActionParams,
    /// Unique identifier for the action
    pub action_id: u64,
}

/// Time limits for different delegate actions
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy)]
pub struct DelegateTimeLimits {
    /// Wait time for fee changes (in seconds)
    pub fee_change_wait_time: u64,
    /// Wait time for withdrawals (in seconds)
    pub withdraw_wait_time: u64,
    /// Wait time for pool pausing (in seconds)
    pub pause_wait_time: u64,
}

impl Default for DelegateTimeLimits {
    fn default() -> Self {
        Self {
            // Default all wait times to 72 hours (259200 seconds)
            fee_change_wait_time: 259200,
            withdraw_wait_time: 259200,
            pause_wait_time: 259200,
        }
    }
}

impl PendingDelegateAction {
    /// Creates a new pending delegate action
    pub fn new(
        delegate: Pubkey,
        action_type: DelegateActionType,
        params: DelegateActionParams,
        request_timestamp: i64,
        wait_time: u64,
        action_id: u64,
    ) -> Self {
        Self {
            delegate,
            action_type,
            request_timestamp,
            execution_timestamp: request_timestamp + wait_time as i64,
            params,
            action_id,
        }
    }

    /// Checks if the action is ready to be executed
    pub fn is_executable(&self, current_timestamp: i64) -> bool {
        current_timestamp >= self.execution_timestamp
    }

    /// Gets the packed length of the structure for serialization
    pub fn get_packed_len() -> usize {
        32 + // delegate
        1 +  // action_type
        8 +  // request_timestamp
        8 +  // execution_timestamp
        8 +  // action_id
        // params size varies, but we'll allocate max possible:
        32 + // largest param (Pubkey)
        8   // largest value (u64)
    }
} 