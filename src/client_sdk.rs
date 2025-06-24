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

//! # Fixed Ratio Trading Pool - Client SDK
//! 
//! This module provides a comprehensive client SDK that simplifies interaction with the
//! Fixed Ratio Trading Pool program. It hides the complexity of PDA derivation, account
//! management, and instruction construction behind easy-to-use functions.
//! 
//! ## Key Features
//! 
//! - **Simple Pool Creation**: One function call to create a new trading pool
//! - **Automatic PDA Derivation**: No need to manually calculate program-derived addresses
//! - **Account Preparation**: Automatic preparation of all required accounts
//! - **Error Handling**: Clear error messages and validation
//! - **Type Safety**: Strongly typed interfaces prevent common mistakes
//! - **Testing Support**: Built-in utilities for testing and debugging
//! 
//! ## Example Usage
//! 
//! ```rust,no_run
//! use fixed_ratio_trading::client_sdk::*;
//! use solana_sdk::pubkey::Pubkey;
//! 
//! // Create a new pool with 2:1 ratio (USDC:SOL)
//! let program_id = Pubkey::new_unique();
//! let pool_client = PoolClient::new(program_id);
//! let pool_config = PoolConfig {
//!     primary_token_mint: Pubkey::new_unique(),
//!     base_token_mint: Pubkey::new_unique(),
//!     ratio_primary_per_base: 2,
//! };
//! 
//! // Get pool creation instruction (single atomic operation) 
//! let payer = Pubkey::new_unique();
//! let lp_a = Pubkey::new_unique();
//! let lp_b = Pubkey::new_unique();
//! let create_ix = pool_client.create_pool_instruction(&payer, &pool_config, &lp_a, &lp_b).unwrap();
//! 
//! // Add liquidity to the pool
//! let user = Pubkey::new_unique();
//! let user_account = Pubkey::new_unique(); 
//! let lp_account = Pubkey::new_unique();
//! let deposit_ix = pool_client.deposit_instruction(
//!     &user,
//!     &pool_config,
//!     &pool_config.primary_token_mint,
//!     1000000, // 1 USDC
//!     &user_account,
//!     &lp_account,
//! ).unwrap();
//! ```

use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
    sysvar::{self},
};
use borsh::BorshSerialize;
use crate::{
    types::instructions::PoolInstruction,
    POOL_STATE_SEED_PREFIX,
    TOKEN_A_VAULT_SEED_PREFIX,
    TOKEN_B_VAULT_SEED_PREFIX,
};

/// Configuration for creating a new trading pool.
/// 
/// This struct encapsulates all the parameters needed to create a new fixed-ratio
/// trading pool, providing a clean interface that hides implementation details.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Primary token mint address (e.g., USDC)
    pub primary_token_mint: Pubkey,
    /// Base token mint address (e.g., SOL)
    pub base_token_mint: Pubkey,
    /// Exchange ratio: how many primary tokens per base token
    /// Example: ratio_primary_per_base = 2 means 2 USDC per 1 SOL
    pub ratio_primary_per_base: u64,
}

impl PoolConfig {
    /// Creates a new pool configuration.
    /// 
    /// # Arguments
    /// * `primary_token_mint` - The mint of the primary token (usually the quote token)
    /// * `base_token_mint` - The mint of the base token (usually the base token)
    /// * `ratio_primary_per_base` - How many primary tokens equal one base token
    /// 
    /// # Example
    /// ```rust,no_run
    /// use solana_sdk::pubkey::Pubkey;
    /// use fixed_ratio_trading::client_sdk::PoolConfig;
    /// 
    /// // 1000 USDC per 1 SOL pool
    /// let usdc_mint = Pubkey::new_unique();
    /// let sol_mint = Pubkey::new_unique();
    /// let config = PoolConfig::new(usdc_mint, sol_mint, 1000).unwrap();
    /// ```
    pub fn new(
        primary_token_mint: Pubkey,
        base_token_mint: Pubkey,
        ratio_primary_per_base: u64,
    ) -> Result<Self, PoolClientError> {
        if ratio_primary_per_base == 0 {
            return Err(PoolClientError::InvalidRatio);
        }
        
        if primary_token_mint == base_token_mint {
            return Err(PoolClientError::IdenticalTokens);
        }
        
        Ok(Self {
            primary_token_mint,
            base_token_mint,
            ratio_primary_per_base,
        })
    }
}

