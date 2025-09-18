# MainNet Deployment Checklist - Three-Phase Process

**⚠️ IMPORTANT: This deployment now uses a three-phase process for maximum safety**
- **Phase 1**: Deploy & Initialize
- **Phase 2**: Verify functionality  
- **Phase 3**: Handoff to multisig

See `THREE_PHASE_DEPLOYMENT_GUIDE.md` for complete details.

## 🚨 CRITICAL MISSING ITEM

### Deployment Authority Keypair
**Status**: ❌ **MISSING - MUST BE TRANSFERRED**

The deployment authority keypair for `3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB` must be transferred to:
```bash
/Users/davinci/code/keys/3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB.json
```

**IMPORTANT**: 
- The script will NOT generate any keypairs
- The keypair must have the exact public key: `3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB`
- Script will verify both keypairs exist and have correct public keys before proceeding

## ✅ Existing Items

### 1. MainNet Program Keypair
- **File**: `/Users/davinci/code/keys/MainNet-quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD.json`
- **Status**: ✅ EXISTS
- **Public Key**: `quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD`

### 2. Compiler Directives
- **Status**: ✅ CONFIGURED
- **Location**: `src/lib.rs` lines 99-110
- **Build Command**: `cargo build-bpf --features mainnet --no-default-features`

### 3. Deployment Script
- **Status**: ✅ CREATED
- **Location**: `/scripts/MainNet/deploy.sh`
- **Features**: Full deployment automation with safety checks

### 4. Documentation
- **Status**: ✅ COMPLETE
- **Files**:
  - `/docs/deploy/MAINNET_DEPLOYMENT_GUIDE.md` - Step-by-step guide
  - `/docs/deploy/SECURE_DEPLOYMENT_STRATEGY.md` - Security strategy

## 📋 Pre-Deployment Checklist

### Phase 1: Preparation
- [ ] Generate or recover deployment authority keypair
- [ ] Verify deployment authority public key
- [ ] Backup all keypairs to secure location
- [ ] Test deployment process on Devnet first

### Phase 2: Funding (10 SOL Total Required)
- [ ] Fund deployment authority: 10 SOL minimum (includes Phase 2 verification costs)
- [ ] Admin authority: NOT required for deployment
- [ ] Squads multisig: Will receive remaining SOL automatically in Phase 3

## 📋 Three-Phase Deployment Process

### Phase 1: Deploy & Initialize
- [ ] Run: `./scripts/MainNet/01_deploy.sh`
- [ ] Verify program compilation for MainNet
- [ ] Verify program deployment
- [ ] Verify system initialization with admin authority
- [ ] Check Phase 1 completion status

### Phase 2: Verify Functionality
- [ ] Run: `./scripts/MainNet/02_verify.sh`
- [ ] Verify test token creation (2 tokens, supply of 1 each)
- [ ] Verify test pool creation (1:1 ratio)
- [ ] Verify pool state validation
- [ ] Check Phase 2 completion status

### Phase 3: Handoff to Multisig (USE SAT - SAFE AUTHORITY TRANSFER)
- [ ] **CRITICAL**: DO NOT run old `./scripts/MainNet/03_handoff.sh` (DANGEROUS!)
- [ ] Run: `./scripts/MainNet/03_handoff_safe.sh` (provides SAT instructions)
- [ ] Follow SAT process in Squad interface:
  - [ ] Add program to Squad via "Add Program" button
  - [ ] Use "Create SAT" feature (NOT manual authority transfer)
  - [ ] Get required multisig signatures
  - [ ] Execute SAT transaction
- [ ] Verify upgrade authority transferred to Squad Vault PDA (NOT Squad address)
- [ ] Transfer remaining SOL to multisig manually
- [ ] Secure program keypair in cold storage
- [ ] Delete online copies of sensitive keypairs
- [ ] Verify final deployment status

## 🔑 Key Addresses Summary

| Component | Address | Status |
|-----------|---------|--------|
| Program ID | `quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD` | ✅ Keypair exists |
| Deployment Authority | `3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB` | ❌ **NEEDS GENERATION** |
| Admin Authority | `4ekSqR4pNZ5hp4cRyicji1Yj7ZCphgkYQhwZf2ib9Wko` | ✅ Hardware wallet |
| Squads Multisig | `i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi` | ✅ Ready |

## ⚠️ Critical Warnings

1. **DO NOT** transfer upgrade authority before system initialization
2. **DO NOT** use manual `solana program set-upgrade-authority` with Squad addresses
3. **DO NOT** run old `./scripts/MainNet/03_handoff.sh` (contains dangerous authority transfer)
4. **DO NOT** use the same keypairs for Devnet and MainNet
5. **DO NOT** keep program keypair online after deployment
6. **ALWAYS** use Squad's SAT (Safe Authority Transfer) feature
7. **ALWAYS** verify each step before proceeding
8. **ALWAYS** test SAT process on Devnet first

## 📊 SOL Budget Breakdown

| Account | Purpose | Amount | Notes |
|---------|---------|--------|-------|
| Deployment Authority | Program deployment + verification | 10 SOL | Minimum required |
| Admin Authority | Future operations | 0 SOL | NOT required for deployment |
| Squads Multisig | Future upgrades | 0 SOL | Receives remainder automatically |
| **TOTAL REQUIRED** | | **10 SOL** | Only deployment authority needs funding |

## 🚀 Quick Start Commands

```bash
# 1. Transfer deployment keypair to required location
# Copy your deployment keypair for 3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB to:
# /Users/davinci/code/keys/3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB.json

# 2. Verify keypairs
solana-keygen pubkey /Users/davinci/code/keys/MainNet-quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD.json
# Must output: quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD

solana-keygen pubkey /Users/davinci/code/keys/3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB.json
# Must output: 3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB

# 3. Fund deployment authority (10 SOL minimum)
solana transfer 3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB 10 --from YOUR_FUNDING_WALLET --url mainnet-beta

# 4. Run Three-Phase Deployment
cd /Users/davinci/code/fixed-ratio-trading

# Phase 1: Deploy & Initialize
./scripts/MainNet/01_deploy.sh

# Phase 2: Verify Functionality (only after Phase 1 success)
./scripts/MainNet/02_verify.sh

# Phase 3: Handoff to Multisig (only after Phase 2 success)
# CRITICAL: Use safe SAT process instead of old script
./scripts/MainNet/03_handoff_safe.sh
```

## 📝 Notes

- **Three-Phase Process**: Deployment is split into 3 phases for maximum safety
- **No Keypair Generation**: Scripts will NOT generate any keypairs - all must exist before running
- **Public Key Verification**: Both keypairs must have exact expected public keys or scripts will fail
- **Automatic SOL Transfer**: Phase 3 transfers remaining SOL from deployment authority to Squads multisig
- **Recovery Points**: Can recover/fix issues after Phase 1 and Phase 2, but not after Phase 3
- **Funding**: Only deployment authority needs funding (10 SOL minimum)

## 🆘 Support

- Technical Issues: info@davincicodes.net (Subject: FRT MAINNET DEPLOYMENT)
- Squads Support: https://docs.squads.so/
- Solana Status: https://status.solana.com/

## 📅 Timeline Estimate

- Preparation: 30 minutes
- Funding: 10 minutes
- Build & Deploy: 20 minutes
- Initialize & Transfer: 15 minutes
- Verification: 15 minutes
- **Total: ~90 minutes**

Allow 2-3 hours for the complete process including safety checks and documentation.
