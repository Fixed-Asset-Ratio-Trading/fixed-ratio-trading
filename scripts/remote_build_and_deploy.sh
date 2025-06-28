#!/bin/bash
# Deploy Fixed Ratio Trading Contract to Remote Solana Validator
# This script builds the contract and deploys/upgrades the program to the remote validator
# Targets the remote validator at https://vmdevbox1.dcs1.cc

set -e

# Find the project root directory (where Cargo.toml is located)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Verify we found the correct project directory
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    echo "❌ Error: Could not find Cargo.toml in project root: $PROJECT_ROOT"
    echo "   Please run this script from the fixed-ratio-trading project directory or its subdirectories"
    exit 1
fi

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "🚀 Fixed Ratio Trading - Remote Deployment Script"
echo "=================================================="
echo "📂 Project Root: $PROJECT_ROOT"
echo ""
echo -e "${BLUE}🌐 Targeting Remote Validator: https://vmdevbox1.dcs1.cc${NC}"
echo -e "${BLUE}🎒 Backpack Address: 5GGZiMwU56rYL1L52q7Jz7ELkSN4iYyQqdv418hxPh6t${NC}"
echo -e "${BLUE}   Run './scripts/setup_backpack_keypair.sh' first if you need the keypair file${NC}"

# Check for required tools
echo -e "${YELLOW}🔧 Checking required tools...${NC}"
MISSING_TOOLS=""
command -v solana >/dev/null 2>&1 || MISSING_TOOLS="$MISSING_TOOLS solana"
command -v solana-keygen >/dev/null 2>&1 || MISSING_TOOLS="$MISSING_TOOLS solana-keygen"
command -v jq >/dev/null 2>&1 || echo "  Warning: jq not found (JSON parsing will be limited)"
command -v curl >/dev/null 2>&1 || echo "  Warning: curl not found (endpoint testing will be limited)"

if [ -n "$MISSING_TOOLS" ]; then
    echo -e "${RED}❌ Missing required tools:$MISSING_TOOLS${NC}"
    echo "   Please install the Solana CLI tools first"
    exit 1
fi
echo -e "${GREEN}✅ All required tools found${NC}"

# Configuration - Get program ID from the generated keypair
PROGRAM_KEYPAIR="$PROJECT_ROOT/target/deploy/fixed_ratio_trading-keypair.json"
if [ -f "$PROGRAM_KEYPAIR" ]; then
    PROGRAM_ID=$(solana-keygen pubkey "$PROGRAM_KEYPAIR")
else
    PROGRAM_ID="Will be generated during build"
fi
RPC_URL="https://vmdevbox1.dcs1.cc"
KEYPAIR_PATH="$HOME/.config/solana/id.json"

echo -e "${BLUE}📋 Configuration:${NC}"
echo "  Program ID: $PROGRAM_ID"
echo "  Remote RPC URL: $RPC_URL"
echo "  Keypair: $KEYPAIR_PATH"
echo ""

# Step 1: Test remote endpoint connectivity
echo -e "${YELLOW}🔍 Testing remote endpoint connectivity...${NC}"
if command -v curl >/dev/null 2>&1; then
    if curl -s -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' "$RPC_URL" | grep -q "ok"; then
        echo -e "${GREEN}✅ Remote endpoint is responding correctly${NC}"
    else
        echo -e "${RED}❌ Remote endpoint is not responding or not healthy${NC}"
        echo "   Please ensure the remote validator is running at $RPC_URL"
        exit 1
    fi
else
    echo -e "${YELLOW}⚠️  curl not found. Cannot test endpoint automatically${NC}"
fi

# Step 2: Check if build creates new changes
echo -e "${YELLOW}🔍 Checking if app was modified...${NC}"

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep '^version = ' "$PROJECT_ROOT/Cargo.toml" | head -1 | sed 's/version = "\(.*\)"/\1/')
echo "  Current version: $CURRENT_VERSION"

# Get timestamp of current build artifact (if it exists)
BUILD_ARTIFACT="$PROJECT_ROOT/target/deploy/fixed_ratio_trading.so"
if [ -f "$BUILD_ARTIFACT" ]; then
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS stat format
        OLD_TIMESTAMP=$(stat -f %m "$BUILD_ARTIFACT" 2>/dev/null || echo "0")
    else
        # Linux stat format
        OLD_TIMESTAMP=$(stat -c %Y "$BUILD_ARTIFACT" 2>/dev/null || echo "0")
    fi
    echo "  Previous build timestamp: $OLD_TIMESTAMP"
else
    OLD_TIMESTAMP="0"
    echo "  No previous build found"
fi

