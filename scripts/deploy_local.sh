#!/bin/bash
# Deploy Fixed Ratio Trading Contract to Local Solana Testnet
# This script builds the contract, starts a local validator, and deploys the program
# The dashboard opens in Firefox private mode to avoid JavaScript caching issues

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

echo "🚀 Fixed Ratio Trading - Local Deployment Script"
echo "================================================="
echo "📂 Project Root: $PROJECT_ROOT"

# Check for required tools
echo -e "${YELLOW}🔧 Checking required tools...${NC}"
MISSING_TOOLS=""
command -v solana >/dev/null 2>&1 || MISSING_TOOLS="$MISSING_TOOLS solana"
command -v solana-keygen >/dev/null 2>&1 || MISSING_TOOLS="$MISSING_TOOLS solana-keygen"
command -v solana-test-validator >/dev/null 2>&1 || MISSING_TOOLS="$MISSING_TOOLS solana-test-validator"
command -v jq >/dev/null 2>&1 || echo "  Warning: jq not found (JSON parsing will be limited)"

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
RPC_URL="http://localhost:8899"
KEYPAIR_PATH="$HOME/.config/solana/id.json"

echo -e "${BLUE}📋 Configuration:${NC}"
echo "  Program ID: $PROGRAM_ID"
echo "  RPC URL: $RPC_URL"
echo "  Keypair: $KEYPAIR_PATH"
echo ""

# Step 1: Auto-increment version number
echo -e "${YELLOW}🔢 Auto-incrementing version number...${NC}"

# Read current version from Cargo.toml
CURRENT_VERSION=$(grep '^version = ' "$PROJECT_ROOT/Cargo.toml" | head -1 | sed 's/version = "\(.*\)"/\1/')
echo "  Current version: $CURRENT_VERSION"

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
echo ""

# Step 2: Build the program
echo -e "${YELLOW}🔨 Building Solana program...${NC}"
cd "$PROJECT_ROOT"
RUSTFLAGS="-C link-arg=-zstack-size=131072" cargo build-sbf || true
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Build successful${NC}"
else
    echo -e "${RED}❌ Build failed${NC}"
    exit 1
fi

# Step 3: Check if validator is running
echo -e "${YELLOW}🔍 Checking for running validator...${NC}"
if pgrep -f "solana-test-validator" > /dev/null; then
    echo -e "${YELLOW}⚠️  Validator already running. Stopping existing validator...${NC}"
    pkill -f "solana-test-validator"
    sleep 3
fi

# Step 4: Start local validator
echo -e "${YELLOW}🏁 Starting local Solana validator...${NC}"
solana-test-validator \
    --rpc-port 8899 \
    --compute-unit-limit 1000000 \
    --reset \
    --quiet &

VALIDATOR_PID=$!
echo "  Validator PID: $VALIDATOR_PID"

# Wait for validator to start
echo -e "${YELLOW}⏳ Waiting for validator to start...${NC}"
sleep 8

# Step 5: Configure Solana CLI
echo -e "${YELLOW}⚙️  Configuring Solana CLI...${NC}"
solana config set --url $RPC_URL
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ CLI configured for local testnet${NC}"
else
    echo -e "${RED}❌ CLI configuration failed${NC}"
    kill $VALIDATOR_PID
    exit 1
fi

# Step 6: Check/create keypair
if [ ! -f "$KEYPAIR_PATH" ]; then
    echo -e "${YELLOW}🔑 Creating new keypair...${NC}"
    solana-keygen new --no-bip39-passphrase --outfile $KEYPAIR_PATH
fi

# Step 7: Airdrop SOL
echo -e "${YELLOW}💰 Airdropping SOL...${NC}"
WALLET_ADDRESS=$(solana-keygen pubkey $KEYPAIR_PATH)
echo "  Wallet: $WALLET_ADDRESS"
solana airdrop 100 $WALLET_ADDRESS
sleep 2

# Skip program airdrop to avoid account conflicts during deployment
# (The program will be funded as needed during deployment)

# Check balances
BALANCE=$(solana balance $WALLET_ADDRESS --output json | jq -r '.value')
echo -e "${GREEN}  Wallet Balance: $BALANCE SOL${NC}"

# Step 8: Deploy the program
echo -e "${YELLOW}🚀 Deploying program...${NC}"

DEPLOY_ACTION=""
DEPLOY_RESULT=""

