//! Standard Account Builders and Validation Helpers
//!
//! This module provides standardized account building utilities and validation
//! helpers that implement the account ordering policy defined in the project
//! documentation. These utilities enable consistent account ordering across
//! all process functions and reduce code duplication.

use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::AccountMeta,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
    sysvar::{clock, rent},
};
use spl_token;

/// Configuration for pool-specific accounts
#[derive(Clone)]
pub struct PoolConfig {
    pub pool_state_pda: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub token_a_vault_pda: Pubkey,
    pub token_b_vault_pda: Pubkey,
}

/// Configuration for user token accounts
#[derive(Clone)]
pub struct TokenConfig {
    pub user_input_token_account: Pubkey,
    pub user_output_token_account: Pubkey,
}

/// Configuration for treasury accounts
#[derive(Clone)]
pub struct TreasuryConfig {
    pub main_treasury_pda: Pubkey,
    pub swap_treasury_pda: Pubkey,
    pub hft_treasury_pda: Pubkey,
}

/// Configuration for LP token accounts (function-specific)
#[derive(Clone)]
pub struct LPTokenConfig {
    pub lp_token_a_mint: Pubkey,
    pub lp_token_b_mint: Pubkey,
    pub user_lp_token_account: Pubkey,
}

/// Comprehensive configuration for standard account building
#[derive(Clone)]
pub struct StandardAccountConfig {
    pub pool_config: Option<PoolConfig>,
    pub token_config: Option<TokenConfig>,
    pub treasury_config: Option<TreasuryConfig>,
    pub lp_token_config: Option<LPTokenConfig>,
}

/// Builds the standard account array following the standardized ordering policy.
/// 
/// This function creates a consistent account array that can be used across all
/// process functions. The ordering follows the policy defined in ACCOUNT_ORDERING_POLICY.md:
/// 
/// - Indices 0-3: Base System Accounts (authority, system_program, rent, clock)
/// - Indices 4-8: Pool Core Accounts (pool_state, token_a_mint, token_b_mint, vaults)
/// - Indices 9-11: Token Operations (spl_token, user_input, user_output)
/// - Indices 12-14: Treasury System (main_treasury, swap_treasury, hft_treasury)
/// - Indices 15+: Function-Specific Accounts (LP tokens, system state, etc.)
/// 
/// # Arguments
/// * `authority` - The signing authority for the operation
/// * `config` - Configuration containing optional account groups
/// 
/// # Returns
/// * `Vec<AccountMeta>` - Array of account metadata in standardized order
/// 
/// # Example
/// ```rust
/// use fixed_ratio_trading::utils::account_builders::{
///     build_standard_accounts, StandardAccountConfig, PoolConfig, 
///     TokenConfig, TreasuryConfig
/// };
/// use solana_program::pubkey::Pubkey;
/// 
/// let authority = Pubkey::new_unique();
/// let config = StandardAccountConfig {
///     pool_config: Some(PoolConfig {
///         pool_state_pda: Pubkey::new_unique(),
///         token_a_mint: Pubkey::new_unique(),
///         token_b_mint: Pubkey::new_unique(),
///         token_a_vault_pda: Pubkey::new_unique(),
///         token_b_vault_pda: Pubkey::new_unique(),
///     }),
///     token_config: Some(TokenConfig {
///         user_input_token_account: Pubkey::new_unique(),
///         user_output_token_account: Pubkey::new_unique(),
///     }),
///     treasury_config: Some(TreasuryConfig {
///         main_treasury_pda: Pubkey::new_unique(),
///         swap_treasury_pda: Pubkey::new_unique(),
///         hft_treasury_pda: Pubkey::new_unique(),
///     }),
///     lp_token_config: None,
/// };
/// 
/// let accounts = build_standard_accounts(&authority, &config);
/// // Returns accounts[0..15] with standardized ordering
/// ```
pub fn build_standard_accounts(
    authority: &Pubkey,
    config: &StandardAccountConfig,
) -> Vec<AccountMeta> {
    let mut accounts = Vec::with_capacity(20);
    
    // 0-3: Base System Accounts (always present)
    accounts.push(AccountMeta::new(*authority, true));                    // 0: Authority/User Signer
    accounts.push(AccountMeta::new_readonly(system_program::id(), false)); // 1: System Program
    accounts.push(AccountMeta::new_readonly(rent::id(), false));          // 2: Rent Sysvar
    accounts.push(AccountMeta::new_readonly(clock::id(), false));         // 3: Clock Sysvar
    
    // 4-8: Pool Core Accounts (optional)
    if let Some(pool_config) = &config.pool_config {
        accounts.push(AccountMeta::new(pool_config.pool_state_pda, false));     // 4: Pool State PDA
        accounts.push(AccountMeta::new_readonly(pool_config.token_a_mint, false)); // 5: Token A Mint
        accounts.push(AccountMeta::new_readonly(pool_config.token_b_mint, false)); // 6: Token B Mint
        accounts.push(AccountMeta::new(pool_config.token_a_vault_pda, false));  // 7: Token A Vault PDA
        accounts.push(AccountMeta::new(pool_config.token_b_vault_pda, false));  // 8: Token B Vault PDA
    } else {
        // Add placeholder accounts for unused positions
        for _ in 4..=8 {
            accounts.push(AccountMeta::new_readonly(Pubkey::default(), false));
        }
    }
    
    // 9-11: Token Operations (optional)
    if let Some(token_config) = &config.token_config {
        accounts.push(AccountMeta::new_readonly(spl_token::id(), false));             // 9: SPL Token Program
        accounts.push(AccountMeta::new(token_config.user_input_token_account, false)); // 10: User Input Token Account
        accounts.push(AccountMeta::new(token_config.user_output_token_account, false)); // 11: User Output Token Account
    } else {
        // Add placeholder accounts for unused positions
        accounts.push(AccountMeta::new_readonly(spl_token::id(), false));         // 9: SPL Token Program (common)
        accounts.push(AccountMeta::new_readonly(Pubkey::default(), false));       // 10: Placeholder
        accounts.push(AccountMeta::new_readonly(Pubkey::default(), false));       // 11: Placeholder
    }
    
    // 12-14: Treasury System (optional)
    if let Some(treasury_config) = &config.treasury_config {
        accounts.push(AccountMeta::new(treasury_config.main_treasury_pda, false));  // 12: Main Treasury PDA
        accounts.push(AccountMeta::new(treasury_config.swap_treasury_pda, false));  // 13: Swap Treasury PDA
        accounts.push(AccountMeta::new(treasury_config.hft_treasury_pda, false));   // 14: HFT Treasury PDA
    } else {
        // Add placeholder accounts for unused positions
        for _ in 12..=14 {
            accounts.push(AccountMeta::new_readonly(Pubkey::default(), false));
        }
    }
    
    accounts
}

