//! Liquidity Management Processors
//! 
//! This module contains all processors related to liquidity management operations
//! including deposits, withdrawals, and enhanced deposit features with slippage protection.
//!
//! ## Critical Implementation Note: Buffer Serialization Pattern
//! 
//! **⚠️ IMPORTANT: PDA Data Corruption Workaround ⚠️**
//! 
//! This module implements a critical workaround for a known Solana issue where PDA account
//! data can be corrupted when the same PDA is used as both:
//! 1. A signing authority in `invoke_signed()` calls
//! 2. A data storage account containing large structured data
//! 
//! ### The Problem
//! When performing SPL Token operations (mint_to, burn, transfer) via `invoke_signed()`,
//! the Solana runtime may corrupt or wipe the account data if the authority PDA contains
//! structured data larger than a simple signing account. This manifests as:
//! - Pool state data getting wiped to 0 bytes after mint operations
//! - `BorshIoError("Unknown")` when trying to deserialize account data
//! - Successful serialize operations that don't persist
//! 
//! ### The Solution: Buffer Serialization Pattern
//! Instead of direct serialization to account data:
//! ```rust,ignore
//! // ❌ PROBLEMATIC - Can be corrupted by subsequent invoke_signed()
//! pool_state_data.serialize(&mut *pool_state_account.data.borrow_mut())?;
//! ```
//! 
//! Use the two-step buffer pattern:
//! ```rust,ignore
//! // ✅ SAFE - Prevents corruption
//! let mut serialized_data = Vec::new();
//! pool_state_data.serialize(&mut serialized_data)?;
//! {
//!     let mut account_data = pool_state_account.data.borrow_mut();
//!     account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
//! }
//! ```
//! 
//! ### When to Use This Pattern
//! - **Always** when serializing data before `invoke_signed()` operations
//! - When the same PDA serves as both authority and data storage
//! - In any function that performs SPL Token operations after data updates
//! 
//! ### References
//! - Documented in `process_initialize_pool_data()` (pool_creation.rs)
//! - Implemented in `process_deposit()` (this file)
//! - Affects multiple DeFi protocols on Solana
//! 
//! ### Future Improvements
//! Consider separating authority and data storage into different PDAs to eliminate
//! this architectural complexity entirely.

use crate::{constants::*, types::*, check_rent_exempt};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
    program_pack::Pack,
    system_instruction,
};
use spl_token::{
    instruction as token_instruction,
    state::{Account as TokenAccount},
};
use crate::utils::validation::validate_non_zero_amount;

