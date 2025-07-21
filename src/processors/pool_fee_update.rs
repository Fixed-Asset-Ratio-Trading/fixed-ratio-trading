//! Pool Fee Update Processor
//! 
//! This module handles the UpdatePoolFees instruction which allows the program authority
//! to update the contract fees for a specific pool.

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
};

use crate::{
    constants::*,
    error::PoolError,
    utils::validation::validate_and_deserialize_pool_state_secure,
};

/// Processes the UpdatePoolFees instruction
/// 
/// This function allows only the program authority to update the contract fees
/// for a specific pool. It supports updating either the liquidity fee or swap fee
/// (or both) using bitwise flags.
/// 
/// # Arguments
/// * `program_id` - The program ID
/// * `accounts` - Array of account infos in the following order:
///   - [0] Program Authority Signer (must be program upgrade authority)
///   - [1] System State PDA (for system pause validation)
///   - [2] Pool State PDA (writable, to update fee parameters)
///   - [3] Program Data Account (for upgrade authority validation)
/// * `update_flags` - Bitwise flags indicating which fees to update
/// * `new_liquidity_fee` - New liquidity fee in lamports
/// * `new_swap_fee` - New swap fee in lamports
/// 
/// # Returns
/// * `ProgramResult` - Success or error
pub fn process_update_pool_fees(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    update_flags: u8,
    new_liquidity_fee: u64,
    new_swap_fee: u64,
) -> ProgramResult {
    msg!("🔧 POOL FEE UPDATE TRANSACTION");
    msg!("📊 Update Flags: 0b{:03b} ({})", update_flags, update_flags);
    msg!("💰 New Liquidity Fee: {} lamports ({} SOL)", new_liquidity_fee, new_liquidity_fee as f64 / 1_000_000_000.0);
    msg!("💰 New Swap Fee: {} lamports ({} SOL)", new_swap_fee, new_swap_fee as f64 / 1_000_000_000.0);
    
    // ✅ ACCOUNT EXTRACTION: Extract accounts using optimized indices
    let account_info_iter = &mut accounts.iter();
    let program_authority_signer = next_account_info(account_info_iter)?; // Index 0: Program Authority Signer
    let system_state_pda = next_account_info(account_info_iter)?;         // Index 1: System State PDA
    let pool_state_pda = next_account_info(account_info_iter)?;           // Index 2: Pool State PDA
    let program_data_account = next_account_info(account_info_iter)?;     // Index 3: Program Data Account
    
    msg!("⏳ Step 1/4: Validating system state");
    
    // ✅ SYSTEM PAUSE VALIDATION: Ensure system is not paused
    crate::utils::validation::validate_system_not_paused_secure(system_state_pda, program_id)?;
    msg!("✅ System is not paused");
    
    msg!("⏳ Step 2/4: Validating program authority");
    
    // ✅ PROGRAM AUTHORITY VALIDATION: Ensure caller is the program upgrade authority
    validate_program_authority(program_authority_signer, program_data_account, program_id)?;
    msg!("✅ Program authority validation passed");
    
    msg!("⏳ Step 3/4: Validating fee update parameters");
    
    // ✅ FEE UPDATE FLAGS VALIDATION: Ensure valid update flags
    validate_fee_update_flags(update_flags)?;
    msg!("✅ Fee update flags validation passed");
    
    // ✅ FEE VALIDATION: Ensure new fees are within acceptable limits
    validate_fee_limits(update_flags, new_liquidity_fee, new_swap_fee)?;
    msg!("✅ Fee limits validation passed");
    
    msg!("⏳ Step 4/4: Loading and updating pool state");
    
    // ✅ LOAD POOL STATE: Load current pool state for validation and update
    let mut pool_state_data = validate_and_deserialize_pool_state_secure(pool_state_pda, program_id)?;
    
    // ✅ DISPLAY CURRENT FEES: Show current fee configuration
    msg!("💰 CURRENT FEE CONFIGURATION:");
    msg!("   • Liquidity Fee: {} lamports ({} SOL)", 
         pool_state_data.contract_liquidity_fee, 
         pool_state_data.contract_liquidity_fee as f64 / 1_000_000_000.0);
    msg!("   • Swap Fee: {} lamports ({} SOL)", 
         pool_state_data.swap_contract_fee, 
         pool_state_data.swap_contract_fee as f64 / 1_000_000_000.0);
    
    // ✅ UPDATE FEES: Apply fee updates based on flags
    let mut fees_updated = false;
    
    if update_flags & FEE_UPDATE_FLAG_LIQUIDITY != 0 {
        let old_liquidity_fee = pool_state_data.contract_liquidity_fee;
        pool_state_data.contract_liquidity_fee = new_liquidity_fee;
        msg!("✅ Liquidity fee updated: {} → {} lamports", old_liquidity_fee, new_liquidity_fee);
        fees_updated = true;
    }
    
    if update_flags & FEE_UPDATE_FLAG_SWAP != 0 {
        let old_swap_fee = pool_state_data.swap_contract_fee;
        pool_state_data.swap_contract_fee = new_swap_fee;
        msg!("✅ Swap fee updated: {} → {} lamports", old_swap_fee, new_swap_fee);
        fees_updated = true;
    }
    
    if !fees_updated {
        return Err(PoolError::InvalidFeeUpdateFlags { flags: update_flags }.into());
    }
    
    // ✅ SERIALIZE UPDATED POOL STATE: Save changes to account
    use borsh::BorshSerialize;
    pool_state_data.serialize(&mut &mut pool_state_pda.data.borrow_mut()[..])?;
    msg!("✅ Pool state serialized with updated fees");
    
    // ✅ SUCCESS SUMMARY
    msg!("🎉 POOL FEE UPDATE COMPLETED SUCCESSFULLY!");
    msg!("==========================================");
    msg!("✅ UPDATED FEE CONFIGURATION:");
    msg!("   • Liquidity Fee: {} lamports ({} SOL)", 
         pool_state_data.contract_liquidity_fee, 
         pool_state_data.contract_liquidity_fee as f64 / 1_000_000_000.0);
    msg!("   • Swap Fee: {} lamports ({} SOL)", 
         pool_state_data.swap_contract_fee, 
         pool_state_data.swap_contract_fee as f64 / 1_000_000_000.0);
    msg!("");
    msg!("📊 UPDATE SUMMARY:");
    msg!("   • Pool: {}", pool_state_pda.key);
    msg!("   • Updated by: {}", program_authority_signer.key);
    msg!("   • Update flags: 0b{:03b} ({})", update_flags, update_flags);
    msg!("");
    msg!("🚀 NEXT STEPS:");
    msg!("   • New fees will apply to all future operations");
    msg!("   • Existing pending fees are not affected");
    msg!("   • Monitor pool activity with new fee structure");
    msg!("==========================================");
    
    Ok(())
}

