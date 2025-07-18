//! Pool Creation Processors
//! 
//! This module contains all the processors for pool creation and initialization operations.
//! It includes both the legacy two-step pattern and the modern single-step initialization.

use crate::{
    constants::*,
    error::PoolError,
    state::{MainTreasuryState, PoolState, pool_state::RentRequirements},
    utils::{serialization::serialize_to_account, validation::check_one_to_many_ratio},
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program::{invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,

    sysvar::rent::Rent,
    program_pack::Pack,
};
use spl_token::{
    instruction as token_instruction,
    state::{Account as TokenAccount},
};

/// Processes pool initialization with optimized account ordering and fee collection.
/// 
/// This function creates a new trading pool with fixed token ratios using an optimized
/// account structure by removing all placeholder and redundant accounts. This provides
/// maximum efficiency for pool creation operations including LP token mint creation,
/// token vault setup, and pool state initialization.
/// 
/// # Arguments
/// * `program_id` - The program ID
/// * `ratio_a_numerator` - Token A base units in the ratio
/// * `ratio_b_denominator` - Token B base units in the ratio  
/// * `accounts` - Array of accounts in secure order (13 accounts minimum)
/// 
/// # Account Info
/// The accounts must be provided in the following order:
/// 0. **User Authority Signer** (signer, writable) - User signer creating the pool
/// 1. **System Program Account** (readable) - Solana system program account
/// 2. **System State PDA** (readable) - System state PDA for pause validation
/// 3. **Pool State PDA** (writable) - Pool state PDA to create
/// 4. **SPL Token Program Account** (readable) - Token program account
/// 5. **Main Treasury PDA** (writable) - For registration fee collection
/// 6. **Rent Sysvar Account** (readable) - For rent calculations
/// 7. **Token A Mint Account** (readable) - First token mint account (will be normalized to A or B)
/// 8. **Token B Mint Account** (readable) - Second token mint account (will be normalized to A or B)
/// 9. **Token A Vault PDA** (writable) - Token A vault PDA to create
/// 10. **Token B Vault PDA** (writable) - Token B vault PDA to create
/// 11. **LP Token A Mint PDA** (writable) - LP Token A mint PDA to create
/// 12. **LP Token B Mint PDA** (writable) - LP Token B mint PDA to create
/// 
/// # Returns
/// * `ProgramResult` - Success or error
/// 
/// # Performance CUs
/// 400,000 CUs    2025/7/15 6:21 pm
/// 
/// # Critical Notes
/// - **FIXED VALIDATION**: Fixed broken system pause validation by including system state account
/// - **LP TOKEN SECURITY**: LP token mints are created as PDAs during pool creation, preventing user manipulation
/// - **PDA VALIDATION**: All PDAs strictly validated against derived addresses (no fake PDAs possible)
/// - **ENHANCED SECURITY**: Enhanced error messages for security violations
/// - **IMMEDIATE AVAILABILITY**: LP token mints immediately available for user token account creation
/// - **DRAINAGE PROTECTION**: Eliminated risk of fake LP tokens being used to drain pools
/// - **COMPLETE CONTROL**: Pool has complete control over LP token minting and burning
/// - **CLIENT INTEGRATION**: Simplified client integration - LP mints exist from pool creation
/// - **NO DELAYS**: No on-demand account creation delays during deposits
/// - **SMART CONTRACT CONTROL**: Complete smart contract control over pool infrastructure creation
pub fn process_initialize_pool(
    program_id: &Pubkey,
    ratio_a_numerator: u64,
    ratio_b_denominator: u64,
    accounts: &[AccountInfo],
) -> ProgramResult {
    // ✅ ACCOUNT EXTRACTION: Extract accounts using updated indices
    let user_authority_signer = &accounts[0];                      // Index 0: User Authority Signer
    let system_program_account = &accounts[1];                     // Index 1: System Program Account
    let system_state_pda = &accounts[2];                           // Index 2: System State PDA
    let pool_state_pda = &accounts[3];                             // Index 3: Pool State PDA
    
    // Validate system is not paused
    crate::utils::validation::validate_system_not_paused_secure(system_state_pda, program_id)?;
    let token_program_account = &accounts[4];                      // Index 4: SPL Token Program Account
    let main_treasury_pda = &accounts[5];                          // Index 5: Main Treasury PDA
    let rent_sysvar_account = &accounts[6];                        // Index 6: Rent Sysvar Account
    let token_a_mint_account = &accounts[7];                       // Index 7: Token A Mint Account
    let token_b_mint_account = &accounts[8];                       // Index 8: Token B Mint Account
    let token_a_vault_pda = &accounts[9];                          // Index 9: Token A Vault PDA
    let token_b_vault_pda = &accounts[10];                         // Index 10: Token B Vault PDA
    let lp_token_a_mint_pda = &accounts[11];                       // Index 11: LP Token A Mint PDA
    let lp_token_b_mint_pda = &accounts[12];                       // Index 12: LP Token B Mint PDA

    let rent = &Rent::from_account_info(rent_sysvar_account)?;
    
    // 🎯 DEFI UX BEST PRACTICES: Comprehensive Transaction Summary
    msg!("🏊 FIXED RATIO POOL CREATION");
    msg!("=============================");
    msg!("💰 TRANSACTION COSTS:");
    msg!("  • Registration Fee: {} SOL", REGISTRATION_FEE as f64 / 1_000_000_000.0);
    msg!("  • Account Rent: ~{} SOL (5 PDA accounts)", 
         (rent.minimum_balance(PoolState::get_packed_len()) + 
          rent.minimum_balance(TokenAccount::LEN) * 2 + 
          rent.minimum_balance(spl_token::state::Mint::LEN) * 2) as f64 / 1_000_000_000.0);
    msg!("  • Total Cost: ~{} SOL", 
         (REGISTRATION_FEE + 
          rent.minimum_balance(PoolState::get_packed_len()) + 
          rent.minimum_balance(TokenAccount::LEN) * 2 + 
          rent.minimum_balance(spl_token::state::Mint::LEN) * 2) as f64 / 1_000_000_000.0);
    msg!("");
    msg!("🎁 WHAT YOU'LL GET:");
    msg!("  • Complete pool infrastructure");
    msg!("  • Ability to add liquidity and earn fees");
    msg!("  • Pool owner privileges and fee collection rights");
    msg!("  • LP token minting/burning capabilities");
    msg!("");
    msg!("📋 ACCOUNTS BEING CREATED:");
    msg!("  • Pool State Account (stores pool configuration)");
    msg!("  • Token A Vault (holds Token A liquidity)");
    msg!("  • Token B Vault (holds Token B liquidity)");
    msg!("  • LP Token A Mint (creates LP tokens for Token A)");
    msg!("  • LP Token B Mint (creates LP tokens for Token B)");
    msg!("=============================");
    
    msg!("Processing InitializePool with fixed system pause validation");
    
    // ✅ COMPUTE OPTIMIZATION: No account length verification
    // Solana runtime automatically fails with NotEnoughAccountKeys when accessing
    // accounts[N] if insufficient accounts are provided. Manual length checks are
    // redundant and waste compute units on every function call.
    
    // ✅ COMPUTE OPTIMIZATION: No redundant signer verification
    // Solana runtime automatically fails with MissingRequiredSignature when
    // invoke() operations require signatures. Manual signer checks are
    // redundant and waste compute units on every function call.

    // Validate ratio values
    crate::utils::validation::validate_ratio_values(ratio_a_numerator, ratio_b_denominator)?;

    // ✅ CENTRALIZED FEE COLLECTION - Collect registration fee with real-time tracking
    // This ensures the operation fails immediately if fee payment is not possible
    // and updates treasury state in real-time
    
    // ✅ OPTIMIZED FEE COLLECTION - Use Clock::get() directly instead of clock sysvar account
    // Since we removed the clock sysvar account, we need to use a different approach for fee collection
    use crate::utils::fee_validation::{validate_fee_payment, validate_treasury_account};
    use solana_program::{program::invoke, system_instruction, clock::Clock, sysvar::Sysvar};
    
    // Get current timestamp directly
    let clock = Clock::get()?;
    let current_timestamp = clock.unix_timestamp;
    
    // Validate fee payment capability
    let validation_result = validate_fee_payment(user_authority_signer, REGISTRATION_FEE, VALIDATION_CONTEXT_POOL_CREATION);
    if !validation_result.is_valid {
        return Err(PoolError::InsufficientFeeBalance {
            required: REGISTRATION_FEE,
            available: validation_result.available_balance,
            account: *user_authority_signer.key,
        }.into());
    }
    
    // Validate treasury account
    let (expected_main_treasury, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        program_id,
    );
    validate_treasury_account(main_treasury_pda, &expected_main_treasury, TREASURY_TYPE_MAIN)?;
    
    msg!("💰 Step 1/6: Collecting registration fee");
    msg!("  Amount: {} SOL", REGISTRATION_FEE as f64 / 1_000_000_000.0);
    
    // Transfer fee to treasury
    let transfer_instruction = system_instruction::transfer(
        user_authority_signer.key,
        main_treasury_pda.key,
        REGISTRATION_FEE,
    );
    
    invoke(
        &transfer_instruction,
        &[
            user_authority_signer.clone(),
            main_treasury_pda.clone(),
            system_program_account.clone(),
        ],
    )?;
    
    // Update treasury state with real-time tracking
    let mut treasury_state = MainTreasuryState::try_from_slice(&main_treasury_pda.data.borrow())?;
    
    // 🔍 DEBUG: Show counter values before and after increment
    msg!("🔍 POOL CREATION COUNTER DEBUG:");
    msg!("   Before increment - pool_creation_count: {}", treasury_state.pool_creation_count);
    msg!("   Before increment - total_pool_creation_fees: {}", treasury_state.total_pool_creation_fees);
    msg!("   Adding fee: {} lamports", REGISTRATION_FEE);
    
    treasury_state.add_pool_creation_fee(REGISTRATION_FEE, current_timestamp);
    treasury_state.sync_balance_with_account(main_treasury_pda.lamports());
    
    msg!("   After increment - pool_creation_count: {}", treasury_state.pool_creation_count);
    msg!("   After increment - total_pool_creation_fees: {}", treasury_state.total_pool_creation_fees);
    msg!("   Total balance after sync: {}", treasury_state.total_balance);
    
    // Save updated treasury state
    let serialized_data = treasury_state.try_to_vec()?;
    main_treasury_pda.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);

    msg!("✅ Registration fee collected successfully - proceeding with pool creation");

    // Token normalization: Always store tokens in lexicographic order (Token A < Token B)
    let (token_a_mint_key, token_b_mint_key) = 
        if token_a_mint_account.key < token_b_mint_account.key {
            (token_a_mint_account.key, token_b_mint_account.key)
        } else {
            (token_b_mint_account.key, token_a_mint_account.key)
        };

    msg!("DEBUG: Normalized tokens: token_a_mint_key={}, token_b_mint_key={}, ratio_a_num={}, ratio_b_den={}", 
         token_a_mint_key, token_b_mint_key, ratio_a_numerator, ratio_b_denominator);

    // ✅ SECURITY: Derive LP token mint PDAs to prevent user manipulation
    let (lp_token_a_mint_pda_address, lp_token_a_mint_bump_seed) = Pubkey::find_program_address(
        &[
            LP_TOKEN_A_MINT_SEED_PREFIX,
            pool_state_pda.key.as_ref(),
        ],
        program_id,
    );
    
    let (lp_token_b_mint_pda_address, lp_token_b_mint_bump_seed) = Pubkey::find_program_address(
        &[
            LP_TOKEN_B_MINT_SEED_PREFIX,
            pool_state_pda.key.as_ref(),
        ],
        program_id,
    );
    
    msg!("DEBUG: LP Token A Mint PDA: {}", lp_token_a_mint_pda_address);
    msg!("DEBUG: LP Token B Mint PDA: {}", lp_token_b_mint_pda_address);

    // ✅ SECURITY: Derive pool state PDA and validate provided account matches
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
    
    if *pool_state_pda.key != expected_pool_state_pda {
        msg!("❌ SECURITY VIOLATION: Pool State PDA does not match expected derived PDA");
        msg!("   Expected: {}", expected_pool_state_pda);
        msg!("   Provided: {}", pool_state_pda.key);
        return Err(ProgramError::InvalidAccountData);
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
    if pool_state_pda.data_len() > 0 && !pool_state_pda.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // ✅ SECURITY: Derive vault PDAs and validate provided accounts match
    let (expected_token_a_vault, token_a_vault_bump_seed) = Pubkey::find_program_address(
        &[
            TOKEN_A_VAULT_SEED_PREFIX,
            pool_state_pda.key.as_ref(),
        ],
        program_id,
    );
    
    let (expected_token_b_vault, token_b_vault_bump_seed) = Pubkey::find_program_address(
        &[
            TOKEN_B_VAULT_SEED_PREFIX,
            pool_state_pda.key.as_ref(),
        ],
        program_id,
    );

    if *token_a_vault_pda.key != expected_token_a_vault {
        msg!("❌ SECURITY VIOLATION: Token A vault PDA does not match expected derived PDA");
        msg!("   Expected: {}", expected_token_a_vault);
        msg!("   Provided: {}", token_a_vault_pda.key);
        return Err(ProgramError::InvalidAccountData);
    }
    if *token_b_vault_pda.key != expected_token_b_vault {
        msg!("❌ SECURITY VIOLATION: Token B vault PDA does not match expected derived PDA");
        msg!("   Expected: {}", expected_token_b_vault);
        msg!("   Provided: {}", token_b_vault_pda.key);
        return Err(ProgramError::InvalidAccountData);
    }

    // ✅ SECURITY: Validate LP token mint PDAs match expected derived addresses
    if *lp_token_a_mint_pda.key != lp_token_a_mint_pda_address {
        msg!("❌ SECURITY VIOLATION: LP Token A mint PDA does not match expected derived PDA");
        msg!("   Expected: {}", lp_token_a_mint_pda_address);
        msg!("   Provided: {}", lp_token_a_mint_pda.key);
        return Err(ProgramError::InvalidAccountData);
    }
    if *lp_token_b_mint_pda.key != lp_token_b_mint_pda_address {
        msg!("❌ SECURITY VIOLATION: LP Token B mint PDA does not match expected derived PDA");
        msg!("   Expected: {}", lp_token_b_mint_pda_address);
        msg!("   Provided: {}", lp_token_b_mint_pda.key);
        return Err(ProgramError::InvalidAccountData);
    }

    msg!("✅ All PDAs validated against derived addresses");

    // Create seeds for signing
    let token_a_vault_seeds = &[
        TOKEN_A_VAULT_SEED_PREFIX,
        pool_state_pda.key.as_ref(),
        &[token_a_vault_bump_seed],
    ];
    let token_b_vault_seeds = &[
        TOKEN_B_VAULT_SEED_PREFIX,
        pool_state_pda.key.as_ref(),
        &[token_b_vault_bump_seed],
    ];
    
    // ✅ SECURITY: Create seeds for LP token mint signing
    let lp_token_a_mint_seeds = &[
        LP_TOKEN_A_MINT_SEED_PREFIX,
        pool_state_pda.key.as_ref(),
        &[lp_token_a_mint_bump_seed],
    ];
    let lp_token_b_mint_seeds = &[
        LP_TOKEN_B_MINT_SEED_PREFIX,
        pool_state_pda.key.as_ref(),
        &[lp_token_b_mint_bump_seed],
    ];

    // Create pool state account
    let pool_state_space = PoolState::get_packed_len();
    let pool_state_rent = rent.minimum_balance(pool_state_space);
    
    msg!("🔨 Step 2/6: Creating Pool State Account");
    msg!("  Cost: {} SOL", pool_state_rent as f64 / 1_000_000_000.0);
    
    invoke_signed(
        &system_instruction::create_account(
            user_authority_signer.key,
            pool_state_pda.key,
            pool_state_rent,
            pool_state_space as u64,
            program_id,
        ),
        &[
            user_authority_signer.clone(),
            pool_state_pda.clone(),
            system_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;

    // Create token vaults
    let vault_space = TokenAccount::LEN;
    let vault_rent = rent.minimum_balance(vault_space);
    
    msg!("🔨 Step 3/6: Creating Token A Vault");
    msg!("  Cost: {} SOL", vault_rent as f64 / 1_000_000_000.0);
    
    // Create Token A vault
    invoke_signed(
        &system_instruction::create_account(
            user_authority_signer.key,
            token_a_vault_pda.key,
            vault_rent,
            vault_space as u64,
            &spl_token::id(),
        ),
        &[
            user_authority_signer.clone(),
            token_a_vault_pda.clone(),
            system_program_account.clone(),
        ],
        &[token_a_vault_seeds],
    )?;
    
    // Initialize Token A vault - use correct token mint account that matches token_a_mint_key
    let token_a_mint_account_ref = if token_a_mint_key == token_a_mint_account.key {
        token_a_mint_account
    } else {
        token_b_mint_account
    };
    
    invoke(
        &token_instruction::initialize_account(
            token_program_account.key,
            token_a_vault_pda.key,
            token_a_mint_key,
            pool_state_pda.key,
        )?,
        &[
            token_a_vault_pda.clone(),
            token_a_mint_account_ref.clone(),
            pool_state_pda.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
    )?;

    // Create Token B vault  
    msg!("🔨 Step 4/6: Creating Token B Vault");
    msg!("  Cost: {} SOL", vault_rent as f64 / 1_000_000_000.0);
    
    invoke_signed(
        &system_instruction::create_account(
            user_authority_signer.key,
            token_b_vault_pda.key,
            vault_rent,
            vault_space as u64,
            &spl_token::id(),
        ),
        &[
            user_authority_signer.clone(),
            token_b_vault_pda.clone(),
            system_program_account.clone(),
        ],
        &[token_b_vault_seeds],
    )?;
    
    // Initialize Token B vault - use correct token mint account that matches token_b_mint_key  
    let token_b_mint_account_ref = if token_b_mint_key == token_b_mint_account.key {
        token_b_mint_account
    } else {
        token_a_mint_account
    };
    
    invoke(
        &token_instruction::initialize_account(
            token_program_account.key,
            token_b_vault_pda.key,
            token_b_mint_key,
            pool_state_pda.key,
        )?,
        &[
            token_b_vault_pda.clone(),
            token_b_mint_account_ref.clone(),
            pool_state_pda.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
    )?;

    // ✅ SECURITY: Create LP token mint accounts as PDAs during pool creation
    // This ensures LP token mints exist immediately and are controlled by the smart contract
    let mint_space = spl_token::state::Mint::LEN;
    let mint_rent = rent.minimum_balance(mint_space);
    
    msg!("🔨 Step 5/6: Creating LP Token A Mint");
    msg!("  Cost: {} SOL", mint_rent as f64 / 1_000_000_000.0);
    msg!("  LP Token A Mint PDA: {}", lp_token_a_mint_pda_address);
    msg!("  LP Token B Mint PDA: {}", lp_token_b_mint_pda_address);

    // Create LP Token A mint account
    invoke_signed(
        &system_instruction::create_account(
            user_authority_signer.key,
            lp_token_a_mint_pda.key,
            mint_rent,
            mint_space as u64,
            &spl_token::id(),
        ),
        &[
            user_authority_signer.clone(),
            lp_token_a_mint_pda.clone(),
            system_program_account.clone(),
        ],
        &[lp_token_a_mint_seeds],
    )?;

    // Initialize LP Token A mint with pool state PDA as authority
    invoke_signed(
        &token_instruction::initialize_mint(
            token_program_account.key,
            lp_token_a_mint_pda.key,
            pool_state_pda.key, // Pool controls minting/burning
            None, // No freeze authority
            6, // 6 decimals for LP tokens
        )?,
        &[
            lp_token_a_mint_pda.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds], // Pool state PDA signs as mint authority
    )?;

    // Create LP Token B mint account
    msg!("🔨 Step 6/6: Creating LP Token B Mint");
    msg!("  Cost: {} SOL", mint_rent as f64 / 1_000_000_000.0);
    
    invoke_signed(
        &system_instruction::create_account(
            user_authority_signer.key,
            lp_token_b_mint_pda.key,
            mint_rent,
            mint_space as u64,
            &spl_token::id(),
        ),
        &[
            user_authority_signer.clone(),
            lp_token_b_mint_pda.clone(),
            system_program_account.clone(),
        ],
        &[lp_token_b_mint_seeds],
    )?;

    // Initialize LP Token B mint with pool state PDA as authority
    invoke_signed(
        &token_instruction::initialize_mint(
            token_program_account.key,
            lp_token_b_mint_pda.key,
            pool_state_pda.key, // Pool controls minting/burning
            None, // No freeze authority
            6, // 6 decimals for LP tokens
        )?,
        &[
            lp_token_b_mint_pda.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds], // Pool state PDA signs as mint authority
    )?;

    msg!("✅ LP token mints created and controlled by smart contract");

    // ✅ EXTRACT TOKEN DECIMALS: Extract decimals from token mint accounts for one-to-many ratio calculation
    let token_a_mint_data = token_a_mint_account.try_borrow_data()?;
    let token_a_mint = spl_token::state::Mint::unpack(&token_a_mint_data)?;
    let token_a_decimals = token_a_mint.decimals;
    
    let token_b_mint_data = token_b_mint_account.try_borrow_data()?;
    let token_b_mint = spl_token::state::Mint::unpack(&token_b_mint_data)?;
    let token_b_decimals = token_b_mint.decimals;

    // ✅ ONE-TO-MANY RATIO FLAG: Determine if this pool qualifies for the one-to-many ratio flag
    // This flag is set when one or both tokens have a ratio value of exactly 1 (whole token)
    // and both ratios represent whole numbers only (no fractional amounts)
    let is_one_to_many_ratio = check_one_to_many_ratio(
        ratio_a_numerator,
        ratio_b_denominator,
        token_a_decimals,
        token_b_decimals
    );
    
    msg!("🔍 One-to-Many Ratio Analysis:");
    msg!("  Token A: {} base units ({} decimals)", ratio_a_numerator, token_a_decimals);
    msg!("  Token B: {} base units ({} decimals)", ratio_b_denominator, token_b_decimals);
    msg!("  Display Ratio: {} : {}", 
         ratio_a_numerator / (10_u64.pow(token_a_decimals as u32)),
         ratio_b_denominator / (10_u64.pow(token_b_decimals as u32)));
    msg!("  One-to-Many Flag: {}", if is_one_to_many_ratio { "✅ SET" } else { "❌ NOT SET" });

    // ✅ POOL STATE: Create pool state with comprehensive configuration
    let pool_state = PoolState {
        owner: *user_authority_signer.key,
        token_a_mint: *token_a_mint_key,
        token_b_mint: *token_b_mint_key,
        token_a_vault: *token_a_vault_pda.key,
        token_b_vault: *token_b_vault_pda.key,
        lp_token_a_mint: lp_token_a_mint_pda_address,
        lp_token_b_mint: lp_token_b_mint_pda_address,
        ratio_a_numerator,
        ratio_b_denominator,
        total_token_a_liquidity: 0,
        total_token_b_liquidity: 0,
        pool_authority_bump_seed,
        token_a_vault_bump_seed,
        token_b_vault_bump_seed,
        lp_token_a_mint_bump_seed,
        lp_token_b_mint_bump_seed,
        rent_requirements: RentRequirements::new(rent),
        flags: if is_one_to_many_ratio { 
            crate::constants::POOL_FLAG_ONE_TO_MANY_RATIO 
        } else { 
            0 
        }, // Set ONE_TO_MANY_RATIO flag based on proper validation logic
        collected_fees_token_a: 0,
        collected_fees_token_b: 0,
        total_fees_withdrawn_token_a: 0,
        total_fees_withdrawn_token_b: 0,
        
        // **PHASE 1: NEW DISTRIBUTED COLLECTION FIELDS**
        collected_liquidity_fees: 0,
        collected_swap_contract_fees: 0,
        
        total_sol_fees_collected: 0,
        last_consolidation_timestamp: 0,
        total_consolidations: 0,
        total_fees_consolidated: 0,
    };

    // Fee collection moved to beginning of function (FEES FIRST PATTERN)

    // Serialize pool state to account
    serialize_to_account(&pool_state, pool_state_pda)?;

    // ✅ POOL ID: Emit the unique pool identifier for easy client parsing
    msg!("🎯 POOL_ID: {}", pool_state_pda.key);
    
    msg!("🎉 POOL CREATION COMPLETED SUCCESSFULLY!");
    msg!("==========================================");
    msg!("✅ INFRASTRUCTURE CREATED:");
    msg!("  • Pool State Account: {}", pool_state_pda.key);
    msg!("  • Token A Vault: {}", token_a_vault_pda.key);
    msg!("  • Token B Vault: {}", token_b_vault_pda.key);
    msg!("  • LP Token A Mint: {}", lp_token_a_mint_pda_address);
    msg!("  • LP Token B Mint: {}", lp_token_b_mint_pda_address);
    msg!("");
    msg!("📊 POOL CONFIGURATION:");
    msg!("  • Token A: {}", token_a_mint_key);
    msg!("  • Token B: {}", token_b_mint_key);
    msg!("  • Fixed Ratio: {} : {}", ratio_a_numerator, ratio_b_denominator);
    msg!("  • Pool Owner: {}", user_authority_signer.key);
    msg!("");
    msg!("🚀 NEXT STEPS:");
    msg!("  • Add liquidity to start earning fees");
    msg!("  • Share pool address with other users");
    msg!("  • Monitor pool activity and fee collection");
    msg!("  • Consider setting up automated liquidity management");
    msg!("==========================================");
    
    Ok(())
} 