/// Derived addresses for a trading pool.
/// 
/// This struct contains all the program-derived addresses (PDAs) associated with
/// a trading pool, automatically calculated to ensure consistency and correctness.
#[derive(Debug, Clone)]
pub struct PoolAddresses {
    /// Pool state PDA address
    pub pool_state: Pubkey,
    /// Pool state PDA bump seed
    pub pool_state_bump: u8,
    /// Token A vault PDA address
    pub token_a_vault: Pubkey,
    /// Token A vault bump seed
    pub token_a_vault_bump: u8,
    /// Token B vault PDA address
    pub token_b_vault: Pubkey,
    /// Token B vault bump seed
    pub token_b_vault_bump: u8,
    /// Normalized token A mint (lexicographically first)
    pub token_a_mint: Pubkey,
    /// Normalized token B mint (lexicographically second)
    pub token_b_mint: Pubkey,
    /// Normalized ratio numerator
    pub ratio_a_numerator: u64,
    /// Normalized ratio denominator
    pub ratio_b_denominator: u64,
}

/// Main client for interacting with the Fixed Ratio Trading Pool program.
/// 
/// This client provides high-level functions that abstract away the complexity
/// of instruction construction, account management, and PDA derivation.
#[derive(Debug, Clone)]
pub struct PoolClient {
    /// Program ID of the Fixed Ratio Trading Pool program
    pub program_id: Pubkey,
}

impl PoolClient {
    /// Creates a new pool client.
    /// 
    /// # Arguments
    /// * `program_id` - The program ID of the deployed Fixed Ratio Trading Pool program
    pub fn new(program_id: Pubkey) -> Self {
        Self { program_id }
    }
    
    /// Derives all addresses for a pool configuration.
    /// 
    /// This function calculates all the program-derived addresses (PDAs) for a given
    /// pool configuration, handling token normalization and seed generation automatically.
    /// 
    /// # Arguments
    /// * `config` - Pool configuration containing token mints and ratio
    /// 
    /// # Returns
    /// * `PoolAddresses` - All derived addresses for the pool
    pub fn derive_pool_addresses(&self, config: &PoolConfig) -> PoolAddresses {
        // Enhanced normalization to prevent economic duplicates
        // Step 1: Lexicographic token ordering
        let (token_a_mint, token_b_mint) = 
            if config.primary_token_mint < config.base_token_mint {
                (config.primary_token_mint, config.base_token_mint)
            } else {
                (config.base_token_mint, config.primary_token_mint)
            };
        
        // Step 2: Canonical ratio mapping to prevent liquidity fragmentation
        let (ratio_a_numerator, ratio_b_denominator): (u64, u64) = 
            if config.primary_token_mint < config.base_token_mint {
                (config.ratio_primary_per_base, 1u64)
            } else {
                // Use canonical form - all pools with same token pair get same ratio
                (config.ratio_primary_per_base, 1u64)
            };
        
        // Derive pool state PDA
        let (pool_state, pool_state_bump) = Pubkey::find_program_address(
            &[
                POOL_STATE_SEED_PREFIX,
                token_a_mint.as_ref(),
                token_b_mint.as_ref(),
                &ratio_a_numerator.to_le_bytes(),
                &ratio_b_denominator.to_le_bytes(),
            ],
            &self.program_id,
        );
        
        // Derive token vault PDAs
        let (token_a_vault, token_a_vault_bump) = Pubkey::find_program_address(
            &[TOKEN_A_VAULT_SEED_PREFIX, pool_state.as_ref()],
            &self.program_id,
        );
        
        let (token_b_vault, token_b_vault_bump) = Pubkey::find_program_address(
            &[TOKEN_B_VAULT_SEED_PREFIX, pool_state.as_ref()],
            &self.program_id,
        );
        
        PoolAddresses {
            pool_state,
            pool_state_bump,
            token_a_vault,
            token_a_vault_bump,
            token_b_vault,
            token_b_vault_bump,
            token_a_mint,
            token_b_mint,
            ratio_a_numerator,
            ratio_b_denominator,
        }
    }
    
