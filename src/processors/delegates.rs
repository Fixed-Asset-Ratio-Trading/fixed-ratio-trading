use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
    clock::Clock,
};
use spl_token::{
    instruction as token_instruction,
};

use crate::constants::*;
use crate::types::*;
use crate::utils::*;
use crate::check_rent_exempt;

/// **DELEGATE MANAGEMENT MODULE**
/// 
/// This module handles all delegate-related operations within the trading pool, including:
/// - Adding and removing authorized delegates
/// - Two-step fee withdrawal process with time delays
/// - Withdrawal request management and execution
/// - Wait time configuration for security
/// - Withdrawal history tracking and auditing
/// 
/// The delegate system provides secure, time-delayed fee distribution with configurable
/// security policies and comprehensive audit trails.

/// Allows the pool owner to add delegates for fee withdrawals.
///
/// This function enables the pool owner to authorize up to 3 delegates who can withdraw
/// trading fees collected by the contract. Each delegate will have configurable wait times
/// for withdrawal requests and can withdraw both SOL and SPL token fees.
///
/// # Purpose
/// - Enables delegation of fee withdrawal authority to trusted parties
/// - Supports multi-signature-like governance for fee management
/// - Allows for separation of pool management and fee collection duties
/// - Facilitates integration with external reward distribution systems
///
/// # How it works
/// 1. Verifies the caller is the pool owner (signature required)
/// 2. Checks that the delegate limit (3) hasn't been exceeded
/// 3. Ensures the delegate isn't already authorized
/// 4. Adds the delegate to the authorized list with default wait time (5 minutes)
/// 5. Updates the pool state and logs the operation
///
/// # Arguments
/// * `_program_id` - The program ID of the contract (not used in validation)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool owner account (must be signer)
///   - `accounts[1]` - Pool state PDA account (writable)
/// * `delegate` - The public key of the delegate to add
///
/// # Account Requirements
/// - Pool owner: Must be signer and match the pool's owner field
/// - Pool state: Must be owned by the program and writable
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Owner didn't sign the transaction
/// - `ProgramError::InvalidAccountData` - Caller is not the pool owner
/// - `PoolError::DelegateLimitExceeded` - Already have 3 delegates
/// - `PoolError::DelegateAlreadyExists` - Delegate is already authorized
///
/// # Example Usage
/// ```ignore
/// // Add a delegate for automated fee collection
/// let instruction = PoolInstruction::AddDelegate {
///     delegate: reward_distributor_pubkey,
/// };
/// ```
pub fn process_add_delegate(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    delegate: Pubkey,
) -> ProgramResult {
    msg!("Processing AddDelegate for: {}", delegate);
    let account_info_iter = &mut accounts.iter();

    let owner = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer to add delegate");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if *owner.key != pool_state_data.owner {
        msg!("Only pool owner can add delegates");
        return Err(ProgramError::InvalidAccountData);
    }

    // Add the delegate
    pool_state_data.delegate_management.add_delegate(delegate)?;
    
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
    
    // Log the change for transparency
    msg!("Delegate added successfully: {}. Total delegates: {}", 
         delegate, pool_state_data.delegate_management.delegate_count);

    Ok(())
}

