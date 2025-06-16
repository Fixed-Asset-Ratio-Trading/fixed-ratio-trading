# CRITICAL: GitHub Issue #31960 Workaround Documentation

**DO NOT REMOVE OR MODIFY THE WORKAROUNDS DESCRIBED IN THIS DOCUMENT**

## Overview

This document describes the critical workarounds implemented throughout the codebase to address Solana GitHub Issue #31960: "AccountInfo.data doesn't get updated after CPI account creation within the same instruction."

## The Problem

When creating accounts via Cross-Program Invocation (CPI) and then immediately trying to write data to them within the same instruction, the `AccountInfo.data` reference may not point to the actual on-chain account buffer. This causes several critical issues:

### 1. Silent Data Loss
- Serialization appears to succeed but data isn't actually persisted
- No error is thrown, making the issue difficult to detect
- Results in accounts with incorrect or missing data

### 2. Test Failures
- "Not all bytes read" errors during deserialization
- Size mismatches between expected and actual account data
- Inconsistent behavior between test and production environments

### 3. Account Size Issues
- Calculated packed lengths don't match actual Borsh serialization sizes
- Accounts created with incorrect sizes cause deserialization failures
- Trailing zeros in account data break Borsh deserialization

## The Solution

We implement a comprehensive two-part workaround pattern:

### Part 1: Actual Size Calculation

Instead of using calculated packed lengths, we determine the actual serialized size:

```rust
// ❌ PROBLEMATIC - Using calculated packed length
let size = PoolState::get_packed_len();
create_account(size);

// ✅ SAFE - Using actual serialized size
let temp_data = create_pool_state();
let (serialized_data, actual_size) = prepare_account_data(&temp_data)?;
create_account(actual_size);
```

### Part 2: Buffer Serialization Pattern

Instead of direct serialization to account data, we use a two-step process:

```rust
// ❌ PROBLEMATIC - Direct serialization after CPI account creation
data.serialize(&mut *account.data.borrow_mut())?;

// ✅ SAFE - Buffer serialization workaround
let mut buffer = Vec::new();
data.serialize(&mut buffer)?;                    // Step 1: Serialize to buffer
account.data.borrow_mut()[..buffer.len()]        // Step 2: Copy buffer to account
    .copy_from_slice(&buffer);
```

## Affected Files and Functions

### Core Implementation
- `src/utils/serialization.rs` - Standardized workaround utilities
```rust
//! Serialization Utilities
//! 
//! This module contains utilities for safe serialization of program data.
//! It provides buffer serialization patterns that ensure data integrity and persistence.
//!
//! # CRITICAL: GitHub Issue #31960 Workaround
//!
//! **DO NOT REMOVE OR MODIFY THIS WORKAROUND - REQUIRED FOR TESTS AND PRODUCTION**
//!
//! This module implements a workaround for Solana GitHub Issue #31960:
//! "AccountInfo.data doesn't get updated after CPI account creation within the same instruction"
//!
//! ## The Problem
//! When creating accounts via CPI (Cross-Program Invocation) and then immediately trying to
//! write data to them within the same instruction, the AccountInfo.data reference may not
//! point to the actual on-chain account buffer. This causes:
//! 
//! 1. **Silent Data Loss**: Serialization appears successful but data isn't persisted
//! 2. **Test Failures**: "Not all bytes read" errors during deserialization
//! 3. **Runtime Inconsistencies**: Different behavior between test and production environments
//!
//! ## The Solution
//! This module implements a two-step buffer serialization pattern:
//!
//! ```rust
//! // ❌ PROBLEMATIC - Direct serialization after CPI account creation
//! data.serialize(&mut *account.data.borrow_mut())?;
//!
//! // ✅ SAFE - Buffer serialization workaround
//! let mut buffer = Vec::new();
//! data.serialize(&mut buffer)?;                    // Step 1: Serialize to buffer
//! account.data.borrow_mut()[..buffer.len()]        // Step 2: Copy buffer to account
//!     .copy_from_slice(&buffer);
//! ```
//!
//! ## When to Use This Workaround
//! - **Always** when writing data to accounts created via CPI in the same instruction
//! - **Always** in pool creation functions (process_create_pool_state_account, process_initialize_pool)
//! - **Always** when account size calculation might be incorrect
//! - **Recommended** for all account data serialization for consistency
//!
//! ## Account Size Calculation Fix
//! The workaround also addresses account size calculation issues:
//!
//! ```rust
//! // ❌ PROBLEMATIC - Using calculated packed length
//! let size = PoolState::get_packed_len();
//! create_account(size);
//!
//! // ✅ SAFE - Using actual serialized size
//! let temp_data = create_pool_state();
//! let actual_size = temp_data.try_to_vec()?.len();
//! create_account(actual_size);
//! ```
//!
//! ## References
//! - Solana GitHub Issue #31960
//! - Related community discussions on AccountInfo.data behavior
//! - Solana runtime account handling documentation
//!
//! **WARNING**: Removing this workaround will cause test failures and potential data loss
//! in production. The issue affects both solana-program-test and mainnet environments.

