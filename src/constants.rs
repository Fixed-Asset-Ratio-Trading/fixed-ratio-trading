//! Constants for the Solana Trading Pool Program
//! 
//! This module contains all the configuration constants, fee constants, 
//! system limits, and PDA seed prefixes used throughout the program.

//=============================================================================
// FEE STRUCTURE DOCUMENTATION
//=============================================================================
//
// The Fixed Ratio Trading system implements TWO distinct types of fees:
//
// 1. **CONTRACT FEES** (Fixed SOL amounts):
//    - Paid in Solana (SOL) to cover transaction processing costs
//    - Fixed amounts that do not vary based on trade size
//    - Collected by the pool for operational expenses
//
// 2. **POOL FEES** (Percentage-based on traded assets):
//    - Paid as a percentage of the asset being traded
//    - Variable amounts based on trade size and pool configuration
//    - Default: 0% (can be increased up to 0.5% maximum)
//    - Revenue for pool operators and liquidity providers
//
//=============================================================================

//-----------------------------------------------------------------------------
// CONTRACT FEES (Fixed SOL Amounts)
//-----------------------------------------------------------------------------
// These fees are charged in Solana (SOL) for contract operations and are 
// independent of the tokens being traded or their values.

/// Fee charged for pool registration/initialization in lamports (1.15 SOL)
/// 
/// This one-time fee covers the computational cost of creating a new trading pool,
/// including account creation, PDA derivation, and initial state setup.
/// 
/// **Type**: Contract Fee (Fixed SOL amount)
/// **When Charged**: During pool creation via `InitializePool`
/// **Amount**: 1.15 SOL (1,150,000,000 lamports)
/// **Purpose**: Cover pool creation transaction costs and prevent spam pool creation
pub const REGISTRATION_FEE: u64 = 1_150_000_000; // 1.15 SOL

/// Fee charged for deposit and withdrawal operations in lamports (0.0013 SOL)
/// 
/// This fee covers the computational cost of liquidity operations including
/// token transfers, LP token minting/burning, and pool state updates.
/// 
/// **Type**: Contract Fee (Fixed SOL amount)  
/// **When Charged**: During `Deposit` and `Withdraw` operations
/// **Amount**: 0.0013 SOL (1,300,000 lamports)
/// **Purpose**: Cover transaction processing costs for liquidity operations
pub const DEPOSIT_WITHDRAWAL_FEE: u64 = 1_300_000; // 0.0013 SOL

/// Fee charged for swap operations in lamports (0.00002715 SOL)
/// 
/// This fee covers the computational cost of token swaps including ratio calculations,
/// token transfers, pool balance updates, and fee collection tracking.
/// 
/// **Type**: Contract Fee (Fixed SOL amount)
/// **When Charged**: During `Swap` operations  
/// **Amount**: 0.00002715 SOL (27,150 lamports)
/// **Purpose**: Cover transaction processing costs for swap operations
pub const SWAP_FEE: u64 = 27_150; // 0.00002715 SOL

/// **HFT DISCOUNTED** fee charged for HFT optimized swap operations in lamports (0.0000163 SOL)
/// 
/// This discounted fee provides a 40% reduction for HFT optimized swaps to incentivize
/// the use of compute-efficient swap functions and reward high-frequency traders.
/// 
/// **Type**: Contract Fee (Fixed SOL amount, 40% discount)
/// **When Charged**: During `SwapHftOptimized` operations
/// **Amount**: 0.0000163 SOL (16,290 lamports) - 40% discount from standard SWAP_FEE
/// **Purpose**: Incentivize HFT optimized swaps and reward compute efficiency
/// **Calculation**: SWAP_FEE * 0.6 = 27,150 * 0.6 = 16,290 lamports
pub const HFT_SWAP_FEE: u64 = 16_290; // 0.0000163 SOL (40% discount)

//-----------------------------------------------------------------------------
// POOL FEES (Percentage-based on traded assets)
//-----------------------------------------------------------------------------
// These fees are charged as a percentage of the tokens being traded and can
// be configured by the pool owner to generate revenue.

/// Maximum allowed swap fee in basis points (0.5% maximum)
/// 
/// This represents the maximum percentage fee that can be charged on the input
/// token amount during swap operations. Pool owners can set any fee rate from
/// 0% (no fees) up to this maximum.
/// 
/// **Type**: Pool Fee (Percentage-based)
/// **Applied To**: Input token amount during swaps
/// **Range**: 0 to 50 basis points (0% to 0.5%)
/// **Examples**:
/// - 0 basis points = 0% fee (default, no trading fees)
/// - 10 basis points = 0.1% fee  
/// - 25 basis points = 0.25% fee
/// - 50 basis points = 0.5% fee (maximum allowed)
/// 
/// **Calculation**: `fee_amount = input_amount * fee_basis_points / 10000`
/// **Revenue**: Collected by pool and withdrawable by pool owner
pub const MAX_SWAP_FEE_BASIS_POINTS: u64 = 50; 

