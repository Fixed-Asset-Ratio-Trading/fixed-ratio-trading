//! Constants for the Solana Trading Pool Program
//! 
//! This module contains all the configuration constants, fee constants, 
//! system limits, and PDA seed prefixes used throughout the program.

/// Fee charged for pool registration/initialization
pub const REGISTRATION_FEE: u64 = 1_150_000_000; // 1.15 SOL

/// Fee charged for deposit and withdrawal operations  
pub const DEPOSIT_WITHDRAWAL_FEE: u64 = 1_300_000; // 0.0013 SOL

/// Fee charged for swap operations
pub const SWAP_FEE: u64 = 12_500; // 0.0000125 SOL

/// Maximum allowed swap fee in basis points (0.5% maximum)
pub const MAX_SWAP_FEE_BASIS_POINTS: u64 = 50; 

/// Denominator for basis point calculations (1 basis point = 0.01%)
pub const FEE_BASIS_POINTS_DENOMINATOR: u64 = 10000;

/// Maximum number of delegates allowed per pool
pub const MAX_DELEGATES: usize = 3;

/// Minimum wait time for withdrawal requests (5 minutes in seconds)
pub const MIN_WITHDRAWAL_WAIT_TIME: u64 = 300;

/// Maximum wait time for withdrawal requests (72 hours in seconds)
pub const MAX_WITHDRAWAL_WAIT_TIME: u64 = 259200;

/// PDA seed prefix for pool state accounts
pub const POOL_STATE_SEED_PREFIX: &[u8] = b"pool_state_v2";

/// PDA seed prefix for token A vault accounts
pub const TOKEN_A_VAULT_SEED_PREFIX: &[u8] = b"token_a_vault";

/// PDA seed prefix for token B vault accounts  
pub const TOKEN_B_VAULT_SEED_PREFIX: &[u8] = b"token_b_vault";

/// Additional buffer for rent calculations to account for potential rent increases
pub const MINIMUM_RENT_BUFFER: u64 = 1000; 