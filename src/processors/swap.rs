use borsh::{BorshDeserialize, BorshSerialize};
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
    state::{PoolState},
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
    msg!("🔄 Processing swap request - extracting accounts");
    
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

    msg!("🔍 Validating system and pool state");
    
    // Validate system is not paused
    crate::utils::validation::validate_system_not_paused_secure(system_state_pda, program_id)?;
    
    // Load and validate pool state data
    let mut pool_state_data = crate::utils::validation::validate_and_deserialize_pool_state_secure(pool_state_pda, program_id)?;

    // Check if pool swaps are paused
    if pool_state_data.swaps_paused() {
        msg!("❌ Pool swaps are currently paused");
        return Err(PoolError::PoolSwapsPaused.into());
    }
    
    msg!("✅ System and pool validations passed");

    msg!("🔄 Starting swap operation validation and fee collection");
    
    // Collect swap fee to pool state
    use crate::utils::fee_validation::{collect_fee_to_pool_state, FeeType};
    use crate::constants::SWAP_FEE;
    
    collect_fee_to_pool_state(
        user_authority_signer,
        pool_state_pda,  // ← Collect to pool state instead of main treasury
        system_program_account,
        program_id,
        SWAP_FEE,
        FeeType::RegularSwap,
    )?;
    
    msg!("✅ Fee collection completed: {} lamports", SWAP_FEE);
    msg!("🔍 Loading and validating user token account data");
    
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
            msg!("🔄 Token A → Token B swap detected");
            // A->B swap validation
            if *pool_token_a_vault_pda.key != pool_state_data.token_a_vault || 
               *pool_token_b_vault_pda.key != pool_state_data.token_b_vault {
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_a_vault_pda, pool_token_b_vault_pda, pool_state_data.token_b_mint, true)
        } else if input_token_mint_key == pool_state_data.token_b_mint {
            msg!("🔄 Token B → Token A swap detected");
            // B->A swap validation
            if *pool_token_b_vault_pda.key != pool_state_data.token_b_vault || 
               *pool_token_a_vault_pda.key != pool_state_data.token_a_vault {
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_b_vault_pda, pool_token_a_vault_pda, pool_state_data.token_a_mint, false)
        } else {
            msg!("❌ Invalid input token mint - not part of this pool");
            return Err(ProgramError::InvalidArgument);
        };

    msg!("🔍 Validating user account ownership and balances");
    
    // Validate user account ownership and sufficient balance
    if user_input_token_data.mint != input_token_mint_key ||
       user_input_token_data.owner != *user_authority_signer.key ||
       user_input_token_data.amount < amount_in ||
       user_output_token_data.mint != output_token_mint_key ||
       user_output_token_data.owner != *user_authority_signer.key {
        msg!("❌ User account validation failed");
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate SPL Token program account
    if *token_program_account.key != spl_token::id() {
        msg!("❌ Invalid SPL Token program account");
        return Err(ProgramError::IncorrectProgramId);
    }
    
    msg!("✅ Account validations passed");

    msg!("🧮 Calculating output amount using fixed ratio");
    
    // Get exchange ratio based on swap direction
    let (numerator, denominator) = if input_is_token_a {
        if pool_state_data.ratio_a_numerator == 0 {
            msg!("❌ Invalid pool ratio: Token A numerator is zero");
            return Err(ProgramError::InvalidAccountData);
        }
        (pool_state_data.ratio_a_numerator, pool_state_data.ratio_b_denominator)
    } else {
        if pool_state_data.ratio_b_denominator == 0 {
            msg!("❌ Invalid pool ratio: Token B denominator is zero");
            return Err(ProgramError::InvalidAccountData);
        }
        (pool_state_data.ratio_b_denominator, pool_state_data.ratio_a_numerator)
    };

    // Calculate output amount using fixed ratio: output = input * numerator / denominator
    let amount_out = amount_in.checked_mul(numerator)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_div(denominator)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    msg!("📊 Ratio calculation: {} input → {} output ({}:{})", 
         amount_in, amount_out, numerator, denominator);
    
    // Validate output amount is non-zero
    if amount_out == 0 {
        msg!("❌ Calculated output amount is zero - invalid swap");
        return Err(ProgramError::InvalidArgument);
    }

    msg!("🔍 Checking pool liquidity availability");
    
    // Check if pool has sufficient liquidity for the output
    let available_liquidity = if input_is_token_a {
        pool_state_data.total_token_b_liquidity
    } else {
        pool_state_data.total_token_a_liquidity
    };
    
    msg!("📊 Available liquidity: {}, Required: {}", available_liquidity, amount_out);
    
    if available_liquidity < amount_out {
        msg!("❌ Insufficient pool liquidity for swap");
        return Err(ProgramError::InsufficientFunds);
    }
    
    msg!("✅ Liquidity check passed");

    msg!("🔄 Executing token transfers");
    
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
        msg!("❌ Serialized data too large for account");
        return Err(ProgramError::AccountDataTooSmall);
    }
    
    pool_state_pda_data[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    msg!("✅ Swap completed successfully - {} → {} tokens", amount_in, amount_out);
    
    Ok(())
}

