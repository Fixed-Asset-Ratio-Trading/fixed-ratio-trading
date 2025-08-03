//! Pool Creation Processors
//! 
//! This module contains all the processors for pool creation and initialization operations.
//! It includes both the legacy two-step pattern and the modern single-step initialization.

use crate::{
    constants::*,
    error::PoolError,
    state::{MainTreasuryState, PoolState},
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
    state::{Account as TokenAccount, Mint},
};



/// Processes pool initialization with optimized account ordering and fee collection.
/// 
/// This function creates a new trading pool with fixed token ratios using an optimized
/// account structure by removing all placeholder and redundant accounts. This provides
/// maximum efficiency for pool creation operations including LP token mint creation,
/// token vault setup, and pool state initialization.
/// 
/// **BASIS POINTS REFACTOR: Critical Input Assumptions**
/// 
/// This function assumes that all ratio inputs are **already converted to basis points**
/// by the client application. The contract performs **no decimal conversion** internally.
/// 
/// **Client Responsibility:**
/// - Convert display units to basis points before calling this function
/// - Validate one-to-many ratio requirements before submission  
/// - Fetch token decimals from mint accounts for conversion calculations
/// 
/// **Contract Responsibility:**
/// - Fetch token decimals for validation purposes only
/// - Validate that basis point ratios represent whole numbers (one-to-many check)
/// - Store basis point values directly without conversion
/// 
/// **Example Client Conversion:**
/// ```javascript
/// // User wants: "1.0 SOL = 160.0 USDT"
/// const solDecimals = 9; // Fetched from SOL mint account
/// const usdtDecimals = 6; // Fetched from USDT mint account
/// 
/// const ratioABasisPoints = 1.0 * Math.pow(10, solDecimals);   // 1,000,000,000
/// const ratioBBasisPoints = 160.0 * Math.pow(10, usdtDecimals); // 160,000,000
/// 
/// // Send basis points to contract
/// await initializePool(ratioABasisPoints, ratioBBasisPoints);
/// ```
/// 
/// # Arguments
/// * `program_id` - The program ID
/// * `ratio_a_numerator` - Token A ratio in basis points (client-converted)
/// * `ratio_b_denominator` - Token B ratio in basis points (client-converted)
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
    
    // 🚨 CRITICAL SECURITY FIX: Validate user authority is a signer
    use crate::utils::validation::validate_signer;
    validate_signer(user_authority_signer, "User authority")?;
    
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
    
    // 🔧 FIX: Read decimals from underlying token mints to ensure LP tokens match
    let token_a_decimals = Mint::unpack_from_slice(&token_a_mint_account.data.borrow())?.decimals;
    let token_b_decimals = Mint::unpack_from_slice(&token_b_mint_account.data.borrow())?.decimals;
    
    // Consolidated pool creation summary (single message block)
    msg!("🏊 POOL CREATION | TokenA decimals: {} | TokenB decimals: {} | Registration: {} SOL", 
         token_a_decimals, token_b_decimals, REGISTRATION_FEE as f64 / 1_000_000_000.0);
    
    msg!("Processing InitializePool with fixed system pause validation");
    
    // ✅ COMPUTE OPTIMIZATION: No account length verification
    // Solana runtime automatically fails with NotEnoughAccountKeys when accessing
    // accounts[N] if insufficient accounts are provided. Manual length checks are
    // redundant and waste compute units on every function call.
    
    // ✅ SECURITY: User signer validation now properly implemented above
    // Critical security fix: Explicit signer checks are required for user operations
    // to prevent unauthorized pool creation and fee charges.

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
    treasury_state.add_pool_creation_fee(REGISTRATION_FEE, current_timestamp);
    treasury_state.sync_balance_with_account(main_treasury_pda.lamports());
    
    // Save updated treasury state
    let serialized_data = treasury_state.try_to_vec()?;
    main_treasury_pda.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);

    // Token normalization: Always store tokens in lexicographic order (Token A < Token B)
    let (token_a_mint_key, token_b_mint_key) = 
        if token_a_mint_account.key < token_b_mint_account.key {
            (token_a_mint_account.key, token_b_mint_account.key)
        } else {
            (token_b_mint_account.key, token_a_mint_account.key)
        };

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
            pool_state_pda.key,
            None,
            token_a_decimals, // 🔧 FIX: Use Token A decimals instead of hardcoded 6
        )?,
        &[
            lp_token_a_mint_pda.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds], // Pool state PDA signs as mint authority
    )?;

    // Create LP Token B mint account
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
            pool_state_pda.key,
            None,
            token_b_decimals, // 🔧 FIX: Use Token B decimals instead of hardcoded 6
        )?,
        &[
            lp_token_b_mint_pda.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds], // Pool state PDA signs as mint authority
    )?;



    // ✅ EXTRACT TOKEN DECIMALS: Extract decimals from token mint accounts for one-to-many ratio calculation
    let token_a_mint_data = token_a_mint_account.try_borrow_data()?;
    let token_a_mint = spl_token::state::Mint::unpack(&token_a_mint_data)?;
    let token_a_decimals_v2 = token_a_mint.decimals;
    
    let token_b_mint_data = token_b_mint_account.try_borrow_data()?;
    let token_b_mint = spl_token::state::Mint::unpack(&token_b_mint_data)?;
    let token_b_decimals_v2 = token_b_mint.decimals;
    
    // Check for variable shadowing issues
    if token_a_decimals != token_a_decimals_v2 {
        msg!("🚨 WARNING: Token A decimals mismatch! {} vs {}", token_a_decimals, token_a_decimals_v2);
    }
    if token_b_decimals != token_b_decimals_v2 {
        msg!("🚨 WARNING: Token B decimals mismatch! {} vs {}", token_b_decimals, token_b_decimals_v2);
    }

    // ✅ FIXED: Determine correct decimals for one-to-many ratio calculation
    // The ratios have been normalized, so we need to determine which token is which
    // based on the same logic used in normalize_pool_config
    let token_a_is_the_multiple = token_a_mint_account.key.to_bytes() < token_b_mint_account.key.to_bytes();
    
    // Determine which decimals correspond to which ratio after normalization
    let (ratio_a_decimals, ratio_b_decimals) = if token_a_is_the_multiple {
        // Token A is the multiple token (abundant) - use token A decimals for ratio A
        (token_a_decimals_v2, token_b_decimals_v2)
    } else {
        // Token B is the multiple token (abundant) - ratios were swapped during normalization
        // So ratio A now corresponds to token B decimals, and ratio B corresponds to token A decimals
        (token_b_decimals_v2, token_a_decimals_v2)
    };

    // ✅ ONE-TO-MANY RATIO FLAG: Determine if this pool qualifies for the one-to-many ratio flag
    // This flag is set when one or both tokens have a ratio value of exactly 1 (whole token)
    // and both ratios represent whole numbers only (no fractional amounts)
    
    // 🔧 BASIS POINTS REFACTOR: Input ratios are already in basis points (client responsibility)
    // Contract fetches decimals for validation only, no conversion needed
    let is_one_to_many_ratio = check_one_to_many_ratio(
        ratio_a_numerator,     // Already in basis points
        ratio_b_denominator,   // Already in basis points
        ratio_a_decimals,      // Correct decimals for ratio A after normalization
        ratio_b_decimals       // Correct decimals for ratio B after normalization
    );

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
        flags: if is_one_to_many_ratio { 
            crate::constants::POOL_FLAG_ONE_TO_MANY_RATIO 
        } else { 
            0 
        },
        
        // **NEW: CONFIGURABLE CONTRACT FEES** - Initialize with current constants
        contract_liquidity_fee: crate::constants::DEPOSIT_WITHDRAWAL_FEE,
        swap_contract_fee: crate::constants::SWAP_CONTRACT_FEE,
        
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
        
        // **NEW: POOL-SPECIFIC LIMITS** - Initialize to 0 (no limits)
        max_swap_amount: 0,         // 0 = no limit
        min_swap_amount: 0,         // 0 = no minimum
        max_deposit_amount: 0,      // 0 = no limit
        min_deposit_amount: 0,      // 0 = no minimum
        max_withdrawal_amount: 0,   // 0 = no limit
        min_withdrawal_amount: 0,   // 0 = no minimum
        _reserved: [0; 4],          // Reserved for future use
    };

    // Serialize pool state to account
    serialize_to_account(&pool_state, pool_state_pda)?;
    
    // ✅ POOL ID: Emit the unique pool identifier for easy client parsing
    msg!("🎯 POOL_ID: {} | Ratio: {}:{} | OneToMany: {}", 
         pool_state_pda.key, ratio_a_numerator, ratio_b_denominator, is_one_to_many_ratio);
    
    Ok(())
} 