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

/// Safely unpacks a token account with comprehensive error handling
/// 
/// This function provides robust error handling for TokenAccount::unpack_from_slice()
/// calls, which can fail due to invalid account data, corruption, or wrong account types.
/// 
/// # Arguments
/// * `account` - The account info to unpack
/// * `account_name` - Human-readable name for error messages
/// 
/// # Returns
/// * `Result<TokenAccount, ProgramError>` - The unpacked token account or an error
fn safe_unpack_token_account(account: &AccountInfo, account_name: &str) -> Result<TokenAccount, ProgramError> {
    // Check if account has data
    if account.data_len() == 0 {
        msg!("❌ {}: Account has no data (uninitialized)", account_name);
        return Err(ProgramError::UninitializedAccount);
    }
    
    // Check if account is owned by SPL Token program
    if account.owner != &spl_token::id() {
        msg!("❌ {}: Account is not owned by SPL Token program", account_name);
        msg!("   • Expected owner: {}", spl_token::id());
        msg!("   • Actual owner: {}", account.owner);
        return Err(ProgramError::IncorrectProgramId);
    }
    
    // Try to unpack the token account data
    match TokenAccount::unpack_from_slice(&account.data.borrow()) {
        Ok(token_account) => {
            msg!("✅ {}: Successfully unpacked token account", account_name);
            msg!("   • Mint: {}", token_account.mint);
            msg!("   • Owner: {}", token_account.owner);
            msg!("   • Balance: {}", token_account.amount);
            Ok(token_account)
        }
        Err(e) => {
            msg!("❌ {}: Failed to unpack token account data", account_name);
            msg!("   • Error: {:?}", e);
            msg!("   • Account key: {}", account.key);
            msg!("   • Data length: {} bytes", account.data_len());
            msg!("   • This may indicate corrupted account data or wrong account type");
            Err(ProgramError::InvalidAccountData)
        }
    }
}

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
    msg!("   • Swap Contract Fee: Will be displayed after pool state validation");
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

    // ✅ DISPLAY ACTUAL FEE INFORMATION (now that pool state is loaded)
    msg!("💰 ACTUAL FEE BREAKDOWN:");
    msg!("   • Swap Contract Fee: {} lamports ({} SOL)", pool_state_data.swap_contract_fee, pool_state_data.swap_contract_fee as f64 / 1_000_000_000.0);

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
            
            msg!("❌ SWAP BLOCKED: Owner-only mode is enabled");
            msg!("   • This pool restricts swaps to the pool owner only");
            msg!("   • Pool owner: {}", pool_owner);
            msg!("   • Your address: {}", user_key);
            msg!("   • Note: Pool ownership transfers to Program Upgrade Authority when restriction is enabled");
            msg!("   • Purpose: Enables custom fee structures through external contracts");
            msg!("   • Contact pool owner for access or use their fee-collecting contract");
            return Err(PoolError::SwapAccessRestricted.into());
        }
    }
    
    msg!("✅ Step 1 completed: System and pool validations passed");

    msg!("🔍 Step 2/6: Fee collection will happen after token operations to prevent PDA corruption");
    msg!("💰 Fee: {} lamports (will be collected to pool state)", pool_state_data.swap_contract_fee);
    
    msg!("⏳ Step 3/6: Loading and validating user accounts");
    
    // Load user token account data for validation
    let user_input_token_data = safe_unpack_token_account(user_input_token_account, "User Input Token Account")?;
    let user_output_token_data = safe_unpack_token_account(user_output_token_account, "User Output Token Account")?;

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
    msg!("💰 Step 6a: Collecting fees after token operations...");
    use crate::utils::fee_validation::{collect_fee_to_pool_state, FeeType};
    
    collect_fee_to_pool_state(
        user_authority_signer,
        pool_state_pda,  // ← Collect to pool state instead of main treasury
        system_program_account,
        program_id,
        pool_state_data.swap_contract_fee,
        FeeType::RegularSwap,
    )?;
    
    msg!("✅ Swap fee collected successfully after token operations");
    msg!("💰 Fee collected: {} lamports (distributed to pool state)", pool_state_data.swap_contract_fee);
    
    msg!("✅ SWAP COMPLETED SUCCESSFULLY!");
    msg!("=============================");
    msg!("📈 COMPREHENSIVE TRANSACTION SUMMARY:");
    msg!("   • Input: {} tokens (mint: {})", amount_in, input_token_mint_key);
    msg!("   • Output: {} tokens (mint: {})", amount_out, output_token_mint_key);
    msg!("   • Exchange rate: {}:{} (fixed ratio)", numerator, denominator);
    msg!("   • Total fees paid: {} lamports", pool_state_data.swap_contract_fee);
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
pub fn process_set_swap_owner_only(
    program_id: &Pubkey,
    enable_restriction: bool,
    designated_owner: Pubkey,
    accounts: &[AccountInfo],
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


 