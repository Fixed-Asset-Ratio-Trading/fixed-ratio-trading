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

### Phase 2: Funding (7 SOL Total Required)
- [ ] Fund deployment authority: 7 SOL minimum
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

### Phase 3: Handoff to Multisig
- [ ] Run: `./scripts/MainNet/03_handoff.sh`
- [ ] Verify upgrade authority transfer to Squads
- [ ] Verify remaining SOL transfer to multisig
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
2. **DO NOT** use the same keypairs for Devnet and MainNet
3. **DO NOT** keep program keypair online after deployment
4. **ALWAYS** verify each step before proceeding
5. **ALWAYS** test on Devnet first with identical process

## 📊 SOL Budget Breakdown

| Account | Purpose | Amount | Notes |
|---------|---------|--------|-------|
| Deployment Authority | Program deployment + fees | 7 SOL | Minimum required |
| Admin Authority | Future operations | 0 SOL | NOT required for deployment |
| Squads Multisig | Future upgrades | 0 SOL | Receives remainder automatically |
| **TOTAL REQUIRED** | | **7 SOL** | Only deployment authority needs funding |

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

# 3. Fund deployment authority (7 SOL minimum)
solana transfer 3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB 7 --from YOUR_FUNDING_WALLET --url mainnet-beta

# 4. Run Three-Phase Deployment
cd /Users/davinci/code/fixed-ratio-trading

# Phase 1: Deploy & Initialize
./scripts/MainNet/01_deploy.sh

# Phase 2: Verify Functionality (only after Phase 1 success)
./scripts/MainNet/02_verify.sh

# Phase 3: Handoff to Multisig (only after Phase 2 success)
./scripts/MainNet/03_handoff.sh
```

## 📝 Notes

- **Three-Phase Process**: Deployment is split into 3 phases for maximum safety
- **No Keypair Generation**: Scripts will NOT generate any keypairs - all must exist before running
- **Public Key Verification**: Both keypairs must have exact expected public keys or scripts will fail
- **Automatic SOL Transfer**: Phase 3 transfers remaining SOL from deployment authority to Squads multisig
- **Recovery Points**: Can recover/fix issues after Phase 1 and Phase 2, but not after Phase 3
- **Funding**: Only deployment authority needs funding (7 SOL minimum)

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
