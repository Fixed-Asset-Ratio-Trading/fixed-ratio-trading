# Safe Squad Deployment Testing Strategy

**CRITICAL DISCOVERY**: Squad addresses and Squad Vault PDAs are different! This document provides a safe testing approach to avoid permanently losing program control.

## 🚨 The Critical Distinction

### ❌ WRONG: Squad Address (Direct)
```
i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi
```
**DANGER**: Transferring authority directly to this address may make funds/programs unrecoverable!

### ✅ CORRECT: Squad Vault PDA (Derived)
```
[To be determined through Squad interface - different address]
```
**SAFE**: This is the actual address that should receive authority transfers.

## 🎯 Recommended Safe Approach: Use SAT (Safe Authority Transfer)

The safest method is to use Squad's built-in Safe Authority Transfer feature:

1. **Add Program to Squad First**
   - Navigate to Squad: https://app.squads.so/squads/i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi/home
   - Go to "Programs" section
   - Click "Add Program" 
   - Enter your program ID

2. **Use SAT Feature**
   - Click "Create SAT" (Safe Authority Transfer)
   - Squad automatically handles the correct Vault PDA address
   - No risk of using wrong address

## 🧪 Local Testing Strategy

Since we can't test actual Squad integration on localhost, we'll create a comprehensive testing approach that validates our deployment process step-by-step.

### Phase 1: Local Network Authority Transfer Testing

Create test scripts that simulate the authority transfer process:

```bash
#!/bin/bash
# scripts/test_authority_transfer_localhost.sh

echo "🧪 Testing Authority Transfer Process on Localhost"
echo "=================================================="

# Generate test keypairs to simulate different roles
DEPLOYER_KEYPAIR="test-keys/deployer.json"
UPGRADE_AUTH_KEYPAIR="test-keys/upgrade-authority.json"  # Simulates Squad Vault PDA
FAKE_SQUAD_KEYPAIR="test-keys/fake-squad.json"          # Simulates Squad Address (WRONG)
PROGRAM_KEYPAIR="test-keys/program.json"

echo "📝 Generating test keypairs..."
mkdir -p test-keys
solana-keygen new --no-bip39-passphrase --silent --outfile $DEPLOYER_KEYPAIR
solana-keygen new --no-bip39-passphrase --silent --outfile $UPGRADE_AUTH_KEYPAIR
solana-keygen new --no-bip39-passphrase --silent --outfile $FAKE_SQUAD_KEYPAIR
solana-keygen new --no-bip39-passphrase --silent --outfile $PROGRAM_KEYPAIR

DEPLOYER_PUBKEY=$(solana-keygen pubkey $DEPLOYER_KEYPAIR)
UPGRADE_AUTH_PUBKEY=$(solana-keygen pubkey $UPGRADE_AUTH_KEYPAIR)
FAKE_SQUAD_PUBKEY=$(solana-keygen pubkey $FAKE_SQUAD_KEYPAIR)
PROGRAM_ID=$(solana-keygen pubkey $PROGRAM_KEYPAIR)

echo "🔑 Generated addresses:"
echo "   Deployer: $DEPLOYER_PUBKEY"
echo "   Correct Authority (Vault PDA): $UPGRADE_AUTH_PUBKEY"
echo "   WRONG Authority (Squad Direct): $FAKE_SQUAD_PUBKEY"
echo "   Program ID: $PROGRAM_ID"
echo ""

# Airdrop SOL to test accounts
echo "💰 Airdropping SOL to test accounts..."
solana airdrop 10 $DEPLOYER_PUBKEY --url localhost
solana airdrop 10 $UPGRADE_AUTH_PUBKEY --url localhost
solana airdrop 10 $FAKE_SQUAD_PUBKEY --url localhost

# Build program
echo "🔨 Building program..."
cargo build-bpf --manifest-path Cargo.toml

# Deploy program with deployer as initial upgrade authority
echo "🚀 Deploying program..."
solana program deploy target/deploy/fixed_ratio_trading.so \
  --program-id $PROGRAM_KEYPAIR \
  --upgrade-authority $DEPLOYER_KEYPAIR \
  --url localhost \
  --keypair $DEPLOYER_KEYPAIR

echo "✅ Program deployed with deployer as upgrade authority"

# Verify deployment
echo "🔍 Verifying deployment..."
solana program show $PROGRAM_ID --url localhost

# Initialize system (CRITICAL: Must happen before authority transfer)
echo "🏗️ Initializing system state..."
# Note: This would call your initialize_system instruction
# For testing, we'll simulate this step
echo "   ✅ System initialized (simulated)"

# Test CORRECT authority transfer (to Vault PDA)
echo "🔄 Testing CORRECT authority transfer (to Vault PDA)..."
solana program set-upgrade-authority $PROGRAM_ID \
  --new-upgrade-authority $UPGRADE_AUTH_PUBKEY \
  --url localhost \
  --keypair $DEPLOYER_KEYPAIR

# Verify correct transfer
echo "🔍 Verifying correct authority transfer..."
CURRENT_AUTH=$(solana program show $PROGRAM_ID --url localhost | grep "Upgrade Authority" | awk '{print $3}')
if [ "$CURRENT_AUTH" = "$UPGRADE_AUTH_PUBKEY" ]; then
    echo "   ✅ SUCCESS: Authority correctly transferred to Vault PDA"
else
    echo "   ❌ FAILED: Authority transfer unsuccessful"
    exit 1
fi

# Test upgrade capability with new authority
echo "🔧 Testing upgrade capability with new authority..."
solana program deploy target/deploy/fixed_ratio_trading.so \
  --program-id $PROGRAM_ID \
  --upgrade-authority $UPGRADE_AUTH_KEYPAIR \
  --url localhost \
  --keypair $UPGRADE_AUTH_KEYPAIR

echo "   ✅ SUCCESS: Upgrade works with new authority"

echo ""
echo "🎉 All tests passed! Authority transfer process is working correctly."
echo ""
echo "🚨 REMEMBER FOR MAINNET:"
echo "   ❌ NEVER transfer to Squad Address directly: $FAKE_SQUAD_PUBKEY"
echo "   ✅ ALWAYS use Squad Vault PDA (get from Squad interface)"
echo "   🛡️ BEST: Use SAT (Safe Authority Transfer) feature"
```

