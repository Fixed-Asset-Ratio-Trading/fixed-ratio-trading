//! Utility Processors
//! 
//! This module contains utility processors for helper functions, view operations,
//! PDA derivation, and debugging/testing support functions.

use crate::constants::*;

use crate::PoolState;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    account_info::next_account_info,
};
use borsh::BorshDeserialize;
use crate::error::PoolError;

// ================================================================================================
// PDA HELPER UTILITIES
// ================================================================================================

/// **PDA HELPER**: Returns the Pool State PDA for given token mints and ratio.
/// 
/// This utility function computes the Program Derived Address (PDA) for a pool
/// without requiring any account setup. It's useful for address derivation in
/// client applications and testing scenarios.
/// 
/// # Enhanced Normalization Logic
/// This function implements the same token normalization and ratio mapping logic
/// used during pool creation to ensure consistent PDA derivation. It prevents
/// creation of economically equivalent pools by normalizing token pairs to a
/// canonical form.
/// 
/// # Arguments
/// * `program_id` - The program ID of the Fixed Ratio Trading Pool program
/// * `multiple_token_mint` - The abundant token mint address
/// * `base_token_mint` - The valuable token mint address
/// * `multiple_per_base` - Exchange ratio between tokens
/// 
/// # Returns
/// * `ProgramResult` - Success (logs the PDA) or error
/// 
/// # Logging Output
/// This function logs the following information for client consumption:
/// - Final Pool State PDA address
/// - PDA bump seed for signing operations  
/// - Normalized token A and token B addresses (lexicographic order)
/// - Normalized ratio numerator and denominator
/// 
/// # Note
/// The logged PDA can be used by clients to derive the correct pool address
/// for subsequent operations like deposits, withdrawals, and swaps.
pub fn get_pool_state_pda(
    program_id: &Pubkey,
    multiple_token_mint: Pubkey,
    base_token_mint: Pubkey,
    multiple_per_base: u64,
) -> ProgramResult {
    msg!("DEBUG: get_pool_state_pda: Computing Pool State PDA");
    
    // Enhanced normalization to prevent economic duplicates (same logic as pool creation)
    // Step 1: Lexicographic token ordering
    let (token_a_mint_key, token_b_mint_key) = 
        if multiple_token_mint < base_token_mint {
            (multiple_token_mint, base_token_mint)
        } else {
            (base_token_mint, multiple_token_mint)
        };
    
    // Step 2: Canonical ratio mapping to prevent liquidity fragmentation
    let (ratio_a_numerator, ratio_b_denominator): (u64, u64) = 
        if multiple_token_mint < base_token_mint {
            // Tokens are in normal order: multiple = token_a, base = token_b
            (multiple_per_base, 1u64)
        } else {
            // Tokens are swapped: multiple = token_b, base = token_a
            // So ratio needs to be inverted: if multiple/base was N:1, then token_a/token_b is 1:N
            (1u64, multiple_per_base)
        };
    
    // Find PDA with canonical bump seed
    let (pool_state_pda, bump_seed) = Pubkey::find_program_address(
        &[
            POOL_STATE_SEED_PREFIX,
            token_a_mint_key.as_ref(),
            token_b_mint_key.as_ref(),
            &ratio_a_numerator.to_le_bytes(),
            &ratio_b_denominator.to_le_bytes(),
        ],
        program_id,
    );
    
    msg!("Pool State PDA: {}", pool_state_pda);
    msg!("Pool State PDA Bump Seed: {}", bump_seed);
    msg!("Normalized Token A: {}", token_a_mint_key);
    msg!("Normalized Token B: {}", token_b_mint_key);
    msg!("Normalized Ratio A: {}", ratio_a_numerator);
    msg!("Normalized Ratio B: {}", ratio_b_denominator);
    
    Ok(())
}

