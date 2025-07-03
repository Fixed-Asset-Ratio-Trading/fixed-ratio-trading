//! Pool Creation Processors
//! 
//! This module contains all the processors for pool creation and initialization operations.
//! It includes both the legacy two-step pattern and the modern single-step initialization.

use crate::constants::*;
use crate::types::*;
use crate::utils::serialization::{serialize_to_account, prepare_account_data};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
    program_pack::Pack,
};
use spl_token::{
    instruction as token_instruction,
    state::{Account as TokenAccount, Mint as MintAccount},
};

/// Creates Pool State PDA Account (DEPRECATED - Use InitializePool instead)
/// 
/// **DEPRECATED**: This instruction is part of the legacy two-instruction pattern.
/// Use `process_initialize_pool` instead for better reliability and performance.
/// 
/// This function creates the Pool State PDA account and all associated accounts
/// (vaults, LP token mints) but does not initialize the pool data. It's the first
/// step in the deprecated two-instruction pattern that was needed to work around
/// Solana AccountInfo.data issues.
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA validation
/// * `accounts` - Array of account infos in the required order
/// * `multiple_per_base` - The ratio of multiple tokens per base token
/// * `pool_authority_bump_seed` - Bump seed for pool authority PDA
/// * `multiple_token_vault_bump_seed` - Bump seed for multiple token vault PDA
/// * `base_token_vault_bump_seed` - Bump seed for base token vault PDA
/// 
/// # Returns
/// * `ProgramResult` - Success or error code
#[deprecated(note = "Use process_initialize_pool instead")]
pub fn process_create_pool_state_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    multiple_per_base: u64,
    pool_authority_bump_seed: u8,
    multiple_token_vault_bump_seed: u8,
    base_token_vault_bump_seed: u8,
) -> ProgramResult {
    msg!("DEBUG: process_create_pool_state_account: Entered");
    
    // ✅ SYSTEM PAUSE: Backward compatible validation
    crate::utils::validation::validate_system_not_paused_safe(accounts, 11)?; // Expected: 11 accounts minimum
    
    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Payer: {}", payer.key);
    let pool_state_pda_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Pool State PDA Account (from client): {}", pool_state_pda_account.key);
    let multiple_token_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Multiple Token Mint Account: {}", multiple_token_mint_account.key);
    let base_token_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Base Token Mint Account: {}", base_token_mint_account.key);
    let lp_token_a_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: LP Token A Mint Account: {}", lp_token_a_mint_account.key);
    let lp_token_b_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: LP Token B Mint Account: {}", lp_token_b_mint_account.key);
    let token_a_vault_pda_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Token A Vault PDA Account (from client): {}", token_a_vault_pda_account.key);
    let token_b_vault_pda_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Token B Vault PDA Account (from client): {}", token_b_vault_pda_account.key);
    let system_program_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: System Program Account: {}", system_program_account.key);
    let token_program_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Token Program Account: {}", token_program_account.key);
    let rent_sysvar_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Rent Sysvar Account: {}", rent_sysvar_account.key);
    
    msg!("DEBUG: process_create_pool_state_account: Parsed all accounts");

    let rent = &Rent::from_account_info(rent_sysvar_account)?;

    // Verify that payer is a signer
    if !payer.is_signer {
        msg!("DEBUG: process_create_pool_state_account: Payer is not a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }
    msg!("DEBUG: process_create_pool_state_account: Payer is signer check passed");

    // Verify ratio is non-zero
    if multiple_per_base == 0 {
        msg!("DEBUG: process_create_pool_state_account: Ratio cannot be zero");
        return Err(ProgramError::InvalidArgument);
    }
    msg!("DEBUG: process_create_pool_state_account: Ratio is non-zero check passed");

    // Enhanced normalization to prevent economic duplicates
    msg!("DEBUG: process_create_pool_state_account: Normalizing tokens and ratio...");
    
    // Step 1: Lexicographic token ordering
    let (token_a_mint_key, token_b_mint_key) = 
        if multiple_token_mint_account.key < base_token_mint_account.key {
            msg!("DEBUG: process_create_pool_state_account: Multiple mint < Base mint");
            (multiple_token_mint_account.key, base_token_mint_account.key)
        } else {
            msg!("DEBUG: process_create_pool_state_account: Multiple mint > Base mint");
            (base_token_mint_account.key, multiple_token_mint_account.key)
        };
    
    // Step 2: Canonical ratio mapping to prevent liquidity fragmentation
    // CRITICAL: All pools with the same token pair normalize to the same ratio
    // This prevents both "X A per 1 B" and "X B per 1 A" from being separate pools
    let (ratio_a_numerator, ratio_b_denominator, token_a_is_the_multiple) = 
        if multiple_token_mint_account.key < base_token_mint_account.key {
            // Multiple token is token A: direct mapping
            msg!("DEBUG: process_create_pool_state_account: Multiple token ({}) < Base token ({}) - TokenA is the multiple", 
                 multiple_token_mint_account.key, base_token_mint_account.key);
            (multiple_per_base, 1u64, true)
        } else {
            // Multiple token is token B: use canonical form to prevent economic duplicates
            // Both "X A per 1 B" and "X B per 1 A" normalize to same pool configuration
            msg!("DEBUG: process_create_pool_state_account: Multiple token ({}) >= Base token ({}) - TokenB is the multiple", 
                 multiple_token_mint_account.key, base_token_mint_account.key);
            (multiple_per_base, 1u64, false)
        };

    msg!("DEBUG: process_create_pool_state_account: Normalized: token_a_mint_key={}, token_b_mint_key={}, ratio_a_num={}, ratio_b_den={}, token_a_is_the_multiple={}", 
         token_a_mint_key, token_b_mint_key, ratio_a_numerator, ratio_b_denominator, token_a_is_the_multiple);

    let token_a_mint_account_info_ref = if token_a_is_the_multiple { multiple_token_mint_account } else { base_token_mint_account };
    let token_b_mint_account_info_ref = if token_a_is_the_multiple { base_token_mint_account } else { multiple_token_mint_account };
    msg!("DEBUG: process_create_pool_state_account: Set token_a/b_mint_account_info_refs");

    // Validate mint accounts
    if !multiple_token_mint_account.owner.eq(&spl_token::id()) || multiple_token_mint_account.data_len() != MintAccount::LEN {
        msg!("DEBUG: process_create_pool_state_account: Multiple token mint account is not a valid mint account");
        return Err(ProgramError::InvalidAccountData);
    }

    if !base_token_mint_account.owner.eq(&spl_token::id()) || base_token_mint_account.data_len() != MintAccount::LEN {
        msg!("DEBUG: process_create_pool_state_account: Base token mint account is not a valid mint account");
        return Err(ProgramError::InvalidAccountData);
    }
    msg!("DEBUG: process_create_pool_state_account: Mint account validations passed");

    // Verify the pool state PDA is derived correctly using normalized values
    msg!("DEBUG: process_create_pool_state_account: Verifying Pool State PDA. Pool Auth Bump Seed from instr: {}", pool_authority_bump_seed);
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        token_a_mint_key.as_ref(),
        token_b_mint_key.as_ref(),
        &ratio_a_numerator.to_le_bytes(),
        &ratio_b_denominator.to_le_bytes(),
        &[pool_authority_bump_seed],
    ];
    let expected_pool_state_pda = Pubkey::create_program_address(pool_state_pda_seeds, program_id)?;
    msg!("DEBUG: process_create_pool_state_account: Expected Pool State PDA (program derived): {}", expected_pool_state_pda);
    if *pool_state_pda_account.key != expected_pool_state_pda {
        msg!("DEBUG: process_create_pool_state_account: Invalid Pool State PDA address. Expected {}, got {}", expected_pool_state_pda, pool_state_pda_account.key);
        return Err(ProgramError::InvalidArgument);
    }
    msg!("DEBUG: process_create_pool_state_account: Pool State PDA address verification passed.");

    // Check if pool state already exists
    msg!("DEBUG: process_create_pool_state_account: Checking if pool state already exists. Data len: {}", pool_state_pda_account.data_len());
    if pool_state_pda_account.data_len() > 0 && !pool_state_pda_account.data_is_empty() {
        msg!("DEBUG: process_create_pool_state_account: Pool state account already exists");
        return Err(ProgramError::AccountAlreadyInitialized);
    } else {
        msg!("DEBUG: process_create_pool_state_account: Pool state PDA account is empty, proceeding with creation.");
    }

    // Map vault bump seeds
    msg!("DEBUG: process_create_pool_state_account: Mapping vault bump seeds. Multiple Vault Bump: {}, Base Vault Bump: {}", multiple_token_vault_bump_seed, base_token_vault_bump_seed);
    let (token_a_vault_bump, token_b_vault_bump) = if token_a_is_the_multiple {
        (multiple_token_vault_bump_seed, base_token_vault_bump_seed)
    } else {
        (base_token_vault_bump_seed, multiple_token_vault_bump_seed)
    };
    msg!("DEBUG: process_create_pool_state_account: Normalized token_a_vault_bump: {}, token_b_vault_bump: {}", token_a_vault_bump, token_b_vault_bump);

    // Verify vault PDAs
    msg!("DEBUG: process_create_pool_state_account: Verifying Token A Vault PDA...");
    let token_a_vault_pda_seeds = &[
        TOKEN_A_VAULT_SEED_PREFIX,
        pool_state_pda_account.key.as_ref(),
        &[token_a_vault_bump],
    ];
    let expected_token_a_vault_pda = Pubkey::create_program_address(token_a_vault_pda_seeds, program_id)?;
    msg!("DEBUG: process_create_pool_state_account: Expected Token A Vault PDA (program derived): {}", expected_token_a_vault_pda);
    if *token_a_vault_pda_account.key != expected_token_a_vault_pda {
        msg!("DEBUG: process_create_pool_state_account: Invalid Token A Vault PDA address. Expected {}, got {}", expected_token_a_vault_pda, token_a_vault_pda_account.key);
        return Err(ProgramError::InvalidArgument);
    }
    msg!("DEBUG: process_create_pool_state_account: Token A Vault PDA address verification passed.");

    msg!("DEBUG: process_create_pool_state_account: Verifying Token B Vault PDA...");
    let token_b_vault_pda_seeds = &[
        TOKEN_B_VAULT_SEED_PREFIX,
        pool_state_pda_account.key.as_ref(),
        &[token_b_vault_bump],
    ];
    let expected_token_b_vault_pda = Pubkey::create_program_address(token_b_vault_pda_seeds, program_id)?;
    msg!("DEBUG: process_create_pool_state_account: Expected Token B Vault PDA (program derived): {}", expected_token_b_vault_pda);
    if *token_b_vault_pda_account.key != expected_token_b_vault_pda {
        msg!("DEBUG: process_create_pool_state_account: Invalid Token B Vault PDA address. Expected {}, got {}", expected_token_b_vault_pda, token_b_vault_pda_account.key);
        return Err(ProgramError::InvalidArgument);
    }
    msg!("DEBUG: process_create_pool_state_account: Token B Vault PDA address verification passed.");
    
    // Create the Pool State PDA account with ACTUAL SERIALIZED SIZE
    // CRITICAL: GitHub Issue #31960 workaround - use actual size instead of calculated packed length
    let temp_pool_state = PoolState::default();
    let mut temp_buffer = Vec::new();
    temp_pool_state.serialize(&mut temp_buffer)?;
    let pool_state_account_size = temp_buffer.len();
    let rent_for_pool_state = rent.minimum_balance(pool_state_account_size);
    msg!("DEBUG: process_create_pool_state_account: Creating Pool State PDA account: {}. Size: {} (actual serialized vs {} calculated). Rent: {}", 
         pool_state_pda_account.key, pool_state_account_size, PoolState::get_packed_len(), rent_for_pool_state);
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            pool_state_pda_account.key,
            rent_for_pool_state,
            pool_state_account_size as u64,
            program_id,
        ),
        &[
            payer.clone(),
            pool_state_pda_account.clone(),
            system_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;
    msg!("DEBUG: process_create_pool_state_account: Pool State PDA account created");

    // Transfer registration fee to pool state PDA
    if payer.lamports() < REGISTRATION_FEE {
        msg!("DEBUG: process_create_pool_state_account: Insufficient SOL for registration fee. Required: {}, Payer has: {}", REGISTRATION_FEE, payer.lamports());
        return Err(ProgramError::InsufficientFunds);
    }
    msg!("DEBUG: process_create_pool_state_account: Payer SOL for registration fee check passed. Payer lamports: {}", payer.lamports());

    msg!("DEBUG: process_create_pool_state_account: Transferring registration fee: {} from {} to {}", REGISTRATION_FEE, payer.key, pool_state_pda_account.key);
    invoke(
        &system_instruction::transfer(payer.key, pool_state_pda_account.key, REGISTRATION_FEE),
        &[
            payer.clone(),
            pool_state_pda_account.clone(),
            system_program_account.clone(),
        ],
    )?;
    msg!("DEBUG: process_create_pool_state_account: Registration fee transferred to pool state PDA.");

    // Create LP Token mints
    let rent_for_mint = rent.minimum_balance(MintAccount::LEN);
    msg!("DEBUG: process_create_pool_state_account: Creating LP Token A Mint account: {}. Rent: {}", lp_token_a_mint_account.key, rent_for_mint);
    invoke(
        &system_instruction::create_account(
            payer.key,
            lp_token_a_mint_account.key,
            rent_for_mint,
            MintAccount::LEN as u64,
            token_program_account.key,
        ),
        &[
            payer.clone(), 
            lp_token_a_mint_account.clone(), 
            system_program_account.clone()
        ],
    )?;
    msg!("DEBUG: process_create_pool_state_account: LP Token A Mint account created. Initializing...");
    invoke(
        &token_instruction::initialize_mint(
            token_program_account.key,
            lp_token_a_mint_account.key,
            payer.key,
            None,
            9,
        )?,
        &[
            lp_token_a_mint_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
    )?;
    msg!("DEBUG: process_create_pool_state_account: LP Token A Mint initialized");

    msg!("DEBUG: process_create_pool_state_account: Creating LP Token B Mint account: {}. Rent: {}", lp_token_b_mint_account.key, rent_for_mint);
    invoke(
        &system_instruction::create_account(
            payer.key,
            lp_token_b_mint_account.key,
            rent_for_mint,
            MintAccount::LEN as u64,
            token_program_account.key,
        ),
        &[
            payer.clone(), 
            lp_token_b_mint_account.clone(), 
            system_program_account.clone()
        ],
    )?;
    msg!("DEBUG: process_create_pool_state_account: LP Token B Mint account created. Initializing...");
    invoke(
        &token_instruction::initialize_mint(
            token_program_account.key,
            lp_token_b_mint_account.key,
            payer.key,
            None,
            9,
        )?,
        &[
            lp_token_b_mint_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
    )?;
    msg!("DEBUG: process_create_pool_state_account: LP Token B Mint initialized");

    // Transfer authority of LP token mints to pool state PDA
    msg!("DEBUG: process_create_pool_state_account: Transferring authority of LP Token A Mint to pool state PDA");
    invoke(
        &token_instruction::set_authority(
            token_program_account.key,
            lp_token_a_mint_account.key,
            Some(pool_state_pda_account.key),
            token_instruction::AuthorityType::MintTokens,
            payer.key,
            &[],
        )?,
        &[
            lp_token_a_mint_account.clone(),
            pool_state_pda_account.clone(),
            payer.clone(),
            token_program_account.clone(),
        ],
    )?;

    msg!("DEBUG: process_create_pool_state_account: Transferring authority of LP Token B Mint to pool state PDA");
    invoke(
        &token_instruction::set_authority(
            token_program_account.key,
            lp_token_b_mint_account.key,
            Some(pool_state_pda_account.key),
            token_instruction::AuthorityType::MintTokens,
            payer.key,
            &[],
        )?,
        &[
            lp_token_b_mint_account.clone(),
            pool_state_pda_account.clone(),
            payer.clone(),
            token_program_account.clone(),
        ],
    )?;

    // Create token vaults
    let vault_account_size = TokenAccount::LEN;
    let rent_for_vault = rent.minimum_balance(vault_account_size);
    msg!("DEBUG: process_create_pool_state_account: Creating Token A Vault PDA account: {}. Size: {}. Rent: {}. Mint: {}", token_a_vault_pda_account.key, vault_account_size, rent_for_vault, token_a_mint_account_info_ref.key);
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            token_a_vault_pda_account.key,
            rent_for_vault,
            vault_account_size as u64,
            token_program_account.key,
        ),
        &[
            payer.clone(),
            token_a_vault_pda_account.clone(),
            system_program_account.clone(),
        ],
        &[token_a_vault_pda_seeds],
    )?;
    msg!("DEBUG: process_create_pool_state_account: Token A Vault PDA account created. Initializing...");
    invoke_signed(
        &token_instruction::initialize_account(
            token_program_account.key,
            token_a_vault_pda_account.key,
            token_a_mint_account_info_ref.key,
            pool_state_pda_account.key,
        )?,
        &[
            token_a_vault_pda_account.clone(),
            token_a_mint_account_info_ref.clone(),
            pool_state_pda_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;
    msg!("DEBUG: process_create_pool_state_account: Token A Vault PDA initialized");

    msg!("DEBUG: process_create_pool_state_account: Creating Token B Vault PDA account: {}. Size: {}. Rent: {}. Mint: {}", token_b_vault_pda_account.key, vault_account_size, rent_for_vault, token_b_mint_account_info_ref.key);
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            token_b_vault_pda_account.key,
            rent_for_vault,
            vault_account_size as u64,
            token_program_account.key,
        ),
        &[
            payer.clone(),
            token_b_vault_pda_account.clone(),
            system_program_account.clone(),
        ],
        &[token_b_vault_pda_seeds],
    )?;
    msg!("DEBUG: process_create_pool_state_account: Token B Vault PDA account created. Initializing...");
    invoke_signed(
        &token_instruction::initialize_account(
            token_program_account.key,
            token_b_vault_pda_account.key,
            token_b_mint_account_info_ref.key,
            pool_state_pda_account.key,
        )?,
        &[
            token_b_vault_pda_account.clone(),
            token_b_mint_account_info_ref.clone(),
            pool_state_pda_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;
    msg!("DEBUG: process_create_pool_state_account: Token B Vault PDA initialized");

    msg!("DEBUG: process_create_pool_state_account: All accounts created successfully");
    Ok(())
}

/// Initializes the data in the already-created Pool State PDA account.
/// 
/// This function complements `process_create_pool_state_account` by actually populating
/// the pool state data structure after the account has been created. This separation
/// allows for flexible initialization patterns and workarounds for Solana's account
/// size limitations during creation.
/// 
/// # What it does:
/// 1. Validates all account PDAs match expected derivations
/// 2. Verifies the pool state account is correctly sized but not yet initialized
/// 3. Creates and populates the PoolState data structure
/// 4. Initializes security parameters and rent requirements
/// 5. Sets up fee tracking with 0% swap fees as per requirements
/// 6. Uses buffer serialization workaround for reliable data persistence
/// 
/// # Buffer Serialization Workaround
/// Instead of directly serializing to the account data, this function uses a two-step process:
/// 1. Serialize to a temporary buffer to ensure the operation succeeds
/// 2. Copy the buffer contents to the account data atomically
/// 
/// This approach prevents issues where serialization reports "OK" but data doesn't persist,
/// which can occur with direct serialization on some Solana runtime versions.
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA validation
/// * `accounts` - The same accounts used in pool creation (order must match)
/// * `multiple_per_base` - The fixed ratio between multiple and base tokens
/// * `pool_authority_bump_seed` - Bump seed for pool authority PDA
/// * `multiple_token_vault_bump_seed` - Bump seed for multiple token vault PDA  
/// * `base_token_vault_bump_seed` - Bump seed for base token vault PDA
/// 
/// # Returns
/// * `ProgramResult` - Success or error code
/// 
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Payer not signed
/// - `ProgramError::InvalidArgument` - Invalid ratio or PDA addresses
/// - `ProgramError::AccountAlreadyInitialized` - Pool already initialized
/// - `ProgramError::InvalidAccountData` - Account size or structure issues
pub fn process_initialize_pool_data(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    multiple_per_base: u64,
    pool_authority_bump_seed: u8,
    multiple_token_vault_bump_seed: u8,
    base_token_vault_bump_seed: u8,
) -> ProgramResult {
    msg!("DEBUG: process_initialize_pool_data: Entered");
    
    // ✅ SYSTEM PAUSE: Backward compatible validation
    crate::utils::validation::validate_system_not_paused_safe(accounts, 11)?; // Expected: 11 accounts minimum
    
    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: Payer: {}", payer.key);
    let pool_state_pda_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: Pool State PDA Account (from client): {}", pool_state_pda_account.key);
    let multiple_token_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: Multiple Token Mint Account: {}", multiple_token_mint_account.key);
    let base_token_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: Base Token Mint Account: {}", base_token_mint_account.key);
    let lp_token_a_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: LP Token A Mint Account: {}", lp_token_a_mint_account.key);
    let lp_token_b_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: LP Token B Mint Account: {}", lp_token_b_mint_account.key);
    let token_a_vault_pda_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: Token A Vault PDA Account (from client): {}", token_a_vault_pda_account.key);
    let token_b_vault_pda_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: Token B Vault PDA Account (from client): {}", token_b_vault_pda_account.key);
    let _system_program_account = next_account_info(account_info_iter)?;
    let _token_program_account = next_account_info(account_info_iter)?;
    let rent_sysvar_account = next_account_info(account_info_iter)?;
    
    msg!("DEBUG: process_initialize_pool_data: Parsed all accounts");

    let rent = &Rent::from_account_info(rent_sysvar_account)?;

    // Verify that payer is a signer
    if !payer.is_signer {
        msg!("DEBUG: process_initialize_pool_data: Payer is not a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }
    msg!("DEBUG: process_initialize_pool_data: Payer is signer check passed");

    // Verify ratio is non-zero
    if multiple_per_base == 0 {
        msg!("DEBUG: process_initialize_pool_data: Ratio cannot be zero");
        return Err(ProgramError::InvalidArgument);
    }
    msg!("DEBUG: process_initialize_pool_data: Ratio is non-zero check passed");

    // Enhanced normalization to prevent economic duplicates
    msg!("DEBUG: process_initialize_pool_data: Normalizing tokens and ratio...");
    
    // Step 1: Lexicographic token ordering
    let (token_a_mint_key, token_b_mint_key) = 
        if multiple_token_mint_account.key < base_token_mint_account.key {
            msg!("DEBUG: process_initialize_pool_data: Multiple mint < Base mint");
            (multiple_token_mint_account.key, base_token_mint_account.key)
        } else {
            msg!("DEBUG: process_initialize_pool_data: Multiple mint > Base mint");
            (base_token_mint_account.key, multiple_token_mint_account.key)
        };
    
    // Step 2: Canonical ratio mapping to prevent liquidity fragmentation
    // CRITICAL: All pools with the same token pair normalize to the same ratio
    // This prevents both "X A per 1 B" and "X B per 1 A" from being separate pools
    let (ratio_a_numerator, ratio_b_denominator, token_a_is_the_multiple) = 
        if multiple_token_mint_account.key < base_token_mint_account.key {
            // Multiple token is token A: direct mapping
            msg!("DEBUG: process_initialize_pool_data: Multiple token ({}) < Base token ({}) - TokenA is the multiple", 
                 multiple_token_mint_account.key, base_token_mint_account.key);
            (multiple_per_base, 1u64, true)
        } else {
            // Multiple token is token B: use canonical form to prevent economic duplicates
            // Both "X A per 1 B" and "X B per 1 A" normalize to same pool configuration
            msg!("DEBUG: process_initialize_pool_data: Multiple token ({}) >= Base token ({}) - TokenB is the multiple", 
                 multiple_token_mint_account.key, base_token_mint_account.key);
            (multiple_per_base, 1u64, false)
        };

    msg!("DEBUG: process_initialize_pool_data: Normalized: token_a_mint_key={}, token_b_mint_key={}, ratio_a_num={}, ratio_b_den={}, token_a_is_the_multiple={}", 
         token_a_mint_key, token_b_mint_key, ratio_a_numerator, ratio_b_denominator, token_a_is_the_multiple);

    // Verify the pool state PDA is derived correctly using normalized values
    msg!("DEBUG: process_initialize_pool_data: Verifying Pool State PDA. Pool Auth Bump Seed from instr: {}", pool_authority_bump_seed);
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        token_a_mint_key.as_ref(),
        token_b_mint_key.as_ref(),
        &ratio_a_numerator.to_le_bytes(),
        &ratio_b_denominator.to_le_bytes(),
        &[pool_authority_bump_seed],
    ];
    let expected_pool_state_pda = Pubkey::create_program_address(pool_state_pda_seeds, program_id)?;
    msg!("DEBUG: process_initialize_pool_data: Expected Pool State PDA (program derived): {}", expected_pool_state_pda);
    if *pool_state_pda_account.key != expected_pool_state_pda {
        msg!("DEBUG: process_initialize_pool_data: Invalid Pool State PDA address. Expected {}, got {}", expected_pool_state_pda, pool_state_pda_account.key);
        return Err(ProgramError::InvalidArgument);
    }
    msg!("DEBUG: process_initialize_pool_data: Pool State PDA address verification passed.");

    // Check if pool state account exists and has the correct size
    // CRITICAL: GitHub Issue #31960 workaround - check against actual serialized size
    let temp_pool_state = PoolState::default();
    let mut temp_buffer = Vec::new();
    temp_pool_state.serialize(&mut temp_buffer)?;
    let expected_size = temp_buffer.len();
    
    msg!("DEBUG: process_initialize_pool_data: Checking pool state account. Data len: {}", pool_state_pda_account.data_len());
    if pool_state_pda_account.data_len() != expected_size {
        msg!("DEBUG: process_initialize_pool_data: Pool state account has incorrect size. Expected: {} (actual), Got: {} (packed_len would be: {})", 
             expected_size, pool_state_pda_account.data_len(), PoolState::get_packed_len());
        return Err(ProgramError::InvalidAccountData);
    }

    // Check if pool state is already initialized
    if !pool_state_pda_account.data_is_empty() {
        match PoolState::deserialize(&mut &pool_state_pda_account.data.borrow()[..]) {
            Ok(pool_state_data) => {
                if pool_state_data.is_initialized {
                    msg!("DEBUG: process_initialize_pool_data: Pool state already initialized");
                    return Err(ProgramError::AccountAlreadyInitialized);
                }
                msg!("DEBUG: process_initialize_pool_data: Pool state data found but not initialized, proceeding.");
            }
            Err(_) => {
                // If we can't deserialize, check if it's all zeros (uninitialized)
                let is_zeroed = pool_state_pda_account.data.borrow().iter().all(|&x| x == 0);
                if !is_zeroed {
                    msg!("DEBUG: process_initialize_pool_data: Pool state account has data but is not a valid PoolState struct and not zeroed.");
                    return Err(ProgramError::InvalidAccountData);
                }
                msg!("DEBUG: process_initialize_pool_data: Pool state account data is zeroed, proceeding.");
            }
        }
    }

    // Map vault bump seeds
    msg!("DEBUG: process_initialize_pool_data: Mapping vault bump seeds. Multiple Vault Bump: {}, Base Vault Bump: {}", multiple_token_vault_bump_seed, base_token_vault_bump_seed);
    let (token_a_vault_bump, token_b_vault_bump) = if token_a_is_the_multiple {
        (multiple_token_vault_bump_seed, base_token_vault_bump_seed)
    } else {
        (base_token_vault_bump_seed, multiple_token_vault_bump_seed)
    };
    msg!("DEBUG: process_initialize_pool_data: Normalized token_a_vault_bump: {}, token_b_vault_bump: {}", token_a_vault_bump, token_b_vault_bump);

    // Initialize Pool State data struct
    msg!("DEBUG: process_initialize_pool_data: Initializing Pool State data struct");
    let mut pool_state_data = PoolState::default();
    
    pool_state_data.owner = *payer.key;
    pool_state_data.token_a_mint = *token_a_mint_key;
    pool_state_data.token_b_mint = *token_b_mint_key;
    pool_state_data.token_a_vault = *token_a_vault_pda_account.key;
    pool_state_data.token_b_vault = *token_b_vault_pda_account.key;
    pool_state_data.lp_token_a_mint = *lp_token_a_mint_account.key;
    pool_state_data.lp_token_b_mint = *lp_token_b_mint_account.key;
    pool_state_data.ratio_a_numerator = ratio_a_numerator;
    pool_state_data.ratio_b_denominator = ratio_b_denominator;
    // Note: token_a_is_the_multiple field has been removed in favor of one_to_many_ratio
    // This field was app-specific display logic that is now handled by applications
    pool_state_data.one_to_many_ratio = false; // Will be enhanced with proper detection
    msg!("DEBUG: process_initialize_pool_data: Set one_to_many_ratio = {} (placeholder for legacy function)", pool_state_data.one_to_many_ratio);
    pool_state_data.total_token_a_liquidity = 0;
    pool_state_data.total_token_b_liquidity = 0;
    pool_state_data.pool_authority_bump_seed = pool_authority_bump_seed;
    pool_state_data.token_a_vault_bump_seed = token_a_vault_bump;
    pool_state_data.token_b_vault_bump_seed = token_b_vault_bump;
    pool_state_data.is_initialized = true;

    // Initialize security parameters
    pool_state_data.paused = false;

    // Initialize rent requirements
    let rent_requirements = RentRequirements::new(rent);
    pool_state_data.rent_requirements = rent_requirements;

    // Initialize fee tracking
    pool_state_data.collected_fees_token_a = 0;
    pool_state_data.collected_fees_token_b = 0;
    pool_state_data.total_fees_withdrawn_token_a = 0;
    pool_state_data.total_fees_withdrawn_token_b = 0;
    
    // Initialize swap fee to 0% as per requirements
    pool_state_data.swap_fee_basis_points = 0;
    
    // BUFFER SERIALIZATION WORKAROUND:
    // Instead of directly serializing to AccountInfo.data.borrow_mut(), we use a two-step process:
    // 1. Serialize to a temporary buffer to ensure the operation succeeds
    // 2. Copy the buffer contents to the account data
    // This approach prevents issues where serialization reports "OK" but data doesn't persist.
    
    // Step 1: Serialize the pool state data to a temporary buffer
    let (_serialized_data, _pool_state_account_size) = prepare_account_data(&pool_state_data)?;
    
    // Step 2: Copy the serialized data to the account data
    msg!("DEBUG: process_initialize_pool_data: Copying data to account");
    let account_data_len = pool_state_pda_account.data_len();
    if _serialized_data.len() > account_data_len {
        msg!("DEBUG: process_initialize_pool_data: Serialized data too large for account. Need: {}, Have: {}", 
            _serialized_data.len(), account_data_len);
        return Err(ProgramError::AccountDataTooSmall);
    }

    // Step 3: Atomic copy to account data
    {
        let mut account_data = pool_state_pda_account.data.borrow_mut();
        account_data[.._serialized_data.len()].copy_from_slice(&_serialized_data);
        msg!("DEBUG: process_initialize_pool_data: Data copied to account successfully");
    }
    
    msg!("DEBUG: process_initialize_pool_data: Pool State PDA data len after copy: {}", pool_state_pda_account.data.borrow().len());
    msg!("DEBUG: process_initialize_pool_data: Pool State PDA initialized with data: {:?}", pool_state_data);
    msg!("DEBUG: process_initialize_pool_data: Exiting successfully");

    Ok(())
}

/// **RECOMMENDED**: Single-instruction pool initialization (FIXED)
/// 
/// This function creates and initializes a pool in a single atomic operation,
/// replacing the deprecated two-instruction pattern. It performs all necessary
/// operations including account creation, PDA derivation, and data initialization.
/// 
/// # What it does (All in one atomic transaction):
/// 1. Creates Pool State PDA account with proper size allocation
/// 2. Creates LP token mints and transfers authority to pool
/// 3. Creates token vault PDAs and initializes them
/// 4. Initializes pool state data with all configuration
/// 5. Transfers registration fees to pool state account
/// 6. Uses buffer serialization workaround for reliable data persistence
/// 
/// # Key Improvements over Legacy Pattern:
/// - **Atomic Operation**: All-or-nothing execution prevents partial state
/// - **Simplified Client Integration**: Single instruction call
/// - **Better Error Handling**: Clearer error messages and validation
/// - **Enhanced Security**: Comprehensive validation and rent exemption checks
/// - **Future-Proof**: Designed for extensibility and maintenance
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA validation and account creation
/// * `accounts` - Array of account infos in the required order (see account list below)
/// * `ratio_a_numerator` - Token A base units (replaces multiple_per_base)
/// * `ratio_b_denominator` - Token B base units (was hardcoded to 1, now configurable)
/// * `pool_authority_bump_seed` - Bump seed for pool authority PDA derivation
/// * `token_a_vault_bump_seed` - Bump seed for token A vault PDA (renamed from multiple_token_vault_bump_seed)
/// * `token_b_vault_bump_seed` - Bump seed for token B vault PDA (renamed from base_token_vault_bump_seed)
/// 
/// # Returns
/// * `ProgramResult` - Success or error code
/// 
/// # Account Order (11 accounts required):
/// 0. **Payer** (signer, writable) - Pays for account creation and fees
/// 1. **Pool State PDA** (writable) - Main pool account to be created
/// 2. **Multiple Token Mint** (readable) - The abundant token mint
/// 3. **Base Token Mint** (readable) - The valuable token mint  
/// 4. **LP Token A Mint** (signer, writable) - LP token for Token A liquidity providers
/// 5. **LP Token B Mint** (signer, writable) - LP token for Token B liquidity providers
/// 6. **Token A Vault PDA** (writable) - Vault for Token A liquidity
/// 7. **Token B Vault PDA** (writable) - Vault for Token B liquidity
/// 8. **System Program** (readable) - For account creation
/// 9. **SPL Token Program** (readable) - For token operations
/// 10. **Rent Sysvar** (readable) - For rent exemption calculations
/// 
/// # Security Features
/// - Enhanced normalization prevents economic duplicate pools
/// - Comprehensive PDA validation ensures address correctness
/// - Rent exemption validation prevents account closure
/// - Buffer serialization prevents data corruption
/// - System pause integration for emergency stops
pub fn process_initialize_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ratio_a_numerator: u64,
    ratio_b_denominator: u64,
    pool_authority_bump_seed: u8,
    token_a_vault_bump_seed: u8,
    token_b_vault_bump_seed: u8,
) -> ProgramResult {
    msg!("DEBUG: process_initialize_pool: Starting FIXED single-instruction pool initialization");
    
    // ✅ SYSTEM PAUSE: Backward compatible validation
    crate::utils::validation::validate_system_not_paused_safe(accounts, 11)?; // Expected: 11 accounts minimum
    
    // CRITICAL FIX: Instead of calling separate functions, we implement everything inline
    // to avoid the GITHUB_ISSUE_31960_WORKAROUND issue where AccountInfo.data doesn't 
    // get updated after CPI account creation within the same instruction.
    
    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let pool_state_pda_account = next_account_info(account_info_iter)?;
    let multiple_token_mint_account = next_account_info(account_info_iter)?;
    let base_token_mint_account = next_account_info(account_info_iter)?;
    let lp_token_a_mint_account = next_account_info(account_info_iter)?;
    let lp_token_b_mint_account = next_account_info(account_info_iter)?;
    let token_a_vault_pda_account = next_account_info(account_info_iter)?;
    let token_b_vault_pda_account = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;
    let token_program_account = next_account_info(account_info_iter)?;
    let rent_sysvar_account = next_account_info(account_info_iter)?;

    let rent = &Rent::from_account_info(rent_sysvar_account)?;

    // Verify that payer is a signer
    if !payer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate ratio values
    crate::utils::validation::validate_ratio_values(ratio_a_numerator, ratio_b_denominator)?;

    // Token normalization: Always store tokens in lexicographic order (Token A < Token B)
    let (token_a_mint_key, token_b_mint_key) = 
        if multiple_token_mint_account.key < base_token_mint_account.key {
            (multiple_token_mint_account.key, base_token_mint_account.key)
        } else {
            (base_token_mint_account.key, multiple_token_mint_account.key)
        };

    msg!("DEBUG: process_initialize_pool: Normalized tokens: token_a_mint_key={}, token_b_mint_key={}, ratio_a_num={}, ratio_b_den={}", 
         token_a_mint_key, token_b_mint_key, ratio_a_numerator, ratio_b_denominator);

    // Verify the pool state PDA
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        token_a_mint_key.as_ref(),
        token_b_mint_key.as_ref(),
        &ratio_a_numerator.to_le_bytes(),
        &ratio_b_denominator.to_le_bytes(),
        &[pool_authority_bump_seed],
    ];
    let expected_pool_state_pda = Pubkey::create_program_address(pool_state_pda_seeds, program_id)?;
    if *pool_state_pda_account.key != expected_pool_state_pda {
        return Err(ProgramError::InvalidArgument);
    }

    // Check if pool already exists
    if pool_state_pda_account.data_len() > 0 && !pool_state_pda_account.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // Use the provided vault bump seeds directly (already named correctly)
    let token_a_vault_bump = token_a_vault_bump_seed;
    let token_b_vault_bump = token_b_vault_bump_seed;

    // Derive vault PDAs for size calculation
    let token_a_vault_seeds = &[
        TOKEN_A_VAULT_SEED_PREFIX,
        pool_state_pda_account.key.as_ref(),
        &[token_a_vault_bump],
    ];
    let token_b_vault_seeds = &[
        TOKEN_B_VAULT_SEED_PREFIX,
        pool_state_pda_account.key.as_ref(),
        &[token_b_vault_bump],
    ];

    // CRITICAL FIX: Use get_packed_len() to ensure max size allocation
    let pool_state_account_size = PoolState::get_packed_len();
    
    msg!("DEBUG: Using get_packed_len for allocation - size: {} bytes", 
         pool_state_account_size);
    let rent_for_pool_state = rent.minimum_balance(pool_state_account_size);
    
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            pool_state_pda_account.key,
            rent_for_pool_state,
            pool_state_account_size as u64,
            program_id,
        ),
        &[
            payer.clone(),
            pool_state_pda_account.clone(),
            system_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;

    //=========================================================================
    // CONTRACT FEE TRANSFER (Pool Creation Fee - Fixed SOL Amount)
    //=========================================================================
    // Pool creation requires a one-time contract fee to cover the computational
    // costs of account creation, PDA derivation, and initial setup.
    //
    // Amount: 1.15 SOL (1,150,000,000 lamports)
    // Purpose: Cover pool creation costs and prevent spam pool creation
    
    invoke(
        &system_instruction::transfer(payer.key, pool_state_pda_account.key, REGISTRATION_FEE),
        &[
            payer.clone(),
            pool_state_pda_account.clone(),
            system_program_account.clone(),
        ],
    )?;
    
    msg!("✅ Pool creation contract fee transferred: {} lamports ({} SOL) from creator to pool", 
         REGISTRATION_FEE, REGISTRATION_FEE as f64 / 1_000_000_000.0);

    // Create and initialize LP token mints
    let rent_for_mint = rent.minimum_balance(MintAccount::LEN);
    
    // Create LP Token A Mint
    invoke(
        &system_instruction::create_account(
            payer.key,
            lp_token_a_mint_account.key,
            rent_for_mint,
            MintAccount::LEN as u64,
            token_program_account.key,
        ),
        &[payer.clone(), lp_token_a_mint_account.clone(), system_program_account.clone()],
    )?;
    
    invoke(
        &token_instruction::initialize_mint(
            token_program_account.key,
            lp_token_a_mint_account.key,
            payer.key,
            None,
            9,
        )?,
        &[lp_token_a_mint_account.clone(), rent_sysvar_account.clone(), token_program_account.clone()],
    )?;

    // Create LP Token B Mint
    invoke(
        &system_instruction::create_account(
            payer.key,
            lp_token_b_mint_account.key,
            rent_for_mint,
            MintAccount::LEN as u64,
            token_program_account.key,
        ),
        &[payer.clone(), lp_token_b_mint_account.clone(), system_program_account.clone()],
    )?;
    
    invoke(
        &token_instruction::initialize_mint(
            token_program_account.key,
            lp_token_b_mint_account.key,
            payer.key,
            None,
            9,
        )?,
        &[lp_token_b_mint_account.clone(), rent_sysvar_account.clone(), token_program_account.clone()],
    )?;

    // Transfer LP mint authorities to pool
    invoke(
        &token_instruction::set_authority(
            token_program_account.key,
            lp_token_a_mint_account.key,
            Some(pool_state_pda_account.key),
            token_instruction::AuthorityType::MintTokens,
            payer.key,
            &[],
        )?,
        &[lp_token_a_mint_account.clone(), pool_state_pda_account.clone(), payer.clone(), token_program_account.clone()],
    )?;

    invoke(
        &token_instruction::set_authority(
            token_program_account.key,
            lp_token_b_mint_account.key,
            Some(pool_state_pda_account.key),
            token_instruction::AuthorityType::MintTokens,
            payer.key,
            &[],
        )?,
        &[lp_token_b_mint_account.clone(), pool_state_pda_account.clone(), payer.clone(), token_program_account.clone()],
    )?;

    // Map token mint accounts to normalized token A/B based on lexicographic order
    let token_a_mint_account_ref = if multiple_token_mint_account.key < base_token_mint_account.key {
        multiple_token_mint_account
    } else {
        base_token_mint_account
    };
    let token_b_mint_account_ref = if multiple_token_mint_account.key < base_token_mint_account.key {
        base_token_mint_account
    } else {
        multiple_token_mint_account
    };

    // Create token vaults
    let rent_for_vault = rent.minimum_balance(TokenAccount::LEN);
    
    // Create Token A Vault
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            token_a_vault_pda_account.key,
            rent_for_vault,
            TokenAccount::LEN as u64,
            token_program_account.key,
        ),
        &[payer.clone(), token_a_vault_pda_account.clone(), system_program_account.clone()],
        &[token_a_vault_seeds],
    )?;
    
    invoke_signed(
        &token_instruction::initialize_account(
            token_program_account.key,
            token_a_vault_pda_account.key,
            token_a_mint_account_ref.key,
            pool_state_pda_account.key,
        )?,
        &[
            token_a_vault_pda_account.clone(),
            token_a_mint_account_ref.clone(),
            pool_state_pda_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;

    // Create Token B Vault
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            token_b_vault_pda_account.key,
            rent_for_vault,
            TokenAccount::LEN as u64,
            token_program_account.key,
        ),
        &[payer.clone(), token_b_vault_pda_account.clone(), system_program_account.clone()],
        &[token_b_vault_seeds],
    )?;
    
    invoke_signed(
        &token_instruction::initialize_account(
            token_program_account.key,
            token_b_vault_pda_account.key,
            token_b_mint_account_ref.key,
            pool_state_pda_account.key,
        )?,
        &[
            token_b_vault_pda_account.clone(),
            token_b_mint_account_ref.clone(),
            pool_state_pda_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;

    // **CRITICAL: GitHub Issue #31960 Workaround - Initialize pool state with standardized utility**
    // Create the actual pool state data with all fields properly initialized
    let mut pool_state_data = PoolState::default();
    pool_state_data.owner = *payer.key;
    pool_state_data.token_a_mint = *token_a_mint_key;
    pool_state_data.token_b_mint = *token_b_mint_key;
    pool_state_data.token_a_vault = *token_a_vault_pda_account.key;
    pool_state_data.token_b_vault = *token_b_vault_pda_account.key;
    pool_state_data.lp_token_a_mint = *lp_token_a_mint_account.key;
    pool_state_data.lp_token_b_mint = *lp_token_b_mint_account.key;
    pool_state_data.ratio_a_numerator = ratio_a_numerator;
    pool_state_data.ratio_b_denominator = ratio_b_denominator;
    
    // Determine one-to-many ratio with token decimal information
    let token_a_mint_data = token_a_mint_account_ref.try_borrow_data()?;
    let token_b_mint_data = token_b_mint_account_ref.try_borrow_data()?;
    
    if token_a_mint_data.len() >= MintAccount::LEN && token_b_mint_data.len() >= MintAccount::LEN {
        let token_a_mint_info = MintAccount::unpack(&token_a_mint_data)?;
        let token_b_mint_info = MintAccount::unpack(&token_b_mint_data)?;
        
        pool_state_data.one_to_many_ratio = crate::utils::validation::check_one_to_many_ratio(
            ratio_a_numerator,
            ratio_b_denominator,
            token_a_mint_info.decimals,
            token_b_mint_info.decimals,
        );
        
        msg!("DEBUG: process_initialize_pool: Token A decimals: {}, Token B decimals: {}", 
             token_a_mint_info.decimals, token_b_mint_info.decimals);
        msg!("DEBUG: process_initialize_pool: Determined one_to_many_ratio = {}", pool_state_data.one_to_many_ratio);
    } else {
        // Fallback: assume standard decimals (9 for both) and detect based on that
        pool_state_data.one_to_many_ratio = crate::utils::validation::check_one_to_many_ratio(
            ratio_a_numerator,
            ratio_b_denominator,
            9, // Default to 9 decimals
            9, // Default to 9 decimals
        );
        msg!("DEBUG: process_initialize_pool: Using fallback decimals (9,9), one_to_many_ratio = {}", pool_state_data.one_to_many_ratio);
    }
    pool_state_data.total_token_a_liquidity = 0;
    pool_state_data.total_token_b_liquidity = 0;
    pool_state_data.pool_authority_bump_seed = pool_authority_bump_seed;
    pool_state_data.token_a_vault_bump_seed = token_a_vault_bump;
    pool_state_data.token_b_vault_bump_seed = token_b_vault_bump;
    pool_state_data.is_initialized = true;
    pool_state_data.paused = false;
    pool_state_data.rent_requirements = RentRequirements::new(rent);
    pool_state_data.collected_fees_token_a = 0;
    pool_state_data.collected_fees_token_b = 0;
    pool_state_data.total_fees_withdrawn_token_a = 0;
    pool_state_data.total_fees_withdrawn_token_b = 0;
    pool_state_data.swap_fee_basis_points = 0;
    // Initialize SOL fee tracking with registration fee
    pool_state_data.collected_sol_fees = REGISTRATION_FEE;
    pool_state_data.total_sol_fees_withdrawn = 0;

    // **Use standardized GitHub Issue #31960 workaround**
    serialize_to_account(&pool_state_data, pool_state_pda_account)?;

    msg!("DEBUG: process_initialize_pool: Successfully used standardized workaround for pool initialization");
    Ok(())
} 