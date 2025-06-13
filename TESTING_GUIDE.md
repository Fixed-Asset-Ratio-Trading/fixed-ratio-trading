# Testing Guide

## Overview

This project uses a modular test structure with independent test suites for different functionality areas. The tests have been refactored from a single 3,290-line file into focused, maintainable modules with shared utilities.

## Test Module Structure

```
tests/
├── common/                     # Shared utilities and helpers
│   ├── mod.rs                 # Common imports and logging setup
│   ├── setup.rs               # Test environment configuration
│   ├── tokens.rs              # Token creation and management
│   └── pool_helpers.rs        # Pool creation and verification
├── test_pool_creation.rs      # Pool initialization and validation
├── test_swaps.rs              # Token exchange functionality
├── test_security.rs           # Security parameters and pause features
├── test_delegates.rs          # Delegate management system
├── test_fees.rs               # Fee collection and withdrawal
└── test_utilities.rs          # Unit tests for utility functions
```

## Running Tests

### Run All Tests
```bash
# All tests with minimal logging (default)
cargo test

# All tests with debug logging for detailed output
RUST_LOG=debug cargo test

# All tests with error-only logging
RUST_LOG=error cargo test
```

### Run Individual Test Modules

#### Pool Creation Tests
```bash
# Pool initialization tests (both new and legacy patterns)
cargo test --test test_pool_creation

# Run with debug logging for troubleshooting
RUST_LOG=debug cargo test --test test_pool_creation
```

#### Token Swap Tests
```bash
# Token exchange and swap validation tests
cargo test --test test_swaps
```

#### Security Feature Tests
```bash
# Security parameters, pause/unpause functionality
cargo test --test test_security
```

#### Delegate Management Tests
```bash
# Delegate addition, removal, and authorization
cargo test --test test_delegates
```

#### Fee Management Tests
```bash
# Fee collection and withdrawal request tests
cargo test --test test_fees
```

#### Utility Function Tests
```bash
# Unit tests for helper functions and components
cargo test --test test_utilities
```

### Run Specific Test Cases

#### Single Test Function
```bash
# Run a specific test by name
cargo test test_initialize_pool_new_pattern

# Run with debug output
RUST_LOG=debug cargo test test_initialize_pool_new_pattern
```

#### Pattern-Based Test Selection
```bash
# Run all tests matching a pattern
cargo test delegate

# Run all pool creation tests
cargo test pool_creation

# Run all new pattern tests
cargo test new_pattern
```

### Run Tests in Parallel vs Sequential

```bash
# Run tests in parallel (default)
cargo test

# Run tests sequentially (useful for debugging)
cargo test -- --test-threads=1

# Run specific module sequentially with debug output
RUST_LOG=debug cargo test --test test_pool_creation -- --test-threads=1
```

## Test Categories and Patterns

### Pool Creation Tests
- **New Pattern (Recommended)**: `test_*_new_pattern()`
  - Single-instruction pool initialization
  - Atomic operation with better user experience
  
- **Legacy Pattern (Deprecated)**: `test_*_legacy_pattern()`
  - Two-instruction workaround for historical issues
  - Maintained for backward compatibility

### Pool Initialization Patterns

#### New Single-Instruction Pattern ✅ **RECOMMENDED**
```rust
// Creates pool atomically with InitializePool instruction
let config = create_pool_new_pattern(
    &mut banks_client,
    &payer,
    recent_blockhash,
    &primary_mint,
    &base_mint,
    &lp_token_a_mint,
    &lp_token_b_mint,
    Some(ratio),
).await?;
```

#### Legacy Two-Instruction Pattern ⚠️ **DEPRECATED**
```rust
// Creates pool using CreatePoolStateAccount + InitializePoolData
let config = create_pool_legacy_pattern(
    &mut banks_client,
    &payer,
    recent_blockhash,
    &primary_mint,
    &base_mint,
    &lp_token_a_mint,
    &lp_token_b_mint,
    Some(ratio),
).await?;
```

## Logging Configuration

### Environment Variables
- `RUST_LOG=error`: Error messages only (minimal output)
- `RUST_LOG=info`: Informational messages
- `RUST_LOG=debug`: Detailed debugging information
- `RUST_LOG=trace`: Maximum verbosity