/// **UTILITY**: Derive the unique Pool ID (Pool State PDA) for given parameters.
/// 
/// This function allows clients to calculate the unique pool identifier without
/// creating the pool. The Pool ID is deterministically derived from the normalized
/// pool parameters, ensuring consistency across all operations.
/// 
/// # Arguments
/// * `program_id` - The program ID
/// * `token_mint_1` - First token mint (will be normalized to lexicographic order)
/// * `token_mint_2` - Second token mint (will be normalized to lexicographic order)
/// * `ratio_a_numerator` - Token A base units
/// * `ratio_b_denominator` - Token B base units
/// 
/// # Returns
/// * `(Pubkey, u8)` - The Pool ID (PDA) and its bump seed
/// 
/// # Example
/// ```rust
/// use fixed_ratio_trading::processors::utilities::derive_pool_id;
/// use solana_program::pubkey::Pubkey;
/// 
/// let program_id = Pubkey::new_unique();
/// let token_mint_1 = Pubkey::new_unique();
/// let token_mint_2 = Pubkey::new_unique();
/// 
/// let (pool_id, _bump) = derive_pool_id(
///     &program_id,
///     &token_mint_1,
///     &token_mint_2,
///     1000,  // ratio_a_numerator
///     1,     // ratio_b_denominator
/// );
/// println!("Pool ID: {}", pool_id);
/// ```
pub fn derive_pool_id(
    program_id: &Pubkey,
    token_mint_1: &Pubkey,
    token_mint_2: &Pubkey,
    ratio_a_numerator: u64,
    ratio_b_denominator: u64,
) -> (Pubkey, u8) {
    // Normalize tokens to lexicographic order (same as in process_initialize_pool)
    let (token_a_mint_key, token_b_mint_key) = 
        if token_mint_1 < token_mint_2 {
            (token_mint_1, token_mint_2)
        } else {
            (token_mint_2, token_mint_1)
        };

    // Derive the Pool State PDA (which serves as the unique Pool ID)
    Pubkey::find_program_address(
        &[
            POOL_STATE_SEED_PREFIX,
            token_a_mint_key.as_ref(),
            token_b_mint_key.as_ref(),
            &ratio_a_numerator.to_le_bytes(),
            &ratio_b_denominator.to_le_bytes(),
        ],
        program_id,
    )
}

/// **PDA HELPER**: Computes and returns Token Vault PDA addresses for a given pool.
/// 
/// This utility helps clients derive the token vault addresses for pool operations.
/// Useful for preparing deposit, withdraw, and swap transaction account lists.
/// 
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `pool_state_pda` - The Pool State PDA address
/// 
/// # Returns
/// * `ProgramResult` - Logs the derived vault PDA addresses and bump seeds
pub fn get_token_vault_pdas(
    program_id: &Pubkey,
    pool_state_pda: Pubkey,
) -> ProgramResult {
    msg!("DEBUG: get_token_vault_pdas: Computing Token Vault PDAs for pool: {}", pool_state_pda);
    
    // Find Token A Vault PDA
    let (token_a_vault_pda, token_a_bump) = Pubkey::find_program_address(
        &[
            TOKEN_A_VAULT_SEED_PREFIX,
            pool_state_pda.as_ref(),
        ],
        program_id,
    );
    
    // Find Token B Vault PDA
    let (token_b_vault_pda, token_b_bump) = Pubkey::find_program_address(
        &[
            TOKEN_B_VAULT_SEED_PREFIX,
            pool_state_pda.as_ref(),
        ],
        program_id,
    );
    
    msg!("Token A Vault PDA: {}", token_a_vault_pda);
    msg!("Token A Vault Bump Seed: {}", token_a_bump);
    msg!("Token B Vault PDA: {}", token_b_vault_pda);
    msg!("Token B Vault Bump Seed: {}", token_b_bump);
    
    Ok(())
}

// ================================================================================================
// TEST-SPECIFIC VIEW/GETTER INSTRUCTIONS
// ================================================================================================