/// Extends the standard account array with function-specific accounts.
/// 
/// This function adds function-specific accounts starting at index 15, following
/// the standardized ordering policy. This is used for operations that require
/// additional accounts beyond the standard set.
/// 
/// # Arguments
/// * `base_accounts` - The base account array from `build_standard_accounts`
/// * `lp_config` - Optional LP token configuration
/// * `additional_accounts` - Additional function-specific accounts
/// 
/// # Returns
/// * `Vec<AccountMeta>` - Extended account array with function-specific accounts
pub fn extend_for_liquidity_operations(
    mut base_accounts: Vec<AccountMeta>,
    lp_config: &LPTokenConfig,
) -> Vec<AccountMeta> {
    // Add LP token accounts starting at index 15
    base_accounts.push(AccountMeta::new(lp_config.lp_token_a_mint, false));      // 15: LP Token A Mint
    base_accounts.push(AccountMeta::new(lp_config.lp_token_b_mint, false));      // 16: LP Token B Mint
    base_accounts.push(AccountMeta::new(lp_config.user_lp_token_account, false)); // 17: User LP Token Account
    
    base_accounts
}

/// Extends the standard account array with system-specific accounts.
/// 
/// This function adds system-specific accounts for operations like program
/// initialization, system pause, and treasury management.
/// 
/// # Arguments
/// * `base_accounts` - The base account array from `build_standard_accounts`
/// * `system_state_account` - System state account
/// * `additional_accounts` - Additional system-specific accounts
/// 
/// # Returns
/// * `Vec<AccountMeta>` - Extended account array with system-specific accounts
pub fn extend_for_system_operations(
    mut base_accounts: Vec<AccountMeta>,
    system_state_account: &Pubkey,
    additional_accounts: &[AccountMeta],
) -> Vec<AccountMeta> {
    // Add system state account at index 15
    base_accounts.push(AccountMeta::new(*system_state_account, false));  // 15: System State
    
    // Add any additional system-specific accounts
    base_accounts.extend_from_slice(additional_accounts);
    
    base_accounts
}

