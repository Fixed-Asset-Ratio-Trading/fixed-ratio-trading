//! Pool State Types and Structures
//! 
//! This module contains all the core state structures for the trading pool,
//! including the main PoolState and related helper types.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    pubkey::Pubkey,
};

/// Main pool state containing all configuration and runtime data.
/// 
/// **PHASE 1: DISTRIBUTED COLLECTION ARCHITECTURE**
/// Updated to support distributed SOL fee collection with batch consolidation.
/// Pool creation fees still go directly to MainTreasuryState (optimal for one-time fees).
/// 
/// **BASIS POINTS REFACTOR: Critical Implementation Notes**
/// 
/// All token amounts and ratios in this structure are stored in **basis points** (smallest token units).
/// External applications are responsible for converting between display units and basis points.
/// The smart contract assumes all input values are already in basis points and performs no conversion.
/// 
/// **Basis Points Examples:**
/// - 1.5 USDC (6 decimals) = 1,500,000 basis points
/// - 0.001 BTC (8 decimals) = 100,000 basis points  
/// - 1.0 SOL (9 decimals) = 1,000,000,000 basis points
/// 
/// **Conversion Responsibility:**
/// - **Input**: Client converts display units → basis points before sending to contract
/// - **Storage**: Contract stores basis points directly (this structure)
/// - **Calculations**: Contract performs all calculations in basis points
/// - **Output**: Client converts basis points → display units for user interface
/// - **Decimals**: Fetched from token mint accounts when needed (not stored here)
#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
pub struct PoolState {
    pub owner: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub token_a_vault: Pubkey,
    pub token_b_vault: Pubkey,
    pub lp_token_a_mint: Pubkey,
    pub lp_token_b_mint: Pubkey,
    
    /// **BASIS POINTS: Token A ratio numerator in basis points**
    /// 
    /// This value represents the Token A side of the pool's fixed exchange ratio
    /// in the smallest token units (basis points). External applications must
    /// convert from display units to basis points before pool creation.
    /// 
    /// Example: For "1.0 SOL = 160.0 USDT" with SOL having 9 decimals:
    /// ratio_a_numerator = 1,000,000,000 (1.0 * 10^9)
    pub ratio_a_numerator: u64,
    
    /// **BASIS POINTS: Token B ratio denominator in basis points**
    /// 
    /// This value represents the Token B side of the pool's fixed exchange ratio
    /// in the smallest token units (basis points). External applications must
    /// convert from display units to basis points before pool creation.
    /// 
    /// Example: For "1.0 SOL = 160.0 USDT" with USDT having 6 decimals:
    /// ratio_b_denominator = 160,000,000 (160.0 * 10^6)
    pub ratio_b_denominator: u64,
    
    /// **BASIS POINTS: Total Token A liquidity in basis points**
    /// 
    /// This tracks the total amount of Token A deposited into the pool
    /// in basis points. Updated during deposit/withdrawal operations.
    /// 
    /// Example: If 50.5 SOL (9 decimals) is deposited:
    /// total_token_a_liquidity = 50,500,000,000 (50.5 * 10^9)
    pub total_token_a_liquidity: u64,
    
    /// **BASIS POINTS: Total Token B liquidity in basis points**
    /// 
    /// This tracks the total amount of Token B deposited into the pool
    /// in basis points. Updated during deposit/withdrawal operations.
    /// 
    /// Example: If 1,200.75 USDT (6 decimals) is deposited:
    /// total_token_b_liquidity = 1,200,750,000 (1200.75 * 10^6)
    pub total_token_b_liquidity: u64,
    pub pool_authority_bump_seed: u8,
    pub token_a_vault_bump_seed: u8,
    pub token_b_vault_bump_seed: u8,
    pub lp_token_a_mint_bump_seed: u8,
    pub lp_token_b_mint_bump_seed: u8,
    
