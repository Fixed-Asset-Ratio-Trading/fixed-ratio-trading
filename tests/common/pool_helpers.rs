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

//! # Pool Creation and Management Utilities
//! 
//! This module provides utilities for creating and managing liquidity pools
//! in integration tests, including both the deprecated two-instruction pattern
//! and the new single-instruction pattern for pool initialization.

use solana_program_test::BanksClient;
use solana_sdk::{signature::Keypair, signer::Signer};
use borsh::BorshSerialize;
use crate::common::{constants, TestResult, *};

/// Normalized pool configuration data
/// 
/// Contains the normalized token mints, ratios, and derived PDAs for a pool
#[derive(Debug, Clone)]
#[allow(dead_code)] // Allow unused fields as this is a comprehensive config struct for tests
pub struct PoolConfig {
    /// Normalized token A mint (lexicographically smaller)
    pub token_a_mint: Pubkey,
    /// Normalized token B mint (lexicographically larger)
    pub token_b_mint: Pubkey,
    /// Normalized ratio A numerator
    pub ratio_a_numerator: u64,
    /// Normalized ratio B denominator
    pub ratio_b_denominator: u64,
    /// True if multiple token became token A after normalization
    pub token_a_is_the_multiple: bool,
    /// Pool state PDA
    pub pool_state_pda: Pubkey,
    /// Pool authority bump seed
    pub pool_authority_bump: u8,
    /// Token A vault PDA
    pub token_a_vault_pda: Pubkey,
    /// Token A vault bump seed
    pub token_a_vault_bump: u8,
    /// Token B vault PDA
    pub token_b_vault_pda: Pubkey,
    /// Token B vault bump seed
    pub token_b_vault_bump: u8,
    /// Multiple token vault bump (for instruction)
    pub multiple_vault_bump: u8,
    /// Base token vault bump (for instruction)
    pub base_vault_bump: u8,
}

/// Backwards compatibility wrapper for normalize_pool_config
/// 
/// # Arguments
/// * `multiple_mint` - Multiple token mint (abundant token)
/// * `base_mint` - Base token mint (valuable token)
/// * `multiple_per_base` - Ratio of multiple tokens per base token (legacy format)
/// 
/// # Returns
/// Normalized pool configuration with all derived addresses
pub fn normalize_pool_config_legacy(
    multiple_mint: &Pubkey,
    base_mint: &Pubkey,
    multiple_per_base: u64,
) -> PoolConfig {
    // Convert legacy single ratio to new dual ratio format
    // For backwards compatibility, we assume denominator of 1
    normalize_pool_config(multiple_mint, base_mint, multiple_per_base, 1)
}

/// Normalize pool parameters and derive PDAs
/// 
/// This function performs enhanced normalization logic that prevents creation of 
/// economically equivalent pools. It ensures tokens are ordered lexicographically
/// and detects inverse exchange rates that would fragment liquidity.
/// 
/// **CRITICAL INVARIANT**: This function prevents market fragmentation by ensuring
/// that pools with equivalent exchange rates (like "3 A per 1 B" and "1 B per 3 A")
/// normalize to the same configuration, preventing duplicate economic pools.
/// 
/// # Arguments
/// * `multiple_mint` - Multiple token mint (abundant token)
/// * `base_mint` - Base token mint (valuable token)
/// * `ratio_a_numerator` - Token A base units
/// * `ratio_b_denominator` - Token B base units
/// 
/// # Returns
/// Normalized pool configuration with all derived addresses
/// 
/// # Important Note
/// This prevents liquidity fragmentation by ensuring economically equivalent
/// pools (like A/B at 3:1 and B/A at 1:3) resolve to the same pool configuration.
pub fn normalize_pool_config(
    multiple_mint: &Pubkey,
    base_mint: &Pubkey,
    ratio_a_numerator: u64,
    ratio_b_denominator: u64,
) -> PoolConfig {
    // Step 1: Lexicographic token ordering  
    let (token_a_mint, token_b_mint) = 
        if multiple_mint.to_bytes() < base_mint.to_bytes() {
            (*multiple_mint, *base_mint)
        } else if multiple_mint.to_bytes() > base_mint.to_bytes() {
            (*base_mint, *multiple_mint)
        } else {
            panic!("Multiple and Base token mints cannot be the same");
        };
    
    // Step 2: Use provided ratios directly (already in base units)
    // The ratios are provided as base units, so we use them as-is
    // Token ordering is handled by the lexicographic ordering above
    let token_a_is_the_multiple = multiple_mint.to_bytes() < base_mint.to_bytes();

    // Derive pool state PDA using NORMALIZED values
    let (pool_state_pda, pool_authority_bump) = Pubkey::find_program_address(
        &[
            POOL_STATE_SEED_PREFIX,
            token_a_mint.as_ref(),
            token_b_mint.as_ref(),
            &ratio_a_numerator.to_le_bytes(),
            &ratio_b_denominator.to_le_bytes(),
        ],
        &PROGRAM_ID,
    );

    // Derive vault PDAs
    let (token_a_vault_pda, token_a_vault_bump) = Pubkey::find_program_address(
        &[TOKEN_A_VAULT_SEED_PREFIX, pool_state_pda.as_ref()],
        &PROGRAM_ID,
    );
    let (token_b_vault_pda, token_b_vault_bump) = Pubkey::find_program_address(
        &[TOKEN_B_VAULT_SEED_PREFIX, pool_state_pda.as_ref()],
        &PROGRAM_ID,
    );

    // Map vault bumps back to instruction parameters
    let (multiple_vault_bump, base_vault_bump) = if token_a_is_the_multiple {
        (token_a_vault_bump, token_b_vault_bump)
    } else {
        (token_b_vault_bump, token_a_vault_bump)
    };

    PoolConfig {
        token_a_mint,
        token_b_mint,
        ratio_a_numerator,
        ratio_b_denominator,
        token_a_is_the_multiple,
        pool_state_pda,
        pool_authority_bump,
        token_a_vault_pda,
        token_a_vault_bump,
        token_b_vault_pda,
        token_b_vault_bump,
        multiple_vault_bump,
        base_vault_bump,
    }
}

