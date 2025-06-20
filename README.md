# Fixed Ratio Trading Pool

A Solana program implementing fixed-ratio token trading pools with enhanced security, delegate management, and comprehensive testing.

## 🚨 CRITICAL: Anti-Liquidity Fragmentation

**IMPORTANT**: This contract implements a critical invariant to prevent market fragmentation by ensuring only **ONE pool per token pair** can exist, regardless of token order or ratios.

### Why This Matters

Market fragmentation occurs when multiple pools exist for the same economic relationship, splitting liquidity and creating inefficiencies:

```rust
// ❌ THESE SCENARIOS ARE PREVENTED:
// Pool 1: TokenA/TokenB with ratio 3:1 (3 A per 1 B)  
// Pool 2: TokenB/TokenA with ratio 1:3 (1 B per 3 A) ← Same economic rate!

// Pool 1: USDC/SOL with ratio 100:1 (100 USDC per 1 SOL)
// Pool 2: SOL/USDC with any ratio ← BLOCKED - same token pair!
```

### How It Works

The contract uses **enhanced normalization** to detect and prevent economically equivalent pools:

1. **Token Normalization**: All token pairs are ordered lexicographically (A < B)
2. **Canonical Mapping**: Both "A/B" and "B/A" pools normalize to the same configuration
3. **PDA Collision**: Duplicate pools result in the same PDA, causing creation to fail
4. **Atomic Prevention**: Second pool creation fails at the account level

### Implementation Details

```rust
// Both of these pool configurations:
normalize_pool_config(&token_a, &token_b, ratio_x)  // Pool 1
normalize_pool_config(&token_b, &token_a, ratio_x)  // Pool 2

// Result in IDENTICAL normalized configuration:
// - Same token_a_mint (lexicographically smaller)
// - Same token_b_mint (lexicographically larger)  
// - Same pool_state_pda
// - Same canonical ratio representation
```

### Benefits

✅ **Prevents Market Fragmentation**: All liquidity concentrated in one pool per token pair  
✅ **Eliminates User Confusion**: Clear, unambiguous pool for each token pair  
✅ **Maximizes Liquidity Efficiency**: No splitting of liquidity across equivalent pools  
✅ **Prevents Arbitrage Issues**: No price discrepancies between equivalent pools  
✅ **Simplifies Integration**: Clients only need to handle one pool per token pair  

### For Developers

When creating pools, remember:
- ✅ **First pool created**: Successfully establishes the token pair
- ❌ **Second pool attempt**: Will fail with account already exists error
- 🔍 **Pool lookup**: Use either token order - both resolve to same pool
- 🎯 **Integration**: No need to worry about multiple pools for same pair

```rust
// Example: All these attempts reference the same pool
let pool_usdc_sol = derive_pool_pda(&usdc_mint, &sol_mint, ratio);
let pool_sol_usdc = derive_pool_pda(&sol_mint, &usdc_mint, ratio);
// pool_usdc_sol == pool_sol_usdc ✅

// Only the first creation succeeds
create_pool(&usdc_mint, &sol_mint, ratio_1); // ✅ Success
create_pool(&sol_mint, &usdc_mint, ratio_2); // ❌ Fails - same token pair
```

This design ensures **optimal liquidity concentration** and **market efficiency** while preventing the chaos of fragmented liquidity pools.

## Recent Improvements ✨

### **NEW: Single-Instruction Pool Initialization**
We've implemented a major improvement that replaces the deprecated two-instruction pattern with a modern single-instruction approach:

#### **Before (Deprecated):**
```rust
// Required 2 separate transactions
let create_ix = PoolInstruction::CreatePoolStateAccount { /* ... */ };
let init_ix = PoolInstruction::InitializePoolData { /* ... */ };
// Send transaction 1, then transaction 2
```

#### **After (Recommended):**
```rust
// Single atomic transaction
let init_ix = PoolInstruction::InitializePool { /* ... */ };
// Send single transaction ✅
```

