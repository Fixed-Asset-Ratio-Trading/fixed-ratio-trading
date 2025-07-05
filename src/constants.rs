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

/// Denominator for basis point calculations (1 basis point = 0.01%)
/// 
/// Used to convert basis points to percentages in fee calculations.
/// Formula: `percentage = basis_points / FEE_BASIS_POINTS_DENOMINATOR`
/// 
/// **Examples**:
/// - 50 basis points / 10000 = 0.005 = 0.5%
/// - 25 basis points / 10000 = 0.0025 = 0.25%  
/// - 10 basis points / 10000 = 0.001 = 0.1%
pub const FEE_BASIS_POINTS_DENOMINATOR: u64 = 10000;

//=============================================================================
// FEE EXAMPLES AND SCENARIOS
//=============================================================================
//
// **Example 1: Pool Creation**
// - User creates a new USDC/SOL pool
// - Contract Fee: 1.15 SOL (paid to pool for operational costs)
// - Pool Fee: Not applicable (no trading yet)
//
// **Example 2: Adding Liquidity** 
// - User deposits 1000 USDC into pool
// - Contract Fee: 0.0013 SOL (paid to pool)  
// - Pool Fee: Not applicable (no fee on liquidity operations)
//
// **Example 3: Token Swap (0% pool fee)**
// - User swaps 1000 USDC for SOL
// - Contract Fee: 0.00002715 SOL (paid to pool)
// - Pool Fee: 0 USDC (pool fee set to 0%)
// - User receives: Full SOL amount based on 1000 USDC input
//
// **Example 4: Token Swap (0.25% pool fee)**  
// - User swaps 1000 USDC for SOL
// - Contract Fee: 0.00002715 SOL (paid to pool)
// - Pool Fee: 2.5 USDC (1000 * 0.0025 = 2.5 USDC)  
// - Effective Input: 997.5 USDC (1000 - 2.5 fee)
// - User receives: SOL amount based on 997.5 USDC
// - Pool retains: 2.5 USDC as revenue for pool owner
//
//=============================================================================

/// PDA seed prefix for pool state accounts
pub const POOL_STATE_SEED_PREFIX: &[u8] = b"pool_state";

/// PDA seed prefix for token A vault accounts
pub const TOKEN_A_VAULT_SEED_PREFIX: &[u8] = b"token_a_vault";

/// PDA seed prefix for token B vault accounts  
pub const TOKEN_B_VAULT_SEED_PREFIX: &[u8] = b"token_b_vault";

/// PDA seed for system state account
pub const SYSTEM_STATE_SEED_PREFIX: &[u8] = b"system_state";

/// PDA seed for main treasury account
pub const MAIN_TREASURY_SEED_PREFIX: &[u8] = b"main_treasury";

/// PDA seed prefix for specialized treasury that collects regular swap fees
pub const SWAP_TREASURY_SEED_PREFIX: &[u8] = b"swap_treasury";

/// PDA seed prefix for specialized treasury that collects HFT swap fees
pub const HFT_TREASURY_SEED_PREFIX: &[u8] = b"hft_treasury";

/// Legacy treasury seed prefix (kept for backward compatibility)
/// This points to the main treasury for any existing references
pub const TREASURY_SEED_PREFIX: &[u8] = MAIN_TREASURY_SEED_PREFIX;

/// Additional buffer for rent calculations to account for potential rent increases
pub const MINIMUM_RENT_BUFFER: u64 = 1000; 