/// **VIEW INSTRUCTION**: Returns comprehensive pool information
/// 
/// # Purpose
/// Logs structured pool information for debugging, testing, and frontend integration.
/// Outputs all critical pool state data in a human-readable format.
/// 
/// **⚠️ RACE CONDITION NOTICE**: Pool status reflects real-time state.
/// Temporary pause during large withdrawals (≥5% threshold) is expected behavior.
/// 
/// # Account Layout (Read-Only)
/// 0. **System Authority Signer** (readable) - Placeholder account (not used in pool info)
/// 1. **System Program Account** (readable) - Placeholder account (not used in pool info)
/// 2. **Pool State PDA** (read-only) - Pool state PDA for info query
/// 3. **SPL Token Program Account** (readable) - Placeholder account (not used in pool info)
/// 
/// # Returns
/// * `ProgramResult` - Logs comprehensive pool information
pub fn get_pool_info(accounts: &[AccountInfo]) -> ProgramResult {
    msg!("DEBUG: get_pool_info: Retrieving comprehensive pool information");
    
    // ✅ READ-ONLY OPERATION: This operation can continue during system pause
    // Read-only operations provide essential transparency during emergency situations
    
    // ✅ COMPUTE OPTIMIZATION: No account length verification
    // Solana runtime automatically fails with NotEnoughAccountKeys when accessing
    // accounts[N] if insufficient accounts are provided. Manual length checks are
    // redundant and waste compute units on every function call.
    
    let _system_authority_signer = &accounts[0];             // Index 0: System Authority Signer (placeholder)
    let _system_program_account = &accounts[1];              // Index 1: System Program Account (placeholder)
    let pool_state_account = &accounts[2];                   // Index 2: Pool State PDA
    let _spl_token_program_account = &accounts[3];           // Index 3: SPL Token Program Account (placeholder)
    // Note: Read-only operations still use secure validation for security consistency
    let pool_state = crate::utils::validation::validate_and_deserialize_pool_state_secure(pool_state_account, &crate::id())?;
    
    msg!("=== POOL STATE INFORMATION ===");
    msg!("Pool Owner: {}", pool_state.owner);
    msg!("Pool State PDA: {}", pool_state_account.key);
    msg!("Token A Mint: {}", pool_state.token_a_mint);
    msg!("Token B Mint: {}", pool_state.token_b_mint);
    msg!("Token A Vault: {}", pool_state.token_a_vault);
    msg!("Token B Vault: {}", pool_state.token_b_vault);
    msg!("LP Token A Mint: {}", pool_state.lp_token_a_mint);
    msg!("LP Token B Mint: {}", pool_state.lp_token_b_mint);
    msg!("Ratio A Numerator: {}", pool_state.ratio_a_numerator);
    msg!("Ratio B Denominator: {}", pool_state.ratio_b_denominator);
    msg!("Pool Authority Bump Seed: {}", pool_state.pool_authority_bump_seed);
    msg!("Token A Vault Bump Seed: {}", pool_state.token_a_vault_bump_seed);
    msg!("Token B Vault Bump Seed: {}", pool_state.token_b_vault_bump_seed);
    msg!("Liquidity Paused: {}", pool_state.liquidity_paused());
    msg!("Swaps Paused: {}", pool_state.swaps_paused());
    
    // Enhanced operations status
    msg!("=== OPERATIONS STATUS ===");
    msg!("Deposits: ENABLED");
    msg!("Withdrawals: ENABLED");
    
    if pool_state.swaps_paused() {
        msg!("Swaps: PAUSED (Owner Action)");
        msg!("  - Requires manual unpause by owner");
        msg!("  - Controlled by pool owner");
    } else {
        msg!("Swaps: ENABLED");
    }
    
    msg!("===============================");
    
    Ok(())
}

/// **VIEW INSTRUCTION**: Returns current pool pause status - publicly accessible
/// 
/// # Purpose
/// Provides public visibility into pool operation status and distinguishes between
/// system-wide pause and pool-specific swap pause for user transparency.
/// 
/// # Account Layout (Read-Only)
/// 0. Pool State PDA (read-only)
/// 
/// # Returns
/// * `ProgramResult` - Logs comprehensive pause status information
pub fn get_pool_pause_status(accounts: &[AccountInfo]) -> ProgramResult {
    // ✅ READ-ONLY OPERATION: This operation can continue during system pause
    // Users need transparency about pause status especially during system pause
    
    let pool_state_account = &accounts[0];
    let pool_state_data = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    
    // Log comprehensive pause status for public visibility
    msg!("=== POOL STATUS ===");
    msg!("Swaps: {}", if pool_state_data.swaps_paused() { "PAUSED" } else { "ENABLED" });
    msg!("Deposits: ENABLED");  // Always enabled (only system pause affects)
    msg!("Withdrawals: ENABLED"); // Always enabled (only system pause affects)
    
    if pool_state_data.swaps_paused() {
        msg!("=== OWNER PAUSE ===");
        msg!("Swaps paused by owner action");
        msg!("Control: Pool owner");
        msg!("Note: No auto-unpause - requires manual unpause action");
    }
    
    msg!("==================");
    
    Ok(())
}