# Check if account/program already exists - handle both program and system accounts
if [ "$PROGRAM_ID" != "Will be generated during build" ]; then
    echo "  Checking if account $PROGRAM_ID already exists..."
    
    # Check if any account exists at this address
    if solana account $PROGRAM_ID >/dev/null 2>&1; then
        echo "  Account exists! Checking what type..."
        
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
                echo -e "${BLUE}📈 UPGRADING existing program...${NC}"
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
                DEPLOY_ACTION="REDEPLOY"
                echo -e "${YELLOW}🔄 REDEPLOYING program (not upgradeable)...${NC}"
                echo "  Program exists but is not upgradeable."
                echo "  For local testing, closing existing program and redeploying fresh..."
                
                # Close the existing program to free up the account
                solana program close $PROGRAM_ID --recipient $WALLET_ADDRESS 2>/dev/null || true
                sleep 2
                
                # Deploy fresh
                DEPLOY_OUTPUT=$(solana program deploy "$PROJECT_ROOT/target/deploy/fixed_ratio_trading.so" --program-id "$PROGRAM_KEYPAIR" 2>&1)
                DEPLOY_EXIT_CODE=$?
                DEPLOY_RESULT=$([ $DEPLOY_EXIT_CODE -eq 0 ] && echo "REDEPLOYED" || echo "FAILED")
            fi
        else
            echo "  It's a regular account (not a program). Using force deployment..."
            
            DEPLOY_ACTION="CREATE_FORCE"
            echo -e "${YELLOW}🔄 FORCE deploying to existing account...${NC}"
            echo "  Account exists but is not a program."
            echo "  For local testing, using --force to overwrite the account..."
            
            # Deploy with force to overwrite the existing account
            echo "  Deploying program with --force flag..."
            DEPLOY_OUTPUT=$(solana program deploy "$PROJECT_ROOT/target/deploy/fixed_ratio_trading.so" --program-id "$PROGRAM_KEYPAIR" --upgrade-authority "$KEYPAIR_PATH" --force 2>&1)
            DEPLOY_EXIT_CODE=$?
            DEPLOY_RESULT=$([ $DEPLOY_EXIT_CODE -eq 0 ] && echo "CREATED" || echo "FAILED")
        fi
    else
        echo "  No account exists at this address (expected for first deployment)"
        
        DEPLOY_ACTION="CREATE"
        echo -e "${BLUE}🆕 CREATING new program...${NC}"
        echo "  Using initial deployment with upgrade authority..."
        
        DEPLOY_OUTPUT=$(solana program deploy "$PROJECT_ROOT/target/deploy/fixed_ratio_trading.so" --program-id "$PROGRAM_KEYPAIR" --upgrade-authority "$KEYPAIR_PATH" 2>&1)
        DEPLOY_EXIT_CODE=$?
        DEPLOY_RESULT=$([ $DEPLOY_EXIT_CODE -eq 0 ] && echo "CREATED" || echo "FAILED")
    fi
else
    DEPLOY_ACTION="CREATE"
    echo -e "${BLUE}🆕 CREATING new program...${NC}"
    echo "  Using initial deployment with upgrade authority..."
    
    DEPLOY_OUTPUT=$(solana program deploy "$PROJECT_ROOT/target/deploy/fixed_ratio_trading.so" --upgrade-authority "$KEYPAIR_PATH" 2>&1)
    DEPLOY_EXIT_CODE=$?
    DEPLOY_RESULT=$([ $DEPLOY_EXIT_CODE -eq 0 ] && echo "CREATED" || echo "FAILED")
fi

# Display results with clear status
echo ""
echo -e "${BLUE}📋 DEPLOYMENT SUMMARY${NC}"
echo "================================="

case $DEPLOY_RESULT in
    "CREATED")
        if [ "$DEPLOY_ACTION" = "CREATE_FORCE" ]; then
            echo -e "${GREEN}✅ STATUS: Program successfully CREATED (force deployment)${NC}"
            echo -e "${GREEN}   🔄 Previous account overwritten, new program deployed${NC}"
        else
            echo -e "${GREEN}✅ STATUS: Program successfully CREATED${NC}"
            echo -e "${GREEN}   🆕 New program deployed with upgrade authority${NC}"
        fi
        ;;
    "UPGRADED")
        echo -e "${GREEN}✅ STATUS: Program successfully UPGRADED${NC}"
        echo -e "${GREEN}   📈 Contract code updated, program ID preserved${NC}"
        ;;
    "REDEPLOYED")
        echo -e "${GREEN}✅ STATUS: Program successfully REDEPLOYED${NC}"
        echo -e "${GREEN}   🔄 Fresh deployment (previous program closed)${NC}"
        ;;
    "NO_UPGRADE_NEEDED")
        echo -e "${YELLOW}⚡ STATUS: No upgrade needed${NC}"
        echo -e "${YELLOW}   📊 Program bytecode is already up-to-date${NC}"
        ;;
    "FAILED")
        echo -e "${RED}❌ STATUS: Deployment FAILED${NC}"
        echo -e "${RED}   💥 See error details below${NC}"
        echo ""
        echo "Error output:"
        echo "$DEPLOY_OUTPUT"
        kill $VALIDATOR_PID
        exit 1
        ;;
