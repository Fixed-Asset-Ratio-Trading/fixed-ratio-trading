# Fixed Ratio Trading Pool

A Solana program implementing fixed-ratio token trading pools with enhanced security, owner-controlled operations, and comprehensive testing.

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

## 💰 Fee Structure

The Fixed Ratio Trading system implements **two distinct types of fees** to ensure sustainable operations while maintaining competitive trading costs:

### 1. **Contract Fees** (Fixed SOL Amounts) ⚡

These are **operational fees paid in Solana (SOL)** to cover transaction processing costs. They are **fixed amounts** that do not vary based on trade size or token values.

| Operation | Fee Amount | Purpose |
|-----------|------------|---------|
| **Pool Creation** | **1.15 SOL** | One-time fee for creating a new trading pool, including account setup and PDA creation |
| **Deposit/Withdrawal** | **0.0013 SOL** | Fee for liquidity operations (adding or removing liquidity from pools) |
| **Token Swap** | **0.0000125 SOL** | Fee for executing token swaps and updating pool state |

#### **Contract Fee Examples:**

```rust
// Example 1: Create a new USDC/SOL pool
// Contract Fee: 1.15 SOL (paid once during pool creation)

// Example 2: Add 1000 USDC liquidity to pool  
// Contract Fee: 0.0013 SOL (paid for the deposit operation)

// Example 3: Swap 500 USDC for SOL
// Contract Fee: 0.0000125 SOL (paid for the swap operation)
```

**Key Characteristics:**
- ✅ **Fixed Amounts**: Same fee regardless of transaction size
- ✅ **SOL-Denominated**: Always paid in Solana native token
- ✅ **Operational Cost Coverage**: Cover computational and storage costs
- ✅ **Spam Prevention**: Prevent abuse through nominal fees

### 2. **Pool Fees** (Percentage-Based on Traded Assets) 📊

These are **trading fees paid as a percentage of the tokens being traded**. They generate revenue for pool operators and can be configured by the pool owner.

| Configuration | Fee Rate | Applied To |
|---------------|----------|------------|
| **Default Setting** | **0%** | No trading fees (free trading by default) |
| **Maximum Allowed** | **0.5%** | Maximum percentage fee that can be set |
| **Configurable Range** | **0% to 0.5%** | Pool owner can set any rate within this range |

#### **Pool Fee Calculation:**
```rust
fee_amount = input_token_amount * fee_basis_points / 10_000

// Examples with different fee rates:
// Input: 1000 tokens, Fee: 0% → Fee: 0 tokens
// Input: 1000 tokens, Fee: 0.1% (10 basis points) → Fee: 1 token  
// Input: 1000 tokens, Fee: 0.25% (25 basis points) → Fee: 2.5 tokens
// Input: 1000 tokens, Fee: 0.5% (50 basis points) → Fee: 5 tokens
```

#### **Pool Fee Examples:**

**Scenario 1: Pool with 0% trading fee (default)**
```rust
// User swaps 1000 USDC for SOL
// Pool Fee: 0 USDC (no trading fee)  
// Effective Input: 1000 USDC (full amount)
// User receives: SOL equivalent of 1000 USDC at pool ratio
// Contract Fee: 0.0000125 SOL (separate operational fee)
```

**Scenario 2: Pool with 0.25% trading fee**
```rust
// User swaps 1000 USDC for SOL  
// Pool Fee: 2.5 USDC (1000 × 0.0025)
// Effective Input: 997.5 USDC (1000 - 2.5 fee)
// User receives: SOL equivalent of 997.5 USDC at pool ratio
// Pool retains: 2.5 USDC (revenue for pool operator)
// Contract Fee: 0.0000125 SOL (separate operational fee)
```