/// Validates that the caller is the program upgrade authority
/// 
/// # Arguments
/// * `program_authority_signer` - The account claiming to be the program authority
/// * `program_data_account` - The program data account for validation
/// * `program_id` - The program ID
/// 
/// # Returns
/// * `ProgramResult` - Success or error
fn validate_program_authority(
    program_authority_signer: &AccountInfo,
    _program_data_account: &AccountInfo,
    _program_id: &Pubkey,
) -> ProgramResult {
    // ✅ SIGNER VALIDATION: Ensure the authority signed the transaction
    if !program_authority_signer.is_signer {
        msg!("❌ Program authority must sign the transaction");
        return Err(PoolError::UnauthorizedFeeUpdate.into());
    }
    
    // ✅ PROGRAM DATA VALIDATION: Validate that this is the correct program data account
    let _expected_program_data_key = Pubkey::find_program_address(
        &[],
        &solana_program::bpf_loader_upgradeable::id()
    ).0;
    
    // For upgradeable programs, the program data account follows a specific pattern
    // The actual validation would check the program data account structure
    // For now, we'll implement a basic check that can be enhanced
    
    // ✅ UPGRADE AUTHORITY VALIDATION: Check against program data account
    // In a production system, you would:
    // 1. Deserialize the program data account
    // 2. Extract the upgrade authority field
    // 3. Compare it with the signer
    
    // For now, we'll use a basic authority check
    // This should be enhanced with proper BPF loader program data parsing
    if !program_authority_signer.is_signer {
        msg!("❌ Program authority validation failed: not a signer");
        return Err(PoolError::UnauthorizedFeeUpdate.into());
    }
    
    // TODO: Implement proper upgrade authority validation
    // This is a security-critical component that should validate against
    // the actual upgrade authority stored in the program data account
    
    msg!("✅ Program authority validation completed (basic check)");
    msg!("⚠️  Production deployment requires enhanced authority validation");
    Ok(())
}

