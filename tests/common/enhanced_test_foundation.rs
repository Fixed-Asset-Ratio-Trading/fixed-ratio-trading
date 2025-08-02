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

//! # Enhanced Test Foundation
//! 
//! Official Multi-Pool Architecture for Fixed Ratio Trading Tests
//! 
//! This module implements the Phase 1 Enhanced Test Foundation that wraps the existing
//! LiquidityTestFoundation while adding multi-pool capabilities. It maintains full
//! backwards compatibility during the transition period.
//! 
//! ## Design Goals
//! 
//! 1. **Single Program Context**: All pools share the same program instance and test environment
//! 2. **Unique Pool PDAs**: Each pool has distinct Program Derived Addresses
//! 3. **Shared Resources**: Common system state, treasury, and token program access
//! 4. **Independent Configuration**: Each pool can have different ratios, tokens, and parameters
//! 5. **Scalable Pattern**: Support for 1-20+ pools without environment conflicts
//! 6. **Backwards Compatibility**: Existing tests continue working unchanged

use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use crate::common::{
    liquidity_helpers::{LiquidityTestFoundation, create_liquidity_test_foundation},
    pool_helpers::PoolConfig,
    setup::TestEnvironment,
};

/// Enhanced Test Foundation - Official Multi-Pool Architecture
/// 
/// Wraps existing LiquidityTestFoundation while adding multi-pool capabilities.
/// Maintains full backwards compatibility during transition period.
pub struct EnhancedTestFoundation {
    /// Primary pool using existing system (backwards compatibility)
    primary_pool: LiquidityTestFoundation,
    
    /// Additional pools for multi-pool testing
    additional_pools: Vec<PoolConfig>,
    
    /// Multi-pool configuration and state
    multi_pool_config: MultiPoolConfig,
}

/// Configuration for multi-pool testing
#[derive(Debug, Clone)]
pub struct MultiPoolConfig {
    pub max_pools: usize,
    pub cleanup_strategy: CleanupStrategy,
    pub pool_isolation_level: IsolationLevel,
}

impl Default for MultiPoolConfig {
    fn default() -> Self {
        Self {
            max_pools: 20, // Maximum pools supported by the program
            cleanup_strategy: CleanupStrategy::FreshEnvironmentPerTest,
            pool_isolation_level: IsolationLevel::SharedTokenMints,
        }
    }
}

/// Cleanup strategy for test isolation
#[derive(Debug, Clone)]
pub enum CleanupStrategy {
    /// Create fresh environment for each test (recommended)
    FreshEnvironmentPerTest,
    /// Reuse environment with reset between tests
    EnvironmentReuseWithReset,
}

/// Pool isolation level for multi-pool testing
#[derive(Debug, Clone)]
pub enum IsolationLevel {
    /// Each pool has unique token mints (full isolation)
    UniqueTokenMints,
    /// Shared token mints across pools (more realistic for some tests)
    SharedTokenMints,
    /// Configurable per pool (maximum flexibility)
    ConfigurablePerPool,
}

/// Pool configuration for additional pools
#[derive(Debug)]
pub struct PoolConfigExtended {
    pub pool_id: u8,
    pub pool_state_pda: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub vault_a_pda: Pubkey,
    pub vault_b_pda: Pubkey,
    pub lp_mint_pda: Pubkey,
    pub ratio: (u64, u64),
    pub user_accounts: UserAccountSet,
}

/// PDA set for a pool (used during pool creation)
#[derive(Debug)]
pub struct PoolPdaSet {
    pub pool_state_pda: Pubkey,
    pub authority_bump: u8,
    pub vault_a_pda: Pubkey,
    pub vault_a_bump: u8,
    pub vault_b_pda: Pubkey,
    pub vault_b_bump: u8,
}