# Step 3: Initial build to check for changes
echo -e "${YELLOW}🔨 Running initial build to detect changes...${NC}"
cd "$PROJECT_ROOT"
RUSTFLAGS="-C link-arg=-zstack-size=131072" cargo build-sbf || true
if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Initial build failed${NC}"
    exit 1
fi

# Check if build artifact timestamp changed
if [ -f "$BUILD_ARTIFACT" ]; then
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS stat format
        NEW_TIMESTAMP=$(stat -f %m "$BUILD_ARTIFACT" 2>/dev/null || echo "0")
    else
        # Linux stat format
        NEW_TIMESTAMP=$(stat -c %Y "$BUILD_ARTIFACT" 2>/dev/null || echo "0")
    fi
    echo "  New build timestamp: $NEW_TIMESTAMP"
else
    NEW_TIMESTAMP="0"
fi

# Step 4: Determine if version should be incremented
VERSION_UPDATED=false
if [ "$NEW_TIMESTAMP" != "$OLD_TIMESTAMP" ] && [ "$NEW_TIMESTAMP" != "0" ]; then
    echo -e "${GREEN}✅ Changes detected - updating version number${NC}"
    
    # Parse version components (major.minor.patch)
    IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"
    
    # Increment patch version
    NEW_PATCH=$((PATCH + 1))
    NEW_VERSION="$MAJOR.$MINOR.$NEW_PATCH"
    
    echo "  New version: $NEW_VERSION"
    
    # Update Cargo.toml
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS sed
        sed -i '' "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "$PROJECT_ROOT/Cargo.toml"
    else
        # Linux sed
        sed -i "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "$PROJECT_ROOT/Cargo.toml"
    fi
    
    echo -e "${GREEN}✅ Version updated: $CURRENT_VERSION → $NEW_VERSION${NC}"
    VERSION_UPDATED=true
    
    # Step 5: Rebuild with new version
    echo -e "${YELLOW}🔨 Rebuilding with updated version...${NC}"
    RUSTFLAGS="-C link-arg=-zstack-size=131072" cargo build-sbf || true
    if [ $? -ne 0 ]; then
        echo -e "${RED}❌ Rebuild failed${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ Final build successful${NC}"
    
else
    echo -e "${BLUE}ℹ️  No changes detected - keeping current version${NC}"
    NEW_VERSION="$CURRENT_VERSION"
    echo -e "${GREEN}✅ Build successful (no changes)${NC}"
fi

echo ""

# Step 6: Configure Solana CLI for remote endpoint
echo -e "${YELLOW}⚙️  Configuring Solana CLI for remote endpoint...${NC}"
solana config set --url $RPC_URL
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ CLI configured for remote validator${NC}"
else
    echo -e "${RED}❌ CLI configuration failed${NC}"
    exit 1
fi

# Step 7: Check/create keypair
if [ ! -f "$KEYPAIR_PATH" ]; then
    echo -e "${YELLOW}🔑 Creating new keypair...${NC}"
    solana-keygen new --no-bip39-passphrase --outfile $KEYPAIR_PATH
fi

# Step 8: Check Backpack wallet balance
echo -e "${YELLOW}💰 Checking Backpack wallet balance...${NC}"
BACKPACK_WALLET="5GGZiMwU56rYL1L52q7Jz7ELkSN4iYyQqdv418hxPh6t"
DEFAULT_WALLET_ADDRESS=$(solana-keygen pubkey $KEYPAIR_PATH)
echo "  Default Wallet: $DEFAULT_WALLET_ADDRESS"
echo "  Backpack Wallet: $BACKPACK_WALLET"

# Check current balance
BACKPACK_BALANCE=$(solana balance $BACKPACK_WALLET 2>/dev/null | awk '{print $1}' | head -1)
# Fallback if balance command fails
if [ -z "$BACKPACK_BALANCE" ] || [ "$BACKPACK_BALANCE" = "Error:" ]; then
    BACKPACK_BALANCE="0"
fi

echo -e "${GREEN}  Current Backpack Wallet Balance: $BACKPACK_BALANCE SOL${NC}"

# Display current balance (no automatic funding)
echo -e "${GREEN}✅ Current Backpack wallet balance: $BACKPACK_BALANCE SOL${NC}"
FINAL_BALANCE=$BACKPACK_BALANCE

# Step 9: Check if program exists on remote and compare versions
echo -e "${YELLOW}🔍 Checking remote program status...${NC}"

DEPLOY_ACTION=""
DEPLOY_RESULT=""
REMOTE_VERSION=""

