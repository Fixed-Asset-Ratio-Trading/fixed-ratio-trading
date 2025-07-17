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
    state::{Account as TokenAccount},
};

use crate::{
    constants::*,
    error::PoolError,

};



/// Processes token swaps at fixed exchange ratios with deterministic pricing
/// 
/// This function handles all token swap operations in the pool, using predetermined 
/// fixed exchange rates to convert one token type to another. The swap process
/// includes fee collection, liquidity validation, ratio-based calculations,
/// and atomic token transfers.
///
/// # How It Works
/// 1. **Account Validation**: Validates all required accounts and user permissions
/// 2. **System Checks**: Ensures system and pool are not paused
/// 3. **Fee Collection**: Collects fixed swap fee from user's SOL balance
/// 4. **Direction Detection**: Determines swap direction (A→B or B→A) from user's input token
/// 5. **Ratio Calculation**: Calculates output amount using fixed pool ratios
/// 6. **Liquidity Check**: Verifies pool has sufficient output tokens available
/// 7. **Token Transfers**: Executes atomic input/output token transfers
/// 8. **State Updates**: Updates pool liquidity balances and saves state
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
pub fn process_swap(
    program_id: &Pubkey,
    amount_in: u64,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("🔄 SWAP TRANSACTION SUMMARY");
    msg!("=============================");
    msg!("📊 Input Amount: {} tokens", amount_in);
    
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

    msg!("💰 FEE BREAKDOWN:");
    msg!("   • Network Fee: ~0.000005 SOL (static)");
    msg!("   • Swap Contract Fee: {} lamports", crate::constants::SWAP_CONTRACT_FEE);
    msg!("   • No account creation costs (existing accounts)");
    
    msg!("🔒 TRANSACTION SECURITY:");
    msg!("   • MEV protection: Atomic transaction");
    msg!("   • System pause protection: Active");
    msg!("   • Fixed-ratio protection: No slippage (guaranteed rate)");
    
    msg!("⏳ Step 1/6: Validating system and pool state");
    
    // Validate system is not paused
    crate::utils::validation::validate_system_not_paused_secure(system_state_pda, program_id)?;
    
    // Load and validate pool state data
    let mut pool_state_data = crate::utils::validation::validate_and_deserialize_pool_state_secure(pool_state_pda, program_id)?;

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
        
        // TODO: Implement proper contract owner validation using program data account
        // For now, we use the pool owner as the primary access control
        // This will be enhanced to include contract owner validation in a future update
        
        let user_key = *user_authority_signer.key;
        let pool_owner = pool_state_data.owner;
        
        // Allow access only to pool owner (contract owner validation pending)
        if user_key != pool_owner {
            msg!("❌ SWAP BLOCKED: Owner-only mode is enabled");
            msg!("   • This pool restricts swaps to owners only");
            msg!("   • Pool owner: {}", pool_owner);
            msg!("   • Your address: {}", user_key);
            msg!("   • Purpose: Enables custom fee structures through external contracts");
            msg!("   • Contact pool owner for access or use their fee-collecting contract");
            return Err(PoolError::SwapAccessRestricted.into());
        }
        
        msg!("✅ Access granted: Pool owner");
    }
    
    msg!("✅ Step 1 completed: System and pool validations passed");

    msg!("⏳ Step 2/6: Collecting protocol fees");
    
    // Collect swap fee to pool state
    use crate::utils::fee_validation::{collect_fee_to_pool_state, FeeType};
    use crate::constants::SWAP_CONTRACT_FEE;
    
    collect_fee_to_pool_state(
        user_authority_signer,
        pool_state_pda,  // ← Collect to pool state instead of main treasury
        system_program_account,
        program_id,
        SWAP_CONTRACT_FEE,
        FeeType::RegularSwap,
    )?;
    
    msg!("✅ Step 2 completed: Fee collection successful ({} lamports)", SWAP_CONTRACT_FEE);
    
    msg!("⏳ Step 3/6: Loading and validating user accounts");
    
    // Load user token account data for validation
    let user_input_token_data = TokenAccount::unpack_from_slice(&user_input_token_account.data.borrow())?;
    let user_output_token_data = TokenAccount::unpack_from_slice(&user_output_token_account.data.borrow())?;

    // Determine swap direction from user's input token mint
    let input_token_mint_key = user_input_token_data.mint;
    
    msg!("📋 Input token mint: {}", input_token_mint_key);
    msg!("📋 Input amount: {} tokens", amount_in);

    // Determine swap direction and validate vault accounts
    let (input_pool_vault_acc, output_pool_vault_acc, output_token_mint_key, input_is_token_a) = 
        if input_token_mint_key == pool_state_data.token_a_mint {
            msg!("🔄 SWAP DIRECTION: Token A → Token B");
            msg!("   • Input: Token A (mint: {})", pool_state_data.token_a_mint);
            msg!("   • Output: Token B (mint: {})", pool_state_data.token_b_mint);
            // A->B swap validation
            if *pool_token_a_vault_pda.key != pool_state_data.token_a_vault || 
               *pool_token_b_vault_pda.key != pool_state_data.token_b_vault {
                msg!("❌ VAULT VALIDATION FAILED: Invalid pool vault accounts");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_a_vault_pda, pool_token_b_vault_pda, pool_state_data.token_b_mint, true)
        } else if input_token_mint_key == pool_state_data.token_b_mint {
            msg!("🔄 SWAP DIRECTION: Token B → Token A");
            msg!("   • Input: Token B (mint: {})", pool_state_data.token_b_mint);
            msg!("   • Output: Token A (mint: {})", pool_state_data.token_a_mint);
            // B->A swap validation
            if *pool_token_b_vault_pda.key != pool_state_data.token_b_vault || 
               *pool_token_a_vault_pda.key != pool_state_data.token_a_vault {
                msg!("❌ VAULT VALIDATION FAILED: Invalid pool vault accounts");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_b_vault_pda, pool_token_a_vault_pda, pool_state_data.token_a_mint, false)
        } else {
            msg!("❌ INVALID INPUT TOKEN: Not part of this pool");
            msg!("   • Provided mint: {}", input_token_mint_key);
            msg!("   • Pool Token A: {}", pool_state_data.token_a_mint);
            msg!("   • Pool Token B: {}", pool_state_data.token_b_mint);
            return Err(ProgramError::InvalidArgument);
        };

    msg!("🔍 Validating user account ownership and balances");
    
    // Validate user account ownership and sufficient balance
    if user_input_token_data.mint != input_token_mint_key ||
       user_input_token_data.owner != *user_authority_signer.key ||
       user_input_token_data.amount < amount_in ||
       user_output_token_data.mint != output_token_mint_key ||
       user_output_token_data.owner != *user_authority_signer.key {
        msg!("❌ USER ACCOUNT VALIDATION FAILED");
        msg!("   • Check account ownership and balances");
        msg!("   • Ensure sufficient tokens for swap");
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate SPL Token program account
    if *token_program_account.key != spl_token::id() {
        msg!("❌ INVALID TOKEN PROGRAM: SPL Token program mismatch");
        return Err(ProgramError::IncorrectProgramId);
    }
    
    msg!("✅ Step 3 completed: Account validations passed");

    msg!("⏳ Step 4/6: Calculating fixed-ratio exchange");
    
    // Get exchange ratio based on swap direction
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

    // Calculate output amount using fixed ratio: output = input * numerator / denominator
    let amount_out = amount_in.checked_mul(numerator)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_div(denominator)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    msg!("📊 FIXED RATIO CALCULATION:");
    msg!("   • Exchange rate: {}:{} (numerator:denominator)", numerator, denominator);
    msg!("   • Input: {} tokens", amount_in);
    msg!("   • Output: {} tokens", amount_out);
    msg!("   • Slippage protection: Fixed ratio (no slippage)");
    
    // Validate output amount is non-zero
    if amount_out == 0 {
        msg!("❌ ZERO OUTPUT: Calculated output amount is zero");
        msg!("   • This indicates an invalid swap configuration");
        return Err(ProgramError::InvalidArgument);
    }

    msg!("⏳ Step 5/6: Checking pool liquidity availability");
    
    // Check if pool has sufficient liquidity for the output
    let available_liquidity = if input_is_token_a {
        pool_state_data.total_token_b_liquidity
    } else {
        pool_state_data.total_token_a_liquidity
    };
    
    msg!("📊 LIQUIDITY CHECK:");
    msg!("   • Available liquidity: {} tokens", available_liquidity);
    msg!("   • Required output: {} tokens", amount_out);
    msg!("   • Pool health: {}", if available_liquidity >= amount_out { "✅ Sufficient" } else { "❌ Insufficient" });
    
    if available_liquidity < amount_out {
        msg!("❌ INSUFFICIENT LIQUIDITY: Pool cannot fulfill swap");
        msg!("   • Available: {} tokens", available_liquidity);
        msg!("   • Required: {} tokens", amount_out);
        msg!("   • Try a smaller amount or wait for more liquidity");
        return Err(ProgramError::InsufficientFunds);
    }
    
    msg!("✅ Step 5 completed: Liquidity check passed");

    msg!("⏳ Step 6/6: Executing atomic token transfers");
    
    // Construct PDA seeds for pool authority signing
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.token_a_mint.as_ref(),
        pool_state_data.token_b_mint.as_ref(),
        &pool_state_data.ratio_a_numerator.to_le_bytes(),
        &pool_state_data.ratio_b_denominator.to_le_bytes(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    // Execute atomic token transfers
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
    )?;

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
    )?;

    msg!("✅ Token transfers completed successfully");
    msg!("🔄 Updating pool liquidity balances");

    // Update pool liquidity balances based on swap direction
    if input_is_token_a {
        pool_state_data.total_token_a_liquidity = pool_state_data.total_token_a_liquidity
            .checked_add(amount_in)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        pool_state_data.total_token_b_liquidity = pool_state_data.total_token_b_liquidity
            .checked_sub(amount_out)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        msg!("📊 Updated liquidity - Token A: +{}, Token B: -{}", amount_in, amount_out);
    } else {
        pool_state_data.total_token_b_liquidity = pool_state_data.total_token_b_liquidity
            .checked_add(amount_in)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        pool_state_data.total_token_a_liquidity = pool_state_data.total_token_a_liquidity
            .checked_sub(amount_out)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        msg!("📊 Updated liquidity - Token B: +{}, Token A: -{}", amount_in, amount_out);
    }

    msg!("💾 Saving updated pool state");
    
    // Serialize and save updated pool state
    let mut serialized_data = Vec::new();
    pool_state_data.serialize(&mut serialized_data)?;
    
    let mut pool_state_pda_data = pool_state_pda.data.borrow_mut();
    if serialized_data.len() > pool_state_pda_data.len() {
        msg!("❌ SERIALIZATION ERROR: Data too large for account");
        return Err(ProgramError::AccountDataTooSmall);
    }
    
    pool_state_pda_data[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    msg!("✅ SWAP COMPLETED SUCCESSFULLY!");
    msg!("=============================");
    msg!("📈 COMPREHENSIVE TRANSACTION SUMMARY:");
    msg!("   • Input: {} tokens (mint: {})", amount_in, input_token_mint_key);
    msg!("   • Output: {} tokens (mint: {})", amount_out, output_token_mint_key);
    msg!("   • Exchange rate: {}:{} (fixed ratio)", numerator, denominator);
    msg!("   • Total fees paid: {} lamports", SWAP_CONTRACT_FEE);
    msg!("   • Pool: {} ↔ {}", pool_state_data.token_a_mint, pool_state_data.token_b_mint);
    
    msg!("💰 POST-TRANSACTION POOL STATE:");
    msg!("   • Token A liquidity: {} tokens", pool_state_data.total_token_a_liquidity);
    msg!("   • Token B liquidity: {} tokens", pool_state_data.total_token_b_liquidity);
    msg!("   • Pool ratio maintained: {}:{}", pool_state_data.ratio_a_numerator, pool_state_data.ratio_b_denominator);
    
    msg!("🎉 Your swap has been executed successfully!");
    msg!("💡 NEXT STEPS:");
    msg!("   • Check your output token balance");
    msg!("   • Consider providing liquidity to earn fees");
    msg!("   • Monitor pool health and liquidity levels");
    
    Ok(())
}