/// Denominator for basis points calculations (1 basis point = 1/10000 = 0.01%)
/// 
/// This constant is used to convert basis points to decimal percentages.
/// **Usage**: `percentage = basis_points / FEE_BASIS_POINTS_DENOMINATOR`
/// **Examples**:
/// - 25 basis points / 10000 = 0.0025 = 0.25%
/// - 50 basis points / 10000 = 0.0050 = 0.50%
pub const FEE_BASIS_POINTS_DENOMINATOR: u64 = 10000;

//=============================================================================
// RENT AND BUFFER REQUIREMENTS
//=============================================================================

/// Minimum rent buffer to maintain above Solana's rent-exempt threshold
/// 
/// This buffer ensures accounts remain rent-exempt even if rent rates change
/// slightly between account creation and operations.
/// 
/// **Amount**: 1000 lamports (conservative buffer)
/// **Purpose**: Prevent accidental account closure due to rent calculation variations
pub const MINIMUM_RENT_BUFFER: u64 = 1000;

//=============================================================================
// TREASURY TYPE CODES
//=============================================================================
// These codes identify different treasury types for validation purposes.

/// Treasury type code for main treasury (all fees)
pub const TREASURY_TYPE_MAIN: u8 = 1;

//=============================================================================
// VALIDATION CONTEXT CODES
//=============================================================================
// These codes provide context for validation operations and error messages.

/// Validation context for general fee operations
pub const VALIDATION_CONTEXT_FEE: u8 = 1;

/// Validation context for pool creation operations
pub const VALIDATION_CONTEXT_POOL_CREATION: u8 = 2;

//=============================================================================
// PDA SEED PREFIXES
//=============================================================================
// These byte string prefixes are used for Program Derived Address (PDA) generation.
// Each type of account has a unique prefix to prevent address collisions.

pub const POOL_STATE_SEED_PREFIX: &[u8] = b"pool_state";

pub const TOKEN_A_VAULT_SEED_PREFIX: &[u8] = b"token_a_vault";

pub const TOKEN_B_VAULT_SEED_PREFIX: &[u8] = b"token_b_vault";

pub const SYSTEM_STATE_SEED_PREFIX: &[u8] = b"system_state";

/// Main treasury seed prefix for the centralized treasury PDA
pub const MAIN_TREASURY_SEED_PREFIX: &[u8] = b"main_treasury";

pub const LP_TOKEN_A_MINT_SEED_PREFIX: &[u8] = b"lp_token_a_mint";

pub const LP_TOKEN_B_MINT_SEED_PREFIX: &[u8] = b"lp_token_b_mint";



//=============================================================================
// RENT AND ACCOUNT CONFIGURATION  
//=============================================================================

//=============================================================================
// CONSOLIDATION CONFIGURATION
//=============================================================================

/// Maximum number of pools that can be consolidated in a single batch
/// This limit ensures the transaction stays within the 200K CU limit
pub const MAX_POOLS_PER_CONSOLIDATION_BATCH: u8 = 20;

/// Pause reason code for consolidation operations
/// This code indicates the system was paused specifically for fee consolidation
pub const PAUSE_REASON_CONSOLIDATION: u8 = 15;

//=============================================================================
// FIXED SYSTEM VALUES (MOVED FROM POOLSTATE)
//=============================================================================

/// Fixed swap fee basis points across all pools (0.25% = 25 basis points)
/// Since this is a fixed value, no need to store per pool
pub const FIXED_SWAP_FEE_BASIS_POINTS: u64 = 25;

//=============================================================================
// POOL PAUSE BITWISE FLAGS
//=============================================================================

/// Pause liquidity operations (deposits and withdrawals only)
/// Sets POOL_FLAG_LIQUIDITY_PAUSED in pool_state.flags
pub const PAUSE_FLAG_LIQUIDITY: u8 = 0b01; // 1

/// Pause swap operations only
/// Sets POOL_FLAG_SWAPS_PAUSED in pool_state.flags
pub const PAUSE_FLAG_SWAPS: u8 = 0b10; // 2

/// Pause all operations (liquidity + swaps)
/// Required combination for consolidation eligibility
pub const PAUSE_FLAG_ALL: u8 = PAUSE_FLAG_LIQUIDITY | PAUSE_FLAG_SWAPS; // 3

//=============================================================================
// POOL STATE BITWISE FLAGS
//=============================================================================

/// Pool state flag: One-to-many ratio configuration
pub const POOL_FLAG_ONE_TO_MANY_RATIO: u8 = 0b00001; // 1

/// Pool state flag: Liquidity operations paused (deposits/withdrawals only)
pub const POOL_FLAG_LIQUIDITY_PAUSED: u8 = 0b00010; // 2

/// Pool state flag: Swap operations paused
pub const POOL_FLAG_SWAPS_PAUSED: u8 = 0b00100; // 4

/// Pool state flag: Withdrawal protection active
pub const POOL_FLAG_WITHDRAWAL_PROTECTION: u8 = 0b01000; // 8

/// Pool state flag: Single LP token mode (future feature)
pub const POOL_FLAG_SINGLE_LP_TOKEN: u8 = 0b10000; // 16

