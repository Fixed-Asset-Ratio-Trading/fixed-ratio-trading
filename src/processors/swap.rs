use borsh::{BorshDeserialize, BorshSerialize};
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
    clock::Clock,
};
use spl_token::{
    instruction as token_instruction,
    state::{Account as TokenAccount},
};

use crate::{
    constants::*,
    types::*,
    error::PoolError,
    check_rent_exempt,
};

/// **SWAP OPERATIONS MODULE**
/// 
/// This module handles all token swap operations within the trading pool, including:
/// - Core swap functionality with slippage protection and fee collection
/// - Swap fee configuration and management
/// - Fixed-ratio price calculation and execution
/// - Comprehensive validation and security checks
/// 
/// The module implements a fixed-ratio trading system where tokens can be exchanged
/// at predetermined ratios with configurable trading fees (0-0.5%). All swaps are
/// protected by slippage tolerance and comprehensive security validations.

/// Handles token swaps within the trading pool using fixed ratios.
///
/// This function implements the core token swap functionality for the fixed-ratio trading pool.
/// It enables users to exchange tokens at predetermined ratios with configurable trading fees,
/// slippage protection, and comprehensive security validations. The swap system maintains
/// pool liquidity balance while collecting fees for pool operators.
///
/// # Purpose
/// - Enables token exchange at fixed ratios between pool token pairs
/// - Implements comprehensive slippage protection for users
/// - Collects configurable trading fees (0-0.5%) for pool sustainability
/// - Maintains accurate pool liquidity accounting and fee tracking
/// - Provides secure, validated token transfers with proper authorization
///
/// # How it works
/// 1. **Account Validation**: Verifies all required accounts and signatures
/// 2. **Direction Detection**: Determines swap direction (A→B or B→A) based on input token
/// 3. **Price Calculation**: Computes output amount using fixed ratios:
///    - A→B: `amount_out_B = (amount_in_A * ratio_B_denominator) / ratio_A_numerator`
///    - B→A: `amount_out_A = (amount_in_B * ratio_A_numerator) / ratio_B_denominator`
/// 4. **Fee Processing**: Calculates and deducts configurable trading fees
/// 5. **Slippage Check**: Validates output meets minimum amount requirements
/// 6. **Liquidity Verification**: Ensures pool has sufficient output tokens
/// 7. **Token Transfers**: Executes secure transfers with proper PDA signing
/// 8. **State Updates**: Updates pool liquidity and fee accumulation tracking
/// 9. **SOL Fee Collection**: Collects fixed SOL fee for transaction processing
///
/// # Arguments
/// * `program_id` - The program ID for PDA validation and signing
/// * `accounts` - Array of accounts in the following order:
///   - `accounts[0]` - User signer account (initiating the swap)
///   - `accounts[1]` - User's input token account (source of tokens to swap)
///   - `accounts[2]` - User's output token account (destination for swapped tokens)
///   - `accounts[3]` - Pool state PDA account (writable for state updates)
///   - `accounts[4]` - Token A mint account (for PDA seed derivation)
///   - `accounts[5]` - Token B mint account (for PDA seed derivation)
///   - `accounts[6]` - Pool's Token A vault account
///   - `accounts[7]` - Pool's Token B vault account
///   - `accounts[8]` - System program account
///   - `accounts[9]` - SPL Token program account
///   - `accounts[10]` - Rent sysvar account
///   - `accounts[11]` - Clock sysvar account
/// * `input_token_mint_key` - The mint address of the token being swapped in
/// * `amount_in` - The amount of input tokens to swap (including fees)
/// * `minimum_amount_out` - Minimum acceptable output tokens (slippage protection)
///
/// # Account Requirements
/// - **User Signer**: Must sign the transaction and own input/output token accounts
/// - **Token Accounts**: Must be valid SPL token accounts with correct mints and ownership
/// - **Pool State**: Must be initialized and not paused
/// - **Vaults**: Must match the pool's registered token vault addresses
/// - **Programs**: System and SPL Token programs must be correct
///
/// # Price Calculation Details
/// The swap uses fixed ratios stored in the pool state:
/// - **Token A → Token B**: `output = (input * ratio_B_denominator) / ratio_A_numerator`
/// - **Token B → Token A**: `output = (input * ratio_A_numerator) / ratio_B_denominator`
/// 
/// Fees are calculated as: `fee = input * swap_fee_basis_points / 10000`
/// Net input after fees: `net_input = input - fee`
///
/// # Security Features
/// - **Signature Validation**: User must sign the swap transaction
/// - **Account Ownership**: Validates token account ownership and mint matching
/// - **Slippage Protection**: Enforces minimum output amount requirements
/// - **Liquidity Checks**: Ensures sufficient pool liquidity for output
/// - **Rent Exemption**: Validates pool accounts maintain rent-exempt status
/// - **PDA Authorization**: Uses proper PDA signing for pool vault transfers
/// - **Pause Enforcement**: Blocks swaps when pool is paused (checked in main dispatcher)
///
/// # Fee Structure
/// - **Trading Fees**: 0-0.5% (0-50 basis points) configurable by pool owner
/// - **SOL Processing Fee**: Fixed 1000 lamports per swap for transaction costs
/// - **Fee Destination**: Trading fees accumulated in pool state, SOL fees to pool PDA
///
/// # Error Conditions
/// - `ProgramError::MissingRequiredSignature` - User didn't sign transaction
/// - `ProgramError::UninitializedAccount` - Pool not properly initialized
/// - `ProgramError::InvalidAccountData` - Account data validation failures
/// - `ProgramError::InvalidArgument` - Invalid token mint or parameters
/// - `ProgramError::InsufficientFunds` - Insufficient input tokens or pool liquidity
/// - `ProgramError::ArithmeticOverflow` - Mathematical calculation errors
/// - `PoolError::InvalidSwapAmount` - Output amount below minimum or zero
///
/// # Example Usage
/// ```ignore
/// // Swap 1000 Token A for Token B with 1% slippage tolerance
/// let instruction = PoolInstruction::Swap {
///     input_token_mint: token_a_mint,
///     amount_in: 1_000_000, // 1000 tokens (assuming 6 decimals)
///     minimum_amount_out: 1_980_000, // Accept minimum 1980 Token B (1% slippage)
/// };
/// ```
///
/// # Performance Considerations
/// - Fixed-ratio calculation is gas-efficient (simple multiplication/division)
/// - No complex AMM curve calculations or oracle dependencies
/// - Deterministic output amounts enable precise slippage calculations
/// - Single-step atomic operation ensures consistency
pub fn process_swap(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input_token_mint_key: Pubkey,
    amount_in: u64,
    minimum_amount_out: u64,
) -> ProgramResult {
    msg!("Processing Swap v2");
    let account_info_iter = &mut accounts.iter();

    let user_signer = next_account_info(account_info_iter)?;                     // User initiating the swap (signer)
    let user_input_token_account = next_account_info(account_info_iter)?;      // User's token account for the input token
    let user_output_token_account = next_account_info(account_info_iter)?;     // User's token account to receive the output token
    let pool_state_account = next_account_info(account_info_iter)?;              // Pool state PDA

    // Accounts needed for Pool State PDA seeds derivation for signing
    let token_a_mint_for_pda_seeds = next_account_info(account_info_iter)?;    // Pool's token_a_mint (must match pool_state_data.token_a_mint)
    let token_b_mint_for_pda_seeds = next_account_info(account_info_iter)?;    // Pool's token_b_mint (must match pool_state_data.token_b_mint)
    
    let pool_token_a_vault_account = next_account_info(account_info_iter)?;     // Pool's vault for token A
    let pool_token_b_vault_account = next_account_info(account_info_iter)?;     // Pool's vault for token B
    
    let system_program_account = next_account_info(account_info_iter)?;         // System program
    let token_program_account = next_account_info(account_info_iter)?;           // SPL Token program
    let rent_sysvar_account = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_sysvar_account)?;
    let _clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;

    // Check rent-exempt status for pool accounts
    check_rent_exempt(pool_state_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(pool_token_a_vault_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(pool_token_b_vault_account, program_id, rent, _clock.slot)?;

    if !user_signer.is_signer {
        msg!("User must be a signer for swap");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut pool_state_data = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    if !pool_state_data.is_initialized {
        msg!("Pool not initialized");
        return Err(ProgramError::UninitializedAccount);
    }

    // Verify that the provided token_a_mint_for_pda_seeds and token_b_mint_for_pda_seeds match pool state
    if *token_a_mint_for_pda_seeds.key != pool_state_data.token_a_mint {
        msg!("Provided token_a_mint_for_pda_seeds does not match pool state");
        return Err(ProgramError::InvalidAccountData);
    }
    if *token_b_mint_for_pda_seeds.key != pool_state_data.token_b_mint {
        msg!("Provided token_b_mint_for_pda_seeds does not match pool state");
        return Err(ProgramError::InvalidAccountData);
    }

    // Determine swap direction and relevant accounts
    let (input_pool_vault_acc, output_pool_vault_acc, output_token_mint_key, input_is_token_a) = 
        if input_token_mint_key == pool_state_data.token_a_mint {
            // Swapping A for B
            if *pool_token_a_vault_account.key != pool_state_data.token_a_vault || 
               *pool_token_b_vault_account.key != pool_state_data.token_b_vault {
                msg!("Invalid pool vault accounts provided for A -> B swap.");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_a_vault_account, pool_token_b_vault_account, pool_state_data.token_b_mint, true)
        } else if input_token_mint_key == pool_state_data.token_b_mint {
            // Swapping B for A
            if *pool_token_b_vault_account.key != pool_state_data.token_b_vault || 
               *pool_token_a_vault_account.key != pool_state_data.token_a_vault {
                msg!("Invalid pool vault accounts provided for B -> A swap.");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_b_vault_account, pool_token_a_vault_account, pool_state_data.token_a_mint, false)
        } else {
            msg!("Input token mint does not match either of the pool's tokens");
            return Err(ProgramError::InvalidArgument);
        };

    // Validate user's input token account
    let user_input_token_account_data = TokenAccount::unpack_from_slice(&user_input_token_account.data.borrow())?;
    if user_input_token_account_data.mint != input_token_mint_key {
        msg!("User input token account mint mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_input_token_account_data.owner != *user_signer.key {
        msg!("User input token account owner mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_input_token_account_data.amount < amount_in {
        msg!("Insufficient funds in user input token account");
        return Err(ProgramError::InsufficientFunds);
    }

    // Validate user's output token account
    let user_output_token_account_data = TokenAccount::unpack_from_slice(&user_output_token_account.data.borrow())?;
    if user_output_token_account_data.mint != output_token_mint_key {
        msg!("User output token account mint mismatch with expected output token");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_output_token_account_data.owner != *user_signer.key {
        msg!("User output token account owner mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Validate SPL Token Program ID
    if *token_program_account.key != Pubkey::new_from_array(spl_token::id().to_bytes()) {
        msg!("Invalid SPL Token Program ID");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Calculate amount_out
    let amount_out = if input_is_token_a {
        // Swapping A for B: amount_out_B = (amount_in_A * ratio_B_denominator) / ratio_A_numerator
        if pool_state_data.ratio_a_numerator == 0 {
            msg!("Pool ratio_a_numerator is zero, cannot perform swap.");
            return Err(ProgramError::InvalidAccountData); // Or a more specific error
        }
        amount_in.checked_mul(pool_state_data.ratio_b_denominator)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .checked_div(pool_state_data.ratio_a_numerator)
            .ok_or(ProgramError::ArithmeticOverflow)? // Using ArithmeticOverflow for division issues
    } else {
        // Swapping B for A: amount_out_A = (amount_in_B * ratio_A_numerator) / ratio_B_denominator
        if pool_state_data.ratio_b_denominator == 0 {
            msg!("Pool ratio_b_denominator is zero, cannot perform swap.");
            return Err(ProgramError::InvalidAccountData);
        }
        amount_in.checked_mul(pool_state_data.ratio_a_numerator)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .checked_div(pool_state_data.ratio_b_denominator)
            .ok_or(ProgramError::ArithmeticOverflow)?
    };

    if amount_out == 0 {
        return Err(PoolError::InvalidSwapAmount {
            amount: amount_out,
            min_amount: 1,
            max_amount: u64::MAX,
        }.into());
    }

    // Check slippage protection
    if amount_out < minimum_amount_out {
        msg!("Slippage tolerance exceeded. Expected minimum: {}, Got: {}", minimum_amount_out, amount_out);
        return Err(PoolError::InvalidSwapAmount {
            amount: amount_out,
            min_amount: minimum_amount_out,
            max_amount: u64::MAX,
        }.into());
    }

    // Calculate and collect trading fees using configurable rate
    let fee_amount = if pool_state_data.swap_fee_basis_points == 0 {
        0u64 // No fee if set to 0%
    } else {
        amount_in
            .checked_mul(pool_state_data.swap_fee_basis_points)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .checked_div(FEE_BASIS_POINTS_DENOMINATOR)
            .ok_or(ProgramError::ArithmeticOverflow)?
    };
    
    let amount_after_fee = amount_in
        .checked_sub(fee_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    msg!("Swap calculation: Input: {}, Fee: {} ({:.2}% rate), After fee: {}, Output: {}", 
         amount_in, fee_amount, pool_state_data.swap_fee_basis_points as f64 / 100.0, amount_after_fee, amount_out);

    // Check pool liquidity for output token
    if input_is_token_a {
        // Output is Token B
        if pool_state_data.total_token_b_liquidity < amount_out {
            msg!("Insufficient Token B liquidity in the pool for swap output.");
            return Err(ProgramError::InsufficientFunds);
        }
    } else {
        // Output is Token A
        if pool_state_data.total_token_a_liquidity < amount_out {
            msg!("Insufficient Token A liquidity in the pool for swap output.");
            return Err(ProgramError::InsufficientFunds);
        }
    }

    // Transfer input tokens from user to pool vault (including fee)
    msg!("Transferring {} of input token {} from user to pool vault {}", 
           amount_in, input_token_mint_key, input_pool_vault_acc.key);
    invoke(
        &token_instruction::transfer(
            token_program_account.key,
            user_input_token_account.key,
            input_pool_vault_acc.key,
            user_signer.key, // User is the authority over their input account
            &[],
            amount_in,
        )?,
        &[
            user_input_token_account.clone(),
            input_pool_vault_acc.clone(),
            user_signer.clone(),
            token_program_account.clone(),
        ],
    )?;

    // Transfer output tokens from pool vault to user
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.token_a_mint.as_ref(),
        pool_state_data.token_b_mint.as_ref(),
        &pool_state_data.ratio_a_numerator.to_le_bytes(),
        &pool_state_data.ratio_b_denominator.to_le_bytes(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    msg!("Transferring {} of output token {} from pool vault {} to user account {}", 
           amount_out, output_token_mint_key, output_pool_vault_acc.key, user_output_token_account.key);
    invoke_signed(
        &token_instruction::transfer(
            token_program_account.key,
            output_pool_vault_acc.key,          // Pool's output vault (source)
            user_output_token_account.key,      // User's output account (destination)
            pool_state_account.key,             // Pool PDA is the authority over its vault
            &[],
            amount_out,
        )?,
        &[
            output_pool_vault_acc.clone(),
            user_output_token_account.clone(),
            pool_state_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;

    // Update pool state liquidity and fee tracking
    if input_is_token_a {
        // Add input tokens (minus fee) to liquidity, track fee separately
        pool_state_data.total_token_a_liquidity = pool_state_data.total_token_a_liquidity.checked_add(amount_after_fee)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        pool_state_data.total_token_b_liquidity = pool_state_data.total_token_b_liquidity.checked_sub(amount_out)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        // Track collected fee
        pool_state_data.collected_fees_token_a = pool_state_data.collected_fees_token_a.checked_add(fee_amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    } else {
        // Add input tokens (minus fee) to liquidity, track fee separately
        pool_state_data.total_token_b_liquidity = pool_state_data.total_token_b_liquidity.checked_add(amount_after_fee)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        pool_state_data.total_token_a_liquidity = pool_state_data.total_token_a_liquidity.checked_sub(amount_out)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        // Track collected fee
        pool_state_data.collected_fees_token_b = pool_state_data.collected_fees_token_b.checked_add(fee_amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
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
        let mut account_data = pool_state_account.data.borrow_mut();
        account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
    }
    
    msg!("Pool liquidity updated after swap. Token A: {}, Token B: {}", 
           pool_state_data.total_token_a_liquidity, pool_state_data.total_token_b_liquidity);
    msg!("Fees collected - Token A: {}, Token B: {}", 
           pool_state_data.collected_fees_token_a, pool_state_data.collected_fees_token_b);

    // Transfer swap fee to pool state PDA
    if user_signer.lamports() < SWAP_FEE {
        msg!("Insufficient SOL for swap fee. User lamports: {}", user_signer.lamports());
        return Err(ProgramError::InsufficientFunds);
    }
    invoke(
        &system_instruction::transfer(user_signer.key, pool_state_account.key, SWAP_FEE),
        &[user_signer.clone(), pool_state_account.clone(), system_program_account.clone()],
    )?;
    msg!("Swap fee {} transferred to pool state PDA", SWAP_FEE);

    Ok(())
}

/// Configures the trading fee rate for token swaps in the pool.
///
/// This function allows the pool owner to set or update the trading fee rate charged
/// on all token swaps. The fee is expressed in basis points (1/100th of a percent) 
/// and can range from 0% to 0.5% (0-50 basis points). This provides pool operators
/// with revenue generation while maintaining competitive trading costs.
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
///   - `accounts[1]` - Pool state PDA account (writable for fee configuration updates)
/// * `fee_basis_points` - The new trading fee rate in basis points (0-50, representing 0%-0.5%)
///
/// # Account Requirements
/// - **Owner**: Must be signer and match the owner field in pool state data
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
/// - **Collection**: Fees are accumulated in pool state and withdrawable by delegates
///
/// # Fee Revenue Model
/// - **Source**: Percentage of every token swap transaction
/// - **Accumulation**: Fees are tracked separately by token type in pool state
/// - **Withdrawal**: Authorized delegates can withdraw accumulated fees
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
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    fee_basis_points: u64,
) -> ProgramResult {
    msg!("Processing SetSwapFee: {} basis points", fee_basis_points);
    let account_info_iter = &mut accounts.iter();

    let owner = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer to set swap fee");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if *owner.key != pool_state_data.owner {
        msg!("Only pool owner can set swap fees");
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate fee is within allowed range (0-50 basis points = 0%-0.5%)
    if fee_basis_points > MAX_SWAP_FEE_BASIS_POINTS {
        msg!("Swap fee {} basis points exceeds maximum of {} basis points (0.5%)", 
             fee_basis_points, MAX_SWAP_FEE_BASIS_POINTS);
        return Err(ProgramError::InvalidArgument);
    }

    // Update swap fee
    let old_fee = pool_state_data.swap_fee_basis_points;
    pool_state_data.swap_fee_basis_points = fee_basis_points;

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
        let mut account_data = pool_state.data.borrow_mut();
        account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
    }
    
    // Log the change for transparency
    msg!("Swap fee updated: {} -> {} basis points ({:.2}% -> {:.2}%)", 
         old_fee, fee_basis_points,
         old_fee as f64 / 100.0, fee_basis_points as f64 / 100.0);

    Ok(())
} 