/// Allows the pool owner to remove delegates from fee withdrawal authorization.
///
/// This function enables the pool owner to revoke fee withdrawal authority from a delegate.
/// When a delegate is removed, any pending withdrawal requests they have are automatically
/// cancelled, and they lose access to withdraw fees immediately.
///
/// # Purpose
/// - Revokes fee withdrawal authority from delegates
/// - Provides immediate security response for compromised delegates
/// - Manages delegate lifecycle and permissions
/// - Maintains control over fee distribution access
///
/// # How it works
/// 1. Verifies the caller is the pool owner (signature required)
/// 2. Checks that the delegate exists in the authorized list
/// 3. Removes the delegate and shifts remaining delegates in the array
/// 4. Cancels any pending withdrawal requests for the removed delegate
/// 5. Updates delegate wait times array accordingly
/// 6. Updates the pool state and logs the operation
///
/// # Arguments
/// * `_program_id` - The program ID of the contract (not used in validation)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool owner account (must be signer)
///   - `accounts[1]` - Pool state PDA account (writable)
/// * `delegate` - The public key of the delegate to remove
///
/// # Account Requirements
/// - Pool owner: Must be signer and match the pool's owner field
/// - Pool state: Must be owned by the program and writable
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Owner didn't sign the transaction
/// - `ProgramError::InvalidAccountData` - Caller is not the pool owner
/// - `PoolError::DelegateNotFound` - Delegate is not in the authorized list
///
/// # Example Usage
/// ```ignore
/// // Remove a compromised delegate
/// let instruction = PoolInstruction::RemoveDelegate {
///     delegate: compromised_delegate_pubkey,
/// };
/// ```
pub fn process_remove_delegate(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    delegate: Pubkey,
) -> ProgramResult {
    msg!("Processing RemoveDelegate for: {}", delegate);
    let account_info_iter = &mut accounts.iter();

    let owner = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer to remove delegate");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if *owner.key != pool_state_data.owner {
        msg!("Only pool owner can remove delegates");
        return Err(ProgramError::InvalidAccountData);
    }

    // Remove the delegate
    pool_state_data.delegate_management.remove_delegate(delegate)?;
    
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
    
    // Log the change for transparency
    msg!("Delegate removed successfully: {}. Remaining delegates: {}", 
         delegate, pool_state_data.delegate_management.delegate_count);

    Ok(())
}

