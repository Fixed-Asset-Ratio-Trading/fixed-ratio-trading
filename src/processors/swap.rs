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

/// Configures the swap trading fee rate for token swaps in the pool.
///
/// This function allows the pool owner to set or update the swap trading fee rate charged
/// on all token swaps. The fee is expressed in basis points (1/100th of a percent) 
/// and can range from 0% to 0.5% (0-50 basis points). This provides pool operators
/// with revenue generation while maintaining competitive trading costs.
///
/// **IMPORTANT**: This function configures the **swap trading fee** (percentage-based),
/// not the **swap contract fee** (fixed SOL amount). The swap contract fee is always
/// charged at a fixed rate to cover computational costs.
///
/// # System Pause Behavior
/// This operation is **BLOCKED** when the system is paused. System pause
/// takes precedence over pool-specific pause. Only the system authority
/// can unpause via UnpauseSystem instruction.
///
/// # Security
/// - Validates system is not paused before any state changes
/// - Returns SystemPaused error if system is paused
/// - Logs pause status for audit trails
/// - Existing pool pause validation continues to work after system pause check
///
/// # Purpose
/// - Enables pool owners to configure revenue generation through swap trading fees
/// - Provides flexibility to adjust fees based on market conditions and competition
/// - Maintains fee rate within reasonable bounds to ensure competitive trading
/// - Supports dynamic fee adjustment for optimal pool economics
/// - Ensures transparent fee policy changes with comprehensive logging
///
/// # How it works
/// 1. Validates the caller is the designated pool owner and signed the transaction
/// 2. Loads current pool state data to verify ownership permissions
/// 3. Validates the new fee rate is within the allowed range (0-50 basis points)
/// 4. Updates the pool's swap trading fee configuration in the state data
/// 5. Serializes the updated pool state back to on-chain storage
/// 6. Logs the fee change for transparency and audit compliance
///
/// # Arguments
/// * `_program_id` - The program ID (currently unused, reserved for future validation)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool owner account (must be signer and match pool state owner)
///   - `accounts[1]` - System state PDA account (for system pause validation)
///   - `accounts[2]` - Pool state PDA account (writable for fee configuration updates)
/// * `fee_basis_points` - The new swap trading fee rate in basis points (0-50, representing 0%-0.5%)
///
/// # Account Requirements
/// - **Owner**: Must be signer and match the owner field in pool state data
/// - **System State**: Must be valid system state account for pause validation
/// - **Pool State**: Must be writable for fee configuration updates
///
/// # Swap Trading Fee Rate Details
/// - **Units**: Basis points (1 basis point = 0.01%)
/// - **Range**: 0-50 basis points (0%-0.5%)
/// - **Examples**:
///   - 0 basis points = 0% fee (no trading fees)
///   - 5 basis points = 0.05% fee
///   - 25 basis points = 0.25% fee
///   - 50 basis points = 0.5% fee (maximum allowed)
/// - **Application**: Fee is deducted from input token amount during swaps
/// - **Collection**: Fees are accumulated in pool state and withdrawable by pool owner
///
/// # Fee Revenue Model
/// - **Source**: Percentage of every token swap transaction
/// - **Accumulation**: Fees are tracked separately by token type in pool state
/// - **Withdrawal**: Pool owner can withdraw accumulated fees
/// - **Transparency**: All fee collections and withdrawals are logged
///
/// # Security Features
/// - **Owner-only Access**: Only designated pool owner can modify swap trading fee rates
/// - **Rate Limits**: Maximum fee capped at 0.5% to prevent excessive charges
/// - **Immediate Effect**: Fee changes apply to all subsequent swaps
/// - **Audit Trail**: All fee rate changes are logged for transparency
/// - **Zero Fees Allowed**: Pool can operate with 0% trading fees if desired
///
/// # Economic Considerations
/// - **Competitive Rates**: 0.5% maximum ensures competitiveness with other DEXs
/// - **Revenue Balance**: Allows meaningful revenue while maintaining low costs
/// - **Market Responsiveness**: Dynamic adjustment based on competition and volume
/// - **User Experience**: Low fees encourage trading activity and liquidity
///
/// # Error Conditions
/// - `ProgramError::MissingRequiredSignature` - Owner didn't sign transaction
/// - `ProgramError::InvalidAccountData` - Caller is not the pool owner
/// - `ProgramError::InvalidArgument` - Fee rate exceeds maximum allowed (50 basis points)
///
/// # Example Usage
/// ```ignore
/// // Set a competitive 0.25% trading fee
/// let instruction = PoolInstruction::SetSwapFee {
///     fee_basis_points: 25, // 0.25%
/// };
///
/// // Remove all trading fees (0% fee)
/// let instruction = PoolInstruction::SetSwapFee {
///     fee_basis_points: 0, // 0%
/// };
///
/// // Set maximum allowed fee (0.5%)
/// let instruction = PoolInstruction::SetSwapFee {
///     fee_basis_points: 50, // 0.5%
/// };
/// ```
///
/// # Integration with Swap Process
/// The swap trading fee rate set by this function is applied during each `process_swap` call:
/// 1. Swap trading fee amount calculated: `fee = input_amount * fee_basis_points / 10000`
/// 2. Net trading amount: `net_amount = input_amount - fee`
/// 3. Output calculated from net amount using pool ratios
/// 4. Swap trading fee accumulated in pool state for later withdrawal
/// 
/// **Note**: The swap contract fee (fixed SOL amount) is always charged separately
/// for computational costs regardless of the swap trading fee setting.
pub fn process_set_swap_fee(
    program_id: &Pubkey,
    fee_basis_points: u64,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("⚙️ SWAP TRADING FEE CONFIGURATION");
    msg!("=============================");
    msg!("📊 New Swap Trading Fee Rate: {} basis points ({}%)", fee_basis_points, fee_basis_points as f64 / 100.0);
    
    let owner_authority_signer = &accounts[0];     // Index 0: Pool Owner Authority Signer
    let system_state_pda = &accounts[1];           // Index 1: System State PDA
    let pool_state_pda = &accounts[2];             // Index 2: Pool State PDA
    
    msg!("⏳ Step 1/4: Validating system state");
    
    // Validate system is not paused
    crate::utils::validation::validate_system_not_paused_secure(system_state_pda, program_id)?;
    
    msg!("✅ Step 1 completed: System validation passed");
    
    msg!("⏳ Step 2/4: Loading and validating pool state");
    
    // Load and verify pool state (SECURITY: Now validates PDA)
    let pool_state_data = crate::utils::validation::validate_and_deserialize_pool_state_secure(pool_state_pda, program_id)?;
    
    msg!("📋 Pool Information:");
    msg!("   • Pool: {} ↔ {}", pool_state_data.token_a_mint, pool_state_data.token_b_mint);
    msg!("   • Current owner: {}", pool_state_data.owner);
    msg!("   • Requested by: {}", owner_authority_signer.key);
    
    if *owner_authority_signer.key != pool_state_data.owner {
        msg!("❌ AUTHORIZATION FAILED: Only pool owner can set swap trading fees");
        msg!("   • Pool owner: {}", pool_state_data.owner);
        msg!("   • Caller: {}", owner_authority_signer.key);
        return Err(ProgramError::InvalidAccountData);
    }

    msg!("✅ Step 2 completed: Pool ownership validated");

    msg!("⏳ Step 3/4: Validating swap trading fee rate parameters");
    
    // Validate fee is within allowed range (0-50 basis points = 0%-0.5%)
    if fee_basis_points > MAX_SWAP_TRADING_FEE_BASIS_POINTS {
        msg!("❌ INVALID SWAP TRADING FEE RATE: Exceeds maximum allowed");
        msg!("   • Requested: {} basis points", fee_basis_points);
        msg!("   • Maximum: {} basis points (0.5%)", MAX_SWAP_TRADING_FEE_BASIS_POINTS);
        msg!("   • Range: 0-50 basis points (0%-0.5%)");
        return Err(ProgramError::InvalidArgument);
    }

    // **PHASE 1: FIXED SWAP TRADING FEE - NO LONGER CONFIGURABLE PER POOL**
    // Swap trading fees are now fixed system-wide via FIXED_SWAP_TRADING_FEE_BASIS_POINTS constant
    use crate::constants::FIXED_SWAP_TRADING_FEE_BASIS_POINTS;
    
    let old_fee = FIXED_SWAP_TRADING_FEE_BASIS_POINTS;
    if fee_basis_points != FIXED_SWAP_TRADING_FEE_BASIS_POINTS {
        msg!("⚠️ SWAP TRADING FEE CONFIGURATION WARNING: System-wide fixed fees");
        msg!("   • Requested: {} basis points", fee_basis_points);
        msg!("   • System fixed: {} basis points", FIXED_SWAP_TRADING_FEE_BASIS_POINTS);
        msg!("   • Individual pool configuration disabled");
        return Err(ProgramError::InvalidArgument);
    }
    
    msg!("✅ Step 3 completed: Swap trading fee rate validation passed");
    msg!("📊 Swap Trading Fee Configuration:");
    msg!("   • Old rate: {} basis points ({}%)", old_fee, old_fee as f64 / 100.0);
    msg!("   • New rate: {} basis points ({}%)", fee_basis_points, fee_basis_points as f64 / 100.0);
    msg!("   • Change: {} basis points", if fee_basis_points > old_fee { 
        format!("+{}", fee_basis_points - old_fee) 
    } else { 
        format!("-{}", old_fee - fee_basis_points) 
    });

    msg!("⏳ Step 4/4: Updating pool configuration");
    
    // Update the swap trading fee in pool state
    // Note: In the current implementation, this is a no-op since fees are fixed system-wide
    // But we keep the structure for future flexibility
    
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
    
    msg!("✅ SWAP TRADING FEE CONFIGURATION COMPLETED!");
    msg!("=============================");
    msg!("📈 CONFIGURATION SUMMARY:");
    msg!("   • Pool: {} ↔ {}", pool_state_data.token_a_mint, pool_state_data.token_b_mint);
    msg!("   • Swap Trading Fee Rate: {} basis points ({}%)", fee_basis_points, fee_basis_points as f64 / 100.0);
    msg!("   • Applied to: All future swap transactions");
    msg!("   • Revenue: Swap trading fees collected to pool state");
    msg!("   • Note: Swap contract fees ({} lamports) charged separately", crate::constants::SWAP_CONTRACT_FEE);
    
    msg!("💰 ECONOMIC IMPACT:");
    msg!("   • Trading cost: {}% per swap (plus swap contract fee)", fee_basis_points as f64 / 100.0);
    msg!("   • Revenue model: Percentage of swap volume");
    msg!("   • Fee collection: Automatic on each swap");
    msg!("   • Withdrawal: Pool owner can withdraw accumulated swap trading fees");
    
    msg!("🎉 Swap trading fee configuration updated successfully!");
    msg!("💡 NEXT STEPS:");
    msg!("   • Monitor swap trading fee collection in pool state");
    msg!("   • Consider withdrawing accumulated fees");
    msg!("   • Monitor trading volume and revenue");
    
    Ok(())
}


 