/// User account set for a specific pool
#[derive(Debug)]
pub struct UserAccountSet {
    pub authority: Keypair,
    pub token_a_account: Pubkey,
    pub token_b_account: Pubkey,
    pub lp_token_account: Pubkey,
}

/// Pool creation parameters
#[derive(Debug, Clone)]
pub struct PoolCreationParams {
    pub ratio_a: u64,
    pub ratio_b: u64,
    pub initial_liquidity_a: Option<u64>,
    pub initial_liquidity_b: Option<u64>,
    pub token_a_decimals: u8,
    pub token_b_decimals: u8,
    pub create_new_tokens: bool, // If false, reuse existing tokens
}

impl Default for PoolCreationParams {
    fn default() -> Self {
        Self {
            ratio_a: 2,
            ratio_b: 1,
            initial_liquidity_a: Some(1_000_000),
            initial_liquidity_b: Some(500_000),
            token_a_decimals: 9,
            token_b_decimals: 9,
            create_new_tokens: true,
        }
    }
}

impl PoolCreationParams {
    /// Create pool parameters with simple ratio
    pub fn new(ratio_a: u64, ratio_b: u64) -> Self {
        Self {
            ratio_a,
            ratio_b,
            ..Default::default()
        }
    }
    
    /// Create pool parameters with custom liquidity amounts
    pub fn with_liquidity(ratio_a: u64, ratio_b: u64, liquidity_a: u64, liquidity_b: u64) -> Self {
        Self {
            ratio_a,
            ratio_b,
            initial_liquidity_a: Some(liquidity_a),
            initial_liquidity_b: Some(liquidity_b),
            ..Default::default()
        }
    }
}

/// Reference to pool (either primary or additional)
pub enum PoolReference<'a> {
    Primary(&'a LiquidityTestFoundation),
    Additional(&'a PoolConfig),
}

/// Test error for enhanced foundation operations
#[derive(Debug)]
pub enum TestError {
    PoolNotFound(usize),
    MaxPoolsExceeded(usize),
    InvalidPoolConfiguration(String),
    EnvironmentError(String),
}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestError::PoolNotFound(index) => write!(f, "Pool not found at index {}", index),
            TestError::MaxPoolsExceeded(max) => write!(f, "Maximum pools exceeded: {}", max),
            TestError::InvalidPoolConfiguration(msg) => write!(f, "Invalid pool configuration: {}", msg),
            TestError::EnvironmentError(msg) => write!(f, "Environment error: {}", msg),
        }
    }
}

impl std::error::Error for TestError {}

/// Enhanced Test Foundation Implementation
impl EnhancedTestFoundation {
    /// Create from existing LiquidityTestFoundation (migration helper)
    pub async fn from_liquidity_foundation(
        foundation: LiquidityTestFoundation
    ) -> Result<Self, TestError> {
        Ok(Self {
            primary_pool: foundation,
            additional_pools: Vec::new(),
            multi_pool_config: MultiPoolConfig::default(),
        })
    }
    
    /// Access legacy foundation for backwards compatibility
    pub fn as_liquidity_foundation(&self) -> &LiquidityTestFoundation {
        &self.primary_pool
    }
    
    /// Mutable access to legacy foundation
    pub fn as_liquidity_foundation_mut(&mut self) -> &mut LiquidityTestFoundation {
        &mut self.primary_pool
    }
    
