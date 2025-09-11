//! Pool Management Processor
//!
//! This module handles all pool-related operations including creation,
//! pause/unpause management, and fee updates.

use crate::{
    constants::*,
    error::PoolError,
    state::{MainTreasuryState, PoolState},
    utils::{
        serialization::serialize_to_account, 
        validation::{
            get_ratio_type,
            validate_and_deserialize_pool_state_secure,
            validate_signer,
        }
    },
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke_signed, invoke},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::rent::Rent,
    program_pack::Pack,
    system_instruction,
    clock::Clock,
    sysvar::Sysvar,
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
/// * `flags` - Pool configuration flags (bitwise OR of supported flags)
///   - Supported bits:
///     - `POOL_FLAG_SWAP_FOR_OWNERS_ONLY` (bit 5, value 32): Restrict swaps to owner-only
///     - `POOL_FLAG_EXACT_EXCHANGE_REQUIRED` (bit 6, value 64): Require exact exchange (no rounding)
///   - Note: Flags are documented here for visibility; they are not yet applied in initialization
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
pub fn process_pool_initialize(
    program_id: &Pubkey,
    ratio_a_numerator: u64,
    ratio_b_denominator: u64,
    flags: u8,
    accounts: &[AccountInfo],
) -> ProgramResult {
    // ✅ ACCOUNT EXTRACTION: Extract accounts using updated indices
    let user_authority_signer = &accounts[0];                      // Index 0: User Authority Signer
    let system_program_account = &accounts[1];                     // Index 1: System Program Account
    let system_state_pda = &accounts[2];                           // Index 2: System State PDA
    let pool_state_pda = &accounts[3];                             // Index 3: Pool State PDA
    
    // 🚨 CRITICAL SECURITY FIX: Validate user authority is a signer
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
    
    // Save updated treasury state with size validation
    let serialized_data = treasury_state.try_to_vec()?;
    if main_treasury_pda.data_len() < serialized_data.len() {
        msg!("🚨 Critical Error: Treasury serialized data too large for account");
        return Err(ProgramError::AccountDataTooSmall);
    }
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

    // ✅ RATIO TYPE CLASSIFICATION: Determine the type of ratio for this pool
    // This classifies the ratio into SimpleRatio, DecimalRatio, or EngineeringRatio
    // based on whether values are whole numbers and if one equals exactly 1.0
    
    // 🔧 BASIS POINTS REFACTOR: Input ratios are already in basis points (client responsibility)
    // Contract fetches decimals for validation only, no conversion needed
    let ratio_type = get_ratio_type(
        ratio_a_numerator,     // Already in basis points
        ratio_b_denominator,   // Already in basis points
        ratio_a_decimals,      // Correct decimals for ratio A after normalization
        ratio_b_decimals       // Correct decimals for ratio B after normalization
    );
    
    // Log the ratio type for debugging
    msg!("📊 Pool ratio classified as: {}", ratio_type);

    // ✅ RATIO TYPE VALIDATION: Only allow SimpleRatio and DecimalRatio
    // EngineeringRatio pools are not supported for security and UX reasons
    if ratio_type == crate::types::RatioType::EngineeringRatio {
        msg!("❌ REJECTED: EngineeringRatio pools are not supported");
        msg!("   Only SimpleRatio (1:2, 1:100) and DecimalRatio (1:100.24343) are allowed");
        return Err(PoolError::UnsupportedRatioType {
            ratio_type: ratio_type.short_name().to_string(),
        }.into());
    }
 
    // ✅ POOL STATE: Create pool state with comprehensive configuration
    // Base flags: set SIMPLE_RATIO if applicable; OR-in allowed init flags without clearing
    let mut initial_flags: u8 = if ratio_type == crate::types::RatioType::SimpleRatio {
        crate::constants::POOL_FLAG_SIMPLE_RATIO
    } else {
        0
    };
    // Only allow a curated subset of flags at initialization time
    let allowed_init_mask: u8 =
        crate::constants::POOL_FLAG_SWAP_FOR_OWNERS_ONLY |
        crate::constants::POOL_FLAG_EXACT_EXCHANGE_REQUIRED;
    initial_flags |= flags & allowed_init_mask;

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
        // Preserve base flags and OR-in allowed initialization flags
        flags: initial_flags,
        
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
    msg!("🎯 POOL_ID: {} | Ratio: {}:{} | Type: {}", 
         pool_state_pda.key, ratio_a_numerator, ratio_b_denominator, ratio_type.short_name());
    
    Ok(())
}