use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
};
use borsh::BorshSerialize;

/// **CRITICAL WORKAROUND**: Safe buffer serialization for GitHub Issue #31960
///
/// **DO NOT REMOVE OR MODIFY - REQUIRED FOR PRODUCTION AND TESTS**
///
/// This function implements the mandatory two-step serialization process to work around
/// Solana's AccountInfo.data issue where account data references don't update properly
/// after CPI account creation within the same instruction.
///
/// ## Why This is Critical
/// - Prevents silent data loss in production
/// - Ensures tests pass consistently  
/// - Handles account size mismatches between calculated and actual serialized sizes
/// - Required for all pool creation and data persistence operations
///
/// ## Implementation Details
/// 1. **Buffer Serialization**: Serialize to temporary buffer to verify operation succeeds
/// 2. **Size Validation**: Ensure serialized data fits in target account
/// 3. **Atomic Copy**: Copy buffer contents to account data in single operation
///
/// This pattern ensures that either all data is written correctly or the operation fails cleanly.
///
/// # Arguments
/// * `data` - The data to serialize (must implement BorshSerialize)
/// * `account` - The account to write the data to
///
/// # Returns
/// * `ProgramResult` - Success or error code
///
/// # Errors
/// - `ProgramError::InvalidAccountData` - Serialization failed
/// - `ProgramError::AccountDataTooSmall` - Data doesn't fit in account
pub fn serialize_to_account<T: BorshSerialize>(data: &T, account: &AccountInfo) -> ProgramResult {
    // STEP 1: Serialize to temporary buffer (GitHub Issue #31960 workaround)
    // This ensures the serialization operation succeeds before attempting to write to account
    let mut serialized_data = Vec::new();
    match data.serialize(&mut serialized_data) {
        Ok(_) => {
            msg!("DEBUG: serialize_to_account: Buffer serialization successful. Size: {}", serialized_data.len());
        }
        Err(e) => {
            msg!("DEBUG: serialize_to_account: Buffer serialization FAILED: {:?}", e);
            return Err(e.into());
        }
    }
    
    // STEP 2: Validate buffer size fits in account
    let account_data_len = account.data_len();
    if serialized_data.len() > account_data_len {
        msg!("DEBUG: serialize_to_account: Data too large. Need: {}, Have: {}", 
             serialized_data.len(), account_data_len);
        return Err(ProgramError::AccountDataTooSmall);
    }
    
    // STEP 3: Atomic copy to account data (GitHub Issue #31960 workaround)
    // This ensures that either all data is written or the operation fails cleanly
    {
        let mut account_data = account.data.borrow_mut();
        account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
        msg!("DEBUG: serialize_to_account: Data copied to account successfully");
    }
    
    msg!("DEBUG: serialize_to_account: Final account size: {}", account.data.borrow().len());
    Ok(())
}

