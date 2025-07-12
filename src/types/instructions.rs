//! Pool Instructions
//! 
//! This module contains all the instruction definitions for the Solana Trading Pool Program.
//! Instructions define the operations that can be performed on the pool.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// All supported instructions for the Solana Trading Pool Program.
/// 
/// This enum defines every operation that can be performed on the pool,
/// from initialization and liquidity management to owner-only operations.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum PoolInstruction {

    /// **CRITICAL**: Program-level initialization (MUST BE CALLED FIRST)
    /// 
    /// This instruction must be called once when the program is first deployed.
    /// It creates all the system-level infrastructure that individual pools depend on.
    /// 
    /// # What it creates:
    /// 1. SystemState PDA - Global pause controls and system authority
    /// 2. MainTreasury PDA - Pool creation and liquidity operation fees
    /// 3. SwapTreasury PDA - Regular swap fees (high frequency)
    /// 4. HftTreasury PDA - HFT swap fees (high frequency)
    /// 
    /// # When to call:
    /// - ONCE during initial program deployment
    /// - Before any pools can be created
    /// - Before any other program operations
    /// 
    /// # After this initialization:
    /// - Pool creation will have treasury PDAs to send fees to
    /// - System pause functionality will be available
    /// - Treasury management operations will work
    /// - All subsequent operations will assume these PDAs exist
    /// 
    /// # Arguments:
    /// - System authority is provided via accounts[0] (must be signer)
    /// - The signer's pubkey will control system-wide operations
    ///   (pause/unpause system, withdraw treasury funds, etc.)
    /// 
    /// # Security:
    /// - Can only be called once (fails if SystemState already exists)
    /// - Creates all accounts as PDAs owned by the program
    /// - Sets up proper rent exemption for all accounts
    InitializeProgram {
        // No fields needed - system authority comes from accounts[0]
    },

    /// **RECOMMENDED**: Single-instruction pool initialization
    /// 
    /// This instruction provides a single, atomic operation for pool creation.
    /// 
    /// # What it does:
    /// - Creates Pool State PDA with correct size allocation
    /// - Creates LP token mints and transfers authority to pool
    /// - Creates token vault PDAs and initializes them
    /// - Initializes pool state data with all configuration
    /// - Transfers registration fees
    /// 
    /// # Benefits:
    /// - Atomic operation (all-or-nothing)
    /// - Simpler client integration
    /// - Better user experience
    /// - Eliminates workaround complexity
    /// 
    /// # Arguments:
    /// - `ratio_a_numerator`: Token A base units (replaces multiple_per_base)
    /// - `ratio_b_denominator`: Token B base units (was hardcoded to 1, now configurable)
    /// 
    /// # Note:
    /// - `one_to_many_ratio` is automatically determined by the contract based on the ratio values
    /// - Display preferences are handled by individual applications, not the contract
    /// - Bump seeds for all PDAs are derived internally using `find_program_address`
    InitializePool {
        ratio_a_numerator: u64,
        ratio_b_denominator: u64,
    },

    /// Standard deposit operation for adding liquidity to the pool
    /// 
    /// This instruction enforces a strict 1:1 ratio between deposited tokens and LP tokens.
    /// If the exact 1:1 ratio cannot be achieved, the entire transaction is rolled back.
    /// All fees go to the internal pool PDA for centralized management.
    /// 
    /// # Arguments:
    /// - `deposit_token_mint`: Token mint being deposited (must match pool's Token A or Token B)
    /// - `amount`: Amount of tokens to deposit (will receive exactly this many LP tokens)
    /// 
    /// # Guarantees:
    /// - Strict 1:1 ratio: deposit N tokens → receive exactly N LP tokens
    /// - Transaction fails if 1:1 ratio cannot be maintained
    /// - LP tokens have same decimal precision as underlying tokens
    /// - Unlimited LP token supply (no supply caps)
    /// - Only the contract can mint LP tokens
    Deposit {
        deposit_token_mint: Pubkey,
        amount: u64,
    },
    
    /// Withdraw liquidity from the pool by burning LP tokens
    Withdraw {
        withdraw_token_mint: Pubkey,
        lp_amount_to_burn: u64,
    },
    
    /// Swap tokens at fixed ratio
    /// 
    /// Exchanges tokens using the pool's predetermined fixed exchange rate.
    /// The output amount is deterministically calculated based on the ratio - 
    /// either you get the exact calculated amount or the transaction fails.
    /// No slippage protection needed since exchange rates are constant.
    Swap {
        input_token_mint: Pubkey,
        amount_in: u64,
    },

    /// **HFT OPTIMIZED SWAP**: High-frequency trading optimized version of swap
    /// 
    /// This instruction provides the same functionality as the standard Swap instruction
    /// but with significant compute unit (CU) optimizations for high-frequency trading:
    /// 
    /// **PHASE 6: ULTRA-OPTIMIZED PERFORMANCE**
    /// - Eliminated rent checks entirely (saves ~500-850 CUs)
    /// - Removed redundant token mint accounts (saves ~50-100 CUs)
    /// - Reduced account count: 14 → 10 accounts (saves ~70-140 CUs)
    /// - Single serialization (saves ~800-1200 CUs)
    /// - Reduced logging overhead (saves ~500-800 CUs)
    /// - Batched validations (saves ~200-400 CUs)
    /// - Early failure detection (saves ~50-150 CUs)
    /// 
    /// **Total CU Savings: 2,170-3,640 CUs (30-35% reduction)**
    /// 
    /// All essential security features are preserved. Pool accounts are structurally
    /// protected from rent exemption loss, making rent checks unnecessary.
    /// Output amounts are identical to the standard Swap instruction.
    /// 
    /// # Arguments:
    /// - `input_token_mint`: Token mint being swapped in
    /// - `amount_in`: Amount of input tokens to swap
    /// 
    /// # When to Use:
    /// - Production HFT environments where CU efficiency is critical
    /// - High-volume trading operations
    /// - Arbitrage and market making bots
    /// - When every compute unit matters for profitability
    /// 
    /// # Safety:
    /// Rent checks are eliminated because pool accounts are structurally protected
    /// from rent exemption loss through the protocol's design.
    SwapHftOptimized {
        input_token_mint: Pubkey,
        amount_in: u64,
    },
    
    // Pool owner management instructions removed for governance control
    // Fee management and security controls are now handled through:
    // - System authority (treasury withdrawals, system pause/unpause)
    // - Governance protocols (token fees, pool-specific controls)
    
    /// Get pool state PDA address for given tokens and ratio
    /// Useful for clients to derive addresses before calling other instructions
    GetPoolStatePDA {
        multiple_token_mint: Pubkey,
        base_token_mint: Pubkey,
        multiple_per_base: u64,
    },
    
    /// Returns the Token Vault PDA addresses for a given pool
    /// Helps clients prepare account lists for transactions
    GetTokenVaultPDAs {
        pool_state_pda: Pubkey,
    },
    
    /// Returns comprehensive pool state information in a structured format
    /// Ideal for testing, debugging, and frontend integration
    GetPoolInfo {
        // No parameters needed - reads from pool state account
    },
    
    /// Get current pool pause status (publicly readable)
    /// Returns swap pause status, deposit/withdrawal status, and pause details
    /// Distinguishes between system-wide pause and pool-specific swap pause
    GetPoolPauseStatus {
        // No parameters needed - reads from pool state account
    },
    
    /// Returns detailed liquidity information for both tokens
    /// Useful for calculating exchange rates and available liquidity
    GetLiquidityInfo {
        // No parameters needed - reads from pool state account  
    },
    
    /// **VIEW INSTRUCTION**: Get fee information including rates and collected amounts
    GetFeeInfo {
        // No fields needed - reads from pool state
    },
    
    /// **VIEW INSTRUCTION**: Get pool state PDA SOL balance
    GetPoolSolBalance {
        // No fields needed - reads from pool state account balance
    },
    
    /// Pause the entire system - blocks all operations except unpause (system authority only)
    /// Takes precedence over all pool-specific pause states
    /// 
    /// # Storage Optimization:
    /// Uses standardized reason codes for efficient storage (42 bytes vs 245 bytes)
    /// Client applications map codes to human-readable text
    /// 
    /// # Standard Pause Codes:
    /// - 0: No pause active (default state)
    /// - 1: Temporary consolidation of funds
    /// - 2: Contract upgrade in progress  
    /// - 3: Critical security issue detected
    /// - 4: Routine maintenance and debugging
    /// - 5: Emergency halt due to unexpected behavior
    /// - 255: Custom reason (see external documentation)
    PauseSystem {
        /// Standardized pause reason code for efficient storage
        reason_code: u8,
    },
    
    /// Unpause the entire system - allows all operations to resume (system authority only)
    /// Clears the system pause state completely
    UnpauseSystem,
    
    /// Get the smart contract version information
    /// Returns version data including contract version and schema version
    /// No accounts required - returns constant version information
    GetVersion,
    
    /// **TREASURY MANAGEMENT**: Withdraw contract fees from main treasury (system authority only)
    /// 
    /// Allows the system authority to withdraw accumulated contract fees from the main treasury.
    /// This is the only way to extract SOL fees collected by the protocol.
    /// 
    /// # Requirements:
    /// - Caller must be the system authority (same as system pause authority)
    /// - Main treasury must have sufficient balance above rent-exempt minimum
    /// - Amount must not exceed available balance
    /// 
    /// # Arguments:
    /// - `amount`: Amount of SOL to withdraw in lamports (0 = withdraw all available)
    WithdrawTreasuryFees {
        amount: u64,
    },
    
    /// **PHASE 3: REAL-TIME TREASURY INFORMATION**
    /// 
    /// Returns comprehensive real-time information about the centralized treasury including:
    /// - Current balance and total withdrawn
    /// - Real-time fee counts by category (no consolidation needed)
    /// - Total fees collected by type
    /// - Last update timestamp
    /// - Analytics methods (total operations, average fee, etc.)
    /// 
    /// # Phase 3 Benefits:
    /// - No consolidation needed (real-time data)
    /// - Single source of truth
    /// - No race conditions
    /// - Simplified architecture
    GetTreasuryInfo {
        // No parameters needed - reads main treasury state directly
    },
    
} 