/// Pauses pool operations using bitwise flags (Program Upgrade Authority only)
/// 
/// Uses bitwise flags to control which operations to pause:
/// - PAUSE_FLAG_LIQUIDITY (1): Pause deposits/withdrawals
/// - PAUSE_FLAG_SWAPS (2): Pause swaps
/// - PAUSE_FLAG_ALL (3): Pause both (required for consolidation eligibility)
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA validation
/// * `pause_flags` - Bitwise flags indicating which operations to pause
/// * `pool_id` - Expected Pool ID for security validation
/// * `accounts` - Array of account infos (4 accounts)
/// 
/// **Security**: Only the Program Upgrade Authority can pause individual pools.
/// **Idempotent**: Pausing already paused operations does not cause an error.
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_pool_pause(
    program_id: &Pubkey,
    pause_flags: u8,
    pool_id: Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing PausePool instruction with flags: 0b{:08b} ({})", pause_flags, pause_flags);
    
    // Extract accounts
    let program_authority_signer = &accounts[0];
    let system_state_pda = &accounts[1];
    let pool_state_pda = &accounts[2];
    let program_data_account = &accounts[3];
    
    // Validate system is not paused (allow authority operations during system pause)
    crate::utils::validation::validate_system_not_paused_secure(system_state_pda, program_id)?;
    
    // Validate Admin Authority
    use crate::utils::admin_validation::validate_admin_authority;
    validate_admin_authority(
        program_authority_signer,
        system_state_pda,
        Some(program_data_account),
        program_id,
    )?;
    
    // Load and validate pool state with Pool ID security validation
    let mut pool_state = validate_and_deserialize_pool_state_secure(pool_state_pda, &pool_id, program_id)?;
    
    // Apply pause flags (idempotent - no error if already paused)
    let mut operations_changed = Vec::new();
    
    if pause_flags & PAUSE_FLAG_LIQUIDITY != 0 && !pool_state.liquidity_paused() {
        pool_state.set_liquidity_paused(true);
        operations_changed.push("general operations");
    }
    
    if pause_flags & PAUSE_FLAG_SWAPS != 0 && !pool_state.swaps_paused() {
        pool_state.set_swaps_paused(true);
        operations_changed.push("swaps");
    }
    
    // Save updated pool state with size validation
    let serialized_data = pool_state.try_to_vec()?;
    if pool_state_pda.data_len() < serialized_data.len() {
        msg!("🚨 Critical Error: Pool state serialized data too large for account");
        return Err(ProgramError::AccountDataTooSmall);
    }
    pool_state_pda.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    // Log results
    if operations_changed.is_empty() {
        msg!("ℹ️ No changes made - requested operations were already paused");
    } else {
        msg!("✅ Pool operations paused: {}", operations_changed.join(", "));
    }
    
    msg!("   Pool: {}", pool_state_pda.key);
    msg!("   Liquidity operations: {}", if pool_state.liquidity_paused() { "PAUSED" } else { "ENABLED" });
    msg!("   Swap operations: {}", if pool_state.swaps_paused() { "PAUSED" } else { "ENABLED" });
    msg!("   Consolidation eligible: {}", 
         if pool_state.liquidity_paused() && pool_state.swaps_paused() { "YES" } else { "NO" });
    
    Ok(())
}

