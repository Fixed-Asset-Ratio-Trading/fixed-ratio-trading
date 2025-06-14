#![allow(deprecated)]
/*
MIT License

Copyright (c) 2024 Davinci

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

// This is the main library for the fixed-ratio-trading program
// It contains the program's instructions, error handling, and other functionality
// It also contains the program's constants and PDA seeds
// It is used by the program's entrypoint and other modules


use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
    clock::Clock,
    declare_id,
};
// SPL Token imports removed as they were unused

declare_id!("quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD");

// Client SDK module for simplified interaction with the pool program
pub mod client_sdk;
pub mod constants;
pub mod types;
pub mod processors;
pub mod utils;

// Import and re-export constants from the constants module
pub use constants::*;
// Import and re-export types from the types module
pub use types::*;
// Import and re-export processors from the processors module
pub use processors::*;
// Import and re-export utilities from the utils module
pub use utils::*;











/// Main entry point for the fixed-ratio trading pool Solana program.
///
/// This function serves as the central dispatcher for all pool operations, routing incoming
/// instructions to their appropriate handler functions. It implements global security checks,
/// instruction deserialization, pause state validation, and comprehensive error handling.
/// Every interaction with the pool program flows through this entry point.
///
/// # Purpose
/// - Central instruction routing and dispatch for all pool operations
/// - Global security enforcement including pause state validation
/// - Instruction deserialization with comprehensive error handling
/// - Audit logging for all program interactions
/// - Standardized error handling and program result management
///
/// # How it works
/// 1. **Instruction Deserialization**: Converts raw instruction data into typed `PoolInstruction` enum
/// 2. **Global Pause Check**: Validates pool pause state for user operations (skips owner/management functions)
/// 3. **Instruction Dispatch**: Routes each instruction type to its specific handler function:
///    - `CreatePoolStateAccount` → `process_create_pool_state_account`
///    - `InitializePoolData` → `process_initialize_pool_data`
///    - `Deposit` → `process_deposit`
///    - `Withdraw` → `process_withdraw`
///    - `Swap` → `process_swap`
///    - `WithdrawFees` → `process_withdraw_fees`
///    - `UpdateSecurityParams` → `process_update_security_params`
///    - `AddDelegate` → `process_add_delegate`
///    - `RemoveDelegate` → `process_remove_delegate`
///    - `WithdrawFeesToDelegate` → `process_withdraw_fees_to_delegate`
///    - `SetSwapFee` → `process_set_swap_fee`
///    - `GetWithdrawalHistory` → `process_get_withdrawal_history`
///    - `RequestFeeWithdrawal` → `process_request_fee_withdrawal`
///    - `CancelWithdrawalRequest` → `process_cancel_withdrawal_request`
///    - `SetDelegateWaitTime` → `process_set_delegate_wait_time`
/// 4. **Error Propagation**: Handles and propagates errors from handler functions
/// 5. **Logging**: Provides comprehensive debug logging for troubleshooting
///
/// # Arguments
/// * `program_id` - The program ID for PDA validation and program identification
/// * `accounts` - Array of accounts provided by the client for the specific operation
/// * `instruction_data` - Serialized instruction data containing the operation type and parameters
///
/// # Global Security Features
/// ## Pause State Enforcement
/// - **Protected Operations**: All user operations (deposit, withdraw, swap) are blocked when paused
/// - **Allowed Operations**: Owner and management functions remain accessible during pause:
///   - `WithdrawFees`, `UpdateSecurityParams`, `CreatePoolStateAccount`, `InitializePoolData`
/// - **Emergency Control**: Enables immediate halt of trading during security incidents
///
/// ## Instruction Validation
/// - **Type Safety**: All instructions must deserialize to valid `PoolInstruction` types
/// - **Parameter Validation**: Each handler performs specific parameter validation
/// - **Account Verification**: Comprehensive account ownership and structure validation
///
/// # Error Handling
/// The function handles several categories of errors:
/// - **Deserialization Errors**: Invalid or corrupted instruction data
/// - **Pause State Violations**: User operations attempted while pool is paused
/// - **Handler Function Errors**: Specific errors from individual operation handlers
///
/// # Supported Instructions
/// ## Pool Management
/// - `CreatePoolStateAccount`: Initial pool creation (Step 1)
/// - `InitializePoolData`: Pool data initialization (Step 2)
/// - `UpdateSecurityParams`: Security parameter updates
///
/// ## User Operations
/// - `Deposit`: Add liquidity to receive LP tokens
/// - `Withdraw`: Remove liquidity by burning LP tokens  
/// - `Swap`: Exchange tokens at fixed ratio
///
/// ## Fee Management
/// - `WithdrawFees`: Owner withdraws accumulated SOL fees
/// - `SetSwapFee`: Configure trading fee rates (0-0.5%)
///
/// ## Delegate System
/// - `AddDelegate`: Add authorized fee withdrawal delegates
/// - `RemoveDelegate`: Remove delegates
/// - `WithdrawFeesToDelegate`: Execute delegate fee withdrawals
/// - `RequestFeeWithdrawal`: Request time-delayed fee withdrawal
/// - `CancelWithdrawalRequest`: Cancel pending withdrawal requests
/// - `SetDelegateWaitTime`: Configure delegate-specific wait times
///
/// ## Transparency & Auditing
/// - `GetWithdrawalHistory`: Retrieve withdrawal audit trail
///
/// # Example Usage
/// ```ignore
/// // Called by Solana runtime for each transaction
/// let result = process_instruction(
///     &program_id,
///     &instruction_accounts,
///     &serialized_instruction_data,
/// );
/// ```
///
/// # Error Types
/// - Instruction deserialization failures → `ProgramError::InvalidInstructionData`
/// - Pause state violations → `PoolError::PoolPaused`
/// - Handler-specific errors → Various `ProgramError` and `PoolError` types
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("DEBUG: process_instruction: Entered. Program ID: {}, Instruction data len: {}", program_id, instruction_data.len());
    let instruction = match PoolInstruction::try_from_slice(instruction_data) {
        Ok(instr) => {
            msg!("DEBUG: process_instruction: Successfully deserialized instruction.");
            instr
        }
        Err(e) => {
            msg!("DEBUG: process_instruction: Failed to deserialize instruction_data: {:?}", e);
            return Err(e.into());
        }
    };
    
    // Check if pool is paused for all instructions except WithdrawFees, UpdateSecurityParams, and pool initialization instructions
    if let PoolInstruction::WithdrawFees 
        | PoolInstruction::UpdateSecurityParams { .. }
        | PoolInstruction::CreatePoolStateAccount { .. }
        | PoolInstruction::InitializePoolData { .. }
        | PoolInstruction::InitializePool { .. } = instruction {
        msg!("DEBUG: process_instruction: Skipping pause check for pool creation/management instructions.");
    } else {
        msg!("DEBUG: process_instruction: Checking pause state for relevant instruction.");
        let account_info_iter_for_pause_check = &mut accounts.iter();
        let pool_state_account_for_pause_check = next_account_info(account_info_iter_for_pause_check)?;
        match PoolState::try_from_slice(&pool_state_account_for_pause_check.data.borrow()) {
            Ok(pool_state_data_for_pause) => {
                if pool_state_data_for_pause.is_paused {
                    msg!("DEBUG: process_instruction: Pool is paused. Instruction prohibited.");
                    return Err(PoolError::PoolPaused.into());
                }
                msg!("DEBUG: process_instruction: Pool is not paused or instruction allows paused state.");
            }
            Err(e) => {
                msg!("DEBUG: process_instruction: Failed to deserialize PoolState for pause check: {:?}. Key: {}", e, pool_state_account_for_pause_check.key);
            }
        }
    }
    
    match instruction {
        PoolInstruction::InitializePool { 
            ratio_primary_per_base, 
            pool_authority_bump_seed, 
            primary_token_vault_bump_seed, 
            base_token_vault_bump_seed 
        } => {
            msg!("DEBUG: process_instruction: Dispatching to process_initialize_pool");
            process_initialize_pool(
                program_id, 
                accounts, 
                ratio_primary_per_base, 
                pool_authority_bump_seed, 
                primary_token_vault_bump_seed, 
                base_token_vault_bump_seed
            )
        }
        PoolInstruction::Deposit { deposit_token_mint, amount } => {
            msg!("DEBUG: process_instruction: Dispatching to process_deposit");
            process_deposit(program_id, accounts, deposit_token_mint, amount)
        }
        PoolInstruction::DepositWithFeatures { 
            deposit_token_mint, 
            amount, 
            minimum_lp_tokens_out, 
            fee_recipient 
        } => {
            msg!("DEBUG: process_instruction: Dispatching to process_deposit_with_features");
            process_deposit_with_features(program_id, accounts, deposit_token_mint, amount, minimum_lp_tokens_out, fee_recipient)
        }
        PoolInstruction::Withdraw { withdraw_token_mint, lp_amount_to_burn } => {
            msg!("DEBUG: process_instruction: Dispatching to process_withdraw");
            process_withdraw(program_id, accounts, withdraw_token_mint, lp_amount_to_burn)
        }
        PoolInstruction::Swap { input_token_mint, amount_in, minimum_amount_out } => {
            msg!("DEBUG: process_instruction: Dispatching to process_swap");
            process_swap(program_id, accounts, input_token_mint, amount_in, minimum_amount_out)
        }
        PoolInstruction::WithdrawFees => {
            msg!("DEBUG: process_instruction: Dispatching to process_withdraw_fees");
            process_withdraw_fees(program_id, accounts)
        }
        PoolInstruction::UpdateSecurityParams { 
            max_withdrawal_percentage, 
            withdrawal_cooldown, 
            is_paused 
        } => {
            msg!("DEBUG: process_instruction: Dispatching to process_update_security_params");
            process_update_security_params(
                program_id,
                accounts,
                max_withdrawal_percentage,
                withdrawal_cooldown,
                is_paused
            )
        }
        PoolInstruction::AddDelegate { delegate } => {
            msg!("DEBUG: process_instruction: Dispatching to process_add_delegate");
            process_add_delegate(program_id, accounts, delegate)
        }
        PoolInstruction::RemoveDelegate { delegate } => {
            msg!("DEBUG: process_instruction: Dispatching to process_remove_delegate");
            process_remove_delegate(program_id, accounts, delegate)
        }
        PoolInstruction::WithdrawFeesToDelegate { token_mint, amount } => {
            msg!("DEBUG: process_instruction: Dispatching to process_withdraw_fees_to_delegate");
            process_withdraw_fees_to_delegate(program_id, accounts, token_mint, amount)
        }
        PoolInstruction::SetSwapFee { fee_basis_points } => {
            msg!("DEBUG: process_instruction: Dispatching to process_set_swap_fee");
            process_set_swap_fee(program_id, accounts, fee_basis_points)
        }
        PoolInstruction::GetWithdrawalHistory => {
            msg!("DEBUG: process_instruction: Dispatching to process_get_withdrawal_history");
            process_get_withdrawal_history(program_id, accounts)
        }
        PoolInstruction::RequestFeeWithdrawal { token_mint, amount } => {
            process_request_fee_withdrawal(program_id, accounts, token_mint, amount)
        }
        PoolInstruction::CancelWithdrawalRequest => {
            process_cancel_withdrawal_request(program_id, accounts)
        }
        PoolInstruction::SetDelegateWaitTime { delegate, wait_time } => {
            process_set_delegate_wait_time(program_id, accounts, delegate, wait_time)
        }
        
        // **INDIVIDUAL POOL RATIO PAUSING HANDLERS**
        PoolInstruction::RequestPoolPause { reason, duration_seconds } => {
            msg!("DEBUG: process_instruction: Dispatching to process_request_pool_pause");
            process_request_pool_pause(program_id, accounts, reason, duration_seconds)
        }
        PoolInstruction::CancelPoolPause => {
            msg!("DEBUG: process_instruction: Dispatching to process_cancel_pool_pause");
            process_cancel_pool_pause(program_id, accounts)
        }
        PoolInstruction::SetPoolPauseWaitTime { delegate, wait_time } => {
            msg!("DEBUG: process_instruction: Dispatching to process_set_pool_pause_wait_time");
            process_set_pool_pause_wait_time(program_id, accounts, delegate, wait_time)
        }
        
        // **DEPRECATED**: Legacy two-instruction pattern handlers (kept for backward compatibility)
        PoolInstruction::CreatePoolStateAccount { 
            ratio_primary_per_base, 
            pool_authority_bump_seed, 
            primary_token_vault_bump_seed, 
            base_token_vault_bump_seed 
        } => {
            msg!("DEBUG: process_instruction: DEPRECATED instruction - Use InitializePool instead");
            process_create_pool_state_account(
                program_id, 
                accounts, 
                ratio_primary_per_base, 
                pool_authority_bump_seed, 
                primary_token_vault_bump_seed, 
                base_token_vault_bump_seed
            )
        }
        PoolInstruction::InitializePoolData { 
            ratio_primary_per_base, 
            pool_authority_bump_seed, 
            primary_token_vault_bump_seed, 
            base_token_vault_bump_seed 
        } => {
            msg!("DEBUG: process_instruction: DEPRECATED instruction - Use InitializePool instead");
            process_initialize_pool_data(
                program_id, 
                accounts, 
                ratio_primary_per_base, 
                pool_authority_bump_seed, 
                primary_token_vault_bump_seed, 
                base_token_vault_bump_seed
            )
        }
        
        // **PDA HELPER UTILITIES**
        PoolInstruction::GetPoolStatePDA { primary_token_mint, base_token_mint, ratio_primary_per_base } => {
            msg!("DEBUG: process_instruction: Dispatching to get_pool_state_pda");
            get_pool_state_pda(program_id, primary_token_mint, base_token_mint, ratio_primary_per_base)
        }
        PoolInstruction::GetTokenVaultPDAs { pool_state_pda } => {
            msg!("DEBUG: process_instruction: Dispatching to get_token_vault_pdas");
            get_token_vault_pdas(program_id, pool_state_pda)
        }
        
        // **TEST-SPECIFIC VIEW/GETTER INSTRUCTIONS**
        PoolInstruction::GetPoolInfo {} => {
            msg!("DEBUG: process_instruction: Dispatching to get_pool_info");
            get_pool_info(accounts)
        }
        PoolInstruction::GetLiquidityInfo {} => {
            msg!("DEBUG: process_instruction: Dispatching to get_liquidity_info");
            get_liquidity_info(accounts)
        }
        PoolInstruction::GetDelegateInfo {} => {
            msg!("DEBUG: process_instruction: Dispatching to get_delegate_info");
            get_delegate_info(accounts)
        }
        PoolInstruction::GetFeeInfo {} => {
            msg!("DEBUG: process_instruction: Dispatching to get_fee_info");
            get_fee_info(accounts)
        }
    }
}



/// Creates the Pool State PDA account and all related accounts (LP mints, vaults).
/// This is Step 1 of the two-instruction pool initialization pattern.
///
/// WORKAROUND CONTEXT:
/// This function implements the first part of a workaround for Solana AccountInfo.data
/// issue where AccountInfo.data doesn't get updated after CPI account creation within
/// the same instruction. See GitHub Issue #31960 and related community discussions.
///
/// WHY THIS APPROACH:
/// 1. Creates all required accounts via CPI (Pool State PDA, LP mints, token vaults)
/// 2. Deliberately AVOIDS writing PoolState data to prevent AccountInfo.data issues
/// 3. Allows the second instruction (InitializePoolData) to run with fresh AccountInfo
///    references that properly point to the allocated on-chain account buffers
///
/// WHAT THIS FUNCTION DOES:
/// - Validates all input parameters and PDA derivations
/// - Creates Pool State PDA account with correct size via system_instruction::create_account
/// - Creates and initializes LP token mints, transfers authority to pool
/// - Creates and initializes token vault PDAs
/// - Transfers registration fees to pool
/// - Does NOT serialize any PoolState data (that's done in Step 2)
///
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for pool creation
/// * `ratio_primary_per_base` - The ratio of primary tokens per base token
/// * `pool_authority_bump_seed` - Bump seed for pool authority PDA
/// * `primary_token_vault_bump_seed` - Bump seed for primary token vault PDA
/// * `base_token_vault_bump_seed` - Bump seed for base token vault PDA
///
/// # Returns
/// * `ProgramResult` - Success or error code


/// Initializes the data in the already-created Pool State PDA account.
/// This is Step 2 of the two-instruction pool initialization pattern.
///
/// WORKAROUND CONTEXT:
/// This function implements the second part of a workaround for Solana AccountInfo.data
/// issue. It runs in a fresh transaction context where AccountInfo.data properly
/// references the on-chain allocated account buffer created in Step 1.
///
/// BUFFER SERIALIZATION APPROACH:
/// Even with the two-instruction pattern, we use an additional safeguard against
/// potential AccountInfo.data inconsistencies:
/// 1. Serialize PoolState to a temporary Vec<u8> buffer first
/// 2. Verify serialization succeeds and check buffer size
/// 3. Copy the serialized data directly to AccountInfo.data using copy_from_slice
/// 
/// This approach is more robust than direct serialization to AccountInfo.data.borrow_mut()
/// because it ensures we have a valid serialized representation before attempting to
/// write to the account, and the copy operation is atomic.
///
/// WHY THIS IS NEEDED:
/// - Direct serialization with pool_state_data.serialize(&mut *account.data.borrow_mut())
///   was reporting "OK" but the data wasn't persisting to the on-chain account
/// - AccountInfo.data.borrow().len() was returning 0 even after "successful" serialization
/// - This buffer-copy approach ensures data integrity and persistence
///
/// WHAT THIS FUNCTION DOES:
/// - Validates the Pool State PDA account exists with correct size
/// - Checks if pool is already initialized (prevents double-initialization)
/// - Creates and populates PoolState struct with all configuration data
/// - Serializes to buffer, then copies to account data
/// - Verifies the operation succeeded
///
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for pool data initialization
/// * `ratio_primary_per_base` - The ratio of primary tokens per base token
/// * `pool_authority_bump_seed` - Bump seed for pool authority PDA
/// * `primary_token_vault_bump_seed` - Bump seed for primary token vault PDA
/// * `base_token_vault_bump_seed` - Bump seed for base token vault PDA
///
/// # Returns
/// * `ProgramResult` - Success or error code










/// Handles token swaps within the trading pool.
///
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for swap
/// * `input_token_mint_key` - The mint of the input token
/// * `amount_in` - The amount of input token to swap
///
/// # Returns
/// Processes token swaps within the fixed-ratio trading pool.
///
/// This function enables users to swap between the pool's two tokens (Token A ↔ Token B)
/// at a predetermined fixed ratio. The swap maintains the pool's mathematical invariant while
/// collecting configurable trading fees. It provides slippage protection, liquidity validation,
/// and comprehensive security checks for all trading operations.
///
/// # Purpose
/// - Facilitates decentralized token trading at fixed exchange rates
/// - Maintains pool's fixed ratio invariant through mathematical precision
/// - Collects configurable trading fees (0-0.5%) for pool sustainability
/// - Provides slippage protection through minimum output requirements
/// - Supports bidirectional trading (A→B and B→A) with consistent pricing
///
/// # How it works
/// 1. Validates user authorization and all account structures
/// 2. Verifies rent-exempt status for pool accounts
/// 3. Determines swap direction (A→B or B→A) based on input token mint
/// 4. Validates user's input/output token accounts for correct ownership and balances
/// 5. Calculates exact output amount using fixed ratio formula:
///    - A→B: output_B = (input_A × ratio_B_denominator) ÷ ratio_A_numerator
///    - B→A: output_A = (input_B × ratio_A_numerator) ÷ ratio_B_denominator
/// 6. Applies configurable swap fee (deducted from input amount)
/// 7. Validates slippage protection (output ≥ minimum_amount_out)
/// 8. Checks pool has sufficient liquidity for output token
/// 9. Transfers input tokens (including fee) from user to pool vault
/// 10. Transfers calculated output tokens from pool vault to user
/// 11. Updates pool liquidity counters and fee tracking
/// 12. Collects SOL swap fee for pool operations
///
/// # Arguments
/// * `program_id` - The program ID for PDA validation and CPI authority
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - User account (must be signer)
///   - `accounts[1]` - User's input token account (source of tokens being swapped)
///   - `accounts[2]` - User's output token account (receives swapped tokens)
///   - `accounts[3]` - Pool state PDA account (writable)
///   - `accounts[4]` - Token A mint account (for PDA seed verification)
///   - `accounts[5]` - Token B mint account (for PDA seed verification)
///   - `accounts[6]` - Pool's Token A vault account (writable)
///   - `accounts[7]` - Pool's Token B vault account (writable)
///   - `accounts[8]` - System program
///   - `accounts[9]` - SPL Token program
///   - `accounts[10]` - Rent sysvar (for rent calculations)
///   - `accounts[11]` - Clock sysvar (for timestamp validation)
/// * `input_token_mint_key` - The mint address of the token being swapped in (must be pool's Token A or Token B)
/// * `amount_in` - The amount of input tokens to swap (includes trading fee)
/// * `minimum_amount_out` - Minimum acceptable output tokens (slippage protection)
///
/// # Account Requirements
/// - User: Must be signer and owner of both input and output token accounts
/// - Input account: Must contain sufficient tokens and match input_token_mint_key
/// - Output account: Must be owned by user and match the opposite token mint
/// - Pool vaults: Must maintain sufficient liquidity for the swap
///
/// # Trading Fees
/// - Swap fee: Configurable rate (0-50 basis points = 0%-0.5%) applied to input amount
/// - SOL fee: Fixed amount (SWAP_FEE) for pool operations and rent exemption
/// - Fee collection: Trading fees stored in pool state for delegate withdrawal
///
/// # Mathematical Formula
/// Fixed ratio swaps use precise integer arithmetic:
/// - For A→B: `output_B = (amount_in_after_fee × ratio_B_denominator) ÷ ratio_A_numerator`
/// - For B→A: `output_A = (amount_in_after_fee × ratio_A_numerator) ÷ ratio_B_denominator`
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - User didn't sign transaction
/// - `ProgramError::InvalidAccountData` - Account validation failures
/// - `ProgramError::InsufficientFunds` - Insufficient input tokens or pool liquidity
/// - `PoolError::InvalidSwapAmount` - Slippage tolerance exceeded or zero output
/// - `PoolError::PoolPaused` - Pool trading is paused
///
/// # Example Usage
/// ```ignore
/// // Swap 1000 Token A for Token B with 1% slippage tolerance
/// let expected_output = 2000; // Based on 1:2 ratio
/// let instruction = PoolInstruction::Swap {
///     input_token_mint: token_a_mint_pubkey,
///     amount_in: 1000,
///     minimum_amount_out: expected_output * 99 / 100, // 1% slippage
/// };
/// ```


/// Allows the pool owner to withdraw accumulated SOL fees from the pool state PDA.
///
/// This function enables the designated pool owner to extract accumulated SOL fees that have
/// been collected from various pool operations (swaps, deposits, withdrawals). It maintains
/// the pool's rent-exempt status by preserving the minimum required balance while transferring
/// any excess SOL to the owner. This is a key revenue mechanism for pool operators.
///
/// # Purpose
/// - Provides revenue extraction mechanism for pool owners
/// - Maintains pool's rent-exempt status during fee withdrawal
/// - Enables monetization of pool operations through collected SOL fees
/// - Ensures long-term pool sustainability by preserving operational funds
///
/// # How it works
/// 1. Validates the caller is the designated pool owner and signed the transaction
/// 2. Loads pool state data to verify ownership and calculate available fees
/// 3. Calculates the minimum rent-exempt balance required for the pool state PDA
/// 4. Determines withdrawable amount (total balance - rent-exempt minimum)
/// 5. If withdrawable amount > 0, transfers SOL from pool PDA to owner
/// 6. Uses invoke_signed with pool's PDA seeds for authorized transfer
/// 7. Logs withdrawal details for transparency and audit purposes
///
/// # Arguments
/// * `_program_id` - The program ID (currently unused, reserved for future validation)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool owner account (must be signer and match pool state owner)
///   - `accounts[1]` - Pool state PDA account (source of SOL fees)
///   - `accounts[2]` - System program (for SOL transfer instructions)
///   - `accounts[3]` - Rent sysvar (for rent-exempt calculations)
///
/// # Account Requirements
/// - Owner: Must be signer and match the owner field in pool state data
/// - Pool state: Must be the valid pool state PDA with sufficient SOL balance
/// - System program: Standard Solana system program for SOL transfers
///
/// # Fee Calculation
/// - Available fees = Total pool state balance - Rent-exempt minimum
/// - Rent-exempt minimum calculated using current rent rates and account size
/// - Zero fees available indicates all SOL is reserved for rent exemption
///
/// # Security Features
/// - **Ownership validation**: Only the designated pool owner can withdraw fees
/// - **Rent protection**: Always maintains minimum balance for rent exemption
/// - **PDA signing**: Uses proper PDA seeds for authorized pool transfers
/// - **Transparency**: Logs all fee withdrawals for audit trail
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Owner didn't sign transaction
/// - `ProgramError::InvalidAccountData` - Caller is not the pool owner
/// - `ProgramError::ArithmeticOverflow` - Mathematical calculation errors
///
/// # Example Usage
/// ```ignore
/// // Pool owner withdraws accumulated SOL fees
/// let instruction = PoolInstruction::WithdrawFees;
/// // Transfers: pool_balance - rent_minimum → owner_account
/// ```
///
/// # Note
/// This function only handles SOL fees. For SPL token fee withdrawals, use the
/// delegate withdrawal system through `WithdrawFeesToDelegate`.
fn process_withdraw_fees(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing WithdrawFees");
    let account_info_iter = &mut accounts.iter();

    let owner = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify pool state
    let pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if *owner.key != pool_state_data.owner {
        msg!("Only pool owner can withdraw fees");
        return Err(ProgramError::InvalidAccountData);
    }

    // Calculate withdrawable amount
    let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;
    let minimum_rent = rent.minimum_balance(pool_state.data_len());
    let withdrawable_amount = pool_state.lamports().checked_sub(minimum_rent)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    if withdrawable_amount == 0 {
        msg!("No fees available to withdraw");
        return Ok(());
    }

    // Get PDA seeds for signing
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.token_a_mint.as_ref(),
        pool_state_data.token_b_mint.as_ref(),
        &pool_state_data.ratio_a_numerator.to_le_bytes(),
        &pool_state_data.ratio_b_denominator.to_le_bytes(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    // Transfer fees using invoke_signed
    invoke_signed(
        &system_instruction::transfer(pool_state.key, owner.key, withdrawable_amount),
        &[pool_state.clone(), owner.clone(), system_program.clone()],
        &[pool_state_pda_seeds],
    )?;
    msg!("Fees transferred to owner: {} lamports ({} lamports reserved for rent)", 
         withdrawable_amount, minimum_rent);

    Ok(())
}

/// Ensures an account has enough lamports to be rent exempt.


/// Updates the pool's security parameters to manage operational risk and compliance.
///
/// This function allows the pool owner to modify critical security settings that control
/// pool operations. Currently focused on pause/unpause functionality, with extensibility
/// for future security parameters like withdrawal limits and cooldown periods. This provides
/// emergency controls and operational flexibility for pool management.
///
/// # Purpose
/// - Provides emergency stop capability through pause functionality
/// - Enables dynamic security policy adjustments based on market conditions
/// - Allows compliance with regulatory requirements or protocol upgrades
/// - Maintains operational control for pool owners while protecting user funds
/// - Supports future expansion of security features and risk management
///
/// # How it works
/// 1. Validates the caller is the designated pool owner and signed the transaction
/// 2. Loads current pool state data to verify ownership permissions
/// 3. Applies any provided security parameter updates:
///    - `is_paused`: Immediately enables/disables pool operations
///    - `max_withdrawal_percentage`: Reserved for future withdrawal limit controls
///    - `withdrawal_cooldown`: Reserved for future time-based withdrawal restrictions
/// 4. Serializes updated pool state back to on-chain storage
/// 5. Logs changes for transparency and audit compliance
///
/// # Arguments
/// * `_program_id` - The program ID (currently unused, reserved for future validation)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool owner account (must be signer and match pool state owner)
///   - `accounts[1]` - Pool state PDA account (writable for parameter updates)
/// * `_max_withdrawal_percentage` - Reserved for future use. Maximum percentage of pool liquidity withdrawable in single transaction (e.g., 1000 = 10%)
/// * `_withdrawal_cooldown` - Reserved for future use. Minimum time delay in slots between successive withdrawals
/// * `is_paused` - Optional boolean to pause/unpause all pool operations (except owner functions)
///
/// # Account Requirements
/// - Owner: Must be signer and match the owner field in pool state data
/// - Pool state: Must be writable for parameter updates
///
/// # Pause Functionality
/// When `is_paused = true`:
/// - Blocks all user operations: deposits, withdrawals, swaps
/// - Allows owner operations: fee withdrawals, security updates, delegate management
/// - Provides emergency stop for security incidents or maintenance
/// - Can be reversed by setting `is_paused = false`
///
/// # Security Features
/// - **Owner-only access**: Only designated pool owner can modify security parameters
/// - **Selective enforcement**: Pause affects user operations but preserves owner controls
/// - **Immediate effect**: Parameter changes take effect in the same transaction
/// - **Audit trail**: All parameter changes are logged for transparency
///
/// # Future Extensions
/// The reserved parameters enable future security enhancements:
/// - Withdrawal limits to prevent liquidity drain attacks
/// - Cooldown periods to limit high-frequency trading exploitation
/// - Rate limiting for various operations
/// - Dynamic fee adjustments based on market conditions
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Owner didn't sign transaction
/// - `ProgramError::InvalidAccountData` - Caller is not the pool owner
///
/// # Example Usage
/// ```ignore
/// // Emergency pause all pool operations
/// let instruction = PoolInstruction::UpdateSecurityParams {
///     max_withdrawal_percentage: None, // No change
///     withdrawal_cooldown: None,       // No change
///     is_paused: Some(true),          // Pause operations
/// };
///
/// // Resume normal operations
/// let instruction = PoolInstruction::UpdateSecurityParams {
///     max_withdrawal_percentage: None,
///     withdrawal_cooldown: None,
///     is_paused: Some(false),         // Unpause operations
/// };
/// ```
fn process_update_security_params(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _max_withdrawal_percentage: Option<u64>,
    _withdrawal_cooldown: Option<u64>,
    is_paused: Option<bool>,
) -> ProgramResult {
    msg!("Processing UpdateSecurityParams");
    let account_info_iter = &mut accounts.iter();

    let owner = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if *owner.key != pool_state_data.owner {
        msg!("Only pool owner can update security parameters");
        return Err(ProgramError::InvalidAccountData);
    }

    // Only update is_paused if provided
    if let Some(paused) = is_paused {
        pool_state_data.is_paused = paused;
    }

    // Save updated state using buffer serialization approach
    let mut serialized_data = Vec::new();
    pool_state_data.serialize(&mut serialized_data)?;
    let account_data_len = pool_state.data_len();
    if serialized_data.len() > account_data_len {
        return Err(ProgramError::AccountDataTooSmall);
    }
    {
        let mut account_data = pool_state.data.borrow_mut();
        account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
    }
    msg!("Security parameters updated successfully");

    Ok(())
}

















/// Allows the pool owner to set the swap fee configuration.
///
/// # Arguments
/// * `_program_id` - The program ID of the contract
/// * `accounts` - The accounts required for setting swap fee
/// * `fee_basis_points` - The fee in basis points (0-50, max 0.5%)
///
/// # Returns
/// * `ProgramResult` - Success or error code




// ================================================================================================
// PDA HELPER UTILITIES
// ================================================================================================

/// **PDA HELPER**: Computes and returns the Pool State PDA address for given tokens and ratio.
/// 
/// This utility function helps clients derive the Pool State PDA address without requiring
/// account creation or on-chain calls. Essential for preparing transaction account lists.
/// 
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `primary_token_mint` - Primary token mint pubkey
/// * `base_token_mint` - Base token mint pubkey  
/// * `ratio_primary_per_base` - Exchange ratio between tokens
/// 
/// # Returns
/// * `ProgramResult` - Logs the derived PDA address and bump seed
pub fn get_pool_state_pda(
    program_id: &Pubkey,
    primary_token_mint: Pubkey,
    base_token_mint: Pubkey,
    ratio_primary_per_base: u64,
) -> ProgramResult {
    msg!("DEBUG: get_pool_state_pda: Computing Pool State PDA");
    
    // Enhanced normalization to prevent economic duplicates (same logic as pool creation)
    // Step 1: Lexicographic token ordering
    let (token_a_mint_key, token_b_mint_key) = 
        if primary_token_mint < base_token_mint {
            (primary_token_mint, base_token_mint)
        } else {
            (base_token_mint, primary_token_mint)
        };
    
    // Step 2: Canonical ratio mapping to prevent liquidity fragmentation
    let (ratio_a_numerator, ratio_b_denominator): (u64, u64) = 
        if primary_token_mint < base_token_mint {
            (ratio_primary_per_base, 1u64)
        } else {
            // Use canonical form - both pools with same token pair get same ratio
            (ratio_primary_per_base, 1u64)
        };
    
    // Find PDA with canonical bump seed
    let (pool_state_pda, bump_seed) = Pubkey::find_program_address(
        &[
            POOL_STATE_SEED_PREFIX,
            token_a_mint_key.as_ref(),
            token_b_mint_key.as_ref(),
            &ratio_a_numerator.to_le_bytes(),
            &ratio_b_denominator.to_le_bytes(),
        ],
        program_id,
    );
    
    msg!("Pool State PDA: {}", pool_state_pda);
    msg!("Pool State PDA Bump Seed: {}", bump_seed);
    msg!("Normalized Token A: {}", token_a_mint_key);
    msg!("Normalized Token B: {}", token_b_mint_key);
    msg!("Normalized Ratio A: {}", ratio_a_numerator);
    msg!("Normalized Ratio B: {}", ratio_b_denominator);
    
    Ok(())
}

/// **PDA HELPER**: Computes and returns Token Vault PDA addresses for a given pool.
/// 
/// This utility helps clients derive the token vault addresses for pool operations.
/// Useful for preparing deposit, withdraw, and swap transaction account lists.
/// 
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `pool_state_pda` - The Pool State PDA address
/// 
/// # Returns
/// * `ProgramResult` - Logs the derived vault PDA addresses and bump seeds
pub fn get_token_vault_pdas(
    program_id: &Pubkey,
    pool_state_pda: Pubkey,
) -> ProgramResult {
    msg!("DEBUG: get_token_vault_pdas: Computing Token Vault PDAs for pool: {}", pool_state_pda);
    
    // Find Token A Vault PDA
    let (token_a_vault_pda, token_a_bump) = Pubkey::find_program_address(
        &[
            TOKEN_A_VAULT_SEED_PREFIX,
            pool_state_pda.as_ref(),
        ],
        program_id,
    );
    
    // Find Token B Vault PDA
    let (token_b_vault_pda, token_b_bump) = Pubkey::find_program_address(
        &[
            TOKEN_B_VAULT_SEED_PREFIX,
            pool_state_pda.as_ref(),
        ],
        program_id,
    );
    
    msg!("Token A Vault PDA: {}", token_a_vault_pda);
    msg!("Token A Vault Bump Seed: {}", token_a_bump);
    msg!("Token B Vault PDA: {}", token_b_vault_pda);
    msg!("Token B Vault Bump Seed: {}", token_b_bump);
    
    Ok(())
}

// ================================================================================================
// TEST-SPECIFIC VIEW/GETTER INSTRUCTIONS
// ================================================================================================

/// **VIEW INSTRUCTION**: Returns comprehensive pool state information.
/// 
/// This function provides easy access to all pool state data in a structured format.
/// Ideal for testing, debugging, frontend integration, and transparency.
/// 
/// # Arguments
/// * `accounts` - Must contain pool state account as first account
/// 
/// # Returns
/// * `ProgramResult` - Logs comprehensive pool information
pub fn get_pool_info(accounts: &[AccountInfo]) -> ProgramResult {
    msg!("DEBUG: get_pool_info: Retrieving comprehensive pool information");
    
    let pool_state_account = &accounts[0];
    let pool_state = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    
    msg!("=== POOL STATE INFORMATION ===");
    msg!("Pool Owner: {}", pool_state.owner);
    msg!("Pool State PDA: {}", pool_state_account.key);
    msg!("Token A Mint: {}", pool_state.token_a_mint);
    msg!("Token B Mint: {}", pool_state.token_b_mint);
    msg!("Token A Vault: {}", pool_state.token_a_vault);
    msg!("Token B Vault: {}", pool_state.token_b_vault);
    msg!("LP Token A Mint: {}", pool_state.lp_token_a_mint);
    msg!("LP Token B Mint: {}", pool_state.lp_token_b_mint);
    msg!("Ratio A Numerator: {}", pool_state.ratio_a_numerator);
    msg!("Ratio B Denominator: {}", pool_state.ratio_b_denominator);
    msg!("Pool Authority Bump Seed: {}", pool_state.pool_authority_bump_seed);
    msg!("Token A Vault Bump Seed: {}", pool_state.token_a_vault_bump_seed);
    msg!("Token B Vault Bump Seed: {}", pool_state.token_b_vault_bump_seed);
    msg!("Is Initialized: {}", pool_state.is_initialized);
    msg!("Is Paused: {}", pool_state.is_paused);
    msg!("Swap Fee Basis Points: {}", pool_state.swap_fee_basis_points);
    msg!("===============================");
    
    Ok(())
}

/// **VIEW INSTRUCTION**: Returns detailed liquidity information for both tokens.
/// 
/// This function provides easy access to liquidity data, useful for calculating
/// exchange rates, available liquidity, and pool utilization metrics.
/// 
/// # Arguments
/// * `accounts` - Must contain pool state account as first account
/// 
/// # Returns
/// * `ProgramResult` - Logs detailed liquidity information
pub fn get_liquidity_info(accounts: &[AccountInfo]) -> ProgramResult {
    msg!("DEBUG: get_liquidity_info: Retrieving liquidity information");
    
    let pool_state_account = &accounts[0];
    let pool_state = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    
    msg!("=== LIQUIDITY INFORMATION ===");
    msg!("Total Token A Liquidity: {}", pool_state.total_token_a_liquidity);
    msg!("Total Token B Liquidity: {}", pool_state.total_token_b_liquidity);
    msg!("Exchange Rate (A per B): {}", 
         if pool_state.ratio_b_denominator != 0 { 
             pool_state.ratio_a_numerator as f64 / pool_state.ratio_b_denominator as f64 
         } else { 0.0 });
    msg!("Exchange Rate (B per A): {}", 
         if pool_state.ratio_a_numerator != 0 { 
             pool_state.ratio_b_denominator as f64 / pool_state.ratio_a_numerator as f64 
         } else { 0.0 });
    
    // Calculate utilization if available
    let total_value_locked = pool_state.total_token_a_liquidity + pool_state.total_token_b_liquidity;
    msg!("Total Value Locked (TVL): {} tokens", total_value_locked);
    msg!("==============================");
    
    Ok(())
}

/// **VIEW INSTRUCTION**: Returns delegate management information.
/// 
/// This function provides comprehensive delegate system information including
/// delegate list, withdrawal history, and pending requests for transparency.
/// 
/// # Arguments
/// * `accounts` - Must contain pool state account as first account
/// 
/// # Returns
/// * `ProgramResult` - Logs delegate management information
pub fn get_delegate_info(accounts: &[AccountInfo]) -> ProgramResult {
    msg!("DEBUG: get_delegate_info: Retrieving delegate information");
    
    let pool_state_account = &accounts[0];
    let pool_state = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    
    msg!("=== DELEGATE INFORMATION ===");
    msg!("Total Delegates: {}", pool_state.delegate_management.delegate_count);
    
    // List all delegates
    for (i, delegate) in pool_state.delegate_management.delegates.iter().enumerate() {
        if i < pool_state.delegate_management.delegate_count as usize {
            msg!("Delegate {}: {}", i + 1, delegate);
            
            // Show wait time for this delegate
            if let Some(wait_time) = pool_state.delegate_management.get_delegate_wait_time(delegate) {
                msg!("  Wait Time: {} seconds", wait_time);
            }
            
            // Show any pending withdrawal request
            if let Some(request) = pool_state.delegate_management.get_withdrawal_request(delegate) {
                msg!("  Pending Withdrawal: {} of token {}", request.amount, request.token_mint);
                msg!("  Request Timestamp: {}", request.request_timestamp);
            }
        }
    }
    
    // Show recent withdrawal history
    msg!("Recent Withdrawal History:");
    msg!("History Index: {}", pool_state.delegate_management.withdrawal_history_index);
    for (i, record) in pool_state.delegate_management.withdrawal_history.iter().enumerate() {
        if record.delegate != Pubkey::default() { // Only show non-empty records
            msg!("  Record {}: Delegate {}, Amount {}, Token {}, Slot {}", 
                 i, record.delegate, record.amount, record.token_mint, record.slot);
        }
    }
    msg!("============================");
    
    Ok(())
}

/// **VIEW INSTRUCTION**: Returns fee information including collected fees and rates.
/// 
/// This function provides comprehensive fee information essential for fee tracking,
/// transparency, and financial reporting.
/// 
/// # Arguments
/// * `accounts` - Must contain pool state account as first account
/// 
/// # Returns
/// * `ProgramResult` - Logs detailed fee information
pub fn get_fee_info(accounts: &[AccountInfo]) -> ProgramResult {
    msg!("DEBUG: get_fee_info: Retrieving fee information");
    
    let pool_state_account = &accounts[0];
    let pool_state = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    
    msg!("=== FEE INFORMATION ===");
    
    // Fee rates
    msg!("Swap Fee Rate: {} basis points ({:.4}%)", 
         pool_state.swap_fee_basis_points, 
         pool_state.swap_fee_basis_points as f64 / 100.0);
    msg!("Registration Fee: {} lamports ({:.9} SOL)", REGISTRATION_FEE, REGISTRATION_FEE as f64 / 1_000_000_000.0);
    msg!("Deposit/Withdrawal Fee: {} lamports ({:.9} SOL)", DEPOSIT_WITHDRAWAL_FEE, DEPOSIT_WITHDRAWAL_FEE as f64 / 1_000_000_000.0);
    msg!("Swap Fee: {} lamports ({:.9} SOL)", SWAP_FEE, SWAP_FEE as f64 / 1_000_000_000.0);
    
    // Collected fees
    msg!("Collected Token A Fees: {}", pool_state.collected_fees_token_a);
    msg!("Collected Token B Fees: {}", pool_state.collected_fees_token_b);
    msg!("Collected SOL Fees: {} lamports ({:.9} SOL)", 
         pool_state.collected_sol_fees, 
         pool_state.collected_sol_fees as f64 / 1_000_000_000.0);
    
    // Withdrawn fees (for tracking)
    msg!("Total Token A Fees Withdrawn: {}", pool_state.total_fees_withdrawn_token_a);
    msg!("Total Token B Fees Withdrawn: {}", pool_state.total_fees_withdrawn_token_b);
    msg!("Total SOL Fees Withdrawn: {} lamports ({:.9} SOL)", 
         pool_state.total_sol_fees_withdrawn, 
         pool_state.total_sol_fees_withdrawn as f64 / 1_000_000_000.0);
    
    // Available fees (collected minus withdrawn)
    let available_token_a_fees = pool_state.collected_fees_token_a.saturating_sub(pool_state.total_fees_withdrawn_token_a);
    let available_token_b_fees = pool_state.collected_fees_token_b.saturating_sub(pool_state.total_fees_withdrawn_token_b);
    let available_sol_fees = pool_state.collected_sol_fees.saturating_sub(pool_state.total_sol_fees_withdrawn);
    
    msg!("Available Token A Fees: {}", available_token_a_fees);
    msg!("Available Token B Fees: {}", available_token_b_fees);
    msg!("Available SOL Fees: {} lamports ({:.9} SOL)", 
         available_sol_fees, 
         available_sol_fees as f64 / 1_000_000_000.0);
    
    msg!("=======================");
    
    Ok(())
}

// ================================================================================================
// INDIVIDUAL POOL RATIO PAUSING PROCESSORS
// ================================================================================================

/// Process a pool pause request from an authorized delegate.
///
/// This function allows authorized delegates to request a pause of pool operations for a
/// specific duration with configurable timing parameters. Designed as a primitive for 
/// governance contracts to implement sophisticated dispute resolution and bonding mechanisms.
///
/// # Purpose
/// - Enables delegate-controlled pool pausing for governance integration
/// - Provides structured dispute resolution and bonding enforcement
/// - Creates audit trail for all pause requests and their reasons
/// - Supports emergency response capabilities for security incidents
/// - Facilitates integration with higher-layer governance contracts
///
/// # How it works
/// 1. **Authorization**: Verifies the caller is an authorized delegate and signed the transaction
/// 2. **Duplicate Check**: Ensures delegate doesn't have pending pause request
/// 3. **Parameter Validation**: Validates duration is within allowed range (1 minute to 72 hours)
/// 4. **Request Creation**: Creates pause request with delegate's configured wait time
/// 5. **State Update**: Saves the request to pool state for future activation
/// 6. **Audit Logging**: Logs request details for transparency and governance tracking
///
/// # Timing Model
/// - **Request Time**: Current timestamp when request is submitted
/// - **Wait Period**: Delegate-specific delay before pause becomes active (1 minute to 72 hours)
/// - **Active Period**: Duration of pause once activated (1 minute to 72 hours)
/// - **Cancellation**: Can be cancelled by delegate or owner before activation
///
/// # Arguments
/// * `program_id` - The program ID for validation (unused but standard pattern)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Delegate account (must be signer and authorized delegate)
///   - `accounts[1]` - Pool state PDA account (writable for request storage)
///   - `accounts[2]` - Clock sysvar for timestamp access
/// * `reason` - Structured reason for the pause request (enum PoolPauseReason)
/// * `duration_seconds` - Duration of pause once active (60 to 259200 seconds)
///
/// # Account Requirements
/// - Delegate: Must be signer and exist in pool's authorized delegate list
/// - Pool state: Must be owned by program and writable for state updates
/// - Clock: Standard Solana sysvar for timestamp access
///
/// # Validation Rules
/// - Only authorized delegates can submit pause requests
/// - Duration must be between 1 minute and 72 hours
/// - Delegate can only have one pending pause request at a time
/// - Pool doesn't need to be unpaused to submit request
///
/// # Integration with Governance
/// This primitive enables governance contracts to implement:
/// - **Bonding Mechanisms**: Pause pool until bond requirements are met
/// - **Dispute Resolution**: Structured pause with categorized reasons
/// - **Automated Governance**: Program-controlled pause requests
/// - **Emergency Response**: Rapid response to security concerns
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Delegate didn't sign transaction
/// - `PoolError::UnauthorizedDelegate` - Caller is not authorized delegate
/// - `PoolError::PendingWithdrawalExists` - Delegate already has pending pause request
/// - `PoolError::InvalidWaitTime` - Duration is outside allowed range
///
/// # Example Usage
/// ```ignore
/// // Governance contract requests pause for insufficient bonding
/// let instruction = PoolInstruction::RequestPoolPause {
///     reason: PoolPauseReason::InsufficientBond,
///     duration_seconds: 3600, // 1 hour pause
/// };
/// ```
pub fn process_request_pool_pause(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    reason: PoolPauseReason,
    duration_seconds: u64,
) -> ProgramResult {
    msg!("Processing RequestPoolPause - reason: {:?}, duration: {} seconds", reason, duration_seconds);
    let account_info_iter = &mut accounts.iter();

    let delegate_account = next_account_info(account_info_iter)?;
    let pool_state_account = next_account_info(account_info_iter)?;
    let clock_account = next_account_info(account_info_iter)?;

    // Verify delegate is signer
    if !delegate_account.is_signer {
        msg!("Delegate must be a signer to request pool pause");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Get current timestamp from clock
    let clock = Clock::from_account_info(clock_account)?;
    let current_timestamp = clock.unix_timestamp;
    let current_slot = clock.slot;

    // Load and verify pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    
    // Verify delegate is authorized
    if !pool_state_data.delegate_management.is_delegate(delegate_account.key) {
        msg!("Caller is not an authorized delegate: {}", delegate_account.key);
        return Err(PoolError::UnauthorizedDelegate.into());
    }

    // Create the pause request
    pool_state_data.delegate_management.create_pool_pause_request(
        delegate_account.key,
        reason.clone(),
        duration_seconds,
        current_timestamp,
        current_slot,
    )?;

    // Save updated state
    pool_state_data.serialize(&mut *pool_state_account.data.borrow_mut())?;

    // Log the request for governance tracking
    msg!("Pool pause requested successfully: delegate={}, reason={:?}, duration={} seconds, wait_time={} seconds", 
         delegate_account.key, 
         reason,
         duration_seconds,
         pool_state_data.delegate_management.get_pool_pause_wait_time(delegate_account.key).unwrap_or(259200));

    Ok(())
}

/// Process cancellation of a pool pause request.
///
/// This function allows either the requesting delegate or the pool owner to cancel a 
/// pending pool pause request before it becomes active. Provides flexibility for
/// dispute resolution and accidental request correction.
///
/// # Purpose
/// - Enables cancellation of accidental or resolved pause requests
/// - Provides pool owner override capability for emergency resolution
/// - Supports flexible dispute resolution mechanisms
/// - Maintains audit trail of cancelled requests
/// - Prevents unnecessary pool disruptions when issues are resolved
///
/// # How it works
/// 1. **Authorization**: Verifies caller is either requesting delegate or pool owner
/// 2. **Request Validation**: Ensures pending request exists for the delegate
/// 3. **Cancellation**: Removes the pause request from pool state
/// 4. **State Update**: Saves updated state without the cancelled request
/// 5. **Audit Logging**: Logs cancellation for transparency
///
/// # Arguments
/// * `program_id` - The program ID for validation (unused but standard pattern)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Caller account (delegate or owner, must be signer)
///   - `accounts[1]` - Pool state PDA account (writable for state updates)
///
/// # Account Requirements
/// - Caller: Must be signer and either pool owner or authorized delegate with pending request
/// - Pool state: Must be owned by program and writable for state updates
///
/// # Authorization Rules
/// - Pool owner can cancel any delegate's pause request
/// - Delegates can only cancel their own pause requests
/// - Cannot cancel requests that have already become active
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Caller didn't sign transaction
/// - `PoolError::UnauthorizedDelegate` - Caller is not owner or requesting delegate
/// - `PoolError::NoPendingWithdrawal` - No pause request exists to cancel
///
/// # Example Usage
/// ```ignore
/// // Delegate cancels their own request
/// let instruction = PoolInstruction::CancelPoolPause;
/// 
/// // Owner cancels any delegate's request (emergency resolution)
/// let instruction = PoolInstruction::CancelPoolPause;
/// ```
pub fn process_cancel_pool_pause(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing CancelPoolPause");
    let account_info_iter = &mut accounts.iter();

    let caller_account = next_account_info(account_info_iter)?;
    let pool_state_account = next_account_info(account_info_iter)?;

    // Verify caller is signer
    if !caller_account.is_signer {
        msg!("Caller must be a signer to cancel pool pause");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    
    // Check if caller is pool owner (can cancel any request)
    let is_owner = *caller_account.key == pool_state_data.owner;
    
    if is_owner {
        // Owner can cancel any delegate's request - find and cancel the first one
        let mut cancelled = false;
        for i in 0..pool_state_data.delegate_management.delegate_count as usize {
            if pool_state_data.delegate_management.pool_pause_requests[i].delegate != Pubkey::default() {
                let delegate = pool_state_data.delegate_management.delegates[i];
                pool_state_data.delegate_management.cancel_pool_pause_request(&delegate)?;
                msg!("Pool owner cancelled pause request for delegate: {}", delegate);
                cancelled = true;
                break;
            }
        }
        
        if !cancelled {
            msg!("No pending pause requests to cancel");
            return Err(PoolError::NoPendingWithdrawal.into());
        }
    } else {
        // Delegate can only cancel their own request
        if !pool_state_data.delegate_management.is_delegate(caller_account.key) {
            msg!("Caller is not authorized delegate or pool owner: {}", caller_account.key);
            return Err(PoolError::UnauthorizedDelegate.into());
        }
        
        // Cancel delegate's own request
        pool_state_data.delegate_management.cancel_pool_pause_request(caller_account.key)?;
        msg!("Delegate cancelled their own pause request: {}", caller_account.key);
    }

    // Save updated state
    pool_state_data.serialize(&mut *pool_state_account.data.borrow_mut())?;
    
    msg!("Pool pause request cancelled successfully");
    Ok(())
}

/// Process setting pool pause wait time for a specific delegate.
///
/// This function allows the pool owner to configure delegate-specific wait times for
/// pool pause requests. The wait time is the delay between when a pause is requested
/// and when it becomes active, providing deliberation time for dispute resolution.
///
/// # Purpose
/// - Configures delegate-specific governance timing parameters
/// - Enables fine-tuned control over pause activation delays
/// - Supports different trust levels for different delegates
/// - Provides flexibility for various governance models
/// - Allows optimization of response times for different use cases
///
/// # How it works
/// 1. **Authorization**: Verifies caller is pool owner and signed transaction
/// 2. **Delegate Validation**: Ensures target delegate exists in authorized list
/// 3. **Parameter Validation**: Validates wait time is within allowed range
/// 4. **Configuration Update**: Updates delegate's pause wait time setting
/// 5. **State Persistence**: Saves updated configuration to pool state
/// 6. **Audit Logging**: Logs configuration change for transparency
///
/// # Arguments
/// * `program_id` - The program ID for validation (unused but standard pattern)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool owner account (must be signer)
///   - `accounts[1]` - Pool state PDA account (writable for configuration updates)
/// * `delegate` - Public key of the delegate to configure
/// * `wait_time` - Wait time in seconds (60 to 259200 = 1 minute to 72 hours)
///
/// # Account Requirements
/// - Owner: Must be signer and match pool state owner field
/// - Pool state: Must be owned by program and writable for updates
///
/// # Validation Rules
/// - Only pool owner can set delegate pause wait times
/// - Wait time must be between 1 minute and 72 hours
/// - Delegate must exist in authorized delegate list
/// - Setting applies to future pause requests only
///
/// # Default Values
/// - New delegates default to 72 hours wait time (maximum deliberation)
/// - This provides conservative governance approach by default
/// - Can be reduced for trusted delegates or specific use cases
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Owner didn't sign transaction
/// - `ProgramError::InvalidAccountData` - Caller is not pool owner
/// - `PoolError::DelegateNotFound` - Target delegate is not authorized
/// - `PoolError::InvalidWaitTime` - Wait time is outside allowed range
///
/// # Example Usage
/// ```ignore
/// // Set trusted delegate to 1 hour wait time
/// let instruction = PoolInstruction::SetPoolPauseWaitTime {
///     delegate: trusted_delegate_pubkey,
///     wait_time: 3600, // 1 hour
/// };
/// 
/// // Set new delegate to maximum wait time for safety
/// let instruction = PoolInstruction::SetPoolPauseWaitTime {
///     delegate: new_delegate_pubkey,
///     wait_time: 259200, // 72 hours
/// };
/// ```
pub fn process_set_pool_pause_wait_time(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    delegate: Pubkey,
    wait_time: u64,
) -> ProgramResult {
    msg!("Processing SetPoolPauseWaitTime for delegate: {}, wait_time: {} seconds", delegate, wait_time);
    let account_info_iter = &mut accounts.iter();

    let owner = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer to set pool pause wait time");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if *owner.key != pool_state_data.owner {
        msg!("Only pool owner can set pool pause wait times");
        return Err(ProgramError::InvalidAccountData);
    }

    // Set the delegate's pool pause wait time
    pool_state_data.delegate_management.set_pool_pause_wait_time(&delegate, wait_time)?;

    // Save updated state
    pool_state_data.serialize(&mut *pool_state.data.borrow_mut())?;

    // Log the wait time update
    msg!("Pool pause wait time updated: delegate={}, wait_time={} seconds", delegate, wait_time);

    Ok(())
}
