//! Pool Instructions
//! 
//! This module contains all the instruction definitions for the Solana Trading Pool Program.
//! Instructions define the operations that can be performed on the pool.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// All supported instructions for the Solana Trading Pool Program.
/// 
/// This enum defines every operation that can be performed on the pool,
/// from initialization and liquidity management to owner-only operations.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum PoolInstruction {
    /// **DEPRECATED**: Use `InitializePool` instead.
    /// This instruction was part of a workaround for Solana AccountInfo.data issues
    /// that have been resolved with improved implementation.
    #[deprecated(note = "Use InitializePool instead")]
    CreatePoolStateAccount {
        multiple_per_base: u64,
        pool_authority_bump_seed: u8,
        multiple_token_vault_bump_seed: u8,
        base_token_vault_bump_seed: u8,
    },
    
    /// **DEPRECATED**: Use `InitializePool` instead.
    /// This instruction was part of a workaround for Solana AccountInfo.data issues
    /// that have been resolved with improved implementation.
    #[deprecated(note = "Use InitializePool instead")]
    InitializePoolData {
        multiple_per_base: u64,
        pool_authority_bump_seed: u8,
        multiple_token_vault_bump_seed: u8,
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
    /// 
    /// # Benefits:
    /// - Atomic operation (all-or-nothing)
    /// - Simpler client integration
    /// - Better user experience
    /// - Eliminates workaround complexity
    /// 
    /// # Arguments:
    /// - `multiple_per_base`: Exchange ratio between multiple and base tokens (how many multiple tokens per 1 base token)
    /// - `pool_authority_bump_seed`: Bump seed for pool authority PDA derivation
    /// - `multiple_token_vault_bump_seed`: Bump seed for multiple token vault PDA
    /// - `base_token_vault_bump_seed`: Bump seed for base token vault PDA
    InitializePool {
        multiple_per_base: u64,
        pool_authority_bump_seed: u8,
        multiple_token_vault_bump_seed: u8,
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
    
    /// Updates security parameters for the pool (owner only)
    UpdateSecurityParams {
        /// Whether to pause pool operations
        is_paused: Option<bool>,
    },
    
    /// Change swap fee rate (owner only)
    ChangeFee {
        /// New fee in basis points (0-50 = 0%-0.5%)
        new_fee_basis_points: u64,
    },
    
    /// Withdraw accumulated fees from pool (owner only)
    WithdrawPoolFees {
        /// Token mint to withdraw
        token_mint: Pubkey,
        /// Amount to withdraw
        amount: u64,
    },
    
    /// Pause swap operations for specific pool (owner only)
    PausePoolSwaps,
    
    /// Unpause swap operations for specific pool (owner only)
    UnpausePoolSwaps,
    
    /// Get pool state PDA address for given tokens and ratio
    /// Useful for clients to derive addresses before calling other instructions
    GetPoolStatePDA {
        multiple_token_mint: Pubkey,
        base_token_mint: Pubkey,
        multiple_per_base: u64,
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
    
    /// Get current pool pause status (publicly readable)
    /// Returns swap pause status, deposit/withdrawal status, and pause details
    /// Distinguishes between system-wide pause and pool-specific swap pause
    GetPoolPauseStatus {
        // No parameters needed - reads from pool state account
    },
    
    /// Returns detailed liquidity information for both tokens
    /// Useful for calculating exchange rates and available liquidity
    GetLiquidityInfo {
        // No parameters needed - reads from pool state account  
    },
    
    /// **VIEW INSTRUCTION**: Get fee information including rates and collected amounts
    GetFeeInfo {
        // No fields needed - reads from pool state
    },
    
    /// **VIEW INSTRUCTION**: Get pool state PDA SOL balance
    GetPoolSolBalance {
        // No fields needed - reads from pool state account balance
    },
    
    /// Withdraws accumulated SOL fees from the pool state account (owner only)
    /// Maintains rent exemption for pool state account
    WithdrawFees,
    
    /// Pause the entire system - blocks all operations except unpause (system authority only)
    /// Takes precedence over all pool-specific pause states
    PauseSystem {
        /// Human-readable reason for the system pause
        reason: String,
    },
    
    /// Unpause the entire system - allows all operations to resume (system authority only)
    /// Clears the system pause state completely
    UnpauseSystem,
    
    /// Get the smart contract version information
    /// Returns version data including contract version and schema version
    /// No accounts required - returns constant version information
    GetVersion,
    
} 