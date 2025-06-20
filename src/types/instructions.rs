//! Pool Instructions
//! 
//! This module contains all the instruction definitions for the Solana Trading Pool Program.
//! Instructions define the operations that can be performed on the pool.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;
use super::delegate_actions::{DelegateActionType, DelegateActionParams, DelegateTimeLimits};

/// All supported instructions for the Solana Trading Pool Program.
/// 
/// This enum defines every operation that can be performed on the pool,
/// from initialization and liquidity management to delegate operations
/// and governance functions.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
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
    
    /// Withdraw liquidity from the pool by burning LP tokens
    Withdraw {
        withdraw_token_mint: Pubkey,
        lp_amount_to_burn: u64,
    },
    
    /// Swap tokens at the fixed ratio
    Swap {
        input_token_mint: Pubkey,
        amount_in: u64,
        minimum_amount_out: u64,
    },
    
    /// Updates security parameters for the pool
    UpdateSecurityParams {
        /// Whether to pause pool operations
        is_paused: Option<bool>,
    },
    
    /// Delegate Management Instructions
    AddDelegate {
        delegate: Pubkey,
    },
    
    /// Remove a delegate from authorization
    RemoveDelegate {
        delegate: Pubkey,
    },
    
    /// Request a delegate action (consolidated instruction)
    RequestDelegateAction {
        /// Type of action being requested
        action_type: DelegateActionType,
        /// Parameters for the action
        params: DelegateActionParams,
    },

    /// Execute a pending delegate action
    /// Required accounts for fee withdrawal:
    /// - Signer (executor)
    /// - Pool state PDA
    /// - Clock sysvar
    /// - Delegate's token account (for receiving fees)
    /// - Token program
    /// - Pool vault account (for the token being withdrawn)
    ExecuteDelegateAction {
        /// ID of the action to execute
        action_id: u64,
    },

    /// Revoke a pending delegate action
    RevokeAction {
        /// ID of the action to revoke
        action_id: u64,
    },

    /// Set time limits for delegate actions
    SetDelegateTimeLimits {
        /// Delegate to set limits for
        delegate: Pubkey,
        /// New time limits
        time_limits: DelegateTimeLimits,
    },
    
    /// Get withdrawal history (for transparency)
    GetWithdrawalHistory,
    
    /// Get pool state PDA address for given tokens and ratio
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
    
    /// Withdraws accumulated SOL fees from the pool state account
    /// Only the pool owner can withdraw fees
    /// Maintains rent exemption for pool state account
    WithdrawFees,
    
    /// Pause the entire system - blocks all operations except unpause
    /// Only the system authority can execute this instruction
    /// Takes precedence over all pool-specific pause states
    PauseSystem {
        /// Human-readable reason for the system pause
        reason: String,
    },
    
    /// Unpause the entire system - allows all operations to resume
    /// Only the system authority can execute this instruction
    /// Clears the system pause state completely
    UnpauseSystem,
    
} 