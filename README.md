# Fixed Ratio Trading Pool Program

A Solana smart contract implementing fixed-ratio token trading pools with enhanced security features, liquidity management, and comprehensive testing. This program enables users to create pools where tokens trade at fixed exchange rates, providing predictable pricing and capital efficiency.

## Table of Contents
- [Overview](#overview)
- [Key Features](#key-features)
- [Quick Start](#quick-start)
- [Pool Mechanics](#pool-mechanics)
- [Fee Structure](#fee-structure)
- [Usage Examples](#usage-examples)
- [Testing](#testing)
- [API Documentation](#api-documentation)
- [Development](#development)
- [Security](#security)
- [Support](#support)

## Overview

This program implements **fixed-ratio trading pools** where tokens maintain constant exchange rates, unlike variable-rate AMMs. Each pool is defined by:

- **Multiple Token**: The abundant token in the exchange (higher count)
- **Base Token**: The valuable token in the exchange (count = 1)
- **Fixed Ratio**: How many multiple tokens equal one base token

### Example Pool Configurations
- **1 SOL = 100 USDC**: Base=SOL, Multiple=USDC, Ratio=100
- **1 BTC = 50,000 USDT**: Base=BTC, Multiple=USDT, Ratio=50,000
- **1 ETH = 3,000 DAI**: Base=ETH, Multiple=DAI, Ratio=3,000

## Key Features

### 🎯 **Fixed-Ratio Trading**
- Predictable exchange rates with no slippage
- Capital efficient liquidity provision
- Ideal for stable token pairs and algorithmic trading

### 🔒 **Enhanced Security**
- System-wide emergency pause mechanism
- Individual pool pause controls
- Comprehensive input validation
- Protection against economic attacks

### 🚀 **Modern Architecture**
- Single-instruction pool creation (atomic operations)
- Enhanced token normalization prevents duplicate pools
- Efficient PDA derivation and account management
- Solana best practices compliance

### 💧 **Liquidity Management**
- Asymmetric liquidity provision (deposit either token)
- Proportional LP token rewards
- Fee accumulation and withdrawal system
- Real-time liquidity tracking

### 🛠 **Developer Experience**
- Comprehensive client SDK
- Extensive test coverage (95%+)
- Multiple deployment patterns
- Rich documentation and examples

### 🚨 **Anti-Liquidity Fragmentation**
**CRITICAL**: This contract implements a critical invariant to prevent market fragmentation by ensuring only **ONE pool per token pair** can exist, regardless of token order or ratios.

**Benefits:**
- ✅ Prevents Market Fragmentation: All liquidity concentrated in one pool per token pair
- ✅ Eliminates User Confusion: Clear, unambiguous pool for each token pair
- ✅ Maximizes Liquidity Efficiency: No splitting of liquidity across equivalent pools
- ✅ Prevents Arbitrage Issues: No price discrepancies between equivalent pools
- ✅ Simplifies Integration: Clients only need to handle one pool per token pair

## Quick Start

### Prerequisites
- Rust 1.70+ with Solana toolchain
- Solana CLI tools
- Node.js 16+ (for dashboard)

### Install and Build
```bash
# Clone repository
git clone https://github.com/your-org/fixed-ratio-trading
cd fixed-ratio-trading

# Build smart contract
cargo build-bpf

# Run tests
cargo test

# Deploy locally
solana program deploy target/deploy/fixed_ratio_trading.so
```

### Basic Pool Creation
```rust
use fixed_ratio_trading::client_sdk::*;

// Create pool: 1 SOL = 100 USDC
let pool_config = PoolConfig::new(
    usdc_mint,  // Multiple token (abundant)
    sol_mint,   // Base token (valuable)
    100         // Multiple per base ratio
)?;

let pool_addresses = client.derive_pool_addresses(&pool_config);
let instruction = client.create_pool_instruction(
    &payer, &pool_config, &lp_token_a, &lp_token_b
)?;
```

## Pool Mechanics

### Token Normalization

All pools use **enhanced normalization** to prevent economic duplicates:

1. **Lexicographic Ordering**: Tokens are ordered by public key
2. **Canonical Ratios**: Equivalent ratios resolve to the same pool
3. **Economic Uniqueness**: Prevents "100 USDC per SOL" and "0.01 SOL per USDC" being separate pools

### Exchange Rate Formula

For a pool with ratio `R` (multiple tokens per base token):

- **Multiple → Base**: `output_base = input_multiple / R`
- **Base → Multiple**: `output_multiple = input_base × R`

### Liquidity Provider Rewards

LP tokens are minted proportionally to liquidity contribution:

```
LP_tokens = (deposit_amount × total_LP_supply) / current_liquidity
```

## Fee Structure

The Fixed Ratio Trading system implements **two distinct types of fees** to ensure sustainable operations while maintaining competitive trading costs:

### 1. **Contract Fees** (Fixed SOL Amounts) ⚡

These are **operational fees paid in Solana (SOL)** to cover transaction processing costs. They are **fixed amounts** that do not vary based on trade size or token values.

| Operation | Fee Amount | Purpose |
|-----------|------------|---------|
| **Pool Creation** | **1.15 SOL** | One-time fee for creating a new trading pool, including account setup and PDA creation |
| **Deposit/Withdrawal** | **0.0013 SOL** | Fee for liquidity operations (adding or removing liquidity from pools) |
| **Token Swap** | **0.00002715 SOL** | Fee for executing token swaps and updating pool state |

### 2. **Pool Fees** (Percentage-Based on Traded Assets) 📊

These are **trading fees paid as a percentage of the tokens being traded**. They generate revenue for pool operators and can be configured by the pool owner.

| Configuration | Fee Rate | Applied To |
|---------------|----------|------------|
| **Default Setting** | **0%** | No trading fees (free trading by default) |
| **Maximum Allowed** | **0.5%** | Maximum percentage fee that can be set |
| **Configurable Range** | **0% to 0.5%** | Pool owner can set any rate within this range |

#### **Pool Fee Examples:**

**Free Trading (0% fee - default):**
```rust
// User swaps 1000 USDC for SOL
// Pool Fee: 0 USDC (no trading fee)  
// User receives: SOL equivalent of full 1000 USDC at pool ratio
// Contract Fee: 0.00002715 SOL (separate operational fee)
```

**With Trading Fee (0.25%):**
```rust
// User swaps 1000 USDC for SOL  
// Pool Fee: 2.5 USDC (1000 × 0.0025)
// Effective Input: 997.5 USDC (1000 - 2.5 fee)
// User receives: SOL equivalent of 997.5 USDC at pool ratio
// Pool retains: 2.5 USDC (revenue for pool operator)
// Contract Fee: 0.00002715 SOL (separate operational fee)
```

### **Benefits of This Dual Fee Structure:**

✅ **Predictable Operational Costs**: Fixed SOL fees provide predictable transaction costs  
✅ **Competitive Trading**: 0% default trading fees encourage liquidity and volume  
✅ **Revenue Flexibility**: Pool operators can set trading fees based on market conditions  
✅ **Spam Protection**: Nominal SOL fees prevent abuse and spam transactions  
✅ **Sustainable Operations**: Fee collection supports long-term pool maintenance  
✅ **Transparent Pricing**: Clear separation between operational costs and trading fees  

## Governance-Controlled Architecture

### ⚖️ **Governance-Controlled Architecture**

This smart contract implements a **governance-controlled architecture** where fee management and security controls are centralized through system authority and prepared for **decentralized governance protocols**.

### **Authority Structure:**
- ✅ **System Authority Only**: All fee withdrawals controlled by system authority
- ✅ **Treasury System**: SOL fees flow to central treasury PDAs
- ✅ **Governance Ready**: Architecture prepared for governance protocol takeover
- ✅ **System-Wide Controls**: Pool security managed centrally by system authority

### **Fee Management Under Governance:**

#### **SOL Fees (Contract Fees):**
```bash
# Only system authority can withdraw SOL fees
WithdrawTreasuryFees {
    amount: 1000000000  # 1 SOL (0 = withdraw all available)
}
```

#### **Token Fees (Pool Fees):**
- **Current**: Token fees remain in pool vaults 
- **Future**: Will be managed by governance protocols
- **Access**: No individual pool owner access

#### **System-Wide Security:**
```bash
# Only system authority can pause/unpause entire system
PauseSystem { reason: "Emergency maintenance" }
UnpauseSystem
```

### **Benefits of Governance Architecture:**

✅ **Decentralized Governance**: Prepares for community-controlled protocol governance  
✅ **Fair Fee Distribution**: Prevents individual pool owners from extracting all value  
✅ **Protocol Sustainability**: Ensures fees support overall protocol development  
✅ **Security Centralization**: System-wide security controls prevent fragmented management  
✅ **Governance Tokens**: Enables future governance token distribution mechanisms  
✅ **Treasury Management**: Professional treasury management vs individual fee extraction  

### **Integration Notes:**

**For Pool Creators:**
- Pools remain fully functional for all trading operations
- Liquidity deposits and withdrawals work normally  
- Swaps execute with fee collection flowing to treasury
- Pool creation available to all users

**For Protocol Integration:**
- Implement system authority controls for emergency management
- Plan for governance protocol integration for fee management
- Use treasury system for SOL fee tracking and withdrawal
- Design governance token distribution mechanisms

**Future Governance Protocol:**
A separate governance smart contract will take over ownership of this contract and manage:
- Fee rate adjustments through community voting
- Fee distribution to governance token holders  
- Protocol parameter updates via governance proposals
- Treasury fund allocation and protocol development funding

## Usage Examples

### Pool Creation (Recommended)
```rust
// Single atomic instruction (preferred)
let instruction = PoolInstruction::InitializePool {
    multiple_per_base: 100,  // 1 base = 100 multiple
    pool_authority_bump_seed: bump,
    multiple_token_vault_bump_seed: multiple_bump,
    base_token_vault_bump_seed: base_bump,
};
```

### Deposit Liquidity
```rust
let deposit_ix = PoolInstruction::Deposit {
    deposit_token_mint: usdc_mint,  // Depositing USDC
    amount: 1000_000_000,          // 1000 USDC
};
```

### Perform Swap
```rust
let swap_ix = PoolInstruction::Swap {
    input_token_mint: usdc_mint,    // Swapping USDC
    amount_in: 100_000_000,         // 100 USDC
    minimum_amount_out: 990_000,    // Minimum 0.99 SOL
};
```

### Withdraw Liquidity
```rust
let withdraw_ix = PoolInstruction::Withdraw {
    withdraw_token_mint: sol_mint,  // Withdrawing SOL
    lp_amount_to_burn: 50_000_000, // Burn 50 LP tokens
};
```

### System Authority Operations
```rust
// System-wide pause/unpause controls (system authority only)
let pause_system_ix = PoolInstruction::PauseSystem {
    reason: "Emergency maintenance".to_string()
};

let unpause_system_ix = PoolInstruction::UnpauseSystem;

// Treasury fee withdrawal (system authority only)
let withdraw_treasury_ix = PoolInstruction::WithdrawTreasuryFees {
    amount: 0 // 0 = withdraw all available SOL fees
};

// Treasury management and analytics
let treasury_info_ix = PoolInstruction::GetTreasuryInfo {};
let consolidate_ix = PoolInstruction::ConsolidateTreasuries;
```

## Testing

### Comprehensive Test Suite
```bash
# Run all tests
cargo test

# Run specific test categories
cargo test test_pool_creation
cargo test test_liquidity_management  
cargo test test_security
```

### Test Coverage Areas
- ✅ Pool creation and initialization
- ✅ Liquidity deposits and withdrawals
- ✅ Token swaps and exchange rates
- ✅ Fee calculation and distribution
- ✅ Security controls and pause mechanisms
- ✅ Error handling and edge cases
- ✅ PDA derivation and account validation

### Browser Testing
The included dashboard provides browser-based testing:
```bash
cd dashboard
python3 -m http.server 8000
# Visit http://localhost:8000
```

## API Documentation

### REST API Endpoints

The dashboard provides REST API endpoints for pool interaction:

#### Pool Information
```http
GET /api/pools/{pool_id}
```

Response:
```json
{
  "pool": {
    "id": "uuid",
    "poolAddress": "base58_address",
    "tokenASymbol": "USDC",
    "tokenBSymbol": "SOL", 
    "ratioDisplay": "1 SOL = 100.00 USDC",
    "liquidityTokenA": 50000,
    "liquidityTokenB": 500,
    "totalLPTokens": 5000,
    "isActive": true
  }
}
```

#### Create Pool
```http
POST /api/pools
Content-Type: application/json

{
  "multipleTokenMint": "base58_address",
  "baseTokenMint": "base58_address", 
  "multiplePerBase": 100
}
```

### Smart Contract Instructions

All instructions are documented in `src/types/instructions.rs` with comprehensive parameter descriptions and examples.

## Development

### Project Structure
```
fixed-ratio-trading/
├── src/                     # Smart contract source
│   ├── processors/          # Instruction processors
│   ├── types/              # Type definitions
│   ├── state/              # Account state structures
│   └── utils/              # Utility functions
├── tests/                  # Test suite
├── dashboard/              # Web interface
├── FixedRatioTrading.Dashboard/  # .NET API server
└── docs/                   # Documentation
```

### Adding New Features

1. **Define Instruction**: Add to `src/types/instructions.rs`
2. **Implement Processor**: Create in `src/processors/`
3. **Add Dispatch**: Update `src/lib.rs`
4. **Write Tests**: Add to `tests/`
5. **Update Client SDK**: Modify `src/client_sdk.rs`

### Code Standards

- **Error Handling**: Use `ProgramResult` and detailed error messages
- **Validation**: Validate all inputs and PDAs
- **Documentation**: Document all public functions
- **Testing**: Achieve >95% test coverage
- **Security**: Follow Solana security best practices

## Security

### Security Features

- **Input Validation**: All parameters validated before processing
- **PDA Verification**: All program-derived addresses verified
- **Ownership Checks**: Token account ownership validated
- **Rent Exemption**: All accounts maintain rent exemption
- **System Pause**: Emergency stop mechanism for critical issues

### Audit Status

- ✅ **Internal Security Review**: Completed
- ⏳ **External Audit**: Scheduled
- ✅ **Fuzz Testing**: Ongoing
- ✅ **Economic Model Review**: Completed

### Known Limitations

- **Fixed Ratios Only**: Cannot change ratios after pool creation
- **No Impermanent Loss Protection**: Standard liquidity provider risks apply
- **Token Mint Dependency**: Pools tied to specific token mints

## Support

### Documentation
- 📖 **Technical Implementation**: [docs/TECHNICAL_IMPLEMENTATION.md](docs/TECHNICAL_IMPLEMENTATION.md)
- 🔄 **Migration Guide**: [docs/MIGRATION_GUIDE.md](docs/MIGRATION_GUIDE.md)
- 🛑 **System Pause Details**: [docs/SYSTEM_PAUSE.md](docs/SYSTEM_PAUSE.md)
- 🆕 **Recent Improvements**: [docs/RECENT_IMPROVEMENTS.md](docs/RECENT_IMPROVEMENTS.md)
- 🧪 **Testing Guide**: [docs/tests/TESTING_GUIDE.md](docs/tests/TESTING_GUIDE.md)
- 🔧 **Setup Guide**: [LOCAL_TEST_DEPLOYMENT_GUIDE.md](LOCAL_TEST_DEPLOYMENT_GUIDE.md)

### Community
- 💬 **Discord**: [Join our Discord](https://discord.gg/your-server)
- 🐛 **Issues**: [GitHub Issues](https://github.com/your-org/fixed-ratio-trading/issues)
- 📚 **Wiki**: [Project Wiki](https://github.com/your-org/fixed-ratio-trading/wiki)

### Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details on:

- Code standards and style
- Pull request process
- Issue reporting guidelines
- Development setup

---

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.

---

**⚠️ Disclaimer**: This software is in active development. Use at your own risk on mainnet. Always test thoroughly on devnet first. 