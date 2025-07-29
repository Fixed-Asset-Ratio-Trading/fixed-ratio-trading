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
//! 
//! **BASIS POINTS REFACTOR: Test Utilities**
//! 
//! This module now includes basis point-aware utilities for pool creation that handle
//! the conversion from display units to basis points before calling the smart contract.
//! New test functions should use the `*_with_display_units` variants for clarity.

use solana_program_test::BanksClient;
use solana_sdk::{signature::Keypair, signer::Signer};
use borsh::BorshSerialize;
use crate::common::{constants, *};
use fixed_ratio_trading::constants as frt_constants;
use fixed_ratio_trading::id;

// **PHASE 2.1**: Import treasury and pool state structures for consolidation
use fixed_ratio_trading::state::{MainTreasuryState};
use crate::common::liquidity_helpers::PoolFeeState;

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
        &id(),
    );

    // Derive vault PDAs
    let (token_a_vault_pda, token_a_vault_bump) = Pubkey::find_program_address(
        &[TOKEN_A_VAULT_SEED_PREFIX, pool_state_pda.as_ref()],
        &id(),
    );
    let (token_b_vault_pda, token_b_vault_bump) = Pubkey::find_program_address(
        &[TOKEN_B_VAULT_SEED_PREFIX, pool_state_pda.as_ref()],
        &id(),
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
    multiple_per_base: Option<u64>,
) -> Result<PoolConfig, BanksClientError> {
    let ratio = multiple_per_base.unwrap_or(constants::DEFAULT_RATIO);
    
    // Get normalized pool configuration
    let config = normalize_pool_config_legacy(&multiple_mint.pubkey(), &base_mint.pubkey(), ratio);

    // Check if pool already exists
    if let Some(_existing_pool) = get_pool_state(banks, &config.pool_state_pda).await {
        return Err(BanksClientError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "Pool already exists with this configuration"
        )));
    }

    // Derive main treasury PDA for fee collection
    let (main_treasury_pda, _) = Pubkey::find_program_address(
        &[frt_constants::MAIN_TREASURY_SEED_PREFIX],
        &id(),
    );

    // Derive system state PDA for pause validation
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[frt_constants::SYSTEM_STATE_SEED_PREFIX],
        &id(),
    );

    // Derive LP token mint PDAs
    let (lp_token_a_mint_pda, _) = Pubkey::find_program_address(
        &[frt_constants::LP_TOKEN_A_MINT_SEED_PREFIX, config.pool_state_pda.as_ref()],
        &id(),
    );
    let (lp_token_b_mint_pda, _) = Pubkey::find_program_address(
        &[frt_constants::LP_TOKEN_B_MINT_SEED_PREFIX, config.pool_state_pda.as_ref()],
        &id(),
    );

    // Use main treasury for all operations (Phase 3: Centralized Treasury)
    // Old specialized treasuries have been consolidated into main treasury

    // ✅ CORRECTED ACCOUNT ORDERING: Match processor expectations (13 accounts)
    let initialize_pool_ix = Instruction {
        program_id: id(),
        accounts: vec![
            // Account ordering matching processor documentation:
            AccountMeta::new(payer.pubkey(), true),                          // Index 0: User Authority Signer
            AccountMeta::new_readonly(solana_program::system_program::id(), false), // Index 1: System Program Account
            AccountMeta::new_readonly(system_state_pda, false),              // Index 2: System State PDA
            AccountMeta::new(config.pool_state_pda, false),                  // Index 3: Pool State PDA
            AccountMeta::new_readonly(spl_token::id(), false),               // Index 4: SPL Token Program Account
            AccountMeta::new(main_treasury_pda, false),                      // Index 5: Main Treasury PDA
            AccountMeta::new_readonly(solana_program::sysvar::rent::id(), false),   // Index 6: Rent Sysvar Account
            AccountMeta::new_readonly(multiple_mint.pubkey(), false),        // Index 7: Token A Mint Account
            AccountMeta::new_readonly(base_mint.pubkey(), false),            // Index 8: Token B Mint Account
            AccountMeta::new(config.token_a_vault_pda, false),               // Index 9: Token A Vault PDA
            AccountMeta::new(config.token_b_vault_pda, false),               // Index 10: Token B Vault PDA
            AccountMeta::new(lp_token_a_mint_pda, false),                    // Index 11: LP Token A Mint PDA
            AccountMeta::new(lp_token_b_mint_pda, false),                    // Index 12: LP Token B Mint PDA
        ],
        data: PoolInstruction::InitializePool {
            ratio_a_numerator: config.ratio_a_numerator,
            ratio_b_denominator: config.ratio_b_denominator,
        }.try_to_vec().unwrap(),
    };

    // ✅ COMPUTE BUDGET: Add compute budget instruction for pool creation (500K CUs)
    use solana_sdk::compute_budget::ComputeBudgetInstruction;
    let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(500_000);
    
    // ✅ PHASE 9 SECURITY: Send transaction with compute budget and pool creation instruction
    let mut transaction = Transaction::new_with_payer(
        &[compute_budget_ix, initialize_pool_ix], 
        Some(&payer.pubkey())
    );
    let signers = [payer]; // Only payer signs - LP token mints are derived as PDAs
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
        multiple_per_base,
    ).await
}