/// Unpauses pool operations using bitwise flags (Program Upgrade Authority only)
/// 
/// Uses bitwise flags to control which operations to unpause:
/// - PAUSE_FLAG_LIQUIDITY (1): Unpause deposits/withdrawals
/// - PAUSE_FLAG_SWAPS (2): Unpause swaps
/// - PAUSE_FLAG_ALL (3): Unpause both operations
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA validation
/// * `unpause_flags` - Bitwise flags indicating which operations to unpause
/// * `pool_id` - Expected Pool ID for security validation
/// * `accounts` - Array of account infos (4 accounts)
/// 
/// **Security**: Only the Program Upgrade Authority can unpause individual pools.
/// **Idempotent**: Unpausing already unpaused operations does not cause an error.
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_pool_unpause(
    program_id: &Pubkey,
    unpause_flags: u8,
    pool_id: Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing UnpausePool instruction with flags: 0b{:08b} ({})", unpause_flags, unpause_flags);
    
    // Extract accounts
    let program_authority_signer = &accounts[0];
    let system_state_pda = &accounts[1];
    let pool_state_pda = &accounts[2];
    let program_data_account = &accounts[3];
    
    // Validate system is not paused
    crate::utils::validation::validate_system_not_paused_secure(system_state_pda, program_id)?;
    
    // Validate Admin Authority
    use crate::utils::admin_validation::validate_admin_authority;
    validate_admin_authority(
        program_authority_signer,
        system_state_pda,
        Some(program_data_account),
        program_id,
    )?;
    
    // Load and validate pool state with Pool ID security validation
    let mut pool_state = validate_and_deserialize_pool_state_secure(pool_state_pda, &pool_id, program_id)?;
    
    // Apply unpause flags (idempotent - no error if already unpaused)
    let mut operations_changed = Vec::new();
    
    if unpause_flags & PAUSE_FLAG_LIQUIDITY != 0 && pool_state.liquidity_paused() {
        pool_state.set_liquidity_paused(false);
        operations_changed.push("general operations");
    }
    
    if unpause_flags & PAUSE_FLAG_SWAPS != 0 && pool_state.swaps_paused() {
        pool_state.set_swaps_paused(false);
        operations_changed.push("swaps");
    }
    
    // Save updated pool state with size validation
    let serialized_data = pool_state.try_to_vec()?;
    if pool_state_pda.data_len() < serialized_data.len() {
        msg!("🚨 Critical Error: Pool state serialized data too large for account");
        return Err(ProgramError::AccountDataTooSmall);
    }
    pool_state_pda.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    // Log results
    if operations_changed.is_empty() {
        msg!("ℹ️ No changes made - requested operations were already unpaused");
    } else {
        msg!("✅ Pool operations unpaused: {}", operations_changed.join(", "));
    }
    
    msg!("   Pool: {}", pool_state_pda.key);
    msg!("   Liquidity operations: {}", if pool_state.liquidity_paused() { "PAUSED" } else { "ENABLED" });
    msg!("   Swap operations: {}", if pool_state.swaps_paused() { "PAUSED" } else { "ENABLED" });
    msg!("   Consolidation eligible: {}", 
         if pool_state.liquidity_paused() && pool_state.swaps_paused() { "YES" } else { "NO" });
    
    Ok(())
}

/// BPF Loader Upgradeable Program Data Account Structure
/// 
/// This structure represents the layout of the program data account
/// created by the BPF Loader Upgradeable program.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct ProgramDataAccount {
    /// Account type discriminator (should be 3 for ProgramData)
    pub account_type: u32,
    /// Program's upgrade authority (None if frozen)
    pub upgrade_authority: Option<Pubkey>,
    /// Last time the program was deployed (slot)
    pub slot: u64,
}