/// Executes fee withdrawals for authorized delegates (Step 2 of two-step process).
///
/// This function allows authorized delegates to execute previously requested fee withdrawals
/// after the required wait time has elapsed. It supports withdrawing both SOL and SPL token
/// fees collected from trading activities. This is the second step of a two-step withdrawal
/// process that enhances security through time-delayed execution.
///
/// # Purpose
/// - Executes time-delayed fee withdrawals for delegates
/// - Supports both SOL and SPL token fee withdrawals
/// - Maintains audit trail of all withdrawal activities
/// - Ensures rent-exempt status is preserved during SOL withdrawals
/// - Provides secure fee distribution mechanism
///
/// # How it works
/// 1. Verifies the delegate is authorized and signed the transaction
/// 2. Checks that the pool is not paused
/// 3. Validates that a withdrawal request exists and wait time has elapsed
/// 4. Confirms the withdrawal request matches the current parameters
/// 5. For SOL withdrawals:
///    - Verifies sufficient collected SOL fees
///    - Ensures pool maintains rent-exempt status
///    - Transfers SOL directly from pool state PDA to delegate
/// 6. For SPL token withdrawals:
///    - Validates token vault and delegate token accounts
///    - Transfers tokens from vault to delegate's token account
/// 7. Updates fee tracking counters and withdrawal history
/// 8. Clears the withdrawal request to allow new requests
///
/// # Arguments
/// * `program_id` - The program ID for PDA validation and CPI authority
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Delegate account (must be signer and authorized)
///   - `accounts[1]` - Pool state PDA account (writable)
///   - `accounts[2]` - System program (for SOL transfers)
///   - `accounts[3]` - Rent sysvar (for rent calculations)
///   - `accounts[4]` - Clock sysvar (for timestamp validation)
///   - For SPL token withdrawals only:
///     - `accounts[5]` - Token vault account (writable)
///     - `accounts[6]` - Delegate's token account (writable)
///     - `accounts[7]` - Token program
/// * `token_mint` - The mint of the token to withdraw (use Pubkey::default() for SOL)
/// * `amount` - The amount to withdraw (in lamports for SOL, token units for SPL)
///
/// # Account Requirements
/// - Delegate: Must be signer and in the authorized delegates list
/// - Pool state: Must be owned by the program and writable
/// - For SOL: Must maintain rent-exempt balance after withdrawal
/// - For SPL tokens: Token accounts must match the expected mint and owner
///
/// # Errors
/// - `ProgramError::MissingRequiredSignature` - Delegate didn't sign
/// - `PoolError::PoolPaused` - Pool operations are paused
/// - `PoolError::UnauthorizedDelegate` - Caller is not an authorized delegate
/// - `PoolError::WithdrawalNotReady` - Wait time hasn't elapsed
/// - `PoolError::NoPendingWithdrawal` - No withdrawal request exists
/// - `PoolError::InvalidWithdrawalRequest` - Request doesn't match parameters
/// - `ProgramError::InsufficientFunds` - Not enough fees collected or SOL balance
/// - `ProgramError::InvalidAccountData` - Invalid token vault or accounts
///
/// # Example Usage
/// ```ignore
/// // Execute SOL fee withdrawal (after wait time)
/// let instruction = PoolInstruction::WithdrawFeesToDelegate {
///     token_mint: Pubkey::default(), // SOL
///     amount: 1_000_000, // 0.001 SOL
/// };
///
/// // Execute SPL token fee withdrawal
/// let instruction = PoolInstruction::WithdrawFeesToDelegate {
///     token_mint: usdc_mint_pubkey,
///     amount: 1_000_000, // 1 USDC (6 decimals)
/// };
/// ```
pub fn process_withdraw_fees_to_delegate(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    token_mint: Pubkey,
    amount: u64,
) -> ProgramResult {
    msg!("Processing WithdrawFeesToDelegate for token: {}, amount: {}", token_mint, amount);
    let account_info_iter = &mut accounts.iter();

    let delegate = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let rent_sysvar = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;

    // Verify delegate is signer
    if !delegate.is_signer {
        msg!("Delegate must be a signer for fee withdrawal");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    
    // Verify pool is not paused
    if pool_state_data.is_paused {
        msg!("Fee withdrawals are paused");
        return Err(PoolError::PoolPaused.into());
    }

    // Verify caller is a delegate
    if !pool_state_data.delegate_management.is_delegate(delegate.key) {
        msg!("Caller is not an authorized delegate: {}", delegate.key);
        return Err(PoolError::UnauthorizedDelegate.into());
    }

    // Two-step withdrawal verification
    // Check if withdrawal request exists and is ready
    if !pool_state_data.delegate_management.is_withdrawal_ready(delegate.key, clock.unix_timestamp)? {
        msg!("Withdrawal not ready for delegate: {}", delegate.key);
        return Err(PoolError::WithdrawalNotReady.into());
    }

    // Get withdrawal request
    let request = pool_state_data.delegate_management.get_withdrawal_request(delegate.key)
        .ok_or(PoolError::NoPendingWithdrawal)?;

    // Verify request matches current withdrawal
    if request.token_mint != token_mint || request.amount != amount {
        msg!("Withdrawal request mismatch: requested token={}, amount={}, actual token={}, amount={}", 
             request.token_mint, request.amount, token_mint, amount);
        return Err(PoolError::InvalidWithdrawalRequest.into());
    }

    // Handle SOL withdrawal
    if token_mint == Pubkey::default() {
        // Check if enough SOL fees collected
        if amount > pool_state_data.collected_sol_fees {
            msg!("Insufficient collected SOL fees. Available: {}, Requested: {}", 
                 pool_state_data.collected_sol_fees, amount);
            return Err(ProgramError::InsufficientFunds);
        }

        // Check rent exempt requirements
        let rent = &Rent::from_account_info(rent_sysvar)?;
        check_rent_exempt(pool_state, program_id, rent, clock.slot)?;

        // Calculate minimum balance to maintain rent exemption
        let minimum_balance = rent.minimum_balance(pool_state.data_len());
        if pool_state.lamports() < amount + minimum_balance {
            msg!("Insufficient SOL balance. Required: {}, Available: {}", 
                 amount + minimum_balance, pool_state.lamports());
            return Err(ProgramError::InsufficientFunds);
        }

        // Transfer SOL to delegate
        let pool_state_pda_seeds = &[
            POOL_STATE_SEED_PREFIX,
            pool_state_data.token_a_mint.as_ref(),
            pool_state_data.token_b_mint.as_ref(),
            &pool_state_data.ratio_a_numerator.to_le_bytes(),
            &pool_state_data.ratio_b_denominator.to_le_bytes(),
            &[pool_state_data.pool_authority_bump_seed],
        ];

        invoke_signed(
            &system_instruction::transfer(pool_state.key, delegate.key, amount),
            &[pool_state.clone(), delegate.clone(), system_program.clone()],
            &[pool_state_pda_seeds],
        )?;

        // Update pool state
        pool_state_data.collected_sol_fees = pool_state_data.collected_sol_fees
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        pool_state_data.total_sol_fees_withdrawn = pool_state_data.total_sol_fees_withdrawn
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        // Add withdrawal record
        let withdrawal_record = WithdrawalRecord::new(
            *delegate.key,
            token_mint,
            amount,
            clock.unix_timestamp,
            clock.slot,
        );
        pool_state_data.delegate_management.add_withdrawal_record(withdrawal_record);

        // Clear withdrawal request after successful withdrawal
        pool_state_data.delegate_management.cancel_withdrawal_request(delegate.key)?;

        // Save updated state
        pool_state_data.serialize(&mut *pool_state.data.borrow_mut())?;

        // Log the withdrawal for transparency
        msg!("SOL fee withdrawal completed: Delegate: {}, Amount: {}, Timestamp: {}", 
             delegate.key, amount, clock.unix_timestamp);

        return Ok(());
    }

    // Handle SPL token withdrawal
    let token_vault = next_account_info(account_info_iter)?;
    let delegate_token_account = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    // Determine token index (0 for token_a, 1 for token_b)
    let (token_index, vault_key, collected_fees) = if token_mint == pool_state_data.token_a_mint {
        (0, pool_state_data.token_a_vault, pool_state_data.collected_fees_token_a)
    } else if token_mint == pool_state_data.token_b_mint {
        (1, pool_state_data.token_b_vault, pool_state_data.collected_fees_token_b)
    } else {
        msg!("Invalid token mint for withdrawal: {}", token_mint);
        return Err(ProgramError::InvalidArgument);
    };

    // Verify vault account
    if *token_vault.key != vault_key {
        msg!("Invalid token vault provided");
        return Err(ProgramError::InvalidAccountData);
    }

    // Check if enough fees collected
    if amount > collected_fees {
        msg!("Insufficient collected fees. Available: {}, Requested: {}", collected_fees, amount);
        return Err(ProgramError::InsufficientFunds);
    }

    // Check rent exempt requirements
    let rent = &Rent::from_account_info(rent_sysvar)?;
    check_rent_exempt(pool_state, program_id, rent, clock.slot)?;

    // Transfer fees to delegate
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.token_a_mint.as_ref(),
        pool_state_data.token_b_mint.as_ref(),
        &pool_state_data.ratio_a_numerator.to_le_bytes(),
        &pool_state_data.ratio_b_denominator.to_le_bytes(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    invoke_signed(
        &token_instruction::transfer(
            token_program.key,
            token_vault.key,
            delegate_token_account.key,
            pool_state.key,
            &[],
            amount,
        )?,
        &[
            token_vault.clone(),
            delegate_token_account.clone(),
            pool_state.clone(),
            token_program.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;

    // Update pool state
    if token_index == 0 {
        pool_state_data.collected_fees_token_a = pool_state_data.collected_fees_token_a
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        pool_state_data.total_fees_withdrawn_token_a = pool_state_data.total_fees_withdrawn_token_a
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    } else {
        pool_state_data.collected_fees_token_b = pool_state_data.collected_fees_token_b
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        pool_state_data.total_fees_withdrawn_token_b = pool_state_data.total_fees_withdrawn_token_b
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }

    // Add withdrawal record
    let withdrawal_record = WithdrawalRecord::new(
        *delegate.key,
        token_mint,
        amount,
        clock.unix_timestamp,
        clock.slot,
    );
    pool_state_data.delegate_management.add_withdrawal_record(withdrawal_record);

    // Clear withdrawal request after successful withdrawal
    pool_state_data.delegate_management.cancel_withdrawal_request(delegate.key)?;

    // Save updated state
    pool_state_data.serialize(&mut *pool_state.data.borrow_mut())?;

    // Log the withdrawal for transparency
    msg!("Fee withdrawal completed: Delegate: {}, Token: {}, Amount: {}, Timestamp: {}", 
         delegate.key, token_mint, amount, clock.unix_timestamp);

    Ok(())
}

/// Retrieves and logs withdrawal history for transparency and auditing.
///
/// This function provides read-only access to the withdrawal history, showing the last 10
/// fee withdrawals made by delegates. It also displays current delegate information and
/// aggregate fee withdrawal statistics. This function is essential for transparency,
/// auditing, and monitoring of fee distribution activities.
///
/// # Purpose
/// - Provides transparency into fee withdrawal activities
/// - Enables auditing of delegate fee withdrawals
/// - Shows current delegate authorization status
/// - Displays aggregate withdrawal statistics
/// - Supports monitoring and compliance requirements
///
/// # How it works
/// 1. Loads the pool state to access withdrawal history
/// 2. Iterates through the last 10 withdrawal records
/// 3. Logs each withdrawal with delegate, token, amount, and timestamp
/// 4. Displays total fees withdrawn by token type
/// 5. Shows current authorized delegates and their count
/// 6. All information is logged to the transaction logs for transparency
///
/// # Arguments
/// * `_program_id` - The program ID of the contract (not used for validation)
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool state PDA account (read-only)
///
/// # Account Requirements
/// - Pool state: Must be readable (no signature or write access required)
///
/// # Information Displayed
/// - **Withdrawal History**: Last 10 withdrawals with full details
/// - **Delegate Info**: Public key of each withdrawal's delegate
/// - **Token Info**: Token mint address (Pubkey::default() for SOL)
/// - **Amount**: Withdrawal amount in token-specific units
/// - **Timestamp**: Unix timestamp of the withdrawal
/// - **Slot**: Solana slot number when withdrawal occurred
/// - **Aggregate Stats**: Total fees withdrawn per token type
/// - **Current Delegates**: List of all currently authorized delegates
///
/// # Errors
/// - `ProgramError::InvalidAccountData` - Pool state account data is corrupted
///
/// # Example Usage
/// ```ignore
/// // Query withdrawal history for auditing
/// let instruction = PoolInstruction::GetWithdrawalHistory;
/// 
/// // Results logged to transaction logs:
/// // "Withdrawal History (last 10 withdrawals):"
/// // "Record 0: Delegate: ABC..., Token: DEF..., Amount: 1000000, Timestamp: 1234567890, Slot: 98765"
/// // "Total fees withdrawn - Token A: 5000000, Token B: 3000000"
/// // "Current delegates (3): GHI..., JKL..., MNO..."
/// ```
///
/// # Use Cases
/// - **Auditing**: Review all recent fee withdrawals
/// - **Monitoring**: Track delegate withdrawal patterns
/// - **Compliance**: Verify fee distribution activities
/// - **Analytics**: Analyze fee collection and distribution
/// - **Debugging**: Investigate withdrawal-related issues
pub fn process_get_withdrawal_history(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing GetWithdrawalHistory");
    let account_info_iter = &mut accounts.iter();

    let pool_state = next_account_info(account_info_iter)?;

    // Load pool state
    let pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;

    // Log withdrawal history for transparency
    msg!("Withdrawal History (last 10 withdrawals):");
    for (i, record) in pool_state_data.delegate_management.withdrawal_history.iter().enumerate() {
        if record.delegate != Pubkey::default() {
            msg!("Record {}: Delegate: {}, Token: {}, Amount: {}, Timestamp: {}, Slot: {}", 
                 i, record.delegate, record.token_mint, record.amount, record.timestamp, record.slot);
        }
    }

    msg!("Total fees withdrawn - Token A: {}, Token B: {}", 
         pool_state_data.total_fees_withdrawn_token_a,
         pool_state_data.total_fees_withdrawn_token_b);

    msg!("Current delegates ({}):", pool_state_data.delegate_management.delegate_count);
    for i in 0..pool_state_data.delegate_management.delegate_count as usize {
        msg!("Delegate {}: {}", i, pool_state_data.delegate_management.delegates[i]);
    }

    Ok(())
}

/// Creates a fee withdrawal request for authorized delegates (Step 1 of two-step process).
///
/// This function allows authorized delegates to request fee withdrawals with a time delay
/// for enhanced security. Delegates must specify the token type and amount they wish to
/// withdraw. Each delegate can have only one active withdrawal request at a time, and the
/// request must wait for a configurable period (5 minutes to 72 hours) before execution.
///
/// # Purpose
/// - Initiates the two-step withdrawal process for enhanced security
/// - Allows delegates to request both SOL and SPL token fee withdrawals
/// - Implements time-delayed execution to prevent immediate unauthorized access
/// - Provides transparency through logged withdrawal requests
/// - Prevents multiple concurrent requests per delegate
///
/// # How it works
/// 1. Verifies the delegate is authorized and signed the transaction
/// 2. Checks that the pool is not paused
/// 3. Ensures the delegate doesn't have a pending withdrawal request
/// 4. Creates a withdrawal request with current timestamp and delegate's wait time
/// 5. Stores the request in the pool state for later execution
/// 6. Logs the request details for transparency
///
/// # Arguments
/// * `program_id` - The program ID for account ownership validation
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool state PDA account (writable)
///   - `accounts[1]` - Delegate account (must be signer and authorized)
///   - `accounts[2]` - Clock sysvar (for timestamp)
/// * `token_mint` - The mint of the token to withdraw (use Pubkey::default() for SOL)
/// * `amount` - The amount to withdraw (in lamports for SOL, token units for SPL)
///
/// # Account Requirements
/// - Pool state: Must be owned by the program and writable
/// - Delegate: Must be signer and in the authorized delegates list
/// - Clock: System clock sysvar for timestamp validation
///
/// # Errors
/// - `ProgramError::IncorrectProgramId` - Pool state not owned by program
/// - `ProgramError::MissingRequiredSignature` - Delegate didn't sign
/// - `PoolError::PoolPaused` - Pool operations are paused
/// - `PoolError::UnauthorizedDelegate` - Caller is not an authorized delegate
/// - `PoolError::PendingWithdrawalExists` - Delegate already has a pending request
///
/// # Example Usage
/// ```ignore
/// // Request SOL fee withdrawal
/// let instruction = PoolInstruction::RequestFeeWithdrawal {
///     token_mint: Pubkey::default(), // SOL
///     amount: 1_000_000, // 0.001 SOL
/// };
///
/// // Request SPL token fee withdrawal
/// let instruction = PoolInstruction::RequestFeeWithdrawal {
///     token_mint: usdc_mint_pubkey,
///     amount: 1_000_000, // 1 USDC (6 decimals)
/// };
/// ```
///
/// # Security Features
/// - **Time Delay**: Configurable wait time prevents immediate execution
/// - **Single Request**: Only one active request per delegate
/// - **Authorization**: Only authorized delegates can create requests
/// - **Pause Protection**: Requests blocked when pool is paused
/// - **Audit Trail**: All requests logged for transparency
pub fn process_request_fee_withdrawal(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    token_mint: Pubkey,
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_state_info = next_account_info(account_info_iter)?;
    let delegate_info = next_account_info(account_info_iter)?;
    let clock_info = next_account_info(account_info_iter)?;

    // Verify pool state account
    if pool_state_info.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Verify delegate is signer
    if !delegate_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load pool state
    let mut pool_state = PoolState::try_from_slice(&pool_state_info.data.borrow())?;

    // Check if pool is paused
    if pool_state.is_paused {
        return Err(PoolError::PoolPaused.into());
    }

    // Verify delegate is authorized
    if !pool_state.delegate_management.is_delegate(delegate_info.key) {
        return Err(PoolError::UnauthorizedDelegate.into());
    }

    // Validate token mint is one of the pool's valid tokens (or SOL for fee withdrawals)
    if token_mint != pool_state.token_a_mint && token_mint != pool_state.token_b_mint && token_mint != Pubkey::default() {
        msg!("Invalid token mint for withdrawal: {}. Valid mints: {}, {}, SOL (default)", 
             token_mint, pool_state.token_a_mint, pool_state.token_b_mint);
        return Err(ProgramError::InvalidArgument);
    }

    // Get current timestamp
    let clock = Clock::from_account_info(clock_info)?;
    let current_timestamp = clock.unix_timestamp;

    // Create withdrawal request
    pool_state.delegate_management.create_withdrawal_request(
        delegate_info.key,
        token_mint,
        amount,
        current_timestamp,
        clock.slot,
    )?;

    // Save updated pool state using buffer serialization approach
    let mut serialized_data = Vec::new();
    pool_state.serialize(&mut serialized_data)?;
    let account_data_len = pool_state_info.data_len();
    if serialized_data.len() > account_data_len {
        return Err(ProgramError::AccountDataTooSmall);
    }
    {
        let mut account_data = pool_state_info.data.borrow_mut();
        account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
    }

    // Log the withdrawal request
    msg!("Withdrawal requested: delegate={}, token_mint={}, amount={}, timestamp={}", 
         delegate_info.key, token_mint, amount, current_timestamp);

    Ok(())
}

/// Cancels a pending fee withdrawal request.
///
/// This function allows either the pool owner or the requesting delegate to cancel a
/// pending withdrawal request before it becomes executable. This provides flexibility
/// for delegates to change their minds and emergency intervention capability for the
/// pool owner in case of security concerns.
///
/// # Purpose
/// - Provides flexibility for delegates to cancel their own requests
/// - Enables pool owner emergency intervention for security
/// - Allows correction of erroneous withdrawal requests
/// - Resets delegate status to allow new withdrawal requests
/// - Maintains control and security over the withdrawal process
///
/// # How it works
/// 1. Verifies the caller is either the pool owner or the requesting delegate
/// 2. Checks that the pool is not paused (for normal operations)
/// 3. Clears the withdrawal request from the delegate's slot
/// 4. Allows the delegate to create a new withdrawal request immediately
/// 5. Logs the cancellation details for transparency
///
/// # Arguments
/// * `program_id` - The program ID for account ownership validation
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool state PDA account (writable)
///   - `accounts[1]` - Canceler account (must be signer - owner or delegate)
///   - `accounts[2]` - Delegate account (whose request is being cancelled)
///
/// # Account Requirements
/// - Pool state: Must be owned by the program and writable
/// - Canceler: Must be signer and either the pool owner or the delegate
/// - Delegate: The account whose withdrawal request is being cancelled
///
/// # Authorization Rules
/// - **Pool Owner**: Can cancel any delegate's withdrawal request
/// - **Delegate**: Can only cancel their own withdrawal request
/// - **Others**: Cannot cancel withdrawal requests
///
/// # Errors
/// - `ProgramError::IncorrectProgramId` - Pool state not owned by program
/// - `ProgramError::MissingRequiredSignature` - Canceler didn't sign
/// - `PoolError::PoolPaused` - Pool operations are paused
/// - `PoolError::Unauthorized` - Caller is neither owner nor the delegate
/// - `PoolError::DelegateNotFound` - Delegate is not in authorized list
///
/// # Example Usage
/// ```ignore
/// // Delegate cancels their own request
/// let instruction = PoolInstruction::CancelWithdrawalRequest;
/// // Accounts: [pool_state, delegate_signer, delegate_signer]
///
/// // Owner cancels any delegate's request (emergency)
/// let instruction = PoolInstruction::CancelWithdrawalRequest;
/// // Accounts: [pool_state, owner_signer, target_delegate]
/// ```
///
/// # Use Cases
/// - **Self-Cancellation**: Delegate changes mind about withdrawal
/// - **Error Correction**: Fix incorrect amount or token type
/// - **Security Response**: Owner cancels suspicious requests
/// - **Emergency Control**: Immediate intervention capability
/// - **Process Reset**: Clear state to allow new requests
pub fn process_cancel_withdrawal_request(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_state_info = next_account_info(account_info_iter)?;
    let canceler_info = next_account_info(account_info_iter)?;
    let delegate_info = next_account_info(account_info_iter)?;

    // Verify pool state account
    if pool_state_info.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Verify canceler is signer
    if !canceler_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load pool state
    let mut pool_state = PoolState::try_from_slice(&pool_state_info.data.borrow())?;

    // Check if pool is paused
    if pool_state.is_paused {
        return Err(PoolError::PoolPaused.into());
    }

    // Verify canceler is either the owner or the delegate
    if *canceler_info.key != pool_state.owner && *canceler_info.key != *delegate_info.key {
        return Err(PoolError::Unauthorized.into());
    }

    // Cancel withdrawal request
    pool_state.delegate_management.cancel_withdrawal_request(delegate_info.key)?;

    // Save updated pool state
    pool_state.serialize(&mut *pool_state_info.data.borrow_mut())?;

    // Log the cancellation
    msg!("Withdrawal request cancelled: delegate={}, cancelled_by={}", 
         delegate_info.key, canceler_info.key);

    Ok(())
}

/// Sets the withdrawal wait time for a specific delegate.
///
/// This function allows the pool owner to configure individual wait times for each
/// delegate, providing fine-grained control over the security level for different
/// delegates. Wait times can range from 5 minutes to 72 hours, allowing for flexible
/// security policies based on delegate trust levels and roles.
///
/// # Purpose
/// - Configures individual security policies for each delegate
/// - Allows differentiated trust levels based on delegate roles
/// - Provides dynamic security adjustment capabilities
/// - Enables risk-based withdrawal controls
/// - Supports governance and security best practices
///
/// # How it works
/// 1. Verifies the caller is the pool owner (signature required)
/// 2. Validates the wait time is within allowed bounds (5 min - 72 hours)
/// 3. Confirms the target is an authorized delegate
/// 4. Updates the delegate's wait time in the pool state
/// 5. Logs the change for transparency and auditing
///
/// # Arguments
/// * `program_id` - The program ID for account ownership validation
/// * `accounts` - Array of account infos in the following order:
///   - `accounts[0]` - Pool state PDA account (writable)
///   - `accounts[1]` - Pool owner account (must be signer)
/// * `delegate` - The public key of the delegate whose wait time is being set
/// * `wait_time` - The wait time in seconds (300 to 259,200 seconds)
///
/// # Account Requirements
/// - Pool state: Must be owned by the program and writable
/// - Owner: Must be signer and match the pool's owner field
///
/// # Wait Time Constraints
/// - **Minimum**: 300 seconds (5 minutes)
/// - **Maximum**: 259,200 seconds (72 hours)
/// - **Default**: 300 seconds (applied when delegate is first added)
/// - **Granularity**: 1 second
///
/// # Errors
/// - `ProgramError::IncorrectProgramId` - Pool state not owned by program
/// - `ProgramError::MissingRequiredSignature` - Owner didn't sign
/// - `PoolError::Unauthorized` - Caller is not the pool owner
/// - `PoolError::DelegateNotFound` - Target is not an authorized delegate
/// - `PoolError::InvalidWaitTime` - Wait time outside allowed range
///
/// # Example Usage
/// ```ignore
/// // Set short wait time for trusted delegate (5 minutes)
/// let instruction = PoolInstruction::SetDelegateWaitTime {
///     delegate: trusted_delegate_pubkey,
///     wait_time: 300, // 5 minutes
/// };
///
/// // Set longer wait time for less trusted delegate (24 hours)
/// let instruction = PoolInstruction::SetDelegateWaitTime {
///     delegate: external_delegate_pubkey,
///     wait_time: 86400, // 24 hours
/// };
///
/// // Set maximum wait time for high-security scenarios (72 hours)
/// let instruction = PoolInstruction::SetDelegateWaitTime {
///     delegate: high_security_delegate_pubkey,
///     wait_time: 259200, // 72 hours
/// };
/// ```
///
/// # Security Considerations
/// - **Risk-Based**: Higher wait times for higher-risk delegates
/// - **Role-Based**: Different wait times for different delegate roles
/// - **Dynamic**: Can be adjusted based on changing security needs
/// - **Immediate Effect**: New wait time applies to future requests
/// - **Existing Requests**: Pending requests use their original wait time
///
/// # Common Wait Time Strategies
/// - **Automated Systems**: 5-15 minutes for trusted automated processes
/// - **Trusted Partners**: 1-6 hours for known and trusted entities
/// - **External Delegates**: 12-24 hours for external or less trusted delegates
/// - **High-Value Operations**: 48-72 hours for maximum security scenarios
pub fn process_set_delegate_wait_time(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    delegate: Pubkey,
    wait_time: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_state_info = next_account_info(account_info_iter)?;
    let owner_info = next_account_info(account_info_iter)?;

    // Verify pool state account
    if pool_state_info.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Verify owner is signer
    if !owner_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load pool state
    let mut pool_state = PoolState::try_from_slice(&pool_state_info.data.borrow())?;

    // Verify caller is owner
    if *owner_info.key != pool_state.owner {
        return Err(PoolError::Unauthorized.into());
    }

    // Set delegate wait time
    pool_state.delegate_management.set_delegate_wait_time(&delegate, wait_time)?;

    // Save updated pool state
    pool_state.serialize(&mut *pool_state_info.data.borrow_mut())?;

    // Log the wait time update
    msg!("Delegate wait time updated: delegate={}, wait_time={}", delegate, wait_time);

    Ok(())
}

// ================================================================================================
// POOL PAUSE GOVERNANCE PROCESSORS (Part of Delegate/Governance Management)
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

    // Save updated state using buffer serialization approach
    serialize_to_account(&pool_state_data, pool_state_account)?;

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

    // Save updated state using buffer serialization approach
    serialize_to_account(&pool_state_data, pool_state_account)?;
    
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

    // Save updated state using buffer serialization approach
    serialize_to_account(&pool_state_data, pool_state)?;

    // Log the wait time update
    msg!("Pool pause wait time updated: delegate={}, wait_time={} seconds", delegate, wait_time);

    Ok(())
} 