// Security parameter updates moved to governance control
// Pool owners no longer have direct security management rights

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
    _lp_token_a_mint: &Pubkey,
    _lp_token_b_mint: &Pubkey,
) -> Result<(), String> {
    let pool_state = get_pool_state(banks, &config.pool_state_pda).await
        .ok_or("Pool state account not found")?;

    // Verify basic state
    // Pool existence = initialization (no is_initialized field needed)
    if false { // Pool is always initialized if we can deserialize it
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

    // ✅ PHASE 9 SECURITY: Verify LP token mints are derived PDAs (not user-provided)
    let (expected_lp_token_a_mint, _) = Pubkey::find_program_address(
        &[
            frt_constants::LP_TOKEN_A_MINT_SEED_PREFIX,
            config.pool_state_pda.as_ref(),
        ],
        &id(),
    );
    
    let (expected_lp_token_b_mint, _) = Pubkey::find_program_address(
        &[
            frt_constants::LP_TOKEN_B_MINT_SEED_PREFIX,
            config.pool_state_pda.as_ref(),
        ],
        &id(),
    );
    
    if pool_state.lp_token_a_mint != expected_lp_token_a_mint {
        return Err("LP Token A mint mismatch - should be derived PDA".to_string());
    }
    if pool_state.lp_token_b_mint != expected_lp_token_b_mint {
        return Err("LP Token B mint mismatch - should be derived PDA".to_string());
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

// # Phase 1.1: Enhanced Pool Creation Helpers
// 
// These functions provide comprehensive pool creation with treasury counter verification
// and detailed result tracking for legitimate integration testing.

use fixed_ratio_trading::constants::MAIN_TREASURY_SEED_PREFIX;
use borsh::BorshDeserialize;

/// Result structure for enhanced pool creation operations
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PoolCreationResult {
    /// The created pool's PDA
    pub pool_pda: Pubkey,
    /// Treasury state before pool creation
    pub initial_treasury_state: MainTreasuryState,
    /// Treasury state after pool creation
    pub post_creation_treasury_state: MainTreasuryState,
    /// Amount of fees collected during pool creation
    pub fee_collected: u64,
    /// The pool configuration used
    pub pool_config: PoolConfig,
    /// Whether the pool creation was successful
    pub creation_successful: bool,
}

/// Result structure for multiple pool creation operations
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MultiPoolResult {
    /// Results from individual pool creations
    pub pool_results: Vec<PoolCreationResult>,
    /// Total fees collected across all pool creations
    pub total_fees_collected: u64,
    /// Total pools created successfully
    pub successful_pools: u32,
    /// Failed pool creation attempts
    pub failed_pools: u32,
}

/// **Phase 1.1: Enhanced pool creation with comprehensive treasury counter verification**
/// 
/// This function creates a pool and verifies that treasury counters are properly incremented.
/// It provides the foundation for legitimate integration testing of treasury functionality.
/// 
/// # Arguments
/// * `env` - Test environment containing banks client and program context
/// * `multiple_per_base` - Ratio of multiple token to base token
/// * `_ignored` - Ignored parameter for function compatibility
/// 
/// # Returns
/// * `PoolCreationResult` - Comprehensive results including treasury state changes
#[allow(dead_code)]
pub async fn execute_pool_creation_with_counter_verification(
    env: &mut crate::common::setup::TestEnvironment,
    multiple_per_base: u64,
    _ignored: u64,
) -> Result<PoolCreationResult, Box<dyn std::error::Error>> {
    println!("🏗️ Phase 1.1: Enhanced pool creation with treasury verification...");
    
    // Step 1: Get initial treasury state
    let (main_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        &fixed_ratio_trading::id(),
    );
    
    let initial_treasury_account = env.banks_client.get_account(main_treasury_pda).await?;
    let initial_treasury_state = if let Some(account) = initial_treasury_account {
        MainTreasuryState::try_from_slice(&account.data)?
    } else {
        return Err("Treasury account not found - ensure system is properly initialized".into());
    };
    
    println!("💰 Initial treasury state:");
    println!("   - Pool creation count: {}", initial_treasury_state.pool_creation_count);
    println!("   - Total pool creation fees: {}", initial_treasury_state.total_pool_creation_fees);
    println!("   - Total balance: {}", initial_treasury_state.total_balance);
    
    // Step 2: Create tokens for pool creation
    use crate::common::tokens::create_mint;
    use solana_sdk::signature::Keypair;
    let primary_mint = Keypair::new();
    let base_mint = Keypair::new();
    create_mint(&mut env.banks_client, &env.payer, env.recent_blockhash, &primary_mint, Some(6)).await?;
    create_mint(&mut env.banks_client, &env.payer, env.recent_blockhash, &base_mint, Some(6)).await?;
    
    // Step 3: Create the pool using existing infrastructure
    let pool_result = create_pool_new_pattern(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &primary_mint,
        &base_mint,
        Some(multiple_per_base), // Use multiple_per_base ratio
    ).await;
    
    let creation_successful = pool_result.is_ok();
    let pool_config = if creation_successful {
        pool_result.unwrap()
    } else {
        return Err(format!("Pool creation failed: {:?}", pool_result.err()).into());
    };
    
    // Step 4: Get post-creation treasury state
    let post_creation_treasury_account = env.banks_client.get_account(main_treasury_pda).await?;
    let post_creation_treasury_state = if let Some(account) = post_creation_treasury_account {
        MainTreasuryState::try_from_slice(&account.data)?
    } else {
        return Err("Treasury account not found after pool creation".into());
    };
    
    // Step 5: Verify treasury counter increments
    let pool_creation_count_increment = post_creation_treasury_state.pool_creation_count - initial_treasury_state.pool_creation_count;
    let fee_collected = post_creation_treasury_state.total_pool_creation_fees - initial_treasury_state.total_pool_creation_fees;
    let balance_change = post_creation_treasury_state.total_balance - initial_treasury_state.total_balance;
    
    println!("📊 Treasury verification results:");
    println!("   - Pool creation count increment: {}", pool_creation_count_increment);
    println!("   - Fees collected: {} lamports", fee_collected);
    println!("   - Balance change: {} lamports", balance_change);
    
    // Step 6: Validate increments are correct
    if pool_creation_count_increment != 1 {
        return Err(format!("Expected pool creation count to increment by 1, got {}", pool_creation_count_increment).into());
    }
    
    if fee_collected == 0 {
        return Err("Expected pool creation fees to be collected, but got 0".into());
    }
    
    if balance_change <= 0 {
        return Err(format!("Expected treasury balance to increase, but got change of {}", balance_change).into());
    }
    
    println!("✅ Treasury counter verification successful!");
    
    Ok(PoolCreationResult {
        pool_pda: pool_config.pool_state_pda,
        initial_treasury_state,
        post_creation_treasury_state,
        fee_collected,
        pool_config,
        creation_successful,
    })
}

/// **Phase 1.1: Create multiple pools for comprehensive testing**
/// 
/// This function creates multiple pools with different configurations and tracks
/// the cumulative impact on treasury counters.
/// 
/// # Arguments
/// * `env` - Test environment
/// * `pool_configs` - Vector of (ratio_a, ratio_b) tuples for different pools
/// 
/// # Returns
/// * `MultiPoolResult` - Results from all pool creation attempts
#[allow(dead_code)]
pub async fn create_multiple_pools_for_testing(
    env: &mut crate::common::setup::TestEnvironment,
    pool_configs: Vec<(u64, u64)>,
) -> Result<MultiPoolResult, Box<dyn std::error::Error>> {
    println!("🏗️ Phase 1.1: Creating {} pools for testing...", pool_configs.len());
    
    let mut pool_results = Vec::new();
    let mut total_fees_collected = 0u64;
    let mut successful_pools = 0u32;
    let mut failed_pools = 0u32;
    
    for (i, (ratio_a, ratio_b)) in pool_configs.iter().enumerate() {
        println!("🔄 Creating pool {}/{} with ratio {}:{}", i + 1, pool_configs.len(), ratio_a, ratio_b);
        
        match execute_pool_creation_with_counter_verification(env, *ratio_a, *ratio_b).await {
            Ok(result) => {
                total_fees_collected += result.fee_collected;
                successful_pools += 1;
                pool_results.push(result);
                println!("   ✅ Pool {} created successfully", i + 1);
            }
            Err(e) => {
                failed_pools += 1;
                println!("   ❌ Pool {} creation failed: {}", i + 1, e);
                // Create a failed result entry
                pool_results.push(PoolCreationResult {
                    pool_pda: Pubkey::default(),
                    initial_treasury_state: MainTreasuryState::new(),
                    post_creation_treasury_state: MainTreasuryState::new(), 
                    fee_collected: 0,
                    pool_config: PoolConfig {
                        token_a_mint: Pubkey::default(),
                        token_b_mint: Pubkey::default(),
                        ratio_a_numerator: *ratio_a,
                        ratio_b_denominator: *ratio_b,
                        token_a_is_the_multiple: false,
                        pool_state_pda: Pubkey::default(),
                        pool_authority_bump: 0,
                        token_a_vault_pda: Pubkey::default(),
                        token_a_vault_bump: 0,
                        token_b_vault_pda: Pubkey::default(),
                        token_b_vault_bump: 0,
                        multiple_vault_bump: 0,
                        base_vault_bump: 0,
                    },
                    creation_successful: false,
                });
            }
        }
    }
    
    println!("📊 Multi-pool creation summary:");
    println!("   - Total pools attempted: {}", pool_configs.len());
    println!("   - Successful: {}", successful_pools);
    println!("   - Failed: {}", failed_pools);
    println!("   - Total fees collected: {} lamports", total_fees_collected);
    
    Ok(MultiPoolResult {
        pool_results,
        total_fees_collected,
        successful_pools,
        failed_pools,
    })
}

/// **Phase 1.1: Verify pool creation fee collection in treasury**
/// 
/// This function verifies that pool creation fees were properly collected
/// by comparing treasury states before and after operations.
/// 
/// # Arguments
/// * `env` - Test environment
/// * `initial_treasury_state` - Treasury state before operations
/// 
/// # Returns
/// * `Result<u64, String>` - Amount of fees collected or error message
#[allow(dead_code)]
pub async fn verify_pool_creation_fee_collection(
    env: &mut crate::common::setup::TestEnvironment,
    initial_treasury_state: &MainTreasuryState,
) -> Result<u64, String> {
    println!("🔍 Phase 1.1: Verifying pool creation fee collection...");
    
    // Get current treasury state
    let (main_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        &fixed_ratio_trading::id(),
    );
    
    let current_treasury_account = env.banks_client.get_account(main_treasury_pda).await
        .map_err(|e| format!("Failed to get treasury account: {}", e))?;
    
    let current_treasury_state = if let Some(account) = current_treasury_account {
        MainTreasuryState::try_from_slice(&account.data)
            .map_err(|e| format!("Failed to deserialize treasury state: {}", e))?
    } else {
        return Err("Treasury account not found".to_string());
    };
    
    // Calculate changes
    let pool_creation_count_change = current_treasury_state.pool_creation_count - initial_treasury_state.pool_creation_count;
    let fees_collected = current_treasury_state.total_pool_creation_fees - initial_treasury_state.total_pool_creation_fees;
    let balance_change = current_treasury_state.total_balance - initial_treasury_state.total_balance;
    
    println!("📊 Fee collection verification:");
    println!("   - Pool creation count change: {}", pool_creation_count_change);
    println!("   - Pool creation fees collected: {} lamports", fees_collected);
    println!("   - Treasury balance change: {} lamports", balance_change);
    
    // Validate the changes make sense
    if pool_creation_count_change > 0 && fees_collected == 0 {
        return Err("Pool creation count increased but no fees were collected".to_string());
    }
    
    if fees_collected > 0 && balance_change <= 0 {
        return Err("Fees were collected but treasury balance did not increase".to_string());
    }
    
    println!("✅ Pool creation fee collection verified successfully");
    Ok(fees_collected)
} 

// ========================================
// PHASE 2.1: CONSOLIDATION HELPERS
// ========================================

/// **PHASE 2.1**: Result of a consolidation operation with comprehensive tracking
/// 
/// This structure provides detailed information about fee consolidation from pools
/// to the main treasury, including before/after states and operation metrics.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ConsolidationResult {
    /// Pool fee state before consolidation
    pub initial_pool_fees: PoolFeeState,
    /// Treasury state before consolidation
    pub initial_treasury_state: MainTreasuryState,
    /// Treasury state after consolidation
    pub post_consolidation_treasury_state: MainTreasuryState,
    /// Total SOL fees transferred from pool to treasury
    pub fees_transferred: u64,
    /// Number of liquidity operations consolidated from this pool
    pub liquidity_operations_consolidated: u32,
    /// Number of swap operations consolidated from this pool
    pub swap_operations_consolidated: u32,
    /// Whether the consolidation operation completed successfully
    pub consolidation_successful: bool,
    /// Error message if consolidation failed
    pub error_message: Option<String>,
    /// Timestamp when consolidation was performed
    pub consolidation_timestamp: i64,
}

/// **PHASE 2.1**: Result of multi-pool consolidation operations
/// 
/// This structure tracks the results of consolidating fees from multiple pools
/// in a single batch operation, providing aggregated metrics and individual results.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MultiConsolidationResult {
    /// Results from individual pool consolidations
    pub individual_results: Vec<ConsolidationResult>,
    /// Total fees transferred across all pools
    pub total_fees_transferred: u64,
    /// Total liquidity operations consolidated across all pools
    pub total_liquidity_operations: u32,
    /// Total swap operations consolidated across all pools
    pub total_swap_operations: u32,
    /// Number of pools successfully consolidated
    pub successful_consolidations: u32,
    /// Number of pools that failed consolidation
    pub failed_consolidations: u32,
    /// Overall success rate (successful / total attempted)
    pub success_rate: f64,
    /// Total time taken for batch consolidation
    pub batch_processing_time_ms: u64,
}

/// **PHASE 2.1**: Execute consolidation operation on a single pool
/// 
/// This function consolidates accumulated fees from a pool state to the main treasury.
/// It handles the transfer of SOL fees and updates both pool and treasury state tracking.
/// 
/// **INFRASTRUCTURE TESTING**: Uses mock data for predictable consolidation testing.
/// 
/// # Arguments
/// * `env` - Test environment with access to blockchain state
/// * `pool_pda` - Pool state PDA to consolidate fees from
/// 
/// # Returns
/// * `ConsolidationResult` - Detailed consolidation tracking data
/// 
/// # Test Criteria (Phase 2.1)
/// ✅ Can consolidate fees from pools that have accumulated fees (from Phase 1)
/// ✅ Can verify fees actually transfer from pool to treasury
#[allow(dead_code)]
pub async fn execute_consolidation_operation(
    env: &mut TestEnvironment, 
    pool_pda: &Pubkey
) -> Result<ConsolidationResult, Box<dyn std::error::Error>> {
    println!("🔄 PHASE 2.1: Executing consolidation operation...");
    println!("   • Pool: {}", pool_pda);
    
    // Get PDAs
    let (main_treasury_pda, _) = Pubkey::find_program_address(
        &[fixed_ratio_trading::MAIN_TREASURY_SEED_PREFIX],
        &fixed_ratio_trading::id(),
    );
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[fixed_ratio_trading::SYSTEM_STATE_SEED_PREFIX],
        &fixed_ratio_trading::id(),
    );
    
    // Get initial treasury state
    let initial_treasury_state = crate::common::treasury_helpers::get_treasury_state_verified().await?;
    
    // Get initial pool state
    let pool_account = env.banks_client.get_account(*pool_pda).await?.unwrap();
    let initial_pool_state: fixed_ratio_trading::PoolState = fixed_ratio_trading::PoolState::try_from_slice(&pool_account.data)?;
    
    let initial_pool_fees = initial_pool_state.pending_sol_fees();
    println!("   • Initial pool fees: {} lamports", initial_pool_fees);
    
    // If no fees, return early with appropriate result
    if initial_pool_fees == 0 {
        println!("   • No fees to consolidate, returning early");
        return Ok(ConsolidationResult {
            initial_pool_fees: PoolFeeState {
                pool_pda: *pool_pda,
                total_liquidity_fees: 0,
                liquidity_operation_count: 0,
                pool_balance_primary: 0,
                pool_balance_base: 0,
                timestamp: 0,
            },
            initial_treasury_state: initial_treasury_state.clone(),
            post_consolidation_treasury_state: initial_treasury_state,
            fees_transferred: 0,
            liquidity_operations_consolidated: 0,
            swap_operations_consolidated: 0,
            consolidation_successful: true, // Still successful, just no fees to consolidate
            error_message: None,
            consolidation_timestamp: 0,
        });
    }
    
    // **PHASE 2.1**: Execute actual consolidation instruction
    println!("💰 Executing actual consolidation instruction...");
    
    let consolidate_instruction = fixed_ratio_trading::PoolInstruction::ConsolidatePoolFees {
        pool_count: 1,
    };
    
    let accounts = vec![
        solana_sdk::instruction::AccountMeta::new(system_state_pda, false),
        solana_sdk::instruction::AccountMeta::new(main_treasury_pda, false),
        solana_sdk::instruction::AccountMeta::new(*pool_pda, false),
    ];
    
    let instruction = solana_sdk::instruction::Instruction {
        program_id: fixed_ratio_trading::id(),
        accounts,
        data: consolidate_instruction.try_to_vec()?,
    };
    
    let transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[instruction],
        Some(&env.payer.pubkey()),
        &[&env.payer],
        env.recent_blockhash,
    );
    
    // Execute the consolidation transaction
    match env.banks_client.process_transaction(transaction).await {
        Ok(_) => {
            println!("✅ Consolidation instruction executed successfully");
        }
        Err(e) => {
            println!("❌ Consolidation instruction failed: {}", e);
            return Ok(ConsolidationResult {
                initial_pool_fees: PoolFeeState {
                    pool_pda: *pool_pda,
                    total_liquidity_fees: initial_pool_fees,
                    liquidity_operation_count: 0,
                    pool_balance_primary: 0,
                    pool_balance_base: 0,
                    timestamp: 0,
                },
                initial_treasury_state: initial_treasury_state.clone(),
                post_consolidation_treasury_state: initial_treasury_state,
                fees_transferred: 0,
                liquidity_operations_consolidated: 0,
                swap_operations_consolidated: 0,
                consolidation_successful: false,
                error_message: Some(e.to_string()),
                consolidation_timestamp: 0,
            });
        }
    }
    
    // Get post-consolidation states
    let post_consolidation_treasury_state = crate::common::treasury_helpers::get_treasury_state_verified().await?;
    let pool_account_after = env.banks_client.get_account(*pool_pda).await?.unwrap();
    let post_consolidation_pool_state: fixed_ratio_trading::PoolState = fixed_ratio_trading::PoolState::try_from_slice(&pool_account_after.data)?;
    
    let final_pool_fees = post_consolidation_pool_state.pending_sol_fees();
    let fees_transferred = initial_pool_fees - final_pool_fees;
    
    // Calculate operation counts from fee differences
    let liquidity_ops_consolidated = if initial_pool_state.collected_liquidity_fees > post_consolidation_pool_state.collected_liquidity_fees {
        (initial_pool_state.collected_liquidity_fees - post_consolidation_pool_state.collected_liquidity_fees) / fixed_ratio_trading::DEPOSIT_WITHDRAWAL_FEE
    } else {
        0
    };
    
    let swap_ops_consolidated = if initial_pool_state.collected_swap_contract_fees > post_consolidation_pool_state.collected_swap_contract_fees {
        (initial_pool_state.collected_swap_contract_fees - post_consolidation_pool_state.collected_swap_contract_fees) / fixed_ratio_trading::SWAP_CONTRACT_FEE
    } else {
        0
    };
    
    println!("✅ PHASE 2.1: Consolidation completed successfully");
    println!("   • Fees transferred: {} lamports", fees_transferred);
    println!("   • Liquidity operations consolidated: {}", liquidity_ops_consolidated);
    println!("   • Swap operations consolidated: {}", swap_ops_consolidated);
    println!("   • Treasury balance updated: {} -> {} lamports", 
             initial_treasury_state.total_balance, 
             post_consolidation_treasury_state.total_balance);
    
    Ok(ConsolidationResult {
        initial_pool_fees: PoolFeeState {
            pool_pda: *pool_pda,
            total_liquidity_fees: initial_pool_state.collected_liquidity_fees,
            liquidity_operation_count: 0, // We don't track this in pool state
            pool_balance_primary: 0,
            pool_balance_base: 0,
            timestamp: 0,
        },
        initial_treasury_state,
        post_consolidation_treasury_state,
        fees_transferred,
        liquidity_operations_consolidated: liquidity_ops_consolidated as u32,
        swap_operations_consolidated: swap_ops_consolidated as u32,
        consolidation_successful: true,
        error_message: None,
        consolidation_timestamp: 0,
    })
}

