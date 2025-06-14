//! Pool Instructions
//! 
//! This module contains all the instruction definitions for the Solana Trading Pool Program.
//! Instructions define the operations that can be performed on the pool.

use crate::types::PoolPauseReason;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// All supported instructions for the Solana Trading Pool Program.
/// 
/// This enum defines every operation that can be performed on the pool,
/// from initialization and liquidity management to delegate operations
/// and governance functions.
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
    
    /// Owner withdraws accumulated SOL fees
    WithdrawFees,
    
    /// Update security parameters for the pool
    UpdateSecurityParams {
        max_withdrawal_percentage: Option<u64>,
        withdrawal_cooldown: Option<u64>,
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
    
    /// Request a time-delayed fee withdrawal
    RequestFeeWithdrawal {
        token_mint: Pubkey,
        amount: u64,
    },
    
    /// Cancel a pending withdrawal request
    CancelWithdrawalRequest,
    
    /// Set withdrawal wait time for a specific delegate
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