# Check if account/program already exists on remote
if [ "$PROGRAM_ID" != "Will be generated during build" ]; then
    echo "  Checking if account $PROGRAM_ID exists on remote..."
    
    # Check if any account exists at this address
    if solana account $PROGRAM_ID >/dev/null 2>&1; then
        echo "  Account exists on remote! Checking what type..."
        
        # Check if it's a program
        if solana program show $PROGRAM_ID >/dev/null 2>&1; then
            echo "  It's a program! Checking if it's upgradeable..."
            
            # Try to get program info for upgrade check
            if command -v jq >/dev/null 2>&1; then
                PROGRAM_INFO=$(solana program show $PROGRAM_ID --output json 2>/dev/null)
                if [ $? -eq 0 ]; then
                    IS_UPGRADEABLE=$(echo "$PROGRAM_INFO" | jq -r '.programdataAddress != null' 2>/dev/null)
                    echo "  Upgradeable check result: $IS_UPGRADEABLE"
                else
                    echo "  Could not get program info, assuming upgradeable"
                    IS_UPGRADEABLE="true"
                fi
            else
                echo "  jq not found, assuming program is upgradeable"
                IS_UPGRADEABLE="true"
            fi
            
            if [ "$IS_UPGRADEABLE" = "true" ]; then
                DEPLOY_ACTION="UPGRADE"
                echo -e "${BLUE}📈 UPGRADING existing program on remote...${NC}"
                echo "  Program exists and is upgradeable. Attempting upgrade..."
                
                # Attempt upgrade
                DEPLOY_OUTPUT=$(solana program deploy "$PROJECT_ROOT/target/deploy/fixed_ratio_trading.so" --program-id "$PROGRAM_KEYPAIR" --upgrade-authority "$KEYPAIR_PATH" 2>&1)
                DEPLOY_EXIT_CODE=$?
                
                # Check if Solana detected no changes
                if echo "$DEPLOY_OUTPUT" | grep -q "Program was not upgraded"; then
                    DEPLOY_RESULT="NO_UPGRADE_NEEDED"
                elif [ $DEPLOY_EXIT_CODE -eq 0 ]; then
                    DEPLOY_RESULT="UPGRADED"
                else
                    DEPLOY_RESULT="FAILED"
                fi
            else
                echo -e "${RED}❌ Program exists but is not upgradeable${NC}"
                echo "   Cannot upgrade immutable program on remote validator"
                echo "   A new program ID would be required"
                exit 1
            fi
        else
            echo -e "${RED}❌ Account exists but is not a program${NC}"
            echo "   Cannot deploy to existing non-program account on remote validator"
            exit 1
        fi
    else
        echo "  No account exists at this address on remote (expected for first deployment)"
        
        DEPLOY_ACTION="CREATE"
        echo -e "${BLUE}🆕 CREATING new program on remote...${NC}"
        echo "  Using initial deployment with upgrade authority..."
        
        DEPLOY_OUTPUT=$(solana program deploy "$PROJECT_ROOT/target/deploy/fixed_ratio_trading.so" --program-id "$PROGRAM_KEYPAIR" --upgrade-authority "$KEYPAIR_PATH" 2>&1)
        DEPLOY_EXIT_CODE=$?
        DEPLOY_RESULT=$([ $DEPLOY_EXIT_CODE -eq 0 ] && echo "CREATED" || echo "FAILED")
    fi
else
    DEPLOY_ACTION="CREATE"
    echo -e "${BLUE}🆕 CREATING new program on remote...${NC}"
    echo "  Using initial deployment with upgrade authority..."
    
    DEPLOY_OUTPUT=$(solana program deploy "$PROJECT_ROOT/target/deploy/fixed_ratio_trading.so" --upgrade-authority "$KEYPAIR_PATH" 2>&1)
    DEPLOY_EXIT_CODE=$?
    DEPLOY_RESULT=$([ $DEPLOY_EXIT_CODE -eq 0 ] && echo "CREATED" || echo "FAILED")
fi

# Display results with clear status
echo ""
echo -e "${BLUE}📋 REMOTE DEPLOYMENT SUMMARY${NC}"
echo "====================================="

case $DEPLOY_RESULT in
    "CREATED")
        echo -e "${GREEN}✅ STATUS: Program successfully CREATED on remote${NC}"
        echo -e "${GREEN}   🆕 New program deployed with upgrade authority${NC}"
        ;;
    "UPGRADED")
        echo -e "${GREEN}✅ STATUS: Program successfully UPGRADED on remote${NC}"
        echo -e "${GREEN}   📈 Contract code updated, program ID preserved${NC}"
        ;;
    "NO_UPGRADE_NEEDED")
        echo -e "${YELLOW}⚡ STATUS: No upgrade needed on remote${NC}"
        echo -e "${YELLOW}   📊 Program bytecode is already up-to-date${NC}"
        ;;
    "FAILED")
        echo -e "${RED}❌ STATUS: Remote deployment FAILED${NC}"
        echo -e "${RED}   💥 See error details below${NC}"
        echo ""
        echo "Error output:"
        echo "$DEPLOY_OUTPUT"
        exit 1
        ;;
