//! Utility Functions
//! 
//! This module contains shared utility functions used throughout the program.
//! These utilities are organized by functionality and provide common operations
//! for validation, serialization, and rent calculations.

pub mod validation;
pub mod serialization;
pub mod rent;

// Re-export utility functions for easy access
pub use validation::*;
pub use serialization::*;
pub use rent::*; 