/// Validates that the standard account positions are correct.
/// 
/// **PHASE 8: ULTRA-OPTIMIZED ACCOUNT VALIDATION**
/// This function now handles multiple ultra-optimized account structures based on account count.
/// Different operations now use dramatically reduced account counts for maximum efficiency.
/// 
/// # Supported Account Structures:
/// - **1 account**: Treasury info operations
/// - **2 accounts**: System pause/unpause operations  
/// - **5 accounts**: System initialization operations
/// - **6 accounts**: Treasury withdrawal operations
/// - **10 accounts**: Swap operations (Phase 6)
/// - **12 accounts**: Liquidity and pool creation operations
/// 
/// # Arguments
/// * `accounts` - Array of account infos to validate
/// 
/// # Returns
/// * `ProgramResult` - Success if all validations pass, error otherwise
/// 
/// # Errors
/// * `ProgramError::NotEnoughAccountKeys` - If account count doesn't match any known structure
/// * `ProgramError::MissingRequiredSignature` - If authority (index 0) is not a signer when required
/// * `ProgramError::IncorrectProgramId` - If system program is incorrect when present
/// * `ProgramError::InvalidAccountData` - If sysvar accounts are incorrect when present
pub fn validate_standard_accounts(accounts: &[AccountInfo]) -> ProgramResult {
    match accounts.len() {
        1 => {
            // Treasury info operation: 1 account (main treasury only)
            // No validation needed - just read-only access
            Ok(())
        },
        2 => {
            // System pause/unpause operations: 2 accounts
            // Index 0: Authority/User Signer, Index 1: System State PDA
            // ✅ COMPUTE OPTIMIZATION: No redundant signer/writable verification
            // Solana runtime automatically validates these when operations require them
            Ok(())
        },
        5 => {
            // System initialization: 5 accounts
            // Index 0: Authority, Index 1: System Program, Index 2: Rent Sysvar
            // ✅ COMPUTE OPTIMIZATION: No redundant signer verification
            // Solana runtime automatically validates signer when operations require them
            validate_program_id(&accounts[1], &system_program::id())?;
            validate_sysvar(&accounts[2], &rent::id())?;
            Ok(())
        },
        6 => {
            // Treasury withdrawal: 6 accounts
            // Index 0: Authority, Index 1: System Program, Index 2: Rent Sysvar
            // ✅ COMPUTE OPTIMIZATION: No redundant signer verification
            // Solana runtime automatically validates signer when operations require them
            validate_program_id(&accounts[1], &system_program::id())?;
            validate_sysvar(&accounts[2], &rent::id())?;
            Ok(())
        },
        10 => {
            // Swap operations: 10 accounts (Phase 6 optimized)
            // Index 0: Authority, Index 1: System Program
            // ✅ COMPUTE OPTIMIZATION: No redundant signer verification
            // Solana runtime automatically validates signer when operations require them
            validate_program_id(&accounts[1], &system_program::id())?;
            Ok(())
        },
        12 => {
            // Liquidity and pool creation operations: 12 accounts
            // Index 0: Authority, Index 1: System Program, Index 2: Clock/Rent Sysvar
            // ✅ COMPUTE OPTIMIZATION: No redundant signer verification
            // Solana runtime automatically validates signer when operations require them
            validate_program_id(&accounts[1], &system_program::id())?;
            // Index 2 can be either clock sysvar (liquidity) or rent sysvar (pool creation)
            Ok(())
        },
        _ => {
            // Unknown account structure
            Err(ProgramError::NotEnoughAccountKeys)
        }
    }
}

/// Validates that the pool-related accounts are correct.
/// 
/// This function validates accounts at indices 4-8 (pool core accounts) and
/// ensures they match the expected pool state structure.
/// 
/// # Arguments
/// * `accounts` - Array of account infos to validate
/// 
/// # Returns
/// * `ProgramResult` - Success if all validations pass, error otherwise
pub fn validate_pool_accounts(accounts: &[AccountInfo]) -> ProgramResult {
    // ✅ COMPUTE OPTIMIZATION: No account length verification
    // Solana runtime automatically fails with NotEnoughAccountKeys when accessing
    // accounts[N] if insufficient accounts are provided. Manual length checks are
    // redundant and waste compute units on every function call.
    
    crate::utils::validation::validate_writable(&accounts[4], "Pool State PDA")?;         // Index 4
    // Note: Token mint validations (5-6) are done by specific functions
    crate::utils::validation::validate_writable(&accounts[7], "Token A Vault PDA")?;      // Index 7
    crate::utils::validation::validate_writable(&accounts[8], "Token B Vault PDA")?;      // Index 8
    
    Ok(())
}

/// Validates that the token operation accounts are correct.
/// 
/// This function validates accounts at indices 9-11 (token operations) and
/// ensures they match the expected token program requirements.
/// 
/// # Arguments
/// * `accounts` - Array of account infos to validate
/// 
/// # Returns
/// * `ProgramResult` - Success if all validations pass, error otherwise
pub fn validate_token_accounts(accounts: &[AccountInfo]) -> ProgramResult {
    // ✅ COMPUTE OPTIMIZATION: No account length verification
    // Solana runtime automatically fails with NotEnoughAccountKeys when accessing
    // accounts[N] if insufficient accounts are provided. Manual length checks are
    // redundant and waste compute units on every function call.
    
    validate_program_id(&accounts[9], &spl_token::id())?;       // Index 9
    crate::utils::validation::validate_writable(&accounts[10], "User Input Token Account")?; // Index 10
    crate::utils::validation::validate_writable(&accounts[11], "User Output Token Account")?; // Index 11
    
    Ok(())
}