/// Processes the UpdatePoolFees instruction
/// 
/// This function allows only the program authority to update the contract fees
/// for a specific pool. It supports updating either the liquidity fee or swap fee
/// (or both) using bitwise flags.
/// 
/// # Arguments
/// * `program_id` - The program ID
/// * `accounts` - Array of account infos (4 accounts)
/// * `update_flags` - Bitwise flags indicating which fees to update
/// * `new_liquidity_fee` - New liquidity fee in lamports
/// * `new_swap_fee` - New swap fee in lamports
/// * `pool_id` - Expected Pool ID for security validation
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_pool_update_fees(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    update_flags: u8,
    new_liquidity_fee: u64,
    new_swap_fee: u64,
    pool_id: Pubkey,
) -> ProgramResult {
    msg!("🔧 POOL FEE UPDATE TRANSACTION");
    msg!("📊 Update Flags: 0b{:03b} ({})", update_flags, update_flags);
    msg!("💰 New Liquidity Fee: {} lamports ({} SOL)", new_liquidity_fee, new_liquidity_fee as f64 / 1_000_000_000.0);
    msg!("💰 New Swap Fee: {} lamports ({} SOL)", new_swap_fee, new_swap_fee as f64 / 1_000_000_000.0);
    
    // ✅ ACCOUNT EXTRACTION: Extract accounts using optimized indices
    let account_info_iter = &mut accounts.iter();
    let program_authority_signer = next_account_info(account_info_iter)?; // Index 0: Program Authority Signer
    let system_state_pda = next_account_info(account_info_iter)?;         // Index 1: System State PDA
    let pool_state_pda = next_account_info(account_info_iter)?;           // Index 2: Pool State PDA
    let program_data_account = next_account_info(account_info_iter)?;     // Index 3: Program Data Account
    
    msg!("⏳ Step 1/4: Validating system state");
    
    // ✅ SYSTEM PAUSE VALIDATION: Ensure system is not paused
    crate::utils::validation::validate_system_not_paused_secure(system_state_pda, program_id)?;
    msg!("✅ System is not paused");
    
    msg!("⏳ Step 2/4: Validating program authority");
    
    // ✅ ADMIN AUTHORITY VALIDATION: Ensure caller is the admin authority
    use crate::utils::admin_validation::validate_admin_authority;
    validate_admin_authority(
        program_authority_signer,
        system_state_pda,
        Some(program_data_account),
        program_id,
    )?;
    msg!("✅ Admin authority validation passed");
    
    msg!("⏳ Step 3/4: Validating fee update parameters");
    
    // ✅ FEE UPDATE FLAGS VALIDATION: Ensure valid update flags
    validate_fee_update_flags(update_flags)?;
    msg!("✅ Fee update flags validation passed");
    
    // ✅ FEE VALIDATION: Ensure new fees are within acceptable limits
    validate_fee_limits(update_flags, new_liquidity_fee, new_swap_fee)?;
    msg!("✅ Fee limits validation passed");
    
    msg!("⏳ Step 4/4: Loading and updating pool state");
    
    // ✅ LOAD POOL STATE: Load current pool state with Pool ID security validation
    let mut pool_state_data = validate_and_deserialize_pool_state_secure(pool_state_pda, &pool_id, program_id)?;
    
    // ✅ DISPLAY CURRENT FEES: Show current fee configuration
    msg!("💰 CURRENT FEE CONFIGURATION:");
    msg!("   • Liquidity Fee: {} lamports ({} SOL)", 
         pool_state_data.contract_liquidity_fee, 
         pool_state_data.contract_liquidity_fee as f64 / 1_000_000_000.0);
    msg!("   • Swap Fee: {} lamports ({} SOL)", 
         pool_state_data.swap_contract_fee, 
         pool_state_data.swap_contract_fee as f64 / 1_000_000_000.0);
    
    // ✅ UPDATE FEES: Apply fee updates based on flags
    let mut fees_updated = false;
    
    if update_flags & FEE_UPDATE_FLAG_LIQUIDITY != 0 {
        let old_liquidity_fee = pool_state_data.contract_liquidity_fee;
        pool_state_data.contract_liquidity_fee = new_liquidity_fee;
        msg!("✅ Liquidity fee updated: {} → {} lamports", old_liquidity_fee, new_liquidity_fee);
        fees_updated = true;
    }
    
    if update_flags & FEE_UPDATE_FLAG_SWAP != 0 {
        let old_swap_fee = pool_state_data.swap_contract_fee;
        pool_state_data.swap_contract_fee = new_swap_fee;
        msg!("✅ Swap fee updated: {} → {} lamports", old_swap_fee, new_swap_fee);
        fees_updated = true;
    }
    
    if !fees_updated {
        return Err(PoolError::InvalidFeeUpdateFlags { flags: update_flags }.into());
    }
    
    // ✅ SERIALIZE UPDATED POOL STATE: Save changes to account
    pool_state_data.serialize(&mut &mut pool_state_pda.data.borrow_mut()[..])?;
    msg!("✅ Pool state serialized with updated fees");
    
    // ✅ SUCCESS SUMMARY
    msg!("🎉 POOL FEE UPDATE COMPLETED SUCCESSFULLY!");
    msg!("==========================================");
    msg!("✅ UPDATED FEE CONFIGURATION:");
    msg!("   • Liquidity Fee: {} lamports ({} SOL)", 
         pool_state_data.contract_liquidity_fee, 
         pool_state_data.contract_liquidity_fee as f64 / 1_000_000_000.0);
    msg!("   • Swap Fee: {} lamports ({} SOL)", 
         pool_state_data.swap_contract_fee, 
         pool_state_data.swap_contract_fee as f64 / 1_000_000_000.0);
    msg!("");
    msg!("📊 UPDATE SUMMARY:");
    msg!("   • Pool: {}", pool_state_pda.key);
    msg!("   • Updated by: {}", program_authority_signer.key);
    msg!("   • Update flags: 0b{:03b} ({})", update_flags, update_flags);
    msg!("");
    msg!("🚀 NEXT STEPS:");
    msg!("   • New fees will apply to all future operations");
    msg!("   • Existing pending fees are not affected");
    msg!("   • Monitor pool activity with new fee structure");
    msg!("==========================================");
    
    Ok(())
}