**Benefits:**
- ✅ **Atomic Operation**: All-or-nothing execution prevents partial states
- ✅ **Simpler Integration**: Single instruction call vs. two separate transactions  
- ✅ **Better UX**: Reduces transaction costs and complexity for users
- ✅ **Eliminates Race Conditions**: No possibility of partial pool creation
- ✅ **Future-Proof**: Uses modern Solana best practices

### **NEW: Client SDK**
Added a comprehensive client SDK that simplifies integration:

```rust
use fixed_ratio_trading::client_sdk::*;

// Create pool client
let pool_client = PoolClient::new(program_id);

// Configure pool
let pool_config = PoolConfig::new(usdc_mint, sol_mint, 1000)?; // 1000:1 ratio

// Get pool creation instruction (single atomic operation)
let create_ix = pool_client.create_pool_instruction(&payer, &pool_config, &lp_a_mint, &lp_b_mint)?;

// Add liquidity 
let deposit_ix = pool_client.deposit_instruction(&user, &pool_config, &usdc_mint, 1000000, &user_source, &user_lp)?;
```

**SDK Features:**
- 🔧 **Automatic PDA Derivation**: No manual address calculation needed
- 🔧 **Account Preparation**: Automatically prepares all required accounts
- 🔧 **Type Safety**: Strongly typed interfaces prevent common mistakes
- 🔧 **Error Handling**: Clear error messages and validation
- 🔧 **Testing Support**: Built-in utilities for testing and debugging

### **NEW: Enhanced Deposit Instructions**
Added `DepositWithFeatures` for advanced use cases:

```rust
let enhanced_deposit_ix = pool_client.deposit_with_features_instruction(
    &user,
    &pool_config,
    &token_mint,
    amount,
    minimum_lp_tokens_out, // Slippage protection
    custom_fee_recipient,   // Optional custom fee recipient
    &user_source,
    &user_lp,
)?;
```

**Features:**
- 🛡️ **Slippage Protection**: Minimum LP token guarantees
- 💰 **Custom Fee Recipients**: Flexible fee distribution
- 🔍 **Enhanced Validation**: Additional checks and error handling

### **NEW: PDA Helper Utilities**
Simplified PDA derivation with helper instructions:

```rust
// Get pool state PDA
let get_pda_ix = PoolInstruction::GetPoolStatePDA {
    primary_token_mint: usdc_mint,
    base_token_mint: sol_mint,
    ratio_primary_per_base: 1000,
};

// Get token vault PDAs
let get_vaults_ix = PoolInstruction::GetTokenVaultPDAs {
    pool_state_pda: pool_pda,
};
```

### **NEW: Test-Specific View Instructions**
Easy access to pool state data for testing and debugging:

```rust
// Get comprehensive pool information
let pool_info_ix = PoolInstruction::GetPoolInfo;

// Get liquidity information
let liquidity_ix = PoolInstruction::GetLiquidityInfo;

// Get delegate information
let delegate_ix = PoolInstruction::GetDelegateInfo;

// Get fee information
let fee_ix = PoolInstruction::GetFeeInfo;
```

**Provides:**
- 📊 **Pool State**: Comprehensive pool configuration data
- 💧 **Liquidity Info**: Token balances, exchange rates, TVL
- 👥 **Delegate Info**: Delegate list, wait times, withdrawal history
- 💸 **Fee Info**: Fee rates, collected fees, available balances

### **NEW: Individual Pool Ratio Pausing**
Added sophisticated delegate-controlled pool pausing system for governance integration:

```rust
// Delegate requests pool pause for dispute resolution
let pause_request_ix = PoolInstruction::RequestPoolPause {
    reason: PoolPauseReason::RatioDispute,
    duration_seconds: 3600, // 1 hour pause
};

// Owner or delegate can cancel before activation
let cancel_pause_ix = PoolInstruction::CancelPoolPause;

// Owner configures delegate-specific wait times
let set_wait_time_ix = PoolInstruction::SetPoolPauseWaitTime {
    delegate: delegate_pubkey,
    wait_time: 3600, // 1 hour delay before activation
};
```

