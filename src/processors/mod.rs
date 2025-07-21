//! Processors Module
//! 
//! This module contains all processor functions organized by functionality.

pub mod consolidation;
pub mod pool_creation;
pub mod pool_management;
pub mod liquidity;
pub mod swap;
pub mod system_pause;
pub mod process_initialize;
pub mod utilities;
pub mod treasury;
pub mod pool_fee_update;

// Re-export consolidation functions
pub use consolidation::*;

// Re-export pool creation functions
pub use pool_creation::*;

// Re-export pool management functions
pub use pool_management::*;

// Re-export liquidity management functions  
pub use liquidity::*;

// Re-export swap operations functions
pub use swap::*;

// Fee and security management functions removed for governance control

// Re-export system pause functions
pub use system_pause::*;

// Re-export program initialization functions
pub use process_initialize::*;

// Re-export utility functions
pub use utilities::*;

// Re-export treasury management functions
pub use treasury::*;

// Re-export pool fee update functions
pub use pool_fee_update::*; 