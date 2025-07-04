//! State Module
//! 
//! This module contains all state-related types and management for the program.

pub mod system_state;
pub mod treasury_state;

// Re-export all state types for easy access
pub use system_state::*;
pub use treasury_state::*; 