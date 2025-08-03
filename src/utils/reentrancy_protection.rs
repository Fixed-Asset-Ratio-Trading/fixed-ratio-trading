//! Enhanced Reentrancy Protection with Global State Tracking
//!
//! This module provides enhanced reentrancy protection with:
//! - Global reentrancy flag tracking
//! - Multi-level protection (per-account and per-program)
//! - Automatic cleanup on errors
//! - Thread-local storage for better isolation

use solana_program::{
    account_info::AccountInfo,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use std::cell::RefCell;
use std::collections::HashSet;

// Maximum allowed reentrancy depth (for legitimate nested calls)
const MAX_ALLOWED_DEPTH: u32 = 2;

// Thread-local storage for tracking active operations
thread_local! {
    // Tracks which accounts are currently in use to prevent reentrancy
    static ACTIVE_ACCOUNTS: RefCell<HashSet<Pubkey>> = RefCell::new(HashSet::new());
    
    // Global reentrancy counter for the entire program
    static REENTRANCY_DEPTH: RefCell<u32> = RefCell::new(0);
}

/// Guard that ensures cleanup even on error
pub struct ReentrancyGuard {
    account_keys: Vec<Pubkey>,
    operation_name: String,
}

impl ReentrancyGuard {
    /// Create a new reentrancy guard for the specified accounts
    pub fn new(accounts: &[&AccountInfo], operation_name: &str) -> Result<Self, ProgramError> {
        let account_keys: Vec<Pubkey> = accounts.iter().map(|acc| *acc.key).collect();
        
        // Check global reentrancy depth
        REENTRANCY_DEPTH.with(|depth| {
            let current = *depth.borrow();
            if current >= MAX_ALLOWED_DEPTH {
                msg!("❌ REENTRANCY DETECTED: Max depth {} exceeded for {}", MAX_ALLOWED_DEPTH, operation_name);
                msg!("   Current depth: {}", current);
                return Err(ProgramError::InvalidAccountData);
            }
            
            // Increment depth
            *depth.borrow_mut() = current + 1;
            msg!("🔒 REENTRANCY GUARD: Entered {} (depth: {})", operation_name, current + 1);
            Ok(())
        })?;
        
        // Check if any accounts are already locked
        ACTIVE_ACCOUNTS.with(|active| {
            let mut active_set = active.borrow_mut();
            
            for key in &account_keys {
                if active_set.contains(key) {
                    msg!("❌ REENTRANCY DETECTED: Account {} already in use during {}", key, operation_name);
                    msg!("   This indicates a potential reentrancy attack or nested operation");
                    
                    // Cleanup: decrement depth before returning error
                    REENTRANCY_DEPTH.with(|depth| {
                        *depth.borrow_mut() -= 1;
                    });
                    
                    return Err(ProgramError::InvalidAccountData);
                }
            }
            
            // Lock all accounts
            for key in &account_keys {
                active_set.insert(*key);
                msg!("🔐 LOCKED: Account {} for {}", key, operation_name);
            }
            
            Ok(())
        })?;
        
        Ok(Self {
            account_keys,
            operation_name: operation_name.to_string(),
        })
    }
    
    /// Execute a function with reentrancy protection
    pub fn execute<F, R>(accounts: &[&AccountInfo], operation_name: &str, f: F) -> Result<R, ProgramError>
    where
        F: FnOnce() -> Result<R, ProgramError>,
    {
        // Create guard (will lock accounts and increment depth)
        let _guard = Self::new(accounts, operation_name)?;
        
        // Execute the protected function
        let result = f();
        
        // Guard will be dropped here, triggering cleanup
        result
    }
}

impl Drop for ReentrancyGuard {
    fn drop(&mut self) {
        // Unlock accounts
        ACTIVE_ACCOUNTS.with(|active| {
            let mut active_set = active.borrow_mut();
            for key in &self.account_keys {
                active_set.remove(key);
                msg!("🔓 UNLOCKED: Account {} after {}", key, self.operation_name);
            }
        });
        
        // Decrement depth
        REENTRANCY_DEPTH.with(|depth| {
            let current = *depth.borrow();
            *depth.borrow_mut() = current.saturating_sub(1);
            msg!("🔒 REENTRANCY GUARD: Exited {} (depth: {})", self.operation_name, current - 1);
        });
    }
}

/// Macro for easy reentrancy protection
#[macro_export]
macro_rules! with_reentrancy_protection {
    ($accounts:expr, $operation:expr, $body:expr) => {
        ReentrancyGuard::execute($accounts, $operation, || $body)
    };
}

/// Check if we're currently in a protected operation
pub fn is_in_protected_operation() -> bool {
    REENTRANCY_DEPTH.with(|depth| *depth.borrow() > 0)
}

/// Get current reentrancy depth
pub fn get_reentrancy_depth() -> u32 {
    REENTRANCY_DEPTH.with(|depth| *depth.borrow())
}

/// Reset reentrancy state for current transaction only (use with caution, only for error recovery)
pub fn emergency_tx_stop() {
    msg!("⚠️ EMERGENCY TX STOP: Clearing reentrancy locks for current transaction");
    
    ACTIVE_ACCOUNTS.with(|active| {
        active.borrow_mut().clear();
    });
    
    REENTRANCY_DEPTH.with(|depth| {
        *depth.borrow_mut() = 0;
    });
}

/// Alias for clarity - same as emergency_tx_stop
pub fn abort_current_transaction() {
    emergency_tx_stop();
}

/// Wrapper for safely executing token transfer operations with snapshot-based protection
pub struct SafeTokenTransfer<'a> {
    pub source_account: &'a AccountInfo<'a>,
    pub destination_account: &'a AccountInfo<'a>,
    pub amount: u64,
    pub operation_name: String,
}