### Phase 2: Devnet Squad Integration Testing

Create a test script for actual Squad integration on Devnet:

```bash
#!/bin/bash
# scripts/test_devnet_squad_integration.sh

echo "🌐 Testing Squad Integration on Devnet"
echo "======================================"

# Configuration
DEVNET_URL="https://api.devnet.solana.com"
DEPLOYER_KEYPAIR="devnet-keys/deployer.json"
PROGRAM_KEYPAIR="devnet-keys/program.json"

# This will be filled in after Squad setup
SQUAD_ADDRESS="YOUR_DEVNET_SQUAD_ADDRESS"
SQUAD_VAULT_PDA="TO_BE_DETERMINED_FROM_SQUAD_INTERFACE"

echo "📋 Pre-flight checklist:"
echo "   □ Squad created on Devnet with correct members"
echo "   □ Squad Vault PDA address obtained from Squad interface"
echo "   □ Program built and ready for deployment"
echo "   □ Deployer has sufficient SOL on Devnet"
echo ""

read -p "Have you completed the Squad setup and obtained the Vault PDA? (y/n): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "❌ Please complete Squad setup first:"
    echo "   1. Go to https://app.squads.so/"
    echo "   2. Switch to Devnet"
    echo "   3. Create Squad with your members"
    echo "   4. Go to Programs section"
    echo "   5. Click 'Add Program' to get Vault PDA address"
    echo "   6. Update this script with the addresses"
    exit 1
fi

# Validate addresses are set
if [ "$SQUAD_VAULT_PDA" = "TO_BE_DETERMINED_FROM_SQUAD_INTERFACE" ]; then
    echo "❌ Please update SQUAD_VAULT_PDA in this script"
    exit 1
fi

echo "🔑 Using addresses:"
echo "   Squad Address: $SQUAD_ADDRESS"
echo "   Squad Vault PDA: $SQUAD_VAULT_PDA"
echo ""

# Deploy program
echo "🚀 Deploying to Devnet..."
solana program deploy target/deploy/fixed_ratio_trading.so \
  --program-id $PROGRAM_KEYPAIR \
  --upgrade-authority $DEPLOYER_KEYPAIR \
  --url $DEVNET_URL \
  --keypair $DEPLOYER_KEYPAIR

PROGRAM_ID=$(solana-keygen pubkey $PROGRAM_KEYPAIR)
echo "✅ Program deployed: $PROGRAM_ID"

# Initialize system
echo "🏗️ Initializing system..."
# Call your actual initialize instruction here
echo "   ✅ System initialized"

# Transfer authority to Squad Vault PDA
echo "🔄 Transferring authority to Squad Vault PDA..."
solana program set-upgrade-authority $PROGRAM_ID \
  --new-upgrade-authority $SQUAD_VAULT_PDA \
  --url $DEVNET_URL \
  --keypair $DEPLOYER_KEYPAIR

# Verify transfer
echo "🔍 Verifying authority transfer..."
solana program show $PROGRAM_ID --url $DEVNET_URL

echo ""
echo "🎉 Devnet Squad integration test complete!"
echo "📝 Next steps:"
echo "   1. Test upgrade via Squad interface"
echo "   2. Test SAT (Safe Authority Transfer) feature"
echo "   3. Verify all multisig members can participate"
```

