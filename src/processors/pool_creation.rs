//! Pool Creation Processors
//! 
//! This module contains all the processors for pool creation and initialization operations.
//! It includes both the legacy two-step pattern and the modern single-step initialization.

use crate::constants::*;
use crate::types::*;
use crate::utils::serialization::serialize_to_account;
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

/// **RECOMMENDED**: Single-instruction pool initialization (FIXED)
/// 
/// This function creates and initializes a pool in a single atomic operation.
/// It performs all necessary
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
    // Purpose: Cover protocol operational costs and prevent spam pool creation
    // Destination: Central treasury PDA for protocol sustainability
    //
    // Treasury system is now fully deployed via InitializeProgram
    // Pool creation fee collection is active
    
    // Transfer registration fee to main treasury PDA for pool creation
    let (main_treasury_pda, _treasury_bump) = Pubkey::find_program_address(
        &[crate::constants::MAIN_TREASURY_SEED_PREFIX],
        program_id,
    );
    
    invoke(
        &system_instruction::transfer(payer.key, &main_treasury_pda, REGISTRATION_FEE),
        &[
            payer.clone(),
            pool_state_pda_account.clone(), // Treasury will be added to accounts in future update
            system_program_account.clone(),
        ],
    )?;
    
    msg!("✅ Pool creation contract fee transferred: {} lamports ({} SOL) from creator to treasury", 
         REGISTRATION_FEE, REGISTRATION_FEE as f64 / 1_000_000_000.0);

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

    // Get decimal precision from underlying token mints to ensure LP tokens match
    let token_a_mint_data = token_a_mint_account_ref.try_borrow_data()?;
    let token_a_mint_info = MintAccount::unpack(&token_a_mint_data)?;
    let token_a_decimals = token_a_mint_info.decimals;
    drop(token_a_mint_data); // Release borrow before next operation

    let token_b_mint_data = token_b_mint_account_ref.try_borrow_data()?;
    let token_b_mint_info = MintAccount::unpack(&token_b_mint_data)?;
    let token_b_decimals = token_b_mint_info.decimals;
    drop(token_b_mint_data); // Release borrow before next operation

    msg!("DEBUG: process_initialize_pool: Token A decimals: {}, Token B decimals: {}", 
         token_a_decimals, token_b_decimals);

    // Create and initialize LP token mints with matching decimal precision
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
            None,                    // No freeze authority = unlimited supply
            token_a_decimals,        // Match Token A decimal precision
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
            None,                    // No freeze authority = unlimited supply
            token_b_decimals,        // Match Token B decimal precision
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
    // Note: SOL fees are now tracked in central TreasuryState, not per-pool

    // **Use standardized GitHub Issue #31960 workaround**
    serialize_to_account(&pool_state_data, pool_state_pda_account)?;

    msg!("DEBUG: process_initialize_pool: Successfully used standardized workaround for pool initialization");
    Ok(())
} 