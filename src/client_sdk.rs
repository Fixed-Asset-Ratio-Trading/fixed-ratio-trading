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
//! This module provides a high-level client SDK for interacting with the Fixed Ratio Trading Pool program.
//! It simplifies the process of creating pools, managing liquidity, and performing swaps.
//!
//! ## Features
//! - Pool creation and configuration
//! - Address derivation for PDAs (Program Derived Addresses)
//! - Instruction building for all pool operations
//! - Error handling and validation
//! - Type-safe pool configuration
//!
//! ## Quick Start
//! 
//! ```rust
//! use fixed_ratio_trading::client_sdk::{PoolClient, PoolConfig};
//! use solana_program::pubkey::Pubkey;
//! 
//! // Create a pool client
//! let client = PoolClient::new(program_id);
//! 
//! // Configure a pool
//! let config = PoolConfig {
//!     multiple_token_mint: multiple_token_mint,
//!     base_token_mint: base_token_mint,
//!     multiple_per_base: 2,
//! };
//! 
//! // Derive pool addresses
//! let addresses = client.derive_pool_addresses(&config);
//! 
//! // Create pool instruction
//! let instruction = client.create_pool_instruction(&payer, &config, &lp_token_a_mint, &lp_token_b_mint)?;
//! ```

use borsh::BorshSerialize;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
    sysvar::{rent, clock},
};
use spl_token;

use crate::{
    constants::{POOL_STATE_SEED_PREFIX, TOKEN_A_VAULT_SEED_PREFIX, TOKEN_B_VAULT_SEED_PREFIX},
    types::instructions::PoolInstruction,
};

/// Errors that can occur when using the pool client
#[derive(Debug)]
pub enum PoolClientError {
    /// Invalid ratio provided (must be > 0)
    InvalidRatio,
    /// Invalid deposit token (must be either multiple or base token)
    InvalidDepositToken,
    /// Feature not yet implemented
    NotImplemented,
    /// Error during instruction serialization
    SerializationError,
}

impl From<std::io::Error> for PoolClientError {
    fn from(_error: std::io::Error) -> Self {
        Self::SerializationError
    }
}

impl std::fmt::Display for PoolClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolClientError::InvalidRatio => write!(f, "Invalid ratio: must be greater than 0"),
            PoolClientError::InvalidDepositToken => write!(f, "Invalid deposit token: must be either multiple or base token"),
            PoolClientError::NotImplemented => write!(f, "Feature not yet implemented"),
            PoolClientError::SerializationError => write!(f, "Failed to serialize instruction data"),
        }
    }
}

impl std::error::Error for PoolClientError {}

/// Configuration for creating a trading pool
/// 
/// This struct defines the parameters needed to create a new fixed-ratio trading pool.
/// The pool will exchange tokens at a fixed rate determined by the multiple_per_base ratio.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// The token that appears in larger quantities in the ratio (abundant token)
    /// Example: In a 1000:1 ratio, if USDC:SOL, then USDC is the multiple token
    pub multiple_token_mint: Pubkey,
    
    /// The token that appears as 1 in the ratio (valuable token)
    /// Example: In a 1000:1 ratio, if USDC:SOL, then SOL is the base token
    pub base_token_mint: Pubkey,
    
    /// Exchange ratio: how many multiple tokens per base token
    /// Example: multiple_per_base = 2 means 2 USDC per 1 SOL
    pub multiple_per_base: u64,
}

impl PoolConfig {
    /// Creates a new pool configuration
    /// 
    /// # Arguments
    /// * `multiple_token_mint` - Mint address of the multiple token (abundant)
    /// * `base_token_mint` - Mint address of the base token (valuable)
    /// * `multiple_per_base` - How many multiple tokens equal one base token
    /// 
    /// # Returns
    /// * `Result<PoolConfig, PoolClientError>` - The pool configuration or an error
    /// 
    /// # Errors
    /// * `InvalidRatio` - If multiple_per_base is 0
    pub fn new(
        multiple_token_mint: Pubkey,
        base_token_mint: Pubkey,
        multiple_per_base: u64,
    ) -> Result<Self, PoolClientError> {
        if multiple_per_base == 0 {
            return Err(PoolClientError::InvalidRatio);
        }

        Ok(Self {
            multiple_token_mint,
            base_token_mint,
            multiple_per_base,
        })
    }
}

