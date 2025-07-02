//! Pool State Types and Structures
//! 
//! This module contains all the core state structures for the trading pool,
//! including the main PoolState and related helper types.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    pubkey::Pubkey,
    sysvar::rent::Rent,
    program_pack::Pack,
};
use spl_token::state::{Account as TokenAccount, Mint as MintAccount};
use crate::{
    constants::MINIMUM_RENT_BUFFER,
};

/// Tracks rent requirements for pool accounts to ensure rent exemption.
#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
pub struct RentRequirements {
    pub last_update_slot: u64,
    pub rent_exempt_minimum: u64,
    pub pool_state_rent: u64,
    pub token_vault_rent: u64,
    pub lp_mint_rent: u64,
}

impl RentRequirements {
    pub fn new(rent: &Rent) -> Self {
        Self {
            last_update_slot: 0,
            rent_exempt_minimum: rent.minimum_balance(0),
            pool_state_rent: rent.minimum_balance(PoolState::get_packed_len()),
            token_vault_rent: rent.minimum_balance(TokenAccount::LEN),
            lp_mint_rent: rent.minimum_balance(MintAccount::LEN),
        }
    }

    pub fn update_if_needed(&mut self, rent: &Rent, current_slot: u64) -> bool {
        // Update rent requirements if they've changed or if it's been a while
        let needs_update = self.last_update_slot == 0 || 
                          current_slot - self.last_update_slot > 1000 || // Update every ~1000 slots
                          self.pool_state_rent != rent.minimum_balance(PoolState::get_packed_len()) ||
                          self.token_vault_rent != rent.minimum_balance(TokenAccount::LEN) ||
                          self.lp_mint_rent != rent.minimum_balance(MintAccount::LEN);

        if needs_update {
            self.pool_state_rent = rent.minimum_balance(PoolState::get_packed_len());
            self.token_vault_rent = rent.minimum_balance(TokenAccount::LEN);
            self.lp_mint_rent = rent.minimum_balance(MintAccount::LEN);
            self.last_update_slot = current_slot;
        }

        needs_update
    }

    pub fn get_total_required_rent(&self) -> u64 {
        self.pool_state_rent + 
        (2 * self.token_vault_rent) + // Two token vaults
        (2 * self.lp_mint_rent) + // Two LP mints
        MINIMUM_RENT_BUFFER // Additional buffer
    }

    pub fn get_packed_len() -> usize {
        8 + // last_update_slot
        8 + // rent_exempt_minimum
        8 + // pool_state_rent
        8 + // token_vault_rent
        8   // lp_mint_rent
    }
}

/// Main pool state containing all configuration and runtime data.
/// NOTE: Pool state only contains pool-specific data and owner information.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct PoolState {
    pub owner: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub token_a_vault: Pubkey,
    pub token_b_vault: Pubkey,
    pub lp_token_a_mint: Pubkey,
    pub lp_token_b_mint: Pubkey,
    pub ratio_a_numerator: u64,
    pub ratio_b_denominator: u64,
    pub token_a_is_the_multiple: bool,
    pub total_token_a_liquidity: u64,
    pub total_token_b_liquidity: u64,
    pub pool_authority_bump_seed: u8,
    pub token_a_vault_bump_seed: u8,
    pub token_b_vault_bump_seed: u8,
    pub is_initialized: bool,
    pub rent_requirements: RentRequirements,
    pub paused: bool, // Pool-specific pause (separate from system pause)
    pub swaps_paused: bool, // Swap-specific pause within this pool
    
    // Automatic withdrawal protection
    pub withdrawal_protection_active: bool,
    
    // Future feature: Single LP token mode
    // When implemented, this will allow only LP token A (the "multiple" token) to be issued
    // for liquidity provision, while still allowing withdrawals of either token A or B
    // at the fixed ratio. This simplifies LP token management for certain pool configurations.
    // NOTE: Currently not implemented - remains false regardless of input
    pub only_lp_token_a_for_both: bool,
    
    // Fee collection and withdrawal tracking
    pub collected_fees_token_a: u64,
    pub collected_fees_token_b: u64,
    pub total_fees_withdrawn_token_a: u64,
    pub total_fees_withdrawn_token_b: u64,
    pub swap_fee_basis_points: u64,
    pub collected_sol_fees: u64,
    pub total_sol_fees_withdrawn: u64,
}

impl Default for PoolState {
    fn default() -> Self {
        Self {
            owner: Pubkey::default(),
            token_a_mint: Pubkey::default(),
            token_b_mint: Pubkey::default(),
            token_a_vault: Pubkey::default(),
            token_b_vault: Pubkey::default(),
            lp_token_a_mint: Pubkey::default(),
            lp_token_b_mint: Pubkey::default(),
            ratio_a_numerator: 0,
            ratio_b_denominator: 0,
            token_a_is_the_multiple: false,
            total_token_a_liquidity: 0,
            total_token_b_liquidity: 0,
            pool_authority_bump_seed: 0,
            token_a_vault_bump_seed: 0,
            token_b_vault_bump_seed: 0,
            is_initialized: false,
            rent_requirements: RentRequirements::default(),
            paused: false,
            swaps_paused: false,
            withdrawal_protection_active: false,
            only_lp_token_a_for_both: false,
            collected_fees_token_a: 0,
            collected_fees_token_b: 0,
            total_fees_withdrawn_token_a: 0,
            total_fees_withdrawn_token_b: 0,
            swap_fee_basis_points: 0,
            collected_sol_fees: 0,
            total_sol_fees_withdrawn: 0,
        }
    }
}

impl PoolState {
    pub fn get_packed_len() -> usize {
        32 + // owner
        32 + // token_a_mint
        32 + // token_b_mint
        32 + // token_a_vault
        32 + // token_b_vault
        32 + // lp_token_a_mint
        32 + // lp_token_b_mint
        8 +  // ratio_a_numerator
        8 +  // ratio_b_denominator
        1 +  // token_a_is_the_multiple
        8 +  // total_token_a_liquidity
        8 +  // total_token_b_liquidity
        1 +  // pool_authority_bump_seed
        1 +  // token_a_vault_bump_seed
        1 +  // token_b_vault_bump_seed
        1 +  // is_initialized
        RentRequirements::get_packed_len() + // rent_requirements
        1 +  // paused
        1 +  // swaps_paused
        1 +  // withdrawal_protection_active
        1 +  // only_lp_token_a_for_both
        
        // Fee collection and withdrawal tracking
        8 +  // collected_fees_token_a
        8 +  // collected_fees_token_b
        8 +  // total_fees_withdrawn_token_a
        8 +  // total_fees_withdrawn_token_b
        8 +  // swap_fee_basis_points
        8 +  // collected_sol_fees
        8    // total_sol_fees_withdrawn
    }
} 