/// **VIEW INSTRUCTION**: Returns detailed liquidity information for both tokens.
/// 
/// This function provides easy access to liquidity data, useful for calculating
/// exchange rates, available liquidity, and pool utilization metrics.
/// 
/// # Arguments
/// * `accounts` - Must contain pool state account as first account
/// 
/// # Returns
/// * `ProgramResult` - Logs detailed liquidity information
pub fn get_liquidity_info(accounts: &[AccountInfo]) -> ProgramResult {
    msg!("DEBUG: get_liquidity_info: Retrieving liquidity information");
    
    let pool_state_account = &accounts[0];
    // Note: Read-only operations still use secure validation for security consistency
    let pool_state = crate::utils::validation::validate_and_deserialize_pool_state_secure(pool_state_account, &crate::id())?;
    
    msg!("=== LIQUIDITY INFORMATION ===");
    msg!("Total Token A Liquidity: {}", pool_state.total_token_a_liquidity);
    msg!("Total Token B Liquidity: {}", pool_state.total_token_b_liquidity);
    msg!("Exchange Rate (A per B): {}", 
         if pool_state.ratio_b_denominator != 0 { 
             pool_state.ratio_a_numerator as f64 / pool_state.ratio_b_denominator as f64 
         } else { 0.0 });
    msg!("Exchange Rate (B per A): {}", 
         if pool_state.ratio_a_numerator != 0 { 
             pool_state.ratio_b_denominator as f64 / pool_state.ratio_a_numerator as f64 
         } else { 0.0 });
    
    // Calculate utilization if available
    let total_value_locked = pool_state.total_token_a_liquidity + pool_state.total_token_b_liquidity;
    msg!("Total Value Locked (TVL): {} tokens", total_value_locked);
    msg!("==============================");
    
    Ok(())
}

/// **VIEW INSTRUCTION**: Returns fee information including collected fees and rates.
/// 
/// This function provides comprehensive fee information essential for fee tracking,
/// transparency, and financial reporting. Shows both tracked fee amounts and 
/// actual account balances for complete transparency.
/// 
/// # Arguments
/// * `accounts` - Must contain pool state account as first account
/// 
/// # Returns
/// * `ProgramResult` - Logs detailed fee information
pub fn get_fee_info(accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_state_account = next_account_info(account_info_iter)?;

    // Note: Read-only operations still use secure validation for security consistency
    let pool_state = crate::utils::validation::validate_and_deserialize_pool_state_secure(pool_state_account, &crate::id())?;

    msg!("=== FEE INFORMATION ===");
    
    // Pool fees (percentage-based on tokens)
    msg!("Pool Fees (Trading Fees):");
    msg!("  Collected Token A Fees: {}", pool_state.collected_fees_token_a);
    msg!("  Collected Token B Fees: {} ({} tokens)",
         pool_state.collected_fees_token_b,
         pool_state.collected_fees_token_b as f64 / 1_000_000.0);
    msg!("   Withdrawn Token A Fees: {} ({} tokens)",
         pool_state.total_fees_withdrawn_token_a,
         pool_state.total_fees_withdrawn_token_a as f64 / 1_000_000.0);
    msg!("   Withdrawn Token B Fees: {} ({} tokens)",
         pool_state.total_fees_withdrawn_token_b,
         pool_state.total_fees_withdrawn_token_b as f64 / 1_000_000.0);
    msg!("📊 SOL FEES (MOVED TO CENTRAL TREASURY):");
    msg!("   ⚠️  SOL fees are now tracked centrally in TreasuryState");
    msg!("   ⚠️  Use GetTreasuryInfo instruction for SOL fee data");
    msg!("   ⚠️  Per-pool SOL fee tracking no longer available");
    
    // Actual pool state PDA balance
    let current_pool_balance = pool_state_account.lamports();
    msg!("Pool State PDA Balance:");
    msg!("  Current SOL Balance: {} lamports ({:.6} SOL)", 
         current_pool_balance,
         current_pool_balance as f64 / 1_000_000_000.0);
    
    // Calculate available fees for withdrawal (balance minus rent-exempt minimum)
    // Note: This is an approximation since we don't have rent sysvar here
    let estimated_rent_minimum = 2_500_000; // Conservative estimate for pool state account
    let estimated_available_fees = if current_pool_balance > estimated_rent_minimum {
        current_pool_balance - estimated_rent_minimum
    } else {
        0
    };
    
    msg!("  Estimated Available for Withdrawal: {} lamports ({:.6} SOL)", 
         estimated_available_fees,
         estimated_available_fees as f64 / 1_000_000_000.0);
    msg!("  (Note: Exact amount calculated during withdrawal with current rent rates)");
    
    msg!("=======================");

    Ok(())
}

