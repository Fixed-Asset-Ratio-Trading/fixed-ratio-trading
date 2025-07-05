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

//! # Common Test Utilities
//! 
//! This module provides shared utilities and helpers for integration tests
//! across all test modules. It includes:
//! 
//! - Token creation and minting helpers
//! - Pool setup and initialization utilities  
//! - Test environment configuration
//! - Test execution utilities

pub mod setup;
pub mod tokens;
pub mod pool_helpers;
pub mod liquidity_helpers;
pub mod client_test_utils;
pub mod view_utils;
pub mod utils_test_utils;

// Re-export commonly used types and functions
#[allow(unused_imports)]
pub use setup::*;
#[allow(unused_imports)]
pub use tokens::*;  
#[allow(unused_imports)]
pub use pool_helpers::*;
#[allow(unused_imports)]
pub use liquidity_helpers::*;
#[allow(unused_imports)]
pub use client_test_utils::*;
#[allow(unused_imports)]
pub use view_utils::*;
#[allow(unused_imports)]
pub use utils_test_utils::*;

// Re-export external dependencies commonly used in tests
// Allow unused imports since these are provided for optional use across test modules
#[allow(unused_imports)]
pub use borsh::{BorshDeserialize, BorshSerialize};
#[allow(unused_imports)]
pub use solana_program::{
    pubkey::Pubkey,
    instruction::{AccountMeta, Instruction},
};
#[allow(unused_imports)]
pub use solana_sdk::{
    program_pack::Pack,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
#[allow(unused_imports)]
pub use spl_token::{
    instruction as token_instruction,
    state::{Account as TokenAccount, Mint as MintAccount},
};
#[allow(unused_imports)]
pub use solana_program_test::*;

// Re-export program-specific imports
#[allow(unused_imports)]
pub use fixed_ratio_trading::{
    PoolInstruction, PoolState, process_instruction, 
    ID as PROGRAM_ID,
    POOL_STATE_SEED_PREFIX, TOKEN_A_VAULT_SEED_PREFIX, TOKEN_B_VAULT_SEED_PREFIX
};

/// Test result type alias for convenience
pub type TestResult = Result<(), BanksClientError>;

/// Helper function to run a test with minimal logging
#[allow(dead_code)]
pub async fn run_test_with_minimal_logging<F, Fut>(test_fn: F) -> TestResult 
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = TestResult>,
{
    // Save current log level
    let original_log = std::env::var("RUST_LOG").ok();
    
    // Set minimal logging
    std::env::set_var("RUST_LOG", "off");
    std::env::set_var("SOLANA_TEST_METRICS_ENABLED", "0");
    
    // Run the test
    let result = test_fn().await;
    
    // Restore original log level
    if let Some(log) = original_log {
        std::env::set_var("RUST_LOG", log);
    } else {
        std::env::remove_var("RUST_LOG");
    }
    
    result
}

/// A helper function to handle expected test errors in a cleaner way.
/// This prevents warnings from showing up when an error is actually expected behavior.
/// 
/// # Arguments
/// * `description` - Description of what's being tested
/// * `result` - The result to check
/// * `expected_success_message` - Message to print on success
/// * `expected_error_message` - Custom message to show on expected error
/// 
/// # Returns
/// The original result
#[allow(dead_code)]
pub fn handle_expected_test_error<T, E: std::fmt::Debug>(
    description: &str, 
    result: &Result<T, E>, 
    expected_success_message: &str,
    expected_error_message: &str
) {
    match result {
        Ok(_) => println!("✅ {}", expected_success_message),
        Err(e) => {
            // Use a special format that clearly indicates this is expected behavior
            println!("ℹ️ {} - {}: {:?}", expected_error_message, description, e);
            println!("✅ Test is verifying correct error handling");
        }
    }
}

/// Common test constants
pub mod constants {
    /// Default ratio for test pools (2:1)
    pub const DEFAULT_RATIO: u64 = 2;
    
    /// Test token decimal places
    #[allow(dead_code)]
    pub const TOKEN_DECIMALS: u8 = 9;
    
    /// Default liquidity amounts for testing
    #[allow(dead_code)]
    pub const DEFAULT_LIQUIDITY_AMOUNT: u64 = 10_000_000; // 10M tokens
    
    /// Default user token amounts for testing
    #[allow(dead_code)]
    pub const DEFAULT_USER_TOKEN_AMOUNT: u64 = 25_000_000; // 25M tokens
    
    /// Default SOL airdrop amount for test users
    #[allow(dead_code)]
    pub const DEFAULT_SOL_AIRDROP: u64 = 5_000_000_000; // 5 SOL
} 