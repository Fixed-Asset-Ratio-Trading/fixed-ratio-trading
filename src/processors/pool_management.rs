//! Pool Management Operations
//! 
//! This module handles pool-specific pause/unpause operations using bitwise flags
//! that allow the Program Upgrade Authority to control individual pools without affecting
//! other pools or requiring system-wide pause.

use borsh::BorshSerialize;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
};

use crate::{
    constants::*,
    utils::validation::validate_and_deserialize_pool_state_secure,
};

/// Pauses pool operations using bitwise flags (Program Upgrade Authority only)
/// 
/// Uses bitwise flags to control which operations to pause:
/// - PAUSE_FLAG_LIQUIDITY (1): Pause deposits/withdrawals
/// - PAUSE_FLAG_SWAPS (2): Pause swaps
/// - PAUSE_FLAG_ALL (3): Pause both (required for consolidation eligibility)
/// 
/// **Security**: Only the Program Upgrade Authority can pause individual pools.
/// **Idempotent**: Pausing already paused operations does not cause an error.
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA validation
/// * `pause_flags` - Bitwise flags indicating which operations to pause
/// * `accounts` - Array of accounts in the following order:
///   - [0] Program Upgrade Authority Signer (must be program upgrade authority)
///   - [1] System State PDA (for system pause validation)  
///   - [2] Pool State PDA (writable, to update pause state)
///   - [3] Program Data Account (for authority validation)
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_pause_pool(
    program_id: &Pubkey,
    pause_flags: u8,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing PausePool instruction with flags: 0b{:08b} ({})", pause_flags, pause_flags);
    
    // Extract accounts
    let program_authority_signer = &accounts[0];
    let system_state_pda = &accounts[1];
    let pool_state_pda = &accounts[2];
    let program_data_account = &accounts[3];
    
    // Validate system is not paused (allow authority operations during system pause)
    crate::utils::validation::validate_system_not_paused_secure(system_state_pda, program_id)?;
    
    // Validate Program Upgrade Authority
    use crate::utils::program_authority::validate_program_upgrade_authority;
    validate_program_upgrade_authority(program_id, program_data_account, program_authority_signer)?;
    
    // Load and validate pool state
    let mut pool_state = validate_and_deserialize_pool_state_secure(pool_state_pda, program_id)?;
    
    // Apply pause flags (idempotent - no error if already paused)
    let mut operations_changed = Vec::new();
    
    if pause_flags & PAUSE_FLAG_LIQUIDITY != 0 && !pool_state.liquidity_paused() {
        pool_state.set_liquidity_paused(true);
        operations_changed.push("general operations");
    }
    
    if pause_flags & PAUSE_FLAG_SWAPS != 0 && !pool_state.swaps_paused() {
        pool_state.set_swaps_paused(true);
        operations_changed.push("swaps");
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
    msg!("   Liquidity operations: {}", if pool_state.liquidity_paused() { "PAUSED" } else { "ENABLED" });
    msg!("   Swap operations: {}", if pool_state.swaps_paused() { "PAUSED" } else { "ENABLED" });
    msg!("   Consolidation eligible: {}", 
         if pool_state.liquidity_paused() && pool_state.swaps_paused() { "YES" } else { "NO" });
    
    Ok(())
}

/// Unpauses pool operations using bitwise flags (Program Upgrade Authority only)
/// 
/// Uses bitwise flags to control which operations to unpause:
/// - PAUSE_FLAG_LIQUIDITY (1): Unpause deposits/withdrawals
/// - PAUSE_FLAG_SWAPS (2): Unpause swaps
/// - PAUSE_FLAG_ALL (3): Unpause both operations
/// 
/// **Security**: Only the Program Upgrade Authority can unpause individual pools.
/// **Idempotent**: Unpausing already unpaused operations does not cause an error.
/// 
/// # Arguments
/// * `program_id` - The program ID for PDA validation
/// * `unpause_flags` - Bitwise flags indicating which operations to unpause
/// * `accounts` - Array of accounts in the following order:
///   - [0] Program Upgrade Authority Signer (must be program upgrade authority)
///   - [1] System State PDA (for system pause validation)  
///   - [2] Pool State PDA (writable, to update pause state)
///   - [3] Program Data Account (for authority validation)
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_unpause_pool(
    program_id: &Pubkey,
    unpause_flags: u8,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing UnpausePool instruction with flags: 0b{:08b} ({})", unpause_flags, unpause_flags);
    
    // Extract accounts
    let program_authority_signer = &accounts[0];
    let system_state_pda = &accounts[1];
    let pool_state_pda = &accounts[2];
    let program_data_account = &accounts[3];
    
    // Validate system is not paused
    crate::utils::validation::validate_system_not_paused_secure(system_state_pda, program_id)?;
    
    // Validate Program Upgrade Authority
    use crate::utils::program_authority::validate_program_upgrade_authority;
    validate_program_upgrade_authority(program_id, program_data_account, program_authority_signer)?;
    
    // Load and validate pool state
    let mut pool_state = validate_and_deserialize_pool_state_secure(pool_state_pda, program_id)?;
    
    // Apply unpause flags (idempotent - no error if already unpaused)
    let mut operations_changed = Vec::new();
    
    if unpause_flags & PAUSE_FLAG_LIQUIDITY != 0 && pool_state.liquidity_paused() {
        pool_state.set_liquidity_paused(false);
        operations_changed.push("general operations");
    }
    
    if unpause_flags & PAUSE_FLAG_SWAPS != 0 && pool_state.swaps_paused() {
        pool_state.set_swaps_paused(false);
        operations_changed.push("swaps");
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
    msg!("   Liquidity operations: {}", if pool_state.liquidity_paused() { "PAUSED" } else { "ENABLED" });
    msg!("   Swap operations: {}", if pool_state.swaps_paused() { "PAUSED" } else { "ENABLED" });
    msg!("   Consolidation eligible: {}", 
         if pool_state.liquidity_paused() && pool_state.swaps_paused() { "YES" } else { "NO" });
    
    Ok(())
} 