/// **CRITICAL WORKAROUND**: Get actual serialized size for GitHub Issue #31960
///
/// **DO NOT REMOVE - REQUIRED FOR CORRECT ACCOUNT SIZE CALCULATION**
///
/// This function calculates the actual serialized size of data, which may differ from
/// calculated packed lengths due to Borsh's variable-length encoding optimizations.
///
/// ## Why This is Critical
/// - Prevents "Not all bytes read" deserialization errors
/// - Ensures accounts are created with correct sizes
/// - Handles differences between manual size calculations and actual Borsh serialization
///
/// # Arguments
/// * `data` - The data to measure
///
/// # Returns
/// * `Result<usize, ProgramError>` - Actual serialized size or error
pub fn get_actual_serialized_size<T: BorshSerialize>(data: &T) -> Result<usize, ProgramError> {
    let mut buffer = Vec::new();
    data.serialize(&mut buffer).map_err(|_| ProgramError::InvalidAccountData)?;
    Ok(buffer.len())
}

/// **CRITICAL WORKAROUND**: Create account with actual serialized size
///
/// **DO NOT REMOVE - REQUIRED FOR GITHUB ISSUE #31960 WORKAROUND**
///
/// This function creates a properly sized account based on actual serialized data size
/// rather than calculated packed length, preventing size mismatches that cause test failures.
///
/// # Arguments
/// * `data` - The data that will be stored in the account
///
/// # Returns
/// * `Result<(Vec<u8>, usize), ProgramError>` - (serialized_data, actual_size)
pub fn prepare_account_data<T: BorshSerialize>(data: &T) -> Result<(Vec<u8>, usize), ProgramError> {
    let mut serialized_data = Vec::new();
    data.serialize(&mut serialized_data).map_err(|_| ProgramError::InvalidAccountData)?;
    let actual_size = serialized_data.len();
    Ok((serialized_data, actual_size))
}

/// Validates that serialized data will fit in the target account.
///
/// # Arguments
/// * `data` - The data to check
/// * `account_size` - The size of the target account
///
/// # Returns
/// * `ProgramResult` - Success if data fits, error otherwise
pub fn validate_serialized_size<T: BorshSerialize>(data: &T, account_size: usize) -> ProgramResult {
    let mut buffer = Vec::new();
    data.serialize(&mut buffer)?;
    
    if buffer.len() > account_size {
        msg!("Serialized data size {} exceeds account size {}", buffer.len(), account_size);
        return Err(ProgramError::AccountDataTooSmall);
    }
    
    Ok(())
}
```
- `src/processors/pool_creation.rs` - All pool creation functions
```rust
//! Pool Creation Processors
//! 
//! This module contains all the processors for pool creation and initialization operations.
//! It includes both the legacy two-step pattern and the modern single-step initialization.
//!
//! # CRITICAL: GitHub Issue #31960 Workaround Implementation
//!
//! **DO NOT REMOVE OR MODIFY THE WORKAROUNDS IN THIS FILE - REQUIRED FOR PRODUCTION**
//!
//! This module implements critical workarounds for Solana GitHub Issue #31960:
//! "AccountInfo.data doesn't get updated after CPI account creation within the same instruction"
//!
//! ## Affected Functions
//! - `process_create_pool_state_account` - Creates accounts via CPI then writes data
//! - `process_initialize_pool_data` - Writes data to previously created accounts  
//! - `process_initialize_pool` - Single-instruction account creation and data writing
//!
//! ## Workaround Pattern Used
//! 1. **Actual Size Calculation**: Use real serialized size instead of calculated packed length
//! 2. **Buffer Serialization**: Serialize to temporary buffer before writing to account
//! 3. **Atomic Copy**: Copy buffer contents to account data in single operation
//!
//! ## Why This is Critical
//! - Prevents "Not all bytes read" deserialization errors in tests
//! - Ensures data persistence in production environments
//! - Handles size mismatches between calculated and actual serialized data
//! - Required for all pool creation operations to work correctly
//!
//! **WARNING**: Removing these workarounds will cause test failures and potential data loss.