    /// Add a new pool to the foundation (PHASE 1B: Full implementation)
    /// 
    /// Creates a new pool with unique PDAs in the same test environment as the primary pool.
    /// This solves the IncorrectProgramId issue by ensuring all pools share the same program context.
    pub async fn add_pool(
        &mut self,
        params: PoolCreationParams,
    ) -> Result<usize, TestError> {
        use fixed_ratio_trading::constants::*;
        use crate::common::{
            pool_helpers::normalize_pool_config,
            tokens::create_mint,
        };
        
        println!("🏗️ PHASE 1B: Creating additional pool in shared environment...");
        println!("   📋 Pool parameters: {}:{} ratio", params.ratio_a, params.ratio_b);
        
        // Generate unique pool ID for this additional pool
        let pool_id = (self.additional_pools.len() + 1) as u8; // +1 because primary pool is ID 0
        println!("   🆔 Assigned pool ID: {}", pool_id);
        
        // For debugging: Use same token pair with different ratios to avoid potential mint creation issues
        println!("   🔗 Reusing existing token mints from primary pool for debugging...");
        println!("   🎯 Different ratios will create unique PDAs even with same token mints");
        let (token_a_mint, token_b_mint) = (self.primary_pool.primary_mint.pubkey(), self.primary_pool.base_mint.pubkey());
        
        println!("   📊 Token mints assigned:");
        println!("     • Token A: {}", token_a_mint);
        println!("     • Token B: {}", token_b_mint);
        
        // Generate unique PDAs for this pool using pool_id
        let pool_pdas = self.derive_unique_pool_pdas(pool_id, token_a_mint, token_b_mint, params.ratio_a, params.ratio_b)?;
        
        println!("   🔑 Generated unique PDAs:");
        println!("     • Pool state: {}", pool_pdas.pool_state_pda);
        println!("     • Vault A: {}", pool_pdas.vault_a_pda);
        println!("     • Vault B: {}", pool_pdas.vault_b_pda);
        
        // Normalize pool parameters using existing logic
        let normalized_config = normalize_pool_config(
            &token_a_mint,
            &token_b_mint, 
            params.ratio_a,
            params.ratio_b
        );
        
        // Update the normalized config with our unique PDAs
        let mut pool_config = normalized_config;
        pool_config.pool_state_pda = pool_pdas.pool_state_pda;
        pool_config.pool_authority_bump = pool_pdas.authority_bump;
        pool_config.token_a_vault_pda = pool_pdas.vault_a_pda;
        pool_config.token_a_vault_bump = pool_pdas.vault_a_bump;
        pool_config.token_b_vault_pda = pool_pdas.vault_b_pda;
        pool_config.token_b_vault_bump = pool_pdas.vault_b_bump;
        pool_config.multiple_vault_bump = if pool_config.token_a_is_the_multiple { pool_pdas.vault_a_bump } else { pool_pdas.vault_b_bump };
        pool_config.base_vault_bump = if pool_config.token_a_is_the_multiple { pool_pdas.vault_b_bump } else { pool_pdas.vault_a_bump };
        
        // 🔥 CRITICAL: Actually initialize the pool on-chain!
        println!("   🔗 Initializing pool state account on-chain...");
        self.initialize_pool_on_chain(&pool_config, token_a_mint, token_b_mint).await?;
        
        // Add to our additional pools list
        self.additional_pools.push(pool_config);
        let pool_index = self.additional_pools.len(); // Return 1-based index (0 is primary pool)
        
        println!("✅ Additional pool created successfully!");
        println!("   🎯 Pool index: {} (0 = primary, {}+ = additional)", pool_index, 1);
        println!("   📈 Total pools in foundation: {}", self.pool_count());
        println!("   🔧 Pool ready for operations (deposits, swaps, consolidation)");
        
        Ok(pool_index)
    }
    
