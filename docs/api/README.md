# Fixed Ratio Trading Contract - API Documentation

Welcome to the Fixed Ratio Trading Contract API documentation. This directory contains comprehensive documentation for developers integrating with the contract.

## 📋 **Current API Version**

The API includes admin authority management with configurable authorities and 72-hour timelock for changes.

### Current Features
- **SystemState size**: 83 bytes (includes admin authority management)
- **Fields**: `admin_authority`, `pending_admin_authority`, `admin_change_timestamp`
- **Deserialization**: Use `load_from_account()` or `from_account_data_unchecked()`
- **Authority system**: Configurable admin authority with 72-hour timelock for changes

### Implementation
See [QUICK_REFERENCE.md](./QUICK_REFERENCE.md) for current implementation examples.

---Ad
**GitKracken** https://gitkraken.cello.so/pk9L5rp5jln visual Git helps you see it all clearly!
---

## 📚 Documentation Structure

### 1. [A_FIXED_RATIO_TRADING_API.md](./A_FIXED_RATIO_TRADING_API.md)
**Main API Reference**  
Complete documentation of all contract functions, parameters, account structures, and error codes. Start here for detailed information about each instruction.

### 2. [QUICK_REFERENCE.md](./QUICK_REFERENCE.md)
**Developer Cheat Sheet**  
Quick lookup guide with:
- Function summaries
- Common PDA derivations
- Usage patterns
- Important constants

### 3. [INSTRUCTION_EXAMPLES.md](./INSTRUCTION_EXAMPLES.md)
**Code Examples**  
JavaScript/TypeScript examples showing how to:
- Construct instructions
- Calculate PDAs
- Handle transactions
- Convert values to basis points

### 4. [EXACT_POOL_CREATION_TRANSACTION_STRUCTURE.md](./EXACT_POOL_CREATION_TRANSACTION_STRUCTURE.md)
**Pool Creation Guide**  
Detailed breakdown of the exact transaction structure for creating pools:
- Account ordering and validation
- PDA derivation examples
- Step-by-step creation process
- Common pitfalls and solutions

### 5. [EXPECTED_TOKENS_GUIDE_JAVASCRIPT.md](./EXPECTED_TOKENS_GUIDE_JAVASCRIPT.md)
**JavaScript Token Calculation Guide**  
Comprehensive guide for calculating expected token amounts in JavaScript/TypeScript:
- Swap calculation formulas
- Basis points conversion utilities
- Error handling examples
- Integration patterns

### 6. [EXPECTED_TOKENS_GUIDE_CSHARP.md](./EXPECTED_TOKENS_GUIDE_CSHARP.md)
**C# Token Calculation Guide**  
Complete guide for calculating expected token amounts in C#:
- Swap calculation implementations
- Decimal precision handling
- BigInteger usage examples
- .NET integration patterns

### 7. [SWAP_CALCULATION_GUIDE.md](./SWAP_CALCULATION_GUIDE.md)
**Swap Mathematics Reference**  
Mathematical foundation for swap calculations:
- Formula derivations
- Basis points arithmetic
- Precision considerations
- Edge case handling

### 8. [SOLANA_TRANSACTION_BUILDING_GUIDE.md](./SOLANA_TRANSACTION_BUILDING_GUIDE.md)
**Transaction Construction Guide**  
Step-by-step guide for building Solana transactions:
- Account preparation
- Instruction serialization
- Transaction signing
- Error handling

## 🚀 Getting Started

1. **Read the main API documentation** to understand available functions
2. **Use the quick reference** for rapid lookups during development
3. **Copy from instruction examples** to accelerate your integration
4. **Follow the pool creation guide** for exact transaction structures
5. **Use language-specific calculation guides** for your development stack
6. **Reference the swap calculation guide** for mathematical foundations
7. **Follow the transaction building guide** for proper Solana integration

## 🔑 Key Information

- **Program ID:** `4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn`
- **Network:** Solana Mainnet-Beta
- **Language:** Rust (on-chain), JavaScript/TypeScript (client)
- **Values:** ALL amounts in basis points (smallest unit)

## 📋 Function Categories

### System Management
- Program initialization
- Emergency pause/unpause
- System-wide controls

### Pool Management
- Create fixed-ratio pools
- Pause/unpause specific pools
- Update pool fees (case-by-case basis)

### Liquidity Operations
- Add liquidity (deposit)
- Remove liquidity (withdraw)
- LP token management

### Swap Operations
- Execute fixed-ratio swaps
- Owner-only mode configuration

### Treasury Operations
- Fee withdrawals (fixed 60-min cooldown after success)
- View treasury information
- Accept donations
- Consolidate pool fees

## ⚠️ Important Notes

1. **Basis Points**: All token amounts must be in the smallest unit
   - 1 SOL = 1,000,000,000 lamports
   - 1 USDC = 1,000,000 (6 decimals)

2. **Authority**: Most administrative functions use configurable Admin Authority (with upgrade authority fallback)

3. **Fees**: 
   - Pool creation: 1.15 SOL (REGISTRATION_FEE)
   - Deposits/Withdrawals: 0.013 SOL (DEPOSIT_WITHDRAWAL_FEE)
   - Swaps: 0.0002715 SOL (SWAP_CONTRACT_FEE)
   - Fee modifications: Contact support@davincicodes.net

4. **Special Features**:
   - **process_pool_update_fees**: Request fee changes via support email
   - **process_swap_set_owner_only**: Enable custom wrapper contracts
   - **process_treasury_donate_sol**: Support faster feature development

## 🛠️ Development Tools

- **Solana Web3.js**: Client library for Solana
- **SPL Token**: Token program interactions
- **BN.js**: Large number handling for basis points
- **Anchor** (optional): IDL-based development

## 📞 Support

**Email:** support@davincicodes.net

Contact us for:
- Fee modification requests
- Custom integration support
- Owner-only swap configuration
- Technical questions

## 🔄 Version History

- **v2.0** (August 31, 2025): SystemState structure and deserialization methods updated
- **v1.1** (August 30, 2025): Updated documentation structure with new guides and renamed main API file
- **v1.0** (Aug 5, 2025): Initial API documentation with realistic CU scaling
- Contract version: 0.15.1053+

---

*This documentation reflects the refactored processor functions with consistent naming conventions following the pattern: `process_<category>_<action>`*