/// Derived addresses for a pool configuration
/// 
/// This struct contains all the program-derived addresses (PDAs) that are
/// automatically calculated for a given pool configuration.
#[derive(Debug, Clone)]
pub struct PoolAddresses {
    /// Pool state account address
    pub pool_state: Pubkey,
    /// Pool authority bump seed for PDA derivation
    pub pool_authority_bump: u8,
    /// Normalized token A mint (lexicographically first)
    pub token_a_mint: Pubkey,
    /// Normalized token B mint (lexicographically second)
    pub token_b_mint: Pubkey,
    /// Normalized ratio A numerator  
    pub ratio_a_numerator: u64,
    /// Normalized ratio B denominator (always 1)
    pub ratio_b_denominator: u64,
    /// Token A vault address
    pub token_a_vault: Pubkey,
    /// Token A vault bump seed
    pub token_a_vault_bump: u8,
    /// Token B vault address
    pub token_b_vault: Pubkey,
    /// Token B vault bump seed
    pub token_b_vault_bump: u8,
}

/// High-level client for interacting with Fixed Ratio Trading Pools
/// 
/// This client provides convenient methods for all pool operations including:
/// - Creating new pools
/// - Deriving addresses
/// - Building instructions
/// - Managing liquidity
/// - Performing swaps
pub struct PoolClient {
    /// The program ID of the deployed pool program
    program_id: Pubkey,
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
            if config.multiple_token_mint < config.base_token_mint {
                (config.multiple_token_mint, config.base_token_mint)
            } else {
                (config.base_token_mint, config.multiple_token_mint)
            };
        
        // Step 2: Canonical ratio mapping to prevent liquidity fragmentation
        let (ratio_a_numerator, ratio_b_denominator): (u64, u64) = 
            if config.multiple_token_mint < config.base_token_mint {
                (config.multiple_per_base, 1u64)
            } else {
                // Use canonical form - all pools with same token pair get same ratio
                (config.multiple_per_base, 1u64)
            };
        
        // Derive pool state PDA
        let (pool_state, pool_authority_bump) = Pubkey::find_program_address(
            &[
                POOL_STATE_SEED_PREFIX,
                token_a_mint.as_ref(),
                token_b_mint.as_ref(),
                &ratio_a_numerator.to_le_bytes(),
                &ratio_b_denominator.to_le_bytes(),
            ],
            &self.program_id,
        );
        