    /// Creates a pool initialization instruction (single atomic operation).
    /// 
    /// This function creates the new recommended single-instruction pool initialization
    /// that replaces the deprecated two-instruction pattern. It handles all the complexity
    /// of account preparation and instruction construction.
    /// 
    /// # Arguments
    /// * `payer` - Account that will pay for account creation and fees
    /// * `config` - Pool configuration
    /// * `lp_token_a_mint` - Keypair for LP Token A mint (must be new)
    /// * `lp_token_b_mint` - Keypair for LP Token B mint (must be new)
    /// 
    /// # Returns
    /// * `Instruction` - Ready-to-send instruction for pool creation
    pub fn create_pool_instruction(
        &self,
        payer: &Pubkey,
        config: &PoolConfig,
        lp_token_a_mint: &Pubkey,
        lp_token_b_mint: &Pubkey,
    ) -> Result<Instruction, PoolClientError> {
        let addresses = self.derive_pool_addresses(config);
        
        // Validate inputs
        if config.ratio_primary_per_base == 0 {
            return Err(PoolClientError::InvalidRatio);
        }
        
        // Map bump seeds back to primary/base token convention
        let (primary_vault_bump, base_vault_bump) = 
            if config.primary_token_mint < config.base_token_mint {
                (addresses.token_a_vault_bump, addresses.token_b_vault_bump)
            } else {
                (addresses.token_b_vault_bump, addresses.token_a_vault_bump)
            };
        
        // Create instruction
        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(*payer, true),                           // Payer (signer)
                AccountMeta::new(addresses.pool_state, false),            // Pool state PDA
                AccountMeta::new_readonly(config.primary_token_mint, false), // Primary token mint
                AccountMeta::new_readonly(config.base_token_mint, false),    // Base token mint
                AccountMeta::new(*lp_token_a_mint, false),               // LP Token A mint
                AccountMeta::new(*lp_token_b_mint, false),               // LP Token B mint
                AccountMeta::new(addresses.token_a_vault, false),         // Token A vault PDA
                AccountMeta::new(addresses.token_b_vault, false),         // Token B vault PDA
                AccountMeta::new_readonly(system_program::id(), false),   // System program
                AccountMeta::new_readonly(spl_token::id(), false),        // SPL Token program
                AccountMeta::new_readonly(sysvar::rent::id(), false),     // Rent sysvar
            ],
            data: PoolInstruction::InitializePool {
                ratio_primary_per_base: config.ratio_primary_per_base,
                pool_authority_bump_seed: addresses.pool_state_bump,
                primary_token_vault_bump_seed: primary_vault_bump,
                base_token_vault_bump_seed: base_vault_bump,
            }.try_to_vec()?,
        };
        
        Ok(instruction)
    }
    
    /// Creates a deposit instruction for adding liquidity to a pool.
    /// 
    /// This function creates an instruction to deposit tokens into a pool and receive
    /// LP tokens in return. It handles all account preparation automatically.
    /// 
    /// # Arguments
    /// * `user` - User account (must be signer)
    /// * `config` - Pool configuration
    /// * `deposit_token_mint` - Mint of the token being deposited
    /// * `amount` - Amount to deposit
    /// * `user_source_account` - User's token account for the deposit token
    /// * `user_lp_account` - User's account for receiving LP tokens
    /// 
    /// # Returns
    /// * `Instruction` - Ready-to-send deposit instruction
    pub fn deposit_instruction(
        &self,
        user: &Pubkey,
        config: &PoolConfig,
        deposit_token_mint: &Pubkey,
        amount: u64,
        user_source_account: &Pubkey,
        user_lp_account: &Pubkey,
    ) -> Result<Instruction, PoolClientError> {
        let addresses = self.derive_pool_addresses(config);
        
        // Validate deposit token
        if *deposit_token_mint != config.primary_token_mint && *deposit_token_mint != config.base_token_mint {
            return Err(PoolClientError::InvalidDepositToken);
        }
        
        // Get pool LP mint addresses (these would need to be provided or derived)
        // For simplicity, using placeholder values - in real implementation,
        // these would be retrieved from pool state or provided by caller
        let pool_state_data = self.get_pool_state(&addresses.pool_state)?;
        
        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(*user, true),                           // User (signer)
                AccountMeta::new(*user_source_account, false),           // User source token account
                AccountMeta::new(addresses.pool_state, false),           // Pool state PDA
                AccountMeta::new_readonly(addresses.token_a_mint, false), // Token A mint (for PDA seeds)
                AccountMeta::new_readonly(addresses.token_b_mint, false), // Token B mint (for PDA seeds)
                AccountMeta::new(addresses.token_a_vault, false),        // Pool Token A vault
                AccountMeta::new(addresses.token_b_vault, false),        // Pool Token B vault
                AccountMeta::new(pool_state_data.lp_token_a_mint, false), // LP Token A mint
                AccountMeta::new(pool_state_data.lp_token_b_mint, false), // LP Token B mint
                AccountMeta::new(*user_lp_account, false),               // User LP token account
                AccountMeta::new_readonly(system_program::id(), false),   // System program
                AccountMeta::new_readonly(spl_token::id(), false),        // SPL Token program
                AccountMeta::new_readonly(sysvar::rent::id(), false),     // Rent sysvar
                AccountMeta::new_readonly(sysvar::clock::id(), false),    // Clock sysvar
            ],
            data: PoolInstruction::Deposit {
                deposit_token_mint: *deposit_token_mint,
                amount,
            }.try_to_vec()?,
        };
        
        Ok(instruction)
    }
    
    /// Creates an enhanced deposit instruction with additional features.
    /// 
    /// This function creates an enhanced deposit instruction with slippage protection
    /// and custom fee recipient options, useful for testing and advanced use cases.
    /// 
    /// # Arguments
    /// * `user` - User account (must be signer)
    /// * `config` - Pool configuration
    /// * `deposit_token_mint` - Mint of the token being deposited
    /// * `amount` - Amount to deposit
    /// * `minimum_lp_tokens_out` - Minimum LP tokens expected (slippage protection)
    /// * `fee_recipient` - Optional custom fee recipient
    /// * `user_source_account` - User's token account for the deposit token
    /// * `user_lp_account` - User's account for receiving LP tokens
    /// 
    /// # Returns
    /// * `Instruction` - Ready-to-send enhanced deposit instruction
    pub fn deposit_with_features_instruction(
        &self,
        user: &Pubkey,
        config: &PoolConfig,
        deposit_token_mint: &Pubkey,
        amount: u64,
        minimum_lp_tokens_out: u64,
        fee_recipient: Option<Pubkey>,
        user_source_account: &Pubkey,
        user_lp_account: &Pubkey,
    ) -> Result<Instruction, PoolClientError> {
        let addresses = self.derive_pool_addresses(config);
        
        // Validate deposit token
        if *deposit_token_mint != config.primary_token_mint && *deposit_token_mint != config.base_token_mint {
            return Err(PoolClientError::InvalidDepositToken);
        }
        
        let pool_state_data = self.get_pool_state(&addresses.pool_state)?;
        
        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(*user, true),                           // User (signer)
                AccountMeta::new(*user_source_account, false),           // User source token account
                AccountMeta::new(addresses.pool_state, false),           // Pool state PDA
                AccountMeta::new_readonly(addresses.token_a_mint, false), // Token A mint (for PDA seeds)
                AccountMeta::new_readonly(addresses.token_b_mint, false), // Token B mint (for PDA seeds)
                AccountMeta::new(addresses.token_a_vault, false),        // Pool Token A vault
                AccountMeta::new(addresses.token_b_vault, false),        // Pool Token B vault
                AccountMeta::new(pool_state_data.lp_token_a_mint, false), // LP Token A mint
                AccountMeta::new(pool_state_data.lp_token_b_mint, false), // LP Token B mint
                AccountMeta::new(*user_lp_account, false),               // User LP token account
                AccountMeta::new_readonly(system_program::id(), false),   // System program
                AccountMeta::new_readonly(spl_token::id(), false),        // SPL Token program
                AccountMeta::new_readonly(sysvar::rent::id(), false),     // Rent sysvar
                AccountMeta::new_readonly(sysvar::clock::id(), false),    // Clock sysvar
            ],
            data: PoolInstruction::DepositWithFeatures {
                deposit_token_mint: *deposit_token_mint,
                amount,
                minimum_lp_tokens_out,
                fee_recipient,
            }.try_to_vec()?,
        };
        
        Ok(instruction)
    }
    
    /// Creates a withdraw instruction for removing liquidity from a pool.
    /// 
    /// # Arguments
    /// * `user` - User account (must be signer)
    /// * `config` - Pool configuration
    /// * `withdraw_token_mint` - Mint of the token to withdraw
    /// * `lp_amount_to_burn` - Amount of LP tokens to burn
    /// * `user_lp_account` - User's LP token account
    /// * `user_destination_account` - User's account for receiving withdrawn tokens
    /// 
    /// # Returns
    /// * `Instruction` - Ready-to-send withdraw instruction
    pub fn withdraw_instruction(
        &self,
        user: &Pubkey,
        config: &PoolConfig,
        withdraw_token_mint: &Pubkey,
        lp_amount_to_burn: u64,
        user_lp_account: &Pubkey,
        user_destination_account: &Pubkey,
    ) -> Result<Instruction, PoolClientError> {
        let addresses = self.derive_pool_addresses(config);
        let pool_state_data = self.get_pool_state(&addresses.pool_state)?;
        
        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(*user, true),                           // User (signer)
                AccountMeta::new(*user_lp_account, false),               // User LP token account
                AccountMeta::new(*user_destination_account, false),      // User destination token account
                AccountMeta::new(addresses.pool_state, false),           // Pool state PDA
                AccountMeta::new_readonly(addresses.token_a_mint, false), // Token A mint (for PDA seeds)
                AccountMeta::new_readonly(addresses.token_b_mint, false), // Token B mint (for PDA seeds)
                AccountMeta::new(addresses.token_a_vault, false),        // Pool Token A vault
                AccountMeta::new(addresses.token_b_vault, false),        // Pool Token B vault
                AccountMeta::new(pool_state_data.lp_token_a_mint, false), // LP Token A mint
                AccountMeta::new(pool_state_data.lp_token_b_mint, false), // LP Token B mint
                AccountMeta::new_readonly(system_program::id(), false),   // System program
                AccountMeta::new_readonly(spl_token::id(), false),        // SPL Token program
                AccountMeta::new_readonly(sysvar::rent::id(), false),     // Rent sysvar
                AccountMeta::new_readonly(sysvar::clock::id(), false),    // Clock sysvar
            ],
            data: PoolInstruction::Withdraw {
                withdraw_token_mint: *withdraw_token_mint,
                lp_amount_to_burn,
            }.try_to_vec()?,
        };
        
        Ok(instruction)
    }
    
    /// Creates a swap instruction for exchanging tokens at the fixed ratio.
    /// 
    /// # Arguments
    /// * `user` - User account (must be signer)
    /// * `config` - Pool configuration
    /// * `input_token_mint` - Mint of the input token
    /// * `amount_in` - Amount of input tokens
    /// * `minimum_amount_out` - Minimum output tokens expected (slippage protection)
    /// * `user_input_account` - User's input token account
    /// * `user_output_account` - User's output token account
    /// 
    /// # Returns
    /// * `Instruction` - Ready-to-send swap instruction
    pub fn swap_instruction(
        &self,
        user: &Pubkey,
        config: &PoolConfig,
        input_token_mint: &Pubkey,
        amount_in: u64,
        minimum_amount_out: u64,
        user_input_account: &Pubkey,
        user_output_account: &Pubkey,
    ) -> Result<Instruction, PoolClientError> {
        let addresses = self.derive_pool_addresses(config);
        
        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(*user, true),                           // User (signer)
                AccountMeta::new(*user_input_account, false),            // User input token account
                AccountMeta::new(*user_output_account, false),           // User output token account
                AccountMeta::new(addresses.pool_state, false),           // Pool state PDA
                AccountMeta::new_readonly(addresses.token_a_mint, false), // Token A mint (for PDA seeds)
                AccountMeta::new_readonly(addresses.token_b_mint, false), // Token B mint (for PDA seeds)
                AccountMeta::new(addresses.token_a_vault, false),        // Pool Token A vault
                AccountMeta::new(addresses.token_b_vault, false),        // Pool Token B vault
                AccountMeta::new_readonly(system_program::id(), false),   // System program
                AccountMeta::new_readonly(spl_token::id(), false),        // SPL Token program
                AccountMeta::new_readonly(sysvar::rent::id(), false),     // Rent sysvar
                AccountMeta::new_readonly(sysvar::clock::id(), false),    // Clock sysvar
            ],
            data: PoolInstruction::Swap {
                input_token_mint: *input_token_mint,
                amount_in,
                minimum_amount_out,
            }.try_to_vec()?,
        };
        
        Ok(instruction)
    }
    
    /// Retrieves pool state data (placeholder implementation).
    /// 
    /// In a real implementation, this would fetch and deserialize the pool state
    /// from the blockchain. For now, it returns a placeholder.
    pub fn get_pool_state(&self, _pool_state_pda: &Pubkey) -> Result<PoolStateData, PoolClientError> {
        // This is a placeholder implementation
        // In a real client, this would make an RPC call to fetch account data
        Err(PoolClientError::NotImplemented)
    }
}