/// **PHASE 2.1**: Execute consolidation operation with verification
/// 
/// This function performs consolidation with additional verification steps to ensure
/// the consolidation was successful and all state updates are correct.
/// 
/// **INFRASTRUCTURE TESTING**: Provides comprehensive verification with mock data.
/// 
/// # Arguments
/// * `env` - Test environment with access to blockchain state
/// * `pool_pda` - Pool state PDA to consolidate fees from
/// 
/// # Returns
/// * `ConsolidationResult` - Detailed consolidation tracking data with verification
/// 
/// # Test Criteria (Phase 2.1)
/// ✅ Can verify consolidation updates treasury liquidity_operation_count
/// ✅ Can verify consolidation updates treasury regular_swap_count
#[allow(dead_code)]
pub async fn execute_consolidation_with_verification(
    env: &mut TestEnvironment, 
    pool_pda: &Pubkey
) -> Result<ConsolidationResult, Box<dyn std::error::Error>> {
    println!("🔍 PHASE 2.1: Executing consolidation with comprehensive verification...");
    
    // Execute the base consolidation operation
    let mut result = execute_consolidation_operation(env, pool_pda).await?;
    
    // **PHASE 2.1**: Comprehensive verification steps
    println!("🔎 Verifying consolidation results...");
    
    // Verify treasury counter updates
    let liquidity_count_delta = result.post_consolidation_treasury_state.liquidity_operation_count - 
                               result.initial_treasury_state.liquidity_operation_count;
    let swap_count_delta = result.post_consolidation_treasury_state.regular_swap_count - 
                          result.initial_treasury_state.regular_swap_count;
    
    println!("📊 Counter verification:");
    println!("   • Liquidity operation count delta: {} (expected: {})", 
             liquidity_count_delta, result.liquidity_operations_consolidated);
    println!("   • Regular swap count delta: {} (expected: {})", 
             swap_count_delta, result.swap_operations_consolidated);
    
    // Verify treasury counter updates match expectations
    if liquidity_count_delta != result.liquidity_operations_consolidated as u64 {
        result.consolidation_successful = false;
        result.error_message = Some(format!(
            "Liquidity operation count mismatch: expected {}, got {}", 
            result.liquidity_operations_consolidated, liquidity_count_delta
        ));
        return Ok(result);
    }
    
    if swap_count_delta != result.swap_operations_consolidated as u64 {
        result.consolidation_successful = false;
        result.error_message = Some(format!(
            "Swap operation count mismatch: expected {}, got {}", 
            result.swap_operations_consolidated, swap_count_delta
        ));
        return Ok(result);
    }
    
    // Verify fee transfer amounts
    let balance_delta = result.post_consolidation_treasury_state.total_balance - 
                       result.initial_treasury_state.total_balance;
    
    println!("💰 Balance verification:");
    println!("   • Treasury balance delta: {} lamports (expected: {})", 
             balance_delta, result.fees_transferred);
    
    if balance_delta != result.fees_transferred {
        result.consolidation_successful = false;
        result.error_message = Some(format!(
            "Fee transfer amount mismatch: expected {}, got {}", 
            result.fees_transferred, balance_delta
        ));
        return Ok(result);
    }
    
    // Verify consolidation tracking incremented (only if fees were transferred)
    let consolidation_count_delta = result.post_consolidation_treasury_state.total_consolidations_performed - 
                                   result.initial_treasury_state.total_consolidations_performed;
    
    let expected_count_delta = if result.fees_transferred > 0 { 1 } else { 0 };
    if consolidation_count_delta != expected_count_delta {
        result.consolidation_successful = false;
        result.error_message = Some(format!(
            "Consolidation count should increment by {} (fees transferred: {}), got delta: {}", 
            expected_count_delta, result.fees_transferred, consolidation_count_delta
        ));
        return Ok(result);
    }
    
    println!("✅ PHASE 2.1: Consolidation verification completed successfully");
    println!("   • All counter updates verified correctly");
    println!("   • Fee transfer amounts verified correctly");
    println!("   • Consolidation tracking verified correctly");
    
    Ok(result)
}

