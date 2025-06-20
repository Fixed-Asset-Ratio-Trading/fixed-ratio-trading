use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    clock::Clock,
    sysvar::Sysvar,
    program::invoke_signed,
};

use spl_token;

use crate::{
    error::PoolError,
    types::{
        PoolState,
        pool_state::PoolPauseReason,
        delegate_actions::{
            DelegateActionType,
            DelegateActionParams,
            DelegateTimeLimits,
            PendingDelegateAction,
            PauseReason,
        },
    },
    constants::{MAX_SWAP_FEE_BASIS_POINTS, POOL_STATE_SEED_PREFIX},
};

/// Process a request for a delegate action
pub fn process_request_delegate_action(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    action_type: DelegateActionType,
    params: DelegateActionParams,
) -> ProgramResult {
    msg!("Processing RequestDelegateAction: {:?}", action_type);
    let account_info_iter = &mut accounts.iter();

    // ✅ CRITICAL: System pause validation (takes precedence over pool pause)
    let system_state_account = next_account_info(account_info_iter)?;
    crate::utils::validation::validate_system_not_paused(system_state_account)?;

    let delegate_account = next_account_info(account_info_iter)?;
    let pool_state_account = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;

    // Verify delegate is signer
    if !delegate_account.is_signer {
        msg!("Delegate must be a signer to request action");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load pool state
    let mut pool_state = PoolState::deserialize(&mut &pool_state_account.data.borrow()[..])?;

    // Verify delegate is authorized
    if !pool_state.delegate_management.is_delegate(delegate_account.key) {
        msg!("Caller is not an authorized delegate: {}", delegate_account.key);
        return Err(PoolError::UnauthorizedDelegate.into());
    }

    // Get delegate's time limits
    let time_limits = pool_state.delegate_management.get_delegate_time_limits(delegate_account.key)
        .ok_or(PoolError::DelegateNotFound { delegate: *delegate_account.key })?;

    // Get appropriate wait time based on action type
    let wait_time = match action_type {
        DelegateActionType::FeeChange => time_limits.fee_change_wait_time,
        DelegateActionType::Withdrawal => time_limits.withdraw_wait_time,
        DelegateActionType::PoolPause => time_limits.pause_wait_time,
    };

    // Validate action parameters
    validate_action_params(&action_type, &params)?;

    // Create the pending action
    let pending_action = PendingDelegateAction::new(
        *delegate_account.key,
        action_type,
        params,
        clock.unix_timestamp,
        wait_time,
        pool_state.delegate_management.next_action_id,
    );

    // Add the action to pending actions
    let action_id = pool_state.delegate_management.add_pending_action(pending_action)?;

    // Save updated state
    let mut serialized_data = Vec::new();
    pool_state.serialize(&mut serialized_data)?;
    pool_state_account.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);

    msg!("Delegate action requested successfully. Action ID: {}", action_id);
    Ok(())
}