esac

echo "   Action: $DEPLOY_ACTION"
echo "   Program ID: $PROGRAM_ID"
echo "   Remote RPC: $RPC_URL"
echo ""

if [ "$DEPLOY_RESULT" != "FAILED" ]; then
    echo -e "${GREEN}✅ Remote program deployment completed successfully!${NC}"
else
    echo -e "${RED}❌ Remote deployment failed${NC}"
    exit 1
fi

# Step 10: Get the actual deployed program ID and verify
echo -e "${YELLOW}🔍 Getting deployed program ID and verifying on remote...${NC}"
if [ -f "$PROGRAM_KEYPAIR" ]; then
    DEPLOYED_PROGRAM_ID=$(solana-keygen pubkey "$PROGRAM_KEYPAIR")
    echo -e "${GREEN}✅ Program ID: $DEPLOYED_PROGRAM_ID${NC}"
    
    # Verify deployment on remote
    PROGRAM_INFO=$(solana program show $DEPLOYED_PROGRAM_ID --output json 2>/dev/null)
    if [ $? -eq 0 ]; then
        PROGRAM_DATA_ADDRESS=$(echo $PROGRAM_INFO | jq -r '.programdataAddress // "N/A"')
        PROGRAM_SIZE=$(echo $PROGRAM_INFO | jq -r '.dataLen // "N/A"')
        echo -e "${GREEN}✅ Remote program verification successful${NC}"
        echo "  Program Data Address: $PROGRAM_DATA_ADDRESS"
        echo "  Program Size: $PROGRAM_SIZE bytes"
    else
        echo -e "${YELLOW}⚠️  Program deployed but verification data not immediately available${NC}"
    fi
    PROGRAM_ID=$DEPLOYED_PROGRAM_ID
else
    echo -e "${RED}❌ Program keypair not found${NC}"
fi

# Step 11: Save deployment info
echo -e "${YELLOW}💾 Saving remote deployment information...${NC}"
cat > "$PROJECT_ROOT/remote_deployment_info.json" << EOF
{
  "program_id": "$PROGRAM_ID",
  "version": "$NEW_VERSION",
  "previous_version": "$CURRENT_VERSION",
  "rpc_url": "$RPC_URL",
  "wallet_address": "$DEFAULT_WALLET_ADDRESS",
  "deployment_timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "deployment_type": "remote",
  "program_data_address": "$PROGRAM_DATA_ADDRESS",
  "program_size": "$PROGRAM_SIZE",
  "backpack_wallet": "$BACKPACK_WALLET",
  "backpack_wallet_balance": "$FINAL_BALANCE",
  "deploy_action": "$DEPLOY_ACTION",
  "deploy_result": "$DEPLOY_RESULT"
}
EOF

echo -e "${GREEN}✅ Remote deployment information saved to remote_deployment_info.json${NC}"

# Final status
echo ""
echo "======================================================"
echo -e "${GREEN}🎉 REMOTE DEPLOYMENT COMPLETE!${NC}"
echo "======================================================"
echo -e "${BLUE}📊 Your Fixed Ratio Trading contract is deployed to remote:${NC}"
echo ""
echo "  🌐 Remote RPC: $RPC_URL"
echo "  📊 Program ID: $PROGRAM_ID"
echo "  🔢 Version: $NEW_VERSION"
echo "  💳 Default Wallet: $DEFAULT_WALLET_ADDRESS"
echo "  🎒 Backpack Wallet: $BACKPACK_WALLET"
echo "  💰 Backpack Balance: $FINAL_BALANCE SOL"
echo ""
echo -e "${BLUE}📋 Deployment Details:${NC}"
echo "  📈 Action: $DEPLOY_ACTION"
echo "  ✅ Result: $DEPLOY_RESULT"
echo "  📊 Program Data: $PROGRAM_DATA_ADDRESS"
echo "  📏 Program Size: $PROGRAM_SIZE bytes"
echo ""
echo -e "${GREEN}💡 The contract is now live on the remote validator!${NC}"
echo -e "${YELLOW}📝 Next Steps:${NC}"
echo "  1. ✅ Contract is deployed and ready for use"
echo "  2. 🌐 Access via dashboard pointing to $RPC_URL"
echo "  3. 📊 Monitor with: $PROJECT_ROOT/scripts/monitor_pools.sh"
echo ""
echo -e "${BLUE}🔗 Test connection:${NC}"
echo "  curl -X POST -H \"Content-Type: application/json\" \\"
echo "       -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getAccountInfo\",\"params\":[\"$PROGRAM_ID\"]}' \\"
echo "       \"$RPC_URL\""
echo "" 