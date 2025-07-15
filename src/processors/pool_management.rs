//! Pool Management Operations
//! 
//! This module handles pool-specific pause/unpause operations using bitwise flags
//! that allow pool owners to control their individual pools without affecting
//! other pools or requiring system-wide authority.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    constants::*,
    error::PoolError,
    state::PoolState,
    utils::validation::{validate_signer, validate_and_deserialize_pool_state_secure},
};

/// Pauses pool operations using bitwise flags (pool owner only)
/// 
/// Uses bitwise flags to control which operations to pause:
/// - PAUSE_FLAG_GENERAL (1): Pause deposits/withdrawals
/// - PAUSE_FLAG_SWAPS (2): Pause swaps
/// - PAUSE_FLAG_ALL (3): Pause both (required for consolidation eligibility)
/// 
/// **Idempotent**: Pausing already paused operations does not cause an error.
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA validation
/// * `pause_flags` - Bitwise flags indicating which operations to pause
/// * `accounts` - Array of accounts in the following order:
///   - [0] Pool Owner Signer (must match pool.owner)
///   - [1] System State PDA (for system pause validation)  
///   - [2] Pool State PDA (writable, to update pause state)
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_pause_pool(
    program_id: &Pubkey,
    pause_flags: u8,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing PausePool instruction with flags: 0b{:08b} ({})", pause_flags, pause_flags);
    
    // Validate flags
    if pause_flags == 0 {
        msg!("❌ Invalid pause flags: cannot be zero");
        return Err(ProgramError::InvalidArgument);
    }
    if pause_flags > PAUSE_FLAG_MAX {
        msg!("❌ Invalid pause flags: {} exceeds maximum {}", pause_flags, PAUSE_FLAG_MAX);
        return Err(ProgramError::InvalidArgument);
    }
    
    // Extract accounts
    let pool_owner_signer = &accounts[0];
    let system_state_pda = &accounts[1];
    let pool_state_pda = &accounts[2];
    
    // Validate system is not paused (allow pool owner operations during system pause)
    crate::utils::validation::validate_system_not_paused_secure(system_state_pda, program_id)?;
    
    // Validate signer
    validate_signer(pool_owner_signer, "pool pause")?;
    
    // Load and validate pool state
    let mut pool_state = validate_and_deserialize_pool_state_secure(pool_state_pda, program_id)?;
    
    // Validate pool owner authority
    if pool_state.owner != *pool_owner_signer.key {
        msg!("❌ Unauthorized: Only pool owner can pause pool operations");
        msg!("   Pool owner: {}", pool_state.owner);
        msg!("   Attempted by: {}", pool_owner_signer.key);
        return Err(PoolError::Unauthorized.into());
    }
    
    // Apply pause flags (idempotent - no error if already paused)
    let mut operations_changed = Vec::new();
    
    if pause_flags & PAUSE_FLAG_GENERAL != 0 {
        if !pool_state.paused {
            pool_state.paused = true;
            operations_changed.push("general operations");
        }
    }
    
    if pause_flags & PAUSE_FLAG_SWAPS != 0 {
        if !pool_state.swaps_paused {
            pool_state.swaps_paused = true;
            operations_changed.push("swaps");
        }
    }
    
    // Save updated pool state
    let serialized_data = pool_state.try_to_vec()?;
    pool_state_pda.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    // Log results
    if operations_changed.is_empty() {
        msg!("ℹ️ No changes made - requested operations were already paused");
    } else {
        msg!("✅ Pool operations paused: {}", operations_changed.join(", "));
    }
    
    msg!("   Pool: {}", pool_state_pda.key);
    msg!("   General operations: {}", if pool_state.paused { "PAUSED" } else { "ENABLED" });
    msg!("   Swap operations: {}", if pool_state.swaps_paused { "PAUSED" } else { "ENABLED" });
    msg!("   Consolidation eligible: {}", 
         if pool_state.paused && pool_state.swaps_paused { "YES" } else { "NO" });
    
    Ok(())
}

/// Unpauses pool operations using bitwise flags (pool owner only)
/// 
/// Uses bitwise flags to control which operations to unpause:
/// - PAUSE_FLAG_GENERAL (1): Unpause deposits/withdrawals
/// - PAUSE_FLAG_SWAPS (2): Unpause swaps
/// - PAUSE_FLAG_ALL (3): Unpause both operations
/// 
/// **Idempotent**: Unpausing already unpaused operations does not cause an error.
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA validation
/// * `unpause_flags` - Bitwise flags indicating which operations to unpause
/// * `accounts` - Array of accounts in the following order:
///   - [0] Pool Owner Signer (must match pool.owner)
///   - [1] System State PDA (for system pause validation)  
///   - [2] Pool State PDA (writable, to update pause state)
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_unpause_pool(
    program_id: &Pubkey,
    unpause_flags: u8,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing UnpausePool instruction with flags: 0b{:08b} ({})", unpause_flags, unpause_flags);
    
    // Validate flags
    if unpause_flags == 0 {
        msg!("❌ Invalid unpause flags: cannot be zero");
        return Err(ProgramError::InvalidArgument);
    }
    if unpause_flags > PAUSE_FLAG_MAX {
        msg!("❌ Invalid unpause flags: {} exceeds maximum {}", unpause_flags, PAUSE_FLAG_MAX);
        return Err(ProgramError::InvalidArgument);
    }
    
    // Extract accounts
    let pool_owner_signer = &accounts[0];
    let system_state_pda = &accounts[1];
    let pool_state_pda = &accounts[2];
    
    // Validate system is not paused
    crate::utils::validation::validate_system_not_paused_secure(system_state_pda, program_id)?;
    
    // Validate signer
    validate_signer(pool_owner_signer, "pool unpause")?;
    
    // Load and validate pool state
    let mut pool_state = validate_and_deserialize_pool_state_secure(pool_state_pda, program_id)?;
    
    // Validate pool owner authority
    if pool_state.owner != *pool_owner_signer.key {
        msg!("❌ Unauthorized: Only pool owner can unpause pool operations");
        msg!("   Pool owner: {}", pool_state.owner);
        msg!("   Attempted by: {}", pool_owner_signer.key);
        return Err(PoolError::Unauthorized.into());
    }
    
    // Apply unpause flags (idempotent - no error if already unpaused)
    let mut operations_changed = Vec::new();
    
    if unpause_flags & PAUSE_FLAG_GENERAL != 0 {
        if pool_state.paused {
            pool_state.paused = false;
            operations_changed.push("general operations");
        }
    }
    
    if unpause_flags & PAUSE_FLAG_SWAPS != 0 {
        if pool_state.swaps_paused {
            pool_state.swaps_paused = false;
            operations_changed.push("swaps");
        }
    }
    
    // Save updated pool state
    let serialized_data = pool_state.try_to_vec()?;
    pool_state_pda.data.borrow_mut()[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    // Log results
    if operations_changed.is_empty() {
        msg!("ℹ️ No changes made - requested operations were already unpaused");
    } else {
        msg!("✅ Pool operations unpaused: {}", operations_changed.join(", "));
    }
    
    msg!("   Pool: {}", pool_state_pda.key);
    msg!("   General operations: {}", if pool_state.paused { "PAUSED" } else { "ENABLED" });
    msg!("   Swap operations: {}", if pool_state.swaps_paused { "PAUSED" } else { "ENABLED" });
    msg!("   Consolidation eligible: {}", 
         if pool_state.paused && pool_state.swaps_paused { "YES" } else { "NO" });
    
    Ok(())
} 