**Features:**
- 🔄 **Individual Delegate Control**: Each delegate can pause pools independently
- ⏱️ **Configurable Timing**: 1 minute to 72 hours wait times and durations
- 🏛️ **Governance Integration**: Structured reasons for dispute resolution
- 🛡️ **Owner Override**: Pool owner can cancel any delegate's pause request
- 🎯 **Bonding Support**: Designed for integration with bonding mechanisms

**Pause Reasons:**
- `RatioDispute` - Dispute over fixed ratio accuracy or fairness
- `InsufficientBond` - Insufficient bonding by pool participants
- `SecurityConcern` - General security concern requiring investigation
- `GovernanceAction` - Governance action or proposal execution
- `ManualIntervention` - Manual intervention by authorized delegate
- `Emergency` - Emergency response to detected issues

**Timing Model:**
1. **Request**: Delegate submits pause request with reason and duration
2. **Wait Period**: Configurable delay (1 minute to 72 hours) before activation
3. **Active Period**: Pool operations paused for specified duration
4. **Auto-Expiry**: Pause automatically lifts after duration completes

**Use Cases:**
- 💰 **Bonding Mechanisms**: Pause pool until bond requirements are met
- ⚖️ **Dispute Resolution**: Structured pause system for governance
- 🔒 **Security Response**: Rapid response to detected issues
- 🏛️ **Governance Integration**: Primitive for higher-layer governance contracts

### **NEW: Consolidated Delegate Management & Swap Fees** 🔄
Added a streamlined delegate management system with consolidated instructions and time-based authorization:

```rust
// Single instruction for all delegate actions
let request_ix = PoolInstruction::RequestDelegateAction {
    action_type: DelegateActionType::FeeChange,
    params: DelegateActionParams::FeeChange { 
        new_fee_basis_points: 25 // 0.25% fee
    },
};

// Execute after wait time
let execute_ix = PoolInstruction::ExecuteDelegateAction {
    action_id: pending_action_id,
};

// Owner can revoke before execution
let revoke_ix = PoolInstruction::RevokeAction {
    action_id: pending_action_id,
};

// Configure per-delegate time limits
let time_limits_ix = PoolInstruction::SetDelegateTimeLimits {
    delegate: delegate_pubkey,
    time_limits: DelegateTimeLimits {
        fee_change_wait_time: 3600,    // 1 hour for fee changes
        withdraw_wait_time: 86400,     // 24 hours for withdrawals
        pause_wait_time: 300,          // 5 minutes for pausing
    },
};
```

**Features:**
- 🔄 **Consolidated Instructions**: Single instruction for all delegate actions
- ⏱️ **Time-based Authorization**: Configurable wait times per delegate and action
- 🔍 **Action Tracking**: Complete history of all delegate actions
- 🛡️ **Enhanced Security**: Double validation for critical operations
- 📊 **Rate Limiting**: Prevents rapid successive changes

**Action Types:**
- `FeeChange` - Modify swap fee rates (0-0.5%)
- `Withdrawal` - Request fee withdrawal to delegate
- `PoolPause` - Pause pool operations

**Benefits:**
- ✅ **Simpler Integration**: Fewer instructions to handle
- ✅ **Better Security**: Consistent time-based authorization
- ✅ **More Flexibility**: Per-delegate configuration
- ✅ **Enhanced Governance**: Complete action history

### **Delegate Action Limits & Concurrency** ⚖️

The delegate management system implements sophisticated limits to prevent system overload while allowing operational flexibility:

#### **Concurrent Action Limits**
- **Per Delegate**: Each delegate can have up to **2 pending actions** simultaneously
- **System Total**: Maximum of **6 pending actions** across all delegates (3 delegates × 2 actions each)
- **Action Types**: Fee changes, withdrawals, and pool pauses can be pending concurrently

#### **When MaxPendingActionsReached Occurs**

The `MaxPendingActionsReached` error is triggered when the total number of pending actions across **all delegates** reaches 6:

```rust
// Example scenario that triggers the error:
// Delegate A: 2 pending actions (withdrawal + fee change)
// Delegate B: 2 pending actions (withdrawal + pool pause)  
// Delegate C: 2 pending actions (withdrawal + fee change)
// Total: 6 pending actions
// → Any new action request will fail with MaxPendingActionsReached
```

#### **Key Behaviors**

- **✅ Concurrent Requests**: Delegates do NOT need to wait for one action to complete before making another
- **✅ Mixed Action Types**: Different action types can be pending simultaneously per delegate
- **✅ Independent Execution**: Actions can be executed independently when their wait times expire
- **✅ Flexible Timing**: Each action type has configurable wait times per delegate

#### **Example Usage Patterns**

```rust
// Valid: Delegate can request multiple actions simultaneously
let withdrawal_request = request_delegate_action(DelegateActionType::Withdrawal, params1);
let fee_change_request = request_delegate_action(DelegateActionType::FeeChange, params2);
// Both succeed if under limits

// Valid: Actions execute independently
execute_delegate_action(withdrawal_action_id);  // Executes when withdrawal wait time expires
execute_delegate_action(fee_change_action_id);  // Executes when fee change wait time expires

// Invalid: Would exceed system limit
// If system already has 6 pending actions total:
let pool_pause_request = request_delegate_action(DelegateActionType::PoolPause, params3);
// → Fails with MaxPendingActionsReached
```

#### **Why These Limits Exist**

- **🛡️ Resource Protection**: Prevents unbounded growth of pending operations
- **📊 State Management**: Keeps on-chain state size manageable
- **⚡ Performance**: Ensures efficient processing and storage
- **🎯 Governance**: Prevents gridlock from excessive simultaneous requests
- **💾 Storage Optimization**: Limits on-chain account storage requirements

## 🛑 System-Wide Pause Functionality

The contract includes a comprehensive system-wide pause mechanism for emergency situations:

### System Authority Control
- **Pause System**: Authority can immediately pause all contract operations
- **Unpause System**: Authority can resume all contract operations
- **Emergency Response**: Instant response to security threats or critical bugs

### When System is Paused
- ❌ **Blocked**: ALL operations including swaps, liquidity, fees, pool creation, delegate actions
- ✅ **Allowed**: System state queries, info retrieval, system unpause operation

### Security Model
- **Single Point of Control**: Simple authority-based control for emergency situations
- **No Complex Governance**: No delegate voting or waiting periods during emergencies
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

**Pool Pause (Existing):**
- 🎯 **Individual**: Affects specific pools only
- 👥 **Delegate-Controlled**: Managed through delegate system
- ⏱️ **Time-Delayed**: Includes waiting periods and governance features
- 🏛️ **Governance**: Designed for routine governance and dispute resolution

### Example Usage

```rust
// Emergency system pause (blocks ALL operations)
let pause_instruction = PoolInstruction::PauseSystem {
    reason: "Critical security vulnerability detected".to_string(),
};

// Resume operations after issue resolution
let unpause_instruction = PoolInstruction::UnpauseSystem;
```

### Implementation Notes

**For Developers:**
- All operations now accept an optional system state account as the first account
- When provided, system pause validation takes precedence over all other checks
- When not provided, operations work normally (backward compatibility)
- System pause errors are clearly distinguished from pool pause errors

**Integration:**
```rust
// New operations with system pause support
let accounts = vec![
    system_state_account,  // Optional: for system pause validation
    user_account,          // Required: user performing operation
    pool_state_account,    // Required: pool being operated on
    // ... other required accounts
];
```

**Error Handling:**
- `SystemPaused`: Returned when operation attempted during system pause
- `SystemAlreadyPaused`: Returned when trying to pause already-paused system
- `SystemNotPaused`: Returned when trying to unpause non-paused system
- `UnauthorizedAccess`: Returned when non-authority attempts system pause/unpause

## Backward Compatibility

The legacy two-instruction pattern is still supported but marked as deprecated:

```rust
// ⚠️ DEPRECATED - Still works but not recommended
PoolInstruction::CreatePoolStateAccount { /* ... */ }
PoolInstruction::InitializePoolData { /* ... */ }

// ✅ RECOMMENDED - Use this instead
PoolInstruction::InitializePool { /* ... */ }
```

## Testing

Run the comprehensive test suite including new single-instruction pattern tests:

```bash
# Run all tests
cargo test

# Run only new pattern tests
cargo test test_initialize_pool_new_pattern
cargo test test_helper_functions_new_pattern

# Run with output
cargo test -- --nocapture
```

**Test Coverage:**
- ✅ **21 tests passing** - Complete functionality coverage
- ✅ **Legacy pattern tests** - Ensures backward compatibility
- ✅ **New pattern tests** - Validates improvements
- ✅ **Helper utility tests** - Verifies SDK functionality
- ✅ **Integration tests** - End-to-end validation

## Migration Guide

### For New Projects
Use the new single-instruction pattern and client SDK:

```rust
// 1. Use the client SDK
use fixed_ratio_trading::client_sdk::*;

// 2. Create pool with single instruction
let pool_client = PoolClient::new(program_id);
let config = PoolConfig::new(primary_mint, base_mint, ratio)?;
let create_ix = pool_client.create_pool_instruction(&payer, &config, &lp_a, &lp_b)?;

// 3. Use helper utilities
let deposit_ix = pool_client.deposit_instruction(/* ... */)?;
```

### For Existing Projects
You can migrate incrementally:

1. **Immediate**: Existing code continues to work with deprecated instructions
2. **Gradual**: Replace two-instruction calls with single-instruction calls
3. **Full**: Adopt client SDK for new features

## Program Features

### Core Functionality
- **Fixed-Ratio Trading**: Deterministic exchange rates between token pairs
- **Liquidity Provision**: Deposit tokens to earn LP tokens representing pool ownership
- **Token Swapping**: Exchange tokens at predetermined fixed ratios
- **Fee Collection**: Configurable fees for sustainability and growth

### Security Features
- **Delegate System**: Authorize up to 3 delegates for fee management
- **Time-Delayed Withdrawals**: Configurable wait times (5 minutes to 72 hours)
- **Emergency Pause**: Pool owner can pause operations during security incidents
- **Rent Protection**: Automatic rent-exempt status maintenance

### Advanced Features
- **Swap Fee Configuration**: Owner-configurable fees (0-0.5%)
- **Withdrawal History**: Complete audit trail of all fee withdrawals
- **Request/Cancel System**: Two-step withdrawal process for enhanced security
- **Individual Wait Times**: Per-delegate security policies

## Instructions

### Pool Management
- `InitializePool` - **NEW**: Single-instruction pool creation (recommended)
- `CreatePoolStateAccount` - **DEPRECATED**: Legacy pool creation step 1
- `InitializePoolData` - **DEPRECATED**: Legacy pool creation step 2
- `UpdateSecurityParams` - Update pool security settings

### User Operations
- `Deposit` - Add liquidity to receive LP tokens
- `DepositWithFeatures` - **NEW**: Enhanced deposit with slippage protection
- `Withdraw` - Remove liquidity by burning LP tokens
- `Swap` - Exchange tokens at fixed ratio

### Delegate Management (Updated)
- `RequestDelegateAction` - **NEW**: Consolidated delegate action requests
- `ExecuteDelegateAction` - **NEW**: Execute pending delegate action
- `RevokeAction` - **NEW**: Revoke pending delegate action
- `SetDelegateTimeLimits` - **NEW**: Configure per-delegate time limits
- `AddDelegate` - Add authorized delegate
- `RemoveDelegate` - Remove delegate authorization

### System-Wide Pause Management
- `PauseSystem` - **NEW**: Emergency pause of all contract operations
- `UnpauseSystem` - **NEW**: Resume all contract operations after emergency

### Helper Utilities
- `GetPoolStatePDA` - **NEW**: Compute pool state PDA address
- `GetTokenVaultPDAs` - **NEW**: Compute token vault PDA addresses