    /// Pool state flags using bitwise operations
    /// Bit 0 (1): One-to-many ratio configuration
    /// Bit 1 (2): Liquidity operations paused (deposits/withdrawals only)
    /// Bit 2 (4): Swap operations paused
    /// Bit 3 (8): Withdrawal protection active
    /// Bit 4 (16): Single LP token mode (future feature)
    pub flags: u8,
    
    // **NEW: CONFIGURABLE CONTRACT FEES**
    /// Contract fee for liquidity operations (deposits/withdrawals) in lamports
    /// This fee is charged in SOL to cover computational costs
    /// **Future**: Will be modifiable via pool fee update function
    pub contract_liquidity_fee: u64,
    
    /// Contract fee for swap operations in lamports  
    /// This fee is charged in SOL to cover computational costs
    /// **Future**: Will be modifiable via pool fee update function
    pub swap_contract_fee: u64,
    
    // Fee collection and withdrawal tracking (Token fees only)
    pub collected_fees_token_a: u64,
    pub collected_fees_token_b: u64,
    pub total_fees_withdrawn_token_a: u64,
    pub total_fees_withdrawn_token_b: u64,
    
    // **NEW: DISTRIBUTED SOL FEE TRACKING**
    /// SOL fees collected from liquidity operations (accumulated locally)  
    pub collected_liquidity_fees: u64,
    
    /// Total collected swap contract fees (fixed SOL amounts) accumulated from swap operations
    /// These are the fixed SOL fees charged per swap to cover computational costs
    pub collected_swap_contract_fees: u64,
    
    // **NEW: LIFETIME SOL FEE TRACKING**
    /// Total SOL fees collected by this pool since inception (never resets)
    /// This is the authoritative count of all SOL fees ever collected
    /// Formula: total_sol_fees_collected = total_fees_consolidated + current_pending_fees
    pub total_sol_fees_collected: u64,
    
    // **NEW: CONSOLIDATION MANAGEMENT**
    /// Timestamp of last consolidation (0 if never consolidated)
    pub last_consolidation_timestamp: i64,
    
    /// Total number of consolidations performed on this pool
    pub total_consolidations: u64,
    
    /// Total SOL fees transferred to treasury via consolidation
    pub total_fees_consolidated: u64,
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
        8 +  // total_token_a_liquidity
        8 +  // total_token_b_liquidity
        1 +  // pool_authority_bump_seed
        1 +  // token_a_vault_bump_seed
        1 +  // token_b_vault_bump_seed
        1 +  // lp_token_a_mint_bump_seed
        1 +  // lp_token_b_mint_bump_seed
        1 +  // flags (bitwise: one_to_many_ratio, liquidity_paused, swaps_paused, withdrawal_protection_active, only_lp_token_a_for_both)
        
        // **NEW: CONFIGURABLE CONTRACT FEES** (+16 bytes)
        8 +  // contract_liquidity_fee
        8 +  // swap_contract_fee
        
        // Fee collection and withdrawal tracking (Token fees)
        8 +  // collected_fees_token_a
        8 +  // collected_fees_token_b
        8 +  // total_fees_withdrawn_token_a
        8 +  // total_fees_withdrawn_token_b
        
        // **NEW: DISTRIBUTED SOL FEE TRACKING** (+32 bytes)
        8 +  // collected_liquidity_fees  
        8 +  // collected_swap_contract_fees
        8 +  // total_sol_fees_collected
        
        // **NEW: CONSOLIDATION MANAGEMENT** (+24 bytes)
        8 +  // last_consolidation_timestamp
        8 +  // total_consolidations
        8    // total_fees_consolidated
        
        // **REMOVED FIELDS** (-57 bytes):
        // - is_initialized: bool (1 byte) - Pool existence = initialization
        // - swap_fee_basis_points: u64 (8 bytes) - Moved to constants as fixed value
        // - collected_pool_creation_fees: u64 (8 bytes) - Pool creation happens only once, goes to MainTreasury
        // - rent_requirements: RentRequirements (40 bytes) - Rent calculations done on-demand
        
