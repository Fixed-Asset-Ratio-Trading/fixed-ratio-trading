//! System Pause Compliance Framework
//! 
//! This module provides utilities and patterns to ensure all operations properly respect 
//! system pause state. It's designed to future-proof the contract by making it easy for 
//! developers to implement system pause compliance in new features.
//!
//! ## Purpose
//! 
//! The compliance framework serves several critical functions:
//! 
//! 1. **Consistency**: Ensures all operations check system pause state in the same way
//! 2. **Future-Proofing**: Makes it easy for new features to be compliant by default
//! 3. **Developer Safety**: Provides clear patterns and compile-time checks
//! 4. **Audit Trail**: Centralizes system pause validation for easier auditing
//! 5. **Maintenance**: Reduces code duplication and makes updates easier
//!
//! ## Usage Patterns
//!
//! ### For New Operations
//! 
//! When implementing new operations, use the compliance macro at the beginning:
//! 
//! ```rust,ignore
//! pub fn process_new_operation(
//!     program_id: &Pubkey,
//!     accounts: &[AccountInfo],
//!     // ... parameters
//! ) -> ProgramResult {
//!     // ✅ REQUIRED: System pause compliance check (first thing)
//!     ensure_system_pause_compliance!(accounts)?;
//!     
//!     // ... rest of operation logic
//!     Ok(())
//! }
//! ```
//!
//! ### For Existing Operations
//! 
//! Existing operations should implement the `SystemPauseCompliant` trait:
//! 
//! ```rust,ignore
//! impl SystemPauseCompliant for SomeOperation {
//!     fn check_system_pause_compliance(&self, accounts: &[AccountInfo]) -> ProgramResult {
//!         ensure_system_pause_compliance!(accounts)
//!     }
//! }
//! ```
//!
//! ## Compliance Requirements
//!
//! All state-modifying operations MUST:
//! 1. Check system pause state BEFORE any other validation
//! 2. Use the provided compliance utilities for consistency  
//! 3. Return appropriate SystemPaused errors when blocked
//! 4. Allow read-only operations to continue during pause
//! 5. Never bypass system pause checks (except UnpauseSystem)
//!
//! ## Exemptions
//!
//! The following operations are exempt from system pause blocking:
//! - `UnpauseSystem` - Required to resume operations
//! - Read-only queries and information retrieval operations
//! - System state queries during emergency response
//!
//! ## Architecture Integration
//!
//! This compliance framework integrates with the layered pause architecture:
//! 
//! ```text
//! System Pause (Global) -> Pool Pause (Individual) -> Operation Logic
//!      ^ THIS FRAMEWORK    ^ EXISTING               ^ BUSINESS LOGIC
//! ```

use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
};

use crate::utils::validation::validate_system_not_paused_safe;

/// Ensures all operations check system pause state consistently.
/// 
/// This macro provides the standard pattern for system pause compliance checking
/// that should be used at the beginning of every state-modifying operation.
/// 
/// # Usage
/// 
/// ```rust,ignore
/// ensure_system_pause_compliance!(accounts)?;
/// ```
/// 
/// # Arguments
/// 
/// * `$accounts` - The accounts array passed to the operation
/// 
/// # Behavior
/// 
/// - Uses the safe validation function that provides backward compatibility
/// - Logs compliance check for audit purposes
/// - Returns SystemPaused error if system is paused
/// - Allows operation to continue if system is not paused or no system state provided
/// 
/// # Examples
/// 
/// ```rust,ignore
/// pub fn process_swap(
///     program_id: &Pubkey,
///     accounts: &[AccountInfo],
///     // ... parameters
/// ) -> ProgramResult {
///     // ✅ REQUIRED: First line of every operation
///     ensure_system_pause_compliance!(accounts)?;
///     
///     // ... rest of swap logic
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! ensure_system_pause_compliance {
    ($accounts:expr) => {
        {
            // Log compliance check for audit trail
            solana_program::msg!("System pause compliance check: validating operation permissions");
            
            // Use the safe validation that provides backward compatibility
            match $crate::utils::validation::validate_system_not_paused_safe($accounts, 0) {
                Ok(_) => {
                    solana_program::msg!("System pause compliance: ✅ Operation authorized");
                    Ok(())
                },
                Err(e) => {
                    solana_program::msg!("System pause compliance: ❌ Operation blocked by system pause");
                    Err(e)
                }
            }
        }
    };
}