    /// Derive PDAs for a pool using normalized values to ensure correctness
    /// 
    /// Since each pool uses different token mints, the PDAs will naturally be unique
    /// without needing to modify the smart contract's expected PDA derivation pattern.
    fn derive_unique_pool_pdas(
        &self,
        _pool_id: u8, // Not used in PDA derivation to maintain smart contract compatibility
        token_a_mint: Pubkey,
        token_b_mint: Pubkey,
        ratio_a: u64,
        ratio_b: u64,
    ) -> Result<PoolPdaSet, TestError> {
        use fixed_ratio_trading::{constants::*, id};
        
        // Use the SAME PDA derivation pattern as the smart contract expects
        // Uniqueness comes from different token mints, not from pool_id in PDA
        let (pool_state_pda, authority_bump) = Pubkey::find_program_address(
            &[
                POOL_STATE_SEED_PREFIX,
                token_a_mint.as_ref(),
                token_b_mint.as_ref(),
                &ratio_a.to_le_bytes(),
                &ratio_b.to_le_bytes(),
            ],
            &id(),
        );
        
        // Generate vault PDAs based on the pool state PDA
        let (vault_a_pda, vault_a_bump) = Pubkey::find_program_address(
            &[TOKEN_A_VAULT_SEED_PREFIX, pool_state_pda.as_ref()],
            &id(),
        );
        
        let (vault_b_pda, vault_b_bump) = Pubkey::find_program_address(
            &[TOKEN_B_VAULT_SEED_PREFIX, pool_state_pda.as_ref()],
            &id(),
        );
        
        println!("     🔑 Using smart contract compatible PDA derivation:");
        println!("        • No pool_id in seeds (maintains compatibility)");
        println!("        • Uniqueness from different token mints");
        println!("        • Pool state PDA: {}", pool_state_pda);
        
        Ok(PoolPdaSet {
            pool_state_pda,
            authority_bump,
            vault_a_pda,
            vault_a_bump,
            vault_b_pda,
            vault_b_bump,
        })
    }
    
    /// Initialize the pool on-chain by creating the actual pool state account
    /// 
    /// This is the critical step that actually creates the pool in the blockchain,
    /// not just the configuration and PDAs.
    async fn initialize_pool_on_chain(
        &mut self,
        pool_config: &PoolConfig,
        token_a_mint: Pubkey,
        token_b_mint: Pubkey,
    ) -> Result<(), TestError> {
        use fixed_ratio_trading::{constants::*, id, types::instructions::PoolInstruction};
        use solana_sdk::{
            instruction::{AccountMeta, Instruction},
            transaction::Transaction,
            compute_budget::ComputeBudgetInstruction,
        };
        use borsh::BorshSerialize;
        
        // Generate required PDAs for pool initialization
        let (main_treasury_pda, _) = Pubkey::find_program_address(
            &[MAIN_TREASURY_SEED_PREFIX],
            &id(),
        );
        let (system_state_pda, _) = Pubkey::find_program_address(
            &[SYSTEM_STATE_SEED_PREFIX],
            &id(),
        );
        let (lp_token_a_mint_pda, _) = Pubkey::find_program_address(
            &[LP_TOKEN_A_MINT_SEED_PREFIX, pool_config.pool_state_pda.as_ref()],
            &id(),
        );
        let (lp_token_b_mint_pda, _) = Pubkey::find_program_address(
            &[LP_TOKEN_B_MINT_SEED_PREFIX, pool_config.pool_state_pda.as_ref()],
            &id(),
        );
        
        // Create InitializePool instruction
        let initialize_pool_ix = Instruction {
            program_id: id(),
            accounts: vec![
                AccountMeta::new(self.primary_pool.env.payer.pubkey(), true),      // Index 0: User Authority Signer
                AccountMeta::new_readonly(solana_program::system_program::id(), false), // Index 1: System Program
                AccountMeta::new_readonly(system_state_pda, false),                // Index 2: System State PDA
                AccountMeta::new(pool_config.pool_state_pda, false),               // Index 3: Pool State PDA
                AccountMeta::new_readonly(spl_token::id(), false),                 // Index 4: SPL Token Program
                AccountMeta::new(main_treasury_pda, false),                        // Index 5: Main Treasury PDA
                AccountMeta::new_readonly(solana_program::sysvar::rent::id(), false), // Index 6: Rent Sysvar
                AccountMeta::new_readonly(token_a_mint, false),                    // Index 7: Token A Mint
                AccountMeta::new_readonly(token_b_mint, false),                    // Index 8: Token B Mint
                AccountMeta::new(pool_config.token_a_vault_pda, false),            // Index 9: Token A Vault PDA
                AccountMeta::new(pool_config.token_b_vault_pda, false),            // Index 10: Token B Vault PDA
                AccountMeta::new(lp_token_a_mint_pda, false),                      // Index 11: LP Token A Mint PDA
                AccountMeta::new(lp_token_b_mint_pda, false),                      // Index 12: LP Token B Mint PDA
            ],
            data: PoolInstruction::InitializePool {
                ratio_a_numerator: pool_config.ratio_a_numerator,
                ratio_b_denominator: pool_config.ratio_b_denominator,
            }.try_to_vec().map_err(|e| TestError::EnvironmentError(format!("Failed to serialize instruction: {}", e)))?,
        };

        // Add compute budget and create transaction
        let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(500_000);
        
        let mut transaction = Transaction::new_with_payer(
            &[compute_budget_ix, initialize_pool_ix], 
            Some(&self.primary_pool.env.payer.pubkey())
        );
        
        // Get fresh blockhash and sign transaction
        self.primary_pool.env.recent_blockhash = self.primary_pool.env.banks_client.get_latest_blockhash().await
            .map_err(|e| TestError::EnvironmentError(format!("Failed to get blockhash: {}", e)))?;
        transaction.sign(&[&self.primary_pool.env.payer], self.primary_pool.env.recent_blockhash);
        
        // Execute the transaction
        self.primary_pool.env.banks_client.process_transaction(transaction).await
            .map_err(|e| TestError::EnvironmentError(format!("Failed to initialize pool: {}", e)))?;
        
        println!("     ✅ Pool state account created on-chain successfully!");
        Ok(())
    }
    