/// **PHASE 2.1**: Consolidate fees from multiple pools in a batch operation
/// 
/// This function processes multiple pools in a single consolidation batch,
/// providing efficient bulk consolidation with comprehensive tracking.
/// 
/// **INFRASTRUCTURE TESTING**: Simulates batch consolidation with mock data.
/// 
/// # Arguments
/// * `env` - Test environment with access to blockchain state
/// * `pool_pdas` - Vector of pool state PDAs to consolidate fees from
/// 
/// # Returns
/// * `MultiConsolidationResult` - Aggregated results from batch consolidation
/// 
/// # Test Criteria (Phase 2.1)
/// ✅ Builds on proven Phase 1 operations
#[allow(dead_code)]
pub async fn consolidate_multiple_pools(
    env: &mut TestEnvironment, 
    pool_pdas: Vec<Pubkey>
) -> Result<MultiConsolidationResult, Box<dyn std::error::Error>> {
    println!("🔄 PHASE 2.1: Starting batch consolidation for {} pools...", pool_pdas.len());
    
    let batch_start_time = std::time::Instant::now();
    let mut individual_results = Vec::new();
    let mut total_fees_transferred = 0u64;
    let mut total_liquidity_operations = 0u32;
    let mut total_swap_operations = 0u32;
    let mut successful_consolidations = 0u32;
    let mut failed_consolidations = 0u32;
    
    // Process each pool in the batch
    for (i, pool_pda) in pool_pdas.iter().enumerate() {
        println!("🔄 Processing pool {}/{}: {}", i + 1, pool_pdas.len(), pool_pda);
        
        match execute_consolidation_with_verification(env, pool_pda).await {
            Ok(result) => {
                if result.consolidation_successful {
                    total_fees_transferred += result.fees_transferred;
                    total_liquidity_operations += result.liquidity_operations_consolidated;
                    total_swap_operations += result.swap_operations_consolidated;
                    successful_consolidations += 1;
                    println!("✅ Pool {} consolidated successfully", i + 1);
                } else {
                    failed_consolidations += 1;
                    println!("❌ Pool {} consolidation failed: {:?}", i + 1, result.error_message);
                }
                individual_results.push(result);
            }
            Err(e) => {
                println!("❌ Pool {} consolidation error: {}", i + 1, e);
                failed_consolidations += 1;
                
                // Create failed result for tracking
                let failed_result = ConsolidationResult {
                    initial_pool_fees: PoolFeeState {
                        pool_pda: *pool_pda,
                        total_liquidity_fees: 0,
                        liquidity_operation_count: 0,
                        pool_balance_primary: 0,
                        pool_balance_base: 0,
                        timestamp: 1640995200,
                    },
                    initial_treasury_state: MainTreasuryState::new(),
                    post_consolidation_treasury_state: MainTreasuryState::new(),
                    fees_transferred: 0,
                    liquidity_operations_consolidated: 0,
                    swap_operations_consolidated: 0,
                    consolidation_successful: false,
                    error_message: Some(e.to_string()),
                    consolidation_timestamp: 1640995200,
                };
                individual_results.push(failed_result);
            }
        }
    }
    
    let batch_processing_time = batch_start_time.elapsed().as_millis() as u64;
    let success_rate = if pool_pdas.is_empty() {
        1.0
    } else {
        successful_consolidations as f64 / pool_pdas.len() as f64
    };
    
    println!("📈 PHASE 2.1: Batch consolidation completed");
    println!("   • Total pools processed: {}", pool_pdas.len());
    println!("   • Successful consolidations: {}", successful_consolidations);
    println!("   • Failed consolidations: {}", failed_consolidations);
    println!("   • Success rate: {:.1}%", success_rate * 100.0);
    println!("   • Total fees transferred: {} lamports", total_fees_transferred);
    println!("   • Total liquidity operations: {}", total_liquidity_operations);
    println!("   • Total swap operations: {}", total_swap_operations);
    println!("   • Batch processing time: {} ms", batch_processing_time);
    
    Ok(MultiConsolidationResult {
        individual_results,
        total_fees_transferred,
        total_liquidity_operations,
        total_swap_operations,
        successful_consolidations,
        failed_consolidations,
        success_rate,
        batch_processing_time_ms: batch_processing_time,
    })
} 

