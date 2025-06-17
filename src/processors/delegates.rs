use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::types::PoolState;

/// **DELEGATE MANAGEMENT MODULE**
/// 
/// This module handles all delegate-related operations within the trading pool, including:
/// - Adding and removing authorized delegates
/// - Delegate action management and execution
/// - Time limit configuration for security
/// - Action history tracking and auditing
/// 
/// The delegate system provides secure, time-delayed operations with configurable
/// security policies and comprehensive audit trails.

/// Allows the pool owner to add delegates.
///
/// This function enables the pool owner to authorize up to 3 delegates who can perform
/// various actions in the pool. Each delegate will have configurable time limits
/// for different types of actions.
///
/// # Purpose
/// - Enables delegation of authority to trusted parties
/// - Supports multi-signature-like governance
/// - Allows for separation of pool management duties
/// - Facilitates integration with external systems
///
/// # How it works
/// 1. Verifies the caller is the pool owner (signature required)
/// 2. Checks that the delegate limit (3) hasn't been exceeded
/// 3. Ensures the delegate isn't already authorized
/// 4. Adds the delegate to the authorized list with default time limits
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

/// Allows the pool owner to remove delegates.
///
/// This function enables the pool owner to revoke authority from a delegate.
/// When a delegate is removed, any pending actions they have are automatically
/// cancelled, and they lose access to all pool operations immediately.
///
/// # Purpose
/// - Revokes delegate authority
/// - Provides immediate security response for compromised delegates
/// - Manages delegate lifecycle and permissions
/// - Maintains control over pool access
///
/// # How it works
/// 1. Verifies the caller is the pool owner (signature required)
/// 2. Checks that the delegate exists in the authorized list
/// 3. Removes the delegate and shifts remaining delegates in the array
/// 4. Cancels any pending actions for the removed delegate
/// 5. Updates delegate time limits array accordingly
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

/// Retrieves and logs action history for transparency and auditing.
///
/// This function provides read-only access to the action history, showing the last 10
/// actions performed by delegates. It also displays current delegate information and
/// aggregate statistics. This function is essential for transparency, auditing, and
/// monitoring of delegate activities.
///
/// # Purpose
/// - Provides transparency into delegate activities
/// - Enables auditing of delegate actions
/// - Shows current delegate authorization status
/// - Displays aggregate statistics
/// - Supports monitoring and compliance requirements
///
/// # How it works
/// 1. Loads the pool state to access action history
/// 2. Iterates through the last 10 action records
/// 3. Logs each action with delegate, type, and timestamp
/// 4. Shows current authorized delegates and their count
/// 5. All information is logged to the transaction logs for transparency
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
/// - **Action History**: Last 10 actions with full details
/// - **Delegate Info**: Public key of each action's delegate
/// - **Action Type**: Type of action performed
/// - **Timestamp**: Unix timestamp of the action
/// - **Current Delegates**: List of all currently authorized delegates
///
/// # Errors
/// - `ProgramError::InvalidAccountData` - Pool state account data is corrupted
pub fn process_get_action_history(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing GetActionHistory");
    let account_info_iter = &mut accounts.iter();

    let pool_state = next_account_info(account_info_iter)?;

    // Load pool state
    let pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;

    // Log action history for transparency
    msg!("Action History (last 10 actions):");
    for (i, action) in pool_state_data.delegate_management.action_history.iter().enumerate() {
        msg!("Record {}: Delegate: {}, Action Type: {:?}, Action ID: {}, Timestamp: {}", 
             i, action.delegate, action.action_type, action.action_id, action.request_timestamp);
    }

    msg!("Current delegates ({}):", pool_state_data.delegate_management.delegate_count);
    for i in 0..pool_state_data.delegate_management.delegate_count as usize {
        msg!("Delegate {}: {}", i, pool_state_data.delegate_management.delegates[i]);
    }

    Ok(())
} 