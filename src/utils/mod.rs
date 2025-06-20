//! Utility Functions
//! 
//! This module contains shared utility functions used throughout the program.
//! These utilities are organized by functionality and provide common operations
//! for validation, serialization, and rent calculations.

pub mod validation;
pub mod serialization;
pub mod rent;
pub mod system_pause_compliance;
pub mod compliance_examples;

// Re-export utility functions for easy access
pub use validation::*;
pub use serialization::*;
pub use rent::*;
pub use system_pause_compliance::*;
pub use compliance_examples::*; 