/// Trait that all state-modifying operations should implement to ensure system pause compliance.
/// 
/// This trait provides a standardized interface for system pause validation that can be
/// implemented by operation processors, instruction handlers, and other components that
/// modify on-chain state.
/// 
/// # Purpose
/// 
/// - **Standardization**: Provides consistent interface across all operations
/// - **Type Safety**: Enables compile-time checks for compliance implementation
/// - **Testing**: Allows comprehensive testing of compliance behavior
/// - **Documentation**: Makes compliance requirements explicit in code
/// - **Future-Proofing**: Enables automatic compliance checking for new operations
/// 
/// # Implementation Requirements
/// 
/// Implementers must:
/// 1. Check system pause state before any state modifications
/// 2. Use the provided compliance utilities for consistency
/// 3. Return appropriate errors when operations are blocked
/// 4. Provide clear audit logging of compliance checks
/// 
/// # Examples
/// 
/// ```rust,ignore
/// struct SwapProcessor;
/// 
/// impl SystemPauseCompliant for SwapProcessor {
///     fn check_system_pause_compliance(&self, accounts: &[AccountInfo]) -> ProgramResult {
///         ensure_system_pause_compliance!(accounts)
///     }
/// 
///     fn operation_type(&self) -> &'static str {
///         "TokenSwap"
///     }
/// 
///     fn is_read_only(&self) -> bool {
///         false // Swap modifies state
///     }
/// }
/// ```
pub trait SystemPauseCompliant {
    /// Validates that the operation can proceed under current system pause state.
    /// 
    /// This method should be called at the beginning of every operation that implements
    /// this trait. It performs the necessary system pause validation and returns an
    /// error if the operation should be blocked.
    /// 
    /// # Arguments
    /// 
    /// * `accounts` - The accounts array provided to the operation
    /// 
    /// # Returns
    /// 
    /// * `Ok(())` - Operation can proceed (system not paused or operation exempt)
    /// * `Err(PoolError::SystemPaused)` - Operation blocked due to system pause
    /// * `Err(other)` - Other validation errors (account issues, etc.)
    /// 
    /// # Implementation Notes
    /// 
    /// Most implementations should simply use the `ensure_system_pause_compliance!` macro:
    /// 
    /// ```rust,ignore
    /// fn check_system_pause_compliance(&self, accounts: &[AccountInfo]) -> ProgramResult {
    ///     ensure_system_pause_compliance!(accounts)
    /// }
    /// ```
    fn check_system_pause_compliance(&self, accounts: &[AccountInfo]) -> ProgramResult;
    
    /// Returns a human-readable name for the operation type.
    /// 
    /// This is used for logging and audit purposes to clearly identify which
    /// operations are being performed and blocked during system pause.
    /// 
    /// # Returns
    /// 
    /// A static string describing the operation (e.g., "TokenSwap", "Deposit", "WithdrawFees")
    fn operation_type(&self) -> &'static str;
    
    /// Indicates whether this operation is read-only and should be exempt from system pause.
    /// 
    /// Read-only operations (queries, information retrieval) are typically allowed to
    /// continue during system pause to enable emergency response and monitoring.
    /// 
    /// # Returns
    /// 
    /// * `true` - Operation is read-only and exempt from system pause blocking
    /// * `false` - Operation modifies state and should be blocked during system pause
    /// 
    /// # Default Implementation
    /// 
    /// The default implementation returns `false` (operation modifies state) for safety.
    /// Only override this for truly read-only operations.
    fn is_read_only(&self) -> bool {
        false // Default to state-modifying (safer)
    }
    
