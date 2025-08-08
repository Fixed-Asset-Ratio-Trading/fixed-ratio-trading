//! Ratio Type Definitions
//!
//! This module contains types related to pool ratio classifications.

use borsh::{BorshDeserialize, BorshSerialize};

/// Represents different types of trading pool ratios based on their numeric characteristics.
/// 
/// This enum classifies pool ratios into three categories:
/// - Simple ratios (one-to-many with whole numbers)
/// - Decimal ratios (one side is 1.0, other has decimals)
/// - Engineering ratios (arbitrary decimal values on both sides)
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum RatioType {
    /// Simple ratio where one side equals exactly 1.0 and both sides are whole numbers.
    /// Examples: 1:2, 1:100, 1000:1, 1:50
    /// This is the most basic and common type of trading ratio.
    SimpleRatio,
    
    /// Decimal ratio where one side equals exactly 1.0 but the other side has decimal places.
    /// Examples: 1:100.24343, 1:0.5, 1:1234.56789
    /// One side is a whole 1, but fractional values are allowed on the other side.
    DecimalRatio,
    
    /// Engineering ratio where neither side equals 1.0 or both sides have decimal values.
    /// Examples: 223.34984:10.2345, 0.5:0.3, 2.5:3.7
    /// Arbitrary decimal values on both sides, used for complex engineering calculations.
    EngineeringRatio,
}

impl RatioType {
    /// Returns a human-readable description of the ratio type
    pub fn description(&self) -> &'static str {
        match self {
            RatioType::SimpleRatio => "Simple one-to-many ratio with whole numbers",
            RatioType::DecimalRatio => "Ratio with one side equal to 1 and decimals allowed",
            RatioType::EngineeringRatio => "Complex ratio with arbitrary decimal values",
        }
    }
    
    /// Returns a short name for the ratio type suitable for logging
    pub fn short_name(&self) -> &'static str {
        match self {
            RatioType::SimpleRatio => "Simple",
            RatioType::DecimalRatio => "Decimal",
            RatioType::EngineeringRatio => "Engineering",
        }
    }
}

impl std::fmt::Display for RatioType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.short_name(), self.description())
    }
}
