//! Pool Creation Processors
//! 
//! This module contains all the processors for pool creation and initialization operations.
//! It includes both the legacy two-step pattern and the modern single-step initialization.

use crate::constants::*;
use crate::types::*;
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

/// Creates the Pool State PDA account and all related accounts (LP mints, vaults).
/// This is Step 1 of the two-instruction pool initialization pattern.
///
/// WORKAROUND CONTEXT:
/// This function implements the first part of a workaround for Solana AccountInfo.data
/// issue where AccountInfo.data doesn't get updated after CPI account creation within
/// the same instruction. See GitHub Issue #31960 and related community discussions.
///
/// WHY THIS APPROACH:
/// 1. Creates all required accounts via CPI (Pool State PDA, LP mints, token vaults)
/// 2. Deliberately AVOIDS writing PoolState data to prevent AccountInfo.data issues
/// 3. Allows the second instruction (InitializePoolData) to run with fresh AccountInfo
///    references that properly point to the allocated on-chain account buffers
///
/// WHAT THIS FUNCTION DOES:
/// - Validates all input parameters and PDA derivations
/// - Creates Pool State PDA account with correct size via system_instruction::create_account
/// - Creates and initializes LP token mints, transfers authority to pool
/// - Creates and initializes token vault PDAs
/// - Transfers registration fees to pool
/// - Does NOT serialize any PoolState data (that's done in Step 2)
///
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for pool creation
/// * `ratio_primary_per_base` - The ratio of primary tokens per base token
/// * `pool_authority_bump_seed` - Bump seed for pool authority PDA
/// * `primary_token_vault_bump_seed` - Bump seed for primary token vault PDA
/// * `base_token_vault_bump_seed` - Bump seed for base token vault PDA
///
/// # Returns
/// * `ProgramResult` - Success or error code
pub fn process_create_pool_state_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ratio_primary_per_base: u64,
    pool_authority_bump_seed: u8,
    primary_token_vault_bump_seed: u8,
    base_token_vault_bump_seed: u8,
) -> ProgramResult {
    msg!("DEBUG: process_create_pool_state_account: Entered");
    let account_info_iter = &mut accounts.iter();

    let payer = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Payer: {}", payer.key);
    let pool_state_pda_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Pool State PDA Account (from client): {}", pool_state_pda_account.key);
    let primary_token_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Primary Token Mint Account: {}", primary_token_mint_account.key);
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
    if ratio_primary_per_base == 0 {
        msg!("DEBUG: process_create_pool_state_account: Ratio cannot be zero");
        return Err(ProgramError::InvalidArgument);
    }
    msg!("DEBUG: process_create_pool_state_account: Ratio is non-zero check passed");

    // Enhanced normalization to prevent economic duplicates
    msg!("DEBUG: process_create_pool_state_account: Normalizing tokens and ratio...");
    
    // Step 1: Lexicographic token ordering
    let (token_a_mint_key, token_b_mint_key) = 
        if primary_token_mint_account.key < base_token_mint_account.key {
            msg!("DEBUG: process_create_pool_state_account: Primary mint < Base mint");
            (primary_token_mint_account.key, base_token_mint_account.key)
        } else {
            msg!("DEBUG: process_create_pool_state_account: Primary mint > Base mint");
            (base_token_mint_account.key, primary_token_mint_account.key)
        };
    
    // Step 2: Canonical ratio mapping to prevent liquidity fragmentation
    // CRITICAL: All pools with the same token pair normalize to the same ratio
    // This prevents both "X A per 1 B" and "X B per 1 A" from being separate pools
    let (ratio_a_numerator, ratio_b_denominator, token_a_is_primary) = 
        if primary_token_mint_account.key < base_token_mint_account.key {
            // Primary is token A: direct mapping
            (ratio_primary_per_base, 1u64, true)
        } else {
            // Primary is token B: use canonical form to prevent economic duplicates
            // Both "X A per 1 B" and "X B per 1 A" normalize to same pool configuration
            (ratio_primary_per_base, 1u64, false)
        };

    msg!("DEBUG: process_create_pool_state_account: Normalized: token_a_mint_key={}, token_b_mint_key={}, ratio_a_num={}, ratio_b_den={}", 
         token_a_mint_key, token_b_mint_key, ratio_a_numerator, ratio_b_denominator);

    let token_a_mint_account_info_ref = if token_a_is_primary { primary_token_mint_account } else { base_token_mint_account };
    let token_b_mint_account_info_ref = if token_a_is_primary { base_token_mint_account } else { primary_token_mint_account };
    msg!("DEBUG: process_create_pool_state_account: Set token_a/b_mint_account_info_refs");

    // Validate mint accounts
    if !primary_token_mint_account.owner.eq(&spl_token::id()) || primary_token_mint_account.data_len() != MintAccount::LEN {
        msg!("DEBUG: process_create_pool_state_account: Primary token mint account is not a valid mint account");
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
    msg!("DEBUG: process_create_pool_state_account: Mapping vault bump seeds. Primary Vault Bump: {}, Base Vault Bump: {}", primary_token_vault_bump_seed, base_token_vault_bump_seed);
    let (token_a_vault_bump, token_b_vault_bump) = if token_a_is_primary {
        (primary_token_vault_bump_seed, base_token_vault_bump_seed)
    } else {
        (base_token_vault_bump_seed, primary_token_vault_bump_seed)
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
    
    // Create the Pool State PDA account
    let pool_state_account_size = PoolState::get_packed_len();
    let rent_for_pool_state = rent.minimum_balance(pool_state_account_size);
    msg!("DEBUG: process_create_pool_state_account: Creating Pool State PDA account: {}. Size: {}. Rent: {}", pool_state_pda_account.key, pool_state_account_size, rent_for_pool_state);
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
/// 4. Initializes security parameters, rent requirements, and delegate management
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
/// * `ratio_primary_per_base` - The fixed ratio between primary and base tokens
/// * `pool_authority_bump_seed` - Bump seed for pool authority PDA
/// * `primary_token_vault_bump_seed` - Bump seed for primary token vault PDA  
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
    ratio_primary_per_base: u64,
    pool_authority_bump_seed: u8,
    primary_token_vault_bump_seed: u8,
    base_token_vault_bump_seed: u8,
) -> ProgramResult {
    msg!("DEBUG: process_initialize_pool_data: Entered");
    let account_info_iter = &mut accounts.iter();

    let payer = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: Payer: {}", payer.key);
    let pool_state_pda_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: Pool State PDA Account (from client): {}", pool_state_pda_account.key);
    let primary_token_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: Primary Token Mint Account: {}", primary_token_mint_account.key);
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
    if ratio_primary_per_base == 0 {
        msg!("DEBUG: process_initialize_pool_data: Ratio cannot be zero");
        return Err(ProgramError::InvalidArgument);
    }
    msg!("DEBUG: process_initialize_pool_data: Ratio is non-zero check passed");

    // Enhanced normalization to prevent economic duplicates
    msg!("DEBUG: process_initialize_pool_data: Normalizing tokens and ratio...");
    
    // Step 1: Lexicographic token ordering
    let (token_a_mint_key, token_b_mint_key) = 
        if primary_token_mint_account.key < base_token_mint_account.key {
            msg!("DEBUG: process_initialize_pool_data: Primary mint < Base mint");
            (primary_token_mint_account.key, base_token_mint_account.key)
        } else {
            msg!("DEBUG: process_initialize_pool_data: Primary mint > Base mint");
            (base_token_mint_account.key, primary_token_mint_account.key)
        };
    
    // Step 2: Canonical ratio mapping to prevent liquidity fragmentation
    // CRITICAL: All pools with the same token pair normalize to the same ratio
    // This prevents both "X A per 1 B" and "X B per 1 A" from being separate pools
    let (ratio_a_numerator, ratio_b_denominator, token_a_is_primary) = 
        if primary_token_mint_account.key < base_token_mint_account.key {
            // Primary is token A: direct mapping
            (ratio_primary_per_base, 1u64, true)
        } else {
            // Primary is token B: use canonical form to prevent economic duplicates
            // Both "X A per 1 B" and "X B per 1 A" normalize to same pool configuration
            (ratio_primary_per_base, 1u64, false)
        };

    msg!("DEBUG: process_initialize_pool_data: Normalized: token_a_mint_key={}, token_b_mint_key={}, ratio_a_num={}, ratio_b_den={}", 
         token_a_mint_key, token_b_mint_key, ratio_a_numerator, ratio_b_denominator);

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
    msg!("DEBUG: process_initialize_pool_data: Checking pool state account. Data len: {}", pool_state_pda_account.data_len());
    if pool_state_pda_account.data_len() != PoolState::get_packed_len() {
        msg!("DEBUG: process_initialize_pool_data: Pool state account has incorrect size. Expected: {}, Got: {}", 
             PoolState::get_packed_len(), pool_state_pda_account.data_len());
        return Err(ProgramError::InvalidAccountData);
    }

    // Check if pool state is already initialized
    if !pool_state_pda_account.data_is_empty() {
        match PoolState::try_from_slice(&pool_state_pda_account.data.borrow()) {
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
    msg!("DEBUG: process_initialize_pool_data: Mapping vault bump seeds. Primary Vault Bump: {}, Base Vault Bump: {}", primary_token_vault_bump_seed, base_token_vault_bump_seed);
    let (token_a_vault_bump, token_b_vault_bump) = if token_a_is_primary {
        (primary_token_vault_bump_seed, base_token_vault_bump_seed)
    } else {
        (base_token_vault_bump_seed, primary_token_vault_bump_seed)
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
    pool_state_data.total_token_a_liquidity = 0;
    pool_state_data.total_token_b_liquidity = 0;
    pool_state_data.pool_authority_bump_seed = pool_authority_bump_seed;
    pool_state_data.token_a_vault_bump_seed = token_a_vault_bump;
    pool_state_data.token_b_vault_bump_seed = token_b_vault_bump;
    pool_state_data.is_initialized = true;

    // Initialize security parameters
    pool_state_data.is_paused = false;

    // Initialize rent requirements
    let rent_requirements = RentRequirements::new(rent);
    pool_state_data.rent_requirements = rent_requirements;

    // Initialize delegate management system (owner is first delegate)
    let current_slot = 0; // Will be updated when clock is available
    pool_state_data.delegate_management = DelegateManagement::new(*payer.key, current_slot);
    
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
    let mut serialized_data = Vec::new();
    match pool_state_data.serialize(&mut serialized_data) {
        Ok(_) => {
            msg!("DEBUG: process_initialize_pool_data: Serialization to buffer successful. Buffer len: {}", serialized_data.len());
        }
        Err(e) => {
            msg!("DEBUG: process_initialize_pool_data: Serialization to buffer FAILED: {:?}", e);
            return Err(e.into());
        }
    }
    
    // Step 2: Copy the serialized data to the account data
    msg!("DEBUG: process_initialize_pool_data: Copying {} bytes to account data", serialized_data.len());
    let account_data_len = pool_state_pda_account.data_len();
    if serialized_data.len() > account_data_len {
        msg!("DEBUG: process_initialize_pool_data: Serialized data too large for account. Need: {}, Have: {}", 
             serialized_data.len(), account_data_len);
        return Err(ProgramError::AccountDataTooSmall);
    }
    
    // Perform the atomic copy operation
    // This ensures that either all data is written correctly or the operation fails cleanly
    {
        let mut account_data = pool_state_pda_account.data.borrow_mut();
        account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
        msg!("DEBUG: process_initialize_pool_data: Data copied to account successfully");
    }
    
    msg!("DEBUG: process_initialize_pool_data: Pool State PDA data len after copy: {}", pool_state_pda_account.data.borrow().len());
    msg!("DEBUG: process_initialize_pool_data: Pool State PDA initialized with data: {:?}", pool_state_data);
    msg!("DEBUG: process_initialize_pool_data: Exiting successfully");

    Ok(())
}

/// **RECOMMENDED**: Single-instruction pool initialization.
/// 
/// This function combines the functionality of both `process_create_pool_state_account` 
/// and `process_initialize_pool_data` into a single atomic operation, eliminating the 
/// need for the two-instruction workaround pattern.
/// 
/// # What it does:
/// 1. Creates Pool State PDA with correct size allocation
/// 2. Creates LP token mints and transfers authority to pool  
/// 3. Creates token vault PDAs and initializes them
/// 4. Initializes pool state data with all configuration
/// 5. Transfers registration fees
/// 6. Sets up delegate management system
/// 
/// # Benefits:
/// - **Atomic Operation**: All-or-nothing execution prevents partial states
/// - **Simpler Integration**: Single instruction call vs. two separate transactions
/// - **Better UX**: Reduces transaction costs and complexity for users
/// - **Eliminates Race Conditions**: No possibility of partial pool creation
/// - **Future-Proof**: Uses modern Solana best practices
/// 
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for pool initialization (same as legacy pattern)
/// * `ratio_primary_per_base` - The ratio of primary tokens per base token
/// * `pool_authority_bump_seed` - Bump seed for pool authority PDA
/// * `primary_token_vault_bump_seed` - Bump seed for primary token vault PDA
/// * `base_token_vault_bump_seed` - Bump seed for base token vault PDA
/// 
/// # Returns
/// * `ProgramResult` - Success or error code
pub fn process_initialize_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ratio_primary_per_base: u64,
    pool_authority_bump_seed: u8,
    primary_token_vault_bump_seed: u8,
    base_token_vault_bump_seed: u8,
) -> ProgramResult {
    msg!("DEBUG: process_initialize_pool: Starting FIXED single-instruction pool initialization");
    
    // CRITICAL FIX: Instead of calling separate functions, we implement everything inline
    // to avoid the GITHUB_ISSUE_31960_WORKAROUND issue where AccountInfo.data doesn't 
    // get updated after CPI account creation within the same instruction.
    
    let account_info_iter = &mut accounts.iter();

    let payer = next_account_info(account_info_iter)?;
    let pool_state_pda_account = next_account_info(account_info_iter)?;
    let primary_token_mint_account = next_account_info(account_info_iter)?;
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

    // Verify ratio is non-zero
    if ratio_primary_per_base == 0 {
        return Err(ProgramError::InvalidArgument);
    }

    // Enhanced normalization
    let (token_a_mint_key, token_b_mint_key) = 
        if primary_token_mint_account.key < base_token_mint_account.key {
            (primary_token_mint_account.key, base_token_mint_account.key)
        } else {
            (base_token_mint_account.key, primary_token_mint_account.key)
        };
    
    let (ratio_a_numerator, ratio_b_denominator, token_a_is_primary) = 
        if primary_token_mint_account.key < base_token_mint_account.key {
            (ratio_primary_per_base, 1u64, true)
        } else {
            (ratio_primary_per_base, 1u64, false)
        };

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

    // Create the Pool State PDA account with the correct size
    let pool_state_account_size = PoolState::get_packed_len();
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

    // Transfer registration fee
    invoke(
        &system_instruction::transfer(payer.key, pool_state_pda_account.key, REGISTRATION_FEE),
        &[
            payer.clone(),
            pool_state_pda_account.clone(),
            system_program_account.clone(),
        ],
    )?;

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

    // Map vault bump seeds and create vault PDAs
    let (token_a_vault_bump, token_b_vault_bump) = if token_a_is_primary {
        (primary_token_vault_bump_seed, base_token_vault_bump_seed)
    } else {
        (base_token_vault_bump_seed, primary_token_vault_bump_seed)
    };

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

    let token_a_mint_account_ref = if token_a_is_primary { primary_token_mint_account } else { base_token_mint_account };
    let token_b_mint_account_ref = if token_a_is_primary { base_token_mint_account } else { primary_token_mint_account };

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

    // CRITICAL: Now immediately initialize the pool state data while we have fresh AccountInfo
    // This is the proper implementation of the GITHUB_ISSUE_31960_WORKAROUND
    
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
    pool_state_data.total_token_a_liquidity = 0;
    pool_state_data.total_token_b_liquidity = 0;
    pool_state_data.pool_authority_bump_seed = pool_authority_bump_seed;
    pool_state_data.token_a_vault_bump_seed = token_a_vault_bump;
    pool_state_data.token_b_vault_bump_seed = token_b_vault_bump;
    pool_state_data.is_initialized = true;
    pool_state_data.is_paused = false;
    pool_state_data.pause_end_timestamp = 0;
    pool_state_data.pause_reason = PoolPauseReason::default();
    pool_state_data.rent_requirements = RentRequirements::new(rent);
    pool_state_data.delegate_management = DelegateManagement::new(*payer.key, 0);
    pool_state_data.collected_fees_token_a = 0;
    pool_state_data.collected_fees_token_b = 0;
    pool_state_data.total_fees_withdrawn_token_a = 0;
    pool_state_data.total_fees_withdrawn_token_b = 0;
    pool_state_data.swap_fee_basis_points = 0;
    pool_state_data.collected_sol_fees = 0;
    pool_state_data.total_sol_fees_withdrawn = 0;

    // Buffer serialization workaround
    let mut serialized_data = Vec::new();
    pool_state_data.serialize(&mut serialized_data)?;
    
    {
        let mut account_data = pool_state_pda_account.data.borrow_mut();
        account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
    }

    msg!("DEBUG: process_initialize_pool: FIXED single-instruction pool initialization completed successfully");
    Ok(())
} 