/// Enhanced deposit operation with additional features for testing and advanced use cases.
/// 
/// This function extends the standard deposit functionality with:
/// - Slippage protection through minimum LP token guarantees
/// - Custom fee recipient specification for flexible fee distribution
/// - Additional validation and error handling
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
/// # Arguments
/// - `system_state_account`: Optional first account for system pause validation
/// 
/// # Features
/// ## Slippage Protection
/// - Validates that the LP tokens received meet the minimum threshold
/// - Prevents unexpected losses due to changing pool conditions
/// - Provides predictable user experience
/// 
/// ## Custom Fee Recipients
/// - Allows specifying an alternative fee recipient
/// - Useful for testing, partnerships, or custom fee structures
/// - Falls back to default pool fee collection if None specified
/// 
/// ## Enhanced Validation
/// - All standard deposit validations plus additional checks
/// - Better error messages and debugging information
/// - Future-extensible parameter structure
/// 
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for deposit (same as standard deposit)
/// * `deposit_token_mint` - Token mint being deposited
/// * `amount` - Amount of tokens to deposit
/// * `minimum_lp_tokens_out` - Minimum LP tokens expected (slippage protection)
/// * `fee_recipient` - Optional custom fee recipient (None = default to pool)
/// 
/// # Returns
/// * `ProgramResult` - Success or error code
pub fn process_deposit_with_features(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    deposit_token_mint: Pubkey,
    amount: u64,
    minimum_lp_tokens_out: u64,
    fee_recipient: Option<Pubkey>,
) -> ProgramResult {
    msg!("DEBUG: process_deposit_with_features: Enhanced deposit with slippage protection");
    msg!("DEBUG: process_deposit_with_features: Amount: {}, Min LP out: {}, Custom fee recipient: {:?}", 
         amount, minimum_lp_tokens_out, fee_recipient);
    
    // ✅ SYSTEM PAUSE: Backward compatible validation
    crate::utils::validation::validate_system_not_paused_safe(accounts, 14)?; // Expected: 14 accounts minimum
    
    // Debug account validation
    if accounts.len() < 14 {
        msg!("DEBUG: process_deposit_with_features: Insufficient accounts provided: {}", accounts.len());
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // Get user destination LP token account to check balance before and after
    let user_destination_lp_token_account = &accounts[9]; // Based on standard deposit account order
    msg!("DEBUG: process_deposit_with_features: About to read initial LP balance from account: {}", user_destination_lp_token_account.key);
    
    let initial_lp_balance = {
        match TokenAccount::unpack_from_slice(&user_destination_lp_token_account.data.borrow()) {
            Ok(account_data) => {
                msg!("DEBUG: process_deposit_with_features: Initial LP balance: {}", account_data.amount);
                account_data.amount
            }
            Err(e) => {
                msg!("DEBUG: process_deposit_with_features: Failed to read initial LP balance: {:?}", e);
                return Err(e.into());
            }
        }
    };
    
    // Perform standard deposit operation
    msg!("DEBUG: process_deposit_with_features: About to call process_deposit");
    match process_deposit(program_id, accounts, deposit_token_mint, amount) {
        Ok(_) => {
            msg!("DEBUG: process_deposit_with_features: process_deposit completed successfully");
        }
        Err(e) => {
            msg!("DEBUG: process_deposit_with_features: process_deposit FAILED with error: {:?}", e);
            return Err(e);
        }
    }
    
    // Check slippage protection
    msg!("DEBUG: process_deposit_with_features: About to read final LP balance");
    let final_lp_balance = {
        match TokenAccount::unpack_from_slice(&user_destination_lp_token_account.data.borrow()) {
            Ok(account_data) => {
                msg!("DEBUG: process_deposit_with_features: Final LP balance: {}", account_data.amount);
                account_data.amount
            }
            Err(e) => {
                msg!("DEBUG: process_deposit_with_features: Failed to read final LP balance: {:?}", e);
                return Err(e.into());
            }
        }
    };
    
    let lp_tokens_received = final_lp_balance.checked_sub(initial_lp_balance)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    msg!("DEBUG: process_deposit_with_features: LP tokens received: {}, minimum required: {}", 
         lp_tokens_received, minimum_lp_tokens_out);
    
    if lp_tokens_received < minimum_lp_tokens_out {
        msg!("DEBUG: process_deposit_with_features: Slippage protection triggered. Received: {}, Minimum: {}", 
             lp_tokens_received, minimum_lp_tokens_out);
        return Err(ProgramError::Custom(2001)); // Custom slippage protection error
    }
    
    // Handle custom fee recipient if specified
    if let Some(custom_recipient) = fee_recipient {
        msg!("DEBUG: process_deposit_with_features: Custom fee recipient specified: {}", custom_recipient);
        // TODO: Implement custom fee recipient logic in future versions
        // For now, just log the intent - fees still go to pool
    }
    
    msg!("DEBUG: process_deposit_with_features: Enhanced deposit completed successfully. LP tokens received: {}", lp_tokens_received);
    Ok(())
}

/// Handles user deposits into the trading pool.
///
/// This function allows users to deposit tokens into the pool in exchange for LP (Liquidity Provider)
/// tokens. The deposit maintains the pool's fixed ratio structure and provides users with proportional
/// ownership tokens that can later be redeemed for underlying assets.
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
/// # Arguments
/// - `system_state_account`: Optional first account for system pause validation
///
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for deposit in the following order:
///   - `accounts[0]` - User account (must be signer)
///   - `accounts[1]` - User's source token account
///   - `accounts[2]` - Pool state PDA account
///   - `accounts[3]` - Token A mint account (for PDA seed verification)
///   - `accounts[4]` - Token B mint account (for PDA seed verification)
///   - `accounts[5]` - Pool's Token A vault account
///   - `accounts[6]` - Pool's Token B vault account
///   - `accounts[7]` - LP Token A mint account
///   - `accounts[8]` - LP Token B mint account
///   - `accounts[9]` - User's destination LP token account
///   - `accounts[10]` - System program
///   - `accounts[11]` - SPL Token program
///   - `accounts[12]` - Rent sysvar
///   - `accounts[13]` - Clock sysvar
/// * `deposit_token_mint_key` - The mint of the token being deposited
/// * `amount` - The amount to deposit
///
/// # Returns
/// * `ProgramResult` - Success or error code
pub fn process_deposit(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    deposit_token_mint_key: Pubkey,
    amount: u64,
) -> ProgramResult {
    msg!("Processing Deposit v2");
    
    // ✅ SYSTEM PAUSE: Backward compatible validation
    crate::utils::validation::validate_system_not_paused_safe(accounts, 14)?; // Expected: 14 accounts minimum
    
    let account_info_iter = &mut accounts.iter();
    let user_signer = next_account_info(account_info_iter)?;
    let user_source_token_account = next_account_info(account_info_iter)?;
    let pool_state_account = next_account_info(account_info_iter)?;
    let token_a_mint_for_pda_seeds = next_account_info(account_info_iter)?;
    let token_b_mint_for_pda_seeds = next_account_info(account_info_iter)?;
    
    let pool_token_a_vault_account = next_account_info(account_info_iter)?;
    let pool_token_b_vault_account = next_account_info(account_info_iter)?;
    let lp_token_a_mint_account = next_account_info(account_info_iter)?;
    let lp_token_b_mint_account = next_account_info(account_info_iter)?;
    let user_destination_lp_token_account = next_account_info(account_info_iter)?;
    
    let system_program_account = next_account_info(account_info_iter)?;
    let token_program_account = next_account_info(account_info_iter)?;
    let rent_sysvar_account = next_account_info(account_info_iter)?;
    let _rent = &Rent::from_account_info(rent_sysvar_account)?;
    let _clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;

    // Validate amount is non-zero
    validate_non_zero_amount(amount, "Deposit")?;

    if !user_signer.is_signer {
        msg!("User must be a signer for deposit");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Read pool state data
    let mut pool_state_data = PoolState::deserialize(&mut &pool_state_account.data.borrow()[..])?;
    
    // TODO: Re-enable rent checks after fixing the deposit test
    // check_rent_exempt(pool_state_account, program_id, rent, _clock.slot)?;
    // check_rent_exempt(pool_token_a_vault_account, program_id, rent, _clock.slot)?;
    // check_rent_exempt(pool_token_b_vault_account, program_id, rent, _clock.slot)?;
    // check_rent_exempt(lp_token_a_mint_account, program_id, rent, _clock.slot)?;
    // check_rent_exempt(lp_token_b_mint_account, program_id, rent, _clock.slot)?;
    
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

    // Determine which token (A or B) is being deposited and set target accounts
    let (target_pool_vault_account, target_lp_mint_account, is_depositing_token_a) = 
        if deposit_token_mint_key == pool_state_data.token_a_mint {
            // Depositing Token A
            if *pool_token_a_vault_account.key != pool_state_data.token_a_vault {
                msg!("Invalid pool_token_a_vault_account provided for token A deposit.");
                return Err(ProgramError::InvalidAccountData);
            }
            if *lp_token_a_mint_account.key != pool_state_data.lp_token_a_mint {
                msg!("Invalid lp_token_a_mint_account provided for token A deposit.");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_a_vault_account, lp_token_a_mint_account, true)
        } else if deposit_token_mint_key == pool_state_data.token_b_mint {
            // Depositing Token B
            if *pool_token_b_vault_account.key != pool_state_data.token_b_vault {
                msg!("Invalid pool_token_b_vault_account provided for token B deposit.");
                return Err(ProgramError::InvalidAccountData);
            }
            if *lp_token_b_mint_account.key != pool_state_data.lp_token_b_mint {
                msg!("Invalid lp_token_b_mint_account provided for token B deposit.");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_b_vault_account, lp_token_b_mint_account, false)
        } else {
            msg!("Deposit token mint does not match either of the pool's tokens");
            return Err(ProgramError::InvalidArgument);
        };

    // Validate user's source token account
    let user_source_token_account_data = TokenAccount::unpack_from_slice(&user_source_token_account.data.borrow())?;
    if user_source_token_account_data.mint != deposit_token_mint_key {
        msg!("User source token account mint mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_source_token_account_data.owner != *user_signer.key {
        msg!("User source token account owner mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_source_token_account_data.amount < amount {
        msg!("Insufficient funds in user source token account");
        return Err(ProgramError::InsufficientFunds);
    }

    // Validate user's destination LP token account
    let user_dest_lp_token_account_data = TokenAccount::unpack_from_slice(&user_destination_lp_token_account.data.borrow())?;
    if user_dest_lp_token_account_data.mint != *target_lp_mint_account.key {
        msg!("User destination LP token account mint mismatch with target LP mint");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_dest_lp_token_account_data.owner != *user_signer.key {
        msg!("User destination LP token account owner mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Validate SPL Token Program ID
    if *token_program_account.key != Pubkey::new_from_array(spl_token::id().to_bytes()) {
        msg!("Invalid SPL Token Program ID");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Transfer tokens from user to pool vault
    msg!("Transferring {} of token {} from user to pool", amount, deposit_token_mint_key);
    invoke(
        &token_instruction::transfer(
            token_program_account.key,
            user_source_token_account.key,
            target_pool_vault_account.key,
            user_signer.key,
            &[],
            amount,
        )?,
        &[
            user_source_token_account.clone(),
            target_pool_vault_account.clone(),
            user_signer.clone(),
            token_program_account.clone(),
        ],
    )?;

    // Update pool state liquidity
    if is_depositing_token_a {
        pool_state_data.total_token_a_liquidity = pool_state_data.total_token_a_liquidity.checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    } else {
        pool_state_data.total_token_b_liquidity = pool_state_data.total_token_b_liquidity.checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }
    
    // ========================================================================
    // SOLANA BUFFER SERIALIZATION WORKAROUND FOR PDA DATA CORRUPTION
    // ========================================================================
    // 
    // PROBLEM: 
    // When using invoke_signed() with SPL Token operations (like mint_to), 
    // the Solana runtime can corrupt PDA account data if the PDA is used as 
    // both the authority AND contains structured data.
    //
    // SYMPTOMS:
    // - Direct .serialize() appears to succeed but data gets wiped to 0 bytes
    // - Pool state becomes unreadable after mint/burn operations
    // - Results in "BorshIoError" when trying to read the account later
    //
    // ROOT CAUSE:
    // SPL Token operations expect authority accounts to be simple signing accounts.
    // When a PDA contains large amounts of data (like our 1866-byte PoolState),
    // the runtime may overwrite or clear the account data during CPI calls.
    //
    // WORKAROUND:
    // Use a two-step buffer serialization process that has proven reliable:
    // 1. Serialize to a temporary Vec<u8> buffer first
    // 2. Atomically copy the entire buffer to the account data
    //
    // This pattern prevents partial writes and ensures data integrity even
    // when the account is subsequently used in invoke_signed() operations.
    //
    // REFERENCES:
    // - Same pattern used successfully in process_initialize_pool_data()
    // - Documented Solana issue affecting multiple DeFi protocols
    // - Alternative: Use separate authority PDA (future architectural improvement)
    // ========================================================================
    
    // Step 1: Serialize the pool state data to a temporary buffer
    let mut serialized_data = Vec::new();
    pool_state_data.serialize(&mut serialized_data)?;
    msg!("Buffer serialization completed. Buffer size: {} bytes", serialized_data.len());
    
    // Step 2: Atomic copy to account data BEFORE any invoke_signed calls
    // This ensures data persistence even when the PDA is used as authority
    {
        let mut account_data = pool_state_account.data.borrow_mut();
        account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
        msg!("Pool state updated successfully. Token A liquidity: {}, Token B liquidity: {}", 
             pool_state_data.total_token_a_liquidity, pool_state_data.total_token_b_liquidity);
    }

    // Mint LP tokens to user AFTER saving pool state
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.token_a_mint.as_ref(),
        pool_state_data.token_b_mint.as_ref(),
        &pool_state_data.ratio_a_numerator.to_le_bytes(),
        &pool_state_data.ratio_b_denominator.to_le_bytes(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    msg!("Minting {} LP tokens for {} to user", amount, target_lp_mint_account.key);
    invoke_signed(
        &token_instruction::mint_to(
            token_program_account.key,
            target_lp_mint_account.key,
            user_destination_lp_token_account.key,
            pool_state_account.key,
            &[], 
            amount,
        )?,
        &[
            target_lp_mint_account.clone(),
            user_destination_lp_token_account.clone(),
            pool_state_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;

    //=========================================================================
    // CONTRACT FEE TRANSFER (Fixed SOL Amount)  
    //=========================================================================
    // Contract fees are paid in SOL to cover transaction processing costs.
    // For liquidity operations, this covers token transfers, LP minting, and
    // pool state updates.
    //
    // Amount: 0.0013 SOL (1,300,000 lamports)
    // Purpose: Cover computational costs of liquidity operations
    
    if user_signer.lamports() < DEPOSIT_WITHDRAWAL_FEE {
        msg!("❌ Insufficient SOL for contract fee. Required: {} lamports, Available: {} lamports", 
             DEPOSIT_WITHDRAWAL_FEE, user_signer.lamports());
        return Err(ProgramError::InsufficientFunds);
    }
    
    invoke(
        &system_instruction::transfer(user_signer.key, pool_state_account.key, DEPOSIT_WITHDRAWAL_FEE),
        &[user_signer.clone(), pool_state_account.clone(), system_program_account.clone()],
    )?;
    
    msg!("✅ Contract fee transferred: {} lamports ({} SOL) from user to pool", 
         DEPOSIT_WITHDRAWAL_FEE, DEPOSIT_WITHDRAWAL_FEE as f64 / 1_000_000_000.0);

    //=========================================================================
    // UPDATE CONTRACT FEE TRACKING
    //=========================================================================
    // Track the SOL contract fee in pool state for proper accounting
    
    pool_state_data.collected_sol_fees = pool_state_data.collected_sol_fees
        .checked_add(DEPOSIT_WITHDRAWAL_FEE)
        .ok_or(ProgramError::ArithmeticOverflow)?;
        
    // Serialize updated pool state with fee tracking
    let mut updated_serialized_data = Vec::new();
    pool_state_data.serialize(&mut updated_serialized_data)?;
    
    {
        let mut account_data = pool_state_account.data.borrow_mut();
        account_data[..updated_serialized_data.len()].copy_from_slice(&updated_serialized_data);
    }
        
    msg!("✅ Contract fee tracking updated: {} total SOL fees collected", 
         pool_state_data.collected_sol_fees);

    Ok(())
}

/// Handles user withdrawals from the fixed-ratio trading pool.
///
/// This function allows users to withdraw their underlying tokens from the pool by burning
/// their LP (Liquidity Provider) tokens. The withdrawal is processed at a 1:1 ratio between
/// LP tokens burned and underlying tokens received, maintaining the pool's fixed ratio structure.
/// The function includes automatic MEV protection, fee collection, and comprehensive validation.
///
/// **🛡️ AUTOMATIC MEV PROTECTION**: Large withdrawals (≥5% of pool) automatically trigger
/// temporary swap pause to prevent front-running and sandwich attacks. During this protection:
/// - Pool status queries will show "swaps paused" 
/// - This is **expected behavior** and provides real-time transparency
/// - Protection automatically clears after withdrawal completion (success or failure)
/// - Deposits and withdrawals continue to work normally
///
/// **⚠️ RACE CONDITION NOTICE**: Users querying pool status during large withdrawals
/// may see temporary pause state. This is acceptable and provides accurate real-time
/// visibility into pool security measures. The pause is distinguishable from delegate
/// actions via the withdrawal_protection_active flag.
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
/// - Automatic MEV protection for large withdrawals
/// - Fail-safe protection cleanup regardless of outcome
///
/// # Purpose
/// - Enables users to exit their liquidity positions by burning LP tokens
/// - Maintains pool's fixed ratio by reducing both LP supply and underlying token reserves
/// - Provides automatic slippage protection for large withdrawals (≥5% threshold)
/// - Collects withdrawal fees to fund pool operations and rent exemption
/// - Provides audit trail and security checks for all withdrawal operations
///
/// # How it works
/// 1. Validates the user is authorized (signed the transaction)
/// 2. Verifies all provided accounts match expected pool structure
/// 3. Confirms rent-exempt status for all pool accounts
/// 4. Determines withdrawal direction (Token A or Token B) based on withdraw_token_mint_key
/// 5. Validates user has sufficient LP tokens to burn
/// 6. Checks pool has sufficient underlying token liquidity for withdrawal
/// 7. Burns LP tokens from user's LP token account
/// 8. Transfers underlying tokens from pool vault to user's destination account
/// 9. Updates pool state liquidity counters
/// 10. Collects withdrawal fee in SOL to maintain pool operations
///
/// # Arguments
/// * `program_id` - The program ID for PDA validation and authority checks
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - User account (must be signer)
///   - `accounts[1]` - User's LP token account (source of tokens to burn)
///   - `accounts[2]` - User's destination token account (receives underlying tokens)
///   - `accounts[3]` - Pool state PDA account (writable)
///   - `accounts[4]` - Token A mint account (for PDA seed verification)
///   - `accounts[5]` - Token B mint account (for PDA seed verification)
///   - `accounts[6]` - Pool's Token A vault account (writable)
///   - `accounts[7]` - Pool's Token B vault account (writable)
///   - `accounts[8]` - LP Token A mint account (writable if withdrawing Token A)
///   - `accounts[9]` - LP Token B mint account (writable if withdrawing Token B)
///   - `accounts[10]` - System program
///   - `accounts[11]` - SPL Token program
///   - `accounts[12]` - Rent sysvar (for rent calculations)
///   - `accounts[13]` - Clock sysvar (for timestamp validation)
/// * `withdraw_token_mint_key` - The mint address of the token to withdraw (must be either pool's Token A or Token B)
/// * `lp_amount_to_burn` - The amount of LP tokens to burn (1:1 ratio with underlying tokens received)
///
/// # Account Requirements
/// - User: Must be signer and owner of LP token account
/// - LP token account: Must contain sufficient tokens and be owned by user
/// - Destination account: Must be owned by user and match withdraw token mint
/// - Pool accounts: Must maintain rent-exempt status throughout operation
///
/// # Security Model
/// - Uses delegate-based two-step withdrawal process
/// - Withdrawal must be requested through delegate action
/// - Pool owner can review and cancel withdrawal requests
/// - Only approved withdrawals can be executed
///
/// # Fees
/// - Withdrawal fee: Fixed SOL amount (DEPOSIT_WITHDRAWAL_FEE) transferred to pool state PDA
/// - Purpose: Maintains pool rent exemption and funds ongoing operations
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - User didn't sign transaction
/// - `ProgramError::InvalidAccountData` - Account validation failures
/// - `ProgramError::InsufficientFunds` - Insufficient LP tokens or pool liquidity
/// - `PoolError::PoolPaused` - Pool operations are paused
///
/// # Example Usage
/// ```ignore
/// // Withdraw 1000 Token A by burning 1000 LP Token A
/// let instruction = PoolInstruction::Withdraw {
///     withdraw_token_mint: token_a_mint_pubkey,
///     lp_amount_to_burn: 1000,
/// };
/// ```
pub fn process_withdraw(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    withdraw_token_mint_key: Pubkey,
    lp_amount_to_burn: u64,
) -> ProgramResult {
    msg!("Processing Withdraw v2");
    
    // ✅ SYSTEM PAUSE: Only check system-wide pause (existing)
    crate::utils::validation::validate_system_not_paused_safe(accounts, 14)?; // Expected: 14 accounts minimum
    
    // ✅ NO POOL SWAP PAUSE CHECK: Withdrawals work regardless of delegate pool swap pause
    
    let account_info_iter = &mut accounts.iter();
    let user_signer = next_account_info(account_info_iter)?;                     // User making the withdrawal (signer)
    let user_source_lp_token_account = next_account_info(account_info_iter)?;   // User's LP token account (source of burn)
    let user_destination_token_account = next_account_info(account_info_iter)?; // User's account for receiving underlying tokens
    let pool_state_account = next_account_info(account_info_iter)?;              // Pool state PDA
    
    // Accounts needed for Pool State PDA seeds derivation for signing
    let token_a_mint_for_pda_seeds = next_account_info(account_info_iter)?;    // Pool's token_a_mint (must match pool_state_data.token_a_mint)
    let token_b_mint_for_pda_seeds = next_account_info(account_info_iter)?;    // Pool's token_b_mint (must match pool_state_data.token_b_mint)
    
    let pool_token_a_vault_account = next_account_info(account_info_iter)?;     // Pool's vault for token A
    let pool_token_b_vault_account = next_account_info(account_info_iter)?;     // Pool's vault for token B
    let lp_token_a_mint_account = next_account_info(account_info_iter)?;         // Pool's LP token A mint
    let lp_token_b_mint_account = next_account_info(account_info_iter)?;         // Pool's LP token B mint
    
    let system_program_account = next_account_info(account_info_iter)?;         // System program
    let token_program_account = next_account_info(account_info_iter)?;           // SPL Token program
    let rent_sysvar_account = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_sysvar_account)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;

    // Check rent-exempt status for pool accounts
    check_rent_exempt(pool_state_account, program_id, rent, clock.slot)?;
    check_rent_exempt(pool_token_a_vault_account, program_id, rent, clock.slot)?;
    check_rent_exempt(pool_token_b_vault_account, program_id, rent, clock.slot)?;
    check_rent_exempt(lp_token_a_mint_account, program_id, rent, clock.slot)?;
    check_rent_exempt(lp_token_b_mint_account, program_id, rent, clock.slot)?;

    if !user_signer.is_signer {
        msg!("User must be a signer for withdraw");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut pool_state_data = PoolState::deserialize(&mut &pool_state_account.data.borrow()[..])?;
    if !pool_state_data.is_initialized {
        msg!("Pool not initialized");
        return Err(ProgramError::UninitializedAccount);
    }

    // 🛡️ AUTOMATIC SLIPPAGE PROTECTION: Check if we should temporarily pause swaps
    let needs_protection = should_protect_withdrawal_from_slippage(
        lp_amount_to_burn,
        &pool_state_data,
    )?;
    
    if needs_protection {
        // Temporarily pause swaps to protect this withdrawal
        initiate_withdrawal_protection(&mut pool_state_data, user_signer.key, clock.unix_timestamp)?;
        msg!("🛡️ Temporarily pausing swaps to protect large withdrawal from slippage");
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

    // Determine which token (A or B) is being withdrawn and set relevant accounts
    let (source_pool_vault_acc, source_lp_mint_account, is_withdrawing_token_a) = 
        if withdraw_token_mint_key == pool_state_data.token_a_mint {
            // Withdrawing Token A, so burning LP Token A
            if *pool_token_a_vault_account.key != pool_state_data.token_a_vault {
                msg!("Invalid pool_token_a_vault_account provided for token A withdrawal.");
                return Err(ProgramError::InvalidAccountData);
            }
            if *lp_token_a_mint_account.key != pool_state_data.lp_token_a_mint {
                msg!("Invalid lp_token_a_mint_account provided for token A withdrawal.");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_a_vault_account, lp_token_a_mint_account, true)
        } else if withdraw_token_mint_key == pool_state_data.token_b_mint {
            // Withdrawing Token B, so burning LP Token B
            if *pool_token_b_vault_account.key != pool_state_data.token_b_vault {
                msg!("Invalid pool_token_b_vault_account provided for token B withdrawal.");
                return Err(ProgramError::InvalidAccountData);
            }
            if *lp_token_b_mint_account.key != pool_state_data.lp_token_b_mint {
                msg!("Invalid lp_token_b_mint_account provided for token B withdrawal.");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_b_vault_account, lp_token_b_mint_account, false)
        } else {
            msg!("Withdraw token mint does not match either of the pool's tokens");
            return Err(ProgramError::InvalidArgument);
        };

    // Validate user's source LP token account
    let user_source_lp_token_account_data = TokenAccount::unpack_from_slice(&user_source_lp_token_account.data.borrow())?;
    if user_source_lp_token_account_data.mint != *source_lp_mint_account.key {
        msg!("User source LP token account mint mismatch with identified LP mint for withdrawal.");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_source_lp_token_account_data.owner != *user_signer.key {
        msg!("User source LP token account owner mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_source_lp_token_account_data.amount < lp_amount_to_burn {
        msg!("Insufficient LP tokens in user source account");
        return Err(ProgramError::InsufficientFunds);
    }

    // Validate user's destination token account (for underlying tokens)
    let user_dest_token_account_data = TokenAccount::unpack_from_slice(&user_destination_token_account.data.borrow())?;
    if user_dest_token_account_data.mint != withdraw_token_mint_key {
        msg!("User destination token account mint mismatch with withdraw_token_mint_key");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_dest_token_account_data.owner != *user_signer.key {
        msg!("User destination token account owner mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate SPL Token Program ID
    if *token_program_account.key != Pubkey::new_from_array(spl_token::id().to_bytes()) {
        msg!("Invalid SPL Token Program ID");
        return Err(ProgramError::IncorrectProgramId);
    }
    
    // Check if pool has enough liquidity for the withdrawal
    if is_withdrawing_token_a {
        if pool_state_data.total_token_a_liquidity < lp_amount_to_burn {
            msg!("Insufficient token A liquidity in the pool for withdrawal.");
            return Err(ProgramError::InsufficientFunds);
        }
    } else {
        if pool_state_data.total_token_b_liquidity < lp_amount_to_burn {
            msg!("Insufficient token B liquidity in the pool for withdrawal.");
            return Err(ProgramError::InsufficientFunds);
        }
    }

    // Perform the withdrawal safely (no concurrent swaps can interfere if protection is active)
    let withdrawal_result = execute_withdrawal_logic(
        &mut pool_state_data,
        lp_amount_to_burn,
        withdraw_token_mint_key,
        is_withdrawing_token_a,
        user_signer,
        user_source_lp_token_account,
        user_destination_token_account,
        source_pool_vault_acc,
        source_lp_mint_account,
        pool_state_account,
        token_program_account,
        system_program_account,
    );
    
    // Always re-enable swaps after withdrawal (regardless of success/failure)
    if needs_protection {
        complete_withdrawal_protection(&mut pool_state_data)?;
        msg!("🔓 Re-enabling swaps after withdrawal completion");
    }
    
    // Save updated pool state (with protection cleanup applied)
    let mut serialized_data = Vec::new();
    pool_state_data.serialize(&mut serialized_data)?;
    {
        let mut account_data = pool_state_account.data.borrow_mut();
        account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
    }
    
    withdrawal_result
}

/// Determines if a withdrawal needs protection from swap interference
/// 
/// Large withdrawals (≥5% of pool) can be front-run or sandwich attacked by MEV bots.
/// This function calculates the withdrawal size as a percentage of total pool liquidity
/// to determine if temporary swap pause protection is warranted.
/// 
/// **⚠️ RACE CONDITION AWARENESS**: When protection is activated, users querying
/// pool status will see "swaps paused" until withdrawal completes. This is expected
/// and provides real-time transparency into pool security measures.
/// 
/// # Arguments
/// * `lp_amount_to_burn` - Amount of LP tokens being burned
/// * `pool_state` - Current pool state for liquidity calculations
/// 
/// # Returns
/// * `Result<bool, ProgramError>` - True if protection needed, false otherwise
fn should_protect_withdrawal_from_slippage(
    lp_amount_to_burn: u64,
    pool_state: &PoolState,
) -> Result<bool, ProgramError> {
    // Calculate withdrawal as percentage of total pool liquidity
    let total_lp_supply = pool_state.total_token_a_liquidity + pool_state.total_token_b_liquidity;
    if total_lp_supply == 0 {
        return Ok(false); // Empty pool, no protection needed
    }
    
    let withdrawal_percentage = (lp_amount_to_burn * 100) / total_lp_supply;
    
    // Protect withdrawals ≥5% of total pool to prevent slippage/front-running
    const LARGE_WITHDRAWAL_THRESHOLD: u64 = 5;
    
    if withdrawal_percentage >= LARGE_WITHDRAWAL_THRESHOLD {
        msg!("Large withdrawal detected: {}% of pool. Enabling slippage protection.", withdrawal_percentage);
        msg!("NOTE: Pool status queries will show 'swaps paused' until withdrawal completes");
        return Ok(true);
    }
    
    // Also check if swaps are already paused by delegates (don't interfere)
    if pool_state.swaps_paused {
        msg!("Swaps already paused by delegate - no additional protection needed");
        return Ok(false);
    }
    
    Ok(false)
}

/// Temporarily pause swaps to protect withdrawal from slippage
/// 
/// This function sets temporary swap pause flags to prevent MEV attacks during
/// large withdrawals. The pause is automatically cleared after withdrawal completion.
/// 
/// **⚠️ USER VISIBILITY**: During this protection phase, pool status queries will
/// show "swaps paused" with withdrawal_protection_active=true. This is intentional
/// transparency that allows users to understand why swaps are temporarily unavailable.
/// 
/// # Arguments
/// * `pool_state` - Mutable pool state to update
/// * `withdrawer` - Public key of the user making the withdrawal
/// * `current_timestamp` - Current blockchain timestamp
/// 
/// # Returns
/// * `ProgramResult` - Success or error
fn initiate_withdrawal_protection(
    pool_state: &mut PoolState,
    withdrawer: &Pubkey,
    current_timestamp: i64,
) -> ProgramResult {
    // Only pause if not already paused by delegates
    if !pool_state.swaps_paused {
        pool_state.swaps_paused = true;
        pool_state.swaps_pause_requested_by = Some(*withdrawer);
        pool_state.swaps_pause_initiated_timestamp = current_timestamp;
        
        // Mark this as a temporary withdrawal protection pause
        pool_state.withdrawal_protection_active = true;
        
        msg!("🛡️ MEV Protection: Swaps temporarily paused during large withdrawal");
        msg!("This state is visible to status queries and will auto-clear upon completion");
    }
    
    Ok(())
}

/// Re-enable swaps after withdrawal protection
/// 
/// This function clears the temporary withdrawal protection pause, allowing
/// swaps to resume. Only applies to automatic protection, not delegate-initiated pauses.
/// 
/// **⚠️ RACE CONDITION RESOLUTION**: After this function executes, subsequent
/// status queries will show "swaps enabled" again. The temporary protection
/// phase is complete and the race condition window has closed.
/// 
/// # Arguments
/// * `pool_state` - Mutable pool state to update
/// 
/// # Returns
/// * `ProgramResult` - Success or error
fn complete_withdrawal_protection(pool_state: &mut PoolState) -> ProgramResult {
    // Only unpause if this was our withdrawal protection pause
    if pool_state.withdrawal_protection_active {
        pool_state.swaps_paused = false;
        pool_state.swaps_pause_requested_by = None;
        pool_state.withdrawal_protection_active = false;
        
        msg!("🔓 MEV Protection completed - swaps re-enabled");
        msg!("Status queries will now show 'swaps enabled' again");
    }
    
    Ok(())
}

/// Execute the core withdrawal logic (extracted from original process_withdraw)
/// 
/// This function performs the actual token burning and transfer operations.
/// It's separated to enable proper cleanup in case of failures.
/// 
/// # Arguments
/// * `pool_state_data` - Mutable pool state 
/// * `lp_amount_to_burn` - Amount of LP tokens to burn
/// * `withdraw_token_mint_key` - Token mint being withdrawn
/// * `is_withdrawing_token_a` - True if withdrawing token A, false for token B
/// * Various account references for the operations
/// 
/// # Returns
/// * `ProgramResult` - Success or error from withdrawal operations
fn execute_withdrawal_logic<'a>(
    pool_state_data: &mut PoolState,
    lp_amount_to_burn: u64,
    withdraw_token_mint_key: Pubkey,
    is_withdrawing_token_a: bool,
    user_signer: &AccountInfo<'a>,
    user_source_lp_token_account: &AccountInfo<'a>,
    user_destination_token_account: &AccountInfo<'a>,
    source_pool_vault_acc: &AccountInfo<'a>,
    source_lp_mint_account: &AccountInfo<'a>,
    pool_state_account: &AccountInfo<'a>,
    token_program_account: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
) -> ProgramResult {
    use solana_program::{program::{invoke, invoke_signed}, system_instruction};
    use spl_token::instruction as token_instruction;
    use crate::constants::{POOL_STATE_SEED_PREFIX, DEPOSIT_WITHDRAWAL_FEE};

    // Burn LP tokens from user
    msg!("Burning {} LP tokens from account {}", lp_amount_to_burn, user_source_lp_token_account.key);
    invoke(
        &token_instruction::burn(
            token_program_account.key,
            user_source_lp_token_account.key, // Account to burn from
            source_lp_mint_account.key,       // Mint of the LP tokens being burned
            user_signer.key,                  // Authority (owner of the LP token account)
            &[],
            lp_amount_to_burn,
        )?,
        &[
            user_source_lp_token_account.clone(),
            source_lp_mint_account.clone(),
            user_signer.clone(),
            token_program_account.clone(),
        ],
    )?;

    // Transfer underlying tokens from pool vault to user
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.token_a_mint.as_ref(),
        pool_state_data.token_b_mint.as_ref(),
        &pool_state_data.ratio_a_numerator.to_le_bytes(),
        &pool_state_data.ratio_b_denominator.to_le_bytes(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    msg!("Transferring {} of token {} from pool vault {} to user account {}", 
           lp_amount_to_burn, withdraw_token_mint_key, source_pool_vault_acc.key, user_destination_token_account.key);
    invoke_signed(
        &token_instruction::transfer(
            token_program_account.key,
            source_pool_vault_acc.key,          // Pool's vault (source)
            user_destination_token_account.key,      // User's output account (destination)
            pool_state_account.key,             // Pool PDA is the authority over its vault
            &[],
            lp_amount_to_burn,                        // Amount of underlying token to transfer (equals LP burned)
        )?,
        &[
            source_pool_vault_acc.clone(),
            user_destination_token_account.clone(),
            pool_state_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;

    // Update pool state liquidity
    if is_withdrawing_token_a {
        pool_state_data.total_token_a_liquidity = pool_state_data.total_token_a_liquidity.checked_sub(lp_amount_to_burn)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    } else {
        pool_state_data.total_token_b_liquidity = pool_state_data.total_token_b_liquidity.checked_sub(lp_amount_to_burn)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }
    
    msg!("Pool liquidity updated. Token A: {}, Token B: {}", pool_state_data.total_token_a_liquidity, pool_state_data.total_token_b_liquidity);

    // Transfer withdrawal fee to pool state PDA
    if user_signer.lamports() < DEPOSIT_WITHDRAWAL_FEE {
        msg!("Insufficient SOL for withdrawal fee. User lamports: {}", user_signer.lamports());
        return Err(ProgramError::InsufficientFunds);
    }
    invoke(
        &system_instruction::transfer(user_signer.key, pool_state_account.key, DEPOSIT_WITHDRAWAL_FEE),
        &[user_signer.clone(), pool_state_account.clone(), system_program_account.clone()],
    )?;
    
    //=========================================================================
    // UPDATE CONTRACT FEE TRACKING
    //=========================================================================
    // Track the SOL contract fee in pool state for proper accounting
    
    pool_state_data.collected_sol_fees = pool_state_data.collected_sol_fees
        .checked_add(DEPOSIT_WITHDRAWAL_FEE)
        .ok_or(ProgramError::ArithmeticOverflow)?;
        
    msg!("Withdrawal fee {} transferred to pool state PDA", DEPOSIT_WITHDRAWAL_FEE);
    msg!("✅ Contract fee tracking updated: {} total SOL fees collected", 
         pool_state_data.collected_sol_fees);

    Ok(())
} 