        // Derive vault PDAs
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
            pool_authority_bump,
            token_a_mint,
            token_b_mint,
            ratio_a_numerator,
            ratio_b_denominator,
            token_a_vault,
            token_a_vault_bump,
            token_b_vault,
            token_b_vault_bump,
        }
    }
    
    /// Creates a pool initialization instruction.
    /// 
    /// This function creates the instruction needed to initialize a new trading pool
    /// with the specified configuration.
    /// 
    /// # Arguments
    /// * `payer` - Account that will pay for pool creation and sign the transaction
    /// * `config` - Pool configuration containing token mints and ratio
    /// * `lp_token_a_mint` - LP token mint for token A liquidity providers
    /// * `lp_token_b_mint` - LP token mint for token B liquidity providers
    /// 
    /// # Returns
    /// * `Result<Instruction, PoolClientError>` - The pool creation instruction or an error
    /// 
    /// # Errors
    /// * `InvalidRatio` - If the ratio in config is 0
    /// * `SerializationError` - If instruction data serialization fails
    pub fn create_pool_instruction(
        &self,
        payer: &Pubkey,
        config: &PoolConfig,
        lp_token_a_mint: &Pubkey,
        lp_token_b_mint: &Pubkey,
    ) -> Result<Instruction, PoolClientError> {
        let addresses = self.derive_pool_addresses(config);
        
        // Validate inputs
        if config.multiple_per_base == 0 {
            return Err(PoolClientError::InvalidRatio);
        }
        
        // Map bump seeds back to multiple/base token convention
        let (multiple_vault_bump, base_vault_bump) = 
            if config.multiple_token_mint < config.base_token_mint {
                (addresses.token_a_vault_bump, addresses.token_b_vault_bump)
            } else {
                (addresses.token_b_vault_bump, addresses.token_a_vault_bump)
            };
        
        // Create instruction
        let instruction_data = PoolInstruction::InitializePool {
            multiple_per_base: config.multiple_per_base,
            pool_authority_bump_seed: addresses.pool_authority_bump,
            multiple_token_vault_bump_seed: multiple_vault_bump,
            base_token_vault_bump_seed: base_vault_bump,
        };
        
        let data = instruction_data
            .try_to_vec()
            .map_err(|_| PoolClientError::SerializationError)?;
        
        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(*payer, true),                         // Payer (signer)
                AccountMeta::new(addresses.pool_state, false),          // Pool state PDA
                AccountMeta::new_readonly(config.multiple_token_mint, false), // Multiple token mint
                AccountMeta::new_readonly(config.base_token_mint, false),     // Base token mint
                AccountMeta::new(*lp_token_a_mint, true),               // LP Token A mint (signer)
                AccountMeta::new(*lp_token_b_mint, true),               // LP Token B mint (signer)
                AccountMeta::new(addresses.token_a_vault, false),       // Token A vault PDA
                AccountMeta::new(addresses.token_b_vault, false),       // Token B vault PDA
                AccountMeta::new_readonly(system_program::id(), false), // System program
                AccountMeta::new_readonly(spl_token::id(), false),      // SPL Token program
                AccountMeta::new_readonly(rent::id(), false),           // Rent sysvar
            ],
            data,
        })
    }
    
    /// Creates a deposit instruction for adding liquidity to a pool.
    /// 
    /// # Arguments
    /// * `user` - The user performing the deposit
    /// * `config` - Pool configuration
    /// * `deposit_token_mint` - Token being deposited
    /// * `amount` - Amount to deposit
    /// * `user_source_account` - User's token account
    /// * `user_lp_account` - User's LP token account
    /// 
    /// # Returns
    /// * `Result<Instruction, PoolClientError>` - The deposit instruction or an error
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
        if *deposit_token_mint != config.multiple_token_mint && *deposit_token_mint != config.base_token_mint {
            return Err(PoolClientError::InvalidDepositToken);
        }

        let instruction_data = PoolInstruction::Deposit {
            deposit_token_mint: *deposit_token_mint,
            amount,
        };

        let data = instruction_data.try_to_vec()?;

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(*user, true),                          // User (signer)
                AccountMeta::new(addresses.pool_state, false),          // Pool state
                AccountMeta::new(*user_source_account, false),          // User source token account
                AccountMeta::new(*user_lp_account, false),              // User LP token account
                AccountMeta::new(addresses.token_a_vault, false),       // Token A vault
                AccountMeta::new(addresses.token_b_vault, false),       // Token B vault
                AccountMeta::new_readonly(system_program::id(), false), // System program
                AccountMeta::new_readonly(spl_token::id(), false),      // SPL Token program
                AccountMeta::new_readonly(rent::id(), false),           // Rent sysvar
                AccountMeta::new_readonly(clock::id(), false),          // Clock sysvar
            ],
            data,
        })
    }
    
    /// Creates an enhanced deposit instruction with additional features.
    /// 
    /// # Arguments  
    /// * `user` - The user performing the deposit
    /// * `config` - Pool configuration
    /// * `deposit_token_mint` - Token being deposited
    /// * `amount` - Amount to deposit
    /// * `minimum_lp_tokens_out` - Minimum LP tokens expected (slippage protection)
    /// * `fee_recipient` - Optional custom fee recipient
    /// * `user_source_account` - User's token account
    /// * `user_lp_account` - User's LP token account
    /// 
    /// # Returns
    /// * `Result<Instruction, PoolClientError>` - The enhanced deposit instruction or an error
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
        if *deposit_token_mint != config.multiple_token_mint && *deposit_token_mint != config.base_token_mint {
            return Err(PoolClientError::InvalidDepositToken);
        }

        let instruction_data = PoolInstruction::DepositWithFeatures {
            deposit_token_mint: *deposit_token_mint,
            amount,
            minimum_lp_tokens_out,
            fee_recipient,
        };

        let data = instruction_data.try_to_vec()?;

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(*user, true),                          // User (signer)
                AccountMeta::new(addresses.pool_state, false),          // Pool state
                AccountMeta::new(*user_source_account, false),          // User source token account
                AccountMeta::new(*user_lp_account, false),              // User LP token account
                AccountMeta::new(addresses.token_a_vault, false),       // Token A vault
                AccountMeta::new(addresses.token_b_vault, false),       // Token B vault
                AccountMeta::new_readonly(system_program::id(), false), // System program
                AccountMeta::new_readonly(spl_token::id(), false),      // SPL Token program
                AccountMeta::new_readonly(rent::id(), false),           // Rent sysvar
                AccountMeta::new_readonly(clock::id(), false),          // Clock sysvar
            ],
            data,
        })
    }
    
    /// Creates a withdraw instruction for removing liquidity from a pool.
    /// 
    /// # Arguments
    /// * `user` - The user performing the withdrawal
    /// * `config` - Pool configuration
    /// * `withdraw_token_mint` - Token being withdrawn
    /// * `lp_amount_to_burn` - Amount of LP tokens to burn
    /// * `user_destination_account` - User's destination token account
    /// * `user_lp_account` - User's LP token account
    /// 
    /// # Returns
    /// * `Result<Instruction, PoolClientError>` - The withdraw instruction or an error
    pub fn withdraw_instruction(
        &self,
        user: &Pubkey,
        config: &PoolConfig,
        withdraw_token_mint: &Pubkey,
        lp_amount_to_burn: u64,
        user_destination_account: &Pubkey,
        user_lp_account: &Pubkey,
    ) -> Result<Instruction, PoolClientError> {
        let addresses = self.derive_pool_addresses(config);

        let instruction_data = PoolInstruction::Withdraw {
            withdraw_token_mint: *withdraw_token_mint,
            lp_amount_to_burn,
        };

        let data = instruction_data.try_to_vec()?;

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(*user, true),                          // User (signer)
                AccountMeta::new(addresses.pool_state, false),          // Pool state
                AccountMeta::new(*user_destination_account, false),     // User destination token account
                AccountMeta::new(*user_lp_account, false),              // User LP token account
                AccountMeta::new(addresses.token_a_vault, false),       // Token A vault
                AccountMeta::new(addresses.token_b_vault, false),       // Token B vault
                AccountMeta::new_readonly(system_program::id(), false), // System program
                AccountMeta::new_readonly(spl_token::id(), false),      // SPL Token program
                AccountMeta::new_readonly(rent::id(), false),           // Rent sysvar
                AccountMeta::new_readonly(clock::id(), false),          // Clock sysvar
            ],
            data,
        })
    }

    /// Creates a swap instruction for exchanging tokens.
    /// 
    /// # Arguments
    /// * `user` - The user performing the swap
    /// * `config` - Pool configuration
    /// * `input_token_mint` - Token being swapped
    /// * `amount_in` - Amount of input tokens
    /// * `minimum_amount_out` - Minimum amount of output tokens expected
    /// * `user_source_account` - User's source token account
    /// * `user_destination_account` - User's destination token account
    /// 
    /// # Returns
    /// * `Result<Instruction, PoolClientError>` - The swap instruction or an error
    pub fn swap_instruction(
        &self,
        user: &Pubkey,
        config: &PoolConfig,
        input_token_mint: &Pubkey,
        amount_in: u64,
        minimum_amount_out: u64,
        user_source_account: &Pubkey,
        user_destination_account: &Pubkey,
    ) -> Result<Instruction, PoolClientError> {
        let addresses = self.derive_pool_addresses(config);

        let instruction_data = PoolInstruction::Swap {
            input_token_mint: *input_token_mint,
            amount_in,
            minimum_amount_out,
        };

        let data = instruction_data.try_to_vec()?;

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(*user, true),                          // User (signer)
                AccountMeta::new(addresses.pool_state, false),          // Pool state
                AccountMeta::new(*user_source_account, false),          // User source token account
                AccountMeta::new(*user_destination_account, false),     // User destination token account
                AccountMeta::new(addresses.token_a_vault, false),       // Token A vault
                AccountMeta::new(addresses.token_b_vault, false),       // Token B vault
                AccountMeta::new_readonly(system_program::id(), false), // System program
                AccountMeta::new_readonly(spl_token::id(), false),      // SPL Token program
                AccountMeta::new_readonly(rent::id(), false),           // Rent sysvar
                AccountMeta::new_readonly(clock::id(), false),          // Clock sysvar
            ],
            data,
        })
    }

    /// Placeholder for additional pool operations.
    /// 
    /// # Returns
    /// * `Result<(), PoolClientError>` - Currently returns NotImplemented
    pub fn additional_operations(&self) -> Result<(), PoolClientError> {
        Err(PoolClientError::NotImplemented)
    }
}

/// Pool state information for debugging and testing
#[derive(Debug)]
pub struct PoolState {
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub ratio_a_numerator: u64,
    pub ratio_b_denominator: u64,
    pub is_paused: bool,
}

/// Utility functions for testing and development.
pub mod test_utils {
    use super::*;

    /// Creates a test pool configuration for testing purposes.
    /// 
    /// # Returns
    /// * `PoolConfig` - A test configuration with random mints and 1000:1 ratio
    pub fn create_test_pool_config() -> PoolConfig {
        PoolConfig {
            multiple_token_mint: Pubkey::new_unique(),
            base_token_mint: Pubkey::new_unique(),
            multiple_per_base: 1000, // 1000:1 ratio
        }
    }
} 