# System-Wide Pause Functionality

This document describes the comprehensive system-wide pause mechanism for emergency situations in the Fixed Ratio Trading Pool smart contract.

## Table of Contents
- [Overview](#overview)
- [System Authority Control](#system-authority-control)
- [Pause Architecture](#pause-architecture)
- [Implementation Details](#implementation-details)
- [Usage Examples](#usage-examples)
- [Integration Guide](#integration-guide)

## Overview

The contract includes a comprehensive system-wide pause mechanism for emergency situations that provides immediate response capabilities to security threats or critical bugs.

## System Authority Control

### System Pause Operations
- **Pause System**: Authority can immediately pause all contract operations
- **Unpause System**: Authority can resume all contract operations
- **Emergency Response**: Instant response to security threats or critical bugs

### When System is Paused
- ❌ **Blocked**: ALL operations including swaps, liquidity, fees, pool creation
- ✅ **Allowed**: System state queries, info retrieval, system unpause operation

## Pause Architecture

### Security Model
- **Single Point of Control**: Simple authority-based control for emergency situations
- **No Complex Governance**: No waiting periods during emergencies
- **Clear Response Capability**: Immediate emergency stop with clear audit trail
- **Hierarchical Precedence**: System pause takes precedence over pool-specific pause states

### System Pause vs Pool Pause

The system implements a layered pause architecture:

```rust
System Pause (Global) → Pool Pause (Individual) → Normal Operations
     ↑ TAKES PRECEDENCE     ↑ EXISTING              ↑ NORMAL FLOW
```

**System Pause:**
- 🌐 **Global**: Affects ALL pools and operations across the entire contract
- ⚡ **Immediate**: No waiting periods or governance delays
- 🔑 **Authority-Only**: Only system authority can pause/unpause
- 🚨 **Emergency**: Designed for critical security situations

**Pool Pause (Owner-Controlled):**
- 🎯 **Individual**: Affects specific pools only
- 👤 **Owner-Controlled**: Managed by pool owner only
- ⚡ **Immediate**: No waiting periods or delays
- 🏛️ **Operational**: Designed for routine operational control

## Implementation Details

### System State Account Structure

```rust
#[account]
pub struct SystemState {
    /// System authority (can pause/unpause entire system)
    pub authority: Pubkey,
    
    /// Whether the entire system is paused
    pub is_paused: bool,
    
    /// Timestamp when system was paused (0 if not paused)
    pub paused_at: i64,
    
    /// Reason for system pause (optional)
    pub pause_reason: String,
    
    /// Reserved space for future upgrades
    pub _reserved: [u8; 64],
}
```

### Authority Validation

```rust
pub fn validate_system_authority(
    authority_account: &AccountInfo,
    system_state: &SystemState,
) -> ProgramResult {
    if authority_account.key != &system_state.authority {
        return Err(ProgramError::InvalidAccountData);
    }
    
    if !authority_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    Ok(())
}
```

### Pause Validation Check

```rust
pub fn check_system_pause_status(
    system_state_account: Option<&AccountInfo>,
    program_id: &Pubkey,
) -> ProgramResult {
    if let Some(system_account) = system_state_account {
        // Validate system state account
        if system_account.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        
        let system_state = SystemState::try_from_slice(&system_account.data.borrow())?;
        
        if system_state.is_paused {
            return Err(PoolError::SystemPaused.into());
        }
    }
    
    Ok(())
}
```

## Usage Examples

### Emergency System Pause

```rust
// Emergency system pause (blocks ALL operations)
let pause_instruction = PoolInstruction::PauseSystem {
    reason: "Critical security vulnerability detected".to_string(),
};

// Build transaction
let transaction = Transaction::new_signed_with_payer(
    &[pause_instruction],
    Some(&authority.pubkey()),
    &[&authority],
    recent_blockhash,
);

// Send transaction
client.send_and_confirm_transaction(&transaction)?;
```

### Resume Operations

```rust
// Resume operations after issue resolution
let unpause_instruction = PoolInstruction::UnpauseSystem;

// Build transaction
let transaction = Transaction::new_signed_with_payer(
    &[unpause_instruction],
    Some(&authority.pubkey()),
    &[&authority],
    recent_blockhash,
);

// Send transaction
client.send_and_confirm_transaction(&transaction)?;
```

### Check System Status

```rust
// Query system pause status
let system_state_account = client.get_account(&system_state_pda)?;
let system_state = SystemState::try_from_slice(&system_state_account.data)?;

if system_state.is_paused {
    println!("System is paused: {}", system_state.pause_reason);
    println!("Paused at: {}", system_state.paused_at);
} else {
    println!("System is operational");
}
```

## Integration Guide

### For Developers

All operations now accept an optional system state account as the first account:

```rust
// Account structure for all operations
pub struct Accounts<'info> {
    /// Optional: System state account for pause validation
    #[account(mut)]
    pub system_state: Option<Account<'info, SystemState>>,
    
    /// User performing the operation
    #[account(mut)]
    pub user: Signer<'info>,
    
    /// Pool being operated on
    #[account(mut)]
    pub pool_state: Account<'info, PoolState>,
    
    // ... other required accounts
}
```

### Account Setup

```rust
// New operations with system pause support
let accounts = vec![
    system_state_account,  // Optional: for system pause validation
    user_account,          // Required: user performing operation
    pool_state_account,    // Required: pool being operated on
    // ... other required accounts
];
```

### Error Handling

The system provides specific error codes for pause-related issues:

```rust
pub enum PoolError {
    /// System is paused - no operations allowed
    #[error("System is currently paused")]
    SystemPaused,
    
    /// Trying to pause already paused system
    #[error("System is already paused")]
    SystemAlreadyPaused,
    
    /// Trying to unpause non-paused system
    #[error("System is not paused")]
    SystemNotPaused,
    
    /// Non-authority attempting system pause/unpause
    #[error("Unauthorized access - only system authority can pause/unpause")]
    UnauthorizedAccess,
}
```

### Example Error Handling

```rust
match result {
    Err(ProgramError::Custom(error_code)) => {
        match PoolError::from_u32(error_code) {
            Some(PoolError::SystemPaused) => {
                println!("Operation failed: System is currently paused");
                // Handle pause gracefully
            },
            Some(PoolError::UnauthorizedAccess) => {
                println!("Access denied: Only system authority can pause/unpause");
            },
            _ => {
                println!("Other error occurred: {}", error_code);
            }
        }
    },
    Ok(result) => {
        // Handle success
    },
    Err(other_error) => {
        // Handle other errors
    }
}
```

### Backward Compatibility

When system state account is not provided, operations work normally:

```rust
// Legacy operation (no system pause check)
let accounts = vec![
    user_account,          // Required: user performing operation
    pool_state_account,    // Required: pool being operated on
    // ... other required accounts
];

// Modern operation (with system pause check)
let accounts = vec![
    system_state_account,  // Optional: enables system pause validation
    user_account,          // Required: user performing operation
    pool_state_account,    // Required: pool being operated on
    // ... other required accounts
];
```

### System Pause Errors

- `SystemPaused`: Returned when operation attempted during system pause
- `SystemAlreadyPaused`: Returned when trying to pause already-paused system
- `SystemNotPaused`: Returned when trying to unpause non-paused system
- `UnauthorizedAccess`: Returned when non-authority attempts system pause/unpause

### Best Practices

1. **Always Check System Status**: Include system state account in critical operations
2. **Handle Pause Gracefully**: Provide clear error messages to users
3. **Monitor System Events**: Watch for pause/unpause events
4. **Implement Retry Logic**: Retry operations after system unpause
5. **Emergency Contacts**: Maintain communication channels for emergency situations

### System Pause vs Pool Pause Precedence

```rust
// Precedence order (highest to lowest):
// 1. System Pause (blocks everything)
// 2. Pool Pause (blocks pool-specific operations)
// 3. Normal Operations

pub fn validate_operation_allowed(
    system_state: Option<&SystemState>,
    pool_state: &PoolState,
    operation_type: OperationType,
) -> ProgramResult {
    // Check system pause first (highest precedence)
    if let Some(system) = system_state {
        if system.is_paused {
            return Err(PoolError::SystemPaused.into());
        }
    }
    
    // Check pool pause second (lower precedence)
    if matches!(operation_type, OperationType::Swap) && pool_state.is_swap_paused {
        return Err(PoolError::PoolSwapsPaused.into());
    }
    
    Ok(())
}
``` 