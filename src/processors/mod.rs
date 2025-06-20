//! Processors Module
//! 
//! This module contains all processor functions organized by functionality.

pub mod pool_creation;
pub mod liquidity;
pub mod swap;
pub mod delegates;
pub mod fees;
pub mod security;
pub mod system_pause;
pub mod utilities;
pub mod delegate_actions;

// Re-export pool creation functions
pub use pool_creation::*;

// Re-export liquidity management functions  
pub use liquidity::*;

// Re-export swap operations functions
pub use swap::*;

// Re-export delegate management functions (includes governance/pause functions)
pub use delegates::*;

// Re-export fee management functions
pub use fees::*;

// Re-export security management functions
pub use security::*;

// Re-export system pause functions
pub use system_pause::*;

// Re-export utility functions
pub use utilities::*;

// Re-export delegate actions functions
pub use delegate_actions::*; 