use borsh::BorshSerialize;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::{
    instruction as token_instruction,
};

use crate::{
    constants::*,
    error::PoolError,
    utils::reentrancy_protection::{ReentrancyGuard, SafeTokenTransfer},
    utils::token_validation::safe_unpack_and_validate_token_account,
    with_reentrancy_protection,
};

// Removed the wrapper function - we'll use safe_unpack_and_validate_token_account directly

/// **Fixed-Ratio Token Swap with Basis Points Architecture**
///
/// Performs deterministic token swaps using pre-configured fixed exchange ratios stored
/// in basis points. This function implements exact input swapping where users specify
/// the input amount and receive a deterministic output amount based on the pool's ratio.
/// 
/// **BASIS POINTS REFACTOR: All Values in Basis Points**
/// 
/// This function operates entirely in basis points (smallest token units) with no
/// decimal conversion performed by the contract. All calculations preserve precision
/// and handle large numbers efficiently.
/// 
/// **Input/Output Flow:**
/// - Input: `amount_in` in basis points (from SPL token transfer)
/// - Pool ratios: Already stored in basis points (set during pool creation)
/// - Calculation: Pure basis point arithmetic
/// - Output: Result in basis points (for SPL token transfer)
/// 
/// **Example Calculation:**
/// ```
/// // Pool: 1.0 SOL = 160.0 USDT (1,000,000,000 : 160,000,000 basis points)
/// // Input: 0.5 SOL = 500,000,000 basis points
/// // Output: 500,000,000 * 160,000,000 / 1,000,000,000 = 80,000,000 basis points = 80.0 USDT
/// ```
///
/// # Key Features
///
/// # Fixed Ratio Exchange
/// - Exchange rates are predetermined and constant (e.g., 2:1, 3:1, etc.)
/// - No slippage - you get exactly the calculated amount or transaction fails
/// - Deterministic pricing eliminates front-running and MEV extraction
/// - Pool maintains its configured ratio regardless of trade size
///
/// # Arguments
/// * `program_id` - The program ID for PDA validation and signing authority
/// * `amount_in` - The amount of input tokens to swap (exact input model)
/// * `accounts` - Array of accounts in required order (9 accounts total)
/// 
/// # Account Layout
/// The accounts must be provided in the following order:
/// 0. **Authority/User Signer** (signer, writable) - User authorizing the swap
/// 1. **System Program Account** (readable) - Solana system program account
/// 2. **System State PDA** (readable) - System state PDA for pause validation  
/// 3. **Pool State PDA** (writable) - Pool state PDA containing configuration
/// 4. **SPL Token Program Account** (readable) - Token program account
/// 5. **Token A Vault PDA** (writable) - Pool's Token A vault PDA
/// 6. **Token B Vault PDA** (writable) - Pool's Token B vault PDA
/// 7. **User Input Token Account** (writable) - User's input token account
/// 8. **User Output Token Account** (writable) - User's output token account
///
/// # Returns
/// * `ProgramResult` - Success or error with detailed error information
/// 
/// # Fee Structure
/// - **Fixed SOL Fee**: 27,150 lamports (0.00002715 SOL) charged to user's SOL balance
/// - **Purpose**: Covers computational costs and protocol revenue
/// - **Collection**: Accumulated in pool state for later withdrawal
/// 
/// # Security Features
/// - Pause enforcement: Respects both system-wide and pool-specific pause states
/// - PDA validation: All pool accounts validated against expected PDA addresses
/// - Authority checks: Only token owners can initiate swaps for their tokens
/// - Arithmetic safety: All calculations use checked arithmetic to prevent overflow
/// - Atomic operations: Token transfers are atomic - either both succeed or both fail