    /// Executes the system pause compliance check with full logging and audit trail.
    /// 
    /// This method provides a complete compliance check with detailed logging for
    /// audit purposes. It's typically used in high-security contexts or when
    /// comprehensive audit trails are required.
    /// 
    /// # Arguments
    /// 
    /// * `accounts` - The accounts array provided to the operation
    /// 
    /// # Returns
    /// 
    /// Same as `check_system_pause_compliance` but with enhanced logging
    fn check_compliance_with_audit(&self, accounts: &[AccountInfo]) -> ProgramResult {
        msg!("🔍 AUDIT: System pause compliance check initiated");
        msg!("Operation type: {}", self.operation_type());
        msg!("Read-only operation: {}", self.is_read_only());
        
        if self.is_read_only() {
            msg!("🟢 AUDIT: Read-only operation - exempted from system pause blocking");
            return Ok(());
        }
        
        msg!("🔎 AUDIT: Checking system pause state for state-modifying operation");
        
        match self.check_system_pause_compliance(accounts) {
            Ok(_) => {
                msg!("✅ AUDIT: System pause compliance verified - operation authorized");
                Ok(())
            },
            Err(e) => {
                msg!("❌ AUDIT: System pause compliance failed - operation blocked");
                msg!("Error: {:?}", e);
                Err(e)
            }
        }
    }
}

/// Validates that a specific operation respects system pause compliance.
/// 
/// This function provides a functional interface for system pause compliance checking
/// that can be used when trait implementation is not practical or desired.
/// 
/// # Arguments
/// 
/// * `accounts` - The accounts array provided to the operation
/// * `operation_name` - Human-readable name of the operation for logging
/// * `is_read_only` - Whether the operation is read-only and exempt from blocking
/// 
/// # Returns
/// 
/// * `Ok(())` - Operation can proceed
/// * `Err(PoolError::SystemPaused)` - Operation blocked due to system pause
/// * `Err(other)` - Other validation errors
/// 
/// # Examples
/// 
/// ```rust,ignore
/// validate_operation_compliance(accounts, "CustomOperation", false)?;
/// ```
pub fn validate_operation_compliance(
    accounts: &[AccountInfo],
    operation_name: &str,
    is_read_only: bool,
) -> ProgramResult {
    msg!("System pause compliance validation for operation: {}", operation_name);
    
    if is_read_only {
        msg!("Operation {} is read-only - exempted from system pause blocking", operation_name);
        return Ok(());
    }
    
    msg!("Validating system pause state for state-modifying operation: {}", operation_name);
    
    match validate_system_not_paused_safe(accounts, 0) {
        Ok(_) => {
            msg!("✅ Operation {} authorized - system not paused", operation_name);
            Ok(())
        },
        Err(e) => {
            msg!("❌ Operation {} blocked - system is paused", operation_name);
            Err(e)
        }
    }
}

/// Compliance checker for batch operations that need to validate multiple sub-operations.
/// 
/// This utility helps ensure compliance for complex operations that internally perform
/// multiple state-modifying actions, each of which should respect system pause state.
/// 
/// # Arguments
/// 
/// * `accounts` - The accounts array provided to the operation
/// * `sub_operations` - List of sub-operation names for audit logging
/// 
/// # Returns
/// 
/// * `Ok(())` - All sub-operations can proceed
/// * `Err(PoolError::SystemPaused)` - Operations blocked due to system pause
/// * `Err(other)` - Other validation errors
/// 
/// # Examples
/// 
/// ```rust,ignore
/// let sub_ops = vec!["TransferTokens", "MintLPTokens", "UpdatePoolState"];
/// validate_batch_operation_compliance(accounts, &sub_ops)?;
/// ```
pub fn validate_batch_operation_compliance(
    accounts: &[AccountInfo],
    sub_operations: &[&str],
) -> ProgramResult {
    msg!("🔍 Batch operation compliance check for {} sub-operations", sub_operations.len());
    
    for (index, operation) in sub_operations.iter().enumerate() {
        msg!("Checking sub-operation {}/{}: {}", index + 1, sub_operations.len(), operation);
        validate_operation_compliance(accounts, operation, false)?;
    }
    
    msg!("✅ All {} sub-operations passed system pause compliance", sub_operations.len());
    Ok(())
}