**Scenario 3: Pool with maximum 0.5% trading fee**
```rust
// User swaps 1000 USDC for SOL
// Pool Fee: 5 USDC (1000 × 0.005)  
// Effective Input: 995 USDC (1000 - 5 fee)
// User receives: SOL equivalent of 995 USDC at pool ratio
// Pool retains: 5 USDC (revenue for pool operator)
// Contract Fee: 0.0000125 SOL (separate operational fee)
```

#### **Pool Fee Management:**

**Setting Trading Fees:**
```rust
// Pool owner can modify trading fees immediately
let fee_change_ix = PoolInstruction::ChangeFee {
    new_fee_basis_points: 25 // Set to 0.25% (25 basis points)
};
```

**Fee Revenue Collection:**
```rust
// Pool owner can withdraw collected trading fees immediately
let withdraw_fees_ix = PoolInstruction::WithdrawPoolFees {
    token_mint: usdc_mint,
    amount: collected_usdc_fees,
};
```

### **Fee Structure Summary**

| Fee Type | Payment Method | Amount | When Applied | Purpose |
|----------|----------------|---------|--------------|----------|
| **Contract Fees** | Fixed SOL amounts | 1.15 SOL (pool creation)<br/>0.0013 SOL (liquidity)<br/>0.0000125 SOL (swaps) | All operations | Transaction processing costs |
| **Pool Fees** | Percentage of traded tokens | 0% to 0.5% configurable | Token swaps only | Revenue generation for pool operators |

### **Benefits of This Dual Fee Structure:**

✅ **Predictable Operational Costs**: Fixed SOL fees provide predictable transaction costs  
✅ **Competitive Trading**: 0% default trading fees encourage liquidity and volume  
✅ **Revenue Flexibility**: Pool operators can set trading fees based on market conditions  
✅ **Spam Protection**: Nominal SOL fees prevent abuse and spam transactions  
✅ **Sustainable Operations**: Fee collection supports long-term pool maintenance  
✅ **Transparent Pricing**: Clear separation between operational costs and trading fees  

### **For Developers and Integrators:**

```rust
// Always account for both fee types in your calculations:

// 1. Contract Fee (SOL) - paid separately by transaction sender
let required_sol_balance = user_sol_balance >= SWAP_FEE; // 0.0000125 SOL

// 2. Pool Fee (tokens) - deducted from input token amount  
let pool_fee = input_amount * pool.swap_fee_basis_points / 10_000;
let effective_input = input_amount - pool_fee;
let expected_output = calculate_swap_output(effective_input, pool_ratio);

// Total user cost:
// - SOL: 0.0000125 SOL (contract fee)
// - Tokens: pool_fee amount of input token (if pool fee > 0%)
```

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

// Get fee information
let fee_ix = PoolInstruction::GetFeeInfo;
```

**Provides:**
- 📊 **Pool State**: Comprehensive pool configuration data
- 💧 **Liquidity Info**: Token balances, exchange rates, TVL
- 💸 **Fee Info**: Fee rates, collected fees, available balances

### **Owner-Only Operations**
Simplified owner-controlled operations with immediate effect:

```rust
// Owner changes swap fee rate (immediate effect)
let change_fee_ix = PoolInstruction::ChangeFee {
    new_fee_basis_points: 25 // 0.25% fee
};

// Owner withdraws pool fees (immediate effect)
let withdraw_fees_ix = PoolInstruction::WithdrawPoolFees {
    token_mint: usdc_mint,
    amount: 1000,
};

// Owner pauses swap operations (immediate effect)
let pause_swaps_ix = PoolInstruction::PausePoolSwaps;