### Default Behavior
- Tests use minimal logging by default unless `RUST_LOG` is set
- Debug logging can be enabled per test for troubleshooting
- All logging is controlled through the common utilities module

### Examples
```bash
# Minimal output for CI/automation
RUST_LOG=error cargo test

# Debug output for development
RUST_LOG=debug cargo test --test test_pool_creation

# Trace level for deep debugging
RUST_LOG=trace cargo test test_specific_failing_test
```

## Test Independence and Isolation

### Key Features
- **Isolated Environments**: Each test creates its own program test environment
- **Independent State**: No shared state between test modules or functions
- **Parallel Execution**: Tests can run in parallel safely
- **Shared Utilities**: Common functionality available through `tests/common/`

### Test Environment Setup
```rust
// Basic test environment
let env = start_test_environment().await;

// Pool-specific test context
let ctx = setup_pool_test_context(false).await;

// With debug logging
let ctx = setup_pool_test_context(true).await;
```

## Common Test Utilities

### Token Operations
```rust
use crate::common::*;

// Create test mints
create_test_mints(&mut banks_client, &payer, recent_blockhash, &[&mint1, &mint2]).await?;

// Setup user with token accounts
let (user, primary_account, base_account) = setup_test_user(
    &mut banks_client, &payer, recent_blockhash,
    &primary_mint.pubkey(), &base_mint.pubkey(), None
).await?;
```

### Pool Operations
```rust
// Normalize pool configuration (handles token ordering)
let config = normalize_pool_config(&primary_mint.pubkey(), &base_mint.pubkey(), ratio);

// Verify pool state matches expectations
verify_pool_state(&mut banks_client, &config, &owner, &lp_a_mint, &lp_b_mint).await?;
```

## Troubleshooting

### Common Issues

#### Compilation Errors
```bash
# Clean build to resolve dependency issues
cargo clean && cargo test --test test_module_name
```

#### Test Failures
```bash
# Run specific failing test with debug output
RUST_LOG=debug cargo test test_failing_function_name -- --nocapture

# Run sequentially to avoid race conditions
cargo test --test test_module_name -- --test-threads=1
```

#### Import Errors
- Ensure `mod common;` and `use common::*;` are present in test files
- Check that required types are imported from `fixed_ratio_trading::`

### Debug Techniques

#### Enable Verbose Output
```bash
# Show println! output from tests
cargo test -- --nocapture

# Combine with debug logging
RUST_LOG=debug cargo test test_name -- --nocapture
```

#### Isolate Specific Tests
```bash
# Run only one test to isolate issues
cargo test test_specific_name -- --exact

# Run tests matching pattern
cargo test "test_pool" -- --nocapture
```

## Best Practices

### Writing New Tests
1. Use the appropriate test module for the functionality being tested
2. Import common utilities: `use crate::common::*;`
3. Create isolated test environments for each test function
4. Use descriptive test names indicating the expected behavior
5. Add debug logging for complex test scenarios

### Test Organization
- **Unit tests**: Add to `test_utilities.rs`
- **Pool creation**: Add to `test_pool_creation.rs`
- **Trading functionality**: Add to `test_swaps.rs`
- **Security features**: Add to `test_security.rs`
- **Delegate management**: Add to `test_delegates.rs`
- **Fee operations**: Add to `test_fees.rs`

### Performance Considerations
- Use parallel execution for independent tests
- Use sequential execution (`--test-threads=1`) for debugging only
- Leverage shared utilities to avoid code duplication
- Clean up resources properly in test teardown

## Integration with CI/CD

### Recommended Commands
```bash
# Full test suite with minimal output
RUST_LOG=error cargo test

# Individual module validation
cargo test --test test_pool_creation
cargo test --test test_swaps
cargo test --test test_security
cargo test --test test_delegates
cargo test --test test_fees
cargo test --test test_utilities

# Ensure no compilation warnings
cargo test --all-features --all-targets
```

### Performance Optimization
- Run tests in parallel by default
- Use caching for `target/` directory
- Consider running different modules in parallel CI jobs 