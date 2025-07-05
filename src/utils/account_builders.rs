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
/// let config = StandardAccountConfig {
///     pool_config: Some(PoolConfig { /* ... */ }),
///     token_config: Some(TokenConfig { /* ... */ }),
///     treasury_config: Some(TreasuryConfig { /* ... */ }),
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
/// This function performs validation on the standard account positions (0-14)
/// to ensure they match the expected types and requirements. It's used as a
/// common validation step across all process functions.
/// 
/// # Arguments
/// * `accounts` - Array of account infos to validate
/// 
/// # Returns
/// * `ProgramResult` - Success if all validations pass, error otherwise
/// 
/// # Errors
/// * `ProgramError::NotEnoughAccountKeys` - If fewer than 15 accounts provided
/// * `ProgramError::MissingRequiredSignature` - If authority (index 0) is not a signer
/// * `ProgramError::IncorrectProgramId` - If system program (index 1) is incorrect
/// * `ProgramError::InvalidAccountData` - If sysvar accounts are incorrect
pub fn validate_standard_accounts(accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() < 15 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // Validate standard account positions
    crate::utils::validation::validate_signer(&accounts[0], "Authority/User")?;           // Index 0
    validate_program_id(&accounts[1], &system_program::id())?;  // Index 1
    validate_sysvar(&accounts[2], &rent::id())?;                // Index 2
    validate_sysvar(&accounts[3], &clock::id())?;               // Index 3
    
    // Note: Pool accounts (4-8) are validated by specific functions
    // Note: Token program (9) is validated by specific functions
    // Note: Treasury accounts (12-14) are validated by specific functions
    
    Ok(())
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
    if accounts.len() < 9 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
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
    if accounts.len() < 12 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    validate_program_id(&accounts[9], &spl_token::id())?;       // Index 9
    crate::utils::validation::validate_writable(&accounts[10], "User Input Token Account")?; // Index 10
    crate::utils::validation::validate_writable(&accounts[11], "User Output Token Account")?; // Index 11
    
    Ok(())
}

/// Validates that the treasury accounts are correct.
/// 
/// This function validates accounts at indices 12-14 (treasury system) and
/// ensures they match the expected treasury structure.
/// 
/// # Arguments
/// * `accounts` - Array of account infos to validate
/// 
/// # Returns
/// * `ProgramResult` - Success if all validations pass, error otherwise
pub fn validate_treasury_accounts(accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() < 15 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    crate::utils::validation::validate_writable(&accounts[12], "Main Treasury PDA")?;     // Index 12
    crate::utils::validation::validate_writable(&accounts[13], "Swap Treasury PDA")?;     // Index 13
    crate::utils::validation::validate_writable(&accounts[14], "HFT Treasury PDA")?;      // Index 14
    
    Ok(())
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
    
    // System state is at index 15 for system operations
    if accounts.len() < 16 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    crate::utils::validation::validate_writable(&accounts[15], "System State")?;
    
    Ok(())
} 