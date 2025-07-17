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

//-----------------------------------------------------------------------------
// SWAP CONTRACT FEES (Fixed SOL amounts)
//-----------------------------------------------------------------------------
// These are fixed SOL fees charged for computational costs and transaction processing.
// Contract fees cover the operational costs of running swap operations on-chain.

/// Swap contract fee charged for computational costs during token swaps.
/// 
/// This fee covers the computational cost of token swaps including ratio calculations,
/// token transfers, pool balance updates, and fee collection tracking.
/// 
/// **Type**: Swap Contract Fee (Fixed SOL amount)
/// **When Charged**: During `Swap` operations  
/// **Amount**: 0.00002715 SOL (27,150 lamports)
/// **Purpose**: Cover transaction processing costs for swap operations
/// **Goes To**: Pool state for operational cost coverage
/// **Cannot Be Changed**: This is a fixed operational cost
pub const SWAP_CONTRACT_FEE: u64 = 27_150; // 0.00002715 SOL

//=============================================================================
// CUSTOM FEE STRUCTURE APPROACH
//=============================================================================
// **ARCHITECTURAL DECISION**: Trading Fee System Removed
//
// This system no longer implements percentage-based trading fees at the protocol level.
// Instead, it provides granular swap access control through the SWAP_FOR_OWNERS_ONLY flag,
// enabling flexible custom fee structures through separate contracts.
//
// **Benefits of This Approach**:
// - Pool owners can implement any fee structure in separate contracts
// - Contract owners have granular control over swap permissions
// - Eliminates protocol-level fee complexity and potential bugs
// - Allows for sophisticated fee models (dynamic fees, tiered fees, etc.)
// - Maintains protocol simplicity while enabling maximum flexibility
//
// **Implementation Strategy**:
// - Use SWAP_FOR_OWNERS_ONLY flag to restrict swap access when needed
// - Custom fee collection handled by external contracts that interface with pools
// - Pool owners can route swaps through their own fee-collecting contracts
// - Contract owners can enable/disable owner-only mode for specific pools
//
// **Migration Path**:
// - Existing pools continue to operate normally (no trading fees)
// - Pool owners wanting custom fees deploy separate fee-collecting contracts
// - Those contracts can be granted special access via owner-only mode
// - This provides backward compatibility while enabling advanced fee structures
//=============================================================================

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

/// Validation context for liquidity operations (for test compatibility)
pub const VALIDATION_CONTEXT_LIQUIDITY: u8 = 3;

/// Validation context for swap operations (for test compatibility)
pub const VALIDATION_CONTEXT_SWAP: u8 = 4;

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

/// Pool state flag: Withdrawal protection active (future feature)
pub const POOL_FLAG_WITHDRAWAL_PROTECTION: u8 = 0b01000; // 8

/// Pool state flag: Single LP token mode (future feature)
pub const POOL_FLAG_SINGLE_LP_TOKEN: u8 = 0b10000; // 16

/// Pool state flag: Swap operations restricted to owners only
/// 
/// When this flag is set, only the pool owner and contract owner can perform swap operations.
/// This enables custom fee structures to be implemented through separate contracts while
/// maintaining granular control over swap access permissions.
/// 
/// **Purpose**: Enables custom fee collection through external contracts
/// **Control**: Only contract owner can enable/disable this flag (not pool owner)
/// **Use Case**: Pool owners deploy custom fee-collecting contracts and route swaps through them
pub const POOL_FLAG_SWAP_FOR_OWNERS_ONLY: u8 = 0b100000; // 32

//-----------------------------------------------------------------------------
// Testing Constants
//-----------------------------------------------------------------------------

/// Test environment program upgrade authority
/// 
/// This constant is used in test environments to validate program upgrade authority
/// when the program is not deployed with the BPF Loader Upgradeable. It should match
/// the keypair created by `create_test_program_authority_keypair()` in test setup.
/// 
/// **IMPORTANT**: This is only used in test environments and should be removed in production
pub const TEST_PROGRAM_UPGRADE_AUTHORITY: &str = "6SBHtCjRodUsFrsHEGjf4WH1v1kU2CMKHNQKFhTfYNQn";