// ========================================
// BASIS POINTS REFACTOR: DISPLAY UNIT HELPERS
// ========================================

/// **BASIS POINTS REFACTOR: Pool Configuration for Display Units**
/// 
/// Configuration for creating pools using display units, which will be converted
/// to basis points before sending to the smart contract.
#[derive(Debug)]
#[allow(dead_code)]
pub struct PoolConfigDisplayUnits {
    /// Multiple token (abundant token) display amount in the ratio
    pub multiple_ratio_display: f64,
    /// Base token (valuable token) display amount in the ratio  
    pub base_ratio_display: f64,
    /// Multiple token mint
    pub multiple_mint: Keypair,
    /// Base token mint
    pub base_mint: Keypair,
    /// Multiple token decimals (fetched from mint)
    pub multiple_decimals: u8,
    /// Base token decimals (fetched from mint)
    pub base_decimals: u8,
}

/// **BASIS POINTS REFACTOR: Create pool using display units**
/// 
/// This function takes display units (e.g., 1.0 SOL = 160.0 USDT) and converts
/// them to basis points before creating the pool. This is the recommended way
/// to create pools in tests for the basis points refactor.
/// 
/// # Arguments
/// * `banks` - Banks client for transaction processing
/// * `payer` - Account that pays for pool creation
/// * `recent_blockhash` - Recent blockhash for transaction
/// * `config` - Pool configuration with display units
/// 
/// # Returns
/// Normalized pool configuration with basis point ratios
/// 
/// # Example
/// ```
/// let config = PoolConfigDisplayUnits {
///     multiple_ratio_display: 1.0,    // 1.0 SOL
///     base_ratio_display: 160.0,      // = 160.0 USDT
///     multiple_mint: sol_mint_keypair,
///     base_mint: usdt_mint_keypair,
///     multiple_decimals: 9,            // SOL decimals
///     base_decimals: 6,                // USDT decimals
/// };
/// let pool_config = create_pool_with_display_units(&mut banks, &payer, recent_blockhash, config).await?;
/// ```
#[allow(dead_code)]
pub async fn create_pool_with_display_units(
    banks: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
    config: PoolConfigDisplayUnits,
) -> Result<PoolConfig, solana_program_test::BanksClientError> {
    // Convert display units to basis points
    let multiple_ratio_basis_points = display_to_basis_points(
        config.multiple_ratio_display, 
        config.multiple_decimals
    );
    let base_ratio_basis_points = display_to_basis_points(
        config.base_ratio_display, 
        config.base_decimals
    );
    
    println!("🔧 BASIS POINTS CONVERSION:");
    println!("  Multiple: {} (display) → {} (basis points)", 
        config.multiple_ratio_display, multiple_ratio_basis_points);
    println!("  Base: {} (display) → {} (basis points)", 
        config.base_ratio_display, base_ratio_basis_points);
    
    // Use the normalize_pool_config function with basis points
    let pool_config = normalize_pool_config(
        &config.multiple_mint.pubkey(),
        &config.base_mint.pubkey(),
        multiple_ratio_basis_points,
        base_ratio_basis_points,
    );
    
    // Create the pool using the same logic as create_pool_new_pattern but with our config
    create_pool_with_normalized_config(banks, payer, recent_blockhash, &config.multiple_mint, &config.base_mint, pool_config).await
}

