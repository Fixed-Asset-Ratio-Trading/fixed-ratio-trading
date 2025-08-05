//! Processors Module
//! 
//! This module contains all processor functions organized by functionality.

pub mod consolidation;
pub mod liquidity;
pub mod swap;
pub mod utilities;
pub mod treasury;
pub mod system;  // System management functions
pub mod pool;    // Pool management functions

// Re-export consolidation functions
pub use consolidation::*;

// Re-export liquidity management functions  
pub use liquidity::*;

// Re-export swap operations functions
pub use swap::*;

// Re-export utility functions
pub use utilities::*;

// Re-export treasury management functions
pub use treasury::*;

// Re-export system management functions
pub use system::*;

// Re-export pool management functions
pub use pool::*; 