        // **NET ADDITION: +15 bytes per pool** (72 added - 57 removed)
    }
    
    // **NEW: BITWISE FLAG HELPER METHODS**
    
    /// Checks if one-to-many ratio is configured
    /// 
    /// **Purpose**: This flag identifies pools with specific whole-number ratio patterns
    /// where one or both tokens have a ratio value of exactly 1 (representing 1 whole token).
    /// 
    /// **Flag Logic**: Returns true when the pool has a token ratio where:
    /// * One or both tokens have a ratio value of exactly 1 (representing 1 whole token, not fractional)
    /// * The corresponding token(s) must have whole number values only (no fractional amounts)
    /// * Both ratios must be positive (greater than zero)
    /// 
    /// **Valid Examples** (flag is SET):
    /// * ✅ 1 SOL = 160 USDT → Returns true
    /// * ✅ 1000 DOGE = 1 USDC → Returns true
    /// * ✅ 1 BTC = 50000 USDT → Returns true
    /// 
    /// **Invalid Examples** (flag is NOT set):
    /// * ❌ 1 SOL = 160.55 USDT → Returns false (fractional value)
    /// * ❌ 2 TokenA = 3 TokenB → Returns false (neither equals 1)
    /// 
    /// **Application Usage**: This enables filtering pools for applications that specifically
    /// target whole-number ratio patterns, while other applications remain free to
    /// implement different ratio types as needed.
    pub fn one_to_many_ratio(&self) -> bool {
        self.flags & crate::constants::POOL_FLAG_ONE_TO_MANY_RATIO != 0
    }
    
    /// Sets or clears the one-to-many ratio flag
    /// 
    /// **Important**: This flag should only be set during pool creation based on the
    /// `check_one_to_many_ratio()` validation function. Manual modification after pool
    /// creation is not recommended as it may create inconsistencies.
    /// 
    /// **Technical Note**: The flag is determined by analyzing token decimals and ratios
    /// to ensure both display values are whole numbers and one equals exactly 1.0.
    /// 
    /// # Arguments
    /// * `value` - true to set the flag, false to clear it
    pub fn set_one_to_many_ratio(&mut self, value: bool) {
        if value {
            self.flags |= crate::constants::POOL_FLAG_ONE_TO_MANY_RATIO;
        } else {
            self.flags &= !crate::constants::POOL_FLAG_ONE_TO_MANY_RATIO;
        }
    }
    
    /// Checks if liquidity operations (deposits/withdrawals) are paused
    pub fn liquidity_paused(&self) -> bool {
        self.flags & crate::constants::POOL_FLAG_LIQUIDITY_PAUSED != 0
    }
    
    /// Sets or clears the liquidity operations pause flag
    pub fn set_liquidity_paused(&mut self, value: bool) {
        if value {
            self.flags |= crate::constants::POOL_FLAG_LIQUIDITY_PAUSED;
        } else {
            self.flags &= !crate::constants::POOL_FLAG_LIQUIDITY_PAUSED;
        }
    }
    
    /// Checks if swap operations are paused
    pub fn swaps_paused(&self) -> bool {
        self.flags & crate::constants::POOL_FLAG_SWAPS_PAUSED != 0
    }
    
    /// Sets or clears the swap operations pause flag
    pub fn set_swaps_paused(&mut self, value: bool) {
        if value {
            self.flags |= crate::constants::POOL_FLAG_SWAPS_PAUSED;
        } else {
            self.flags &= !crate::constants::POOL_FLAG_SWAPS_PAUSED;
        }
    }
    
    /// Checks if withdrawal protection is active
    pub fn withdrawal_protection_active(&self) -> bool {
        self.flags & crate::constants::POOL_FLAG_WITHDRAWAL_PROTECTION != 0
    }
    
    /// Sets or clears the withdrawal protection flag
    pub fn set_withdrawal_protection_active(&mut self, value: bool) {
        if value {
            self.flags |= crate::constants::POOL_FLAG_WITHDRAWAL_PROTECTION;
        } else {
            self.flags &= !crate::constants::POOL_FLAG_WITHDRAWAL_PROTECTION;
        }
    }
    
    /// Checks if single LP token mode is enabled (future feature)
    pub fn only_lp_token_a_for_both(&self) -> bool {
        self.flags & crate::constants::POOL_FLAG_SINGLE_LP_TOKEN != 0
    }
    
    /// Sets or clears the single LP token mode flag
    pub fn set_only_lp_token_a_for_both(&mut self, value: bool) {
        if value {
            self.flags |= crate::constants::POOL_FLAG_SINGLE_LP_TOKEN;
        } else {
            self.flags &= !crate::constants::POOL_FLAG_SINGLE_LP_TOKEN;
        }
    }
    
    /// Checks if swap operations are restricted to owners only
    /// 
    /// When this flag is set, only the pool owner and contract owner can perform swaps.
    /// This enables custom fee structures through separate contracts while maintaining
    /// granular access control.
    pub fn swap_for_owners_only(&self) -> bool {
        self.flags & crate::constants::POOL_FLAG_SWAP_FOR_OWNERS_ONLY != 0
    }
    
    /// Sets or clears the swap operations owner-only restriction flag
    /// 
    /// **IMPORTANT**: This flag can only be modified by the contract owner, not the pool owner.
    /// This restriction is enforced in the processor function, not here.
    pub fn set_swap_for_owners_only(&mut self, value: bool) {
        if value {
            self.flags |= crate::constants::POOL_FLAG_SWAP_FOR_OWNERS_ONLY;
        } else {
            self.flags &= !crate::constants::POOL_FLAG_SWAP_FOR_OWNERS_ONLY;
        }
    }
    
    // **NEW: Pool-level fee collection methods with atomic updates**
    
    /// Records liquidity operation fee collection
    /// 
    /// **ATOMIC UPDATE**: Updates both specific fee counter and total in single operation
    /// to prevent race conditions and ensure consistency.
    pub fn add_liquidity_fee(&mut self, fee_amount: u64, _timestamp: i64) {
        // Atomic update: both counters updated together
        self.collected_liquidity_fees += fee_amount;
        self.total_sol_fees_collected += fee_amount;
        
        // Invariant check (debug mode only) - simplified since pending_sol_fees() uses the mathematical relationship
        debug_assert_eq!(
            self.pending_sol_fees(),
            self.collected_liquidity_fees + self.collected_swap_contract_fees,
            "Pending fees calculation should match sum of individual pending fee types"
        );
    }
    
    /// Adds a swap contract fee to the accumulated fees
    /// 
    /// This function records a swap contract fee (fixed SOL amount) collected during
    /// swap operations. These fees cover computational costs.
    /// 
    /// # Arguments
    /// * `fee_amount` - The swap contract fee amount in lamports
    /// * `_timestamp` - Timestamp of the fee collection (currently unused)
    pub fn add_swap_contract_fee(&mut self, fee_amount: u64, _timestamp: i64) {
        // Atomic update: both counters updated together
        self.collected_swap_contract_fees += fee_amount;
        self.total_sol_fees_collected += fee_amount;
        
        // Invariant check (debug mode only) - simplified since pending_sol_fees() uses the mathematical relationship
        debug_assert_eq!(
            self.pending_sol_fees(),
            self.collected_liquidity_fees + self.collected_swap_contract_fees,
            "Pending fees calculation should match sum of individual pending fee types"
        );
    }
    

    
    /// Calculates current pending SOL fees awaiting consolidation
    /// 
    /// **ACCURATE CALCULATION**: Uses the mathematical relationship:
    /// pending_fees = total_lifetime_fees - already_consolidated_fees
    /// 
    /// This automatically includes ALL fee types (pool creation, liquidity, swaps)
    /// without needing to track consolidation state of individual fee types.
    /// 
    /// **WHY THIS IS BETTER THAN SUMMING INDIVIDUAL FEE TYPES:**
    /// - Pool creation fees go directly to MainTreasuryState, not to individual pools
    /// - Previous total_collected_sol_fees() only summed liquidity + swap fees
    /// - Would need complex logic to determine if pool creation fees were consolidated
    /// - Mathematical approach is simple, accurate, and includes everything automatically
    pub fn pending_sol_fees(&self) -> u64 {
        // Simple and accurate: total collected minus what's been consolidated
        self.total_sol_fees_collected - self.total_fees_consolidated
    }
    
    /// Calculates total operations since last consolidation using fee constants
    pub fn total_operations_since_consolidation(&self) -> u64 {
        use crate::constants::*;
        
        let liquidity_ops = self.collected_liquidity_fees / DEPOSIT_WITHDRAWAL_FEE;
        let regular_swap_ops = self.collected_swap_contract_fees / SWAP_CONTRACT_FEE;
        
        liquidity_ops + regular_swap_ops
    }
    
    /// Calculates individual operation counts since last consolidation
    pub fn operation_counts_since_consolidation(&self) -> (u64, u64) {
        use crate::constants::*;
        
        let liquidity_ops = self.collected_liquidity_fees / DEPOSIT_WITHDRAWAL_FEE;
        let regular_swap_ops = self.collected_swap_contract_fees / SWAP_CONTRACT_FEE;
        
        (liquidity_ops, regular_swap_ops)
    }
    
    /// Resets consolidation counters (called after successful consolidation)
    /// 
    /// **RACE CONDITION PROTECTION**: This method performs atomic updates to ensure
    /// that total_sol_fees_collected remains consistent during consolidation.
    /// The invariant total_sol_fees_collected = total_fees_consolidated + current_pending_fees
    /// is maintained throughout the operation.
    pub fn reset_consolidation_counters(&mut self, timestamp: i64) {
        // Calculate pending fees before any changes using the accurate mathematical relationship
        let pending_fees = self.pending_sol_fees();
        
        // **ATOMIC CONSOLIDATION UPDATE**: 
        // Move pending fees from "collected" to "consolidated" state
        // NOTE: total_sol_fees_collected does NOT change - it's the lifetime total
        self.total_fees_consolidated += pending_fees;
        
        // Reset collected fees (operation counts are calculated from these)
        self.collected_liquidity_fees = 0;
        self.collected_swap_contract_fees = 0;
        
        // Update consolidation metadata
        self.last_consolidation_timestamp = timestamp;
        self.total_consolidations += 1;
        
        // **INVARIANT VERIFICATION**: Ensure consistency after consolidation
        debug_assert_eq!(
            self.pending_sol_fees(),
            0,
            "Pending fees should be zero after consolidation"
        );
        debug_assert_eq!(
            self.total_sol_fees_collected,
            self.total_fees_consolidated,
            "After consolidation, total collected should equal total consolidated"
        );
    }
    
    /// **NEW: Validates internal consistency of fee tracking**
    /// 
    /// This method can be called periodically to ensure that race conditions
    /// or bugs haven't corrupted the fee tracking state.
    pub fn validate_fee_consistency(&self) -> Result<(), &'static str> {
        // Verify the mathematical relationship: pending = total - consolidated
        let calculated_pending = self.total_sol_fees_collected.saturating_sub(self.total_fees_consolidated);
        let actual_pending = self.pending_sol_fees();
        
        if calculated_pending != actual_pending {
            return Err("Pending SOL fees calculation inconsistency");
        }
        
        // Verify individual pending fees sum matches the mathematical pending
        let individual_sum = self.collected_liquidity_fees + 
                           self.collected_swap_contract_fees;
        
        if actual_pending != individual_sum {
            return Err("Individual pending fees don't match calculated pending fees");
        }
        
        // Verify no arithmetic overflow conditions
        let max_safe_value = u64::MAX / 2; // Conservative check
        if self.total_sol_fees_collected > max_safe_value {
            return Err("Total SOL fees approaching overflow risk");
        }
        
        // Verify consolidated fees don't exceed total fees
        if self.total_fees_consolidated > self.total_sol_fees_collected {
            return Err("Consolidated fees exceed total collected fees");
        }
        
        Ok(())
    }
    
    /// **NEW: Calculate available balance for consolidation (respecting rent exemption)**
    /// 
    /// This method calculates how much SOL can be safely consolidated from a pool state
    /// without violating rent exemption requirements. It considers both the rent exempt
    /// minimum and the actual pending fees.
    /// 
    /// # Arguments
    /// * `current_account_balance` - Current lamports balance of the pool state account
    /// * `rent_exempt_minimum` - Minimum balance required for rent exemption
    /// 
    /// # Returns
    /// * `u64` - Amount of SOL that can be safely consolidated (in lamports)
    /// 
    /// # Safety
    /// This function ensures that consolidation never reduces the pool state balance
    /// below the rent exempt minimum, preventing account closure due to insufficient funds.
    pub fn calculate_available_for_consolidation(
        &self,
        current_account_balance: u64,
        rent_exempt_minimum: u64,
    ) -> u64 {
        // Calculate pending fees awaiting consolidation
        let pending_fees = self.pending_sol_fees();
        
        // Calculate available balance above rent exempt minimum
        let available_above_rent_exempt = current_account_balance.saturating_sub(rent_exempt_minimum);
        
        // Return the minimum of available balance and pending fees
        // This ensures we never:
        // 1. Take more than what's available above rent exempt minimum
        // 2. Take more than what's actually owed in pending fees
        std::cmp::min(available_above_rent_exempt, pending_fees)
    }
    
    /// **NEW: Validate consolidation is safe (respecting rent exemption)**
    /// 
    /// This method validates that a proposed consolidation amount is safe and won't
    /// violate rent exemption requirements or exceed pending fees.
    /// 
    /// # Arguments
    /// * `proposed_consolidation_amount` - Amount of SOL proposed for consolidation
    /// * `current_account_balance` - Current lamports balance of the pool state account
    /// * `rent_exempt_minimum` - Minimum balance required for rent exemption
    /// 
    /// # Returns
    /// * `Result<(), &'static str>` - Ok if consolidation is safe, error message if not
    /// 
    /// # Safety
    /// This function provides comprehensive validation to prevent:
    /// - Account closure due to insufficient rent exempt balance
    /// - Over-consolidation beyond pending fees
    /// - Arithmetic underflow in account balance
    pub fn validate_consolidation_safety(
        &self,
        proposed_consolidation_amount: u64,
        current_account_balance: u64,
        rent_exempt_minimum: u64,
    ) -> Result<(), &'static str> {
        // Check if account would have sufficient balance after consolidation
        if current_account_balance < proposed_consolidation_amount {
            return Err("Consolidation amount exceeds current account balance");
        }
        
        let balance_after_consolidation = current_account_balance - proposed_consolidation_amount;
        if balance_after_consolidation < rent_exempt_minimum {
            return Err("Consolidation would reduce balance below rent exempt minimum");
        }
        
        // Check if consolidation amount exceeds pending fees
        let pending_fees = self.pending_sol_fees();
        if proposed_consolidation_amount > pending_fees {
            return Err("Consolidation amount exceeds pending fees");
        }
        
        // Check for edge cases
        if proposed_consolidation_amount == 0 {
            return Err("Consolidation amount cannot be zero");
        }
        
        Ok(())
    }
} 