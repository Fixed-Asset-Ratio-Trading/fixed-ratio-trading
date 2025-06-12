# Fixed Ratio Trading Pool

A Solana program implementing fixed-ratio token trading pools with enhanced security, delegate management, and comprehensive testing.

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

### Fee Management
- `WithdrawFees` - Owner withdraws accumulated SOL fees
- `SetSwapFee` - Configure trading fee rates (0-0.5%)

### Delegate System
- `AddDelegate` - Add authorized fee withdrawal delegate
- `RemoveDelegate` - Remove delegate authorization
- `WithdrawFeesToDelegate` - Execute delegate fee withdrawal
- `RequestFeeWithdrawal` - Request time-delayed fee withdrawal
- `CancelWithdrawalRequest` - Cancel pending withdrawal request
- `SetDelegateWaitTime` - Configure delegate-specific wait times

### Individual Pool Ratio Pausing
- `RequestPoolPause` - **NEW**: Delegate requests pool pause with structured reason
- `CancelPoolPause` - **NEW**: Cancel pending pool pause request (owner or delegate)
- `SetPoolPauseWaitTime` - **NEW**: Configure delegate-specific pause wait times

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