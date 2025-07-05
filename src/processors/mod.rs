//! Processors Module
//! 
//! This module contains all processor functions organized by functionality.

pub mod pool_creation;
pub mod liquidity;
pub mod swap;
pub mod fees;
pub mod security;
pub mod system_pause;
pub mod utilities;
pub mod treasury;

// Re-export pool creation functions
pub use pool_creation::*;

// Re-export liquidity management functions  
pub use liquidity::*;

// Re-export swap operations functions
pub use swap::*;

// Fee management functions removed for governance control
// fees module contains only architecture documentation

// Security management functions removed for governance control  
// security module contains only architecture documentation

// Re-export system pause functions
pub use system_pause::*;

// Re-export utility functions
pub use utilities::*;

// Re-export treasury management functions
pub use treasury::*; 