/// Internal helper function to create a pool from a normalized config
async fn create_pool_with_normalized_config(
    banks: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
    multiple_mint: &Keypair,
    base_mint: &Keypair,
    config: PoolConfig,
) -> Result<PoolConfig, solana_program_test::BanksClientError> {
    // Check if pool already exists
    if let Some(_existing_pool) = get_pool_state(banks, &config.pool_state_pda).await {
        return Err(solana_program_test::BanksClientError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "Pool already exists with this configuration"
        )));
    }

    use solana_sdk::transaction::Transaction;
    use solana_sdk::instruction::{AccountMeta, Instruction};
    use fixed_ratio_trading::types::instructions::PoolInstruction;
    use borsh::BorshSerialize;

    // Derive required PDAs
    let (main_treasury_pda, _) = Pubkey::find_program_address(
        &[frt_constants::MAIN_TREASURY_SEED_PREFIX],
        &id(),
    );
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[frt_constants::SYSTEM_STATE_SEED_PREFIX],
        &id(),
    );
    let (lp_token_a_mint_pda, _) = Pubkey::find_program_address(
        &[frt_constants::LP_TOKEN_A_MINT_SEED_PREFIX, config.pool_state_pda.as_ref()],
        &id(),
    );
    let (lp_token_b_mint_pda, _) = Pubkey::find_program_address(
        &[frt_constants::LP_TOKEN_B_MINT_SEED_PREFIX, config.pool_state_pda.as_ref()],
        &id(),
    );

    // Create InitializePool instruction
    let initialize_pool_ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),                          // Index 0: User Authority Signer
            AccountMeta::new_readonly(solana_program::system_program::id(), false), // Index 1: System Program
            AccountMeta::new_readonly(system_state_pda, false),              // Index 2: System State PDA
            AccountMeta::new(config.pool_state_pda, false),                  // Index 3: Pool State PDA
            AccountMeta::new_readonly(spl_token::id(), false),               // Index 4: SPL Token Program
            AccountMeta::new(main_treasury_pda, false),                      // Index 5: Main Treasury PDA
            AccountMeta::new_readonly(solana_program::sysvar::rent::id(), false), // Index 6: Rent Sysvar
            AccountMeta::new_readonly(multiple_mint.pubkey(), false),        // Index 7: Token A Mint
            AccountMeta::new_readonly(base_mint.pubkey(), false),            // Index 8: Token B Mint
            AccountMeta::new(config.token_a_vault_pda, false),               // Index 9: Token A Vault PDA
            AccountMeta::new(config.token_b_vault_pda, false),               // Index 10: Token B Vault PDA
            AccountMeta::new(lp_token_a_mint_pda, false),                    // Index 11: LP Token A Mint PDA
            AccountMeta::new(lp_token_b_mint_pda, false),                    // Index 12: LP Token B Mint PDA
        ],
        data: PoolInstruction::InitializePool {
            ratio_a_numerator: config.ratio_a_numerator,
            ratio_b_denominator: config.ratio_b_denominator,
        }.try_to_vec().unwrap(),
    };

    // Add compute budget and send transaction
    use solana_sdk::compute_budget::ComputeBudgetInstruction;
    let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(500_000);
    
    let mut transaction = Transaction::new_with_payer(
        &[compute_budget_ix, initialize_pool_ix], 
        Some(&payer.pubkey())
    );
    transaction.sign(&[payer], recent_blockhash);
    banks.process_transaction(transaction).await?;

    Ok(config)
} 