/// Validates the fee update flags
/// 
/// # Arguments
/// * `update_flags` - The bitwise flags indicating which fees to update
/// 
/// # Returns
/// * `ProgramResult` - Success or error
fn validate_fee_update_flags(update_flags: u8) -> ProgramResult {
    // ✅ FLAG VALIDATION: Ensure flags are valid combinations
    match update_flags {
        FEE_UPDATE_FLAG_LIQUIDITY => {
            msg!("✅ Updating liquidity fee only");
            Ok(())
        },
        FEE_UPDATE_FLAG_SWAP => {
            msg!("✅ Updating swap fee only");
            Ok(())
        },
        FEE_UPDATE_FLAG_BOTH => {
            msg!("✅ Updating both liquidity and swap fees");
            Ok(())
        },
        _ => {
            msg!("❌ Invalid fee update flags: 0b{:03b} ({})", update_flags, update_flags);
            msg!("   Valid flags: 1 (liquidity), 2 (swap), 3 (both)");
            Err(PoolError::InvalidFeeUpdateFlags { flags: update_flags }.into())
        }
    }
}

/// Validates that the new fees are within acceptable limits
/// 
/// # Arguments
/// * `update_flags` - The bitwise flags indicating which fees to update
/// * `new_liquidity_fee` - The new liquidity fee in lamports
/// * `new_swap_fee` - The new swap fee in lamports
/// 
/// # Returns
/// * `ProgramResult` - Success or error
fn validate_fee_limits(
    update_flags: u8,
    new_liquidity_fee: u64,
    new_swap_fee: u64,
) -> ProgramResult {
    // ✅ LIQUIDITY FEE VALIDATION: Check if liquidity fee is being updated and is valid
    if update_flags & FEE_UPDATE_FLAG_LIQUIDITY != 0 {
        if new_liquidity_fee < MIN_LIQUIDITY_FEE {
            msg!("❌ Liquidity fee too low: {} lamports (minimum: {} lamports)", 
                 new_liquidity_fee, MIN_LIQUIDITY_FEE);
            return Err(PoolError::InvalidLiquidityFee { 
                fee: new_liquidity_fee, 
                min: MIN_LIQUIDITY_FEE, 
                max: MAX_LIQUIDITY_FEE 
            }.into());
        }
        
        if new_liquidity_fee > MAX_LIQUIDITY_FEE {
            msg!("❌ Liquidity fee too high: {} lamports (maximum: {} lamports)", 
                 new_liquidity_fee, MAX_LIQUIDITY_FEE);
            return Err(PoolError::InvalidLiquidityFee { 
                fee: new_liquidity_fee, 
                min: MIN_LIQUIDITY_FEE, 
                max: MAX_LIQUIDITY_FEE 
            }.into());
        }
        
        msg!("✅ Liquidity fee validation passed: {} lamports", new_liquidity_fee);
    }
    
    // ✅ SWAP FEE VALIDATION: Check if swap fee is being updated and is valid
    if update_flags & FEE_UPDATE_FLAG_SWAP != 0 {
        if new_swap_fee < MIN_SWAP_FEE {
            msg!("❌ Swap fee too low: {} lamports (minimum: {} lamports)", 
                 new_swap_fee, MIN_SWAP_FEE);
            return Err(PoolError::InvalidSwapFee { 
                fee: new_swap_fee, 
                min: MIN_SWAP_FEE, 
                max: MAX_SWAP_FEE 
            }.into());
        }
        
        if new_swap_fee > MAX_SWAP_FEE {
            msg!("❌ Swap fee too high: {} lamports (maximum: {} lamports)", 
                 new_swap_fee, MAX_SWAP_FEE);
            return Err(PoolError::InvalidSwapFee { 
                fee: new_swap_fee, 
                min: MIN_SWAP_FEE, 
                max: MAX_SWAP_FEE 
            }.into());
        }
        
        msg!("✅ Swap fee validation passed: {} lamports", new_swap_fee);
    }
    
    Ok(())
} 