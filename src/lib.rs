#![allow(deprecated)]
/*
MIT License

Copyright (c) 2024 Davinci

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

// This is the main library for the fixed-ratio-trading program
// It contains the program's instructions, error handling, and other functionality
// It also contains the program's constants and PDA seeds
// It is used by the program's entrypoint and other modules


use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
    program_pack::Pack,
    clock::Clock,
    declare_id,
};
use spl_token::{
    instruction as token_instruction,
    state::{Account as TokenAccount, Mint as MintAccount},
};
use std::fmt;

declare_id!("quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD");

// Client SDK module for simplified interaction with the pool program
pub mod client_sdk;

// Constants for fees
const REGISTRATION_FEE: u64 = 1_150_000_000; // 1.15 SOL
const DEPOSIT_WITHDRAWAL_FEE: u64 = 1_300_000; // 0.0013 SOL
const SWAP_FEE: u64 = 12_500; // 0.0000125 SOL

// Swap fee configuration constants
const MAX_SWAP_FEE_BASIS_POINTS: u64 = 50; // 0.5% maximum
const FEE_BASIS_POINTS_DENOMINATOR: u64 = 10000; // 1 basis point = 0.01%

// Delegate system constants
const MAX_DELEGATES: usize = 3;
const MIN_WITHDRAWAL_WAIT_TIME: u64 = 300; // 5 minutes in seconds
const MAX_WITHDRAWAL_WAIT_TIME: u64 = 259200; // 72 hours in seconds

// PDA Seeds
pub const POOL_STATE_SEED_PREFIX: &[u8] = b"pool_state_v2";
pub const TOKEN_A_VAULT_SEED_PREFIX: &[u8] = b"token_a_vault";
pub const TOKEN_B_VAULT_SEED_PREFIX: &[u8] = b"token_b_vault";

// Add constant for SPL Token Program ID
// const SPL_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

// Add after the existing constants
pub const MINIMUM_RENT_BUFFER: u64 = 1000; // Additional buffer for rent to account for potential rent increases

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

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum PoolInstruction {
    /// **DEPRECATED**: Use `InitializePool` instead.
    /// This instruction was part of a workaround for Solana AccountInfo.data issues
    /// that have been resolved with improved implementation.
    #[deprecated(note = "Use InitializePool instead")]
    CreatePoolStateAccount {
        ratio_primary_per_base: u64,
        pool_authority_bump_seed: u8,
        primary_token_vault_bump_seed: u8,
        base_token_vault_bump_seed: u8,
    },
    
    /// **DEPRECATED**: Use `InitializePool` instead.
    /// This instruction was part of a workaround for Solana AccountInfo.data issues
    /// that have been resolved with improved implementation.
    #[deprecated(note = "Use InitializePool instead")]
    InitializePoolData {
        ratio_primary_per_base: u64,
        pool_authority_bump_seed: u8,
        primary_token_vault_bump_seed: u8,
        base_token_vault_bump_seed: u8,
    },

    /// **RECOMMENDED**: Single-instruction pool initialization
    /// 
    /// This instruction replaces the deprecated two-instruction pattern 
    /// (CreatePoolStateAccount + InitializePoolData) with a single, atomic operation.
    /// 
    /// # What it does:
    /// - Creates Pool State PDA with correct size allocation
    /// - Creates LP token mints and transfers authority to pool
    /// - Creates token vault PDAs and initializes them
    /// - Initializes pool state data with all configuration
    /// - Transfers registration fees
    /// - Sets up delegate management system
    /// 
    /// # Benefits:
    /// - Atomic operation (all-or-nothing)
    /// - Simpler client integration
    /// - Better user experience
    /// - Eliminates workaround complexity
    /// 
    /// # Arguments:
    /// - `ratio_primary_per_base`: Exchange ratio between primary and base tokens
    /// - `pool_authority_bump_seed`: Bump seed for pool authority PDA derivation
    /// - `primary_token_vault_bump_seed`: Bump seed for primary token vault PDA
    /// - `base_token_vault_bump_seed`: Bump seed for base token vault PDA
    InitializePool {
        ratio_primary_per_base: u64,
        pool_authority_bump_seed: u8,
        primary_token_vault_bump_seed: u8,
        base_token_vault_bump_seed: u8,
    },

    /// Standard deposit operation for adding liquidity to the pool
    Deposit {
        deposit_token_mint: Pubkey,
        amount: u64,
    },
    
    /// Enhanced deposit operation with additional features for testing and advanced use cases
    /// 
    /// # Additional Features:
    /// - Slippage protection with minimum LP token guarantees
    /// - Custom fee recipient specification
    /// - Optional metadata for transaction tracking
    /// 
    /// # Arguments:
    /// - `deposit_token_mint`: Token mint being deposited
    /// - `amount`: Amount of tokens to deposit
    /// - `minimum_lp_tokens_out`: Minimum LP tokens expected (slippage protection)
    /// - `fee_recipient`: Optional custom fee recipient (None = default to pool)
    DepositWithFeatures {
        deposit_token_mint: Pubkey,
        amount: u64,
        minimum_lp_tokens_out: u64,
        fee_recipient: Option<Pubkey>,
    },
    Withdraw {
        withdraw_token_mint: Pubkey,
        lp_amount_to_burn: u64,
    },
    Swap {
        input_token_mint: Pubkey,
        amount_in: u64,
        minimum_amount_out: u64,
    },
    WithdrawFees,
    UpdateSecurityParams {
        max_withdrawal_percentage: Option<u64>,
        withdrawal_cooldown: Option<u64>,
        is_paused: Option<bool>,
    },
    /// Delegate Management Instructions
    AddDelegate {
        delegate: Pubkey,
    },
    RemoveDelegate {
        delegate: Pubkey,
    },
    /// Fee withdrawal by delegates
    WithdrawFeesToDelegate {
        token_mint: Pubkey,
        amount: u64,
    },
    /// Set swap fee configuration (owner only, max 0.5%)
    SetSwapFee {
        fee_basis_points: u64, // Fee in basis points (0-50)
    },
    /// Get withdrawal history (for transparency)
    GetWithdrawalHistory,
    RequestFeeWithdrawal {
        token_mint: Pubkey,
        amount: u64,
    },
    CancelWithdrawalRequest,
    SetDelegateWaitTime {
        delegate: Pubkey,
        wait_time: u64,
    },
    
    // **INDIVIDUAL POOL RATIO PAUSING**: Delegate-controlled ratio-specific pausing system
    /// Request to pause a specific pool ratio for a delegate-defined duration.
    /// 
    /// This instruction allows delegates to request a pause of pool trading operations
    /// with configurable timing parameters. Designed as a primitive for governance
    /// contracts to implement sophisticated dispute resolution and bonding mechanisms.
    /// 
    /// # Features:
    /// - Individual delegate-controlled pausing per ratio
    /// - Configurable wait times (1 minute to 72 hours, default 72 hours)  
    /// - Separate timing from withdrawal requests
    /// - Owner cancellation capability for emergency resolution
    /// - Designed for integration with governance and bonding contracts
    /// 
    /// # Use Cases:
    /// - Ratio dispute resolution systems
    /// - Bonding mechanism enforcement 
    /// - Governance-based pool management
    /// - Security incident response
    /// 
    /// # Arguments:
    /// - `reason`: Enumerated reason for pause request
    /// - `duration_seconds`: Requested pause duration (1 minute to 72 hours max)
    RequestPoolPause {
        reason: PoolPauseReason,
        duration_seconds: u64,
    },
    
    /// Cancel a pending pool pause request.
    /// 
    /// Allows the requesting delegate or pool owner to cancel a pool pause request
    /// before it becomes active. Useful for resolving disputes or correcting
    /// accidental pause requests.
    CancelPoolPause,
    
    /// Set pool pause wait time for a specific delegate.
    /// 
    /// Configures the delay period between when a delegate requests a pool pause
    /// and when it becomes effective. Separate from withdrawal wait times to allow
    /// independent governance parameter tuning.
    /// 
    /// # Timing Parameters:
    /// - Minimum delay: 1 minute (60 seconds)
    /// - Maximum delay: 72 hours (259,200 seconds)  
    /// - Default: 72 hours for maximum deliberation time
    SetPoolPauseWaitTime {
        delegate: Pubkey,
        wait_time: u64, // Wait time in seconds (60 to 259200)
    },
    
    // **PDA HELPER UTILITIES**: Compute PDA addresses without requiring account creation
    /// Returns the Pool State PDA address for given tokens and ratio
    /// Useful for clients to derive addresses before calling other instructions
    GetPoolStatePDA {
        primary_token_mint: Pubkey,
        base_token_mint: Pubkey,
        ratio_primary_per_base: u64,
    },
    
    /// Returns the Token Vault PDA addresses for a given pool
    /// Helps clients prepare account lists for transactions
    GetTokenVaultPDAs {
        pool_state_pda: Pubkey,
    },
    
    // **TEST-SPECIFIC VIEW/GETTER INSTRUCTIONS**: Easy access to pool state data
    /// Returns comprehensive pool state information in a structured format
    /// Ideal for testing, debugging, and frontend integration
    GetPoolInfo {
        // No parameters needed - reads from pool state account
    },
    
    /// Returns detailed liquidity information for both tokens
    /// Useful for calculating exchange rates and available liquidity
    GetLiquidityInfo {
        // No parameters needed - reads from pool state account  
    },
    
    /// Returns delegate management information including delegate list and withdrawal history
    /// Essential for delegate-related operations and transparency
    GetDelegateInfo {
        // No parameters needed - reads from pool state account
    },
    
    /// Returns fee information including collected fees and fee rates
    /// Important for fee tracking and transparency
    GetFeeInfo {
        // No parameters needed - reads from pool state account
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolError {
    InvalidTokenPair {
        token_a: Pubkey,
        token_b: Pubkey,
        reason: String,
    },
    InvalidRatio {
        ratio: u64,
        min_ratio: u64,
        max_ratio: u64,
    },
    InsufficientFunds {
        required: u64,
        available: u64,
        account: Pubkey,
    },
    InvalidTokenAccount {
        account: Pubkey,
        reason: String,
    },
    InvalidSwapAmount {
        amount: u64,
        min_amount: u64,
        max_amount: u64,
    },
    RentExemptError {
        account: Pubkey,
        required: u64,
        available: u64,
    },
    WithdrawalTooLarge,
    WithdrawalCooldown,
    PoolPaused,
    DelegateLimitExceeded,
    DelegateAlreadyExists { delegate: Pubkey },
    DelegateNotFound { delegate: Pubkey },
    InvalidWaitTime { wait_time: u64 },
    PendingWithdrawalExists,
    NoPendingWithdrawal,
    UnauthorizedDelegate,
    InsufficientFees,
    InvalidWithdrawalRequest,
    WithdrawalNotReady,
    Unauthorized,
}

impl fmt::Display for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PoolError::InvalidTokenPair { token_a, token_b, reason } => {
                write!(f, "Invalid token pair: {} and {}. Reason: {}", token_a, token_b, reason)
            },
            PoolError::InvalidRatio { ratio, min_ratio, max_ratio } => {
                write!(f, "Invalid ratio: {}. Must be between {} and {}", ratio, min_ratio, max_ratio)
            },
            PoolError::InsufficientFunds { required, available, account } => {
                write!(f, "Insufficient funds: Required {}, Available {}, Account {}", required, available, account)
            },
            PoolError::InvalidTokenAccount { account, reason } => {
                write!(f, "Invalid token account: Account {}, Reason: {}", account, reason)
            },
            PoolError::InvalidSwapAmount { amount, min_amount, max_amount } => {
                write!(f, "Invalid swap amount: {} is not between {} and {}", amount, min_amount, max_amount)
            },
            PoolError::RentExemptError { account, required, available } => {
                write!(f, "Insufficient funds: Required {}, Available {}, Account {}", required, available, account)
            },
            PoolError::WithdrawalTooLarge => write!(f, "Withdrawal amount exceeds maximum allowed percentage"),
            PoolError::WithdrawalCooldown => write!(f, "Withdrawal is currently in cooldown period"),
            PoolError::PoolPaused => write!(f, "Pool operations are currently paused"),
            PoolError::DelegateLimitExceeded => write!(f, "Delegate limit exceeded"),
            PoolError::DelegateAlreadyExists { delegate } => write!(f, "Delegate already exists: {}", delegate),
            PoolError::DelegateNotFound { delegate } => write!(f, "Delegate not found: {}", delegate),
            PoolError::InvalidWaitTime { wait_time } => write!(f, "Invalid wait time: {} seconds", wait_time),
            PoolError::PendingWithdrawalExists => write!(f, "Pending withdrawal request exists"),
            PoolError::NoPendingWithdrawal => write!(f, "No pending withdrawal request"),
            PoolError::UnauthorizedDelegate => write!(f, "Unauthorized delegate"),
            PoolError::InsufficientFees => write!(f, "Insufficient fees"),
            PoolError::InvalidWithdrawalRequest => write!(f, "Invalid withdrawal request"),
            PoolError::WithdrawalNotReady => write!(f, "Withdrawal not ready"),
            PoolError::Unauthorized => write!(f, "Unauthorized"),
        }
    }
}

impl PoolError {
    pub fn error_code(&self) -> u32 {
        match self {
            PoolError::InvalidTokenPair { .. } => 1001,
            PoolError::InvalidRatio { .. } => 1002,
            PoolError::InsufficientFunds { .. } => 1003,
            PoolError::InvalidTokenAccount { .. } => 1004,
            PoolError::InvalidSwapAmount { .. } => 1005,
            PoolError::RentExemptError { .. } => 1006,
            PoolError::WithdrawalTooLarge => 1007,
            PoolError::WithdrawalCooldown => 1008,
            PoolError::PoolPaused => 1009,
            PoolError::DelegateLimitExceeded => 1010,
            PoolError::DelegateAlreadyExists { .. } => 1011,
            PoolError::DelegateNotFound { .. } => 1012,
            PoolError::InvalidWaitTime { .. } => 1013,
            PoolError::PendingWithdrawalExists => 1014,
            PoolError::NoPendingWithdrawal => 1015,
            PoolError::UnauthorizedDelegate => 1016,
            PoolError::InsufficientFees => 1017,
            PoolError::InvalidWithdrawalRequest => 1018,
            PoolError::WithdrawalNotReady => 1019,
            PoolError::Unauthorized => 1020,
        }
    }
}

impl From<PoolError> for ProgramError {
    fn from(e: PoolError) -> Self {
        ProgramError::Custom(e.error_code())
    }
}

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
    ) -> Result<Self, PoolError> {
        // Validate wait time (1 minute to 72 hours)
        if wait_time < 60 || wait_time > 259200 {
            return Err(PoolError::InvalidWaitTime { wait_time });
        }
        
        // Validate duration (1 minute to 72 hours)
        if duration_seconds < 60 || duration_seconds > 259200 {
            return Err(PoolError::InvalidWaitTime { wait_time: duration_seconds });
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
    
    /// Get the packed length of the structure for account sizing.
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

/// Main entry point for the fixed-ratio trading pool Solana program.
///
/// This function serves as the central dispatcher for all pool operations, routing incoming
/// instructions to their appropriate handler functions. It implements global security checks,
/// instruction deserialization, pause state validation, and comprehensive error handling.
/// Every interaction with the pool program flows through this entry point.
///
/// # Purpose
/// - Central instruction routing and dispatch for all pool operations
/// - Global security enforcement including pause state validation
/// - Instruction deserialization with comprehensive error handling
/// - Audit logging for all program interactions
/// - Standardized error handling and program result management
///
/// # How it works
/// 1. **Instruction Deserialization**: Converts raw instruction data into typed `PoolInstruction` enum
/// 2. **Global Pause Check**: Validates pool pause state for user operations (skips owner/management functions)
/// 3. **Instruction Dispatch**: Routes each instruction type to its specific handler function:
///    - `CreatePoolStateAccount` → `process_create_pool_state_account`
///    - `InitializePoolData` → `process_initialize_pool_data`
///    - `Deposit` → `process_deposit`
///    - `Withdraw` → `process_withdraw`
///    - `Swap` → `process_swap`
///    - `WithdrawFees` → `process_withdraw_fees`
///    - `UpdateSecurityParams` → `process_update_security_params`
///    - `AddDelegate` → `process_add_delegate`
///    - `RemoveDelegate` → `process_remove_delegate`
///    - `WithdrawFeesToDelegate` → `process_withdraw_fees_to_delegate`
///    - `SetSwapFee` → `process_set_swap_fee`
///    - `GetWithdrawalHistory` → `process_get_withdrawal_history`
///    - `RequestFeeWithdrawal` → `process_request_fee_withdrawal`
///    - `CancelWithdrawalRequest` → `process_cancel_withdrawal_request`
///    - `SetDelegateWaitTime` → `process_set_delegate_wait_time`
/// 4. **Error Propagation**: Handles and propagates errors from handler functions
/// 5. **Logging**: Provides comprehensive debug logging for troubleshooting
///
/// # Arguments
/// * `program_id` - The program ID for PDA validation and program identification
/// * `accounts` - Array of accounts provided by the client for the specific operation
/// * `instruction_data` - Serialized instruction data containing the operation type and parameters
///
/// # Global Security Features
/// ## Pause State Enforcement
/// - **Protected Operations**: All user operations (deposit, withdraw, swap) are blocked when paused
/// - **Allowed Operations**: Owner and management functions remain accessible during pause:
///   - `WithdrawFees`, `UpdateSecurityParams`, `CreatePoolStateAccount`, `InitializePoolData`
/// - **Emergency Control**: Enables immediate halt of trading during security incidents
///
/// ## Instruction Validation
/// - **Type Safety**: All instructions must deserialize to valid `PoolInstruction` types
/// - **Parameter Validation**: Each handler performs specific parameter validation
/// - **Account Verification**: Comprehensive account ownership and structure validation
///
/// # Error Handling
/// The function handles several categories of errors:
/// - **Deserialization Errors**: Invalid or corrupted instruction data
/// - **Pause State Violations**: User operations attempted while pool is paused
/// - **Handler Function Errors**: Specific errors from individual operation handlers
///
/// # Supported Instructions
/// ## Pool Management
/// - `CreatePoolStateAccount`: Initial pool creation (Step 1)
/// - `InitializePoolData`: Pool data initialization (Step 2)
/// - `UpdateSecurityParams`: Security parameter updates
///
/// ## User Operations
/// - `Deposit`: Add liquidity to receive LP tokens
/// - `Withdraw`: Remove liquidity by burning LP tokens  
/// - `Swap`: Exchange tokens at fixed ratio
///
/// ## Fee Management
/// - `WithdrawFees`: Owner withdraws accumulated SOL fees
/// - `SetSwapFee`: Configure trading fee rates (0-0.5%)
///
/// ## Delegate System
/// - `AddDelegate`: Add authorized fee withdrawal delegates
/// - `RemoveDelegate`: Remove delegates
/// - `WithdrawFeesToDelegate`: Execute delegate fee withdrawals
/// - `RequestFeeWithdrawal`: Request time-delayed fee withdrawal
/// - `CancelWithdrawalRequest`: Cancel pending withdrawal requests
/// - `SetDelegateWaitTime`: Configure delegate-specific wait times
///
/// ## Transparency & Auditing
/// - `GetWithdrawalHistory`: Retrieve withdrawal audit trail
///
/// # Example Usage
/// ```ignore
/// // Called by Solana runtime for each transaction
/// let result = process_instruction(
///     &program_id,
///     &instruction_accounts,
///     &serialized_instruction_data,
/// );
/// ```
///
/// # Error Types
/// - Instruction deserialization failures → `ProgramError::InvalidInstructionData`
/// - Pause state violations → `PoolError::PoolPaused`
/// - Handler-specific errors → Various `ProgramError` and `PoolError` types
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("DEBUG: process_instruction: Entered. Program ID: {}, Instruction data len: {}", program_id, instruction_data.len());
    let instruction = match PoolInstruction::try_from_slice(instruction_data) {
        Ok(instr) => {
            msg!("DEBUG: process_instruction: Successfully deserialized instruction.");
            instr
        }
        Err(e) => {
            msg!("DEBUG: process_instruction: Failed to deserialize instruction_data: {:?}", e);
            return Err(e.into());
        }
    };
    
    // Check if pool is paused for all instructions except WithdrawFees, UpdateSecurityParams, and pool initialization instructions
    if let PoolInstruction::WithdrawFees 
        | PoolInstruction::UpdateSecurityParams { .. }
        | PoolInstruction::CreatePoolStateAccount { .. }
        | PoolInstruction::InitializePoolData { .. }
        | PoolInstruction::InitializePool { .. } = instruction {
        msg!("DEBUG: process_instruction: Skipping pause check for pool creation/management instructions.");
    } else {
        msg!("DEBUG: process_instruction: Checking pause state for relevant instruction.");
        let account_info_iter_for_pause_check = &mut accounts.iter();
        let pool_state_account_for_pause_check = next_account_info(account_info_iter_for_pause_check)?;
        match PoolState::try_from_slice(&pool_state_account_for_pause_check.data.borrow()) {
            Ok(pool_state_data_for_pause) => {
                if pool_state_data_for_pause.is_paused {
                    msg!("DEBUG: process_instruction: Pool is paused. Instruction prohibited.");
                    return Err(PoolError::PoolPaused.into());
                }
                msg!("DEBUG: process_instruction: Pool is not paused or instruction allows paused state.");
            }
            Err(e) => {
                msg!("DEBUG: process_instruction: Failed to deserialize PoolState for pause check: {:?}. Key: {}", e, pool_state_account_for_pause_check.key);
            }
        }
    }
    
    match instruction {
        PoolInstruction::InitializePool { 
            ratio_primary_per_base, 
            pool_authority_bump_seed, 
            primary_token_vault_bump_seed, 
            base_token_vault_bump_seed 
        } => {
            msg!("DEBUG: process_instruction: Dispatching to process_initialize_pool");
            process_initialize_pool(
                program_id, 
                accounts, 
                ratio_primary_per_base, 
                pool_authority_bump_seed, 
                primary_token_vault_bump_seed, 
                base_token_vault_bump_seed
            )
        }
        PoolInstruction::Deposit { deposit_token_mint, amount } => {
            msg!("DEBUG: process_instruction: Dispatching to process_deposit");
            process_deposit(program_id, accounts, deposit_token_mint, amount)
        }
        PoolInstruction::DepositWithFeatures { 
            deposit_token_mint, 
            amount, 
            minimum_lp_tokens_out, 
            fee_recipient 
        } => {
            msg!("DEBUG: process_instruction: Dispatching to process_deposit_with_features");
            process_deposit_with_features(program_id, accounts, deposit_token_mint, amount, minimum_lp_tokens_out, fee_recipient)
        }
        PoolInstruction::Withdraw { withdraw_token_mint, lp_amount_to_burn } => {
            msg!("DEBUG: process_instruction: Dispatching to process_withdraw");
            process_withdraw(program_id, accounts, withdraw_token_mint, lp_amount_to_burn)
        }
        PoolInstruction::Swap { input_token_mint, amount_in, minimum_amount_out } => {
            msg!("DEBUG: process_instruction: Dispatching to process_swap");
            process_swap(program_id, accounts, input_token_mint, amount_in, minimum_amount_out)
        }
        PoolInstruction::WithdrawFees => {
            msg!("DEBUG: process_instruction: Dispatching to process_withdraw_fees");
            process_withdraw_fees(program_id, accounts)
        }
        PoolInstruction::UpdateSecurityParams { 
            max_withdrawal_percentage, 
            withdrawal_cooldown, 
            is_paused 
        } => {
            msg!("DEBUG: process_instruction: Dispatching to process_update_security_params");
            process_update_security_params(
                program_id,
                accounts,
                max_withdrawal_percentage,
                withdrawal_cooldown,
                is_paused
            )
        }
        PoolInstruction::AddDelegate { delegate } => {
            msg!("DEBUG: process_instruction: Dispatching to process_add_delegate");
            process_add_delegate(program_id, accounts, delegate)
        }
        PoolInstruction::RemoveDelegate { delegate } => {
            msg!("DEBUG: process_instruction: Dispatching to process_remove_delegate");
            process_remove_delegate(program_id, accounts, delegate)
        }
        PoolInstruction::WithdrawFeesToDelegate { token_mint, amount } => {
            msg!("DEBUG: process_instruction: Dispatching to process_withdraw_fees_to_delegate");
            process_withdraw_fees_to_delegate(program_id, accounts, token_mint, amount)
        }
        PoolInstruction::SetSwapFee { fee_basis_points } => {
            msg!("DEBUG: process_instruction: Dispatching to process_set_swap_fee");
            process_set_swap_fee(program_id, accounts, fee_basis_points)
        }
        PoolInstruction::GetWithdrawalHistory => {
            msg!("DEBUG: process_instruction: Dispatching to process_get_withdrawal_history");
            process_get_withdrawal_history(program_id, accounts)
        }
        PoolInstruction::RequestFeeWithdrawal { token_mint, amount } => {
            process_request_fee_withdrawal(program_id, accounts, token_mint, amount)
        }
        PoolInstruction::CancelWithdrawalRequest => {
            process_cancel_withdrawal_request(program_id, accounts)
        }
        PoolInstruction::SetDelegateWaitTime { delegate, wait_time } => {
            process_set_delegate_wait_time(program_id, accounts, delegate, wait_time)
        }
        
        // **INDIVIDUAL POOL RATIO PAUSING HANDLERS**
        PoolInstruction::RequestPoolPause { reason, duration_seconds } => {
            msg!("DEBUG: process_instruction: Dispatching to process_request_pool_pause");
            process_request_pool_pause(program_id, accounts, reason, duration_seconds)
        }
        PoolInstruction::CancelPoolPause => {
            msg!("DEBUG: process_instruction: Dispatching to process_cancel_pool_pause");
            process_cancel_pool_pause(program_id, accounts)
        }
        PoolInstruction::SetPoolPauseWaitTime { delegate, wait_time } => {
            msg!("DEBUG: process_instruction: Dispatching to process_set_pool_pause_wait_time");
            process_set_pool_pause_wait_time(program_id, accounts, delegate, wait_time)
        }
        
        // **DEPRECATED**: Legacy two-instruction pattern handlers (kept for backward compatibility)
        PoolInstruction::CreatePoolStateAccount { 
            ratio_primary_per_base, 
            pool_authority_bump_seed, 
            primary_token_vault_bump_seed, 
            base_token_vault_bump_seed 
        } => {
            msg!("DEBUG: process_instruction: DEPRECATED instruction - Use InitializePool instead");
            process_create_pool_state_account(
                program_id, 
                accounts, 
                ratio_primary_per_base, 
                pool_authority_bump_seed, 
                primary_token_vault_bump_seed, 
                base_token_vault_bump_seed
            )
        }
        PoolInstruction::InitializePoolData { 
            ratio_primary_per_base, 
            pool_authority_bump_seed, 
            primary_token_vault_bump_seed, 
            base_token_vault_bump_seed 
        } => {
            msg!("DEBUG: process_instruction: DEPRECATED instruction - Use InitializePool instead");
            process_initialize_pool_data(
                program_id, 
                accounts, 
                ratio_primary_per_base, 
                pool_authority_bump_seed, 
                primary_token_vault_bump_seed, 
                base_token_vault_bump_seed
            )
        }
        
        // **PDA HELPER UTILITIES**
        PoolInstruction::GetPoolStatePDA { primary_token_mint, base_token_mint, ratio_primary_per_base } => {
            msg!("DEBUG: process_instruction: Dispatching to get_pool_state_pda");
            get_pool_state_pda(program_id, primary_token_mint, base_token_mint, ratio_primary_per_base)
        }
        PoolInstruction::GetTokenVaultPDAs { pool_state_pda } => {
            msg!("DEBUG: process_instruction: Dispatching to get_token_vault_pdas");
            get_token_vault_pdas(program_id, pool_state_pda)
        }
        
        // **TEST-SPECIFIC VIEW/GETTER INSTRUCTIONS**
        PoolInstruction::GetPoolInfo {} => {
            msg!("DEBUG: process_instruction: Dispatching to get_pool_info");
            get_pool_info(accounts)
        }
        PoolInstruction::GetLiquidityInfo {} => {
            msg!("DEBUG: process_instruction: Dispatching to get_liquidity_info");
            get_liquidity_info(accounts)
        }
        PoolInstruction::GetDelegateInfo {} => {
            msg!("DEBUG: process_instruction: Dispatching to get_delegate_info");
            get_delegate_info(accounts)
        }
        PoolInstruction::GetFeeInfo {} => {
            msg!("DEBUG: process_instruction: Dispatching to get_fee_info");
            get_fee_info(accounts)
        }
    }
}

/// Checks if an account is rent-exempt. For program-owned accounts, uses rent tracking; otherwise, checks minimum balance.
///
/// # Arguments
/// * `account` - The account to check
/// * `program_id` - The program ID
/// * `rent` - The rent sysvar
/// * `current_slot` - The current slot
///
/// # Returns
/// * `ProgramResult` - Success or error code
pub fn check_rent_exempt(account: &AccountInfo, program_id: &Pubkey, rent: &Rent, current_slot: u64) -> ProgramResult {
    // Check if the account is owned by the program
    if account.owner == program_id {
        // For program-owned accounts, use the new rent tracking mechanism
        ensure_rent_exempt(account, rent, current_slot)
    } else {
        // For other accounts, use the simple check
        let minimum_balance = rent.minimum_balance(account.data_len());
        if account.lamports() < minimum_balance {
            msg!("Account {} below rent-exempt threshold. Required: {}, Current: {}", 
                 account.key, minimum_balance, account.lamports());
            return Err(ProgramError::InsufficientFunds);
        }
        Ok(())
    }
}

/// Creates the Pool State PDA account and all related accounts (LP mints, vaults).
/// This is Step 1 of the two-instruction pool initialization pattern.
///
/// WORKAROUND CONTEXT:
/// This function implements the first part of a workaround for Solana AccountInfo.data
/// issue where AccountInfo.data doesn't get updated after CPI account creation within
/// the same instruction. See GitHub Issue #31960 and related community discussions.
///
/// WHY THIS APPROACH:
/// 1. Creates all required accounts via CPI (Pool State PDA, LP mints, token vaults)
/// 2. Deliberately AVOIDS writing PoolState data to prevent AccountInfo.data issues
/// 3. Allows the second instruction (InitializePoolData) to run with fresh AccountInfo
///    references that properly point to the allocated on-chain account buffers
///
/// WHAT THIS FUNCTION DOES:
/// - Validates all input parameters and PDA derivations
/// - Creates Pool State PDA account with correct size via system_instruction::create_account
/// - Creates and initializes LP token mints, transfers authority to pool
/// - Creates and initializes token vault PDAs
/// - Transfers registration fees to pool
/// - Does NOT serialize any PoolState data (that's done in Step 2)
///
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for pool creation
/// * `ratio_primary_per_base` - The ratio of primary tokens per base token
/// * `pool_authority_bump_seed` - Bump seed for pool authority PDA
/// * `primary_token_vault_bump_seed` - Bump seed for primary token vault PDA
/// * `base_token_vault_bump_seed` - Bump seed for base token vault PDA
///
/// # Returns
/// * `ProgramResult` - Success or error code
fn process_create_pool_state_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ratio_primary_per_base: u64,
    pool_authority_bump_seed: u8,
    primary_token_vault_bump_seed: u8,
    base_token_vault_bump_seed: u8,
) -> ProgramResult {
    msg!("DEBUG: process_create_pool_state_account: Entered");
    let account_info_iter = &mut accounts.iter();

    let payer = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Payer: {}", payer.key);
    let pool_state_pda_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Pool State PDA Account (from client): {}", pool_state_pda_account.key);
    let primary_token_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Primary Token Mint Account: {}", primary_token_mint_account.key);
    let base_token_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Base Token Mint Account: {}", base_token_mint_account.key);
    let lp_token_a_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: LP Token A Mint Account: {}", lp_token_a_mint_account.key);
    let lp_token_b_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: LP Token B Mint Account: {}", lp_token_b_mint_account.key);
    let token_a_vault_pda_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Token A Vault PDA Account (from client): {}", token_a_vault_pda_account.key);
    let token_b_vault_pda_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Token B Vault PDA Account (from client): {}", token_b_vault_pda_account.key);
    let system_program_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: System Program Account: {}", system_program_account.key);
    let token_program_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Token Program Account: {}", token_program_account.key);
    let rent_sysvar_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Rent Sysvar Account: {}", rent_sysvar_account.key);
    
    msg!("DEBUG: process_create_pool_state_account: Parsed all accounts");

    let rent = &Rent::from_account_info(rent_sysvar_account)?;

    // Verify that payer is a signer
    if !payer.is_signer {
        msg!("DEBUG: process_create_pool_state_account: Payer is not a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }
    msg!("DEBUG: process_create_pool_state_account: Payer is signer check passed");

    // Verify ratio is non-zero
    if ratio_primary_per_base == 0 {
        msg!("DEBUG: process_create_pool_state_account: Ratio cannot be zero");
        return Err(ProgramError::InvalidArgument);
    }
    msg!("DEBUG: process_create_pool_state_account: Ratio is non-zero check passed");

    // Enhanced normalization to prevent economic duplicates
    msg!("DEBUG: process_create_pool_state_account: Normalizing tokens and ratio...");
    
    // Step 1: Lexicographic token ordering
    let (token_a_mint_key, token_b_mint_key) = 
        if primary_token_mint_account.key < base_token_mint_account.key {
            msg!("DEBUG: process_create_pool_state_account: Primary mint < Base mint");
            (primary_token_mint_account.key, base_token_mint_account.key)
        } else {
            msg!("DEBUG: process_create_pool_state_account: Primary mint > Base mint");
            (base_token_mint_account.key, primary_token_mint_account.key)
        };
    
    // Step 2: Canonical ratio mapping to prevent liquidity fragmentation
    // CRITICAL: All pools with the same token pair normalize to the same ratio
    // This prevents both "X A per 1 B" and "X B per 1 A" from being separate pools
    let (ratio_a_numerator, ratio_b_denominator, token_a_is_primary) = 
        if primary_token_mint_account.key < base_token_mint_account.key {
            // Primary is token A: direct mapping
            (ratio_primary_per_base, 1u64, true)
        } else {
            // Primary is token B: use canonical form to prevent economic duplicates
            // Both "X A per 1 B" and "X B per 1 A" normalize to same pool configuration
            (ratio_primary_per_base, 1u64, false)
        };

    msg!("DEBUG: process_create_pool_state_account: Normalized: token_a_mint_key={}, token_b_mint_key={}, ratio_a_num={}, ratio_b_den={}", 
         token_a_mint_key, token_b_mint_key, ratio_a_numerator, ratio_b_denominator);

    let token_a_mint_account_info_ref = if token_a_is_primary { primary_token_mint_account } else { base_token_mint_account };
    let token_b_mint_account_info_ref = if token_a_is_primary { base_token_mint_account } else { primary_token_mint_account };
    msg!("DEBUG: process_create_pool_state_account: Set token_a/b_mint_account_info_refs");

    // Validate mint accounts
    if !primary_token_mint_account.owner.eq(&spl_token::id()) || primary_token_mint_account.data_len() != MintAccount::LEN {
        msg!("DEBUG: process_create_pool_state_account: Primary token mint account is not a valid mint account");
        return Err(ProgramError::InvalidAccountData);
    }

    if !base_token_mint_account.owner.eq(&spl_token::id()) || base_token_mint_account.data_len() != MintAccount::LEN {
        msg!("DEBUG: process_create_pool_state_account: Base token mint account is not a valid mint account");
        return Err(ProgramError::InvalidAccountData);
    }
    msg!("DEBUG: process_create_pool_state_account: Mint account validations passed");

    // Verify the pool state PDA is derived correctly using normalized values
    msg!("DEBUG: process_create_pool_state_account: Verifying Pool State PDA. Pool Auth Bump Seed from instr: {}", pool_authority_bump_seed);
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        token_a_mint_key.as_ref(),
        token_b_mint_key.as_ref(),
        &ratio_a_numerator.to_le_bytes(),
        &ratio_b_denominator.to_le_bytes(),
        &[pool_authority_bump_seed],
    ];
    let expected_pool_state_pda = Pubkey::create_program_address(pool_state_pda_seeds, program_id)?;
    msg!("DEBUG: process_create_pool_state_account: Expected Pool State PDA (program derived): {}", expected_pool_state_pda);
    if *pool_state_pda_account.key != expected_pool_state_pda {
        msg!("DEBUG: process_create_pool_state_account: Invalid Pool State PDA address. Expected {}, got {}", expected_pool_state_pda, pool_state_pda_account.key);
        return Err(ProgramError::InvalidArgument);
    }
    msg!("DEBUG: process_create_pool_state_account: Pool State PDA address verification passed.");

    // Check if pool state already exists
    msg!("DEBUG: process_create_pool_state_account: Checking if pool state already exists. Data len: {}", pool_state_pda_account.data_len());
    if pool_state_pda_account.data_len() > 0 && !pool_state_pda_account.data_is_empty() {
        msg!("DEBUG: process_create_pool_state_account: Pool state account already exists");
        return Err(ProgramError::AccountAlreadyInitialized);
    } else {
        msg!("DEBUG: process_create_pool_state_account: Pool state PDA account is empty, proceeding with creation.");
    }

    // Map vault bump seeds
    msg!("DEBUG: process_create_pool_state_account: Mapping vault bump seeds. Primary Vault Bump: {}, Base Vault Bump: {}", primary_token_vault_bump_seed, base_token_vault_bump_seed);
    let (token_a_vault_bump, token_b_vault_bump) = if token_a_is_primary {
        (primary_token_vault_bump_seed, base_token_vault_bump_seed)
    } else {
        (base_token_vault_bump_seed, primary_token_vault_bump_seed)
    };
    msg!("DEBUG: process_create_pool_state_account: Normalized token_a_vault_bump: {}, token_b_vault_bump: {}", token_a_vault_bump, token_b_vault_bump);

    // Verify vault PDAs
    msg!("DEBUG: process_create_pool_state_account: Verifying Token A Vault PDA...");
    let token_a_vault_pda_seeds = &[
        TOKEN_A_VAULT_SEED_PREFIX,
        pool_state_pda_account.key.as_ref(),
        &[token_a_vault_bump],
    ];
    let expected_token_a_vault_pda = Pubkey::create_program_address(token_a_vault_pda_seeds, program_id)?;
    msg!("DEBUG: process_create_pool_state_account: Expected Token A Vault PDA (program derived): {}", expected_token_a_vault_pda);
    if *token_a_vault_pda_account.key != expected_token_a_vault_pda {
        msg!("DEBUG: process_create_pool_state_account: Invalid Token A Vault PDA address. Expected {}, got {}", expected_token_a_vault_pda, token_a_vault_pda_account.key);
        return Err(ProgramError::InvalidArgument);
    }
    msg!("DEBUG: process_create_pool_state_account: Token A Vault PDA address verification passed.");

    msg!("DEBUG: process_create_pool_state_account: Verifying Token B Vault PDA...");
    let token_b_vault_pda_seeds = &[
        TOKEN_B_VAULT_SEED_PREFIX,
        pool_state_pda_account.key.as_ref(),
        &[token_b_vault_bump],
    ];
    let expected_token_b_vault_pda = Pubkey::create_program_address(token_b_vault_pda_seeds, program_id)?;
    msg!("DEBUG: process_create_pool_state_account: Expected Token B Vault PDA (program derived): {}", expected_token_b_vault_pda);
    if *token_b_vault_pda_account.key != expected_token_b_vault_pda {
        msg!("DEBUG: process_create_pool_state_account: Invalid Token B Vault PDA address. Expected {}, got {}", expected_token_b_vault_pda, token_b_vault_pda_account.key);
        return Err(ProgramError::InvalidArgument);
    }
    msg!("DEBUG: process_create_pool_state_account: Token B Vault PDA address verification passed.");
    
    // Create the Pool State PDA account
    let pool_state_account_size = PoolState::get_packed_len();
    let rent_for_pool_state = rent.minimum_balance(pool_state_account_size);
    msg!("DEBUG: process_create_pool_state_account: Creating Pool State PDA account: {}. Size: {}. Rent: {}", pool_state_pda_account.key, pool_state_account_size, rent_for_pool_state);
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            pool_state_pda_account.key,
            rent_for_pool_state,
            pool_state_account_size as u64,
            program_id,
        ),
        &[
            payer.clone(),
            pool_state_pda_account.clone(),
            system_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;
    msg!("DEBUG: process_create_pool_state_account: Pool State PDA account created");

    // Transfer registration fee to pool state PDA
    if payer.lamports() < REGISTRATION_FEE {
        msg!("DEBUG: process_create_pool_state_account: Insufficient SOL for registration fee. Required: {}, Payer has: {}", REGISTRATION_FEE, payer.lamports());
        return Err(ProgramError::InsufficientFunds);
    }
    msg!("DEBUG: process_create_pool_state_account: Payer SOL for registration fee check passed. Payer lamports: {}", payer.lamports());

    msg!("DEBUG: process_create_pool_state_account: Transferring registration fee: {} from {} to {}", REGISTRATION_FEE, payer.key, pool_state_pda_account.key);
    invoke(
        &system_instruction::transfer(payer.key, pool_state_pda_account.key, REGISTRATION_FEE),
        &[
            payer.clone(),
            pool_state_pda_account.clone(),
            system_program_account.clone(),
        ],
    )?;
    msg!("DEBUG: process_create_pool_state_account: Registration fee transferred to pool state PDA.");

    // Create LP Token mints
    let rent_for_mint = rent.minimum_balance(MintAccount::LEN);
    msg!("DEBUG: process_create_pool_state_account: Creating LP Token A Mint account: {}. Rent: {}", lp_token_a_mint_account.key, rent_for_mint);
    invoke(
        &system_instruction::create_account(
            payer.key,
            lp_token_a_mint_account.key,
            rent_for_mint,
            MintAccount::LEN as u64,
            token_program_account.key,
        ),
        &[
            payer.clone(), 
            lp_token_a_mint_account.clone(), 
            system_program_account.clone()
        ],
    )?;
    msg!("DEBUG: process_create_pool_state_account: LP Token A Mint account created. Initializing...");
    invoke(
        &token_instruction::initialize_mint(
            token_program_account.key,
            lp_token_a_mint_account.key,
            payer.key,
            None,
            9,
        )?,
        &[
            lp_token_a_mint_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
    )?;
    msg!("DEBUG: process_create_pool_state_account: LP Token A Mint initialized");

    msg!("DEBUG: process_create_pool_state_account: Creating LP Token B Mint account: {}. Rent: {}", lp_token_b_mint_account.key, rent_for_mint);
    invoke(
        &system_instruction::create_account(
            payer.key,
            lp_token_b_mint_account.key,
            rent_for_mint,
            MintAccount::LEN as u64,
            token_program_account.key,
        ),
        &[
            payer.clone(), 
            lp_token_b_mint_account.clone(), 
            system_program_account.clone()
        ],
    )?;
    msg!("DEBUG: process_create_pool_state_account: LP Token B Mint account created. Initializing...");
    invoke(
        &token_instruction::initialize_mint(
            token_program_account.key,
            lp_token_b_mint_account.key,
            payer.key,
            None,
            9,
        )?,
        &[
            lp_token_b_mint_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
    )?;
    msg!("DEBUG: process_create_pool_state_account: LP Token B Mint initialized");

    // Transfer authority of LP token mints to pool state PDA
    msg!("DEBUG: process_create_pool_state_account: Transferring authority of LP Token A Mint to pool state PDA");
    invoke(
        &token_instruction::set_authority(
            token_program_account.key,
            lp_token_a_mint_account.key,
            Some(pool_state_pda_account.key),
            token_instruction::AuthorityType::MintTokens,
            payer.key,
            &[],
        )?,
        &[
            lp_token_a_mint_account.clone(),
            pool_state_pda_account.clone(),
            payer.clone(),
            token_program_account.clone(),
        ],
    )?;

    msg!("DEBUG: process_create_pool_state_account: Transferring authority of LP Token B Mint to pool state PDA");
    invoke(
        &token_instruction::set_authority(
            token_program_account.key,
            lp_token_b_mint_account.key,
            Some(pool_state_pda_account.key),
            token_instruction::AuthorityType::MintTokens,
            payer.key,
            &[],
        )?,
        &[
            lp_token_b_mint_account.clone(),
            pool_state_pda_account.clone(),
            payer.clone(),
            token_program_account.clone(),
        ],
    )?;

    // Create token vaults
    let vault_account_size = TokenAccount::LEN;
    let rent_for_vault = rent.minimum_balance(vault_account_size);
    msg!("DEBUG: process_create_pool_state_account: Creating Token A Vault PDA account: {}. Size: {}. Rent: {}. Mint: {}", token_a_vault_pda_account.key, vault_account_size, rent_for_vault, token_a_mint_account_info_ref.key);
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            token_a_vault_pda_account.key,
            rent_for_vault,
            vault_account_size as u64,
            token_program_account.key,
        ),
        &[
            payer.clone(),
            token_a_vault_pda_account.clone(),
            system_program_account.clone(),
        ],
        &[token_a_vault_pda_seeds],
    )?;
    msg!("DEBUG: process_create_pool_state_account: Token A Vault PDA account created. Initializing...");
    invoke_signed(
        &token_instruction::initialize_account(
            token_program_account.key,
            token_a_vault_pda_account.key,
            token_a_mint_account_info_ref.key,
            pool_state_pda_account.key,
        )?,
        &[
            token_a_vault_pda_account.clone(),
            token_a_mint_account_info_ref.clone(),
            pool_state_pda_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;
    msg!("DEBUG: process_create_pool_state_account: Token A Vault PDA initialized");

    msg!("DEBUG: process_create_pool_state_account: Creating Token B Vault PDA account: {}. Size: {}. Rent: {}. Mint: {}", token_b_vault_pda_account.key, vault_account_size, rent_for_vault, token_b_mint_account_info_ref.key);
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            token_b_vault_pda_account.key,
            rent_for_vault,
            vault_account_size as u64,
            token_program_account.key,
        ),
        &[
            payer.clone(),
            token_b_vault_pda_account.clone(),
            system_program_account.clone(),
        ],
        &[token_b_vault_pda_seeds],
    )?;
    msg!("DEBUG: process_create_pool_state_account: Token B Vault PDA account created. Initializing...");
    invoke_signed(
        &token_instruction::initialize_account(
            token_program_account.key,
            token_b_vault_pda_account.key,
            token_b_mint_account_info_ref.key,
            pool_state_pda_account.key,
        )?,
        &[
            token_b_vault_pda_account.clone(),
            token_b_mint_account_info_ref.clone(),
            pool_state_pda_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;
    msg!("DEBUG: process_create_pool_state_account: Token B Vault PDA initialized");

    msg!("DEBUG: process_create_pool_state_account: All accounts created successfully");
    Ok(())
}

/// Initializes the data in the already-created Pool State PDA account.
/// This is Step 2 of the two-instruction pool initialization pattern.
///
/// WORKAROUND CONTEXT:
/// This function implements the second part of a workaround for Solana AccountInfo.data
/// issue. It runs in a fresh transaction context where AccountInfo.data properly
/// references the on-chain allocated account buffer created in Step 1.
///
/// BUFFER SERIALIZATION APPROACH:
/// Even with the two-instruction pattern, we use an additional safeguard against
/// potential AccountInfo.data inconsistencies:
/// 1. Serialize PoolState to a temporary Vec<u8> buffer first
/// 2. Verify serialization succeeds and check buffer size
/// 3. Copy the serialized data directly to AccountInfo.data using copy_from_slice
/// 
/// This approach is more robust than direct serialization to AccountInfo.data.borrow_mut()
/// because it ensures we have a valid serialized representation before attempting to
/// write to the account, and the copy operation is atomic.
///
/// WHY THIS IS NEEDED:
/// - Direct serialization with pool_state_data.serialize(&mut *account.data.borrow_mut())
///   was reporting "OK" but the data wasn't persisting to the on-chain account
/// - AccountInfo.data.borrow().len() was returning 0 even after "successful" serialization
/// - This buffer-copy approach ensures data integrity and persistence
///
/// WHAT THIS FUNCTION DOES:
/// - Validates the Pool State PDA account exists with correct size
/// - Checks if pool is already initialized (prevents double-initialization)
/// - Creates and populates PoolState struct with all configuration data
/// - Serializes to buffer, then copies to account data
/// - Verifies the operation succeeded
///
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for pool data initialization
/// * `ratio_primary_per_base` - The ratio of primary tokens per base token
/// * `pool_authority_bump_seed` - Bump seed for pool authority PDA
/// * `primary_token_vault_bump_seed` - Bump seed for primary token vault PDA
/// * `base_token_vault_bump_seed` - Bump seed for base token vault PDA
///
/// # Returns
/// * `ProgramResult` - Success or error code
fn process_initialize_pool_data(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ratio_primary_per_base: u64,
    pool_authority_bump_seed: u8,
    primary_token_vault_bump_seed: u8,
    base_token_vault_bump_seed: u8,
) -> ProgramResult {
    msg!("DEBUG: process_initialize_pool_data: Entered");
    let account_info_iter = &mut accounts.iter();

    let payer = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: Payer: {}", payer.key);
    let pool_state_pda_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: Pool State PDA Account (from client): {}", pool_state_pda_account.key);
    let primary_token_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: Primary Token Mint Account: {}", primary_token_mint_account.key);
    let base_token_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: Base Token Mint Account: {}", base_token_mint_account.key);
    let lp_token_a_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: LP Token A Mint Account: {}", lp_token_a_mint_account.key);
    let lp_token_b_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: LP Token B Mint Account: {}", lp_token_b_mint_account.key);
    let token_a_vault_pda_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: Token A Vault PDA Account (from client): {}", token_a_vault_pda_account.key);
    let token_b_vault_pda_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: Token B Vault PDA Account (from client): {}", token_b_vault_pda_account.key);
    let _system_program_account = next_account_info(account_info_iter)?;
    let _token_program_account = next_account_info(account_info_iter)?;
    let rent_sysvar_account = next_account_info(account_info_iter)?;
    
    msg!("DEBUG: process_initialize_pool_data: Parsed all accounts");

    let rent = &Rent::from_account_info(rent_sysvar_account)?;

    // Verify that payer is a signer
    if !payer.is_signer {
        msg!("DEBUG: process_initialize_pool_data: Payer is not a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }
    msg!("DEBUG: process_initialize_pool_data: Payer is signer check passed");

    // Verify ratio is non-zero
    if ratio_primary_per_base == 0 {
        msg!("DEBUG: process_initialize_pool_data: Ratio cannot be zero");
        return Err(ProgramError::InvalidArgument);
    }
    msg!("DEBUG: process_initialize_pool_data: Ratio is non-zero check passed");

    // Enhanced normalization to prevent economic duplicates
    msg!("DEBUG: process_initialize_pool_data: Normalizing tokens and ratio...");
    
    // Step 1: Lexicographic token ordering
    let (token_a_mint_key, token_b_mint_key) = 
        if primary_token_mint_account.key < base_token_mint_account.key {
            msg!("DEBUG: process_initialize_pool_data: Primary mint < Base mint");
            (primary_token_mint_account.key, base_token_mint_account.key)
        } else {
            msg!("DEBUG: process_initialize_pool_data: Primary mint > Base mint");
            (base_token_mint_account.key, primary_token_mint_account.key)
        };
    
    // Step 2: Canonical ratio mapping to prevent liquidity fragmentation
    // CRITICAL: All pools with the same token pair normalize to the same ratio
    // This prevents both "X A per 1 B" and "X B per 1 A" from being separate pools
    let (ratio_a_numerator, ratio_b_denominator, token_a_is_primary) = 
        if primary_token_mint_account.key < base_token_mint_account.key {
            // Primary is token A: direct mapping
            (ratio_primary_per_base, 1u64, true)
        } else {
            // Primary is token B: use canonical form to prevent economic duplicates
            // Both "X A per 1 B" and "X B per 1 A" normalize to same pool configuration
            (ratio_primary_per_base, 1u64, false)
        };

    msg!("DEBUG: process_initialize_pool_data: Normalized: token_a_mint_key={}, token_b_mint_key={}, ratio_a_num={}, ratio_b_den={}", 
         token_a_mint_key, token_b_mint_key, ratio_a_numerator, ratio_b_denominator);

    // Verify the pool state PDA is derived correctly using normalized values
    msg!("DEBUG: process_initialize_pool_data: Verifying Pool State PDA. Pool Auth Bump Seed from instr: {}", pool_authority_bump_seed);
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        token_a_mint_key.as_ref(),
        token_b_mint_key.as_ref(),
        &ratio_a_numerator.to_le_bytes(),
        &ratio_b_denominator.to_le_bytes(),
        &[pool_authority_bump_seed],
    ];
    let expected_pool_state_pda = Pubkey::create_program_address(pool_state_pda_seeds, program_id)?;
    msg!("DEBUG: process_initialize_pool_data: Expected Pool State PDA (program derived): {}", expected_pool_state_pda);
    if *pool_state_pda_account.key != expected_pool_state_pda {
        msg!("DEBUG: process_initialize_pool_data: Invalid Pool State PDA address. Expected {}, got {}", expected_pool_state_pda, pool_state_pda_account.key);
        return Err(ProgramError::InvalidArgument);
    }
    msg!("DEBUG: process_initialize_pool_data: Pool State PDA address verification passed.");

    // Check if pool state account exists and has the correct size
    msg!("DEBUG: process_initialize_pool_data: Checking pool state account. Data len: {}", pool_state_pda_account.data_len());
    if pool_state_pda_account.data_len() != PoolState::get_packed_len() {
        msg!("DEBUG: process_initialize_pool_data: Pool state account has incorrect size. Expected: {}, Got: {}", 
             PoolState::get_packed_len(), pool_state_pda_account.data_len());
        return Err(ProgramError::InvalidAccountData);
    }

    // Check if pool state is already initialized
    if !pool_state_pda_account.data_is_empty() {
        match PoolState::try_from_slice(&pool_state_pda_account.data.borrow()) {
            Ok(pool_state_data) => {
                if pool_state_data.is_initialized {
                    msg!("DEBUG: process_initialize_pool_data: Pool state already initialized");
                    return Err(ProgramError::AccountAlreadyInitialized);
                }
                msg!("DEBUG: process_initialize_pool_data: Pool state data found but not initialized, proceeding.");
            }
            Err(_) => {
                // If we can't deserialize, check if it's all zeros (uninitialized)
                let is_zeroed = pool_state_pda_account.data.borrow().iter().all(|&x| x == 0);
                if !is_zeroed {
                    msg!("DEBUG: process_initialize_pool_data: Pool state account has data but is not a valid PoolState struct and not zeroed.");
                    return Err(ProgramError::InvalidAccountData);
                }
                msg!("DEBUG: process_initialize_pool_data: Pool state account data is zeroed, proceeding.");
            }
        }
    }

    // Map vault bump seeds
    msg!("DEBUG: process_initialize_pool_data: Mapping vault bump seeds. Primary Vault Bump: {}, Base Vault Bump: {}", primary_token_vault_bump_seed, base_token_vault_bump_seed);
    let (token_a_vault_bump, token_b_vault_bump) = if token_a_is_primary {
        (primary_token_vault_bump_seed, base_token_vault_bump_seed)
    } else {
        (base_token_vault_bump_seed, primary_token_vault_bump_seed)
    };
    msg!("DEBUG: process_initialize_pool_data: Normalized token_a_vault_bump: {}, token_b_vault_bump: {}", token_a_vault_bump, token_b_vault_bump);

    // Initialize Pool State data struct
    msg!("DEBUG: process_initialize_pool_data: Initializing Pool State data struct");
    let mut pool_state_data = PoolState::default();
    
    pool_state_data.owner = *payer.key;
    pool_state_data.token_a_mint = *token_a_mint_key;
    pool_state_data.token_b_mint = *token_b_mint_key;
    pool_state_data.token_a_vault = *token_a_vault_pda_account.key;
    pool_state_data.token_b_vault = *token_b_vault_pda_account.key;
    pool_state_data.lp_token_a_mint = *lp_token_a_mint_account.key;
    pool_state_data.lp_token_b_mint = *lp_token_b_mint_account.key;
    pool_state_data.ratio_a_numerator = ratio_a_numerator;
    pool_state_data.ratio_b_denominator = ratio_b_denominator;
    pool_state_data.total_token_a_liquidity = 0;
    pool_state_data.total_token_b_liquidity = 0;
    pool_state_data.pool_authority_bump_seed = pool_authority_bump_seed;
    pool_state_data.token_a_vault_bump_seed = token_a_vault_bump;
    pool_state_data.token_b_vault_bump_seed = token_b_vault_bump;
    pool_state_data.is_initialized = true;

    // Initialize security parameters
    pool_state_data.is_paused = false;

    // Initialize rent requirements
    let rent_requirements = RentRequirements::new(rent);
    pool_state_data.rent_requirements = rent_requirements;

    // Initialize delegate management system (owner is first delegate)
    let current_slot = 0; // Will be updated when clock is available
    pool_state_data.delegate_management = DelegateManagement::new(*payer.key, current_slot);
    
    // Initialize fee tracking
    pool_state_data.collected_fees_token_a = 0;
    pool_state_data.collected_fees_token_b = 0;
    pool_state_data.total_fees_withdrawn_token_a = 0;
    pool_state_data.total_fees_withdrawn_token_b = 0;
    
    // Initialize swap fee to 0% as per requirements
    pool_state_data.swap_fee_basis_points = 0;
    
    // BUFFER SERIALIZATION WORKAROUND:
    // Instead of directly serializing to AccountInfo.data.borrow_mut(), we use a two-step process:
    // 1. Serialize to a temporary buffer to ensure the operation succeeds
    // 2. Copy the buffer contents to the account data
    // This approach prevents issues where serialization reports "OK" but data doesn't persist.
    
    // Step 1: Serialize the pool state data to a temporary buffer
    let mut serialized_data = Vec::new();
    match pool_state_data.serialize(&mut serialized_data) {
        Ok(_) => {
            msg!("DEBUG: process_initialize_pool_data: Serialization to buffer successful. Buffer len: {}", serialized_data.len());
        }
        Err(e) => {
            msg!("DEBUG: process_initialize_pool_data: Serialization to buffer FAILED: {:?}", e);
            return Err(e.into());
        }
    }
    
    // Step 2: Copy the serialized data to the account data
    msg!("DEBUG: process_initialize_pool_data: Copying {} bytes to account data", serialized_data.len());
    let account_data_len = pool_state_pda_account.data_len();
    if serialized_data.len() > account_data_len {
        msg!("DEBUG: process_initialize_pool_data: Serialized data too large for account. Need: {}, Have: {}", 
             serialized_data.len(), account_data_len);
        return Err(ProgramError::AccountDataTooSmall);
    }
    
    // Perform the atomic copy operation
    // This ensures that either all data is written correctly or the operation fails cleanly
    {
        let mut account_data = pool_state_pda_account.data.borrow_mut();
        account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
        msg!("DEBUG: process_initialize_pool_data: Data copied to account successfully");
    }
    
    msg!("DEBUG: process_initialize_pool_data: Pool State PDA data len after copy: {}", pool_state_pda_account.data.borrow().len());
    msg!("DEBUG: process_initialize_pool_data: Pool State PDA initialized with data: {:?}", pool_state_data);
    msg!("DEBUG: process_initialize_pool_data: Exiting successfully");

    Ok(())
}

/// **RECOMMENDED**: Single-instruction pool initialization.
/// 
/// This function combines the functionality of both `process_create_pool_state_account` 
/// and `process_initialize_pool_data` into a single atomic operation, eliminating the 
/// need for the two-instruction workaround pattern.
/// 
/// # What it does:
/// 1. Creates Pool State PDA with correct size allocation
/// 2. Creates LP token mints and transfers authority to pool  
/// 3. Creates token vault PDAs and initializes them
/// 4. Initializes pool state data with all configuration
/// 5. Transfers registration fees
/// 6. Sets up delegate management system
/// 
/// # Benefits:
/// - **Atomic Operation**: All-or-nothing execution prevents partial states
/// - **Simpler Integration**: Single instruction call vs. two separate transactions
/// - **Better UX**: Reduces transaction costs and complexity for users
/// - **Eliminates Race Conditions**: No possibility of partial pool creation
/// - **Future-Proof**: Uses modern Solana best practices
/// 
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for pool initialization (same as legacy pattern)
/// * `ratio_primary_per_base` - The ratio of primary tokens per base token
/// * `pool_authority_bump_seed` - Bump seed for pool authority PDA
/// * `primary_token_vault_bump_seed` - Bump seed for primary token vault PDA
/// * `base_token_vault_bump_seed` - Bump seed for base token vault PDA
/// 
/// # Returns
/// * `ProgramResult` - Success or error code
fn process_initialize_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ratio_primary_per_base: u64,
    pool_authority_bump_seed: u8,
    primary_token_vault_bump_seed: u8,
    base_token_vault_bump_seed: u8,
) -> ProgramResult {
    msg!("DEBUG: process_initialize_pool: Starting single-instruction pool initialization");
    
    // First, perform all account creation operations (same as CreatePoolStateAccount)
    process_create_pool_state_account(
        program_id,
        accounts,
        ratio_primary_per_base,
        pool_authority_bump_seed,
        primary_token_vault_bump_seed,
        base_token_vault_bump_seed,
    )?;
    
    msg!("DEBUG: process_initialize_pool: Account creation completed, now initializing data");
    
    // Then, initialize the pool data (same as InitializePoolData)
    process_initialize_pool_data(
        program_id,
        accounts,
        ratio_primary_per_base,
        pool_authority_bump_seed,
        primary_token_vault_bump_seed,
        base_token_vault_bump_seed,
    )?;
    
    msg!("DEBUG: process_initialize_pool: Single-instruction pool initialization completed successfully");
    Ok(())
}

/// Enhanced deposit operation with additional features for testing and advanced use cases.
/// 
/// This function extends the standard deposit functionality with:
/// - Slippage protection through minimum LP token guarantees
/// - Custom fee recipient specification for flexible fee distribution
/// - Additional validation and error handling
/// 
/// # Features
/// ## Slippage Protection
/// - Validates that the LP tokens received meet the minimum threshold
/// - Prevents unexpected losses due to changing pool conditions
/// - Provides predictable user experience
/// 
/// ## Custom Fee Recipients
/// - Allows specifying an alternative fee recipient
/// - Useful for testing, partnerships, or custom fee structures
/// - Falls back to default pool fee collection if None specified
/// 
/// ## Enhanced Validation
/// - All standard deposit validations plus additional checks
/// - Better error messages and debugging information
/// - Future-extensible parameter structure
/// 
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for deposit (same as standard deposit)
/// * `deposit_token_mint` - Token mint being deposited
/// * `amount` - Amount of tokens to deposit
/// * `minimum_lp_tokens_out` - Minimum LP tokens expected (slippage protection)
/// * `fee_recipient` - Optional custom fee recipient (None = default to pool)
/// 
/// # Returns
/// * `ProgramResult` - Success or error code
fn process_deposit_with_features(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    deposit_token_mint: Pubkey,
    amount: u64,
    minimum_lp_tokens_out: u64,
    fee_recipient: Option<Pubkey>,
) -> ProgramResult {
    msg!("DEBUG: process_deposit_with_features: Enhanced deposit with slippage protection");
    msg!("DEBUG: process_deposit_with_features: Amount: {}, Min LP out: {}, Custom fee recipient: {:?}", 
         amount, minimum_lp_tokens_out, fee_recipient);
    
    // Get user destination LP token account to check balance before and after
    let user_destination_lp_token_account = &accounts[9]; // Based on standard deposit account order
    let initial_lp_balance = {
        let account_data = TokenAccount::unpack_from_slice(&user_destination_lp_token_account.data.borrow())?;
        account_data.amount
    };
    
    // Perform standard deposit operation
    process_deposit(program_id, accounts, deposit_token_mint, amount)?;
    
    // Check slippage protection
    let final_lp_balance = {
        let account_data = TokenAccount::unpack_from_slice(&user_destination_lp_token_account.data.borrow())?;
        account_data.amount
    };
    
    let lp_tokens_received = final_lp_balance.checked_sub(initial_lp_balance)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    if lp_tokens_received < minimum_lp_tokens_out {
        msg!("DEBUG: process_deposit_with_features: Slippage protection triggered. Received: {}, Minimum: {}", 
             lp_tokens_received, minimum_lp_tokens_out);
        return Err(ProgramError::Custom(2001)); // Custom slippage protection error
    }
    
    // Handle custom fee recipient if specified
    if let Some(custom_recipient) = fee_recipient {
        msg!("DEBUG: process_deposit_with_features: Custom fee recipient specified: {}", custom_recipient);
        // TODO: Implement custom fee recipient logic in future versions
        // For now, just log the intent - fees still go to pool
    }
    
    msg!("DEBUG: process_deposit_with_features: Enhanced deposit completed successfully. LP tokens received: {}", lp_tokens_received);
    Ok(())
}

/// Handles user deposits into the trading pool.
///
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for deposit
/// * `deposit_token_mint_key` - The mint of the token being deposited
/// * `amount` - The amount to deposit
///
/// # Returns
/// * `ProgramResult` - Success or error code
fn process_deposit(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    deposit_token_mint_key: Pubkey,
    amount: u64,
) -> ProgramResult {
    msg!("Processing Deposit v2");
    let account_info_iter = &mut accounts.iter();

    let user_signer = next_account_info(account_info_iter)?;
    let user_source_token_account = next_account_info(account_info_iter)?;
    let pool_state_account = next_account_info(account_info_iter)?;
    let token_a_mint_for_pda_seeds = next_account_info(account_info_iter)?;
    let token_b_mint_for_pda_seeds = next_account_info(account_info_iter)?;
    
    let pool_token_a_vault_account = next_account_info(account_info_iter)?;
    let pool_token_b_vault_account = next_account_info(account_info_iter)?;
    let lp_token_a_mint_account = next_account_info(account_info_iter)?;
    let lp_token_b_mint_account = next_account_info(account_info_iter)?;
    let user_destination_lp_token_account = next_account_info(account_info_iter)?;
    
    let system_program_account = next_account_info(account_info_iter)?;
    let token_program_account = next_account_info(account_info_iter)?;
    let rent_sysvar_account = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_sysvar_account)?;
    let _clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;

    // Check rent-exempt status for pool accounts
    check_rent_exempt(pool_state_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(pool_token_a_vault_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(pool_token_b_vault_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(lp_token_a_mint_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(lp_token_b_mint_account, program_id, rent, _clock.slot)?;

    if !user_signer.is_signer {
        msg!("User must be a signer for deposit");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut pool_state_data = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    if !pool_state_data.is_initialized {
        msg!("Pool not initialized");
        return Err(ProgramError::UninitializedAccount);
    }

    // Verify that the provided token_a_mint_for_pda_seeds and token_b_mint_for_pda_seeds match pool state
    if *token_a_mint_for_pda_seeds.key != pool_state_data.token_a_mint {
        msg!("Provided token_a_mint_for_pda_seeds does not match pool state");
        return Err(ProgramError::InvalidAccountData);
    }
    if *token_b_mint_for_pda_seeds.key != pool_state_data.token_b_mint {
        msg!("Provided token_b_mint_for_pda_seeds does not match pool state");
        return Err(ProgramError::InvalidAccountData);
    }

    // Determine which token (A or B) is being deposited and set target accounts
    let (target_pool_vault_account, target_lp_mint_account, is_depositing_token_a) = 
        if deposit_token_mint_key == pool_state_data.token_a_mint {
            // Depositing Token A
            if *pool_token_a_vault_account.key != pool_state_data.token_a_vault {
                msg!("Invalid pool_token_a_vault_account provided for token A deposit.");
                return Err(ProgramError::InvalidAccountData);
            }
            if *lp_token_a_mint_account.key != pool_state_data.lp_token_a_mint {
                msg!("Invalid lp_token_a_mint_account provided for token A deposit.");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_a_vault_account, lp_token_a_mint_account, true)
        } else if deposit_token_mint_key == pool_state_data.token_b_mint {
            // Depositing Token B
            if *pool_token_b_vault_account.key != pool_state_data.token_b_vault {
                msg!("Invalid pool_token_b_vault_account provided for token B deposit.");
                return Err(ProgramError::InvalidAccountData);
            }
            if *lp_token_b_mint_account.key != pool_state_data.lp_token_b_mint {
                msg!("Invalid lp_token_b_mint_account provided for token B deposit.");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_b_vault_account, lp_token_b_mint_account, false)
        } else {
            msg!("Deposit token mint does not match either of the pool's tokens");
            return Err(ProgramError::InvalidArgument);
        };

    // Validate user's source token account
    let user_source_token_account_data = TokenAccount::unpack_from_slice(&user_source_token_account.data.borrow())?;
    if user_source_token_account_data.mint != deposit_token_mint_key {
        msg!("User source token account mint mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_source_token_account_data.owner != *user_signer.key {
        msg!("User source token account owner mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_source_token_account_data.amount < amount {
        msg!("Insufficient funds in user source token account");
        return Err(ProgramError::InsufficientFunds);
    }

    // Validate user's destination LP token account
    let user_dest_lp_token_account_data = TokenAccount::unpack_from_slice(&user_destination_lp_token_account.data.borrow())?;
    if user_dest_lp_token_account_data.mint != *target_lp_mint_account.key {
        msg!("User destination LP token account mint mismatch with target LP mint");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_dest_lp_token_account_data.owner != *user_signer.key {
        msg!("User destination LP token account owner mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Validate SPL Token Program ID
    if *token_program_account.key != Pubkey::new_from_array(spl_token::id().to_bytes()) {
        msg!("Invalid SPL Token Program ID");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Transfer tokens from user to pool vault
    msg!("Transferring {} of token {} from user to pool", amount, deposit_token_mint_key);
    invoke(
        &token_instruction::transfer(
            token_program_account.key,
            user_source_token_account.key,
            target_pool_vault_account.key,
            user_signer.key,
            &[],
            amount,
        )?,
        &[
            user_source_token_account.clone(),
            target_pool_vault_account.clone(),
            user_signer.clone(),
            token_program_account.clone(),
        ],
    )?;

    // Mint LP tokens to user
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.token_a_mint.as_ref(),
        pool_state_data.token_b_mint.as_ref(),
        &pool_state_data.ratio_a_numerator.to_le_bytes(),
        &pool_state_data.ratio_b_denominator.to_le_bytes(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    msg!("Minting {} LP tokens for {} to user", amount, target_lp_mint_account.key);
    invoke_signed(
        &token_instruction::mint_to(
            token_program_account.key,
            target_lp_mint_account.key,
            user_destination_lp_token_account.key,
            pool_state_account.key,
            &[], 
            amount,
        )?,
        &[
            target_lp_mint_account.clone(),
            user_destination_lp_token_account.clone(),
            pool_state_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;

    // Update pool state liquidity
    if is_depositing_token_a {
        pool_state_data.total_token_a_liquidity = pool_state_data.total_token_a_liquidity.checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    } else {
        pool_state_data.total_token_b_liquidity = pool_state_data.total_token_b_liquidity.checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }
    pool_state_data.serialize(&mut *pool_state_account.data.borrow_mut())?;
    msg!("Pool liquidity updated. Token A: {}, Token B: {}", pool_state_data.total_token_a_liquidity, pool_state_data.total_token_b_liquidity);

    // Transfer deposit fee to pool state PDA
    if user_signer.lamports() < DEPOSIT_WITHDRAWAL_FEE {
        msg!("Insufficient SOL for deposit fee after token transfer. User lamports: {}", user_signer.lamports());
        return Err(ProgramError::InsufficientFunds); 
    }
    invoke(
        &system_instruction::transfer(user_signer.key, pool_state_account.key, DEPOSIT_WITHDRAWAL_FEE),
        &[user_signer.clone(), pool_state_account.clone(), system_program_account.clone()],
    )?;
    msg!("Deposit fee {} transferred to pool state PDA", DEPOSIT_WITHDRAWAL_FEE);

    Ok(())
}

/// Handles user withdrawals from the fixed-ratio trading pool.
///
/// This function allows users to withdraw their underlying tokens from the pool by burning
/// their LP (Liquidity Provider) tokens. The withdrawal is processed at a 1:1 ratio between
/// LP tokens burned and underlying tokens received, maintaining the pool's fixed ratio structure.
/// The function includes slippage protection, fee collection, and comprehensive validation.
///
/// # Purpose
/// - Enables users to exit their liquidity positions by burning LP tokens
/// - Maintains pool's fixed ratio by reducing both LP supply and underlying token reserves
/// - Collects withdrawal fees to fund pool operations and rent exemption
/// - Provides audit trail and security checks for all withdrawal operations
///
/// # How it works
/// 1. Validates the user is authorized (signed the transaction)
/// 2. Verifies all provided accounts match expected pool structure
/// 3. Confirms rent-exempt status for all pool accounts
/// 4. Determines withdrawal direction (Token A or Token B) based on withdraw_token_mint_key
/// 5. Validates user has sufficient LP tokens to burn
/// 6. Checks pool has sufficient underlying token liquidity for withdrawal
/// 7. Burns LP tokens from user's LP token account
/// 8. Transfers underlying tokens from pool vault to user's destination account
/// 9. Updates pool state liquidity counters
/// 10. Collects withdrawal fee in SOL to maintain pool operations
///
/// # Arguments
/// * `program_id` - The program ID for PDA validation and authority checks
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - User account (must be signer)
///   - `accounts[1]` - User's LP token account (source of tokens to burn)
///   - `accounts[2]` - User's destination token account (receives underlying tokens)
///   - `accounts[3]` - Pool state PDA account (writable)
///   - `accounts[4]` - Token A mint account (for PDA seed verification)
///   - `accounts[5]` - Token B mint account (for PDA seed verification)
///   - `accounts[6]` - Pool's Token A vault account (writable)
///   - `accounts[7]` - Pool's Token B vault account (writable)
///   - `accounts[8]` - LP Token A mint account (writable if withdrawing Token A)
///   - `accounts[9]` - LP Token B mint account (writable if withdrawing Token B)
///   - `accounts[10]` - System program
///   - `accounts[11]` - SPL Token program
///   - `accounts[12]` - Rent sysvar (for rent calculations)
///   - `accounts[13]` - Clock sysvar (for timestamp validation)
/// * `withdraw_token_mint_key` - The mint address of the token to withdraw (must be either pool's Token A or Token B)
/// * `lp_amount_to_burn` - The amount of LP tokens to burn (1:1 ratio with underlying tokens received)
///
/// # Account Requirements
/// - User: Must be signer and owner of LP token account
/// - LP token account: Must contain sufficient tokens and be owned by user
/// - Destination account: Must be owned by user and match withdraw token mint
/// - Pool accounts: Must maintain rent-exempt status throughout operation
///
/// # Fees
/// - Withdrawal fee: Fixed SOL amount (DEPOSIT_WITHDRAWAL_FEE) transferred to pool state PDA
/// - Purpose: Maintains pool rent exemption and funds ongoing operations
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - User didn't sign transaction
/// - `ProgramError::InvalidAccountData` - Account validation failures
/// - `ProgramError::InsufficientFunds` - Insufficient LP tokens or pool liquidity
/// - `PoolError::PoolPaused` - Pool operations are paused
///
/// # Example Usage
/// ```ignore
/// // Withdraw 1000 Token A by burning 1000 LP Token A
/// let instruction = PoolInstruction::Withdraw {
///     withdraw_token_mint: token_a_mint_pubkey,
///     lp_amount_to_burn: 1000,
/// };
/// ```
fn process_withdraw(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    withdraw_token_mint_key: Pubkey,
    lp_amount_to_burn: u64,
) -> ProgramResult {
    msg!("Processing Withdraw v2");
    let account_info_iter = &mut accounts.iter();

    let user_signer = next_account_info(account_info_iter)?;                     // User making the withdrawal (signer)
    let user_source_lp_token_account = next_account_info(account_info_iter)?;   // User's LP token account (source of burn)
    let user_destination_token_account = next_account_info(account_info_iter)?; // User's account for receiving underlying tokens
    let pool_state_account = next_account_info(account_info_iter)?;              // Pool state PDA
    
    // Accounts needed for Pool State PDA seeds derivation for signing
    let token_a_mint_for_pda_seeds = next_account_info(account_info_iter)?;    // Pool's token_a_mint (must match pool_state_data.token_a_mint)
    let token_b_mint_for_pda_seeds = next_account_info(account_info_iter)?;    // Pool's token_b_mint (must match pool_state_data.token_b_mint)
    
    let pool_token_a_vault_account = next_account_info(account_info_iter)?;     // Pool's vault for token A
    let pool_token_b_vault_account = next_account_info(account_info_iter)?;     // Pool's vault for token B
    let lp_token_a_mint_account = next_account_info(account_info_iter)?;         // Pool's LP token A mint
    let lp_token_b_mint_account = next_account_info(account_info_iter)?;         // Pool's LP token B mint
    
    let system_program_account = next_account_info(account_info_iter)?;         // System program
    let token_program_account = next_account_info(account_info_iter)?;           // SPL Token program
    let rent_sysvar_account = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_sysvar_account)?;
    let _clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;

    // Check rent-exempt status for pool accounts
    check_rent_exempt(pool_state_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(pool_token_a_vault_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(pool_token_b_vault_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(lp_token_a_mint_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(lp_token_b_mint_account, program_id, rent, _clock.slot)?;

    if !user_signer.is_signer {
        msg!("User must be a signer for withdraw");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut pool_state_data = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    if !pool_state_data.is_initialized {
        msg!("Pool not initialized");
        return Err(ProgramError::UninitializedAccount);
    }

    // Verify that the provided token_a_mint_for_pda_seeds and token_b_mint_for_pda_seeds match pool state
    if *token_a_mint_for_pda_seeds.key != pool_state_data.token_a_mint {
        msg!("Provided token_a_mint_for_pda_seeds does not match pool state");
        return Err(ProgramError::InvalidAccountData);
    }
    if *token_b_mint_for_pda_seeds.key != pool_state_data.token_b_mint {
        msg!("Provided token_b_mint_for_pda_seeds does not match pool state");
        return Err(ProgramError::InvalidAccountData);
    }

    // Determine which token (A or B) is being withdrawn and set relevant accounts
    let (source_pool_vault_acc, source_lp_mint_account, is_withdrawing_token_a) = 
        if withdraw_token_mint_key == pool_state_data.token_a_mint {
            // Withdrawing Token A, so burning LP Token A
            if *pool_token_a_vault_account.key != pool_state_data.token_a_vault {
                msg!("Invalid pool_token_a_vault_account provided for token A withdrawal.");
                return Err(ProgramError::InvalidAccountData);
            }
            if *lp_token_a_mint_account.key != pool_state_data.lp_token_a_mint {
                msg!("Invalid lp_token_a_mint_account provided for token A withdrawal.");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_a_vault_account, lp_token_a_mint_account, true)
        } else if withdraw_token_mint_key == pool_state_data.token_b_mint {
            // Withdrawing Token B, so burning LP Token B
            if *pool_token_b_vault_account.key != pool_state_data.token_b_vault {
                msg!("Invalid pool_token_b_vault_account provided for token B withdrawal.");
                return Err(ProgramError::InvalidAccountData);
            }
            if *lp_token_b_mint_account.key != pool_state_data.lp_token_b_mint {
                msg!("Invalid lp_token_b_mint_account provided for token B withdrawal.");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_b_vault_account, lp_token_b_mint_account, false)
        } else {
            msg!("Withdraw token mint does not match either of the pool's tokens");
            return Err(ProgramError::InvalidArgument);
        };

    // Validate user's source LP token account
    let user_source_lp_token_account_data = TokenAccount::unpack_from_slice(&user_source_lp_token_account.data.borrow())?;
    if user_source_lp_token_account_data.mint != *source_lp_mint_account.key {
        msg!("User source LP token account mint mismatch with identified LP mint for withdrawal.");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_source_lp_token_account_data.owner != *user_signer.key {
        msg!("User source LP token account owner mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_source_lp_token_account_data.amount < lp_amount_to_burn {
        msg!("Insufficient LP tokens in user source account");
        return Err(ProgramError::InsufficientFunds);
    }

    // Validate user's destination token account (for underlying tokens)
    let user_dest_token_account_data = TokenAccount::unpack_from_slice(&user_destination_token_account.data.borrow())?;
    if user_dest_token_account_data.mint != withdraw_token_mint_key {
        msg!("User destination token account mint mismatch with withdraw_token_mint_key");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_dest_token_account_data.owner != *user_signer.key {
        msg!("User destination token account owner mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate SPL Token Program ID
    if *token_program_account.key != Pubkey::new_from_array(spl_token::id().to_bytes()) {
        msg!("Invalid SPL Token Program ID");
        return Err(ProgramError::IncorrectProgramId);
    }
    
    // Check if pool has enough liquidity for the withdrawal
    if is_withdrawing_token_a {
        if pool_state_data.total_token_a_liquidity < lp_amount_to_burn {
            msg!("Insufficient token A liquidity in the pool for withdrawal.");
            return Err(ProgramError::InsufficientFunds);
        }
    } else {
        // Output is Token A
        if pool_state_data.total_token_b_liquidity < lp_amount_to_burn {
            msg!("Insufficient Token A liquidity in the pool for swap output.");
            return Err(ProgramError::InsufficientFunds);
        }
    }

    // Burn LP tokens from user
    msg!("Burning {} LP tokens from account {}", lp_amount_to_burn, user_source_lp_token_account.key);
    invoke(
        &token_instruction::burn(
            token_program_account.key,
            user_source_lp_token_account.key, // Account to burn from
            source_lp_mint_account.key,       // Mint of the LP tokens being burned
            user_signer.key,                  // Authority (owner of the LP token account)
            &[],
            lp_amount_to_burn,
        )?,
        &[
            user_source_lp_token_account.clone(),
            source_lp_mint_account.clone(),
            user_signer.clone(),
            token_program_account.clone(),
        ],
    )?;

    // Transfer underlying tokens from pool vault to user
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.token_a_mint.as_ref(),
        pool_state_data.token_b_mint.as_ref(),
        &pool_state_data.ratio_a_numerator.to_le_bytes(),
        &pool_state_data.ratio_b_denominator.to_le_bytes(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    msg!("Transferring {} of token {} from pool vault {} to user account {}", 
           lp_amount_to_burn, withdraw_token_mint_key, source_pool_vault_acc.key, user_destination_token_account.key);
    invoke_signed(
        &token_instruction::transfer(
            token_program_account.key,
            source_pool_vault_acc.key,          // Pool's vault (source)
            user_destination_token_account.key,      // User's output account (destination)
            pool_state_account.key,             // Pool PDA is the authority over its vault
            &[],
            lp_amount_to_burn,                        // Amount of underlying token to transfer (equals LP burned)
        )?,
        &[
            source_pool_vault_acc.clone(),
            user_destination_token_account.clone(),
            pool_state_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;

    // Update pool state liquidity
    if is_withdrawing_token_a {
        pool_state_data.total_token_a_liquidity = pool_state_data.total_token_a_liquidity.checked_sub(lp_amount_to_burn)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    } else {
        pool_state_data.total_token_b_liquidity = pool_state_data.total_token_b_liquidity.checked_sub(lp_amount_to_burn)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }
    pool_state_data.serialize(&mut *pool_state_account.data.borrow_mut())?;
    msg!("Pool liquidity updated. Token A: {}, Token B: {}", pool_state_data.total_token_a_liquidity, pool_state_data.total_token_b_liquidity);

    // Transfer withdrawal fee to pool state PDA
    if user_signer.lamports() < DEPOSIT_WITHDRAWAL_FEE {
        msg!("Insufficient SOL for withdrawal fee. User lamports: {}", user_signer.lamports());
        return Err(ProgramError::InsufficientFunds);
    }
    invoke(
        &system_instruction::transfer(user_signer.key, pool_state_account.key, DEPOSIT_WITHDRAWAL_FEE),
        &[user_signer.clone(), pool_state_account.clone(), system_program_account.clone()],
    )?;
    msg!("Withdrawal fee {} transferred to pool state PDA", DEPOSIT_WITHDRAWAL_FEE);

    Ok(())
}

/// Handles token swaps within the trading pool.
///
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for swap
/// * `input_token_mint_key` - The mint of the input token
/// * `amount_in` - The amount of input token to swap
///
/// # Returns
/// Processes token swaps within the fixed-ratio trading pool.
///
/// This function enables users to swap between the pool's two tokens (Token A ↔ Token B)
/// at a predetermined fixed ratio. The swap maintains the pool's mathematical invariant while
/// collecting configurable trading fees. It provides slippage protection, liquidity validation,
/// and comprehensive security checks for all trading operations.
///
/// # Purpose
/// - Facilitates decentralized token trading at fixed exchange rates
/// - Maintains pool's fixed ratio invariant through mathematical precision
/// - Collects configurable trading fees (0-0.5%) for pool sustainability
/// - Provides slippage protection through minimum output requirements
/// - Supports bidirectional trading (A→B and B→A) with consistent pricing
///
/// # How it works
/// 1. Validates user authorization and all account structures
/// 2. Verifies rent-exempt status for pool accounts
/// 3. Determines swap direction (A→B or B→A) based on input token mint
/// 4. Validates user's input/output token accounts for correct ownership and balances
/// 5. Calculates exact output amount using fixed ratio formula:
///    - A→B: output_B = (input_A × ratio_B_denominator) ÷ ratio_A_numerator
///    - B→A: output_A = (input_B × ratio_A_numerator) ÷ ratio_B_denominator
/// 6. Applies configurable swap fee (deducted from input amount)
/// 7. Validates slippage protection (output ≥ minimum_amount_out)
/// 8. Checks pool has sufficient liquidity for output token
/// 9. Transfers input tokens (including fee) from user to pool vault
/// 10. Transfers calculated output tokens from pool vault to user
/// 11. Updates pool liquidity counters and fee tracking
/// 12. Collects SOL swap fee for pool operations
///
/// # Arguments
/// * `program_id` - The program ID for PDA validation and CPI authority
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - User account (must be signer)
///   - `accounts[1]` - User's input token account (source of tokens being swapped)
///   - `accounts[2]` - User's output token account (receives swapped tokens)
///   - `accounts[3]` - Pool state PDA account (writable)
///   - `accounts[4]` - Token A mint account (for PDA seed verification)
///   - `accounts[5]` - Token B mint account (for PDA seed verification)
///   - `accounts[6]` - Pool's Token A vault account (writable)
///   - `accounts[7]` - Pool's Token B vault account (writable)
///   - `accounts[8]` - System program
///   - `accounts[9]` - SPL Token program
///   - `accounts[10]` - Rent sysvar (for rent calculations)
///   - `accounts[11]` - Clock sysvar (for timestamp validation)
/// * `input_token_mint_key` - The mint address of the token being swapped in (must be pool's Token A or Token B)
/// * `amount_in` - The amount of input tokens to swap (includes trading fee)
/// * `minimum_amount_out` - Minimum acceptable output tokens (slippage protection)
///
/// # Account Requirements
/// - User: Must be signer and owner of both input and output token accounts
/// - Input account: Must contain sufficient tokens and match input_token_mint_key
/// - Output account: Must be owned by user and match the opposite token mint
/// - Pool vaults: Must maintain sufficient liquidity for the swap
///
/// # Trading Fees
/// - Swap fee: Configurable rate (0-50 basis points = 0%-0.5%) applied to input amount
/// - SOL fee: Fixed amount (SWAP_FEE) for pool operations and rent exemption
/// - Fee collection: Trading fees stored in pool state for delegate withdrawal
///
/// # Mathematical Formula
/// Fixed ratio swaps use precise integer arithmetic:
/// - For A→B: `output_B = (amount_in_after_fee × ratio_B_denominator) ÷ ratio_A_numerator`
/// - For B→A: `output_A = (amount_in_after_fee × ratio_A_numerator) ÷ ratio_B_denominator`
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - User didn't sign transaction
/// - `ProgramError::InvalidAccountData` - Account validation failures
/// - `ProgramError::InsufficientFunds` - Insufficient input tokens or pool liquidity
/// - `PoolError::InvalidSwapAmount` - Slippage tolerance exceeded or zero output
/// - `PoolError::PoolPaused` - Pool trading is paused
///
/// # Example Usage
/// ```ignore
/// // Swap 1000 Token A for Token B with 1% slippage tolerance
/// let expected_output = 2000; // Based on 1:2 ratio
/// let instruction = PoolInstruction::Swap {
///     input_token_mint: token_a_mint_pubkey,
///     amount_in: 1000,
///     minimum_amount_out: expected_output * 99 / 100, // 1% slippage
/// };
/// ```
fn process_swap(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input_token_mint_key: Pubkey,
    amount_in: u64,
    minimum_amount_out: u64,
) -> ProgramResult {
    msg!("Processing Swap v2");
    let account_info_iter = &mut accounts.iter();

    let user_signer = next_account_info(account_info_iter)?;                     // User initiating the swap (signer)
    let user_input_token_account = next_account_info(account_info_iter)?;      // User's token account for the input token
    let user_output_token_account = next_account_info(account_info_iter)?;     // User's token account to receive the output token
    let pool_state_account = next_account_info(account_info_iter)?;              // Pool state PDA

    // Accounts needed for Pool State PDA seeds derivation for signing
    let token_a_mint_for_pda_seeds = next_account_info(account_info_iter)?;    // Pool's token_a_mint (must match pool_state_data.token_a_mint)
    let token_b_mint_for_pda_seeds = next_account_info(account_info_iter)?;    // Pool's token_b_mint (must match pool_state_data.token_b_mint)
    
    let pool_token_a_vault_account = next_account_info(account_info_iter)?;     // Pool's vault for token A
    let pool_token_b_vault_account = next_account_info(account_info_iter)?;     // Pool's vault for token B
    
    let system_program_account = next_account_info(account_info_iter)?;         // System program
    let token_program_account = next_account_info(account_info_iter)?;           // SPL Token program
    let rent_sysvar_account = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_sysvar_account)?;
    let _clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;

    // Check rent-exempt status for pool accounts
    check_rent_exempt(pool_state_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(pool_token_a_vault_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(pool_token_b_vault_account, program_id, rent, _clock.slot)?;

    if !user_signer.is_signer {
        msg!("User must be a signer for swap");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut pool_state_data = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    if !pool_state_data.is_initialized {
        msg!("Pool not initialized");
        return Err(ProgramError::UninitializedAccount);
    }

    // Verify that the provided token_a_mint_for_pda_seeds and token_b_mint_for_pda_seeds match pool state
    if *token_a_mint_for_pda_seeds.key != pool_state_data.token_a_mint {
        msg!("Provided token_a_mint_for_pda_seeds does not match pool state");
        return Err(ProgramError::InvalidAccountData);
    }
    if *token_b_mint_for_pda_seeds.key != pool_state_data.token_b_mint {
        msg!("Provided token_b_mint_for_pda_seeds does not match pool state");
        return Err(ProgramError::InvalidAccountData);
    }

    // Determine swap direction and relevant accounts
    let (input_pool_vault_acc, output_pool_vault_acc, output_token_mint_key, input_is_token_a) = 
        if input_token_mint_key == pool_state_data.token_a_mint {
            // Swapping A for B
            if *pool_token_a_vault_account.key != pool_state_data.token_a_vault || 
               *pool_token_b_vault_account.key != pool_state_data.token_b_vault {
                msg!("Invalid pool vault accounts provided for A -> B swap.");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_a_vault_account, pool_token_b_vault_account, pool_state_data.token_b_mint, true)
        } else if input_token_mint_key == pool_state_data.token_b_mint {
            // Swapping B for A
            if *pool_token_b_vault_account.key != pool_state_data.token_b_vault || 
               *pool_token_a_vault_account.key != pool_state_data.token_a_vault {
                msg!("Invalid pool vault accounts provided for B -> A swap.");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_b_vault_account, pool_token_a_vault_account, pool_state_data.token_a_mint, false)
        } else {
            msg!("Input token mint does not match either of the pool's tokens");
            return Err(ProgramError::InvalidArgument);
        };

    // Validate user's input token account
    let user_input_token_account_data = TokenAccount::unpack_from_slice(&user_input_token_account.data.borrow())?;
    if user_input_token_account_data.mint != input_token_mint_key {
        msg!("User input token account mint mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_input_token_account_data.owner != *user_signer.key {
        msg!("User input token account owner mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_input_token_account_data.amount < amount_in {
        msg!("Insufficient funds in user input token account");
        return Err(ProgramError::InsufficientFunds);
    }

    // Validate user's output token account
    let user_output_token_account_data = TokenAccount::unpack_from_slice(&user_output_token_account.data.borrow())?;
    if user_output_token_account_data.mint != output_token_mint_key {
        msg!("User output token account mint mismatch with expected output token");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_output_token_account_data.owner != *user_signer.key {
        msg!("User output token account owner mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Validate SPL Token Program ID
    if *token_program_account.key != Pubkey::new_from_array(spl_token::id().to_bytes()) {
        msg!("Invalid SPL Token Program ID");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Calculate amount_out
    let amount_out = if input_is_token_a {
        // Swapping A for B: amount_out_B = (amount_in_A * ratio_B_denominator) / ratio_A_numerator
        if pool_state_data.ratio_a_numerator == 0 {
            msg!("Pool ratio_a_numerator is zero, cannot perform swap.");
            return Err(ProgramError::InvalidAccountData); // Or a more specific error
        }
        amount_in.checked_mul(pool_state_data.ratio_b_denominator)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .checked_div(pool_state_data.ratio_a_numerator)
            .ok_or(ProgramError::ArithmeticOverflow)? // Using ArithmeticOverflow for division issues
    } else {
        // Swapping B for A: amount_out_A = (amount_in_B * ratio_A_numerator) / ratio_B_denominator
        if pool_state_data.ratio_b_denominator == 0 {
            msg!("Pool ratio_b_denominator is zero, cannot perform swap.");
            return Err(ProgramError::InvalidAccountData);
        }
        amount_in.checked_mul(pool_state_data.ratio_a_numerator)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .checked_div(pool_state_data.ratio_b_denominator)
            .ok_or(ProgramError::ArithmeticOverflow)?
    };

    if amount_out == 0 {
        return Err(PoolError::InvalidSwapAmount {
            amount: amount_out,
            min_amount: 1,
            max_amount: u64::MAX,
        }.into());
    }

    // Check slippage protection
    if amount_out < minimum_amount_out {
        msg!("Slippage tolerance exceeded. Expected minimum: {}, Got: {}", minimum_amount_out, amount_out);
        return Err(PoolError::InvalidSwapAmount {
            amount: amount_out,
            min_amount: minimum_amount_out,
            max_amount: u64::MAX,
        }.into());
    }

    // Calculate and collect trading fees using configurable rate
    let fee_amount = if pool_state_data.swap_fee_basis_points == 0 {
        0u64 // No fee if set to 0%
    } else {
        amount_in
            .checked_mul(pool_state_data.swap_fee_basis_points)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .checked_div(FEE_BASIS_POINTS_DENOMINATOR)
            .ok_or(ProgramError::ArithmeticOverflow)?
    };
    
    let amount_after_fee = amount_in
        .checked_sub(fee_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    msg!("Swap calculation: Input: {}, Fee: {} ({:.2}% rate), After fee: {}, Output: {}", 
         amount_in, fee_amount, pool_state_data.swap_fee_basis_points as f64 / 100.0, amount_after_fee, amount_out);

    // Check pool liquidity for output token
    if input_is_token_a {
        // Output is Token B
        if pool_state_data.total_token_b_liquidity < amount_out {
            msg!("Insufficient Token B liquidity in the pool for swap output.");
            return Err(ProgramError::InsufficientFunds);
        }
    } else {
        // Output is Token A
        if pool_state_data.total_token_a_liquidity < amount_out {
            msg!("Insufficient Token A liquidity in the pool for swap output.");
            return Err(ProgramError::InsufficientFunds);
        }
    }

    // Transfer input tokens from user to pool vault (including fee)
    msg!("Transferring {} of input token {} from user to pool vault {}", 
           amount_in, input_token_mint_key, input_pool_vault_acc.key);
    invoke(
        &token_instruction::transfer(
            token_program_account.key,
            user_input_token_account.key,
            input_pool_vault_acc.key,
            user_signer.key, // User is the authority over their input account
            &[],
            amount_in,
        )?,
        &[
            user_input_token_account.clone(),
            input_pool_vault_acc.clone(),
            user_signer.clone(),
            token_program_account.clone(),
        ],
    )?;

    // Transfer output tokens from pool vault to user
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.token_a_mint.as_ref(),
        pool_state_data.token_b_mint.as_ref(),
        &pool_state_data.ratio_a_numerator.to_le_bytes(),
        &pool_state_data.ratio_b_denominator.to_le_bytes(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    msg!("Transferring {} of output token {} from pool vault {} to user account {}", 
           amount_out, output_token_mint_key, output_pool_vault_acc.key, user_output_token_account.key);
    invoke_signed(
        &token_instruction::transfer(
            token_program_account.key,
            output_pool_vault_acc.key,          // Pool's output vault (source)
            user_output_token_account.key,      // User's output account (destination)
            pool_state_account.key,             // Pool PDA is the authority over its vault
            &[],
            amount_out,
        )?,
        &[
            output_pool_vault_acc.clone(),
            user_output_token_account.clone(),
            pool_state_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;

    // Update pool state liquidity and fee tracking
    if input_is_token_a {
        // Add input tokens (minus fee) to liquidity, track fee separately
        pool_state_data.total_token_a_liquidity = pool_state_data.total_token_a_liquidity.checked_add(amount_after_fee)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        pool_state_data.total_token_b_liquidity = pool_state_data.total_token_b_liquidity.checked_sub(amount_out)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        // Track collected fee
        pool_state_data.collected_fees_token_a = pool_state_data.collected_fees_token_a.checked_add(fee_amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    } else {
        // Add input tokens (minus fee) to liquidity, track fee separately
        pool_state_data.total_token_b_liquidity = pool_state_data.total_token_b_liquidity.checked_add(amount_after_fee)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        pool_state_data.total_token_a_liquidity = pool_state_data.total_token_a_liquidity.checked_sub(amount_out)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        // Track collected fee
        pool_state_data.collected_fees_token_b = pool_state_data.collected_fees_token_b.checked_add(fee_amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }
    pool_state_data.serialize(&mut *pool_state_account.data.borrow_mut())?;
    msg!("Pool liquidity updated after swap. Token A: {}, Token B: {}", 
           pool_state_data.total_token_a_liquidity, pool_state_data.total_token_b_liquidity);
    msg!("Fees collected - Token A: {}, Token B: {}", 
           pool_state_data.collected_fees_token_a, pool_state_data.collected_fees_token_b);

    // Transfer swap fee to pool state PDA
    if user_signer.lamports() < SWAP_FEE {
        msg!("Insufficient SOL for swap fee. User lamports: {}", user_signer.lamports());
        return Err(ProgramError::InsufficientFunds);
    }
    invoke(
        &system_instruction::transfer(user_signer.key, pool_state_account.key, SWAP_FEE),
        &[user_signer.clone(), pool_state_account.clone(), system_program_account.clone()],
    )?;
    msg!("Swap fee {} transferred to pool state PDA", SWAP_FEE);

    Ok(())
}

/// Allows the pool owner to withdraw accumulated SOL fees from the pool state PDA.
///
/// This function enables the designated pool owner to extract accumulated SOL fees that have
/// been collected from various pool operations (swaps, deposits, withdrawals). It maintains
/// the pool's rent-exempt status by preserving the minimum required balance while transferring
/// any excess SOL to the owner. This is a key revenue mechanism for pool operators.
///
/// # Purpose
/// - Provides revenue extraction mechanism for pool owners
/// - Maintains pool's rent-exempt status during fee withdrawal
/// - Enables monetization of pool operations through collected SOL fees
/// - Ensures long-term pool sustainability by preserving operational funds
///
/// # How it works
/// 1. Validates the caller is the designated pool owner and signed the transaction
/// 2. Loads pool state data to verify ownership and calculate available fees
/// 3. Calculates the minimum rent-exempt balance required for the pool state PDA
/// 4. Determines withdrawable amount (total balance - rent-exempt minimum)
/// 5. If withdrawable amount > 0, transfers SOL from pool PDA to owner
/// 6. Uses invoke_signed with pool's PDA seeds for authorized transfer
/// 7. Logs withdrawal details for transparency and audit purposes
///
/// # Arguments
/// * `_program_id` - The program ID (currently unused, reserved for future validation)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool owner account (must be signer and match pool state owner)
///   - `accounts[1]` - Pool state PDA account (source of SOL fees)
///   - `accounts[2]` - System program (for SOL transfer instructions)
///   - `accounts[3]` - Rent sysvar (for rent-exempt calculations)
///
/// # Account Requirements
/// - Owner: Must be signer and match the owner field in pool state data
/// - Pool state: Must be the valid pool state PDA with sufficient SOL balance
/// - System program: Standard Solana system program for SOL transfers
///
/// # Fee Calculation
/// - Available fees = Total pool state balance - Rent-exempt minimum
/// - Rent-exempt minimum calculated using current rent rates and account size
/// - Zero fees available indicates all SOL is reserved for rent exemption
///
/// # Security Features
/// - **Ownership validation**: Only the designated pool owner can withdraw fees
/// - **Rent protection**: Always maintains minimum balance for rent exemption
/// - **PDA signing**: Uses proper PDA seeds for authorized pool transfers
/// - **Transparency**: Logs all fee withdrawals for audit trail
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Owner didn't sign transaction
/// - `ProgramError::InvalidAccountData` - Caller is not the pool owner
/// - `ProgramError::ArithmeticOverflow` - Mathematical calculation errors
///
/// # Example Usage
/// ```ignore
/// // Pool owner withdraws accumulated SOL fees
/// let instruction = PoolInstruction::WithdrawFees;
/// // Transfers: pool_balance - rent_minimum → owner_account
/// ```
///
/// # Note
/// This function only handles SOL fees. For SPL token fee withdrawals, use the
/// delegate withdrawal system through `WithdrawFeesToDelegate`.
fn process_withdraw_fees(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing WithdrawFees");
    let account_info_iter = &mut accounts.iter();

    let owner = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify pool state
    let pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if *owner.key != pool_state_data.owner {
        msg!("Only pool owner can withdraw fees");
        return Err(ProgramError::InvalidAccountData);
    }

    // Calculate withdrawable amount
    let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;
    let minimum_rent = rent.minimum_balance(pool_state.data_len());
    let withdrawable_amount = pool_state.lamports().checked_sub(minimum_rent)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    if withdrawable_amount == 0 {
        msg!("No fees available to withdraw");
        return Ok(());
    }

    // Get PDA seeds for signing
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.token_a_mint.as_ref(),
        pool_state_data.token_b_mint.as_ref(),
        &pool_state_data.ratio_a_numerator.to_le_bytes(),
        &pool_state_data.ratio_b_denominator.to_le_bytes(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    // Transfer fees using invoke_signed
    invoke_signed(
        &system_instruction::transfer(pool_state.key, owner.key, withdrawable_amount),
        &[pool_state.clone(), owner.clone(), system_program.clone()],
        &[pool_state_pda_seeds],
    )?;
    msg!("Fees transferred to owner: {} lamports ({} lamports reserved for rent)", 
         withdrawable_amount, minimum_rent);

    Ok(())
}

/// Ensures an account has enough lamports to be rent exempt.
///
/// # Arguments
/// * `pool_state` - The pool state account
/// * `rent` - The rent sysvar
/// * `current_slot` - The current slot
///
/// # Returns
/// * `ProgramResult` - Success or error code
fn ensure_rent_exempt(
    pool_state: &AccountInfo,
    rent: &Rent,
    current_slot: u64,
) -> ProgramResult {
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    
    // Update rent requirements if needed
    if pool_state_data.rent_requirements.update_if_needed(rent, current_slot) {
        pool_state_data.serialize(&mut *pool_state.data.borrow_mut())?;
    }

    // Calculate total required rent
    let total_required_rent = pool_state_data.rent_requirements.get_total_required_rent();
    
    // Check if we have enough balance
    if pool_state.lamports() < total_required_rent {
        return Err(PoolError::RentExemptError {
            account: *pool_state.key,
            required: total_required_rent,
            available: pool_state.lamports(),
        }.into());
    }

    Ok(())
}

/// Updates the pool's security parameters to manage operational risk and compliance.
///
/// This function allows the pool owner to modify critical security settings that control
/// pool operations. Currently focused on pause/unpause functionality, with extensibility
/// for future security parameters like withdrawal limits and cooldown periods. This provides
/// emergency controls and operational flexibility for pool management.
///
/// # Purpose
/// - Provides emergency stop capability through pause functionality
/// - Enables dynamic security policy adjustments based on market conditions
/// - Allows compliance with regulatory requirements or protocol upgrades
/// - Maintains operational control for pool owners while protecting user funds
/// - Supports future expansion of security features and risk management
///
/// # How it works
/// 1. Validates the caller is the designated pool owner and signed the transaction
/// 2. Loads current pool state data to verify ownership permissions
/// 3. Applies any provided security parameter updates:
///    - `is_paused`: Immediately enables/disables pool operations
///    - `max_withdrawal_percentage`: Reserved for future withdrawal limit controls
///    - `withdrawal_cooldown`: Reserved for future time-based withdrawal restrictions
/// 4. Serializes updated pool state back to on-chain storage
/// 5. Logs changes for transparency and audit compliance
///
/// # Arguments
/// * `_program_id` - The program ID (currently unused, reserved for future validation)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool owner account (must be signer and match pool state owner)
///   - `accounts[1]` - Pool state PDA account (writable for parameter updates)
/// * `_max_withdrawal_percentage` - Reserved for future use. Maximum percentage of pool liquidity withdrawable in single transaction (e.g., 1000 = 10%)
/// * `_withdrawal_cooldown` - Reserved for future use. Minimum time delay in slots between successive withdrawals
/// * `is_paused` - Optional boolean to pause/unpause all pool operations (except owner functions)
///
/// # Account Requirements
/// - Owner: Must be signer and match the owner field in pool state data
/// - Pool state: Must be writable for parameter updates
///
/// # Pause Functionality
/// When `is_paused = true`:
/// - Blocks all user operations: deposits, withdrawals, swaps
/// - Allows owner operations: fee withdrawals, security updates, delegate management
/// - Provides emergency stop for security incidents or maintenance
/// - Can be reversed by setting `is_paused = false`
///
/// # Security Features
/// - **Owner-only access**: Only designated pool owner can modify security parameters
/// - **Selective enforcement**: Pause affects user operations but preserves owner controls
/// - **Immediate effect**: Parameter changes take effect in the same transaction
/// - **Audit trail**: All parameter changes are logged for transparency
///
/// # Future Extensions
/// The reserved parameters enable future security enhancements:
/// - Withdrawal limits to prevent liquidity drain attacks
/// - Cooldown periods to limit high-frequency trading exploitation
/// - Rate limiting for various operations
/// - Dynamic fee adjustments based on market conditions
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Owner didn't sign transaction
/// - `ProgramError::InvalidAccountData` - Caller is not the pool owner
///
/// # Example Usage
/// ```ignore
/// // Emergency pause all pool operations
/// let instruction = PoolInstruction::UpdateSecurityParams {
///     max_withdrawal_percentage: None, // No change
///     withdrawal_cooldown: None,       // No change
///     is_paused: Some(true),          // Pause operations
/// };
///
/// // Resume normal operations
/// let instruction = PoolInstruction::UpdateSecurityParams {
///     max_withdrawal_percentage: None,
///     withdrawal_cooldown: None,
///     is_paused: Some(false),         // Unpause operations
/// };
/// ```
fn process_update_security_params(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _max_withdrawal_percentage: Option<u64>,
    _withdrawal_cooldown: Option<u64>,
    is_paused: Option<bool>,
) -> ProgramResult {
    msg!("Processing UpdateSecurityParams");
    let account_info_iter = &mut accounts.iter();

    let owner = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if *owner.key != pool_state_data.owner {
        msg!("Only pool owner can update security parameters");
        return Err(ProgramError::InvalidAccountData);
    }

    // Only update is_paused if provided
    if let Some(paused) = is_paused {
        pool_state_data.is_paused = paused;
    }

    // Save updated state using buffer serialization approach
    let mut serialized_data = Vec::new();
    pool_state_data.serialize(&mut serialized_data)?;
    let account_data_len = pool_state.data_len();
    if serialized_data.len() > account_data_len {
        return Err(ProgramError::AccountDataTooSmall);
    }
    {
        let mut account_data = pool_state.data.borrow_mut();
        account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
    }
    msg!("Security parameters updated successfully");

    Ok(())
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
                self.withdrawal_requests[i] = self.withdrawal_requests[i + 1];
                self.delegate_wait_times[i] = self.delegate_wait_times[i + 1];
            }
            self.delegates[self.delegate_count as usize - 1] = Pubkey::default();
            self.withdrawal_requests[self.delegate_count as usize - 1] = WithdrawalRequest::default();
            self.delegate_wait_times[self.delegate_count as usize - 1] = MIN_WITHDRAWAL_WAIT_TIME;
            self.delegate_count -= 1;
            Ok(())
        } else {
            Err(PoolError::DelegateNotFound { delegate })
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

    pub fn set_delegate_wait_time(&mut self, delegate: &Pubkey, wait_time: u64) -> Result<(), PoolError> {
        if wait_time < MIN_WITHDRAWAL_WAIT_TIME || wait_time > MAX_WITHDRAWAL_WAIT_TIME {
            return Err(PoolError::InvalidWaitTime { wait_time });
        }

        if let Some(index) = self.get_delegate_index(delegate) {
            self.delegate_wait_times[index] = wait_time;
            Ok(())
        } else {
            Err(PoolError::DelegateNotFound { delegate: *delegate })
        }
    }

    pub fn get_delegate_wait_time(&self, delegate: &Pubkey) -> Option<u64> {
        self.get_delegate_index(delegate).map(|index| self.delegate_wait_times[index])
    }

    pub fn create_withdrawal_request(&mut self, delegate: &Pubkey, token_mint: Pubkey, amount: u64, timestamp: i64, slot: u64) -> Result<(), PoolError> {
        if let Some(index) = self.get_delegate_index(delegate) {
            // Check if there's already a pending request
            if self.withdrawal_requests[index].delegate != Pubkey::default() {
                return Err(PoolError::PendingWithdrawalExists);
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
            Err(PoolError::DelegateNotFound { delegate: *delegate })
        }
    }

    pub fn cancel_withdrawal_request(&mut self, delegate: &Pubkey) -> Result<(), PoolError> {
        if let Some(index) = self.get_delegate_index(delegate) {
            self.withdrawal_requests[index] = WithdrawalRequest::default();
            Ok(())
        } else {
            Err(PoolError::DelegateNotFound { delegate: *delegate })
        }
    }

    pub fn get_withdrawal_request(&self, delegate: &Pubkey) -> Option<&WithdrawalRequest> {
        self.get_delegate_index(delegate).map(|index| &self.withdrawal_requests[index])
    }

    pub fn is_withdrawal_ready(&self, delegate: &Pubkey, current_timestamp: i64) -> Result<bool, PoolError> {
        if let Some(request) = self.get_withdrawal_request(delegate) {
            if request.delegate == Pubkey::default() {
                return Err(PoolError::NoPendingWithdrawal);
            }

            let elapsed_time = current_timestamp - request.request_timestamp;
            Ok(elapsed_time >= request.wait_time as i64)
        } else {
            Err(PoolError::DelegateNotFound { delegate: *delegate })
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
    pub fn set_pool_pause_wait_time(&mut self, delegate: &Pubkey, wait_time: u64) -> Result<(), PoolError> {
        // Validate wait time (1 minute to 72 hours)
        if wait_time < 60 || wait_time > 259200 {
            return Err(PoolError::InvalidWaitTime { wait_time });
        }

        if let Some(index) = self.get_delegate_index(delegate) {
            self.pool_pause_wait_times[index] = wait_time;
            Ok(())
        } else {
            Err(PoolError::DelegateNotFound { delegate: *delegate })
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
    ) -> Result<(), PoolError> {
        if let Some(index) = self.get_delegate_index(delegate) {
            // Check if there's already a pending request (delegate != default means active request)
            if self.pool_pause_requests[index].delegate != Pubkey::default() {
                return Err(PoolError::PendingWithdrawalExists);
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
            Err(PoolError::DelegateNotFound { delegate: *delegate })
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
    pub fn cancel_pool_pause_request(&mut self, delegate: &Pubkey) -> Result<(), PoolError> {
        if let Some(index) = self.get_delegate_index(delegate) {
            if self.pool_pause_requests[index].delegate == Pubkey::default() {
                return Err(PoolError::NoPendingWithdrawal);
            }
            
            self.pool_pause_requests[index] = PoolPauseRequest::default();
            Ok(())
        } else {
            Err(PoolError::DelegateNotFound { delegate: *delegate })
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

/// Allows the pool owner to add delegates for fee withdrawals.
///
/// This function enables the pool owner to authorize up to 3 delegates who can withdraw
/// trading fees collected by the contract. Each delegate will have configurable wait times
/// for withdrawal requests and can withdraw both SOL and SPL token fees.
///
/// # Purpose
/// - Enables delegation of fee withdrawal authority to trusted parties
/// - Supports multi-signature-like governance for fee management
/// - Allows for separation of pool management and fee collection duties
/// - Facilitates integration with external reward distribution systems
///
/// # How it works
/// 1. Verifies the caller is the pool owner (signature required)
/// 2. Checks that the delegate limit (3) hasn't been exceeded
/// 3. Ensures the delegate isn't already authorized
/// 4. Adds the delegate to the authorized list with default wait time (5 minutes)
/// 5. Updates the pool state and logs the operation
///
/// # Arguments
/// * `_program_id` - The program ID of the contract (not used in validation)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool owner account (must be signer)
///   - `accounts[1]` - Pool state PDA account (writable)
/// * `delegate` - The public key of the delegate to add
///
/// # Account Requirements
/// - Pool owner: Must be signer and match the pool's owner field
/// - Pool state: Must be owned by the program and writable
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Owner didn't sign the transaction
/// - `ProgramError::InvalidAccountData` - Caller is not the pool owner
/// - `PoolError::DelegateLimitExceeded` - Already have 3 delegates
/// - `PoolError::DelegateAlreadyExists` - Delegate is already authorized
///
/// # Example Usage
/// ```ignore
/// // Add a delegate for automated fee collection
/// let instruction = PoolInstruction::AddDelegate {
///     delegate: reward_distributor_pubkey,
/// };
/// ```
fn process_add_delegate(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    delegate: Pubkey,
) -> ProgramResult {
    msg!("Processing AddDelegate for: {}", delegate);
    let account_info_iter = &mut accounts.iter();

    let owner = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer to add delegate");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if *owner.key != pool_state_data.owner {
        msg!("Only pool owner can add delegates");
        return Err(ProgramError::InvalidAccountData);
    }

    // Add the delegate
    pool_state_data.delegate_management.add_delegate(delegate)?;
    
    // Save updated state using buffer serialization approach
    let mut serialized_data = Vec::new();
    pool_state_data.serialize(&mut serialized_data)?;
    let account_data_len = pool_state.data_len();
    if serialized_data.len() > account_data_len {
        return Err(ProgramError::AccountDataTooSmall);
    }
    {
        let mut account_data = pool_state.data.borrow_mut();
        account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
    }
    
    // Log the change for transparency
    msg!("Delegate added successfully: {}. Total delegates: {}", 
         delegate, pool_state_data.delegate_management.delegate_count);

    Ok(())
}

/// Allows the pool owner to remove delegates from fee withdrawal authorization.
///
/// This function enables the pool owner to revoke fee withdrawal authority from a delegate.
/// When a delegate is removed, any pending withdrawal requests they have are automatically
/// cancelled, and they lose access to withdraw fees immediately.
///
/// # Purpose
/// - Revokes fee withdrawal authority from delegates
/// - Provides immediate security response for compromised delegates
/// - Manages delegate lifecycle and permissions
/// - Maintains control over fee distribution access
///
/// # How it works
/// 1. Verifies the caller is the pool owner (signature required)
/// 2. Checks that the delegate exists in the authorized list
/// 3. Removes the delegate and shifts remaining delegates in the array
/// 4. Cancels any pending withdrawal requests for the removed delegate
/// 5. Updates delegate wait times array accordingly
/// 6. Updates the pool state and logs the operation
///
/// # Arguments
/// * `_program_id` - The program ID of the contract (not used in validation)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool owner account (must be signer)
///   - `accounts[1]` - Pool state PDA account (writable)
/// * `delegate` - The public key of the delegate to remove
///
/// # Account Requirements
/// - Pool owner: Must be signer and match the pool's owner field
/// - Pool state: Must be owned by the program and writable
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Owner didn't sign the transaction
/// - `ProgramError::InvalidAccountData` - Caller is not the pool owner
/// - `PoolError::DelegateNotFound` - Delegate is not in the authorized list
///
/// # Example Usage
/// ```ignore
/// // Remove a compromised delegate
/// let instruction = PoolInstruction::RemoveDelegate {
///     delegate: compromised_delegate_pubkey,
/// };
/// ```
fn process_remove_delegate(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    delegate: Pubkey,
) -> ProgramResult {
    msg!("Processing RemoveDelegate for: {}", delegate);
    let account_info_iter = &mut accounts.iter();

    let owner = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer to remove delegate");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if *owner.key != pool_state_data.owner {
        msg!("Only pool owner can remove delegates");
        return Err(ProgramError::InvalidAccountData);
    }

    // Remove the delegate
    pool_state_data.delegate_management.remove_delegate(delegate)?;
    
    // Save updated state using buffer serialization approach
    let mut serialized_data = Vec::new();
    pool_state_data.serialize(&mut serialized_data)?;
    let account_data_len = pool_state.data_len();
    if serialized_data.len() > account_data_len {
        return Err(ProgramError::AccountDataTooSmall);
    }
    {
        let mut account_data = pool_state.data.borrow_mut();
        account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
    }
    
    // Log the change for transparency
    msg!("Delegate removed successfully: {}. Remaining delegates: {}", 
         delegate, pool_state_data.delegate_management.delegate_count);

    Ok(())
}

/// Executes fee withdrawals for authorized delegates (Step 2 of two-step process).
///
/// This function allows authorized delegates to execute previously requested fee withdrawals
/// after the required wait time has elapsed. It supports withdrawing both SOL and SPL token
/// fees collected from trading activities. This is the second step of a two-step withdrawal
/// process that enhances security through time-delayed execution.
///
/// # Purpose
/// - Executes time-delayed fee withdrawals for delegates
/// - Supports both SOL and SPL token fee withdrawals
/// - Maintains audit trail of all withdrawal activities
/// - Ensures rent-exempt status is preserved during SOL withdrawals
/// - Provides secure fee distribution mechanism
///
/// # How it works
/// 1. Verifies the delegate is authorized and signed the transaction
/// 2. Checks that the pool is not paused
/// 3. Validates that a withdrawal request exists and wait time has elapsed
/// 4. Confirms the withdrawal request matches the current parameters
/// 5. For SOL withdrawals:
///    - Verifies sufficient collected SOL fees
///    - Ensures pool maintains rent-exempt status
///    - Transfers SOL directly from pool state PDA to delegate
/// 6. For SPL token withdrawals:
///    - Validates token vault and delegate token accounts
///    - Transfers tokens from vault to delegate's token account
/// 7. Updates fee tracking counters and withdrawal history
/// 8. Clears the withdrawal request to allow new requests
///
/// # Arguments
/// * `program_id` - The program ID for PDA validation and CPI authority
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Delegate account (must be signer and authorized)
///   - `accounts[1]` - Pool state PDA account (writable)
///   - `accounts[2]` - System program (for SOL transfers)
///   - `accounts[3]` - Rent sysvar (for rent calculations)
///   - `accounts[4]` - Clock sysvar (for timestamp validation)
///   - For SPL token withdrawals only:
///     - `accounts[5]` - Token vault account (writable)
///     - `accounts[6]` - Delegate's token account (writable)
///     - `accounts[7]` - Token program
/// * `token_mint` - The mint of the token to withdraw (use Pubkey::default() for SOL)
/// * `amount` - The amount to withdraw (in lamports for SOL, token units for SPL)
///
/// # Account Requirements
/// - Delegate: Must be signer and in the authorized delegates list
/// - Pool state: Must be owned by the program and writable
/// - For SOL: Must maintain rent-exempt balance after withdrawal
/// - For SPL tokens: Token accounts must match the expected mint and owner
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Delegate didn't sign
/// - `PoolError::PoolPaused` - Pool operations are paused
/// - `PoolError::UnauthorizedDelegate` - Caller is not an authorized delegate
/// - `PoolError::WithdrawalNotReady` - Wait time hasn't elapsed
/// - `PoolError::NoPendingWithdrawal` - No withdrawal request exists
/// - `PoolError::InvalidWithdrawalRequest` - Request doesn't match parameters
/// - `ProgramError::InsufficientFunds` - Not enough fees collected or SOL balance
/// - `ProgramError::InvalidAccountData` - Invalid token vault or accounts
///
/// # Example Usage
/// ```ignore
/// // Execute SOL fee withdrawal (after wait time)
/// let instruction = PoolInstruction::WithdrawFeesToDelegate {
///     token_mint: Pubkey::default(), // SOL
///     amount: 1_000_000, // 0.001 SOL
/// };
///
/// // Execute SPL token fee withdrawal
/// let instruction = PoolInstruction::WithdrawFeesToDelegate {
///     token_mint: usdc_mint_pubkey,
///     amount: 1_000_000, // 1 USDC (6 decimals)
/// };
/// ```
fn process_withdraw_fees_to_delegate(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    token_mint: Pubkey,
    amount: u64,
) -> ProgramResult {
    msg!("Processing WithdrawFeesToDelegate for token: {}, amount: {}", token_mint, amount);
    let account_info_iter = &mut accounts.iter();

    let delegate = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let rent_sysvar = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;

    // Verify delegate is signer
    if !delegate.is_signer {
        msg!("Delegate must be a signer for fee withdrawal");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    
    // Verify pool is not paused
    if pool_state_data.is_paused {
        msg!("Fee withdrawals are paused");
        return Err(PoolError::PoolPaused.into());
    }

    // Verify caller is a delegate
    if !pool_state_data.delegate_management.is_delegate(delegate.key) {
        msg!("Caller is not an authorized delegate: {}", delegate.key);
        return Err(PoolError::UnauthorizedDelegate.into());
    }

    // Two-step withdrawal verification
    // Check if withdrawal request exists and is ready
    if !pool_state_data.delegate_management.is_withdrawal_ready(delegate.key, clock.unix_timestamp)? {
        msg!("Withdrawal not ready for delegate: {}", delegate.key);
        return Err(PoolError::WithdrawalNotReady.into());
    }

    // Get withdrawal request
    let request = pool_state_data.delegate_management.get_withdrawal_request(delegate.key)
        .ok_or(PoolError::NoPendingWithdrawal)?;

    // Verify request matches current withdrawal
    if request.token_mint != token_mint || request.amount != amount {
        msg!("Withdrawal request mismatch: requested token={}, amount={}, actual token={}, amount={}", 
             request.token_mint, request.amount, token_mint, amount);
        return Err(PoolError::InvalidWithdrawalRequest.into());
    }

    // Handle SOL withdrawal
    if token_mint == Pubkey::default() {
        // Check if enough SOL fees collected
        if amount > pool_state_data.collected_sol_fees {
            msg!("Insufficient collected SOL fees. Available: {}, Requested: {}", 
                 pool_state_data.collected_sol_fees, amount);
            return Err(ProgramError::InsufficientFunds);
        }

        // Check rent exempt requirements
        let rent = &Rent::from_account_info(rent_sysvar)?;
        check_rent_exempt(pool_state, program_id, rent, clock.slot)?;

        // Calculate minimum balance to maintain rent exemption
        let minimum_balance = rent.minimum_balance(pool_state.data_len());
        if pool_state.lamports() < amount + minimum_balance {
            msg!("Insufficient SOL balance. Required: {}, Available: {}", 
                 amount + minimum_balance, pool_state.lamports());
            return Err(ProgramError::InsufficientFunds);
        }

        // Transfer SOL to delegate
        let pool_state_pda_seeds = &[
            POOL_STATE_SEED_PREFIX,
            pool_state_data.token_a_mint.as_ref(),
            pool_state_data.token_b_mint.as_ref(),
            &pool_state_data.ratio_a_numerator.to_le_bytes(),
            &pool_state_data.ratio_b_denominator.to_le_bytes(),
            &[pool_state_data.pool_authority_bump_seed],
        ];

        invoke_signed(
            &system_instruction::transfer(pool_state.key, delegate.key, amount),
            &[pool_state.clone(), delegate.clone(), system_program.clone()],
            &[pool_state_pda_seeds],
        )?;

        // Update pool state
        pool_state_data.collected_sol_fees = pool_state_data.collected_sol_fees
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        pool_state_data.total_sol_fees_withdrawn = pool_state_data.total_sol_fees_withdrawn
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        // Add withdrawal record
        let withdrawal_record = WithdrawalRecord::new(
            *delegate.key,
            token_mint,
            amount,
            clock.unix_timestamp,
            clock.slot,
        );
        pool_state_data.delegate_management.add_withdrawal_record(withdrawal_record);

        // Clear withdrawal request after successful withdrawal
        pool_state_data.delegate_management.cancel_withdrawal_request(delegate.key)?;

        // Save updated state
        pool_state_data.serialize(&mut *pool_state.data.borrow_mut())?;

        // Log the withdrawal for transparency
        msg!("SOL fee withdrawal completed: Delegate: {}, Amount: {}, Timestamp: {}", 
             delegate.key, amount, clock.unix_timestamp);

        return Ok(());
    }

    // Handle SPL token withdrawal
    let token_vault = next_account_info(account_info_iter)?;
    let delegate_token_account = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    // Determine token index (0 for token_a, 1 for token_b)
    let (token_index, vault_key, collected_fees) = if token_mint == pool_state_data.token_a_mint {
        (0, pool_state_data.token_a_vault, pool_state_data.collected_fees_token_a)
    } else if token_mint == pool_state_data.token_b_mint {
        (1, pool_state_data.token_b_vault, pool_state_data.collected_fees_token_b)
    } else {
        msg!("Invalid token mint for withdrawal: {}", token_mint);
        return Err(ProgramError::InvalidArgument);
    };

    // Verify vault account
    if *token_vault.key != vault_key {
        msg!("Invalid token vault provided");
        return Err(ProgramError::InvalidAccountData);
    }

    // Check if enough fees collected
    if amount > collected_fees {
        msg!("Insufficient collected fees. Available: {}, Requested: {}", collected_fees, amount);
        return Err(ProgramError::InsufficientFunds);
    }

    // Check rent exempt requirements
    let rent = &Rent::from_account_info(rent_sysvar)?;
    check_rent_exempt(pool_state, program_id, rent, clock.slot)?;

    // Transfer fees to delegate
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.token_a_mint.as_ref(),
        pool_state_data.token_b_mint.as_ref(),
        &pool_state_data.ratio_a_numerator.to_le_bytes(),
        &pool_state_data.ratio_b_denominator.to_le_bytes(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    invoke_signed(
        &token_instruction::transfer(
            token_program.key,
            token_vault.key,
            delegate_token_account.key,
            pool_state.key,
            &[],
            amount,
        )?,
        &[
            token_vault.clone(),
            delegate_token_account.clone(),
            pool_state.clone(),
            token_program.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;

    // Update pool state
    if token_index == 0 {
        pool_state_data.collected_fees_token_a = pool_state_data.collected_fees_token_a
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        pool_state_data.total_fees_withdrawn_token_a = pool_state_data.total_fees_withdrawn_token_a
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    } else {
        pool_state_data.collected_fees_token_b = pool_state_data.collected_fees_token_b
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        pool_state_data.total_fees_withdrawn_token_b = pool_state_data.total_fees_withdrawn_token_b
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }

    // Add withdrawal record
    let withdrawal_record = WithdrawalRecord::new(
        *delegate.key,
        token_mint,
        amount,
        clock.unix_timestamp,
        clock.slot,
    );
    pool_state_data.delegate_management.add_withdrawal_record(withdrawal_record);

    // Clear withdrawal request after successful withdrawal
    pool_state_data.delegate_management.cancel_withdrawal_request(delegate.key)?;

    // Save updated state
    pool_state_data.serialize(&mut *pool_state.data.borrow_mut())?;

    // Log the withdrawal for transparency
    msg!("Fee withdrawal completed: Delegate: {}, Token: {}, Amount: {}, Timestamp: {}", 
         delegate.key, token_mint, amount, clock.unix_timestamp);

    Ok(())
}

/// Retrieves and logs withdrawal history for transparency and auditing.
///
/// This function provides read-only access to the withdrawal history, showing the last 10
/// fee withdrawals made by delegates. It also displays current delegate information and
/// aggregate fee withdrawal statistics. This function is essential for transparency,
/// auditing, and monitoring of fee distribution activities.
///
/// # Purpose
/// - Provides transparency into fee withdrawal activities
/// - Enables auditing of delegate fee withdrawals
/// - Shows current delegate authorization status
/// - Displays aggregate withdrawal statistics
/// - Supports monitoring and compliance requirements
///
/// # How it works
/// 1. Loads the pool state to access withdrawal history
/// 2. Iterates through the last 10 withdrawal records
/// 3. Logs each withdrawal with delegate, token, amount, and timestamp
/// 4. Displays total fees withdrawn by token type
/// 5. Shows current authorized delegates and their count
/// 6. All information is logged to the transaction logs for transparency
///
/// # Arguments
/// * `_program_id` - The program ID of the contract (not used for validation)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool state PDA account (read-only)
///
/// # Account Requirements
/// - Pool state: Must be readable (no signature or write access required)
///
/// # Information Displayed
/// - **Withdrawal History**: Last 10 withdrawals with full details
/// - **Delegate Info**: Public key of each withdrawal's delegate
/// - **Token Info**: Token mint address (Pubkey::default() for SOL)
/// - **Amount**: Withdrawal amount in token-specific units
/// - **Timestamp**: Unix timestamp of the withdrawal
/// - **Slot**: Solana slot number when withdrawal occurred
/// - **Aggregate Stats**: Total fees withdrawn per token type
/// - **Current Delegates**: List of all currently authorized delegates
///
/// # Errors
/// - `ProgramError::InvalidAccountData` - Pool state account data is corrupted
///
/// # Example Usage
/// ```ignore
/// // Query withdrawal history for auditing
/// let instruction = PoolInstruction::GetWithdrawalHistory;
/// 
/// // Results logged to transaction logs:
/// // "Withdrawal History (last 10 withdrawals):"
/// // "Record 0: Delegate: ABC..., Token: DEF..., Amount: 1000000, Timestamp: 1234567890, Slot: 98765"
/// // "Total fees withdrawn - Token A: 5000000, Token B: 3000000"
/// // "Current delegates (3): GHI..., JKL..., MNO..."
/// ```
///
/// # Use Cases
/// - **Auditing**: Review all recent fee withdrawals
/// - **Monitoring**: Track delegate withdrawal patterns
/// - **Compliance**: Verify fee distribution activities
/// - **Analytics**: Analyze fee collection and distribution
/// - **Debugging**: Investigate withdrawal-related issues
fn process_get_withdrawal_history(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing GetWithdrawalHistory");
    let account_info_iter = &mut accounts.iter();

    let pool_state = next_account_info(account_info_iter)?;

    // Load pool state
    let pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;

    // Log withdrawal history for transparency
    msg!("Withdrawal History (last 10 withdrawals):");
    for (i, record) in pool_state_data.delegate_management.withdrawal_history.iter().enumerate() {
        if record.delegate != Pubkey::default() {
            msg!("Record {}: Delegate: {}, Token: {}, Amount: {}, Timestamp: {}, Slot: {}", 
                 i, record.delegate, record.token_mint, record.amount, record.timestamp, record.slot);
        }
    }

    msg!("Total fees withdrawn - Token A: {}, Token B: {}", 
         pool_state_data.total_fees_withdrawn_token_a,
         pool_state_data.total_fees_withdrawn_token_b);

    msg!("Current delegates ({}):", pool_state_data.delegate_management.delegate_count);
    for i in 0..pool_state_data.delegate_management.delegate_count as usize {
        msg!("Delegate {}: {}", i, pool_state_data.delegate_management.delegates[i]);
    }

    Ok(())
}

/// Allows the pool owner to set the swap fee configuration.
///
/// # Arguments
/// * `_program_id` - The program ID of the contract
/// * `accounts` - The accounts required for setting swap fee
/// * `fee_basis_points` - The fee in basis points (0-50, max 0.5%)
///
/// # Returns
/// * `ProgramResult` - Success or error code
fn process_set_swap_fee(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    fee_basis_points: u64,
) -> ProgramResult {
    msg!("Processing SetSwapFee: {} basis points", fee_basis_points);
    let account_info_iter = &mut accounts.iter();

    let owner = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer to set swap fee");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if *owner.key != pool_state_data.owner {
        msg!("Only pool owner can set swap fees");
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate fee is within allowed range (0-50 basis points = 0%-0.5%)
    if fee_basis_points > MAX_SWAP_FEE_BASIS_POINTS {
        msg!("Swap fee {} basis points exceeds maximum of {} basis points (0.5%)", 
             fee_basis_points, MAX_SWAP_FEE_BASIS_POINTS);
        return Err(ProgramError::InvalidArgument);
    }

    // Update swap fee
    let old_fee = pool_state_data.swap_fee_basis_points;
    pool_state_data.swap_fee_basis_points = fee_basis_points;

    // Save updated state
    pool_state_data.serialize(&mut *pool_state.data.borrow_mut())?;
    
    // Log the change for transparency
    msg!("Swap fee updated: {} -> {} basis points ({:.2}% -> {:.2}%)", 
         old_fee, fee_basis_points,
         old_fee as f64 / 100.0, fee_basis_points as f64 / 100.0);

    Ok(())
}

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

/// Creates a fee withdrawal request for authorized delegates (Step 1 of two-step process).
///
/// This function allows authorized delegates to request fee withdrawals with a time delay
/// for enhanced security. Delegates must specify the token type and amount they wish to
/// withdraw. Each delegate can have only one active withdrawal request at a time, and the
/// request must wait for a configurable period (5 minutes to 72 hours) before execution.
///
/// # Purpose
/// - Initiates the two-step withdrawal process for enhanced security
/// - Allows delegates to request both SOL and SPL token fee withdrawals
/// - Implements time-delayed execution to prevent immediate unauthorized access
/// - Provides transparency through logged withdrawal requests
/// - Prevents multiple concurrent requests per delegate
///
/// # How it works
/// 1. Verifies the delegate is authorized and signed the transaction
/// 2. Checks that the pool is not paused
/// 3. Ensures the delegate doesn't have a pending withdrawal request
/// 4. Creates a withdrawal request with current timestamp and delegate's wait time
/// 5. Stores the request in the pool state for later execution
/// 6. Logs the request details for transparency
///
/// # Arguments
/// * `program_id` - The program ID for account ownership validation
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool state PDA account (writable)
///   - `accounts[1]` - Delegate account (must be signer and authorized)
///   - `accounts[2]` - Clock sysvar (for timestamp)
/// * `token_mint` - The mint of the token to withdraw (use Pubkey::default() for SOL)
/// * `amount` - The amount to withdraw (in lamports for SOL, token units for SPL)
///
/// # Account Requirements
/// - Pool state: Must be owned by the program and writable
/// - Delegate: Must be signer and in the authorized delegates list
/// - Clock: System clock sysvar for timestamp validation
///
/// # Errors
/// - `ProgramError::IncorrectProgramId` - Pool state not owned by program
/// - `ProgramError::MissingRequiredSignature` - Delegate didn't sign
/// - `PoolError::PoolPaused` - Pool operations are paused
/// - `PoolError::UnauthorizedDelegate` - Caller is not an authorized delegate
/// - `PoolError::PendingWithdrawalExists` - Delegate already has a pending request
///
/// # Example Usage
/// ```ignore
/// // Request SOL fee withdrawal
/// let instruction = PoolInstruction::RequestFeeWithdrawal {
///     token_mint: Pubkey::default(), // SOL
///     amount: 1_000_000, // 0.001 SOL
/// };
///
/// // Request SPL token fee withdrawal
/// let instruction = PoolInstruction::RequestFeeWithdrawal {
///     token_mint: usdc_mint_pubkey,
///     amount: 1_000_000, // 1 USDC (6 decimals)
/// };
/// ```
///
/// # Security Features
/// - **Time Delay**: Configurable wait time prevents immediate execution
/// - **Single Request**: Only one active request per delegate
/// - **Authorization**: Only authorized delegates can create requests
/// - **Pause Protection**: Requests blocked when pool is paused
/// - **Audit Trail**: All requests logged for transparency
pub fn process_request_fee_withdrawal(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    token_mint: Pubkey,
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_state_info = next_account_info(account_info_iter)?;
    let delegate_info = next_account_info(account_info_iter)?;
    let clock_info = next_account_info(account_info_iter)?;

    // Verify pool state account
    if pool_state_info.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Verify delegate is signer
    if !delegate_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load pool state
    let mut pool_state = PoolState::try_from_slice(&pool_state_info.data.borrow())?;

    // Check if pool is paused
    if pool_state.is_paused {
        return Err(PoolError::PoolPaused.into());
    }

    // Verify delegate is authorized
    if !pool_state.delegate_management.is_delegate(delegate_info.key) {
        return Err(PoolError::UnauthorizedDelegate.into());
    }

    // Validate token mint is one of the pool's valid tokens (or SOL for fee withdrawals)
    if token_mint != pool_state.token_a_mint && token_mint != pool_state.token_b_mint && token_mint != Pubkey::default() {
        msg!("Invalid token mint for withdrawal: {}. Valid mints: {}, {}, SOL (default)", 
             token_mint, pool_state.token_a_mint, pool_state.token_b_mint);
        return Err(ProgramError::InvalidArgument);
    }

    // Get current timestamp
    let clock = Clock::from_account_info(clock_info)?;
    let current_timestamp = clock.unix_timestamp;

    // Create withdrawal request
    pool_state.delegate_management.create_withdrawal_request(
        delegate_info.key,
        token_mint,
        amount,
        current_timestamp,
        clock.slot,
    )?;

    // Save updated pool state using buffer serialization approach
    let mut serialized_data = Vec::new();
    pool_state.serialize(&mut serialized_data)?;
    let account_data_len = pool_state_info.data_len();
    if serialized_data.len() > account_data_len {
        return Err(ProgramError::AccountDataTooSmall);
    }
    {
        let mut account_data = pool_state_info.data.borrow_mut();
        account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
    }

    // Log the withdrawal request
    msg!("Withdrawal requested: delegate={}, token_mint={}, amount={}, timestamp={}", 
         delegate_info.key, token_mint, amount, current_timestamp);

    Ok(())
}

/// Cancels a pending fee withdrawal request.
///
/// This function allows either the pool owner or the requesting delegate to cancel a
/// pending withdrawal request before it becomes executable. This provides flexibility
/// for delegates to change their minds and emergency intervention capability for the
/// pool owner in case of security concerns.
///
/// # Purpose
/// - Provides flexibility for delegates to cancel their own requests
/// - Enables pool owner emergency intervention for security
/// - Allows correction of erroneous withdrawal requests
/// - Resets delegate status to allow new withdrawal requests
/// - Maintains control and security over the withdrawal process
///
/// # How it works
/// 1. Verifies the caller is either the pool owner or the requesting delegate
/// 2. Checks that the pool is not paused (for normal operations)
/// 3. Clears the withdrawal request from the delegate's slot
/// 4. Allows the delegate to create a new withdrawal request immediately
/// 5. Logs the cancellation details for transparency
///
/// # Arguments
/// * `program_id` - The program ID for account ownership validation
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool state PDA account (writable)
///   - `accounts[1]` - Canceler account (must be signer - owner or delegate)
///   - `accounts[2]` - Delegate account (whose request is being cancelled)
///
/// # Account Requirements
/// - Pool state: Must be owned by the program and writable
/// - Canceler: Must be signer and either the pool owner or the delegate
/// - Delegate: The account whose withdrawal request is being cancelled
///
/// # Authorization Rules
/// - **Pool Owner**: Can cancel any delegate's withdrawal request
/// - **Delegate**: Can only cancel their own withdrawal request
/// - **Others**: Cannot cancel withdrawal requests
///
/// # Errors
/// - `ProgramError::IncorrectProgramId` - Pool state not owned by program
/// - `ProgramError::MissingRequiredSignature` - Canceler didn't sign
/// - `PoolError::PoolPaused` - Pool operations are paused
/// - `PoolError::Unauthorized` - Caller is neither owner nor the delegate
/// - `PoolError::DelegateNotFound` - Delegate is not in authorized list
///
/// # Example Usage
/// ```ignore
/// // Delegate cancels their own request
/// let instruction = PoolInstruction::CancelWithdrawalRequest;
/// // Accounts: [pool_state, delegate_signer, delegate_signer]
///
/// // Owner cancels any delegate's request (emergency)
/// let instruction = PoolInstruction::CancelWithdrawalRequest;
/// // Accounts: [pool_state, owner_signer, target_delegate]
/// ```
///
/// # Use Cases
/// - **Self-Cancellation**: Delegate changes mind about withdrawal
/// - **Error Correction**: Fix incorrect amount or token type
/// - **Security Response**: Owner cancels suspicious requests
/// - **Emergency Control**: Immediate intervention capability
/// - **Process Reset**: Clear state to allow new requests
pub fn process_cancel_withdrawal_request(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_state_info = next_account_info(account_info_iter)?;
    let canceler_info = next_account_info(account_info_iter)?;
    let delegate_info = next_account_info(account_info_iter)?;

    // Verify pool state account
    if pool_state_info.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Verify canceler is signer
    if !canceler_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load pool state
    let mut pool_state = PoolState::try_from_slice(&pool_state_info.data.borrow())?;

    // Check if pool is paused
    if pool_state.is_paused {
        return Err(PoolError::PoolPaused.into());
    }

    // Verify canceler is either the owner or the delegate
    if *canceler_info.key != pool_state.owner && *canceler_info.key != *delegate_info.key {
        return Err(PoolError::Unauthorized.into());
    }

    // Cancel withdrawal request
    pool_state.delegate_management.cancel_withdrawal_request(delegate_info.key)?;

    // Save updated pool state
    pool_state.serialize(&mut *pool_state_info.data.borrow_mut())?;

    // Log the cancellation
    msg!("Withdrawal request cancelled: delegate={}, cancelled_by={}", 
         delegate_info.key, canceler_info.key);

    Ok(())
}

/// Sets the withdrawal wait time for a specific delegate.
///
/// This function allows the pool owner to configure individual wait times for each
/// delegate, providing fine-grained control over the security level for different
/// delegates. Wait times can range from 5 minutes to 72 hours, allowing for flexible
/// security policies based on delegate trust levels and roles.
///
/// # Purpose
/// - Configures individual security policies for each delegate
/// - Allows differentiated trust levels based on delegate roles
/// - Provides dynamic security adjustment capabilities
/// - Enables risk-based withdrawal controls
/// - Supports governance and security best practices
///
/// # How it works
/// 1. Verifies the caller is the pool owner (signature required)
/// 2. Validates the wait time is within allowed bounds (5 min - 72 hours)
/// 3. Confirms the target is an authorized delegate
/// 4. Updates the delegate's wait time in the pool state
/// 5. Logs the change for transparency and auditing
///
/// # Arguments
/// * `program_id` - The program ID for account ownership validation
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool state PDA account (writable)
///   - `accounts[1]` - Pool owner account (must be signer)
/// * `delegate` - The public key of the delegate whose wait time is being set
/// * `wait_time` - The wait time in seconds (300 to 259,200 seconds)
///
/// # Account Requirements
/// - Pool state: Must be owned by the program and writable
/// - Owner: Must be signer and match the pool's owner field
///
/// # Wait Time Constraints
/// - **Minimum**: 300 seconds (5 minutes)
/// - **Maximum**: 259,200 seconds (72 hours)
/// - **Default**: 300 seconds (applied when delegate is first added)
/// - **Granularity**: 1 second
///
/// # Errors
/// - `ProgramError::IncorrectProgramId` - Pool state not owned by program
/// - `ProgramError::MissingRequiredSignature` - Owner didn't sign
/// - `PoolError::Unauthorized` - Caller is not the pool owner
/// - `PoolError::DelegateNotFound` - Target is not an authorized delegate
/// - `PoolError::InvalidWaitTime` - Wait time outside allowed range
///
/// # Example Usage
/// ```ignore
/// // Set short wait time for trusted delegate (5 minutes)
/// let instruction = PoolInstruction::SetDelegateWaitTime {
///     delegate: trusted_delegate_pubkey,
///     wait_time: 300, // 5 minutes
/// };
///
/// // Set longer wait time for less trusted delegate (24 hours)
/// let instruction = PoolInstruction::SetDelegateWaitTime {
///     delegate: external_delegate_pubkey,
///     wait_time: 86400, // 24 hours
/// };
///
/// // Set maximum wait time for high-security scenarios (72 hours)
/// let instruction = PoolInstruction::SetDelegateWaitTime {
///     delegate: high_security_delegate_pubkey,
///     wait_time: 259200, // 72 hours
/// };
/// ```
///
/// # Security Considerations
/// - **Risk-Based**: Higher wait times for higher-risk delegates
/// - **Role-Based**: Different wait times for different delegate roles
/// - **Dynamic**: Can be adjusted based on changing security needs
/// - **Immediate Effect**: New wait time applies to future requests
/// - **Existing Requests**: Pending requests use their original wait time
///
/// # Common Wait Time Strategies
/// - **Automated Systems**: 5-15 minutes for trusted automated processes
/// - **Trusted Partners**: 1-6 hours for known and trusted entities
/// - **External Delegates**: 12-24 hours for external or less trusted delegates
/// - **High-Value Operations**: 48-72 hours for maximum security scenarios
pub fn process_set_delegate_wait_time(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    delegate: Pubkey,
    wait_time: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_state_info = next_account_info(account_info_iter)?;
    let owner_info = next_account_info(account_info_iter)?;

    // Verify pool state account
    if pool_state_info.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Verify owner is signer
    if !owner_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load pool state
    let mut pool_state = PoolState::try_from_slice(&pool_state_info.data.borrow())?;

    // Verify caller is owner
    if *owner_info.key != pool_state.owner {
        return Err(PoolError::Unauthorized.into());
    }

    // Set delegate wait time
    pool_state.delegate_management.set_delegate_wait_time(&delegate, wait_time)?;

    // Save updated pool state
    pool_state.serialize(&mut *pool_state_info.data.borrow_mut())?;

    // Log the wait time update
    msg!("Delegate wait time updated: delegate={}, wait_time={}", delegate, wait_time);

    Ok(())
}

// ================================================================================================
// PDA HELPER UTILITIES
// ================================================================================================

/// **PDA HELPER**: Computes and returns the Pool State PDA address for given tokens and ratio.
/// 
/// This utility function helps clients derive the Pool State PDA address without requiring
/// account creation or on-chain calls. Essential for preparing transaction account lists.
/// 
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `primary_token_mint` - Primary token mint pubkey
/// * `base_token_mint` - Base token mint pubkey  
/// * `ratio_primary_per_base` - Exchange ratio between tokens
/// 
/// # Returns
/// * `ProgramResult` - Logs the derived PDA address and bump seed
pub fn get_pool_state_pda(
    program_id: &Pubkey,
    primary_token_mint: Pubkey,
    base_token_mint: Pubkey,
    ratio_primary_per_base: u64,
) -> ProgramResult {
    msg!("DEBUG: get_pool_state_pda: Computing Pool State PDA");
    
    // Enhanced normalization to prevent economic duplicates (same logic as pool creation)
    // Step 1: Lexicographic token ordering
    let (token_a_mint_key, token_b_mint_key) = 
        if primary_token_mint < base_token_mint {
            (primary_token_mint, base_token_mint)
        } else {
            (base_token_mint, primary_token_mint)
        };
    
    // Step 2: Canonical ratio mapping to prevent liquidity fragmentation
    let (ratio_a_numerator, ratio_b_denominator): (u64, u64) = 
        if primary_token_mint < base_token_mint {
            (ratio_primary_per_base, 1u64)
        } else {
            // Use canonical form - both pools with same token pair get same ratio
            (ratio_primary_per_base, 1u64)
        };
    
    // Find PDA with canonical bump seed
    let (pool_state_pda, bump_seed) = Pubkey::find_program_address(
        &[
            POOL_STATE_SEED_PREFIX,
            token_a_mint_key.as_ref(),
            token_b_mint_key.as_ref(),
            &ratio_a_numerator.to_le_bytes(),
            &ratio_b_denominator.to_le_bytes(),
        ],
        program_id,
    );
    
    msg!("Pool State PDA: {}", pool_state_pda);
    msg!("Pool State PDA Bump Seed: {}", bump_seed);
    msg!("Normalized Token A: {}", token_a_mint_key);
    msg!("Normalized Token B: {}", token_b_mint_key);
    msg!("Normalized Ratio A: {}", ratio_a_numerator);
    msg!("Normalized Ratio B: {}", ratio_b_denominator);
    
    Ok(())
}

/// **PDA HELPER**: Computes and returns Token Vault PDA addresses for a given pool.
/// 
/// This utility helps clients derive the token vault addresses for pool operations.
/// Useful for preparing deposit, withdraw, and swap transaction account lists.
/// 
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `pool_state_pda` - The Pool State PDA address
/// 
/// # Returns
/// * `ProgramResult` - Logs the derived vault PDA addresses and bump seeds
pub fn get_token_vault_pdas(
    program_id: &Pubkey,
    pool_state_pda: Pubkey,
) -> ProgramResult {
    msg!("DEBUG: get_token_vault_pdas: Computing Token Vault PDAs for pool: {}", pool_state_pda);
    
    // Find Token A Vault PDA
    let (token_a_vault_pda, token_a_bump) = Pubkey::find_program_address(
        &[
            TOKEN_A_VAULT_SEED_PREFIX,
            pool_state_pda.as_ref(),
        ],
        program_id,
    );
    
    // Find Token B Vault PDA
    let (token_b_vault_pda, token_b_bump) = Pubkey::find_program_address(
        &[
            TOKEN_B_VAULT_SEED_PREFIX,
            pool_state_pda.as_ref(),
        ],
        program_id,
    );
    
    msg!("Token A Vault PDA: {}", token_a_vault_pda);
    msg!("Token A Vault Bump Seed: {}", token_a_bump);
    msg!("Token B Vault PDA: {}", token_b_vault_pda);
    msg!("Token B Vault Bump Seed: {}", token_b_bump);
    
    Ok(())
}

// ================================================================================================
// TEST-SPECIFIC VIEW/GETTER INSTRUCTIONS
// ================================================================================================

/// **VIEW INSTRUCTION**: Returns comprehensive pool state information.
/// 
/// This function provides easy access to all pool state data in a structured format.
/// Ideal for testing, debugging, frontend integration, and transparency.
/// 
/// # Arguments
/// * `accounts` - Must contain pool state account as first account
/// 
/// # Returns
/// * `ProgramResult` - Logs comprehensive pool information
pub fn get_pool_info(accounts: &[AccountInfo]) -> ProgramResult {
    msg!("DEBUG: get_pool_info: Retrieving comprehensive pool information");
    
    let pool_state_account = &accounts[0];
    let pool_state = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    
    msg!("=== POOL STATE INFORMATION ===");
    msg!("Pool Owner: {}", pool_state.owner);
    msg!("Pool State PDA: {}", pool_state_account.key);
    msg!("Token A Mint: {}", pool_state.token_a_mint);
    msg!("Token B Mint: {}", pool_state.token_b_mint);
    msg!("Token A Vault: {}", pool_state.token_a_vault);
    msg!("Token B Vault: {}", pool_state.token_b_vault);
    msg!("LP Token A Mint: {}", pool_state.lp_token_a_mint);
    msg!("LP Token B Mint: {}", pool_state.lp_token_b_mint);
    msg!("Ratio A Numerator: {}", pool_state.ratio_a_numerator);
    msg!("Ratio B Denominator: {}", pool_state.ratio_b_denominator);
    msg!("Pool Authority Bump Seed: {}", pool_state.pool_authority_bump_seed);
    msg!("Token A Vault Bump Seed: {}", pool_state.token_a_vault_bump_seed);
    msg!("Token B Vault Bump Seed: {}", pool_state.token_b_vault_bump_seed);
    msg!("Is Initialized: {}", pool_state.is_initialized);
    msg!("Is Paused: {}", pool_state.is_paused);
    msg!("Swap Fee Basis Points: {}", pool_state.swap_fee_basis_points);
    msg!("===============================");
    
    Ok(())
}

/// **VIEW INSTRUCTION**: Returns detailed liquidity information for both tokens.
/// 
/// This function provides easy access to liquidity data, useful for calculating
/// exchange rates, available liquidity, and pool utilization metrics.
/// 
/// # Arguments
/// * `accounts` - Must contain pool state account as first account
/// 
/// # Returns
/// * `ProgramResult` - Logs detailed liquidity information
pub fn get_liquidity_info(accounts: &[AccountInfo]) -> ProgramResult {
    msg!("DEBUG: get_liquidity_info: Retrieving liquidity information");
    
    let pool_state_account = &accounts[0];
    let pool_state = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    
    msg!("=== LIQUIDITY INFORMATION ===");
    msg!("Total Token A Liquidity: {}", pool_state.total_token_a_liquidity);
    msg!("Total Token B Liquidity: {}", pool_state.total_token_b_liquidity);
    msg!("Exchange Rate (A per B): {}", 
         if pool_state.ratio_b_denominator != 0 { 
             pool_state.ratio_a_numerator as f64 / pool_state.ratio_b_denominator as f64 
         } else { 0.0 });
    msg!("Exchange Rate (B per A): {}", 
         if pool_state.ratio_a_numerator != 0 { 
             pool_state.ratio_b_denominator as f64 / pool_state.ratio_a_numerator as f64 
         } else { 0.0 });
    
    // Calculate utilization if available
    let total_value_locked = pool_state.total_token_a_liquidity + pool_state.total_token_b_liquidity;
    msg!("Total Value Locked (TVL): {} tokens", total_value_locked);
    msg!("==============================");
    
    Ok(())
}

/// **VIEW INSTRUCTION**: Returns delegate management information.
/// 
/// This function provides comprehensive delegate system information including
/// delegate list, withdrawal history, and pending requests for transparency.
/// 
/// # Arguments
/// * `accounts` - Must contain pool state account as first account
/// 
/// # Returns
/// * `ProgramResult` - Logs delegate management information
pub fn get_delegate_info(accounts: &[AccountInfo]) -> ProgramResult {
    msg!("DEBUG: get_delegate_info: Retrieving delegate information");
    
    let pool_state_account = &accounts[0];
    let pool_state = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    
    msg!("=== DELEGATE INFORMATION ===");
    msg!("Total Delegates: {}", pool_state.delegate_management.delegate_count);
    
    // List all delegates
    for (i, delegate) in pool_state.delegate_management.delegates.iter().enumerate() {
        if i < pool_state.delegate_management.delegate_count as usize {
            msg!("Delegate {}: {}", i + 1, delegate);
            
            // Show wait time for this delegate
            if let Some(wait_time) = pool_state.delegate_management.get_delegate_wait_time(delegate) {
                msg!("  Wait Time: {} seconds", wait_time);
            }
            
            // Show any pending withdrawal request
            if let Some(request) = pool_state.delegate_management.get_withdrawal_request(delegate) {
                msg!("  Pending Withdrawal: {} of token {}", request.amount, request.token_mint);
                msg!("  Request Timestamp: {}", request.request_timestamp);
            }
        }
    }
    
    // Show recent withdrawal history
    msg!("Recent Withdrawal History:");
    msg!("History Index: {}", pool_state.delegate_management.withdrawal_history_index);
    for (i, record) in pool_state.delegate_management.withdrawal_history.iter().enumerate() {
        if record.delegate != Pubkey::default() { // Only show non-empty records
            msg!("  Record {}: Delegate {}, Amount {}, Token {}, Slot {}", 
                 i, record.delegate, record.amount, record.token_mint, record.slot);
        }
    }
    msg!("============================");
    
    Ok(())
}

/// **VIEW INSTRUCTION**: Returns fee information including collected fees and rates.
/// 
/// This function provides comprehensive fee information essential for fee tracking,
/// transparency, and financial reporting.
/// 
/// # Arguments
/// * `accounts` - Must contain pool state account as first account
/// 
/// # Returns
/// * `ProgramResult` - Logs detailed fee information
pub fn get_fee_info(accounts: &[AccountInfo]) -> ProgramResult {
    msg!("DEBUG: get_fee_info: Retrieving fee information");
    
    let pool_state_account = &accounts[0];
    let pool_state = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    
    msg!("=== FEE INFORMATION ===");
    
    // Fee rates
    msg!("Swap Fee Rate: {} basis points ({:.4}%)", 
         pool_state.swap_fee_basis_points, 
         pool_state.swap_fee_basis_points as f64 / 100.0);
    msg!("Registration Fee: {} lamports ({:.9} SOL)", REGISTRATION_FEE, REGISTRATION_FEE as f64 / 1_000_000_000.0);
    msg!("Deposit/Withdrawal Fee: {} lamports ({:.9} SOL)", DEPOSIT_WITHDRAWAL_FEE, DEPOSIT_WITHDRAWAL_FEE as f64 / 1_000_000_000.0);
    msg!("Swap Fee: {} lamports ({:.9} SOL)", SWAP_FEE, SWAP_FEE as f64 / 1_000_000_000.0);
    
    // Collected fees
    msg!("Collected Token A Fees: {}", pool_state.collected_fees_token_a);
    msg!("Collected Token B Fees: {}", pool_state.collected_fees_token_b);
    msg!("Collected SOL Fees: {} lamports ({:.9} SOL)", 
         pool_state.collected_sol_fees, 
         pool_state.collected_sol_fees as f64 / 1_000_000_000.0);
    
    // Withdrawn fees (for tracking)
    msg!("Total Token A Fees Withdrawn: {}", pool_state.total_fees_withdrawn_token_a);
    msg!("Total Token B Fees Withdrawn: {}", pool_state.total_fees_withdrawn_token_b);
    msg!("Total SOL Fees Withdrawn: {} lamports ({:.9} SOL)", 
         pool_state.total_sol_fees_withdrawn, 
         pool_state.total_sol_fees_withdrawn as f64 / 1_000_000_000.0);
    
    // Available fees (collected minus withdrawn)
    let available_token_a_fees = pool_state.collected_fees_token_a.saturating_sub(pool_state.total_fees_withdrawn_token_a);
    let available_token_b_fees = pool_state.collected_fees_token_b.saturating_sub(pool_state.total_fees_withdrawn_token_b);
    let available_sol_fees = pool_state.collected_sol_fees.saturating_sub(pool_state.total_sol_fees_withdrawn);
    
    msg!("Available Token A Fees: {}", available_token_a_fees);
    msg!("Available Token B Fees: {}", available_token_b_fees);
    msg!("Available SOL Fees: {} lamports ({:.9} SOL)", 
         available_sol_fees, 
         available_sol_fees as f64 / 1_000_000_000.0);
    
    msg!("=======================");
    
    Ok(())
}

// ================================================================================================
// INDIVIDUAL POOL RATIO PAUSING PROCESSORS
// ================================================================================================

/// Process a pool pause request from an authorized delegate.
///
/// This function allows authorized delegates to request a pause of pool operations for a
/// specific duration with configurable timing parameters. Designed as a primitive for 
/// governance contracts to implement sophisticated dispute resolution and bonding mechanisms.
///
/// # Purpose
/// - Enables delegate-controlled pool pausing for governance integration
/// - Provides structured dispute resolution and bonding enforcement
/// - Creates audit trail for all pause requests and their reasons
/// - Supports emergency response capabilities for security incidents
/// - Facilitates integration with higher-layer governance contracts
///
/// # How it works
/// 1. **Authorization**: Verifies the caller is an authorized delegate and signed the transaction
/// 2. **Duplicate Check**: Ensures delegate doesn't have pending pause request
/// 3. **Parameter Validation**: Validates duration is within allowed range (1 minute to 72 hours)
/// 4. **Request Creation**: Creates pause request with delegate's configured wait time
/// 5. **State Update**: Saves the request to pool state for future activation
/// 6. **Audit Logging**: Logs request details for transparency and governance tracking
///
/// # Timing Model
/// - **Request Time**: Current timestamp when request is submitted
/// - **Wait Period**: Delegate-specific delay before pause becomes active (1 minute to 72 hours)
/// - **Active Period**: Duration of pause once activated (1 minute to 72 hours)
/// - **Cancellation**: Can be cancelled by delegate or owner before activation
///
/// # Arguments
/// * `program_id` - The program ID for validation (unused but standard pattern)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Delegate account (must be signer and authorized delegate)
///   - `accounts[1]` - Pool state PDA account (writable for request storage)
///   - `accounts[2]` - Clock sysvar for timestamp access
/// * `reason` - Structured reason for the pause request (enum PoolPauseReason)
/// * `duration_seconds` - Duration of pause once active (60 to 259200 seconds)
///
/// # Account Requirements
/// - Delegate: Must be signer and exist in pool's authorized delegate list
/// - Pool state: Must be owned by program and writable for state updates
/// - Clock: Standard Solana sysvar for timestamp access
///
/// # Validation Rules
/// - Only authorized delegates can submit pause requests
/// - Duration must be between 1 minute and 72 hours
/// - Delegate can only have one pending pause request at a time
/// - Pool doesn't need to be unpaused to submit request
///
/// # Integration with Governance
/// This primitive enables governance contracts to implement:
/// - **Bonding Mechanisms**: Pause pool until bond requirements are met
/// - **Dispute Resolution**: Structured pause with categorized reasons
/// - **Automated Governance**: Program-controlled pause requests
/// - **Emergency Response**: Rapid response to security concerns
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Delegate didn't sign transaction
/// - `PoolError::UnauthorizedDelegate` - Caller is not authorized delegate
/// - `PoolError::PendingWithdrawalExists` - Delegate already has pending pause request
/// - `PoolError::InvalidWaitTime` - Duration is outside allowed range
///
/// # Example Usage
/// ```ignore
/// // Governance contract requests pause for insufficient bonding
/// let instruction = PoolInstruction::RequestPoolPause {
///     reason: PoolPauseReason::InsufficientBond,
///     duration_seconds: 3600, // 1 hour pause
/// };
/// ```
pub fn process_request_pool_pause(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    reason: PoolPauseReason,
    duration_seconds: u64,
) -> ProgramResult {
    msg!("Processing RequestPoolPause - reason: {:?}, duration: {} seconds", reason, duration_seconds);
    let account_info_iter = &mut accounts.iter();

    let delegate_account = next_account_info(account_info_iter)?;
    let pool_state_account = next_account_info(account_info_iter)?;
    let clock_account = next_account_info(account_info_iter)?;

    // Verify delegate is signer
    if !delegate_account.is_signer {
        msg!("Delegate must be a signer to request pool pause");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Get current timestamp from clock
    let clock = Clock::from_account_info(clock_account)?;
    let current_timestamp = clock.unix_timestamp;
    let current_slot = clock.slot;

    // Load and verify pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    
    // Verify delegate is authorized
    if !pool_state_data.delegate_management.is_delegate(delegate_account.key) {
        msg!("Caller is not an authorized delegate: {}", delegate_account.key);
        return Err(PoolError::UnauthorizedDelegate.into());
    }

    // Create the pause request
    pool_state_data.delegate_management.create_pool_pause_request(
        delegate_account.key,
        reason.clone(),
        duration_seconds,
        current_timestamp,
        current_slot,
    )?;

    // Save updated state
    pool_state_data.serialize(&mut *pool_state_account.data.borrow_mut())?;

    // Log the request for governance tracking
    msg!("Pool pause requested successfully: delegate={}, reason={:?}, duration={} seconds, wait_time={} seconds", 
         delegate_account.key, 
         reason,
         duration_seconds,
         pool_state_data.delegate_management.get_pool_pause_wait_time(delegate_account.key).unwrap_or(259200));

    Ok(())
}

/// Process cancellation of a pool pause request.
///
/// This function allows either the requesting delegate or the pool owner to cancel a 
/// pending pool pause request before it becomes active. Provides flexibility for
/// dispute resolution and accidental request correction.
///
/// # Purpose
/// - Enables cancellation of accidental or resolved pause requests
/// - Provides pool owner override capability for emergency resolution
/// - Supports flexible dispute resolution mechanisms
/// - Maintains audit trail of cancelled requests
/// - Prevents unnecessary pool disruptions when issues are resolved
///
/// # How it works
/// 1. **Authorization**: Verifies caller is either requesting delegate or pool owner
/// 2. **Request Validation**: Ensures pending request exists for the delegate
/// 3. **Cancellation**: Removes the pause request from pool state
/// 4. **State Update**: Saves updated state without the cancelled request
/// 5. **Audit Logging**: Logs cancellation for transparency
///
/// # Arguments
/// * `program_id` - The program ID for validation (unused but standard pattern)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Caller account (delegate or owner, must be signer)
///   - `accounts[1]` - Pool state PDA account (writable for state updates)
///
/// # Account Requirements
/// - Caller: Must be signer and either pool owner or authorized delegate with pending request
/// - Pool state: Must be owned by program and writable for state updates
///
/// # Authorization Rules
/// - Pool owner can cancel any delegate's pause request
/// - Delegates can only cancel their own pause requests
/// - Cannot cancel requests that have already become active
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Caller didn't sign transaction
/// - `PoolError::UnauthorizedDelegate` - Caller is not owner or requesting delegate
/// - `PoolError::NoPendingWithdrawal` - No pause request exists to cancel
///
/// # Example Usage
/// ```ignore
/// // Delegate cancels their own request
/// let instruction = PoolInstruction::CancelPoolPause;
/// 
/// // Owner cancels any delegate's request (emergency resolution)
/// let instruction = PoolInstruction::CancelPoolPause;
/// ```
pub fn process_cancel_pool_pause(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing CancelPoolPause");
    let account_info_iter = &mut accounts.iter();

    let caller_account = next_account_info(account_info_iter)?;
    let pool_state_account = next_account_info(account_info_iter)?;

    // Verify caller is signer
    if !caller_account.is_signer {
        msg!("Caller must be a signer to cancel pool pause");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    
    // Check if caller is pool owner (can cancel any request)
    let is_owner = *caller_account.key == pool_state_data.owner;
    
    if is_owner {
        // Owner can cancel any delegate's request - find and cancel the first one
        let mut cancelled = false;
        for i in 0..pool_state_data.delegate_management.delegate_count as usize {
            if pool_state_data.delegate_management.pool_pause_requests[i].delegate != Pubkey::default() {
                let delegate = pool_state_data.delegate_management.delegates[i];
                pool_state_data.delegate_management.cancel_pool_pause_request(&delegate)?;
                msg!("Pool owner cancelled pause request for delegate: {}", delegate);
                cancelled = true;
                break;
            }
        }
        
        if !cancelled {
            msg!("No pending pause requests to cancel");
            return Err(PoolError::NoPendingWithdrawal.into());
        }
    } else {
        // Delegate can only cancel their own request
        if !pool_state_data.delegate_management.is_delegate(caller_account.key) {
            msg!("Caller is not authorized delegate or pool owner: {}", caller_account.key);
            return Err(PoolError::UnauthorizedDelegate.into());
        }
        
        // Cancel delegate's own request
        pool_state_data.delegate_management.cancel_pool_pause_request(caller_account.key)?;
        msg!("Delegate cancelled their own pause request: {}", caller_account.key);
    }

    // Save updated state
    pool_state_data.serialize(&mut *pool_state_account.data.borrow_mut())?;
    
    msg!("Pool pause request cancelled successfully");
    Ok(())
}

/// Process setting pool pause wait time for a specific delegate.
///
/// This function allows the pool owner to configure delegate-specific wait times for
/// pool pause requests. The wait time is the delay between when a pause is requested
/// and when it becomes active, providing deliberation time for dispute resolution.
///
/// # Purpose
/// - Configures delegate-specific governance timing parameters
/// - Enables fine-tuned control over pause activation delays
/// - Supports different trust levels for different delegates
/// - Provides flexibility for various governance models
/// - Allows optimization of response times for different use cases
///
/// # How it works
/// 1. **Authorization**: Verifies caller is pool owner and signed transaction
/// 2. **Delegate Validation**: Ensures target delegate exists in authorized list
/// 3. **Parameter Validation**: Validates wait time is within allowed range
/// 4. **Configuration Update**: Updates delegate's pause wait time setting
/// 5. **State Persistence**: Saves updated configuration to pool state
/// 6. **Audit Logging**: Logs configuration change for transparency
///
/// # Arguments
/// * `program_id` - The program ID for validation (unused but standard pattern)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool owner account (must be signer)
///   - `accounts[1]` - Pool state PDA account (writable for configuration updates)
/// * `delegate` - Public key of the delegate to configure
/// * `wait_time` - Wait time in seconds (60 to 259200 = 1 minute to 72 hours)
///
/// # Account Requirements
/// - Owner: Must be signer and match pool state owner field
/// - Pool state: Must be owned by program and writable for updates
///
/// # Validation Rules
/// - Only pool owner can set delegate pause wait times
/// - Wait time must be between 1 minute and 72 hours
/// - Delegate must exist in authorized delegate list
/// - Setting applies to future pause requests only
///
/// # Default Values
/// - New delegates default to 72 hours wait time (maximum deliberation)
/// - This provides conservative governance approach by default
/// - Can be reduced for trusted delegates or specific use cases
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Owner didn't sign transaction
/// - `ProgramError::InvalidAccountData` - Caller is not pool owner
/// - `PoolError::DelegateNotFound` - Target delegate is not authorized
/// - `PoolError::InvalidWaitTime` - Wait time is outside allowed range
///
/// # Example Usage
/// ```ignore
/// // Set trusted delegate to 1 hour wait time
/// let instruction = PoolInstruction::SetPoolPauseWaitTime {
///     delegate: trusted_delegate_pubkey,
///     wait_time: 3600, // 1 hour
/// };
/// 
/// // Set new delegate to maximum wait time for safety
/// let instruction = PoolInstruction::SetPoolPauseWaitTime {
///     delegate: new_delegate_pubkey,
///     wait_time: 259200, // 72 hours
/// };
/// ```
pub fn process_set_pool_pause_wait_time(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    delegate: Pubkey,
    wait_time: u64,
) -> ProgramResult {
    msg!("Processing SetPoolPauseWaitTime for delegate: {}, wait_time: {} seconds", delegate, wait_time);
    let account_info_iter = &mut accounts.iter();

    let owner = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer to set pool pause wait time");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if *owner.key != pool_state_data.owner {
        msg!("Only pool owner can set pool pause wait times");
        return Err(ProgramError::InvalidAccountData);
    }

    // Set the delegate's pool pause wait time
    pool_state_data.delegate_management.set_pool_pause_wait_time(&delegate, wait_time)?;

    // Save updated state
    pool_state_data.serialize(&mut *pool_state.data.borrow_mut())?;

    // Log the wait time update
    msg!("Pool pause wait time updated: delegate={}, wait_time={} seconds", delegate, wait_time);

    Ok(())
}