/// **BASIS POINTS REFACTOR: Convenience function for common test scenarios**
/// 
/// Creates a pool with simple display unit ratios for common testing patterns.
/// 
/// # Arguments
/// * `banks` - Banks client
/// * `payer` - Payer keypair
/// * `recent_blockhash` - Recent blockhash
/// * `multiple_mint` - Multiple token mint keypair
/// * `base_mint` - Base token mint keypair
/// * `multiple_display` - Multiple token amount in display units
/// * `base_display` - Base token amount in display units
/// * `multiple_decimals` - Multiple token decimals
/// * `base_decimals` - Base token decimals
/// 
/// # Example
/// ```
/// // Create "1 SOL = 160 USDT" pool
/// let pool_config = create_simple_display_pool(
///     &mut banks, &payer, recent_blockhash,
///     &sol_mint, &usdt_mint,
///     1.0, 160.0,  // 1 SOL = 160 USDT
///     9, 6         // SOL=9 decimals, USDT=6 decimals
/// ).await?;
/// ```
#[allow(dead_code)]
pub async fn create_simple_display_pool(
    banks: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
    multiple_mint: &Keypair,
    base_mint: &Keypair,
    multiple_display: f64,
    base_display: f64,
    multiple_decimals: u8,
    base_decimals: u8,
) -> Result<PoolConfig, solana_program_test::BanksClientError> {
    let config = PoolConfigDisplayUnits {
        multiple_ratio_display: multiple_display,
        base_ratio_display: base_display,
        multiple_mint: Keypair::from_bytes(&multiple_mint.to_bytes()).unwrap(),
        base_mint: Keypair::from_bytes(&base_mint.to_bytes()).unwrap(),
        multiple_decimals,
        base_decimals,
    };
    
    create_pool_with_display_units(banks, payer, recent_blockhash, config).await
} 