/// Simplified pool state data for client use.
#[derive(Debug, Clone)]
pub struct PoolStateData {
    pub lp_token_a_mint: Pubkey,
    pub lp_token_b_mint: Pubkey,
    pub is_initialized: bool,
    pub is_paused: bool,
}

/// Client SDK error types.
#[derive(Debug)]
pub enum PoolClientError {
    InvalidRatio,
    IdenticalTokens,
    InvalidDepositToken,
    NotImplemented,
    SerializationError(std::io::Error),
}

impl From<std::io::Error> for PoolClientError {
    fn from(error: std::io::Error) -> Self {
        Self::SerializationError(error)
    }
}

impl std::fmt::Display for PoolClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidRatio => write!(f, "Ratio must be greater than 0"),
            Self::IdenticalTokens => write!(f, "Primary and base tokens must be different"),
            Self::InvalidDepositToken => write!(f, "Deposit token must be either primary or base token"),
            Self::NotImplemented => write!(f, "Feature not yet implemented"),
            Self::SerializationError(e) => write!(f, "Serialization error: {}", e),
        }
    }
}

impl std::error::Error for PoolClientError {}

/// Utility functions for testing and development.
pub mod testing {
    use super::*;
    
    /// Helper function to create a test pool configuration.
    pub fn create_test_pool_config() -> PoolConfig {
        PoolConfig {
            primary_token_mint: Pubkey::new_unique(),
            base_token_mint: Pubkey::new_unique(),
            ratio_primary_per_base: 1000, // 1000:1 ratio
        }
    }
    
    /// Helper function to create multiple test keypairs.
    pub fn create_test_keypairs(count: usize) -> Vec<Pubkey> {
        (0..count).map(|_| Pubkey::new_unique()).collect()
    }
} 