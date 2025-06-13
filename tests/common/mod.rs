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
LIABILITY, WHETHER IN AN ACTION OF CONTRACT IN TORT OR OTHERWISE, ARISING FROM,
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
//! - Logging control utilities

pub mod setup;
pub mod tokens;
pub mod pool_helpers;

// Re-export commonly used types and functions
pub use setup::*;
pub use tokens::*;  
pub use pool_helpers::*;

// Re-export external dependencies commonly used in tests
pub use borsh::{BorshDeserialize, BorshSerialize};
pub use solana_program::{
    pubkey::Pubkey,
    instruction::{AccountMeta, Instruction},
};
pub use solana_sdk::{
    program_pack::Pack,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
pub use spl_token::{
    instruction as token_instruction,
    state::{Account as TokenAccount, Mint as MintAccount},
};
pub use solana_program_test::*;

// Re-export program-specific imports
pub use fixed_ratio_trading::{
    PoolInstruction, PoolState, process_instruction, 
    ID as PROGRAM_ID,
    POOL_STATE_SEED_PREFIX, TOKEN_A_VAULT_SEED_PREFIX, TOKEN_B_VAULT_SEED_PREFIX
};

/// Default test logging configuration
/// 
/// Sets up minimal logging by default unless overridden by RUST_LOG environment variable
pub fn init_test_logging() {
    use std::env;
    
    // Only initialize if not already set
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "error");
    }
    
    // Initialize env_logger, ignoring errors if already initialized
    let _ = env_logger::try_init();
}

/// Enhanced test logging for debugging
/// 
/// Use this in specific tests that need detailed logging output
pub fn init_debug_logging() {
    use std::env;
    env::set_var("RUST_LOG", "debug");
    let _ = env_logger::try_init();
}

/// Test result type alias for convenience
pub type TestResult = Result<(), BanksClientError>;

/// Common test constants
pub mod constants {
    /// Default ratio for test pools (2:1)
    pub const DEFAULT_RATIO: u64 = 2;
    
    /// Test token decimal places
    pub const TOKEN_DECIMALS: u8 = 9;
    
    /// Default liquidity amounts for testing
    pub const DEFAULT_LIQUIDITY_AMOUNT: u64 = 10_000_000; // 10M tokens
    
    /// Default user token amounts for testing
    pub const DEFAULT_USER_TOKEN_AMOUNT: u64 = 25_000_000; // 25M tokens
    
    /// Default SOL airdrop amount for test users
    pub const DEFAULT_SOL_AIRDROP: u64 = 5_000_000_000; // 5 SOL
} 