# MainNet Deployment Guide for Fixed Ratio Trading

## 🚀 Deployment Status: COMPLETED

**Program Successfully Deployed to MainNet**
- **Program ID**: `quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD`
- **Upgrade Authority**: Squad Multisig `i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi`
- **Admin Authority**: Hardware Wallet `4ekSqR4pNZ5hp4cRyicji1Yj7ZCphgkYQhwZf2ib9Wko`
- **Squad Management Interface**: [View Program in Squad](https://app.squads.so/squads/i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi/developer/programs/quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD)

**Key Links:**
- **Solana Explorer**: [quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD](https://explorer.solana.com/address/quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD)
- **Squad Multisig**: [i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi](https://explorer.solana.com/address/i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi)

---

## Critical Information

### Program IDs and Keys
- **MainNet Program ID**: `quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD`
  - Keypair: `/Users/davinci/code/keys/MainNet-quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD.json`
  - **CRITICAL**: Store this keypair in cold storage after deployment

- **Deployment Authority**: `3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB`
  - Keypair: `/Users/davinci/code/keys/3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB.json`
  - **MISSING**: This keypair needs to be generated

- **Initial Admin Authority**: `4ekSqR4pNZ5hp4cRyicji1Yj7ZCphgkYQhwZf2ib9Wko`
  - This will be set during system initialization
  - This is your hardware wallet (davincij15)

- **Final Squads Multisig**: `i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi`
  - ✅ **COMPLETED**: Upgrade authority successfully transferred
  - **Squad Interface**: https://app.squads.so/squads/i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi/developer/programs/quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD

## SOL Funding Requirements

### Minimum SOL Amounts Needed

1. **Deployment Authority Account** (`3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB`)
   - Program Deployment: ~5 SOL (for program account rent)
   - Transaction Fees: ~0.5 SOL (buffer for multiple transactions)
   - System Initialization: ~0.1 SOL
   - Authority Transfer: ~0.01 SOL
   - **Total Required**: **7 SOL minimum**
   - **Note**: Remaining SOL will be automatically transferred to Squads multisig after deployment

2. **Admin Authority Account** (`4ekSqR4pNZ5hp4cRyicji1Yj7ZCphgkYQhwZf2ib9Wko`)
   - **NOT REQUIRED** for deployment script execution
   - Only needed for future admin operations (can be funded later)

3. **Squads Multisig Account** (`i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi`)
   - Will receive remaining SOL from deployment authority automatically
   - Additional funding optional for future upgrades

### Total SOL Needed: **7 SOL** (for deployment authority only)

## Pre-Deployment Checklist

### 1. Required Keypairs

**CRITICAL - Transfer Deployment Authority Keypair:**

The deployment authority keypair for `3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB` must be transferred to:
```bash
/Users/davinci/code/keys/3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB.json
```

**Note**: The deployment script will NOT generate any keypairs. All required keypairs must exist with correct public keys before running the script.

Verify keypairs:
```bash
# Verify program keypair
solana-keygen pubkey /Users/davinci/code/keys/MainNet-quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD.json
# Must output: quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD

# Verify deployment authority keypair
solana-keygen pubkey /Users/davinci/code/keys/3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB.json
# Must output: 3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB
```

### 2. Fund Accounts

```bash
# Set to mainnet
solana config set --url https://api.mainnet-beta.solana.com

# Fund deployment authority (7 SOL minimum)
solana transfer 3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB 7 --from <YOUR_FUNDING_WALLET>

# Verify balance
solana balance 3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB
# Should show at least 7 SOL

# Note: Admin authority and Squads multisig funding is optional
# Remaining SOL from deployment authority will be transferred to Squads automatically
```

## Step-by-Step Deployment Process

### Step 1: Build for MainNet

```bash
cd /Users/davinci/code/fixed-ratio-trading

# Clean previous builds
cargo clean

# Build with MainNet feature flag (ensures only mainnet feature is active)
cargo build-bpf --features mainnet --no-default-features

# Verify the build
ls -la target/deploy/fixed_ratio_trading.so

# Calculate and record the hash
sha256sum target/deploy/fixed_ratio_trading.so
# Record this hash: _________________
```

### Step 2: Deploy Program

```bash
# Set deployment authority as default signer
solana config set --keypair /Users/davinci/code/keys/3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB.json

# Deploy the program
solana program deploy \
  target/deploy/fixed_ratio_trading.so \
  --program-id /Users/davinci/code/keys/MainNet-quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD.json \
  --url https://api.mainnet-beta.solana.com \
  --keypair /Users/davinci/code/keys/3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB.json

# Record deployment transaction signature: _________________
```

### Step 3: Verify Deployment

```bash
# Check program details
solana program show quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD --url https://api.mainnet-beta.solana.com

# Should show:
# Program Id: quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD
# Authority: 3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB

# Dump and verify program hash
solana program dump quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD dumped_mainnet.so --url https://api.mainnet-beta.solana.com
sha256sum dumped_mainnet.so
# Verify this matches the build hash from Step 1
```

### Step 4: Initialize System State (CRITICAL - Must be done BEFORE authority transfer)

```bash
# Run the initialization script
cd /Users/davinci/code/fixed-ratio-trading

# Create MainNet initialization script (see deploy.sh below)
./scripts/MainNet/deploy.sh initialize

# Or manually:
node scripts/initialize_system.js \
  quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD \
  https://api.mainnet-beta.solana.com \
  /Users/davinci/code/keys/3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB.json

# Record initialization transaction: _________________
```

### Step 5: Verify System State

```bash
# The initialization should have created:
# 1. SystemState PDA with admin_authority = 4ekSqR4pNZ5hp4cRyicji1Yj7ZCphgkYQhwZf2ib9Wko
# 2. MainTreasury PDA for fee collection

# You can verify this by checking the transaction on Solana Explorer
```

### Step 6: Transfer Upgrade Authority to Squads and Remaining SOL

**WARNING**: Only proceed after confirming successful system initialization!

```bash
# Transfer upgrade authority to Squads multisig
solana program set-upgrade-authority \
  quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD \
  --new-upgrade-authority i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi \
  --keypair /Users/davinci/code/keys/3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB.json \
  --url https://api.mainnet-beta.solana.com

# Record transfer transaction: _________________

# The deployment script will automatically transfer remaining SOL to the Squads multisig
# This ensures the deployment authority retains only minimal rent-exempt balance
```

### Step 7: Final Verification

```bash
# Verify the upgrade authority has been transferred
solana program show quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD --url https://api.mainnet-beta.solana.com

# Should show:
# Program Id: quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD
# Authority: i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi
```

### Step 8: Update Deployment Info

```bash
# Update deployment_info.json with MainNet details
# This should be done by the deploy.sh script automatically
```

### Step 9: Secure Keypairs

```bash
# CRITICAL: Secure the program ID keypair
# 1. Move to cold storage
# 2. Delete from online systems
# 3. Keep paper backup in secure location

# Remove online copy (after confirming backup)
rm /Users/davinci/code/keys/MainNet-quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD.json

# The deployment authority keypair can be kept for emergency use but should be secured
```

## Post-Deployment Verification

### Test Basic Operations

1. **Create a Test Pool** (optional, requires additional setup)
2. **Verify Admin Operations** with hardware wallet
3. **Test Pause/Unpause** functionality
4. **Monitor Treasury** accumulation

### Monitoring

- Program Account: https://explorer.solana.com/address/quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD
- Squads Multisig: https://explorer.solana.com/address/i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi
- Admin Authority: https://explorer.solana.com/address/4ekSqR4pNZ5hp4cRyicji1Yj7ZCphgkYQhwZf2ib9Wko

## Emergency Procedures

### If Initialization Fails

1. Do NOT transfer upgrade authority
2. Debug and fix the issue
3. Redeploy if necessary (while you still have upgrade authority)

### If Authority Transfer Fails

1. Verify you still have upgrade authority
2. Retry the transfer command
3. Consider using Squads UI for manual transfer

### Recovery Contacts

- Technical Support: info@davincicodes.net (Subject: FRT EMERGENCY)
- Squads Support: https://docs.squads.so/

## Important Notes

1. **NEVER** transfer upgrade authority before system initialization
2. **ALWAYS** verify each step before proceeding to the next
3. **BACKUP** all transaction signatures and hashes
4. **TEST** on Devnet first with the same procedure
5. **SECURE** keypairs immediately after deployment

## 🔧 Program Management & Upgrades

### Squad Multisig Control
The program upgrade authority has been transferred to Squad multisig for decentralized governance:

**Squad Interface Access:**
- **Direct Link**: [Fixed Ratio Trading Program](https://app.squads.so/squads/i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi/developer/programs/quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD)
- **Squad Dashboard**: [Main Squad Interface](https://app.squads.so/squads/i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi/home)

**Upgrade Process:**
1. Navigate to the Squad interface using the link above
2. Go to "Developer" → "Programs" section
3. Select the Fixed Ratio Trading program
4. Create upgrade proposals through Squad's interface
5. Squad members vote and execute upgrades through multisig

**Authority Structure:**
- **Upgrade Authority**: Squad Multisig (for program upgrades)
- **Admin Authority**: Hardware Wallet (for emergency pause/unpause)
- **Program Control**: Fully decentralized through Squad governance

### Emergency Controls
The admin authority (`4ekSqR4pNZ5hp4cRyicji1Yj7ZCphgkYQhwZf2ib9Wko`) retains emergency pause capabilities independent of Squad control.

---

## Deployment Record

**✅ COMPLETED DEPLOYMENT**

```
Deployment Date: September 26, 2025
Deployer: 3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB
Program ID: quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD
Binary Hash: _________________
Deploy TX: _________________
Initialize TX: _________________
Authority Transfer TX: _________________
Final Authority: i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi
Admin Authority: 4ekSqR4pNZ5hp4cRyicji1Yj7ZCphgkYQhwZf2ib9Wko
```