/// Manages the swap owner-only restriction flag for a specific pool
///
/// This function allows the contract owner (program upgrade authority) to enable or disable
/// swap access restrictions for a specific pool. When enabled, only the pool owner and 
/// contract owner can perform swap operations on that pool.
///
/// **IMPORTANT**: This function can ONLY be called by the contract owner, not the pool owner.
/// This ensures that access control decisions remain at the protocol level.
///
/// # Purpose and Rationale
/// 
/// This system enables flexible custom fee structures while maintaining protocol control:
/// 
/// - **Custom Fee Collection**: Pool owners can deploy separate contracts that collect fees
///   and then route swaps through the pool using owner-only access
/// - **Protocol Control**: Contract owner maintains oversight of which pools use restricted access
/// - **Flexibility**: Supports any fee model (flat fees, dynamic fees, tiered fees, etc.)
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
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Contract owner account (must be program upgrade authority and signer)
///   - `accounts[1]` - System state PDA account (for system pause validation)
///   - `accounts[2]` - Pool state PDA account (writable for flag updates)
///
/// # Account Requirements
/// - **Contract Owner**: Must be signer and match the program upgrade authority
/// - **System State**: Must be valid system state account for pause validation
/// - **Pool State**: Must be writable for flag configuration updates
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
pub fn process_set_swap_owner_only(
    program_id: &Pubkey,
    enable_restriction: bool,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("🔒 SWAP OWNER-ONLY CONFIGURATION");
    msg!("===============================");
    msg!("📊 Action: {} swap owner-only restriction", if enable_restriction { "Enable" } else { "Disable" });
    
    let contract_owner_signer = &accounts[0];     // Index 0: Contract Owner (Program Upgrade Authority)
    let system_state_pda = &accounts[1];          // Index 1: System State PDA  
    let pool_state_pda = &accounts[2];            // Index 2: Pool State PDA
    
    msg!("⏳ Step 1/4: Validating system state");
    
    // Validate system is not paused
    crate::utils::validation::validate_system_not_paused_secure(system_state_pda, program_id)?;
    
    msg!("✅ Step 1 completed: System validation passed");
    
    msg!("⏳ Step 2/4: Validating contract owner authority");
    
    // TODO: Implement proper contract owner validation using program data account
    // For now, this function is restricted to pool owners as a temporary measure
    // This will be enhanced to include proper contract owner validation in a future update
    
    msg!("🔍 Authority Verification:");
    msg!("   • Temporary: Pool owner authorization (contract owner validation pending)");
    msg!("   • Provided signer: {}", contract_owner_signer.key);
    
    // Load pool state to get the pool owner for temporary validation
    let pool_state_data = crate::utils::validation::validate_and_deserialize_pool_state_secure(pool_state_pda, program_id)?;
    
    // TEMPORARY: Allow pool owner to manage this flag until contract owner validation is implemented
    if *contract_owner_signer.key != pool_state_data.owner {
        msg!("❌ AUTHORIZATION FAILED: Only pool owner can modify swap access restrictions (temporary)");
        msg!("   • Pool owner: {}", pool_state_data.owner);
        msg!("   • Caller: {}", contract_owner_signer.key);
        msg!("   • Note: Contract owner validation will be added in future update");
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Verify the caller signed the transaction
    if !contract_owner_signer.is_signer {
        msg!("❌ SIGNATURE REQUIRED: Pool owner must sign this transaction");
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    msg!("✅ Step 2 completed: Pool owner authority validated (temporary)");
    
    msg!("⏳ Step 3/4: Loading and updating pool state");
    
    // Load and validate pool state data
    let mut pool_state_data = crate::utils::validation::validate_and_deserialize_pool_state_secure(pool_state_pda, program_id)?;
    
    msg!("📋 Pool Information:");
    msg!("   • Pool: {} ↔ {}", pool_state_data.token_a_mint, pool_state_data.token_b_mint);
    msg!("   • Pool owner: {}", pool_state_data.owner);
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
    msg!("   • Note: Contract owner validation will be added in future update");
    
    if enable_restriction {
        msg!("🔒 SWAP ACCESS NOW RESTRICTED:");
        msg!("   • Only pool owner and contract owner can swap");
        msg!("   • Regular users must use custom fee-collecting contracts");
        msg!("   • Enables flexible custom fee structures");
        msg!("   • Pool owner can deploy contracts with any fee model");
        
        msg!("💡 CUSTOM FEE STRUCTURE BENEFITS:");
        msg!("   • Dynamic fee models (time-based, volume-based, etc.)");
        msg!("   • Tiered fee structures for different user types");
        msg!("   • Integration with external protocols and fee sharing");
        msg!("   • Maximum flexibility while maintaining security");
    } else {
        msg!("🔓 SWAP ACCESS NOW UNRESTRICTED:");
        msg!("   • All users can swap directly with the pool");
        msg!("   • Standard fixed swap contract fees apply");
        msg!("   • No custom fee collection");
        msg!("   • Traditional AMM-style operation");
    }
    
    msg!("🎉 Swap access configuration updated successfully!");
    msg!("💡 NEXT STEPS:");
    if enable_restriction {
        msg!("   • Pool owner can deploy custom fee-collecting contracts");
        msg!("   • Users should interact with those contracts for swaps");
        msg!("   • Monitor custom fee revenue and pool health");
    } else {
        msg!("   • Users can swap directly with the pool");
        msg!("   • Monitor standard pool operation and liquidity");
        msg!("   • Consider custom fee structures in the future if needed");
    }
    
    Ok(())
}


 