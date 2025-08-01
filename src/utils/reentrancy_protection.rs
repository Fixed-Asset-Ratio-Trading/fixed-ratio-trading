//! Reentrancy Protection Utilities
//!
//! This module provides utilities to detect and prevent reentrancy attacks during
//! Cross-Program Invocations (CPIs), particularly around token transfer operations.
//!
//! ## Enhanced State Validation Pattern
//!
//! The utilities in this module implement pre/post-condition checks around external
//! calls to detect unexpected state changes that could indicate reentrancy attacks.
//!
//! ### Key Features:
//! - Pre-CPI balance snapshots
//! - Post-CPI balance verification
//! - Expected change validation
//! - Comprehensive error reporting
//! - Minimal performance overhead

use solana_program::{
    account_info::AccountInfo,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
};
use spl_token::state::Account as TokenAccount;

/// Snapshot of token account state before a CPI operation
#[derive(Debug, Clone)]
pub struct TokenAccountSnapshot {
    pub account_key: solana_program::pubkey::Pubkey,
    pub balance: u64,
    pub mint: solana_program::pubkey::Pubkey,
    pub owner: solana_program::pubkey::Pubkey,
    pub frozen: bool,
}

impl TokenAccountSnapshot {
    /// Create a snapshot of a token account's current state
    pub fn capture(account: &AccountInfo, account_name: &str) -> Result<Self, ProgramError> {
        // Validate account ownership
        if account.owner != &spl_token::id() {
            msg!("❌ REENTRANCY PROTECTION: {} is not owned by SPL Token program", account_name);
            return Err(ProgramError::IncorrectProgramId);
        }

        // Unpack token account data
        let token_account = TokenAccount::unpack_from_slice(&account.data.borrow())
            .map_err(|_| {
                msg!("❌ REENTRANCY PROTECTION: Failed to unpack {} as token account", account_name);
                ProgramError::InvalidAccountData
            })?;

        let snapshot = TokenAccountSnapshot {
            account_key: *account.key,
            balance: token_account.amount,
            mint: token_account.mint,
            owner: token_account.owner,
            frozen: token_account.state == spl_token::state::AccountState::Frozen,
        };

        msg!("📸 SNAPSHOT CAPTURED: {} - Balance: {}, Mint: {}", 
             account_name, snapshot.balance, snapshot.mint);

        Ok(snapshot)
    }

    /// Validate that the current account state matches expected changes
    pub fn validate_changes(
        &self,
        account: &AccountInfo,
        expected_balance_change: i64,
        operation_name: &str,
    ) -> Result<(), ProgramError> {
        // Re-read current state
        let current_token_account = TokenAccount::unpack_from_slice(&account.data.borrow())
            .map_err(|_| {
                msg!("❌ REENTRANCY PROTECTION: Failed to re-read account after {}", operation_name);
                ProgramError::InvalidAccountData
            })?;

        // Validate key hasn't changed (should be impossible but good safety check)
        if current_token_account.mint != self.mint {
            msg!("❌ REENTRANCY ATTACK DETECTED: Mint changed during {} operation", operation_name);
            msg!("   Expected mint: {}", self.mint);
            msg!("   Current mint: {}", current_token_account.mint);
            return Err(ProgramError::InvalidAccountData);
        }

        if current_token_account.owner != self.owner {
            msg!("❌ REENTRANCY ATTACK DETECTED: Owner changed during {} operation", operation_name);
            msg!("   Expected owner: {}", self.owner);
            msg!("   Current owner: {}", current_token_account.owner);
            return Err(ProgramError::InvalidAccountData);
        }

        // Calculate actual balance change
        let actual_balance_change = current_token_account.amount as i64 - self.balance as i64;

        // Validate balance change matches expectations
        if actual_balance_change != expected_balance_change {
            msg!("❌ REENTRANCY ATTACK DETECTED: Unexpected balance change during {}", operation_name);
            msg!("   Expected balance change: {}", expected_balance_change);
            msg!("   Actual balance change: {}", actual_balance_change);
            msg!("   Previous balance: {}", self.balance);
            msg!("   Current balance: {}", current_token_account.amount);
            return Err(ProgramError::InvalidAccountData);
        }

        // Check for unexpected freeze state changes
        let current_frozen = current_token_account.state == spl_token::state::AccountState::Frozen;
        if current_frozen != self.frozen {
            msg!("❌ REENTRANCY ATTACK DETECTED: Freeze state changed during {} operation", operation_name);
            msg!("   Previous frozen state: {}", self.frozen);
            msg!("   Current frozen state: {}", current_frozen);
            return Err(ProgramError::InvalidAccountData);
        }

        msg!("✅ REENTRANCY PROTECTION: {} validated successfully", operation_name);
        msg!("   Balance change: {} -> {} (Δ{})", 
             self.balance, current_token_account.amount, actual_balance_change);

        Ok(())
    }
}

