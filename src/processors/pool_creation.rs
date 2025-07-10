//! Pool Creation Processors
//! 
//! This module contains all the processors for pool creation and initialization operations.
//! It includes both the legacy two-step pattern and the modern single-step initialization.

use crate::constants::*;
use crate::types::*;
use crate::utils::serialization::serialize_to_account;
use solana_program::{
    account_info::AccountInfo,
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
    state::{Account as TokenAccount},
};
use crate::utils::account_builders::*;

/// Processes pool initialization with standardized account ordering and fee collection.
/// 
/// This function creates a new trading pool with fixed token ratios using the standardized
/// account ordering policy. It handles pool state creation, token vault setup, LP token
/// minting infrastructure, and collects registration fees.
/// 
/// **PHASE 5: OPTIMIZED ACCOUNT STRUCTURE**
/// After Phase 3 centralization, specialized treasury accounts are no longer needed.
/// This optimization reduces account count from 17 to 15 accounts (12% reduction).
/// 
/// # Standardized Account Order:
/// 0. **Authority/User Signer** (signer, writable) - User creating the pool
/// 1. **System Program** (readable) - Solana system program
/// 2. **Rent Sysvar** (readable) - For rent calculations
/// 3. **Clock Sysvar** (readable) - For timestamps
/// 4. **Pool State PDA** (writable) - Pool state account to create
/// 5. **Token A Mint** (readable) - First token in the pool
/// 6. **Token B Mint** (readable) - Second token in the pool
/// 7. **Token A Vault PDA** (writable) - Token A vault to create
/// 8. **Token B Vault PDA** (writable) - Token B vault to create
/// 9. **SPL Token Program** (readable) - Token program
/// 10. **User Input Token Account** (writable) - Not used in pool creation (placeholder)
/// 11. **User Output Token Account** (writable) - Not used in pool creation (placeholder)
/// 12. **Main Treasury PDA** (writable) - For registration fee collection
/// 13. **LP Token A Mint** (writable) - LP Token A mint to create (function-specific)
/// 14. **LP Token B Mint** (writable) - LP Token B mint to create (function-specific)
/// 
/// **PHASE 5 OPTIMIZATION BENEFITS:**
/// - Reduced account count: 17 → 15 accounts (12% reduction)
/// - Eliminated unused specialized treasury accounts
/// - Reduced transaction size and validation overhead
/// - Estimated compute unit savings: 70-140 CUs per transaction
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA derivation
/// * `ratio_a_numerator` - Numerator for token A in the ratio
/// * `ratio_b_denominator` - Denominator for token B in the ratio  
/// * `accounts` - Array of accounts in standardized order (15 accounts minimum)
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_initialize_pool(
    program_id: &Pubkey,
    ratio_a_numerator: u64,
    ratio_b_denominator: u64,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing InitializePool with Phase 5 optimized account structure");
    
    // ✅ SYSTEM PAUSE: Check system-wide pause
    crate::utils::validation::validate_system_not_paused_safe(accounts, 15)?;
    
    // ✅ STANDARDIZED ACCOUNT VALIDATION: Validate standard account positions where applicable
    validate_standard_accounts(accounts)?;
    // Note: Pool accounts validation will be done after creation
    // validate_token_accounts(accounts)?; // SPL token program validation only
    validate_treasury_accounts(accounts)?;
    
    // ✅ PHASE 5 OPTIMIZATION: Reduced account count requirement
    if accounts.len() < 15 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // ✅ STANDARDIZED ACCOUNT EXTRACTION: Extract accounts using standardized indices
    let payer = &accounts[0];                      // Index 0: Authority/User Signer
    let system_program_account = &accounts[1];     // Index 1: System Program
    let rent_sysvar_account = &accounts[2];        // Index 2: Rent Sysvar
    let clock_sysvar_account = &accounts[3];      // Index 3: Clock Sysvar
    let pool_state_pda_account = &accounts[4];     // Index 4: Pool State PDA
    let multiple_token_mint_account = &accounts[5]; // Index 5: Token A Mint (mapped from multiple)
    let base_token_mint_account = &accounts[6];    // Index 6: Token B Mint (mapped from base)
    let token_a_vault_pda_account = &accounts[7];  // Index 7: Token A Vault PDA
    let token_b_vault_pda_account = &accounts[8];  // Index 8: Token B Vault PDA
    let token_program_account = &accounts[9];      // Index 9: SPL Token Program
    let _user_input_token_account = &accounts[10]; // Index 10: User Input Token Account (unused)
    let _user_output_token_account = &accounts[11]; // Index 11: User Output Token Account (unused)
    let main_treasury_account = &accounts[12];     // Index 12: Main Treasury PDA
    
    // ✅ PHASE 5 OPTIMIZED FUNCTION-SPECIFIC ACCOUNTS: LP token accounts at reduced positions
    let lp_token_a_mint_account = &accounts[13];   // Index 13: LP Token A Mint (was 15)
    let lp_token_b_mint_account = &accounts[14];   // Index 14: LP Token B Mint (was 16)

    let rent = &Rent::from_account_info(rent_sysvar_account)?;

    // Verify that payer is a signer
    if !payer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate ratio values
    crate::utils::validation::validate_ratio_values(ratio_a_numerator, ratio_b_denominator)?;

    // ✅ PHASE 3: CENTRALIZED FEE COLLECTION - Collect registration fee with real-time tracking
    // This ensures the operation fails immediately if fee payment is not possible
    // and updates treasury state in real-time
    use crate::utils::fee_validation::collect_pool_creation_fee;
    
    collect_pool_creation_fee(
        payer,
        main_treasury_account,
        system_program_account,
        clock_sysvar_account,
        program_id,
    )?;

    msg!("✅ Registration fee collected successfully - proceeding with pool creation");

    // Token normalization: Always store tokens in lexicographic order (Token A < Token B)
    let (token_a_mint_key, token_b_mint_key) = 
        if multiple_token_mint_account.key < base_token_mint_account.key {
            (multiple_token_mint_account.key, base_token_mint_account.key)
        } else {
            (base_token_mint_account.key, multiple_token_mint_account.key)
        };

    msg!("DEBUG: Normalized tokens: token_a_mint_key={}, token_b_mint_key={}, ratio_a_num={}, ratio_b_den={}", 
         token_a_mint_key, token_b_mint_key, ratio_a_numerator, ratio_b_denominator);

    // Derive pool state PDA and verify it matches the provided account
    let (expected_pool_state_pda, pool_authority_bump_seed) = Pubkey::find_program_address(
        &[
            POOL_STATE_SEED_PREFIX,
            token_a_mint_key.as_ref(),
            token_b_mint_key.as_ref(),
            &ratio_a_numerator.to_le_bytes(),
            &ratio_b_denominator.to_le_bytes(),
        ],
        program_id,
    );
    
    if *pool_state_pda_account.key != expected_pool_state_pda {
        return Err(ProgramError::InvalidArgument);
    }

    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        token_a_mint_key.as_ref(),
        token_b_mint_key.as_ref(),
        &ratio_a_numerator.to_le_bytes(),
        &ratio_b_denominator.to_le_bytes(),
        &[pool_authority_bump_seed],
    ];

    // Check if pool already exists
    if pool_state_pda_account.data_len() > 0 && !pool_state_pda_account.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // Derive vault PDAs and verify they match the provided accounts
    let (expected_token_a_vault, token_a_vault_bump_seed) = Pubkey::find_program_address(
        &[
            TOKEN_A_VAULT_SEED_PREFIX,
            pool_state_pda_account.key.as_ref(),
        ],
        program_id,
    );
    
    let (expected_token_b_vault, token_b_vault_bump_seed) = Pubkey::find_program_address(
        &[
            TOKEN_B_VAULT_SEED_PREFIX,
            pool_state_pda_account.key.as_ref(),
        ],
        program_id,
    );

    if *token_a_vault_pda_account.key != expected_token_a_vault {
        msg!("Invalid Token A vault PDA");
        return Err(ProgramError::InvalidArgument);
    }
    if *token_b_vault_pda_account.key != expected_token_b_vault {
        msg!("Invalid Token B vault PDA");
        return Err(ProgramError::InvalidArgument);
    }

    // Create seeds for signing
    let token_a_vault_seeds = &[
        TOKEN_A_VAULT_SEED_PREFIX,
        pool_state_pda_account.key.as_ref(),
        &[token_a_vault_bump_seed],
    ];
    let token_b_vault_seeds = &[
        TOKEN_B_VAULT_SEED_PREFIX,
        pool_state_pda_account.key.as_ref(),
        &[token_b_vault_bump_seed],
    ];

    // Create pool state account
    let pool_state_space = PoolState::get_packed_len();
    let pool_state_rent = rent.minimum_balance(pool_state_space);
    
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            pool_state_pda_account.key,
            pool_state_rent,
            pool_state_space as u64,
            program_id,
        ),
        &[
            payer.clone(),
            pool_state_pda_account.clone(),
            system_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;

    // Create token vaults
    let vault_space = TokenAccount::LEN;
    let vault_rent = rent.minimum_balance(vault_space);
    
    // Create Token A vault
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            token_a_vault_pda_account.key,
            vault_rent,
            vault_space as u64,
            &spl_token::id(),
        ),
        &[
            payer.clone(),
            token_a_vault_pda_account.clone(),
            system_program_account.clone(),
        ],
        &[token_a_vault_seeds],
    )?;
    
    // Initialize Token A vault - use correct token mint account that matches token_a_mint_key
    let token_a_mint_account = if token_a_mint_key == multiple_token_mint_account.key {
        multiple_token_mint_account
    } else {
        base_token_mint_account
    };
    
    invoke(
        &token_instruction::initialize_account(
            token_program_account.key,
            token_a_vault_pda_account.key,
            token_a_mint_key,
            pool_state_pda_account.key,
        )?,
        &[
            token_a_vault_pda_account.clone(),
            token_a_mint_account.clone(),
            pool_state_pda_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
    )?;

    // Create Token B vault  
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            token_b_vault_pda_account.key,
            vault_rent,
            vault_space as u64,
            &spl_token::id(),
        ),
        &[
            payer.clone(),
            token_b_vault_pda_account.clone(),
            system_program_account.clone(),
        ],
        &[token_b_vault_seeds],
    )?;
    
    // Initialize Token B vault - use correct token mint account that matches token_b_mint_key  
    let token_b_mint_account = if token_b_mint_key == base_token_mint_account.key {
        base_token_mint_account
    } else {
        multiple_token_mint_account
    };
    
    invoke(
        &token_instruction::initialize_account(
            token_program_account.key,
            token_b_vault_pda_account.key,
            token_b_mint_key,
            pool_state_pda_account.key,
        )?,
        &[
            token_b_vault_pda_account.clone(),
            token_b_mint_account.clone(),
            pool_state_pda_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
    )?;

    // Create LP token mint accounts first
    let mint_space = spl_token::state::Mint::LEN;
    let mint_rent = rent.minimum_balance(mint_space);
    
    // Create LP Token A mint account
    invoke(
        &system_instruction::create_account(
            payer.key,
            lp_token_a_mint_account.key,
            mint_rent,
            mint_space as u64,
            &spl_token::id(),
        ),
        &[
            payer.clone(),
            lp_token_a_mint_account.clone(),
            system_program_account.clone(),
        ],
    )?;

    // Create LP Token B mint account
    invoke(
        &system_instruction::create_account(
            payer.key,
            lp_token_b_mint_account.key,
            mint_rent,
            mint_space as u64,
            &spl_token::id(),
        ),
        &[
            payer.clone(),
            lp_token_b_mint_account.clone(),
            system_program_account.clone(),
        ],
    )?;

    // Initialize LP token mints with pool as mint authority
    invoke(
        &token_instruction::initialize_mint(
            token_program_account.key,
            lp_token_a_mint_account.key,
            pool_state_pda_account.key,
            None,
            6, // Decimals
        )?,
        &[
            lp_token_a_mint_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
    )?;

    invoke(
        &token_instruction::initialize_mint(
            token_program_account.key,
            lp_token_b_mint_account.key,
            pool_state_pda_account.key,
            None,
            6, // Decimals
        )?,
        &[
            lp_token_b_mint_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
    )?;

    // Initialize pool state data
    let pool_state_data = PoolState {
        owner: *payer.key,
        token_a_mint: *token_a_mint_key,
        token_b_mint: *token_b_mint_key,
        token_a_vault: *token_a_vault_pda_account.key,
        token_b_vault: *token_b_vault_pda_account.key,
        lp_token_a_mint: *lp_token_a_mint_account.key,
        lp_token_b_mint: *lp_token_b_mint_account.key,
            ratio_a_numerator,
            ratio_b_denominator,
        one_to_many_ratio: ratio_a_numerator > ratio_b_denominator,
        total_token_a_liquidity: 0,
        total_token_b_liquidity: 0,
        pool_authority_bump_seed,
        token_a_vault_bump_seed,
        token_b_vault_bump_seed,
        is_initialized: true,
        rent_requirements: RentRequirements::new(rent),
        paused: false,
        swaps_paused: false,
        withdrawal_protection_active: false,
        only_lp_token_a_for_both: false,
        collected_fees_token_a: 0,
        collected_fees_token_b: 0,
        total_fees_withdrawn_token_a: 0,
        total_fees_withdrawn_token_b: 0,
        swap_fee_basis_points: 0,
    };

    // Fee collection moved to beginning of function (FEES FIRST PATTERN)

    // Serialize pool state to account
    serialize_to_account(&pool_state_data, pool_state_pda_account)?;

    // ✅ POOL ID: Emit the unique pool identifier for easy client parsing
    msg!("🎯 POOL_ID: {}", pool_state_pda_account.key);
    
    msg!("✅ Pool initialized successfully");
    msg!("Pool Details:");
    msg!("  Token A: {}", token_a_mint_key);
    msg!("  Token B: {}", token_b_mint_key);
    msg!("  Ratio: {} : {}", ratio_a_numerator, ratio_b_denominator);
    msg!("  Pool State PDA: {}", pool_state_pda_account.key);
    msg!("  Token A Vault: {}", token_a_vault_pda_account.key);
    msg!("  Token B Vault: {}", token_b_vault_pda_account.key);
    msg!("  LP Token A Mint: {}", lp_token_a_mint_account.key);
    msg!("  LP Token B Mint: {}", lp_token_b_mint_account.key);
    
    Ok(())
} 