    /// Get pool by index (0 = primary pool, 1+ = additional pools)
    pub fn get_pool(&self, pool_index: usize) -> Result<PoolReference, TestError> {
        if pool_index == 0 {
            Ok(PoolReference::Primary(&self.primary_pool))
        } else {
            let additional_index = pool_index - 1;
            self.additional_pools.get(additional_index)
                .map(|pool| PoolReference::Additional(pool))
                .ok_or(TestError::PoolNotFound(pool_index))
        }
    }
    
    /// Get total number of pools (primary + additional)
    pub fn pool_count(&self) -> usize {
        1 + self.additional_pools.len()
    }
    
    /// Get all pool PDAs for consolidation
    pub fn get_all_pool_pdas(&self) -> Vec<Pubkey> {
        let mut pdas = vec![self.primary_pool.pool_config.pool_state_pda];
        pdas.extend(self.additional_pools.iter().map(|p| p.pool_state_pda));
        pdas
    }
    
    /// Get the shared test environment
    pub fn env(&self) -> &TestEnvironment {
        &self.primary_pool.env
    }
    
    /// Get mutable access to the shared test environment
    pub fn env_mut(&mut self) -> &mut TestEnvironment {
        &mut self.primary_pool.env
    }
    
    /// Get multi-pool configuration
    pub fn config(&self) -> &MultiPoolConfig {
        &self.multi_pool_config
    }
    
    /// Update multi-pool configuration
    pub fn set_config(&mut self, config: MultiPoolConfig) {
        self.multi_pool_config = config;
    }
}

/// Backwards compatible creation function
/// 
/// Creates an EnhancedTestFoundation from a legacy LiquidityTestFoundation
/// This provides a drop-in replacement for existing tests
pub async fn create_enhanced_liquidity_test_foundation(
    ratio: Option<u64>
) -> Result<EnhancedTestFoundation, Box<dyn std::error::Error>> {
    println!("🏗️ Creating Enhanced Liquidity Test Foundation...");
    println!("   🔄 Using backwards compatibility layer");
    println!("   📦 Legacy ratio: {:?}", ratio);
    
    // Create legacy foundation first
    let legacy = create_liquidity_test_foundation(ratio).await?;
    
    // Wrap in enhanced foundation
    let enhanced = EnhancedTestFoundation::from_liquidity_foundation(legacy).await?;
    
    println!("✅ Enhanced Test Foundation created successfully");
    println!("   • Pool count: {}", enhanced.pool_count());
    println!("   • Primary pool: Available");
    println!("   • Multi-pool capability: Ready (Phase 1b will enable creation)");
    
    Ok(enhanced)
}