esac

echo "   Action: $DEPLOY_ACTION"
echo "   Program ID: $PROGRAM_ID"
echo ""

if [ "$DEPLOY_RESULT" != "FAILED" ]; then
    echo -e "${GREEN}✅ Program deployment completed successfully!${NC}"
else
    echo -e "${RED}❌ Deployment failed${NC}"
    kill $VALIDATOR_PID
    exit 1
fi

# Step 9: Get the actual deployed program ID and verify
echo -e "${YELLOW}🔍 Getting deployed program ID...${NC}"
if [ -f "$PROGRAM_KEYPAIR" ]; then
    DEPLOYED_PROGRAM_ID=$(solana-keygen pubkey "$PROGRAM_KEYPAIR")
    echo -e "${GREEN}✅ Program ID: $DEPLOYED_PROGRAM_ID${NC}"
    
    # Verify deployment
    PROGRAM_INFO=$(solana program show $DEPLOYED_PROGRAM_ID --output json 2>/dev/null)
    if [ $? -eq 0 ]; then
        PROGRAM_DATA_ADDRESS=$(echo $PROGRAM_INFO | jq -r '.programdataAddress // "N/A"')
        PROGRAM_SIZE=$(echo $PROGRAM_INFO | jq -r '.dataLen // "N/A"')
        echo -e "${GREEN}✅ Program verification successful${NC}"
        echo "  Program Data Address: $PROGRAM_DATA_ADDRESS"
        echo "  Program Size: $PROGRAM_SIZE bytes"
    else
        echo -e "${YELLOW}⚠️  Program deployed but verification data not immediately available${NC}"
    fi
    PROGRAM_ID=$DEPLOYED_PROGRAM_ID
else
    echo -e "${RED}❌ Program keypair not found${NC}"
fi

# Step 10: Save deployment info
echo -e "${YELLOW}💾 Saving deployment information...${NC}"
cat > "$PROJECT_ROOT/deployment_info.json" << EOF
{
  "program_id": "$PROGRAM_ID",
  "version": "$NEW_VERSION",
  "previous_version": "$CURRENT_VERSION",
  "rpc_url": "$RPC_URL",
  "wallet_address": "$WALLET_ADDRESS",
  "deployment_timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "validator_pid": $VALIDATOR_PID,
  "program_data_address": "$PROGRAM_DATA_ADDRESS",
  "program_size": $PROGRAM_SIZE
}
EOF

echo -e "${GREEN}✅ Deployment information saved to deployment_info.json${NC}"

# Step 11: Start Dashboard Server
echo ""
echo -e "${YELLOW}🌐 Starting dashboard server...${NC}"

# Check if Python 3 is available
if command -v python3 &> /dev/null; then
    PYTHON_CMD="python3"
elif command -v python &> /dev/null; then
    PYTHON_CMD="python"
else
    echo -e "${RED}❌ Python not found. Dashboard will not start automatically.${NC}"
    echo "   Install Python to enable automatic dashboard startup"
    PYTHON_CMD=""
fi

# Start dashboard server if Python is available
if [ -n "$PYTHON_CMD" ]; then
    # Check if port 3000 is already in use
    if lsof -i :3000 > /dev/null 2>&1; then
        echo -e "${YELLOW}⚠️  Port 3000 already in use. Stopping existing server...${NC}"
        pkill -f "python.*http.server.*3000" || true
        sleep 2
    fi
    
    echo "  Starting web server on http://localhost:3000..."
    cd "$PROJECT_ROOT/dashboard"
    $PYTHON_CMD -m http.server 3000 > /dev/null 2>&1 &
    DASHBOARD_PID=$!
    
    # Wait a moment for server to start
    sleep 3
    
    # Verify dashboard server started
    if kill -0 $DASHBOARD_PID 2>/dev/null; then
        echo -e "${GREEN}✅ Dashboard server started (PID: $DASHBOARD_PID)${NC}"
        
        # Step 12: Open Firefox in private mode automatically
        echo ""
        echo -e "${YELLOW}🦊 Opening Firefox in private mode to dashboard...${NC}"
        
        # Open Firefox in private mode (cross-platform)
        if command -v open > /dev/null 2>&1; then
            # macOS - try private mode first, fallback to regular
            echo "  Attempting to open Firefox in private mode..."
            open -a Firefox --args --private-window http://localhost:3000 2>/dev/null || \
            open -a Firefox http://localhost:3000 2>/dev/null || \
            open http://localhost:3000 2>/dev/null || \
            echo -e "${YELLOW}⚠️  Could not open Firefox automatically. Please open http://localhost:3000 manually in private mode${NC}"
        elif command -v firefox > /dev/null 2>&1; then
            # Linux/Windows with firefox command
            echo "  Attempting to open Firefox in private mode..."
            firefox --private-window http://localhost:3000 2>/dev/null &
        else
            echo -e "${YELLOW}⚠️  Auto-open not available. Please open http://localhost:3000 manually in private mode${NC}"
        fi
        
        echo -e "${GREEN}✅ Firefox should now open in private mode to avoid caching issues${NC}"
        
    else
        echo -e "${RED}❌ Dashboard server failed to start${NC}"
        DASHBOARD_PID=""
    fi