impl<'a> SafeTokenTransfer<'a> {
    /// Create a new safe transfer wrapper
    pub fn new(
        source_account: &'a AccountInfo<'a>,
        destination_account: &'a AccountInfo<'a>,
        amount: u64,
        operation_name: &str,
    ) -> Self {
        Self {
            source_account,
            destination_account,
            amount,
            operation_name: operation_name.to_string(),
        }
    }

    /// Execute the transfer with snapshot-based validation
    pub fn execute_with_protection<F>(self, transfer_fn: F) -> Result<(), ProgramError>
    where
        F: FnOnce() -> Result<(), ProgramError>,
    {
        use solana_program::program_pack::Pack;
        use spl_token::state::Account as TokenAccount;
        
        msg!("📸 SNAPSHOT PROTECTION: Starting {} operation", self.operation_name);

        // Capture pre-transfer snapshots
        let source_before = TokenAccount::unpack_from_slice(&self.source_account.data.borrow())?;
        let dest_before = TokenAccount::unpack_from_slice(&self.destination_account.data.borrow())?;
        
        // Execute the actual transfer
        msg!("💸 Executing {} transfer of {} tokens", self.operation_name, self.amount);
        transfer_fn()?;
        
        // Validate post-transfer state
        let source_after = TokenAccount::unpack_from_slice(&self.source_account.data.borrow())?;
        let dest_after = TokenAccount::unpack_from_slice(&self.destination_account.data.borrow())?;
        
        // Check source account
        let source_change = source_after.amount as i64 - source_before.amount as i64;
        if source_change != -(self.amount as i64) {
            msg!("❌ REENTRANCY DETECTED: Unexpected source balance change");
            msg!("   Expected: -{}, Actual: {}", self.amount, source_change);
            return Err(ProgramError::InvalidAccountData);
        }
        
        // Check destination account
        let dest_change = dest_after.amount as i64 - dest_before.amount as i64;
        if dest_change != self.amount as i64 {
            msg!("❌ REENTRANCY DETECTED: Unexpected destination balance change");
            msg!("   Expected: {}, Actual: {}", self.amount, dest_change);
            return Err(ProgramError::InvalidAccountData);
        }
        
        msg!("✅ SNAPSHOT PROTECTION: {} completed safely", self.operation_name);
        Ok(())
    }
}