/// Create pool using the new single-instruction pattern (RECOMMENDED)
/// 
/// This function uses the InitializePool instruction to create and initialize
/// a pool in a single atomic operation.
/// 
/// # Arguments
/// * `banks` - Banks client for transaction processing
/// * `payer` - Account that pays for pool creation
/// * `recent_blockhash` - Recent blockhash for transaction
/// * `multiple_mint` - Multiple token mint keypair (abundant token)
/// * `base_mint` - Base token mint keypair (valuable token)
/// * `lp_token_a_mint` - LP Token A mint keypair
/// * `lp_token_b_mint` - LP Token B mint keypair
/// * `multiple_per_base` - Ratio of multiple tokens per base token
/// 
/// # Returns
/// Pool configuration with all derived addresses
#[allow(dead_code)]
pub async fn create_pool_new_pattern(
    banks: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
    multiple_mint: &Keypair,
    base_mint: &Keypair,
    lp_token_a_mint: &Keypair,
    lp_token_b_mint: &Keypair,
    multiple_per_base: Option<u64>,
) -> Result<PoolConfig, BanksClientError> {
    let ratio = multiple_per_base.unwrap_or(constants::DEFAULT_RATIO);
    
    // Get normalized pool configuration
    let config = normalize_pool_config_legacy(&multiple_mint.pubkey(), &base_mint.pubkey(), ratio);

    // Create InitializePool instruction
    let initialize_pool_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),                          // Payer (signer)
            AccountMeta::new(config.pool_state_pda, false),                  // Pool state PDA
            AccountMeta::new_readonly(multiple_mint.pubkey(), false),        // Multiple token mint
            AccountMeta::new_readonly(base_mint.pubkey(), false),            // Base token mint
            AccountMeta::new(lp_token_a_mint.pubkey(), true),                // LP Token A mint (signer)
            AccountMeta::new(lp_token_b_mint.pubkey(), true),                // LP Token B mint (signer)
            AccountMeta::new(config.token_a_vault_pda, false),               // Token A vault PDA
            AccountMeta::new(config.token_b_vault_pda, false),               // Token B vault PDA
            AccountMeta::new_readonly(solana_program::system_program::id(), false), // System program
            AccountMeta::new_readonly(spl_token::id(), false),                      // SPL Token program
            AccountMeta::new_readonly(solana_program::sysvar::rent::id(), false),   // Rent sysvar
        ],
        data: PoolInstruction::InitializePool {
            ratio_a_numerator: config.ratio_a_numerator,
            ratio_b_denominator: config.ratio_b_denominator,
            pool_authority_bump_seed: config.pool_authority_bump,
            token_a_vault_bump_seed: config.token_a_vault_bump,
            token_b_vault_bump_seed: config.token_b_vault_bump,
        }.try_to_vec().unwrap(),
    };

    // Send transaction
    let mut transaction = Transaction::new_with_payer(&[initialize_pool_ix], Some(&payer.pubkey()));
    let signers = [payer, lp_token_a_mint, lp_token_b_mint];
    transaction.sign(&signers[..], recent_blockhash);
    banks.process_transaction(transaction).await?;

    Ok(config)
}

/// Create pool using the legacy pattern (now redirects to new pattern)
/// 
/// DEPRECATED: Legacy two-instruction pattern is no longer supported.
/// This function now uses the single InitializePool instruction for compatibility.
/// 
/// # Arguments
/// * `banks` - Banks client for transaction processing
/// * `payer` - Account that pays for pool creation
/// * `recent_blockhash` - Recent blockhash for transaction
/// * `multiple_mint` - Multiple token mint keypair (abundant token)
/// * `base_mint` - Base token mint keypair (valuable token)
/// * `lp_token_a_mint` - LP Token A mint keypair
/// * `lp_token_b_mint` - LP Token B mint keypair
/// * `multiple_per_base` - Ratio of multiple tokens per base token
/// 
/// # Returns
/// Pool configuration with all derived addresses
#[allow(dead_code)]
pub async fn create_pool_legacy_pattern(
    banks: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
    multiple_mint: &Keypair,
    base_mint: &Keypair,
    lp_token_a_mint: &Keypair,
    lp_token_b_mint: &Keypair,
    multiple_per_base: Option<u64>,
) -> Result<PoolConfig, BanksClientError> {
    println!("ℹ️ Legacy pattern redirecting to new pattern (InitializePool)");
    
    // Redirect to new pattern since deprecated instructions were removed
    create_pool_new_pattern(
        banks,
        payer,
        recent_blockhash,
        multiple_mint,
        base_mint,
        lp_token_a_mint,
        lp_token_b_mint,
        multiple_per_base,
    ).await
}