else
    DASHBOARD_PID=""
fi

cd "$PROJECT_ROOT"

# Final status
echo ""
echo "======================================================"
echo -e "${GREEN}🎉 COMPLETE DEPLOYMENT & DASHBOARD STARTUP!${NC}"
echo "======================================================"
echo -e "${BLUE}📊 Your Fixed Ratio Trading environment is fully running:${NC}"
echo ""
echo "  🌐 Web Dashboard: http://localhost:3000"
if [ -n "$DASHBOARD_PID" ]; then
    echo "  📱 Browser: Firefox should be opening in private mode (no cache issues)"
    echo "  🟢 Dashboard Status: Running (PID: $DASHBOARD_PID)"
else
    echo "  🟡 Dashboard Status: Not started (Python not available)"
fi
echo "  🔗 RPC Endpoint: $RPC_URL"
echo "  📡 Validator Status: Running (PID: $VALIDATOR_PID)"
echo ""
echo -e "${BLUE}📋 Contract Information:${NC}"
echo "  📊 Program ID: $PROGRAM_ID"
echo "  🔢 Version: $NEW_VERSION (auto-incremented from $CURRENT_VERSION)"
echo "  💳 Wallet: $WALLET_ADDRESS"
echo ""
echo -e "${YELLOW}📝 Next Steps:${NC}"
echo "  1. ✅ Dashboard is running - interact with your contract via web UI"
echo "     💡 Private mode ensures fresh JavaScript (no browser cache issues)"
echo "  2. 🏊 Create test pools: $PROJECT_ROOT/scripts/create_sample_pools.sh"
echo "  3. 📊 Monitor pools: $PROJECT_ROOT/scripts/monitor_pools.sh"
echo ""
echo -e "${GREEN}💡 The dashboard will automatically show: Fixed Ratio Trading Dashboard v$NEW_VERSION${NC}"
echo ""
echo -e "${YELLOW}🛑 To stop everything:${NC}"
if [ -n "$DASHBOARD_PID" ]; then
    echo "  kill $VALIDATOR_PID $DASHBOARD_PID"
    echo "  or: pkill -f \"solana-test-validator\" && pkill -f \"python.*http.server.*3000\""
else
    echo "  kill $VALIDATOR_PID"
    echo "  or: pkill -f \"solana-test-validator\""
fi
echo ""

# Keep the script running so both services stay up
echo -e "${BLUE}🔄 Services running in background:${NC}"
echo "   📡 Validator (PID: $VALIDATOR_PID)"
if [ -n "$DASHBOARD_PID" ]; then
    echo "   🌐 Dashboard (PID: $DASHBOARD_PID)"
fi
echo -e "${BLUE}   Press Ctrl+C to stop all services and exit${NC}"

# Trap Ctrl+C to clean up both services
if [ -n "$DASHBOARD_PID" ]; then
    trap "echo -e '\\n${YELLOW}🛑 Stopping all services...${NC}'; kill $VALIDATOR_PID $DASHBOARD_PID 2>/dev/null; exit 0" INT
else
    trap "echo -e '\\n${YELLOW}🛑 Stopping validator...${NC}'; kill $VALIDATOR_PID 2>/dev/null; exit 0" INT
fi

# Wait for user to stop and monitor both services
while true; do
    sleep 10
    # Check if validator is still running
    if ! kill -0 $VALIDATOR_PID 2>/dev/null; then
        echo -e "${RED}❌ Validator stopped unexpectedly${NC}"
        if [ -n "$DASHBOARD_PID" ]; then
            kill $DASHBOARD_PID 2>/dev/null
        fi
        exit 1
    fi
    
    # Check if dashboard is still running (if it was started)
    if [ -n "$DASHBOARD_PID" ] && ! kill -0 $DASHBOARD_PID 2>/dev/null; then
        echo -e "${YELLOW}⚠️  Dashboard server stopped unexpectedly${NC}"
        DASHBOARD_PID=""
    fi
done 