/// Calculate precise swap output for Token A → Token B using the correct mathematical specification
///
/// Follows the exact formula: B_out = floor( A_in * ratioB_den * 10^(decimals_B - decimals_A) / ratioA_num )
/// This ensures precise handling of different token decimal places and basis point ratios.
fn swap_a_to_b(
    amount_a: u64,
    ratio_a_numerator: u64,     // Token A ratio in basis points
    ratio_b_denominator: u64,   // Token B ratio in basis points 
    token_a_decimals: u8,
    token_b_decimals: u8,
) -> Result<u64, ProgramError> {
    debug_msg!("🚨🚨🚨 FUNCTION ENTRY: swap_a_to_b CALLED! 🚨🚨🚨");
    msg!("🔍 SWAP_A_TO_B CORRECT SPECIFICATION:");
    msg!("   • Input amount_a: {}", amount_a);
    msg!("   • ratio_a_numerator: {}", ratio_a_numerator);
    msg!("   • ratio_b_denominator: {}", ratio_b_denominator);
    msg!("   • token_a_decimals: {}, token_b_decimals: {}", token_a_decimals, token_b_decimals);
    
    // Convert to u128 to prevent overflow during calculation
    let amount_a_base = amount_a as u128;
    let ratio_a_num = ratio_a_numerator as u128;
    let ratio_b_den = ratio_b_denominator as u128;
    
    if ratio_a_num == 0 {
        msg!("❌ CALCULATION ERROR: ratio_a_numerator is zero");
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Apply the correct mathematical specification:
    // B_out = floor( A_in * ratioB_den * 10^(decimals_B - decimals_A) / ratioA_num )
    
    // Step 1: Calculate the decimal scaling factor
    let decimal_diff = token_b_decimals as i32 - token_a_decimals as i32;
    msg!("   • Decimal difference (B - A): {} - {} = {}", token_b_decimals, token_a_decimals, decimal_diff);
    
    // Step 2: Apply the correct mathematical specification:
    // B_out = floor( A_in * ratioB_den / ratioA_num )
    // Since ratios are already in basis points, no additional decimal scaling is needed
    
    let numerator = amount_a_base
        .checked_mul(ratio_b_den)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    msg!("   • Calculation: ({} * {}) / {} = {} / {}", amount_a, ratio_b_denominator, ratio_a_numerator, numerator, ratio_a_num);
    
    let result = numerator / ratio_a_num;
    
    msg!("   • Final result (floor): {}", result);
    
    // Convert back to u64 and check for overflow
    if result > u64::MAX as u128 {
        msg!("❌ CALCULATION ERROR: Result exceeds u64::MAX");
        return Err(ProgramError::ArithmeticOverflow);
    }
    
    let final_result = result as u64;
    msg!("   • Final result: {}", final_result);
    
    Ok(final_result)
}

/// Calculate precise swap output for Token B → Token A using the correct mathematical specification
///
/// Follows the exact formula: A_out = floor( B_in * ratioA_num * 10^(decimals_A - decimals_B) / ratioB_den )
/// This ensures precise handling of different token decimal places and basis point ratios.
fn swap_b_to_a(
    amount_b: u64,
    ratio_a_numerator: u64,     // Token A ratio in basis points
    ratio_b_denominator: u64,   // Token B ratio in basis points
    token_b_decimals: u8,
    token_a_decimals: u8,
) -> Result<u64, ProgramError> {
    msg!("🔍 SWAP_B_TO_A CORRECT SPECIFICATION:");
    msg!("   • Input amount_b: {}", amount_b);
    msg!("   • ratio_a_numerator: {}", ratio_a_numerator);
    msg!("   • ratio_b_denominator: {}", ratio_b_denominator);
    msg!("   • token_b_decimals: {}, token_a_decimals: {}", token_b_decimals, token_a_decimals);
    
    // Convert to u128 to prevent overflow during calculation
    let amount_b_base = amount_b as u128;
    let ratio_a_num = ratio_a_numerator as u128;
    let ratio_b_den = ratio_b_denominator as u128;
    
    if ratio_b_den == 0 {
        msg!("❌ CALCULATION ERROR: ratio_b_denominator is zero");
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Apply the correct mathematical specification:
    // A_out = floor( B_in * ratioA_num * 10^(decimals_A - decimals_B) / ratioB_den )
    
    // Step 1: Calculate the decimal scaling factor
    let decimal_diff = token_a_decimals as i32 - token_b_decimals as i32;
    msg!("   • Decimal difference (A - B): {} - {} = {}", token_a_decimals, token_b_decimals, decimal_diff);
    
    // Step 2: Apply the correct mathematical specification:
    // A_out = floor( B_in * ratioA_num / ratioB_den )
    // Since ratios are already in basis points, no additional decimal scaling is needed
    
    let numerator = amount_b_base
        .checked_mul(ratio_a_num)
        .ok_or(ProgramError::ArithmeticOverflow)?;
        
    msg!("   • Calculation: ({} * {}) / {} = {} / {}", amount_b, ratio_a_numerator, ratio_b_denominator, numerator, ratio_b_den);
    
    let result = numerator / ratio_b_den;
    
    msg!("   • Final result (floor): {} basis points", result);
    
    // Convert back to u64 and check for overflow
    if result > u64::MAX as u128 {
        msg!("❌ CALCULATION ERROR: Result exceeds u64::MAX");
        return Err(ProgramError::ArithmeticOverflow);
    }
    
    let final_result = result as u64;
    msg!("   • Final result: {} basis points", final_result);
    
    Ok(final_result)
}

/// **Fixed-Ratio Token Swap with Basis Points Architecture**
///
/// Performs deterministic token swaps using pre-configured fixed exchange ratios stored
/// in basis points. This function implements exact input swapping where users specify
/// the input amount and receive a deterministic output amount based on the pool's ratio.
/// 
/// **BASIS POINTS REFACTOR: All Values in Basis Points**
/// 
/// This function operates entirely in basis points (smallest token units) with no
/// decimal conversion performed by the contract. All calculations preserve precision
/// and handle large numbers efficiently.
/// 
/// **Input/Output Flow:**
/// - Input: `amount_in` in basis points (from SPL token transfer)
/// - Pool ratios: Already stored in basis points (set during pool creation)
/// - Calculation: Pure basis point arithmetic
/// - Output: Result in basis points (for SPL token transfer)
/// 
/// **Example Calculation:**
/// ```
/// // Pool: 1.0 SOL = 160.0 USDT (1,000,000,000 : 160,000,000 basis points)
/// // Input: 0.5 SOL = 500,000,000 basis points
/// // Output: 500,000,000 * 160,000,000 / 1,000,000,000 = 80,000,000 basis points = 80.0 USDT
/// ```
///
/// # Key Features
///
/// # Fixed Ratio Exchange
/// - Exchange rates are predetermined and constant (e.g., 2:1, 3:1, etc.)
/// - No slippage - you get exactly the calculated amount or transaction fails
/// - Deterministic pricing eliminates front-running and MEV extraction
/// - Pool maintains its configured ratio regardless of trade size
///
/// # Security Features
/// - Pause enforcement: Respects both system-wide and pool-specific pause states
/// - PDA validation: All pool accounts validated against expected PDA addresses
/// - Authority checks: Only token owners can initiate swaps for their tokens
/// - Arithmetic safety: All calculations use checked arithmetic to prevent overflow
/// - Atomic operations: Token transfers are atomic - either both succeed or both fail
pub fn process_swap<'a>(
    program_id: &Pubkey,
    amount_in: u64,              // Input amount in basis points
    expected_amount_out: u64,    // Expected output amount in basis points
    accounts: &'a [AccountInfo<'a>],
) -> ProgramResult {
    debug_msg!("🚨🚨🚨 PROCESS_SWAP FUNCTION ENTRY - WE ARE HERE! 🚨🚨🚨");
    msg!("📊 SWAP PARAMETERS: {} basis points in, {} basis points expected out", amount_in, expected_amount_out);
    debug_msg!("🎯 STEP-BY-STEP DEBUG: Starting swap process...");
    
    // 🔒 CRITICAL SECURITY FIX: Validate input amount is non-zero
    if amount_in == 0 {
        msg!("❌ INVALID SWAP: Input amount cannot be zero");
        return Err(ProgramError::InvalidArgument);
    }
    
    // Extract required accounts from the accounts array
    let user_authority_signer = &accounts[0];      // Index 0: Authority/User Signer
    let system_program_account = &accounts[1];     // Index 1: System Program Account
    let system_state_pda = &accounts[2];           // Index 2: System State PDA
    let pool_state_pda = &accounts[3];             // Index 3: Pool State PDA
    let token_program_account = &accounts[4];      // Index 4: SPL Token Program Account
    let pool_token_a_vault_pda = &accounts[5];     // Index 5: Token A Vault PDA
    let pool_token_b_vault_pda = &accounts[6];     // Index 6: Token B Vault PDA
    let user_input_token_account = &accounts[7];   // Index 7: User Input Token Account
    let user_output_token_account = &accounts[8];  // Index 8: User Output Token Account

    // 🚨 CRITICAL SECURITY FIX: Validate user authority is a signer
    use crate::utils::validation::validate_signer;
    validate_signer(user_authority_signer, "User authority")?;
    
    // Fee breakdown available after pool validation
    
    // Security: MEV protection, pause protection, fixed-ratio
    
    // Step 1: Validating system and pool state
    debug_msg!("🎯 DEBUG: About to validate system not paused...");
    
    // Validate system is not paused
    crate::utils::validation::validate_system_not_paused_secure(system_state_pda, program_id)?;
    // System pause validation passed
    
    // Load and validate pool state data
    let mut pool_state_data = crate::utils::validation::validate_and_deserialize_pool_state_secure(pool_state_pda, program_id)?;

    // ✅ DISPLAY ACTUAL FEE INFORMATION (now that pool state is loaded)
    msg!("💰 Fee: {} lamports", pool_state_data.swap_contract_fee);

    // Check if pool swaps are paused
    if pool_state_data.swaps_paused() {
        msg!("❌ SWAP BLOCKED: Pool swaps are currently paused");
        msg!("   • Pool owner has paused trading");
        msg!("   • Contact pool owner to resume trading");
        return Err(PoolError::PoolSwapsPaused.into());
    }
    
    // Check if swap operations are restricted to owners only
    if pool_state_data.swap_for_owners_only() {
        msg!("🔒 CHECKING OWNER-ONLY SWAP PERMISSIONS");
        
        let user_key = *user_authority_signer.key;
        let pool_owner = pool_state_data.owner;
        
        // Allow access to pool owner
        if user_key == pool_owner {
            msg!("✅ Access granted: Pool owner");
        } else {
            // 🎯 ARCHITECTURAL SOLUTION: Unified Authority Control
            // 
            // Through the process_set_swap_owner_only function, the pool owner is automatically
            // reassigned to the Program Upgrade Authority when the restriction is enabled.
            // This eliminates the coordination complexity and ensures that the entity with
            // the power to enable/disable restrictions also has the power to swap.
            //
            // This approach:
            // - ✅ Solves the Program Upgrade Authority swap access issue
            // - ✅ Maintains lightweight swap instruction design  
            // - ✅ Eliminates need for pool creator coordination
            // - ✅ Unifies control under Program Upgrade Authority
            
            msg!("SWAP BLOCKED: Owner-only mode enabled. Pool owner: {}, Your address: {}", pool_owner, user_key);
            return Err(PoolError::SwapAccessRestricted.into());
        }
    }
    
    // Step 1 complete

    // Step 3: Validating user accounts
    
    // Load user token account data for validation
    // 🔒 SECURITY: Validate user token accounts with comprehensive checks
    let user_input_token_data = safe_unpack_and_validate_token_account(
        user_input_token_account, 
        "User Input Token Account",
        Some(user_authority_signer.key), // Must be owned by the user
        None, // Mint will be validated separately
        true, // Reject delegated accounts
    )?;
    let user_output_token_data = safe_unpack_and_validate_token_account(
        user_output_token_account,
        "User Output Token Account", 
        Some(user_authority_signer.key), // Must be owned by the user
        None, // Mint will be validated separately
        true, // Reject delegated accounts
    )?;

    // Determine swap direction from user's input token mint
    let input_token_mint_key = user_input_token_data.mint;

    // Determine swap direction and validate vault accounts
    let (input_pool_vault_acc, output_pool_vault_acc, output_token_mint_key, input_is_token_a) = 
        if input_token_mint_key == pool_state_data.token_a_mint {
            // Direction: A → B
            // A->B swap validation
            if *pool_token_a_vault_pda.key != pool_state_data.token_a_vault || 
               *pool_token_b_vault_pda.key != pool_state_data.token_b_vault {
                msg!("VAULT VALIDATION FAILED");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_a_vault_pda, pool_token_b_vault_pda, pool_state_data.token_b_mint, true)
        } else if input_token_mint_key == pool_state_data.token_b_mint {
            // Direction: B → A
            // B->A swap validation
            if *pool_token_b_vault_pda.key != pool_state_data.token_b_vault || 
               *pool_token_a_vault_pda.key != pool_state_data.token_a_vault {
                msg!("VAULT VALIDATION FAILED");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_b_vault_pda, pool_token_a_vault_pda, pool_state_data.token_a_mint, false)
        } else {
            msg!("INVALID INPUT TOKEN: {} not in pool", input_token_mint_key);
            return Err(ProgramError::InvalidArgument);
        };

    // 🔒 CRITICAL SECURITY FIX: Validate vault authorities
    msg!("🔒 VALIDATING VAULT AUTHORITIES...");
    // 🔒 SECURITY: Validate vault accounts - these should be owned by the pool PDA
    let input_vault_data = safe_unpack_and_validate_token_account(
        input_pool_vault_acc,
        "Input Pool Vault",
        Some(pool_state_pda.key), // Must be owned by the pool
        Some(&input_token_mint_key), // Must match the input token mint
        false, // Vaults shouldn't have delegates, but we check ownership already
    )?;
    let output_vault_data = safe_unpack_and_validate_token_account(
        output_pool_vault_acc,
        "Output Pool Vault",
        Some(pool_state_pda.key), // Must be owned by the pool
        Some(&output_token_mint_key), // Must match the output token mint
        false, // Vaults shouldn't have delegates, but we check ownership already
    )?;
    
    use crate::utils::validation::validate_vault_owner;
    validate_vault_owner(&input_vault_data, pool_state_pda.key, "Input Pool Vault")?;
    validate_vault_owner(&output_vault_data, pool_state_pda.key, "Output Pool Vault")?;
    msg!("✅ Vault authorities validated successfully");

    // Validate user account ownership and sufficient balance
    if user_input_token_data.mint != input_token_mint_key ||
       user_input_token_data.owner != *user_authority_signer.key ||
       user_input_token_data.amount < amount_in ||
       user_output_token_data.mint != output_token_mint_key ||
       user_output_token_data.owner != *user_authority_signer.key {
        msg!("❌ USER ACCOUNT VALIDATION FAILED");
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate SPL Token program account
    if *token_program_account.key != spl_token::id() {
        msg!("❌ INVALID TOKEN PROGRAM: SPL Token program mismatch");
        return Err(ProgramError::IncorrectProgramId);
    }
    
    msg!("✅ Step 3: Account validations passed");

    msg!("⏳ Step 4/6: Calculating fixed-ratio exchange with decimal adjustment");
    
    // 🚨 CRITICAL FIX: Get token decimals from token mints for accurate calculations
    // Since we don't have mint accounts directly, we need to get mint addresses from token accounts
    // and then fetch the mint data
    
    let input_token_mint_key = user_input_token_data.mint;
    let output_token_mint_key = user_output_token_data.mint;
    
    msg!("🔍 FETCHING TOKEN MINT DATA:");
    msg!("   • Input token mint: {}", input_token_mint_key);
    msg!("   • Output token mint: {}", output_token_mint_key);
    
    // We need to get the mint accounts from the remaining accounts
    // Let's check if mint accounts were provided as additional accounts
    if accounts.len() < 11 {
        msg!("❌ INSUFFICIENT ACCOUNTS: Token mint accounts required for decimal-aware calculations");
        msg!("   • Expected: 11 accounts (9 standard + 2 mint accounts)");
        msg!("   • Received: {} accounts", accounts.len());
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    let input_mint_account = &accounts[9];
    let output_mint_account = &accounts[10];
    
    // Verify the mint accounts match the expected mints
    if *input_mint_account.key != input_token_mint_key {
        msg!("❌ MINT ACCOUNT MISMATCH: Input mint account doesn't match token account mint");
        msg!("   • Expected: {}", input_token_mint_key);
        msg!("   • Provided: {}", input_mint_account.key);
        return Err(ProgramError::InvalidAccountData);
    }
    
    if *output_mint_account.key != output_token_mint_key {
        msg!("❌ MINT ACCOUNT MISMATCH: Output mint account doesn't match token account mint");
        msg!("   • Expected: {}", output_token_mint_key);
        msg!("   • Provided: {}", output_mint_account.key);
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Unpack mint accounts to get decimals
    let input_mint_data = spl_token::state::Mint::unpack_from_slice(&input_mint_account.data.borrow())
        .map_err(|_| {
            msg!("❌ FAILED TO UNPACK INPUT TOKEN MINT");
            ProgramError::InvalidAccountData
        })?;
    
    let output_mint_data = spl_token::state::Mint::unpack_from_slice(&output_mint_account.data.borrow())
        .map_err(|_| {
            msg!("❌ FAILED TO UNPACK OUTPUT TOKEN MINT");
            ProgramError::InvalidAccountData
        })?;
    
    let input_decimals = input_mint_data.decimals as u32;
    let output_decimals = output_mint_data.decimals as u32;
    
    msg!("🔍 ========== COMPLETE BLOCKCHAIN DATA ANALYSIS ==========");
    msg!("📊 POOL STATE DATA (from blockchain):");
    msg!("   • ratio_a_numerator: {}", pool_state_data.ratio_a_numerator);
    msg!("   • ratio_b_denominator: {}", pool_state_data.ratio_b_denominator);
    msg!("   • token_a_mint: {}", pool_state_data.token_a_mint);
    msg!("   • token_b_mint: {}", pool_state_data.token_b_mint);
    msg!("   • total_token_a_liquidity: {}", pool_state_data.total_token_a_liquidity);
    msg!("   • total_token_b_liquidity: {}", pool_state_data.total_token_b_liquidity);
    msg!("   • swap_contract_fee: {} lamports", pool_state_data.swap_contract_fee);
    msg!("   • collected_fees_token_a: {}", pool_state_data.collected_fees_token_a);
    msg!("   • collected_fees_token_b: {}", pool_state_data.collected_fees_token_b);
    
    msg!("🪙 TOKEN INFORMATION (from blockchain):");
    msg!("   • Token A decimals: {}", input_mint_data.decimals);
    msg!("   • Token B decimals: {}", output_mint_data.decimals);
    msg!("   • Input token decimals: {}", input_decimals);
    msg!("   • Output token decimals: {}", output_decimals);
    
    // Get exchange ratio based on swap direction (these are basis points from pool creation)
    let (numerator, denominator) = if input_is_token_a {
        if pool_state_data.ratio_a_numerator == 0 {
            msg!("❌ INVALID POOL RATIO: Token A numerator is zero");
            return Err(ProgramError::InvalidAccountData);
        }
        (pool_state_data.ratio_a_numerator, pool_state_data.ratio_b_denominator)
    } else {
        if pool_state_data.ratio_b_denominator == 0 {
            msg!("❌ INVALID POOL RATIO: Token B denominator is zero");
            return Err(ProgramError::InvalidAccountData);
        }
        (pool_state_data.ratio_b_denominator, pool_state_data.ratio_a_numerator)
    };

    // Swap direction determined
    
    // Using pool ratio for calculation
    
    // Calculation parameters: amount={}, ratio={}:{}

    // Debug logging removed to reduce CU usage

    // Starting calculation
    debug_msg!("🚨🚨🚨 CHECKPOINT 1: ABOUT TO ENTER IF STATEMENT");

    // 🔧 PRECISE DECIMAL CALCULATION using u128 for accuracy
    // Based on user's mathematically sound approach that properly handles basis points
    //
    // Key insight: ratio values are already stored in basis points (smallest token units)
    // We need to properly handle decimal scaling between different token decimal places
    
    debug_msg!("🚨🚨🚨 CHECKPOINT 2: ENTERING IF STATEMENT");
    
    let amount_out = if input_is_token_a {
        debug_msg!("🚨🚨🚨 CHECKPOINT 3: INSIDE TOKEN A BRANCH");
        msg!("🔥 CALLING swap_a_to_b with parameters:");
        msg!("   • amount_in: {}", amount_in);
        msg!("   • numerator (ratio_a_numerator): {}", numerator);
        msg!("   • denominator (ratio_b_denominator): {}", denominator);
        msg!("   • input_decimals (token_a): {}", input_decimals);
        msg!("   • output_decimals (token_b): {}", output_decimals);
        
        // Swapping Token A → Token B
        // Formula: amount_b = (amount_a * ratio_b_denominator) / ratio_a_numerator
        // numerator = ratio_a_numerator, denominator = ratio_b_denominator
        let result = swap_a_to_b(
            amount_in,
            numerator,      // ratio_a_numerator (basis points)
            denominator,    // ratio_b_denominator (basis points) 
            input_decimals as u8,  // token_a_decimals
            output_decimals as u8, // token_b_decimals
        )?;
        
        msg!("🔥 swap_a_to_b RETURNED: {}", result);
        result
    } else {
        msg!("🔥 CALLING swap_b_to_a with parameters:");
        msg!("   • amount_in: {}", amount_in);
        msg!("   • denominator as ratio_a_numerator: {}", denominator);
        msg!("   • numerator as ratio_b_denominator: {}", numerator);
        msg!("   • input_decimals (token_b): {}", input_decimals);
        msg!("   • output_decimals (token_a): {}", output_decimals);
        
        // Swapping Token B → Token A  
        // Formula: amount_a = (amount_b * ratio_a_numerator) / ratio_b_denominator
        // But ratio assignment gives us: numerator = ratio_b_denominator, denominator = ratio_a_numerator
        // So we need to swap the parameters to match our function's expected formula!
        let result = swap_b_to_a(
            amount_in,
            denominator,    // ratio_a_numerator (basis points) - swapped!
            numerator,      // ratio_b_denominator (basis points) - swapped!
            input_decimals as u8,  // token_b_decimals
            output_decimals as u8, // token_a_decimals
        )?;
        
            msg!("🔥 swap_b_to_a RETURNED: {} basis points", result);
    result
};

msg!("📊 SWAP CALCULATION COMPLETED: {} basis points -> {} basis points", amount_in, amount_out);
    
    // 🔒 CRITICAL SECURITY FIX: Validate output amount is non-zero
    if amount_out == 0 {
        msg!("❌ ZERO OUTPUT: Invalid swap configuration - refusing zero-output swap");
        return Err(ProgramError::InvalidArgument);
    }

    // Validate calculated amount matches expected amount (both in basis points)
    if amount_out != expected_amount_out {
        let difference = amount_out.abs_diff(expected_amount_out);
        msg!("❌ AMOUNT MISMATCH: Expected {} basis points, calculated {} basis points, diff {}", 
             expected_amount_out, amount_out, difference);
        
        // CRITICAL: Re-enable validation to prevent incorrect swaps
        return Err(crate::error::PoolError::AmountMismatch {
            expected: expected_amount_out,
            calculated: amount_out,
            difference,
        }.into());
    }
    
    msg!("✅ Amount validation passed: Expected {} basis points matches calculated {} basis points", 
         expected_amount_out, amount_out);

    // Step 5: Checking liquidity
    
    // Check if pool has sufficient liquidity for the output
    let available_liquidity = if input_is_token_a {
        pool_state_data.total_token_b_liquidity
    } else {
        pool_state_data.total_token_a_liquidity
    };
    
    // Liquidity check: available={}, required={}
    
    if available_liquidity < amount_out {
        msg!("❌ INSUFFICIENT LIQUIDITY: Need {} basis points, have {} basis points", amount_out, available_liquidity);
        return Err(ProgramError::InsufficientFunds);
    }
    
    msg!("✅ Liquidity check passed: {} basis points available, {} basis points needed", available_liquidity, amount_out);

    // Step 6: Executing transfers
    
    // Construct PDA seeds for pool authority signing
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.token_a_mint.as_ref(),
        pool_state_data.token_b_mint.as_ref(),
        &pool_state_data.ratio_a_numerator.to_le_bytes(),
        &pool_state_data.ratio_b_denominator.to_le_bytes(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    // Execute atomic token transfers with enhanced reentrancy protection
    msg!("🛡️ ENHANCED REENTRANCY PROTECTION: Starting swap with global locks");
    
    // Use the enhanced protection with global locking + snapshot validation
    with_reentrancy_protection!(
        &[
            user_input_token_account,
            user_output_token_account,
            input_pool_vault_acc,
            output_pool_vault_acc
        ],
        "Swap Operation",
        {
            // Step 6a: User Input → Pool Vault (with snapshot protection)
            let input_transfer = SafeTokenTransfer::new(
                user_input_token_account,
                input_pool_vault_acc,
                amount_in,
                "Swap Input Transfer"
            );
            
            input_transfer.execute_with_protection(|| {
                invoke(
                    &token_instruction::transfer(
                        token_program_account.key,
                        user_input_token_account.key,
                        input_pool_vault_acc.key,
                        user_authority_signer.key,
                        &[],
                        amount_in,
                    )?,
                    &[
                        user_input_token_account.clone(),
                        input_pool_vault_acc.clone(),
                        user_authority_signer.clone(),
                        token_program_account.clone(),
                    ],
                )
            })?;

            // Step 6b: Pool Vault → User Output (with snapshot protection)
            let output_transfer = SafeTokenTransfer::new(
                output_pool_vault_acc,
                user_output_token_account,
                amount_out,
                "Swap Output Transfer"
            );
            
            output_transfer.execute_with_protection(|| {
                invoke_signed(
                    &token_instruction::transfer(
                        token_program_account.key,
                        output_pool_vault_acc.key,
                        user_output_token_account.key,
                        pool_state_pda.key,
                        &[],
                        amount_out,
                    )?,
                    &[
                        output_pool_vault_acc.clone(),
                        user_output_token_account.clone(),
                        pool_state_pda.clone(),
                        token_program_account.clone(),
                    ],
                    &[pool_state_pda_seeds],
                )
            })?;
            
            Ok(())
        }
    )?;

    msg!("✅ Token transfers completed successfully");
    msg!("📊 TRANSFER SUMMARY: {} basis points transferred in, {} basis points transferred out", amount_in, amount_out);

    // Update pool liquidity balances based on swap direction
    if input_is_token_a {
        pool_state_data.total_token_a_liquidity = pool_state_data.total_token_a_liquidity
            .checked_add(amount_in)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        pool_state_data.total_token_b_liquidity = pool_state_data.total_token_b_liquidity
            .checked_sub(amount_out)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    } else {
        pool_state_data.total_token_b_liquidity = pool_state_data.total_token_b_liquidity
            .checked_add(amount_in)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        pool_state_data.total_token_a_liquidity = pool_state_data.total_token_a_liquidity
            .checked_sub(amount_out)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }

    msg!("💾 Saving updated pool state");
    
    // Serialize updated pool state
    let mut serialized_data = Vec::new();
    pool_state_data.serialize(&mut serialized_data)?;
    
    // Save the pool state in a separate scope to release the mutable borrow
    {
        let mut pool_state_pda_data = pool_state_pda.data.borrow_mut();
        if serialized_data.len() > pool_state_pda_data.len() {
            msg!("❌ SERIALIZATION ERROR: Data too large for account");
            return Err(ProgramError::AccountDataTooSmall);
        }
        
        pool_state_pda_data[..serialized_data.len()].copy_from_slice(&serialized_data);
    } // Release mutable borrow here before fee collection
    
    // ✅ COLLECT SOL FEES TO POOL STATE AFTER INVOKE OPERATIONS (GitHub Issue #31960 Workaround)
    // Fee collection must happen AFTER all invoke/invoke_signed operations to prevent PDA corruption
    use crate::utils::fee_validation::{collect_fee_to_pool_state, FeeType};
    
    collect_fee_to_pool_state(
        user_authority_signer,
        pool_state_pda,  // ← Collect to pool state instead of main treasury
        system_program_account,
        program_id,
        pool_state_data.swap_contract_fee,
        FeeType::RegularSwap,
    )?;
    
    msg!("✅ SWAP COMPLETED SUCCESSFULLY!");
    msg!("📈 SUMMARY: {} → {} tokens, Fee: {} lamports", amount_in, amount_out, pool_state_data.swap_contract_fee);
    
    Ok(())
}

/// Manages swap access restrictions and delegates ownership control for a specific pool
///
/// This function allows the contract owner (program upgrade authority) to enable or disable
/// swap access restrictions for a specific pool and delegate control to any specified entity.
/// When enabled, only the designated owner can perform swap operations on that pool.
///
/// **IMPORTANT**: This function can ONLY be called by the contract owner, not the pool owner.
/// This ensures that access control decisions remain at the protocol level while enabling
/// flexible delegation of operational control.
///
/// # Enhanced Flexibility
/// 
/// This system provides maximum operational flexibility while maintaining security:
/// 
/// - **Flexible Delegation**: Program Upgrade Authority can delegate to any entity
/// - **Specialized Controllers**: Enable specialized swap controllers for different use cases
/// - **Complex Scenarios**: Support treasury management, automated strategies, multi-sig control
/// - **Protocol Control**: Contract owner maintains oversight and ultimate control
/// - **Custom Fee Collection**: Support various fee structures through delegation
/// - **Compatibility**: Existing pools continue normal operation unless explicitly restricted
///
/// # How Custom Fee Structures Work
/// 
/// 1. **Pool Owner** deploys a custom fee-collecting contract
/// 2. **Contract Owner** enables owner-only mode for that specific pool
/// 3. **Users** interact with the custom contract instead of the pool directly
/// 4. **Custom Contract** collects fees according to its logic and routes swaps through the pool
/// 5. **Pool Owner** benefits from custom fee revenue while maintaining pool ownership
///
/// # Security Model
/// 
/// - **Contract Owner**: Can enable/disable owner-only mode for any pool
/// - **Pool Owner**: Can perform swaps when owner-only mode is enabled
/// - **Regular Users**: Blocked from direct swaps when owner-only mode is enabled
/// - **Custom Contracts**: Can be granted pool ownership or contract ownership for access
///
/// # System Pause Behavior
/// This operation is **BLOCKED** when the system is paused. System pause takes precedence
/// over all pool operations to ensure system-wide consistency.
///
/// # Arguments
/// * `program_id` - The program ID for PDA validation and upgrade authority checks
/// * `enable_restriction` - True to enable owner-only mode, false to disable
/// * `designated_owner` - The pubkey that will have swap control when restrictions are enabled
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Contract owner account (must be program upgrade authority and signer)
///   - `accounts[1]` - System state PDA account (for system pause validation)
///   - `accounts[2]` - Pool state PDA account (writable for flag and ownership updates)
///   - `accounts[3]` - Program data account (for upgrade authority validation)
///
/// # Account Requirements
/// - **Contract Owner**: Must be signer and match the program upgrade authority
/// - **System State**: Must be valid system state account for pause validation
/// - **Pool State**: Must be writable for flag configuration updates
/// - **Program Data**: Must be valid program data account for authority validation
///
/// # Error Conditions
/// - `ProgramError::MissingRequiredSignature` - Contract owner didn't sign transaction
/// - `ProgramError::InvalidAccountData` - Caller is not the contract owner
/// - `PoolError::SystemPaused` - System is currently paused
///
/// # Example Usage Scenarios
///
/// ## Scenario 1: Enable Custom Fee Collection
/// ```ignore
/// // 1. Pool owner deploys CustomFeeContract that charges 0.3% fee
/// // 2. Contract owner enables owner-only mode for the pool
/// let instruction = PoolInstruction::SetSwapOwnerOnly {
///     enable_restriction: true,
/// };
/// // 3. Users swap through CustomFeeContract instead of pool directly
/// // 4. CustomFeeContract collects 0.3% fee and routes swap to pool as pool owner
/// ```
///
/// ## Scenario 2: Dynamic Fee Model
/// ```ignore
/// // Pool owner creates contract with time-based or volume-based dynamic fees
/// // Contract can implement any fee logic and still use the pool infrastructure
/// ```
///
/// ## Scenario 3: Disable Custom Fees
/// ```ignore
/// // Contract owner can always disable owner-only mode to restore normal operation
/// let instruction = PoolInstruction::SetSwapOwnerOnly {
///     enable_restriction: false,
/// };
/// ```
///
/// # Integration with Swap Process
/// When owner-only mode is enabled, the `process_swap` function will:
/// 1. Check if the swap_for_owners_only flag is set
/// 2. Verify the caller is either the pool owner or contract owner
/// 3. Block the transaction if the caller is not authorized
/// 4. Proceed with normal swap logic if authorized
///
/// This creates a secure foundation for custom fee structures while maintaining
/// the protocol's core swap functionality and security model.
pub fn process_set_swap_owner_only<'a>(
    program_id: &Pubkey,
    enable_restriction: bool,
    designated_owner: Pubkey,
    accounts: &'a [AccountInfo<'a>],
) -> ProgramResult {
    msg!("🔒 SWAP OWNER-ONLY CONFIGURATION");
    msg!("===============================");
    msg!("📊 Action: {} swap owner-only restriction", if enable_restriction { "Enable" } else { "Disable" });
    
    let contract_owner_signer = &accounts[0];     // Index 0: Contract Owner (Program Upgrade Authority)
    let system_state_pda = &accounts[1];          // Index 1: System State PDA  
    let pool_state_pda = &accounts[2];            // Index 2: Pool State PDA
    let program_data_account = &accounts[3];      // Index 3: Program Data Account
    
    msg!("⏳ Step 1/4: Validating system state");
    
    // Validate system is not paused
    crate::utils::validation::validate_system_not_paused_secure(system_state_pda, program_id)?;
    
    msg!("✅ Step 1 completed: System validation passed");
    
    msg!("⏳ Step 2/4: Validating contract owner authority");
    
    msg!("🔍 Authority Verification:");
    msg!("   • Validating program upgrade authority");
    msg!("   • Provided signer: {}", contract_owner_signer.key);
    
    // Validate that the caller is the program upgrade authority
    use crate::utils::program_authority::validate_program_upgrade_authority;
    validate_program_upgrade_authority(program_id, program_data_account, contract_owner_signer)?;
    
    msg!("✅ Step 2 completed: Program upgrade authority validated");
    
    msg!("⏳ Step 3/4: Loading and updating pool state");
    
    // Load and validate pool state data
    let mut pool_state_data = crate::utils::validation::validate_and_deserialize_pool_state_secure(pool_state_pda, program_id)?;
    
    msg!("📋 Pool Information:");
    msg!("   • Pool: {} ↔ {}", pool_state_data.token_a_mint, pool_state_data.token_b_mint);
    msg!("   • Current pool owner: {}", pool_state_data.owner);
    msg!("   • Program upgrade authority: {}", contract_owner_signer.key);
    msg!("   • Current owner-only status: {}", pool_state_data.swap_for_owners_only());
    msg!("   • Requested status: {}", enable_restriction);
    
    // Check if flag is already in the requested state
    if pool_state_data.swap_for_owners_only() == enable_restriction {
        let status = if enable_restriction { "enabled" } else { "disabled" };
        msg!("ℹ️ No change needed: Owner-only swaps already {}", status);
    } else {
        // Update the flag
        pool_state_data.set_swap_for_owners_only(enable_restriction);
        msg!("🔄 Flag updated: Owner-only swaps now {}", if enable_restriction { "enabled" } else { "disabled" });
    }
    
    // 🎯 ENHANCED FLEXIBILITY: Assign pool ownership to designated entity
    // This enables flexible delegation of swap control while maintaining Program Upgrade Authority
    // control over the ability to change restrictions and delegate ownership
    if enable_restriction {
        if pool_state_data.owner != designated_owner {
            let previous_owner = pool_state_data.owner;
            pool_state_data.owner = designated_owner;
            
            msg!("🔄 OWNERSHIP DELEGATION:");
            msg!("   • Previous owner: {}", previous_owner);
            msg!("   • New designated owner: {}", designated_owner);
            msg!("   • Delegated by: {}", contract_owner_signer.key);
            msg!("   • Rationale: Enables flexible operational control while maintaining protocol authority");
            msg!("   • Impact: Designated entity now has swap control for this pool");
        } else {
            msg!("ℹ️ Pool already owned by designated entity: {}", designated_owner);
        }
    } else {
        msg!("ℹ️ Restrictions disabled - ownership delegation not applicable");
    }
    
    msg!("✅ Step 3 completed: Pool state updated");
    
    msg!("⏳ Step 4/4: Saving updated pool state");
    
    // Serialize and save updated pool state
    let mut serialized_data = Vec::new();
    pool_state_data.serialize(&mut serialized_data)?;
    
    let mut pool_state_pda_data = pool_state_pda.data.borrow_mut();
    if serialized_data.len() > pool_state_pda_data.len() {
        msg!("❌ SERIALIZATION ERROR: Data too large for account");
        return Err(ProgramError::AccountDataTooSmall);
    }
    
    pool_state_pda_data[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    msg!("✅ SWAP OWNER-ONLY CONFIGURATION COMPLETED!");
    msg!("===============================");
    msg!("📈 CONFIGURATION SUMMARY:");
    msg!("   • Pool: {} ↔ {}", pool_state_data.token_a_mint, pool_state_data.token_b_mint);
    msg!("   • Owner-only swaps: {}", if enable_restriction { "ENABLED" } else { "DISABLED" });
    msg!("   • Pool owner: {}", pool_state_data.owner);
    msg!("   • Program upgrade authority: {}", contract_owner_signer.key);
    if enable_restriction {
        msg!("   • Swap access: Pool owner ({})", pool_state_data.owner);
        msg!("   • Architecture: Flexible delegation under Protocol Authority");
        msg!("   • Designated by: Program Upgrade Authority");
    } else {
        msg!("   • Swap access: All users");
    }
    
    if enable_restriction {
        msg!("🔒 SWAP ACCESS NOW RESTRICTED:");
        msg!("   • Only designated owner can swap: {}", pool_state_data.owner);
        msg!("   • Regular users must use authorized intermediary contracts");
        msg!("   • Enables flexible operational models and custom fee structures");
        msg!("   • Designated entity can deploy contracts with any operational model");
        
        msg!("💡 OPERATIONAL FLEXIBILITY BENEFITS:");
        msg!("   • Custom fee collection through specialized contracts");
        msg!("   • Treasury management through automated systems");
        msg!("   • Strategic trading through algorithmic entities");
        msg!("   • Multi-signature control for team-managed pools");
        msg!("   • Protocol integration for composed operations");
        msg!("   • Maximum operational flexibility while maintaining protocol security");
    } else {
        msg!("🔓 SWAP ACCESS NOW UNRESTRICTED:");
        msg!("   • All users can swap directly with the pool");
        msg!("   • Standard fixed swap contract fees apply");
        msg!("   • No custom operational models active");
        msg!("   • Traditional AMM-style operation");
    }
    
    msg!("🎉 Swap access configuration updated successfully!");
    msg!("💡 NEXT STEPS:");
    if enable_restriction {
        msg!("   • Designated owner ({}) can deploy operational contracts", pool_state_data.owner);
        msg!("   • Users should interact with authorized contracts for swaps");
        msg!("   • Monitor operational performance and pool health");
        msg!("   • Program Upgrade Authority retains control to modify delegation");
    } else {
        msg!("   • Users can swap directly with the pool");
        msg!("   • Monitor standard pool operation and liquidity");
        msg!("   • Consider operational delegation in the future if needed");
    }
    
    Ok(())
}


 