# Fixed Ratio Trading Contract - API Documentation

Welcome to the Fixed Ratio Trading Contract API documentation. This directory contains comprehensive documentation for developers integrating with the contract.

## 📚 Documentation Structure

### 1. [FIXED_RATIO_TRADING_API.md](./FIXED_RATIO_TRADING_API.md)
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

## 🚀 Getting Started

1. **Read the main API documentation** to understand available functions
2. **Use the quick reference** for rapid lookups during development
3. **Copy from instruction examples** to accelerate your integration

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
- Fee withdrawals (dynamic rate limiting)
- View treasury information
- Accept donations
- Consolidate pool fees

## ⚠️ Important Notes

1. **Basis Points**: All token amounts must be in the smallest unit
   - 1 SOL = 1,000,000,000 lamports
   - 1 USDC = 1,000,000 (6 decimals)

2. **Authority**: Most administrative functions require Program Upgrade Authority

3. **Fees**: 
   - Pool creation: 1.15 SOL (REGISTRATION_FEE)
   - Deposits/Withdrawals: 0.0013 SOL (DEPOSIT_WITHDRAWAL_FEE)
   - Swaps: 0.00002715 SOL (SWAP_CONTRACT_FEE)
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

- **v1.0** (Aug 5, 2025): Initial API documentation with realistic CU scaling
- Contract version: 0.14.1040

---

*This documentation reflects the refactored processor functions with consistent naming conventions following the pattern: `process_<category>_<action>`*