/// Wrapper for safely executing token transfer operations with reentrancy protection
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

    /// Execute the transfer with comprehensive reentrancy protection
    pub fn execute_with_protection<F>(self, transfer_fn: F) -> Result<(), ProgramError>
    where
        F: FnOnce() -> Result<(), ProgramError>,
    {
        msg!("🛡️ REENTRANCY PROTECTION: Starting {} operation", self.operation_name);

        // Capture pre-transfer snapshots
        let source_snapshot = TokenAccountSnapshot::capture(
            self.source_account, 
            &format!("{} Source", self.operation_name)
        )?;
        
        let destination_snapshot = TokenAccountSnapshot::capture(
            self.destination_account, 
            &format!("{} Destination", self.operation_name)
        )?;

        // Execute the actual transfer operation
        msg!("💸 Executing {} transfer of {} tokens", self.operation_name, self.amount);
        transfer_fn()?;

        // Validate post-transfer state
        source_snapshot.validate_changes(
            self.source_account,
            -(self.amount as i64),
            &format!("{} Source", self.operation_name),
        )?;

        destination_snapshot.validate_changes(
            self.destination_account,
            self.amount as i64,
            &format!("{} Destination", self.operation_name),
        )?;

        msg!("✅ REENTRANCY PROTECTION: {} completed safely", self.operation_name);
        Ok(())
    }
}

/// Wrapper for safely executing token mint operations with reentrancy protection
pub struct SafeTokenMint<'a> {
    pub destination_account: &'a AccountInfo<'a>,
    pub amount: u64,
    pub operation_name: String,
}

impl<'a> SafeTokenMint<'a> {
    /// Create a new safe mint wrapper
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

    /// Execute the mint with comprehensive reentrancy protection
    pub fn execute_with_protection<F>(self, mint_fn: F) -> Result<(), ProgramError>
    where
        F: FnOnce() -> Result<(), ProgramError>,
    {
        msg!("🛡️ REENTRANCY PROTECTION: Starting {} operation", self.operation_name);

        // Capture pre-mint snapshot
        let destination_snapshot = TokenAccountSnapshot::capture(
            self.destination_account, 
            &format!("{} Destination", self.operation_name)
        )?;

        // Execute the actual mint operation
        msg!("🪙 Executing {} mint of {} tokens", self.operation_name, self.amount);
        mint_fn()?;

        // Validate post-mint state
        destination_snapshot.validate_changes(
            self.destination_account,
            self.amount as i64,
            &format!("{} Destination", self.operation_name),
        )?;

        msg!("✅ REENTRANCY PROTECTION: {} completed safely", self.operation_name);
        Ok(())
    }
}

/// Wrapper for safely executing token burn operations with reentrancy protection
pub struct SafeTokenBurn<'a> {
    pub source_account: &'a AccountInfo<'a>,
    pub amount: u64,
    pub operation_name: String,
}

impl<'a> SafeTokenBurn<'a> {
    /// Create a new safe burn wrapper
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

    /// Execute the burn with comprehensive reentrancy protection
    pub fn execute_with_protection<F>(self, burn_fn: F) -> Result<(), ProgramError>
    where
        F: FnOnce() -> Result<(), ProgramError>,
    {
        msg!("🛡️ REENTRANCY PROTECTION: Starting {} operation", self.operation_name);

        // Capture pre-burn snapshot
        let source_snapshot = TokenAccountSnapshot::capture(
            self.source_account, 
            &format!("{} Source", self.operation_name)
        )?;

        // Execute the actual burn operation
        msg!("🔥 Executing {} burn of {} tokens", self.operation_name, self.amount);
        burn_fn()?;

        // Validate post-burn state
        source_snapshot.validate_changes(
            self.source_account,
            -(self.amount as i64),
            &format!("{} Source", self.operation_name),
        )?;

        msg!("✅ REENTRANCY PROTECTION: {} completed safely", self.operation_name);
        Ok(())
    }
}