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
    /// - `admin_authority`: The pubkey that will become the admin authority
    /// - System authority is provided via accounts[0] (must be signer, pays for account creation)
    /// - The admin_authority will control system-wide operations
    ///   (pause/unpause system, withdraw treasury funds, etc.)
    /// 
    /// # Security:
    /// - Can only be called once (fails if SystemState already exists)
    /// - Creates all accounts as PDAs owned by the program
    /// - Sets up proper rent exemption for all accounts
    /// - Admin authority is configurable and can be different from the deployer
    InitializeProgram {
        admin_authority: Pubkey,
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
        flags: u8,
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
    /// - `pool_id`: Expected Pool ID (PDA address) for security validation
    /// 
    /// # Security:
    /// - Pool ID validation prevents PDA bypass attacks
    /// - Client must specify exact pool they intend to deposit to
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
        pool_id: Pubkey,
    },
    
    /// Withdraw liquidity from the pool by burning LP tokens
    /// 
    /// # Arguments:
    /// - `withdraw_token_mint`: Token mint to withdraw (must match pool's Token A or Token B)
    /// - `lp_amount_to_burn`: Amount of LP tokens to burn for withdrawal
    /// - `pool_id`: Expected Pool ID (PDA address) for security validation
    /// 
    /// # Security:
    /// - Pool ID validation prevents PDA bypass attacks
    /// - Client must specify exact pool they intend to withdraw from
    Withdraw {
        withdraw_token_mint: Pubkey,
        lp_amount_to_burn: u64,
        pool_id: Pubkey,
    },
    
    /// Swap tokens at fixed ratio
    /// 
    /// Exchanges tokens using the pool's predetermined fixed exchange rate.
    /// The output amount is deterministically calculated based on the ratio - 
    /// either you get the exact calculated amount or the transaction fails.
    /// No slippage protection needed since exchange rates are constant.
    /// 
    /// # Arguments:
    /// - `input_token_mint`: Token mint being swapped from
    /// - `amount_in`: Amount of input tokens to swap
    /// - `expected_amount_out`: Expected output amount (for validation)
    /// - `pool_id`: Expected Pool ID (PDA address) for security validation
    /// 
    /// # Security:
    /// - Pool ID validation prevents PDA bypass attacks
    /// - Client must specify exact pool they intend to swap with
    Swap {
        input_token_mint: Pubkey,
        amount_in: u64,
        expected_amount_out: u64,
        pool_id: Pubkey,
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
    
    /// **TREASURY INFORMATION WITH CONSOLIDATED DATA**
    /// 
    /// Returns comprehensive information about the treasury including:
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
    
    /// **PHASE 3: BATCH POOL FEE CONSOLIDATION**
    /// 
    /// Consolidates SOL fees from multiple pool states to the MainTreasuryState with
    /// flexible pause support. Supports both system-wide pause and individual pool pause.
    /// 
    /// # Consolidation Modes:
    /// - **System Paused**: Consolidates all specified pools regardless of individual pause state
    /// - **System Active**: Only consolidates pools with both `paused=true` AND `swaps_paused=true`
    /// 
    /// # Features:
    /// - Batch processing: 1-20 pools per instruction
    /// - Rent exempt protection: Never reduces pool balance below rent exempt minimum
    /// - Partial consolidation: Consolidates available amount if full consolidation would violate rent exemption
    /// - Atomic operation: All eligible pools processed or entire operation fails
    /// - Comprehensive logging: Detailed consolidation results and safety checks
    /// 
    /// # Arguments:
    /// - `pool_count`: Number of pools to consolidate (1-20)
    /// 
    /// # Account Order:
    /// - [0] System State PDA (for pause validation)
    /// - [1] Main Treasury PDA (receives consolidated fees)
    /// - [2..2+pool_count] Pool State PDAs (pools to consolidate)
    /// 
    /// # CU Estimate: 
    /// - 1 pool: ~5,000 CUs
    /// - 20 pools: ~109,000 CUs
    /// - Scales linearly with pool count
    ConsolidatePoolFees {
        pool_count: u8,
    },
    
    /// **PHASE 3: CONSOLIDATION STATUS REPORT**
    /// 
    /// View-only function that provides detailed consolidation status for multiple pools.
    /// Useful for determining which pools have fees to consolidate and the potential
    /// benefits of consolidation.
    /// 
    /// # Information Provided:
    /// - Individual pool fee amounts and operation counts
    /// - Last consolidation timestamp for each pool
    /// - Total fees available across all pools
    /// - Estimated consolidation cost vs. benefit analysis
    /// 
    /// # Arguments:
    /// - `pool_count`: Number of pools to check (1-20)
    /// 
    /// # Account Order:
    /// - [0..pool_count] Pool State PDAs (pools to check)
    GetConsolidationStatus {
        pool_count: u8,
    },
    
    /// **PHASE 4: POOL PAUSE OPERATIONS**
    /// 
    /// Pauses pool operations using bitwise flags (pool owner only).
    /// Uses bitwise flags to control which operations to pause:
    /// - PAUSE_FLAG_LIQUIDITY (1): Pause deposits/withdrawals
    /// - PAUSE_FLAG_SWAPS (2): Pause swaps
    /// - PAUSE_FLAG_ALL (3): Pause both (required for consolidation eligibility)
    /// 
    /// **Idempotent**: Pausing already paused operations does not cause an error.
    /// 
    /// # Arguments:
    /// - `pause_flags`: Bitwise flags indicating which operations to pause
    /// - `pool_id`: Expected Pool ID (PDA address) for security validation
    /// 
    /// # Security:
    /// - Pool ID validation prevents targeting wrong pool
    /// - Admin must specify exact pool they intend to pause
    /// 
    /// # Account Order:
    /// - [0] Pool Owner Signer (must match pool.owner)
    /// - [1] System State PDA (for system pause validation)
    /// - [2] Pool State PDA (writable, to update pause state)
    PausePool {
        pause_flags: u8,
        pool_id: Pubkey,
    },
    
    /// **PHASE 4: POOL UNPAUSE OPERATIONS**
    /// 
    /// Unpauses pool operations using bitwise flags (pool owner only).
    /// Uses bitwise flags to control which operations to unpause:
    /// - PAUSE_FLAG_LIQUIDITY (1): Unpause deposits/withdrawals
    /// - PAUSE_FLAG_SWAPS (2): Unpause swaps
    /// - PAUSE_FLAG_ALL (3): Unpause both operations
    /// 
    /// **Idempotent**: Unpausing already unpaused operations does not cause an error.
    /// 
    /// # Arguments:
    /// - `unpause_flags`: Bitwise flags indicating which operations to unpause
    /// - `pool_id`: Expected Pool ID (PDA address) for security validation
    /// 
    /// # Security:
    /// - Pool ID validation prevents targeting wrong pool
    /// - Admin must specify exact pool they intend to unpause
    /// 
    /// # Account Order:
    /// - [0] Pool Owner Signer (must match pool.owner)
    /// - [1] System State PDA (for system pause validation)
    /// - [2] Pool State PDA (writable, to update pause state)
    UnpausePool {
        unpause_flags: u8,
        pool_id: Pubkey,
    },
    
    /// **SWAP ACCESS CONTROL**: Enable/disable restrictions and delegate ownership control
    /// 
    /// This instruction allows the contract owner (admin authority) to control
    /// swap access for a specific pool and delegate control to any specified entity.
    /// When enabled, only the designated owner can perform swap operations on that pool.
    /// 
    /// # Enhanced Flexibility:
    /// - Admin Authority retains exclusive right to call this instruction
    /// - Can delegate swap control to any authorized entity (not just Admin Authority)
    /// - Enables complex operational scenarios with specialized swap controllers
    /// - Maintains security through centralized authority validation
    /// 
    /// # Purpose
    /// - Enables custom fee structures by restricting direct pool access
    /// - Allows delegation of swap control to trusted specialized entities
    /// - Supports complex operational scenarios (treasury management, automated strategies)
    /// - Provides flexibility for different fee models and operational patterns
    /// - Maintains compatibility with standard AMM operation when disabled
    /// 
    /// # Security
    /// - Only the Admin Authority can call this instruction
    /// - Delegation does not transfer the ability to change restrictions
    /// - Admin Authority maintains ultimate control over all pools
    /// 
    /// # Arguments:
    /// - `enable_restriction`: True to enable owner-only mode, false to disable
    /// - `designated_owner`: The pubkey that will have swap control when restrictions are enabled
    /// - `pool_id`: Expected Pool ID (PDA address) for security validation
    /// 
    /// # Security:
    /// - Pool ID validation prevents targeting wrong pool
    /// - Admin must specify exact pool they intend to modify
    /// 
    /// # Account Order:
    /// - [0] Contract Owner Signer (must be admin authority)
    /// - [1] System State PDA (for system pause validation)
    /// - [2] Pool State PDA (writable, to update swap access flag and owner)
    /// - [3] Program Data Account (for upgrade authority validation)
    SetSwapOwnerOnly {
        enable_restriction: bool,
        designated_owner: Pubkey,
        pool_id: Pubkey,
    },
    
    /// **POOL FEE UPDATE**: Update pool contract fees (program authority only)
    /// 
    /// Allows the program authority to update the contract fees for a specific pool.
    /// This provides granular control over pool economics while maintaining security
    /// through proper authorization checks.
    /// 
    /// # Security:
    /// - Only the program authority can call this instruction
    /// - Fee updates are applied immediately to all future operations
    /// - Existing pending fees are not affected by the update
    /// - Fee validation ensures reasonable limits
    /// 
    /// # Arguments:
    /// - `update_flags`: Bitwise flags indicating which fees to update
    ///   - 0b01 (1): Update liquidity fee
    ///   - 0b10 (2): Update swap fee  
    ///   - 0b11 (3): Update both fees
    /// - `new_liquidity_fee`: New liquidity fee in lamports (only used if liquidity flag is set)
    /// - `new_swap_fee`: New swap fee in lamports (only used if swap flag is set)
    /// - `pool_id`: Expected Pool ID (PDA address) for security validation
    /// 
    /// # Security:
    /// - Pool ID validation prevents targeting wrong pool
    /// - Admin must specify exact pool they intend to modify
    /// 
    /// # Account Order:
    /// - [0] Program Authority Signer (must be admin authority)
    /// - [1] System State PDA (for system pause validation)
    /// - [2] Pool State PDA (writable, to update fee parameters)
    /// - [3] Program Data Account (for upgrade authority validation)
    UpdatePoolFees {
        update_flags: u8,
        new_liquidity_fee: u64,
        new_swap_fee: u64,
        pool_id: Pubkey,
    },
    
    /// **DONATION: Voluntary SOL contribution to the protocol treasury**
    /// 
    /// Allows anyone to donate SOL to support the protocol. Donations are:
    /// - 100% deposited to the main treasury
    /// - Tracked separately from fees for transparency
    /// - Non-refundable once sent (accidental donations will NOT be returned)
    /// - Not allowed when system is paused
    /// 
    /// # Purpose:
    /// - Testing treasury withdrawals without creating massive transactions
    /// - Voluntary protocol support
    /// - Lost/stuck funds recovery
    /// 
    /// # Arguments:
    /// - `amount`: Amount of SOL to donate in lamports
    /// - `message`: Optional message (logged but not stored on-chain)
    /// 
    /// # Account Order:
    /// - [0] Donor Account (signer, writable) - Account donating SOL
    /// - [1] Main Treasury PDA (writable) - Receives the donation
    /// - [2] System State PDA (readable) - For pause validation
    /// - [3] System Program Account (readable) - For SOL transfer
    DonateSol {
        amount: u64,
        message: String,
    },
    
    /// **ADMIN AUTHORITY MANAGEMENT**: Process admin authority change with automatic completion
    /// 
    /// This unified instruction handles both initiation and completion of admin changes:
    /// 1. If no change is pending or different admin proposed: starts 72-hour timer
    /// 2. If 72+ hours have passed and pending admin differs from current: completes change  
    /// 3. If same admin as current is proposed: acts as cancellation (clears pending)
    /// 
    /// # Security Features:
    /// - 72-hour timelock prevents immediate hostile takeover
    /// - Different admin proposed within 72 hours resets timer
    /// - Same admin proposed acts as cancellation
    /// - Automatic completion when timelock satisfied
    /// 
    /// # Arguments:
    /// - `new_admin`: The proposed new admin authority pubkey
    /// 
    /// # Account Order:
    /// - [0] Current Admin Authority (signer) - Must be current admin
    /// - [1] System State PDA (writable) - To store/update admin state
    /// - [2] Program Data Account (readable) - For upgrade authority fallback during migration
    ProcessAdminChange {
        new_admin: Pubkey,
    },
    
} 