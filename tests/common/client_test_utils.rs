//! Client Test Utilities
//!
//! This module contains test utility functions for client-side testing, moved from main contract code.

use fixed_ratio_trading::client_sdk::{PoolConfig};
use solana_program::pubkey::Pubkey;

/// Creates a test pool configuration for testing purposes.
/// 
/// # Returns
/// * `PoolConfig` - A test configuration with random mints and 1000:1 ratio
#[allow(dead_code)]
pub fn create_test_pool_config() -> PoolConfig {
    PoolConfig {
        multiple_token_mint: Pubkey::new_unique(),
        base_token_mint: Pubkey::new_unique(),
        ratio_a_numerator: 1000,
        ratio_b_denominator: 1,
    }
} 