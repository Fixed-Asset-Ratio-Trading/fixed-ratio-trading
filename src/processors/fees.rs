//! Fee Management Processors (GOVERNANCE CONTROLLED)
//! 
//! This module handles fee-related operations under the governance-controlled architecture.
//!
//! ## Fee Architecture
//!
//! ### 1. Contract Fees (Fixed SOL amounts)
//! - **Pool Creation**: 1.15 SOL per pool creation
//! - **Liquidity Operations**: 0.0013 SOL per deposit/withdrawal  
//! - **Swaps**: 0.00002715 SOL per swap transaction
//! - **Purpose**: Cover operational costs and prevent spam
//! - **Collection**: Automatically transferred to central treasury PDAs
//! - **Withdrawal**: Via `WithdrawTreasuryFees()` by SYSTEM AUTHORITY ONLY
//!
//! ### 2. Pool Fees (Percentage-based on tokens)
//! - **Rate**: Fixed at creation time (0% default)
//! - **Application**: Deducted from input tokens during swaps
//! - **Purpose**: Revenue for governance-controlled protocol development
//! - **Collection**: Tracked in pool state (`collected_fees_token_a`, `collected_fees_token_b`)
//! - **Withdrawal**: Via governance protocols only
//!
//! ## Governance Model:
//! - All fee withdrawals controlled by system authority
//! - Fee rates managed by governance protocols
//! - Pool security parameters controlled by system authority
//! - Individual pool owners have no fee withdrawal rights

// Minimal imports for governance-controlled fee architecture documentation
// No active fee processing functions in this module

// Pool owner fee withdrawal and management functions removed for governance control
// All fee management is now handled through:
// - System authority via WithdrawTreasuryFees instruction
// - Governance protocols for token fee management
// - System-wide pause/unpause controls via system authority 