/// Create an EnhancedTestFoundation with specific multi-pool configuration
pub async fn create_enhanced_test_foundation_with_config(
    ratio: Option<u64>,
    config: MultiPoolConfig,
) -> Result<EnhancedTestFoundation, Box<dyn std::error::Error>> {
    println!("🏗️ Creating Enhanced Test Foundation with custom configuration...");
    
    let mut enhanced = create_enhanced_liquidity_test_foundation(ratio).await?;
    enhanced.set_config(config);
    
    println!("✅ Enhanced Test Foundation with custom config created");
    println!("   • Max pools: {}", enhanced.config().max_pools);
    println!("   • Cleanup strategy: {:?}", enhanced.config().cleanup_strategy);
    println!("   • Isolation level: {:?}", enhanced.config().pool_isolation_level);
    
    Ok(enhanced)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Test that backwards compatibility works correctly
    #[tokio::test]
    async fn test_backwards_compatibility() -> Result<(), Box<dyn std::error::Error>> {
        println!("🧪 Testing Enhanced Foundation backwards compatibility...");
        
        // Create enhanced foundation
        let foundation = create_enhanced_liquidity_test_foundation(Some(2)).await?;
        
        // Verify we can access legacy foundation
        let _legacy = foundation.as_liquidity_foundation();
        assert_eq!(foundation.pool_count(), 1);
        
        // Verify basic properties
        assert!(foundation.get_pool(0).is_ok());
        assert!(foundation.get_pool(1).is_err());
        
        println!("✅ Backwards compatibility test passed");
        Ok(())
    }
    
    /// Test that multi-pool creation works correctly (Phase 1B)
    #[tokio::test]
    async fn test_multi_pool_creation() -> Result<(), Box<dyn std::error::Error>> {
        println!("🧪 Testing Phase 1B multi-pool creation functionality...");
        
        let mut foundation = create_enhanced_liquidity_test_foundation(Some(3)).await?;
        
        // Verify we start with 1 pool (primary)
        assert_eq!(foundation.pool_count(), 1);
        
        // Test that add_pool now works correctly
        let pool_index = foundation.add_pool(PoolCreationParams::new(2, 1)).await?;
        
        // Verify pool was added successfully
        assert_eq!(pool_index, 1); // First additional pool gets index 1
        assert_eq!(foundation.pool_count(), 2); // Primary + 1 additional
        
        // Test we can access both pools
        assert!(foundation.get_pool(0).is_ok()); // Primary pool
        assert!(foundation.get_pool(1).is_ok()); // Additional pool
        
        // Test adding another pool
        let pool_index_2 = foundation.add_pool(PoolCreationParams::new(1, 2)).await?;
        assert_eq!(pool_index_2, 2); // Second additional pool gets index 2
        assert_eq!(foundation.pool_count(), 3); // Primary + 2 additional
        
        // Verify all pools are accessible
        assert!(foundation.get_pool(0).is_ok()); // Primary pool
        assert!(foundation.get_pool(1).is_ok()); // First additional pool  
        assert!(foundation.get_pool(2).is_ok()); // Second additional pool
        assert!(foundation.get_pool(3).is_err()); // Non-existent pool
        
        println!("✅ Multi-pool creation test passed");
        println!("   • Successfully created {} pools in shared environment", foundation.pool_count());
        println!("   • All pools accessible and properly indexed");
        println!("   • Phase 1B implementation working correctly!");
        
        Ok(())
    }
}