/// Update security parameters for a pool
/// 
/// # Arguments
/// * `banks` - Banks client for transaction processing
/// * `payer` - Pool owner (pays for transaction)
/// * `recent_blockhash` - Recent blockhash for transaction
/// * `pool_state_pda` - Pool state account
/// * `paused` - Whether pool is paused (optional)
#[allow(dead_code)]
pub async fn update_security_params(
    banks: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
    pool_state_pda: &Pubkey,
    paused: Option<bool>,
) -> TestResult {
    let update_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),                          // Pool owner (signer)
            AccountMeta::new(*pool_state_pda, false),                        // Pool state account
            AccountMeta::new_readonly(solana_program::sysvar::rent::id(), false), // Rent sysvar
        ],
        data: PoolInstruction::UpdateSecurityParams {
            paused,
            only_lp_token_a_for_both: None, // Not implemented yet
        }.try_to_vec().unwrap(),
    };

    let mut transaction = Transaction::new_with_payer(&[update_ix], Some(&payer.pubkey()));
    transaction.sign(&[payer], recent_blockhash);
    banks.process_transaction(transaction).await
}

/// Get pool state data with debug information
/// 
/// # Arguments
/// * `banks` - Banks client for account fetching
/// * `pool_state_pda` - Pool state account
/// 
/// # Returns
/// Deserialized pool state or None if account doesn't exist
#[allow(dead_code)]
pub async fn get_pool_state(
    banks: &mut BanksClient,
    pool_state_pda: &Pubkey,
) -> Option<PoolState> {
    match banks.get_account(*pool_state_pda).await {
        Ok(Some(account)) => {
            match PoolState::deserialize(&mut &account.data[..]) {
                Ok(pool_state) => Some(pool_state),
                Err(_) => None
            }
        },
        _ => None
    }
}

/// Verify pool state matches expected configuration
/// 
/// # Arguments
/// * `banks` - Banks client for account fetching
/// * `config` - Expected pool configuration
/// * `owner` - Expected pool owner
/// * `lp_token_a_mint` - Expected LP Token A mint
/// * `lp_token_b_mint` - Expected LP Token B mint
#[allow(dead_code)]
pub async fn verify_pool_state(
    banks: &mut BanksClient,
    config: &PoolConfig,
    owner: &Pubkey,
    lp_token_a_mint: &Pubkey,
    lp_token_b_mint: &Pubkey,
) -> Result<(), String> {
    let pool_state = get_pool_state(banks, &config.pool_state_pda).await
        .ok_or("Pool state account not found")?;

    // Verify basic state
    if !pool_state.is_initialized {
        return Err("Pool should be initialized".to_string());
    }
    if pool_state.owner != *owner {
        return Err("Pool owner mismatch".to_string());
    }

    // Verify normalized tokens and ratios
    if pool_state.token_a_mint != config.token_a_mint {
        return Err("Token A mint mismatch".to_string());
    }
    if pool_state.token_b_mint != config.token_b_mint {
        return Err("Token B mint mismatch".to_string());
    }
    if pool_state.ratio_a_numerator != config.ratio_a_numerator {
        return Err("Ratio A numerator mismatch".to_string());
    }
    if pool_state.ratio_b_denominator != config.ratio_b_denominator {
        return Err("Ratio B denominator mismatch".to_string());
    }

    // Verify vault addresses
    if pool_state.token_a_vault != config.token_a_vault_pda {
        return Err("Token A vault PDA mismatch".to_string());
    }
    if pool_state.token_b_vault != config.token_b_vault_pda {
        return Err("Token B vault PDA mismatch".to_string());
    }

    // Verify LP token mints
    if pool_state.lp_token_a_mint != *lp_token_a_mint {
        return Err("LP Token A mint mismatch".to_string());
    }
    if pool_state.lp_token_b_mint != *lp_token_b_mint {
        return Err("LP Token B mint mismatch".to_string());
    }

    // Verify bump seeds
    if pool_state.pool_authority_bump_seed != config.pool_authority_bump {
        return Err("Pool authority bump mismatch".to_string());
    }
    if pool_state.token_a_vault_bump_seed != config.token_a_vault_bump {
        return Err("Token A vault bump mismatch".to_string());
    }
    if pool_state.token_b_vault_bump_seed != config.token_b_vault_bump {
        return Err("Token B vault bump mismatch".to_string());
    }

    Ok(())
} 