/// Wrapper for safely executing token mint operations with snapshot-based protection
pub struct SafeTokenMint<'a> {
    pub destination_account: &'a AccountInfo<'a>,
    pub amount: u64,
    pub operation_name: String,
}

impl<'a> SafeTokenMint<'a> {
    pub fn new(
        destination_account: &'a AccountInfo<'a>,
        amount: u64,
        operation_name: &str,
    ) -> Self {
        Self {
            destination_account,
            amount,
            operation_name: operation_name.to_string(),
        }
    }

    pub fn execute_with_protection<F>(self, mint_fn: F) -> Result<(), ProgramError>
    where
        F: FnOnce() -> Result<(), ProgramError>,
    {
        use solana_program::program_pack::Pack;
        use spl_token::state::Account as TokenAccount;
        
        let balance_before = TokenAccount::unpack_from_slice(&self.destination_account.data.borrow())?.amount;
        
        mint_fn()?;
        
        let balance_after = TokenAccount::unpack_from_slice(&self.destination_account.data.borrow())?.amount;
        let actual_change = balance_after as i64 - balance_before as i64;
        
        if actual_change != self.amount as i64 {
            msg!("❌ REENTRANCY DETECTED: Unexpected mint amount");
            return Err(ProgramError::InvalidAccountData);
        }
        
        Ok(())
    }
}

/// Wrapper for safely executing token burn operations with snapshot-based protection
pub struct SafeTokenBurn<'a> {
    pub source_account: &'a AccountInfo<'a>,
    pub amount: u64,
    pub operation_name: String,
}

impl<'a> SafeTokenBurn<'a> {
    pub fn new(
        source_account: &'a AccountInfo<'a>,
        amount: u64,
        operation_name: &str,
    ) -> Self {
        Self {
            source_account,
            amount,
            operation_name: operation_name.to_string(),
        }
    }

    pub fn execute_with_protection<F>(self, burn_fn: F) -> Result<(), ProgramError>
    where
        F: FnOnce() -> Result<(), ProgramError>,
    {
        use solana_program::program_pack::Pack;
        use spl_token::state::Account as TokenAccount;
        
        let balance_before = TokenAccount::unpack_from_slice(&self.source_account.data.borrow())?.amount;
        
        burn_fn()?;
        
        let balance_after = TokenAccount::unpack_from_slice(&self.source_account.data.borrow())?.amount;
        let actual_change = balance_after as i64 - balance_before as i64;
        
        if actual_change != -(self.amount as i64) {
            msg!("❌ REENTRANCY DETECTED: Unexpected burn amount");
            return Err(ProgramError::InvalidAccountData);
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::account_info::AccountInfo;
    use solana_program::pubkey::Pubkey;
    
    #[test]
    fn test_reentrancy_guard() {
        // Create test accounts
        let key1 = Pubkey::new_unique();
        let key2 = Pubkey::new_unique();
        let mut lamports1 = 0;
        let mut lamports2 = 0;
        let mut data1 = vec![];
        let mut data2 = vec![];
        let owner = Pubkey::default();
        
        let account1 = AccountInfo::new(
            &key1,
            false,
            false,
            &mut lamports1,
            &mut data1,
            &owner,
            false,
            0,
        );
        
        let account2 = AccountInfo::new(
            &key2,
            false,
            false,
            &mut lamports2,
            &mut data2,
            &owner,
            false,
            0,
        );
        
        // Test normal operation
        let result = ReentrancyGuard::execute(&[&account1, &account2], "test_op", || {
            Ok(42)
        });
        assert_eq!(result.unwrap(), 42);
        
        // Test nested operation (should fail)
        let result = ReentrancyGuard::execute(&[&account1], "outer_op", || {
            // This inner operation should fail due to reentrancy
            ReentrancyGuard::execute(&[&account1], "inner_op", || {
                Ok(())
            })
        });
        assert!(result.is_err());
        
        // Verify cleanup happened
        assert_eq!(get_reentrancy_depth(), 0);
    }
}