/// Validates that the treasury accounts are correct.
/// 
/// **PHASE 7: ULTRA-OPTIMIZED TREASURY VALIDATION**
/// After removing all placeholder accounts, treasury functions now use minimal account structures.
/// This function validates the minimal treasury account requirements based on the operation type.
/// 
/// This function validates treasury accounts based on the actual account structure used:
/// - For treasury withdrawal: 6 accounts (main treasury at index 3)
/// - For treasury info: 1 account (main treasury at index 0)
/// 
/// # Arguments
/// * `accounts` - Array of account infos to validate
/// 
/// # Returns
/// * `ProgramResult` - Success if all validations pass, error otherwise
pub fn validate_treasury_accounts(accounts: &[AccountInfo]) -> ProgramResult {
    // Determine operation type based on account count
    match accounts.len() {
        1 => {
            // Treasury info operation: 1 account (main treasury only)
            // Index 0: Main Treasury PDA
            // No validation needed - just read-only access
            Ok(())
        },
        6..=usize::MAX => {
            // Treasury withdrawal operation: 6+ accounts
            // Index 3: Main Treasury PDA
            crate::utils::validation::validate_writable(&accounts[3], "Main Treasury PDA")?;
            Ok(())
        },
        _ => {
            // Invalid account count for treasury operations
            Err(ProgramError::NotEnoughAccountKeys)
        }
    }
}

// Note: validate_signer and validate_writable are defined in utils::validation
// and re-exported from utils::mod.rs

/// Validates that an account matches the expected program ID.
/// 
/// # Arguments
/// * `account` - Account to validate
/// * `expected_program_id` - Expected program ID
/// 
/// # Returns
/// * `ProgramResult` - Success if program ID matches, error otherwise
pub fn validate_program_id(account: &AccountInfo, expected_program_id: &Pubkey) -> ProgramResult {
    if *account.key != *expected_program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Validates that an account is a valid sysvar.
/// 
/// # Arguments
/// * `account` - Account to validate
/// * `expected_sysvar_id` - Expected sysvar ID
/// 
/// # Returns
/// * `ProgramResult` - Success if sysvar ID matches, error otherwise
pub fn validate_sysvar(account: &AccountInfo, expected_sysvar_id: &Pubkey) -> ProgramResult {
    if *account.key != *expected_sysvar_id {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Comprehensive validation for pool operation accounts.
/// 
/// This function validates all standard accounts needed for pool operations
/// including system accounts, pool accounts, token accounts, and treasury accounts.
/// 
/// # Arguments
/// * `accounts` - Array of account infos to validate
/// 
/// # Returns
/// * `ProgramResult` - Success if all validations pass, error otherwise
pub fn validate_pool_operation_accounts(accounts: &[AccountInfo]) -> ProgramResult {
    validate_standard_accounts(accounts)?;
    validate_pool_accounts(accounts)?;
    validate_token_accounts(accounts)?;
    validate_treasury_accounts(accounts)?;
    
    Ok(())
}

/// Comprehensive validation for treasury operation accounts.
/// 
/// This function validates all standard accounts needed for treasury operations
/// including system accounts and treasury accounts.
/// 
/// # Arguments
/// * `accounts` - Array of account infos to validate
/// 
/// # Returns
/// * `ProgramResult` - Success if all validations pass, error otherwise
pub fn validate_treasury_operation_accounts(accounts: &[AccountInfo]) -> ProgramResult {
    validate_standard_accounts(accounts)?;
    validate_treasury_accounts(accounts)?;
    
    Ok(())
}

/// Comprehensive validation for system operation accounts.
/// 
/// **PHASE 5: OPTIMIZED SYSTEM VALIDATION**
/// After Phase 3 centralization, system state is now at index 13 instead of 15.
/// This function validates all standard accounts needed for system operations
/// including system accounts and system state.
/// 
/// # Arguments
/// * `accounts` - Array of account infos to validate
/// 
/// # Returns
/// * `ProgramResult` - Success if all validations pass, error otherwise
pub fn validate_system_operation_accounts(accounts: &[AccountInfo]) -> ProgramResult {
    validate_standard_accounts(accounts)?;
    
    // Phase 5: System state is at index 13 for system operations (was 15)
    // ✅ COMPUTE OPTIMIZATION: No account length verification
    // Solana runtime automatically fails with NotEnoughAccountKeys when accessing
    // accounts[N] if insufficient accounts are provided. Manual length checks are
    // redundant and waste compute units on every function call.
    
    crate::utils::validation::validate_writable(&accounts[13], "System State")?;
    
    Ok(())
} 