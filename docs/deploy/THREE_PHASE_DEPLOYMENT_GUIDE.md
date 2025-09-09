# Three-Phase MainNet Deployment Guide

## Overview

The Fixed Ratio Trading MainNet deployment is split into three separate phases to minimize risk and provide recovery points at each stage. This approach prevents catastrophic losses by allowing verification and rollback at each step.

## Phase Structure

### Phase 1: Deployment & Initialization
- **Script**: `./scripts/MainNet/01_deploy.sh`
- **Purpose**: Compile, deploy, and initialize the program
- **Recovery**: Can redeploy if issues occur

### Phase 2: Verification
- **Script**: `./scripts/MainNet/02_verify.sh`
- **Purpose**: Create test tokens/pool to verify functionality
- **Recovery**: Can fix issues while maintaining upgrade authority

### Phase 3: Handoff
- **Script**: `./scripts/MainNet/03_handoff.sh`
- **Purpose**: Transfer control to Squads multisig
- **Recovery**: No recovery after this point - multisig control only

## SOL Requirements

### Updated Funding Requirements

**Total Required: 10 SOL** (includes buffer for token/pool creation in Phase 2)

| Account | Amount | Purpose | Phase |
|---------|--------|---------|--------|
| Deployment Authority (`3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB`) | 10 SOL | All deployment operations including verification | Phase 1-3 |
| Admin Authority (`4ekSqR4pNZ5hp4cRyicji1Yj7ZCphgkYQhwZf2ib9Wko`) | 0 SOL | Not needed for deployment | N/A |
| Squads Multisig (`i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi`) | 0 SOL | Receives remainder automatically | Phase 3 |

### SOL Usage Breakdown

**Phase 1 (Deployment & Initialization):**
- Program deployment: ~5 SOL
- System initialization: ~0.1 SOL
- Transaction fees: ~0.5 SOL
- **Remaining after Phase 1: ~4.4 SOL**

**Phase 2 (Verification):**
- Token creation (2 tokens): ~0.01 SOL
- Pool creation: ~1.15 SOL (registration fee)
- Transaction fees: ~0.05 SOL
- **Remaining after Phase 2: ~3.2 SOL**

**Phase 3 (Handoff):**
- Authority transfer: ~0.001 SOL
- Remaining SOL transferred to multisig: ~3.199 SOL
- **Final deployment authority balance: 0.001 SOL (rent-exempt)**

## Pre-Deployment Setup

### Required Keypairs

Both keypairs must exist with correct public keys before starting:

```bash
# Verify program keypair
solana-keygen pubkey /Users/davinci/code/keys/MainNet-quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD.json
# Must output: quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD

# Verify deployment authority keypair (you need to transfer this)
solana-keygen pubkey /Users/davinci/code/keys/3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB.json
# Must output: 3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB
```

### Funding

```bash
# Set to mainnet
solana config set --url https://api.mainnet-beta.solana.com

# Fund deployment authority (10 SOL minimum)
solana transfer 3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB 10 --from <YOUR_FUNDING_WALLET>

# Verify funding
solana balance 3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB
# Should show at least 10 SOL
```

## Deployment Process

### Phase 1: Deployment & Initialization

**What it does:**
1. Compiles program for MainNet with proper feature flags
2. Verifies all keypairs have correct public keys
3. Deploys program to MainNet
4. Initializes system state with admin authority `4ekSqR4pNZ5hp4cRyicji1Yj7ZCphgkYQhwZf2ib9Wko`
5. Records deployment information

**Command:**
```bash
cd /Users/davinci/code/fixed-ratio-trading
./scripts/MainNet/01_deploy.sh
```

**Success Indicators:**
- ✅ Program compiled successfully
- ✅ Program deployed to MainNet
- ✅ System state initialized
- ✅ Deployment record created
- ✅ Ready for Phase 2

**Recovery Options:**
- Can redeploy if compilation or deployment fails
- Can fix system initialization issues
- Upgrade authority remains with deployment key

### Phase 2: Verification

**What it does:**
1. Verifies Phase 1 completed successfully
2. Creates 2 test tokens with supply of 1 each
3. Creates a 1:1 pool with those tokens
4. Validates pool state and program functionality
5. Records verification results

**Command:**
```bash
./scripts/MainNet/02_verify.sh
```

**Success Indicators:**
- ✅ Test tokens created (2 tokens, supply of 1 each)
- ✅ Test pool created (1:1 ratio)
- ✅ Pool state validation passed
- ✅ Program functionality confirmed
- ✅ Ready for Phase 3

**Recovery Options:**
- Can debug and fix any program issues
- Can create different test scenarios
- Upgrade authority still with deployment key
- Can upgrade program if needed

### Phase 3: Handoff

**What it does:**
1. Verifies Phase 2 completed successfully
2. Transfers upgrade authority to Squads multisig
3. Transfers remaining SOL to multisig
4. Creates final deployment record
5. Provides security instructions

**Command:**
```bash
./scripts/MainNet/03_handoff.sh
```