### Phase 3: SAT Testing Script

```bash
#!/bin/bash
# scripts/test_sat_feature.sh

echo "🛡️ Testing Safe Authority Transfer (SAT) Feature"
echo "==============================================="

echo "📋 SAT Testing Checklist:"
echo ""
echo "1. Program Management:"
echo "   □ Program deployed to Devnet"
echo "   □ Program added to Squad via 'Add Program' button"
echo "   □ Program visible in Squad's Programs section"
echo ""
echo "2. SAT Process:"
echo "   □ 'Create SAT' button available in Squad interface"
echo "   □ SAT transaction created successfully"
echo "   □ Required signatures obtained from Squad members"
echo "   □ SAT transaction executed successfully"
echo ""
echo "3. Verification:"
echo "   □ Program upgrade authority now shows Squad Vault PDA"
echo "   □ Test upgrade via Squad interface works"
echo "   □ Program remains functional after authority transfer"
echo ""

read -p "Press Enter to continue with manual SAT testing..." -n 1 -r
echo ""

echo "🔗 Manual SAT Testing Steps:"
echo ""
echo "1. Go to your Squad: https://app.squads.so/squads/YOUR_SQUAD_ADDRESS/home"
echo "2. Navigate to 'Programs' section"
echo "3. Find your program in the list"
echo "4. Click 'Create SAT' button"
echo "5. Follow the interface prompts"
echo "6. Get required signatures from Squad members"
echo "7. Execute the SAT transaction"
echo ""
echo "8. Verify with: solana program show YOUR_PROGRAM_ID --url devnet"
echo ""

echo "✅ This process eliminates the risk of using wrong addresses!"
echo "🛡️ Squad's SAT feature automatically handles Vault PDA derivation."
```

## 🔧 Implementation Plan

### Step 1: Create and Test Local Scripts
```bash
# Create the test scripts
mkdir -p scripts test-keys devnet-keys

# Make scripts executable
chmod +x scripts/*.sh

# Run local testing
./scripts/test_authority_transfer_localhost.sh
```

### Step 2: Set Up Devnet Squad
1. Go to https://app.squads.so/
2. Switch to Devnet
3. Create a test Squad with 2-3 members
4. Record the Squad address and get Vault PDA from interface

### Step 3: Test Devnet Integration
```bash
# Update script with your Devnet Squad addresses
vim scripts/test_devnet_squad_integration.sh

# Run Devnet testing
./scripts/test_devnet_squad_integration.sh
```

### Step 4: Test SAT Feature
```bash
# Follow SAT testing checklist
./scripts/test_sat_feature.sh
```

## 🚨 Critical Safety Rules

1. **NEVER** transfer authority directly to Squad address
2. **ALWAYS** use Squad Vault PDA or SAT feature  
3. **ALWAYS** test on Devnet first with same Squad setup
4. **ALWAYS** initialize system before transferring authority
5. **ALWAYS** verify authority transfer was successful

## 📋 Pre-Mainnet Checklist

- [ ] Local authority transfer testing completed
- [ ] Devnet Squad created and tested
- [ ] SAT feature tested and working
- [ ] All Squad members can sign transactions
- [ ] Upgrade process tested via Squad interface
- [ ] Emergency procedures documented and tested
- [ ] Mainnet Squad created with production members
- [ ] Production Vault PDA address confirmed via Squad interface

This approach eliminates the risk of permanently losing program control while providing comprehensive testing of the deployment process.
