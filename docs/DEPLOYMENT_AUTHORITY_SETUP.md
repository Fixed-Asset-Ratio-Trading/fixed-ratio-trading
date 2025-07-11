# Deployment Authority Setup Guide

This guide explains how to properly configure the program authority for production deployment.

## Overview

The Fixed Ratio Trading program uses a hardcoded program authority to ensure only authorized parties can initialize and control the system. This authority is defined in `src/constants.rs` and must be configured correctly for both testing and production deployment.

## Current Setup (Testing)

During development and testing, the program uses the actual program authority keypair:

```rust
// src/constants.rs
pub const PROGRAM_AUTHORITY: &str = "4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn";
pub const TEST_PROGRAM_AUTHORITY: &str = "4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn";
```

### Key Files Location

The program authority keypair is stored in the deployment directory:

- **Program Authority Keypair**: `target/deploy/PROGRAM_AUTHORITY-keypair.json`
- **Program ID Keypair**: `target/deploy/fixed_ratio_trading-keypair.json`

**Important**: Currently, both files contain the same keypair for development purposes. In production deployment, these should be different keypairs.

## Production Deployment Setup

**CRITICAL:** Before deploying to production, you MUST separate the program authority from the program ID and update the keypair files.

### Step 1: Generate Production Authority Keypair

Generate a new keypair for production deployment (different from the program ID):

```bash
solana-keygen new --outfile target/deploy/PROGRAM_AUTHORITY-keypair.json
```

### Step 2: Update Program Authority Constant

Extract the public key from your new production authority keypair:

```bash
cargo run --example get_pubkey_from_keypair target/deploy/PROGRAM_AUTHORITY-keypair.json
```

Update the `PROGRAM_AUTHORITY` constant in `src/constants.rs` with your production public key:

```rust
// src/constants.rs
pub const PROGRAM_AUTHORITY: &str = "YOUR_PRODUCTION_PUBLIC_KEY_HERE";
pub const TEST_PROGRAM_AUTHORITY: &str = "YOUR_PRODUCTION_PUBLIC_KEY_HERE";
```

### Step 3: Remove Test Authority Validation

**CRITICAL FOR PRODUCTION:** Remove the test authority validation from `src/processors/system_pause.rs`:

1. Remove the `TEST_PROGRAM_AUTHORITY` constant import
2. Remove the test authority validation logic  
3. Only allow the production `PROGRAM_AUTHORITY`

This ensures no test code remains in the production binary.

### Step 4: Verify Key Separation

Ensure your deployment uses different keypairs:

- **Program ID**: `target/deploy/fixed_ratio_trading-keypair.json` (used for program deployment)
- **Program Authority**: `target/deploy/PROGRAM_AUTHORITY-keypair.json` (used for program initialization and control)

**Security**: The program authority should be different from the program ID to maintain proper access control.

### Step 5: Secure the Private Key

  - Store the private key files in a secure location
- Consider using a hardware wallet for maximum security
- Implement multi-signature setup for critical operations
- NEVER commit the private key to version control

### Step 6: Deploy the Program

Deploy the program using the program ID keypair:

```bash
solana program deploy target/deploy/fixed_ratio_trading.so --program-id target/deploy/fixed_ratio_trading-keypair.json
```

### Step 7: Initialize the Program

After deployment, initialize the program using the program authority keypair:

```bash
# Use your client application or CLI tool to call InitializeProgram
# The authority account must match the PROGRAM_AUTHORITY constant
# Sign the transaction with target/deploy/PROGRAM_AUTHORITY-keypair.json
```

## Security Considerations

1. **Authority Validation**: The program will ONLY accept initialization from the hardcoded `PROGRAM_AUTHORITY` public key.

2. **No Test Code in Production**: The production build contains NO test-specific code or authorities.

3. **Single Point of Control**: Only the program authority can:
   - Initialize the program
   - Pause/unpause the entire system
   - Withdraw treasury fees

4. **Immutable After Deployment**: The program authority cannot be changed after deployment - it's hardcoded in the program binary.

## Testing vs Production

| Environment | Authority Source | Private Key |
|-------------|-----------------|-------------|
| Testing | `target/deploy/PROGRAM_AUTHORITY-keypair.json` | Publicly visible (test only) |
| Production | `target/deploy/PROGRAM_AUTHORITY-keypair.json` | Securely stored by deployer |

## Verification

To verify the correct authority is configured:

1. Check the `PROGRAM_AUTHORITY` constant in `src/constants.rs`
2. Ensure it matches your production keypair's public key
3. Confirm the private key is securely stored and accessible for deployment

## Emergency Procedures

If you lose access to the production authority keypair:

1. **System Pause**: The system cannot be paused without the authority keypair
2. **Treasury Access**: Treasury funds cannot be withdrawn without the authority keypair
3. **No Recovery**: There is no recovery mechanism - the authority is immutable

Therefore, it's CRITICAL to:
- Backup the authority keypair securely
- Consider multi-signature setups
- Test the authority access before production deployment

## Example Production Deployment

```bash
# 1. Generate production authority (different from program ID)
solana-keygen new --outfile target/deploy/PROGRAM_AUTHORITY-keypair.json

# 2. Extract public key and update PROGRAM_AUTHORITY in src/constants.rs
cargo run --example get_pubkey_from_keypair target/deploy/PROGRAM_AUTHORITY-keypair.json

# 3. Build the program
cargo build-bpf --manifest-path=Cargo.toml --bpf-out-dir=dist/program

# 4. Deploy the program using program ID keypair
solana program deploy dist/program/fixed_ratio_trading.so \
  --program-id target/deploy/fixed_ratio_trading-keypair.json

# 5. Initialize the program using program authority keypair
# (Use your client application with target/deploy/PROGRAM_AUTHORITY-keypair.json)
```

## Contact

For questions about deployment authority setup, please refer to the project documentation or contact the development team. 