**Success Indicators:**
- ✅ Upgrade authority transferred to Squads
- ✅ Remaining SOL transferred to multisig
- ✅ Final deployment record created
- ✅ System ready for production

**⚠️ NO RECOVERY AFTER THIS POINT**
- Upgrade authority is now with Squads multisig
- Any changes require multisig approval
- Program keypair must be secured immediately

## Safety Features

### Built-in Checks
- Each phase verifies the previous phase completed successfully
- Keypair public key verification before any operations
- Balance checks before expensive operations
- Transaction signature verification
- On-chain state validation

### Recovery Points
- **After Phase 1**: Can fix deployment or initialization issues
- **After Phase 2**: Can address functionality problems
- **After Phase 3**: No recovery - multisig control only

### Failure Handling
- Each script exits immediately on any error
- Detailed logging for debugging
- Transaction signatures recorded for audit trail
- Clear error messages with suggested fixes

## Key Addresses

| Component | Address | Role |
|-----------|---------|------|
| **Program ID** | `quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD` | The deployed program |
| **Deployment Authority** | `3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB` | Deploys and initially controls program |
| **Admin Authority** | `4ekSqR4pNZ5hp4cRyicji1Yj7ZCphgkYQhwZf2ib9Wko` | Controls pause/unpause and admin functions |
| **Squads Multisig** | `i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi` | Final upgrade authority |

## Monitoring and Verification

### Explorer Links
- **Program**: https://explorer.solana.com/address/quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD
- **Multisig**: https://explorer.solana.com/address/i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi
- **Admin**: https://explorer.solana.com/address/4ekSqR4pNZ5hp4cRyicji1Yj7ZCphgkYQhwZf2ib9Wko

### Squads Interface
- **URL**: https://app.squads.so/
- **Purpose**: Manage future program upgrades
- **Access**: Requires multisig member approval

## Security Considerations

### Program Keypair Security
**CRITICAL**: After Phase 3 completion, the program keypair must be secured:

1. **Backup**: Copy to secure, offline storage
2. **Verify**: Confirm backup is readable and correct
3. **Delete**: Remove online copy from deployment system
4. **Store**: Keep in secure, offline location (hardware wallet, paper backup, etc.)

### Authority Separation
- **Upgrade Authority**: Squads multisig (program upgrades)
- **Admin Authority**: Hardware wallet (pause/unpause, admin functions)
- **Emergency Control**: Admin can pause system independently

### Multisig Configuration
- **Type**: Squads multisig
- **Address**: `i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi`
- **Purpose**: Decentralized upgrade control
- **Interface**: User-friendly Squads web interface

## Files Created

### Phase 1 Files
- `deployment_info_mainnet_phase1.json` - Deployment record
- `mainnet_deployment_phase1.log` - Deployment log
- `.mainnet_init_info_phase1.json` - Initialization details

### Phase 2 Files
- `verification_info_mainnet_phase2.json` - Verification record
- `mainnet_verification_phase2.log` - Verification log
- `.mainnet_verification_results.json` - Detailed results

### Phase 3 Files
- `deployment_info_mainnet_final.json` - Comprehensive final record
- `mainnet_handoff_phase3.log` - Handoff log

## Timeline Estimate

| Phase | Estimated Time | Activities |
|-------|----------------|------------|
| **Setup** | 15 minutes | Transfer keypairs, fund accounts |
| **Phase 1** | 10 minutes | Compile, deploy, initialize |
| **Phase 2** | 15 minutes | Create tokens, create pool, verify |
| **Phase 3** | 10 minutes | Transfer authority, transfer SOL |
| **Cleanup** | 10 minutes | Secure keypairs, verify deployment |
| **Total** | **60 minutes** | Complete deployment process |

## Troubleshooting

### Common Issues

**Phase 1 Failures:**
- Keypair not found or wrong public key
- Insufficient SOL balance
- Network connectivity issues
- Compilation errors

**Phase 2 Failures:**
- Phase 1 not completed
- Token creation failures
- Pool creation failures
- Program functionality issues

**Phase 3 Failures:**
- Phase 2 not completed
- Authority transfer failures
- SOL transfer issues

### Getting Help
- **Technical Issues**: info@davincicodes.net (Subject: FRT MAINNET DEPLOYMENT)
- **Squads Support**: https://docs.squads.so/
- **Solana Status**: https://status.solana.com/

## Quick Reference

### Complete Deployment Commands
```bash
# Phase 1: Deploy and initialize
./scripts/MainNet/01_deploy.sh

# Phase 2: Verify functionality
./scripts/MainNet/02_verify.sh

# Phase 3: Handoff to multisig
./scripts/MainNet/03_handoff.sh
```

### Verification Commands
```bash
# Check program status
solana program show quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD --url mainnet-beta

# Check balances
solana balance 3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB --url mainnet-beta
solana balance i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi --url mainnet-beta

# Verify keypairs
solana-keygen pubkey /Users/davinci/code/keys/MainNet-quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD.json
solana-keygen pubkey /Users/davinci/code/keys/3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB.json
```

This three-phase approach ensures maximum safety while providing clear checkpoints for verification and recovery throughout the deployment process.