### View/Getter Instructions
- `GetPoolInfo` - **NEW**: Comprehensive pool state information
- `GetLiquidityInfo` - **NEW**: Liquidity and exchange rate data
- `GetDelegateInfo` - **NEW**: Delegate management information
- `GetFeeInfo` - **NEW**: Fee rates and collection data
- `GetWithdrawalHistory` - Fee withdrawal audit trail

## Constants

```rust
const REGISTRATION_FEE: u64 = 1_150_000_000;        // 1.15 SOL
const DEPOSIT_WITHDRAWAL_FEE: u64 = 1_300_000;      // 0.0013 SOL  
const SWAP_FEE: u64 = 12_500;                       // 0.0000125 SOL
const MAX_SWAP_FEE_BASIS_POINTS: u64 = 50;          // 0.5% maximum
const MAX_DELEGATES: usize = 3;                     // Maximum delegates
const MIN_WITHDRAWAL_WAIT_TIME: u64 = 300;          // 5 minutes
const MAX_WITHDRAWAL_WAIT_TIME: u64 = 259200;       // 72 hours

// Pool Pause Constants
const MIN_POOL_PAUSE_TIME: u64 = 60;                // 1 minute minimum
const MAX_POOL_PAUSE_TIME: u64 = 259200;            // 72 hours maximum
const DEFAULT_POOL_PAUSE_WAIT_TIME: u64 = 259200;   // 72 hours default wait

// New Delegate Management Constants
const MIN_ACTION_WAIT_TIME: u64 = 300;          // 5 minutes minimum
const MAX_ACTION_WAIT_TIME: u64 = 259200;       // 72 hours maximum
const DEFAULT_FEE_CHANGE_WAIT_TIME: u64 = 3600; // 1 hour default for fee changes
const MAX_PENDING_ACTIONS: usize = 6;           // Maximum pending actions across ALL delegates
const MAX_PENDING_ACTIONS_PER_DELEGATE: usize = 2; // Each delegate can have up to 2 concurrent actions

// System-Wide Pause Constants
const SYSTEM_STATE_LEN: usize = 245;            // SystemState account size
const MAX_PAUSE_REASON_LENGTH: usize = 200;     // Maximum pause reason string length
```

## Error Handling

The program provides comprehensive error handling with detailed error codes and messages:

```rust
pub enum PoolError {
    InvalidTokenPair { token_a: Pubkey, token_b: Pubkey, reason: String },
    InvalidRatio { ratio: u64, min_ratio: u64, max_ratio: u64 },
    InsufficientFunds { required: u64, available: u64, account: Pubkey },
    // ... and more
}
```

Each error includes:
- **Descriptive Messages**: Clear explanation of what went wrong
- **Error Codes**: Unique numeric codes for programmatic handling  
- **Context Information**: Relevant account addresses and values

## Development

### Building
```bash
cargo build-bpf
```

### Testing
```bash
# Run all tests
cargo test

# Run with logs
cargo test -- --nocapture

# Run specific test
cargo test test_initialize_pool_new_pattern
```

### Deployment
```bash
solana program deploy target/deploy/fixed_ratio_trading.so
```

## License

MIT License - see LICENSE file for details.

## Security

This program implements multiple layers of security:

1. **Authorization Checks**: All operations verify caller permissions
2. **Parameter Validation**: Comprehensive input validation and bounds checking
3. **Rent Protection**: Automatic maintenance of rent-exempt status
4. **Pause Mechanism**: Emergency stop capability for security incidents
5. **Individual Pool Pausing**: Delegate-controlled pause system with governance integration
6. **Time Delays**: Configurable wait times prevent immediate unauthorized access
7. **Structured Dispute Resolution**: Categorized pause reasons for governance systems
8. **Owner Override**: Pool owner can cancel any delegate's pause request for emergency resolution
9. **Audit Trail**: Complete logging of all operations for transparency

For security issues, please review the code thoroughly and test extensively before mainnet deployment. 