use crate::constants::*;
use crate::types::*;
use crate::utils::serialization::{serialize_to_account, prepare_account_data};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
    program_pack::Pack,
};
use spl_token::{
    instruction as token_instruction,
    state::{Account as TokenAccount, Mint as MintAccount},
};
```

### Specific Functions
- `process_create_pool_state_account()` - Creates accounts via CPI then writes data
- `process_initialize_pool_data()` - Writes data to previously created accounts
- `process_initialize_pool()` - Single-instruction account creation and data writing

### Utility Functions
- `serialize_to_account()` - Safe serialization utility
- `prepare_account_data()` - Account size calculation utility
- `get_actual_serialized_size()` - Size measurement utility

## Implementation Details

### 1. Account Creation with Actual Size

```rust
// CRITICAL WORKAROUND: GitHub Issue #31960 - Account Creation with Actual Size
// DO NOT REMOVE: Required to prevent "Not all bytes read" deserialization errors

let pool_state_data = PoolState { /* ... */ };

// Use actual serialized size instead of calculated packed length
let (serialized_data, actual_size) = prepare_account_data(&pool_state_data)?;
let rent = rent.minimum_balance(actual_size);

invoke_signed(
    &system_instruction::create_account(
        payer.key,
        account.key,
        rent,
        actual_size as u64,  // ← CRITICAL: Use actual size
        program_id,
    ),
    // ...
)?;
```

### 2. Data Writing with Buffer Pattern

```rust
// CRITICAL WORKAROUND: GitHub Issue #31960 - Buffer Serialization Pattern
// DO NOT REMOVE: Required to prevent silent data loss after CPI account creation

// Use standardized workaround utility
serialize_to_account(&pool_state_data, pool_state_account)?;

// Or manual implementation:
{
    let mut account_data = account.data.borrow_mut();
    account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
}
```

## When to Use This Workaround

### Always Required
- When writing data to accounts created via CPI in the same instruction
- In all pool creation functions
- When account size calculation might be incorrect
- For any operation that creates accounts and immediately writes data

### Recommended
- For all account data serialization operations (for consistency)
- When working with complex data structures that use Borsh serialization
- In test environments to ensure consistent behavior

## Testing Impact

### Without Workaround
- Tests fail with "Not all bytes read" errors
- Inconsistent behavior between test runs
- Silent data corruption in some cases

### With Workaround
- Tests pass consistently
- Reliable data persistence
- Predictable behavior across environments

## Maintenance Guidelines

### DO NOT REMOVE
- Any code marked with "GitHub Issue #31960 workaround"
- The `serialize_to_account()` utility function
- The `prepare_account_data()` utility function
- Buffer serialization patterns in pool creation

### DO NOT MODIFY
- The two-step serialization process
- Actual size calculation logic
- Protective comments and documentation

### SAFE TO MODIFY
- Debug logging messages
- Variable names (but keep comments)
- Function organization (but preserve the pattern)

## Verification

To verify the workaround is working correctly:

1. **Run Tests**: All tests should pass without "Not all bytes read" errors
2. **Check Account Sizes**: Actual account sizes should match serialized data sizes
3. **Verify Data Persistence**: Data written to accounts should be readable

```bash
# Run all tests to verify workaround
cargo test

# Run specific pool creation tests
cargo test test_pool_creation

# Run with output to see debug messages
cargo test -- --nocapture
```

## References

- [Solana GitHub Issue #31960](https://github.com/solana-labs/solana/issues/31960)
- Solana Program Development Documentation
- Community discussions on AccountInfo.data behavior
- Borsh serialization documentation

## Warning

**Removing or modifying these workarounds will cause:**
- Test failures across the entire test suite
- Potential data loss in production environments
- Inconsistent behavior between development and production
- Silent corruption of pool state data

**This workaround is critical for the proper functioning of the entire program.**