/// Validates the fee update flags
/// 
/// # Arguments
/// * `update_flags` - The bitwise flags indicating which fees to update
/// 
/// # Returns
/// * `ProgramResult` - Success or error
fn validate_fee_update_flags(update_flags: u8) -> ProgramResult {
    // ✅ FLAG VALIDATION: Ensure flags are valid combinations
    match update_flags {
        FEE_UPDATE_FLAG_LIQUIDITY => {
            msg!("✅ Updating liquidity fee only");
            Ok(())
        },
        FEE_UPDATE_FLAG_SWAP => {
            msg!("✅ Updating swap fee only");
            Ok(())
        },
        FEE_UPDATE_FLAG_BOTH => {
            msg!("✅ Updating both liquidity and swap fees");
            Ok(())
        },
        _ => {
            msg!("❌ Invalid fee update flags: 0b{:03b} ({})", update_flags, update_flags);
            msg!("   Valid flags: 1 (liquidity), 2 (swap), 3 (both)");
            Err(PoolError::InvalidFeeUpdateFlags { flags: update_flags }.into())
        }
    }
}

/// Validates that the new fees are within acceptable limits
/// 
/// # Arguments
/// * `update_flags` - The bitwise flags indicating which fees to update
/// * `new_liquidity_fee` - The new liquidity fee in lamports
/// * `new_swap_fee` - The new swap fee in lamports
/// 
/// # Returns
/// * `ProgramResult` - Success or error
fn validate_fee_limits(
    update_flags: u8,
    new_liquidity_fee: u64,
    new_swap_fee: u64,
) -> ProgramResult {
    // ✅ LIQUIDITY FEE VALIDATION: Check if liquidity fee is being updated and is valid
    if update_flags & FEE_UPDATE_FLAG_LIQUIDITY != 0 {
        if new_liquidity_fee < MIN_LIQUIDITY_FEE {
            msg!("❌ Liquidity fee too low: {} lamports (minimum: {} lamports)", 
                 new_liquidity_fee, MIN_LIQUIDITY_FEE);
            return Err(PoolError::InvalidLiquidityFee { 
                fee: new_liquidity_fee, 
                min: MIN_LIQUIDITY_FEE, 
                max: MAX_LIQUIDITY_FEE 
            }.into());
        }
        
        if new_liquidity_fee > MAX_LIQUIDITY_FEE {
            msg!("❌ Liquidity fee too high: {} lamports (maximum: {} lamports)", 
                 new_liquidity_fee, MAX_LIQUIDITY_FEE);
            return Err(PoolError::InvalidLiquidityFee { 
                fee: new_liquidity_fee, 
                min: MIN_LIQUIDITY_FEE, 
                max: MAX_LIQUIDITY_FEE 
            }.into());
        }
        
        msg!("✅ Liquidity fee validation passed: {} lamports", new_liquidity_fee);
    }
    
    // ✅ SWAP FEE VALIDATION: Check if swap fee is being updated and is valid
    if update_flags & FEE_UPDATE_FLAG_SWAP != 0 {
        if new_swap_fee < MIN_SWAP_FEE {
            msg!("❌ Swap fee too low: {} lamports (minimum: {} lamports)", 
                 new_swap_fee, MIN_SWAP_FEE);
            return Err(PoolError::InvalidSwapFee { 
                fee: new_swap_fee, 
                min: MIN_SWAP_FEE, 
                max: MAX_SWAP_FEE 
            }.into());
        }
        
        if new_swap_fee > MAX_SWAP_FEE {
            msg!("❌ Swap fee too high: {} lamports (maximum: {} lamports)", 
                 new_swap_fee, MAX_SWAP_FEE);
            return Err(PoolError::InvalidSwapFee { 
                fee: new_swap_fee, 
                min: MIN_SWAP_FEE, 
                max: MAX_SWAP_FEE 
            }.into());
        }
        
        msg!("✅ Swap fee validation passed: {} lamports", new_swap_fee);
    }
    
    Ok(())
}