/// **VIEW INSTRUCTION**: Returns the actual SOL balance of the pool state PDA.
/// 
/// This function provides direct access to the pool state account's SOL balance,
/// allowing users to see exactly how much SOL is held by the pool.
/// 
/// # Arguments
/// * `accounts` - Must contain pool state account as first account
/// 
/// # Returns
/// * `ProgramResult` - Logs pool state PDA SOL balance information
pub fn get_pool_sol_balance(accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_state_account = next_account_info(account_info_iter)?;

    let current_balance = pool_state_account.lamports();
    let estimated_rent_minimum = 2_500_000; // Conservative estimate
    let estimated_available = if current_balance > estimated_rent_minimum {
        current_balance - estimated_rent_minimum
    } else {
        0
    };

    msg!("=== POOL SOL BALANCE ===");
    msg!("Pool State PDA: {}", pool_state_account.key);
    msg!("Current SOL Balance: {} lamports", current_balance);
    msg!("Current SOL Balance: {:.6} SOL", current_balance as f64 / 1_000_000_000.0);
    msg!("Estimated Rent-Exempt Minimum: {} lamports", estimated_rent_minimum);
    msg!("Estimated Available for Withdrawal: {} lamports", estimated_available);
    msg!("Estimated Available for Withdrawal: {:.6} SOL", estimated_available as f64 / 1_000_000_000.0);
    msg!("Note: Use WithdrawFees instruction for exact calculations with current rent rates");
    msg!("========================");

    Ok(())
}

/// Validates that an account is a signer.
pub fn validate_signer(account: &AccountInfo, context: &str) -> ProgramResult {
    if !account.is_signer {
        msg!("{} must be a signer", context);
        return Err(ProgramError::MissingRequiredSignature);
    }
    Ok(())
}

/// Validates that an account is writable.
pub fn validate_writable(account: &AccountInfo, context: &str) -> ProgramResult {
    if !account.is_writable {
        msg!("{} must be writable", context);
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Validates that an amount is non-zero.
pub fn validate_non_zero_amount(amount: u64, context: &str) -> ProgramResult {
    if amount == 0 {
        msg!("{} amount cannot be zero", context);
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// validate_pool_initialized is no longer needed as we now use the pool state PDA to check if the pool is initialized.
/// **PHASE 1 UPDATE**: Pool existence = initialization status

/// Validates that liquidity operations are not paused.
pub fn validate_liquidity_not_paused(pool_state: &PoolState) -> ProgramResult {
    if pool_state.liquidity_paused() {
        msg!("Liquidity operations (deposits/withdrawals) are currently paused by owner");
        msg!("Note: Swaps may still be available");
        msg!("Note: Owner can manage pause governance and reasons");
        return Err(PoolError::PoolPaused.into());
    }
    Ok(())
}

/// **VIEW INSTRUCTION**: Returns smart contract version information.
/// 
/// This function provides version information for the smart contract including
/// the main contract version from Cargo.toml and the schema version for data structures.
/// 
/// # Purpose
/// - Frontend/client version compatibility checking
/// - Deployment verification and audit trails
/// - API compatibility detection
/// - Development and debugging support
/// 
/// # Returns
/// * `ProgramResult` - Logs comprehensive version information
pub fn process_get_version() -> ProgramResult {
    msg!("=== SMART CONTRACT VERSION ===");
    msg!("Contract Name: {}", env!("CARGO_PKG_NAME"));
    msg!("Contract Version: {}", env!("CARGO_PKG_VERSION"));
    msg!("Contract Description: {}", env!("CARGO_PKG_DESCRIPTION"));
    msg!("Schema Version: v2"); // From POOL_STATE_SEED_PREFIX
    msg!("Solana Program: Yes");
    msg!("License: {}", env!("CARGO_PKG_LICENSE"));
    msg!("Program ID: 4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn");
    msg!("===============================");
    
    Ok(())
} 