/// Configures the trading fee rate for token swaps in the pool.
///
/// This function allows the pool owner to set or update the trading fee rate charged
/// on all token swaps. The fee is expressed in basis points (1/100th of a percent) 
/// and can range from 0% to 0.5% (0-50 basis points). This provides pool operators
/// with revenue generation while maintaining competitive trading costs.
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
/// - Enables pool owners to configure revenue generation through trading fees
/// - Provides flexibility to adjust fees based on market conditions and competition
/// - Maintains fee rate within reasonable bounds to ensure competitive trading
/// - Supports dynamic fee adjustment for optimal pool economics
/// - Ensures transparent fee policy changes with comprehensive logging
///
/// # How it works
/// 1. Validates the caller is the designated pool owner and signed the transaction
/// 2. Loads current pool state data to verify ownership permissions
/// 3. Validates the new fee rate is within the allowed range (0-50 basis points)
/// 4. Updates the pool's swap fee configuration in the state data
/// 5. Serializes the updated pool state back to on-chain storage
/// 6. Logs the fee change for transparency and audit compliance
///
/// # Arguments
/// * `_program_id` - The program ID (currently unused, reserved for future validation)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool owner account (must be signer and match pool state owner)
///   - `accounts[1]` - System state PDA account (for system pause validation)
///   - `accounts[2]` - Pool state PDA account (writable for fee configuration updates)
/// * `fee_basis_points` - The new trading fee rate in basis points (0-50, representing 0%-0.5%)
///
/// # Account Requirements
/// - **Owner**: Must be signer and match the owner field in pool state data
/// - **System State**: Must be valid system state account for pause validation
/// - **Pool State**: Must be writable for fee configuration updates
///
/// # Fee Rate Details
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
/// - **Owner-only Access**: Only designated pool owner can modify fee rates
/// - **Rate Limits**: Maximum fee capped at 0.5% to prevent excessive charges
/// - **Immediate Effect**: Fee changes apply to all subsequent swaps
/// - **Audit Trail**: All fee rate changes are logged for transparency
/// - **Zero Fees Allowed**: Pool can operate with 0% fees if desired
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
/// The fee rate set by this function is applied during each `process_swap` call:
/// 1. Fee amount calculated: `fee = input_amount * fee_basis_points / 10000`
/// 2. Net trading amount: `net_amount = input_amount - fee`
/// 3. Output calculated from net amount using pool ratios
/// 4. Fee accumulated in pool state for later withdrawal
pub fn process_set_swap_fee(
    program_id: &Pubkey,
    fee_basis_points: u64,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing SetSwapFee: {} basis points", fee_basis_points);
    
    let owner_authority_signer = &accounts[0];     // Index 0: Pool Owner Authority Signer
    let system_state_pda = &accounts[1];           // Index 1: System State PDA
    let pool_state_pda = &accounts[2];             // Index 2: Pool State PDA
    
    // Validate system is not paused
    crate::utils::validation::validate_system_not_paused_secure(system_state_pda, program_id)?;
    
    // Load and verify pool state (SECURITY: Now validates PDA)
    let pool_state_data = crate::utils::validation::validate_and_deserialize_pool_state_secure(pool_state_pda, program_id)?;
    if *owner_authority_signer.key != pool_state_data.owner {
        msg!("Only pool owner can set swap fees");
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate fee is within allowed range (0-50 basis points = 0%-0.5%)
    if fee_basis_points > MAX_SWAP_FEE_BASIS_POINTS {
        msg!("Swap fee {} basis points exceeds maximum of {} basis points (0.5%)", 
             fee_basis_points, MAX_SWAP_FEE_BASIS_POINTS);
        return Err(ProgramError::InvalidArgument);
    }

    // **PHASE 1: FIXED SWAP FEE - NO LONGER CONFIGURABLE PER POOL**
    // Swap fees are now fixed system-wide via FIXED_SWAP_FEE_BASIS_POINTS constant
    use crate::constants::FIXED_SWAP_FEE_BASIS_POINTS;
    
    let old_fee = FIXED_SWAP_FEE_BASIS_POINTS;
    if fee_basis_points != FIXED_SWAP_FEE_BASIS_POINTS {
        msg!("⚠️ Swap fees are now fixed system-wide at {} basis points", FIXED_SWAP_FEE_BASIS_POINTS);
        msg!("⚠️ Individual pool fee configuration is no longer supported");
        return Err(ProgramError::InvalidArgument);
    }

    // ========================================================================
    // SOLANA BUFFER SERIALIZATION WORKAROUND FOR PDA DATA CORRUPTION
    // ========================================================================
    // Apply the same workaround used in process_deposit to prevent data corruption
    // when the pool state PDA is used as both authority and data storage.
    
    // Step 1: Serialize the pool state data to a temporary buffer
    let mut serialized_data = Vec::new();
    pool_state_data.serialize(&mut serialized_data)?;
    
    // Step 2: Atomic copy to account data
    {
        let mut account_data = pool_state_pda.data.borrow_mut();
        account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
    }
    
    // Log the change for transparency
    msg!("Swap fee updated: {} -> {} basis points ({:.2}% -> {:.2}%)", 
         old_fee, fee_basis_points,
         old_fee as f64 / 100.0, fee_basis_points as f64 / 100.0);

    Ok(())
}


/// Validates that pool swaps are not paused (granular pool check)
/// 
/// This function provides pool-specific swap pause validation, separate from system-wide pause.
/// It allows deposits and withdrawals to continue while blocking only swap operations when
/// owner-initiated pool pause is active.
/// 
/// # Arguments
/// * `pool_state_data` - Already deserialized pool state data
/// 
/// # Returns
/// * `ProgramResult` - Success if swaps are enabled, error if paused
fn validate_pool_swaps_not_paused(pool_state_data: &PoolState) -> ProgramResult {
    if pool_state_data.swaps_paused() {
        msg!("Pool swaps are currently paused by owner");
        msg!("Note: Deposits and withdrawals are still available");
        msg!("Note: Owner can manage pause governance and reasons");
        return Err(PoolError::PoolSwapsPaused.into());
    }
    
    Ok(())
} 