/// Process the execution of a pending delegate action
pub fn process_execute_delegate_action(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    action_id: u64,
) -> ProgramResult {
    msg!("Processing ExecuteDelegateAction: {}", action_id);
    let account_info_iter = &mut accounts.iter();

    // ✅ CRITICAL: System pause validation (takes precedence over pool pause)
    let system_state_account = next_account_info(account_info_iter)?;
    crate::utils::validation::validate_system_not_paused(system_state_account)?;

    let executor_account = next_account_info(account_info_iter)?;
    let pool_state_account = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;

    // Verify executor is signer
    if !executor_account.is_signer {
        msg!("Executor must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load pool state
    let mut pool_state = PoolState::deserialize(&mut &pool_state_account.data.borrow()[..])?;

    // Check if pool needs to be automatically unpaused
    if pool_state.is_paused && pool_state.pause_end_timestamp > 0 && clock.unix_timestamp >= pool_state.pause_end_timestamp {
        pool_state.is_paused = false;
        pool_state.pause_end_timestamp = 0;
        pool_state.pause_reason = PoolPauseReason::default();
        msg!("Pool automatically unpaused as pause duration has elapsed");
    }

    // Get the pending action
    let action = pool_state.delegate_management.get_pending_action(action_id)
        .ok_or(PoolError::ActionNotFound)?;

    // Verify action is ready for execution
    if !action.is_executable(clock.unix_timestamp) {
        msg!("Action is not ready for execution");
        return Err(PoolError::ActionNotReady.into());
    }

    // Execute the action based on type
    match action.action_type {
        DelegateActionType::FeeChange => {
            if let DelegateActionParams::FeeChange { new_fee_basis_points } = action.params {
                if new_fee_basis_points > MAX_SWAP_FEE_BASIS_POINTS {
                    return Err(PoolError::InvalidActionParameters.into());
                }
                pool_state.swap_fee_basis_points = new_fee_basis_points;
                msg!("Swap fee updated to {} basis points", new_fee_basis_points);
            }
        },
        DelegateActionType::Withdrawal => {
            if let DelegateActionParams::Withdrawal { token_mint, amount } = action.params {
                // Get the delegate's token account for receiving fees
                let delegate_token_account = next_account_info(account_info_iter)?;
                let token_program = next_account_info(account_info_iter)?;

                // Validate token mint matches pool tokens
                if token_mint != pool_state.token_a_mint && token_mint != pool_state.token_b_mint {
                    msg!("Invalid token mint for fee withdrawal");
                    return Err(PoolError::InvalidActionParameters.into());
                }

                // Check if there are sufficient collected fees
                let (collected_fees, vault_account, total_withdrawn) = if token_mint == pool_state.token_a_mint {
                    (pool_state.collected_fees_token_a, 
                     pool_state.token_a_vault,
                     &mut pool_state.total_fees_withdrawn_token_a)
                } else {
                    (pool_state.collected_fees_token_b,
                     pool_state.token_b_vault,
                     &mut pool_state.total_fees_withdrawn_token_b)
                };

                if amount > collected_fees {
                    msg!("Insufficient collected fees for withdrawal");
                    return Err(PoolError::InvalidActionParameters.into());
                }

                // Get the vault account info
                let vault_account_info = next_account_info(account_info_iter)?;
                if *vault_account_info.key != vault_account {
                    msg!("Invalid vault account provided");
                    return Err(PoolError::InvalidActionParameters.into());
                }

                // Transfer fees from vault to delegate
                let pool_state_seeds = &[
                    POOL_STATE_SEED_PREFIX,
                    pool_state.token_a_mint.as_ref(),
                    pool_state.token_b_mint.as_ref(),
                    &pool_state.ratio_a_numerator.to_le_bytes(),
                    &pool_state.ratio_b_denominator.to_le_bytes(),
                    &[pool_state.pool_authority_bump_seed],
                ];

                invoke_signed(
                    &spl_token::instruction::transfer(
                        token_program.key,
                        vault_account_info.key,
                        delegate_token_account.key,
                        pool_state_account.key,
                        &[],
                        amount,
                    )?,
                    &[
                        vault_account_info.clone(),
                        delegate_token_account.clone(),
                        pool_state_account.clone(),
                        token_program.clone(),
                    ],
                    &[pool_state_seeds],
                )?;

                // Update fee tracking
                if token_mint == pool_state.token_a_mint {
                    pool_state.collected_fees_token_a = pool_state.collected_fees_token_a
                        .checked_sub(amount)
                        .ok_or(ProgramError::ArithmeticOverflow)?;
                } else {
                    pool_state.collected_fees_token_b = pool_state.collected_fees_token_b
                        .checked_sub(amount)
                        .ok_or(ProgramError::ArithmeticOverflow)?;
                }

                // Update total withdrawn amount
                *total_withdrawn = total_withdrawn
                    .checked_add(amount)
                    .ok_or(ProgramError::ArithmeticOverflow)?;

                msg!("Fee withdrawal successful: {} tokens of mint {}", amount, token_mint);
            }
        },
        DelegateActionType::PoolPause => {
            if let DelegateActionParams::PoolPause { duration, reason } = action.params {
                // Set pause state
                pool_state.is_paused = true;
                // Convert PauseReason to PoolPauseReason
                pool_state.pause_reason = match reason {
                    PauseReason::RatioDispute => PoolPauseReason::RatioDispute,
                    PauseReason::SecurityConcern => PoolPauseReason::SecurityConcern,
                    PauseReason::GovernanceAction => PoolPauseReason::GovernanceAction,
                    PauseReason::ManualIntervention => PoolPauseReason::ManualIntervention,
                    PauseReason::Emergency => PoolPauseReason::Emergency,
                };
                
                // Calculate and set pause end timestamp
                pool_state.pause_end_timestamp = clock.unix_timestamp
                    .checked_add(duration as i64)
                    .ok_or(ProgramError::ArithmeticOverflow)?;
                
                msg!("Pool paused for {} seconds until timestamp {}", 
                     duration, pool_state.pause_end_timestamp);
                msg!("Pause reason: {:?}", reason);
            }
        },
    }

    // Remove the executed action and add to history
    pool_state.delegate_management.remove_pending_action(action_id)?;

    // Save updated state
    let mut serialized_data = Vec::new();
    pool_state.serialize(&mut serialized_data)?;
    pool_state_account.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);

    msg!("Delegate action executed successfully");
    Ok(())
}

/// Process the revocation of a pending delegate action
pub fn process_revoke_action(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    action_id: u64,
) -> ProgramResult {
    msg!("Processing RevokeAction: {}", action_id);
    let account_info_iter = &mut accounts.iter();

    // ✅ CRITICAL: System pause validation (takes precedence over pool pause)
    let system_state_account = next_account_info(account_info_iter)?;
    crate::utils::validation::validate_system_not_paused(system_state_account)?;

    let revoker_account = next_account_info(account_info_iter)?;
    let pool_state_account = next_account_info(account_info_iter)?;

    // Verify revoker is signer
    if !revoker_account.is_signer {
        msg!("Revoker must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load pool state
    let mut pool_state = PoolState::deserialize(&mut &pool_state_account.data.borrow()[..])?;

    // Get the pending action
    let action = pool_state.delegate_management.get_pending_action(action_id)
        .ok_or(PoolError::ActionNotFound)?;

    // Verify revoker is either the pool owner or the delegate who created the action
    if *revoker_account.key != pool_state.owner && *revoker_account.key != action.delegate {
        msg!("Unauthorized revocation attempt");
        return Err(PoolError::Unauthorized.into());
    }

    // Remove the action
    pool_state.delegate_management.remove_pending_action(action_id)?;

    // Save updated state
    let mut serialized_data = Vec::new();
    pool_state.serialize(&mut serialized_data)?;
    pool_state_account.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);

    msg!("Delegate action revoked successfully");
    Ok(())
}

/// Process setting time limits for delegate actions
pub fn process_set_delegate_time_limits(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    delegate: Pubkey,
    time_limits: DelegateTimeLimits,
) -> ProgramResult {
    msg!("Processing SetDelegateTimeLimits for delegate: {}", delegate);
    let account_info_iter = &mut accounts.iter();

    // ✅ CRITICAL: System pause validation (takes precedence over pool pause)
    let system_state_account = next_account_info(account_info_iter)?;
    crate::utils::validation::validate_system_not_paused(system_state_account)?;

    let owner_account = next_account_info(account_info_iter)?;
    let pool_state_account = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner_account.is_signer {
        msg!("Owner must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load pool state
    let mut pool_state = PoolState::deserialize(&mut &pool_state_account.data.borrow()[..])?;

    // Verify caller is pool owner
    if *owner_account.key != pool_state.owner {
        msg!("Only pool owner can set delegate time limits");
        return Err(PoolError::Unauthorized.into());
    }

    // Validate time limits
    validate_time_limits(&time_limits)?;

    // Set the new time limits
    pool_state.delegate_management.set_delegate_time_limits(&delegate, time_limits)?;

    // Save updated state
    let mut serialized_data = Vec::new();
    pool_state.serialize(&mut serialized_data)?;
    pool_state_account.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);

    msg!("Delegate time limits updated successfully");
    Ok(())
}

/// Validate action parameters based on action type
fn validate_action_params(
    action_type: &DelegateActionType,
    params: &DelegateActionParams,
) -> ProgramResult {
    match (action_type, params) {
        (DelegateActionType::FeeChange, DelegateActionParams::FeeChange { new_fee_basis_points }) => {
            if *new_fee_basis_points > MAX_SWAP_FEE_BASIS_POINTS {
                msg!("Invalid fee basis points: {}", new_fee_basis_points);
                return Err(PoolError::InvalidActionParameters.into());
            }
        },
        (DelegateActionType::Withdrawal, DelegateActionParams::Withdrawal { amount, .. }) => {
            if *amount == 0 {
                msg!("Withdrawal amount cannot be zero");
                return Err(PoolError::InvalidActionParameters.into());
            }
        },
        (DelegateActionType::PoolPause, DelegateActionParams::PoolPause { duration, .. }) => {
            if *duration < 60 || *duration > 259200 {
                msg!("Invalid pause duration: {}", duration);
                return Err(PoolError::InvalidActionParameters.into());
            }
        },
        (action_type, params) => {
            msg!("Mismatched action type and parameters: {:?} {:?}", action_type, params);
            return Err(PoolError::InvalidActionType.into());
        }
    }
    Ok(())
}

/// Validate time limits are within allowed ranges
fn validate_time_limits(time_limits: &DelegateTimeLimits) -> ProgramResult {
    if time_limits.fee_change_wait_time < 300 || time_limits.fee_change_wait_time > 259200 ||
       time_limits.withdraw_wait_time < 300 || time_limits.withdraw_wait_time > 259200 ||
       time_limits.pause_wait_time < 300 || time_limits.pause_wait_time > 259200 {
        msg!("Invalid time limits: must be between 5 minutes and 72 hours");
        return Err(PoolError::InvalidWaitTime { wait_time: 0 }.into());
    }
    Ok(())
} 