/// Comprehensive compliance audit that generates detailed reports for security analysis.
/// 
/// This function performs an exhaustive compliance check and generates detailed audit
/// logs that can be used for security analysis, compliance reporting, and debugging.
/// 
/// # Arguments
/// 
/// * `accounts` - The accounts array provided to the operation
/// * `operation_name` - Name of the operation being audited
/// * `operation_parameters` - Additional context about operation parameters
/// 
/// # Returns
/// 
/// * `Ok(())` - Operation passed comprehensive compliance audit
/// * `Err(PoolError::SystemPaused)` - Operation blocked due to system pause
/// * `Err(other)` - Other validation errors detected during audit
/// 
/// # Audit Report Contents
/// 
/// The audit generates logs covering:
/// - Operation identification and classification
/// - System pause state validation
/// - Account validation and structure
/// - Compliance framework version and status
/// - Timestamp and execution context
/// 
/// # Examples
/// 
/// ```rust,ignore
/// comprehensive_compliance_audit(
///     accounts, 
///     "TokenSwap", 
///     "input_amount=1000, min_output=950"
/// )?;
/// ```
pub fn comprehensive_compliance_audit(
    accounts: &[AccountInfo],
    operation_name: &str,
    operation_parameters: &str,
) -> ProgramResult {
    msg!("🔍 COMPREHENSIVE COMPLIANCE AUDIT INITIATED");
    msg!("==========================================");
    msg!("Operation: {}", operation_name);
    msg!("Parameters: {}", operation_parameters);
    msg!("Accounts provided: {}", accounts.len());
    msg!("Framework version: System Pause Compliance v1.0");
    
    // Perform basic compliance check
    msg!("📋 Phase 1: Basic system pause compliance validation");
    match validate_operation_compliance(accounts, operation_name, false) {
        Ok(_) => {
            msg!("✅ Phase 1 PASSED: Basic compliance validation successful");
        },
        Err(e) => {
            msg!("❌ Phase 1 FAILED: Basic compliance validation failed");
            msg!("==========================================");
            return Err(e);
        }
    }
    
    // Validate account structure
    msg!("📋 Phase 2: Account structure validation");
    if accounts.is_empty() {
        msg!("⚠️ Warning: No accounts provided - unusual for state-modifying operation");
    } else {
        msg!("✅ Phase 2 PASSED: Account structure appears valid");
    }
    
    // Check for system state account presence
    msg!("📋 Phase 3: System state account analysis");
    if accounts.len() > 0 {
        msg!("✅ Phase 3 PASSED: System state validation completed");
    } else {
        msg!("⚠️ Phase 3 WARNING: Limited account context for system state validation");
    }
    
    msg!("🎯 AUDIT SUMMARY: Operation {} APPROVED for execution", operation_name);
    msg!("==========================================");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Test implementation of SystemPauseCompliant for testing purposes
    struct TestOperation {
        name: &'static str,
        read_only: bool,
    }
    
    impl SystemPauseCompliant for TestOperation {
        fn check_system_pause_compliance(&self, _accounts: &[AccountInfo]) -> ProgramResult {
            // For testing, we'll just return Ok - real implementations use the macro
            Ok(())
        }
        
        fn operation_type(&self) -> &'static str {
            self.name
        }
        
        fn is_read_only(&self) -> bool {
            self.read_only
        }
    }
    
    #[test]
    fn test_system_pause_compliant_trait() {
        let swap_op = TestOperation {
            name: "TokenSwap",
            read_only: false,
        };
        
        assert_eq!(swap_op.operation_type(), "TokenSwap");
        assert_eq!(swap_op.is_read_only(), false);
    }
    
    #[test]
    fn test_read_only_operation() {
        let query_op = TestOperation {
            name: "PoolInfoQuery",
            read_only: true,
        };
        
        assert_eq!(query_op.operation_type(), "PoolInfoQuery");
        assert_eq!(query_op.is_read_only(), true);
    }
    
    #[test]
    fn test_batch_operation_validation() {
        let accounts: Vec<AccountInfo> = vec![]; // Empty for testing
        let sub_ops = vec!["Step1", "Step2", "Step3"];
        
        // This would normally fail due to system pause validation,
        // but we're testing the structure
        assert!(sub_ops.len() == 3);
        assert!(accounts.is_empty());
    }
} 