// Owner unpauses swap operations (immediate effect)
let unpause_swaps_ix = PoolInstruction::UnpausePoolSwaps;
```

**Features:**
- ⚡ **Immediate Effect**: All operations take effect immediately
- 🔐 **Owner-Only**: Only the pool owner can perform these operations
- 🎯 **Swap-Only Impact**: Pool pause only affects swap operations (deposits/withdrawals continue normally)
- 💧 **MEV Protection**: Automatic temporary pause during large withdrawals (≥5% of pool)

**Architectural Simplification:**
- ✅ **No Time Delays**: Immediate execution of all owner operations
- 🔐 **Owner-Only**: Only the pool owner can perform these operations
- ✅ **Swap-Only Scope**: Deposits and withdrawals unaffected by pool pause
- ✅ **Direct Control**: Pool owner has immediate control over all operations

**Use Cases:**
- 🔒 **Security Response**: Immediate response to detected issues
- 💰 **Fee Management**: Direct control over fee rates and withdrawals
- ⚡ **Operational Control**: Immediate pause/unpause capabilities

## 🛑 System-Wide Pause Functionality

The contract includes a comprehensive system-wide pause mechanism for emergency situations:

### System Authority Control
- **Pause System**: Authority can immediately pause all contract operations
- **Unpause System**: Authority can resume all contract operations
- **Emergency Response**: Instant response to security threats or critical bugs

### When System is Paused
- ❌ **Blocked**: ALL operations including swaps, liquidity, fees, pool creation
- ✅ **Allowed**: System state queries, info retrieval, system unpause operation

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
- **Owner-Only Control**: Only pool owners can manage fees and pause operations
- **Emergency Pause**: Pool owner can pause operations during security incidents
- **Rent Protection**: Automatic rent-exempt status maintenance

### Advanced Features
- **Swap Fee Configuration**: Owner-configurable fees (0-0.5%)
- **Fee Withdrawal**: Direct fee withdrawal by pool owners
- **Immediate Control**: All operations take effect immediately

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

### Owner Operations
- `ChangeFee` - **NEW**: Change swap fee rate (owner only)
- `WithdrawPoolFees` - **NEW**: Withdraw accumulated pool fees (owner only)
- `PausePoolSwaps` - **NEW**: Pause swap operations (owner only)
- `UnpausePoolSwaps` - **NEW**: Unpause swap operations (owner only)

### System-Wide Pause Management
- `PauseSystem` - **NEW**: Emergency pause of all contract operations
- `UnpauseSystem` - **NEW**: Resume all contract operations after emergency

### Helper Utilities
- `GetPoolStatePDA` - **NEW**: Compute pool state PDA address
- `GetTokenVaultPDAs` - **NEW**: Compute token vault PDA addresses

### View/Getter Instructions
- `GetPoolInfo` - **NEW**: Comprehensive pool state information
- `GetLiquidityInfo` - **NEW**: Liquidity and exchange rate data
- `GetFeeInfo` - **NEW**: Fee rates and collection data
- `WithdrawFees` - Withdraw accumulated SOL fees (owner only)

## Constants

### Fee-Related Constants
```rust
// Contract Fees (Fixed SOL amounts)
const REGISTRATION_FEE: u64 = 1_150_000_000;        // 1.15 SOL (pool creation)
const DEPOSIT_WITHDRAWAL_FEE: u64 = 1_300_000;      // 0.0013 SOL (liquidity operations)
const SWAP_FEE: u64 = 12_500;                       // 0.0000125 SOL (token swaps)

// Pool Fees (Percentage-based)
const MAX_SWAP_FEE_BASIS_POINTS: u64 = 50;          // 0.5% maximum trading fee
const FEE_BASIS_POINTS_DENOMINATOR: u64 = 10_000;   // Basis points conversion (1 bp = 0.01%)
```

### System Configuration Constants
```rust
// System-Wide Pause Constants
const SYSTEM_STATE_LEN: usize = 245;                // SystemState account size
const MAX_PAUSE_REASON_LENGTH: usize = 200;         // Maximum pause reason length
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
5. **Owner-Only Control**: Direct owner control over fees and pool operations
6. **Immediate Response**: No time delays for critical security operations
7. **System-Wide Pause**: Emergency pause capability for the entire contract
8. **Audit Trail**: Complete logging of all operations for transparency

For security issues, please review the code thoroughly and test extensively before mainnet deployment. 