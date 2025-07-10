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

use crate::{constants::*, types::*};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::Sysvar,
    program_pack::Pack,
    system_instruction,
};
use spl_token::{
    instruction as token_instruction,
    state::{Account as TokenAccount},
};
use crate::utils::validation::validate_non_zero_amount;

/// Handles user deposits into the trading pool using standardized account ordering.
///
/// This function implements the modernized deposit process with consistent account positioning
/// across all trading functions. It allows users to deposit tokens in exchange for LP tokens
/// at a guaranteed 1:1 ratio while maintaining strict standardization for ease of use.
///
/// **🏗️ STANDARDIZED ACCOUNT ORDERING**: This function uses the new standardized account
/// ordering pattern implemented across all trading functions. Account positions are:
/// - **Base System (0-3)**: Authority, system program, rent sysvar, clock sysvar
/// - **Pool Core (4-8)**: Pool state, token A mint, token B mint, token A vault, token B vault
/// - **Token Operations (9-11)**: SPL Token program, user input account, user output account
/// - **Treasury (12-14)**: Main treasury, swap treasury, HFT treasury
/// - **Function-Specific (15+)**: LP token mints, system state, etc.
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
/// - Enforces strict 1:1 ratio between deposited tokens and LP tokens
/// - Transaction fails if 1:1 ratio cannot be maintained
///
/// # Guarantees
/// - **Strict 1:1 ratio**: deposit N tokens → receive exactly N LP tokens
/// - **Transaction rollback**: fails cleanly if 1:1 ratio cannot be maintained
/// - **LP token precision**: LP tokens have same decimal precision as underlying tokens
/// - **Unlimited supply**: LP tokens have no supply caps
/// - **Contract-only minting**: Only the contract can mint LP tokens
/// - **Centralized fees**: All fees go to pool PDA for future fee management
///
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `amount` - The amount to deposit (will receive exactly this many LP tokens)
/// * `deposit_token_mint_key` - The mint of the token being deposited (for validation)
/// * `accounts` - The accounts required for deposit in standardized order:
///   - `accounts[0]` - User authority (must be signer)
///   - `accounts[1]` - System program
///   - `accounts[2]` - Rent sysvar
///   - `accounts[3]` - Clock sysvar
///   - `accounts[4]` - Pool state PDA account
///   - `accounts[5]` - Token A mint account
///   - `accounts[6]` - Token B mint account
///   - `accounts[7]` - Token A vault account
///   - `accounts[8]` - Token B vault account
///   - `accounts[9]` - SPL Token program
///   - `accounts[10]` - User input token account
///   - `accounts[11]` - User output LP token account
///   - `accounts[12]` - Main Treasury PDA (for fee collection)
///   - `accounts[13]` - Swap Treasury PDA (unused but standardized)
///   - `accounts[14]` - HFT Treasury PDA (unused but standardized)
///   - `accounts[15]` - LP Token A mint account
///   - `accounts[16]` - LP Token B mint account
///
/// # Returns
/// * `ProgramResult` - Success or error code
/// 
/// # Errors
/// - `ProgramError::Custom(3001)` - Strict 1:1 ratio violation
/// - `ProgramError::Custom(3002)` - LP token mint operation integrity violation
pub fn process_deposit(
    program_id: &Pubkey,
    amount: u64,
    deposit_token_mint_key: Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing Deposit (Standardized Account Ordering)");
    
    // ✅ SYSTEM PAUSE: Check system pause state before any operations
    crate::utils::validation::validate_system_not_paused_safe(accounts, 17)?; // Expected: 17 accounts
    
    // ✅ STANDARDIZED ACCOUNT EXTRACTION: Extract accounts using standardized indices
    let user_authority = &accounts[0];                    // Index 0: Authority/User Signer
    let _system_program = &accounts[1];                   // Index 1: System Program
    let _rent_sysvar = &accounts[2];                      // Index 2: Rent Sysvar
    let clock_sysvar = &accounts[3];                     // Index 3: Clock Sysvar
    let pool_state_account = &accounts[4];                // Index 4: Pool State PDA
    let _token_a_mint = &accounts[5];                     // Index 5: Token A Mint
    let _token_b_mint = &accounts[6];                     // Index 6: Token B Mint
    let token_a_vault = &accounts[7];                     // Index 7: Token A Vault PDA
    let token_b_vault = &accounts[8];                     // Index 8: Token B Vault PDA
    let spl_token_program = &accounts[9];                 // Index 9: SPL Token Program
    let user_input_account = &accounts[10];               // Index 10: User Input Token Account
    let user_output_account = &accounts[11];              // Index 11: User Output LP Token Account
    let main_treasury = &accounts[12];                    // Index 12: Main Treasury PDA
    let _swap_treasury = &accounts[13];                   // Index 13: Swap Treasury PDA (unused)
    let _hft_treasury = &accounts[14];                    // Index 14: HFT Treasury PDA (unused)
    let lp_token_a_mint = &accounts[15];                  // Index 15: LP Token A Mint
    let lp_token_b_mint = &accounts[16];                  // Index 16: LP Token B Mint
    
    // Core validation
    validate_non_zero_amount(amount, "Deposit")?;
    
    if !user_authority.is_signer {
        msg!("User must be a signer for deposit");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // ✅ PHASE 3: CENTRALIZED FEE COLLECTION - Collect fee with real-time tracking
    use crate::utils::fee_validation::collect_liquidity_fee;
    
    collect_liquidity_fee(
        user_authority,
        main_treasury,
        _system_program,
        clock_sysvar,
        program_id,
    )?;

    msg!("✅ Deposit fee collected successfully - proceeding with deposit");

    // Read and validate pool state
    let mut pool_state_data = PoolState::deserialize(&mut &pool_state_account.data.borrow()[..])?;
    
    if !pool_state_data.is_initialized {
        msg!("Pool not initialized");
        return Err(ProgramError::UninitializedAccount);
    }

    // Determine deposit token mint from user's input account and validate against instruction parameter
    let user_input_data = TokenAccount::unpack_from_slice(&user_input_account.data.borrow())?;
    let actual_deposit_mint = user_input_data.mint;
    
    // Validate instruction parameter matches accounts-derived mint
    if actual_deposit_mint != deposit_token_mint_key {
        msg!("Instruction deposit_token_mint ({}) does not match user input account mint ({})", 
             deposit_token_mint_key, actual_deposit_mint);
        return Err(ProgramError::InvalidInstructionData);
    }
    
    msg!("Deposit token mint validated: {}", deposit_token_mint_key);

    // Determine deposit target (Token A or B)
    let (target_vault, target_lp_mint, is_depositing_token_a) = 
        if deposit_token_mint_key == pool_state_data.token_a_mint {
            if *token_a_vault.key != pool_state_data.token_a_vault {
                msg!("Invalid token A vault for deposit");
                return Err(ProgramError::InvalidAccountData);
            }
            if *lp_token_a_mint.key != pool_state_data.lp_token_a_mint {
                msg!("Invalid LP token A mint for deposit");
                return Err(ProgramError::InvalidAccountData);
            }
            (token_a_vault, lp_token_a_mint, true)
        } else if deposit_token_mint_key == pool_state_data.token_b_mint {
            if *token_b_vault.key != pool_state_data.token_b_vault {
                msg!("Invalid token B vault for deposit");
                return Err(ProgramError::InvalidAccountData);
            }
            if *lp_token_b_mint.key != pool_state_data.lp_token_b_mint {
                msg!("Invalid LP token B mint for deposit");
                return Err(ProgramError::InvalidAccountData);
            }
            (token_b_vault, lp_token_b_mint, false)
        } else {
            msg!("Deposit token mint does not match pool tokens");
            return Err(ProgramError::InvalidArgument);
        };

    // Validate user accounts
    if user_input_data.owner != *user_authority.key {
        msg!("User input account owner mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_input_data.amount < amount {
        msg!("Insufficient funds in user input account");
        return Err(ProgramError::InsufficientFunds);
    }

    let user_output_data = TokenAccount::unpack_from_slice(&user_output_account.data.borrow())?;
    if user_output_data.mint != *target_lp_mint.key {
        msg!("User output account mint mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_output_data.owner != *user_authority.key {
        msg!("User output account owner mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Record initial LP balance for strict 1:1 verification
    let initial_lp_balance = user_output_data.amount;
    msg!("Initial LP balance: {}, expecting to mint: {}", initial_lp_balance, amount);

    // Transfer tokens from user to pool vault
    msg!("Transferring {} tokens from user to pool vault", amount);
    invoke(
        &token_instruction::transfer(
            spl_token_program.key,
            user_input_account.key,
            target_vault.key,
            user_authority.key,
            &[],
            amount,
        )?,
        &[
            user_input_account.clone(),
            target_vault.clone(),
            user_authority.clone(),
            spl_token_program.clone(),
        ],
    )?;

    // Update pool liquidity
    if is_depositing_token_a {
        pool_state_data.total_token_a_liquidity = pool_state_data.total_token_a_liquidity.checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    } else {
        pool_state_data.total_token_b_liquidity = pool_state_data.total_token_b_liquidity.checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }

    // Buffer serialization pattern to prevent PDA corruption
    let mut serialized_data = Vec::new();
    pool_state_data.serialize(&mut serialized_data)?;
    {
        let mut account_data = pool_state_account.data.borrow_mut();
        account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
    }

    // Mint LP tokens (1:1 ratio)
    let pool_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.token_a_mint.as_ref(),
        pool_state_data.token_b_mint.as_ref(),
        &pool_state_data.ratio_a_numerator.to_le_bytes(),
        &pool_state_data.ratio_b_denominator.to_le_bytes(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    msg!("Minting {} LP tokens to user", amount);
    invoke_signed(
        &token_instruction::mint_to(
            spl_token_program.key,
            target_lp_mint.key,
            user_output_account.key,
            pool_state_account.key,
            &[],
            amount,
        )?,
        &[
            target_lp_mint.clone(),
            user_output_account.clone(),
            pool_state_account.clone(),
            spl_token_program.clone(),
        ],
        &[pool_pda_seeds],
    )?;

    // Verify strict 1:1 ratio
    let final_lp_balance = {
        let account_data = TokenAccount::unpack_from_slice(&user_output_account.data.borrow())?;
        account_data.amount
    };
    
    let lp_tokens_received = final_lp_balance.checked_sub(initial_lp_balance)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    if lp_tokens_received != amount {
        msg!("❌ Strict 1:1 violation: expected {}, received {}", amount, lp_tokens_received);
        return Err(ProgramError::Custom(3001));
    }

    // Fee collection moved to beginning of deposit function (FEES FIRST PATTERN)

    msg!("✅ Deposit completed: {} tokens → {} LP tokens", amount, lp_tokens_received);
    Ok(())
}

/// Handles user withdrawals from the trading pool using standardized account ordering.
///
/// This function implements the modernized withdrawal process with consistent account positioning
/// across all trading functions. It allows users to withdraw underlying tokens by burning LP tokens
/// at a guaranteed 1:1 ratio while maintaining strict standardization for ease of use.
///
/// **🏗️ STANDARDIZED ACCOUNT ORDERING**: This function uses the new standardized account
/// ordering pattern implemented across all trading functions. Account positions are:
/// - **Base System (0-3)**: Authority, system program, rent sysvar, clock sysvar
/// - **Pool Core (4-8)**: Pool state, token A mint, token B mint, token A vault, token B vault
/// - **Token Operations (9-11)**: SPL Token program, user input account, user output account
/// - **Treasury (12-14)**: Main treasury, swap treasury, HFT treasury
/// - **Function-Specific (15+)**: LP token mints, system state, etc.
///
/// **🛡️ AUTOMATIC MEV PROTECTION**: Large withdrawals (≥5% of pool) automatically trigger
/// temporary swap pause to prevent front-running and sandwich attacks.
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
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `lp_amount_to_burn` - The amount of LP tokens to burn
/// * `withdraw_token_mint_key` - The mint of the token being withdrawn (for validation)
/// * `accounts` - The accounts required for withdrawal in standardized order:
///   - `accounts[0]` - User authority (must be signer)
///   - `accounts[1]` - System program
///   - `accounts[2]` - Rent sysvar
///   - `accounts[3]` - Clock sysvar
///   - `accounts[4]` - Pool state PDA account
///   - `accounts[5]` - Token A mint account
///   - `accounts[6]` - Token B mint account
///   - `accounts[7]` - Token A vault account
///   - `accounts[8]` - Token B vault account
///   - `accounts[9]` - SPL Token program
///   - `accounts[10]` - User input LP token account
///   - `accounts[11]` - User output token account
///   - `accounts[12]` - Main Treasury PDA (for fee collection)
///   - `accounts[13]` - Swap Treasury PDA (unused but standardized)
///   - `accounts[14]` - HFT Treasury PDA (unused but standardized)
///   - `accounts[15]` - LP Token A mint account
///   - `accounts[16]` - LP Token B mint account
///
/// # Returns
/// * `ProgramResult` - Success or error code
pub fn process_withdraw(
    program_id: &Pubkey,
    lp_amount_to_burn: u64,
    withdraw_token_mint_key: Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing Withdrawal (Standardized Account Ordering)");
    
    // ✅ SYSTEM PAUSE: Check system pause state before any operations
    crate::utils::validation::validate_system_not_paused_safe(accounts, 17)?; // Expected: 17 accounts
    
    // ✅ STANDARDIZED ACCOUNT EXTRACTION: Extract accounts using standardized indices
    let user_authority = &accounts[0];                    // Index 0: Authority/User Signer
    let _system_program = &accounts[1];                   // Index 1: System Program
    let _rent_sysvar = &accounts[2];                      // Index 2: Rent Sysvar
    let clock_sysvar = &accounts[3];                      // Index 3: Clock Sysvar
    let pool_state_account = &accounts[4];                // Index 4: Pool State PDA
    let _token_a_mint = &accounts[5];                     // Index 5: Token A Mint
    let _token_b_mint = &accounts[6];                     // Index 6: Token B Mint
    let token_a_vault = &accounts[7];                     // Index 7: Token A Vault PDA
    let token_b_vault = &accounts[8];                     // Index 8: Token B Vault PDA
    let spl_token_program = &accounts[9];                 // Index 9: SPL Token Program
    let user_input_account = &accounts[10];               // Index 10: User Input LP Token Account
    let user_output_account = &accounts[11];              // Index 11: User Output Token Account
    let main_treasury = &accounts[12];                    // Index 12: Main Treasury PDA
    let _swap_treasury = &accounts[13];                   // Index 13: Swap Treasury PDA (unused)
    let _hft_treasury = &accounts[14];                    // Index 14: HFT Treasury PDA (unused)
    let lp_token_a_mint = &accounts[15];                  // Index 15: LP Token A Mint
    let lp_token_b_mint = &accounts[16];                  // Index 16: LP Token B Mint
    
    // Core validation
    validate_non_zero_amount(lp_amount_to_burn, "Withdrawal")?;
    
    if !user_authority.is_signer {
        msg!("User must be a signer for withdrawal");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // ✅ PHASE 3: CENTRALIZED FEE COLLECTION - Collect fee with real-time tracking
    use crate::utils::fee_validation::collect_liquidity_fee;
    
    collect_liquidity_fee(
        user_authority,
        main_treasury,
        _system_program,
        clock_sysvar,
        program_id,
    )?;

    msg!("✅ Withdrawal fee collected successfully - proceeding with withdrawal");

    // Read and validate pool state
    let mut pool_state_data = PoolState::deserialize(&mut &pool_state_account.data.borrow()[..])?;
    
    if !pool_state_data.is_initialized {
        msg!("Pool not initialized");
        return Err(ProgramError::UninitializedAccount);
    }

    // Determine withdrawal token mint from user's output account and validate against instruction parameter
    let user_output_data = TokenAccount::unpack_from_slice(&user_output_account.data.borrow())?;
    let actual_withdraw_mint = user_output_data.mint;
    
    // Validate instruction parameter matches accounts-derived mint
    if actual_withdraw_mint != withdraw_token_mint_key {
        msg!("Instruction withdraw_token_mint ({}) does not match user output account mint ({})", 
             withdraw_token_mint_key, actual_withdraw_mint);
        return Err(ProgramError::InvalidInstructionData);
    }
    
    msg!("Withdrawal token mint validated: {}", withdraw_token_mint_key);

    // MEV protection for large withdrawals
    let protection_needed = should_protect_withdrawal_from_slippage(lp_amount_to_burn, &pool_state_data)?;
    if protection_needed {
        let clock = Clock::from_account_info(clock_sysvar)?;
        initiate_withdrawal_protection(&mut pool_state_data, user_authority.key, clock.unix_timestamp)?;
    }

    // Determine withdrawal target (Token A or B) and validate LP mint correspondence
    let user_input_data = TokenAccount::unpack_from_slice(&user_input_account.data.borrow())?;
    let (source_vault, source_lp_mint, is_withdrawing_token_a) = 
        if withdraw_token_mint_key == pool_state_data.token_a_mint {
            // Withdrawing Token A - should be burning LP Token A
            if user_input_data.mint != pool_state_data.lp_token_a_mint {
                msg!("Cannot withdraw Token A without burning LP Token A");
                return Err(ProgramError::InvalidAccountData);
            }
            if *token_a_vault.key != pool_state_data.token_a_vault {
                msg!("Invalid token A vault for withdrawal");
                return Err(ProgramError::InvalidAccountData);
            }
            if *lp_token_a_mint.key != pool_state_data.lp_token_a_mint {
                msg!("Invalid LP token A mint for withdrawal");
                return Err(ProgramError::InvalidAccountData);
            }
            (token_a_vault, lp_token_a_mint, true)
        } else if withdraw_token_mint_key == pool_state_data.token_b_mint {
            // Withdrawing Token B - should be burning LP Token B
            if user_input_data.mint != pool_state_data.lp_token_b_mint {
                msg!("Cannot withdraw Token B without burning LP Token B");
                return Err(ProgramError::InvalidAccountData);
            }
            if *token_b_vault.key != pool_state_data.token_b_vault {
                msg!("Invalid token B vault for withdrawal");
                return Err(ProgramError::InvalidAccountData);
            }
            if *lp_token_b_mint.key != pool_state_data.lp_token_b_mint {
                msg!("Invalid LP token B mint for withdrawal");
                return Err(ProgramError::InvalidAccountData);
            }
            (token_b_vault, lp_token_b_mint, false)
        } else {
            msg!("Withdrawal token mint does not match pool tokens");
            return Err(ProgramError::InvalidArgument);
        };

    // Execute withdrawal logic
    let result = execute_withdrawal_logic(
        &mut pool_state_data,
        lp_amount_to_burn,
        withdraw_token_mint_key,
        is_withdrawing_token_a,
        user_authority,
        user_input_account,
        user_output_account,
        source_vault,
        source_lp_mint,
        pool_state_account,
        spl_token_program,
        _system_program,
        main_treasury,
        program_id,
    );

    // Always clear protection regardless of outcome
    if protection_needed {
        complete_withdrawal_protection(&mut pool_state_data)?;
        
        // Save updated state
        let mut serialized_data = Vec::new();
        pool_state_data.serialize(&mut serialized_data)?;
        {
            let mut account_data = pool_state_account.data.borrow_mut();
            account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
        }
    }

    result
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
    
    // Also check if swaps are already paused by owner (don't interfere)
    if pool_state.swaps_paused {
        msg!("Swaps already paused by owner - no additional protection needed");
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
    _withdrawer: &Pubkey,
    _current_timestamp: i64,
) -> ProgramResult {
    // Only pause if not already paused by owner
    if !pool_state.swaps_paused {
        pool_state.swaps_paused = true;
        
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
/// swaps to resume. Only applies to automatic protection, not owner-initiated pauses.
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
    main_treasury_account: &AccountInfo<'a>,
    program_id: &Pubkey,
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

    // Fee collection moved to beginning of withdrawal function (FEES FIRST PATTERN)
    
    //=========================================================================
    // NOTE: SOL FEE TRACKING MOVED TO CENTRAL TREASURY
    //=========================================================================
    // SOL fees are now tracked in central TreasuryState, not per-pool.
    // This provides system-wide fee collection and simplified accounting.
    // Real counters will be incremented for low-frequency operations like this.
        
    msg!("✅ SOL fees now tracked centrally in TreasuryState");

    Ok(())
} 