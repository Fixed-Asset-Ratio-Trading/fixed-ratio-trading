#!/bin/bash
# Deploy Fixed Ratio Trading Contract to Remote Solana Validator
# This script builds the contract and deploys/upgrades the program to the remote validator
# Targets the direct validator endpoint at http://192.168.2.88:8899
#
# Usage:
#   ./remote_build_and_deploy.sh [--reset|--noreset]
#
# Options:
#   --reset     Reset the validator before deployment
#   --noreset   Keep existing validator state (default behavior)
#   (no option) Keep existing validator state (default behavior)

set -e

# Parse command line arguments
VALIDATOR_RESET_OPTION="no_reset"  # Default to no reset (changed from auto_reset)
for arg in "$@"; do
    case $arg in
        --reset)
            VALIDATOR_RESET_OPTION="auto_reset"
            ;;
        --noreset)
            VALIDATOR_RESET_OPTION="no_reset"
            ;;
        *)
            echo "Unknown option: $arg"
            echo "Usage: $0 [--reset|--noreset]"
            exit 1
            ;;
    esac
done

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
echo -e "${BLUE}🌐 Targeting Direct Validator Endpoint: http://192.168.2.88:8899${NC}"
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

# Load shared configuration if available
SHARED_CONFIG="$PROJECT_ROOT/shared-config.json"
if [ -f "$SHARED_CONFIG" ] && command -v jq >/dev/null 2>&1; then
    echo -e "${BLUE}📋 Loading shared configuration...${NC}"
    RPC_URL=$(jq -r '.solana.rpcUrl' "$SHARED_CONFIG" 2>/dev/null || echo "http://192.168.2.88:8899")
    BACKPACK_WALLET=$(jq -r '.wallets.expectedBackpackWallet' "$SHARED_CONFIG" 2>/dev/null || echo "5GGZiMwU56rYL1L52q7Jz7ELkSN4iYyQqdv418hxPh6t")
    echo -e "${GREEN}✅ Configuration loaded from shared-config.json${NC}"
else
    echo -e "${YELLOW}⚠️ Using fallback configuration (shared-config.json not found or jq not available)${NC}"
    RPC_URL="http://192.168.2.88:8899"
    BACKPACK_WALLET="5GGZiMwU56rYL1L52q7Jz7ELkSN4iYyQqdv418hxPh6t"
fi

# Configuration - Get program ID from the generated keypair
PROGRAM_KEYPAIR="$PROJECT_ROOT/target/deploy/fixed_ratio_trading-keypair.json"
if [ -f "$PROGRAM_KEYPAIR" ]; then
    PROGRAM_ID=$(solana-keygen pubkey "$PROGRAM_KEYPAIR")
else
    PROGRAM_ID="Will be generated during build"
fi
KEYPAIR_PATH="$HOME/.config/solana/id.json"

echo -e "${BLUE}📋 Configuration:${NC}"
echo "  Program ID: $PROGRAM_ID"
echo "  Remote RPC URL: $RPC_URL"
echo "  Keypair: $KEYPAIR_PATH"
echo "  Backpack Wallet: $BACKPACK_WALLET"
echo ""

# Step 1: Run all tests before deployment
echo -e "${YELLOW}🧪 Running comprehensive test suite...${NC}"
echo "   This ensures code quality before deployment"
cd "$PROJECT_ROOT"

echo "   Running cargo tests..."
if ! cargo test --lib; then
    echo -e "${RED}❌ Unit tests failed! Deployment aborted.${NC}"
    echo "   Please fix failing tests before deploying"
    exit 1
fi

echo "   Running integration tests..."
if ! cargo test --test '*'; then
    echo -e "${RED}❌ Integration tests failed! Deployment aborted.${NC}"
    echo "   Please fix failing tests before deploying"
    exit 1
fi

echo -e "${GREEN}✅ All tests passed successfully${NC}"
echo ""

# Step 2: Determine validator reset action
VALIDATOR_RESET=false

if [ "$VALIDATOR_RESET_OPTION" = "auto_reset" ]; then
    VALIDATOR_RESET=true
    echo -e "${YELLOW}🔄 Resetting validator (--reset specified)${NC}"
else
    # Default no reset
    VALIDATOR_RESET=false
    echo -e "${BLUE}🔄 Keeping existing validator state (default behavior)${NC}"
fi

if [ "$VALIDATOR_RESET" = true ]; then
    echo -e "${YELLOW}🔄 Resetting remote validator...${NC}"
    
    # Check if SSH is available
    if ! command -v ssh >/dev/null 2>&1; then
        echo -e "${RED}❌ SSH not found. Cannot reset remote validator.${NC}"
        exit 1
    fi
    
    echo "   Connecting to dev@vmdevbox1..."
    echo "   Starting fresh validator (script will handle stopping previous one)..."
    
    # Start fresh validator and show output
    echo "   Running: cd ~/code/fixed-ratio-trading && ./scripts/start_production_validator.sh --reset"
    if ssh dev@vmdevbox1 'cd ~/code/fixed-ratio-trading && ./scripts/start_production_validator.sh --reset'; then
        echo -e "${GREEN}✅ Validator start script completed${NC}"
        
        # Verify validator is actually running by testing connectivity
        echo "   Verifying validator is responding..."
        VALIDATOR_CHECK_COUNT=0
        MAX_VALIDATOR_CHECKS=10
        
        while [ $VALIDATOR_CHECK_COUNT -lt $MAX_VALIDATOR_CHECKS ]; do
            if curl -s --connect-timeout 5 -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' "$RPC_URL" | grep -q "ok"; then
                echo -e "${GREEN}✅ Validator is running and responding${NC}"
                
                # Get some basic validator info to confirm it's working
                echo "   Getting validator status..."
                SLOT_INFO=$(curl -s -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getSlot"}' "$RPC_URL" 2>/dev/null)
                if echo "$SLOT_INFO" | grep -q '"result"'; then
                    CURRENT_SLOT=$(echo "$SLOT_INFO" | grep -o '"result":[0-9]*' | cut -d':' -f2)
                    echo -e "${GREEN}   Current slot: $CURRENT_SLOT${NC}"
                fi
                
                # Check if we can get account balance (basic functionality test)
                BALANCE_CHECK=$(solana balance $BACKPACK_WALLET 2>/dev/null | head -1)
                if [ $? -eq 0 ]; then
                    echo -e "${GREEN}   Balance check successful: $BALANCE_CHECK${NC}"
                else
                    echo -e "${YELLOW}   Balance check failed, but validator is responding${NC}"
                fi
                break
            else
                VALIDATOR_CHECK_COUNT=$((VALIDATOR_CHECK_COUNT + 1))
                if [ $VALIDATOR_CHECK_COUNT -lt $MAX_VALIDATOR_CHECKS ]; then
                    echo "   Validator check $VALIDATOR_CHECK_COUNT/$MAX_VALIDATOR_CHECKS - waiting..."
                    sleep 2
                else
                    echo -e "${RED}❌ Validator not responding after $MAX_VALIDATOR_CHECKS attempts${NC}"
                    echo "   The start script completed but validator may not be ready yet"
                    echo "   You may need to wait a bit longer or check vmdevbox1 manually"
                    exit 1
                fi
            fi
        done
    else
        echo -e "${RED}❌ Failed to start fresh validator${NC}"
        echo "   You may need to manually start the validator on vmdevbox1"
        exit 1
    fi
    
    echo -e "${GREEN}✅ Remote validator reset completed${NC}"
else
    echo -e "${BLUE}ℹ️  Keeping existing validator state${NC}"
fi

echo ""

# Step 3: Test remote endpoint connectivity (skip if we just reset validator)
if [[ $VALIDATOR_RESET == false ]]; then
    echo -e "${YELLOW}🔍 Testing remote endpoint connectivity...${NC}"
    if command -v curl >/dev/null 2>&1; then
        # Test endpoint with retry logic
        RETRY_COUNT=0
        MAX_RETRIES=5
        
        while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
            if curl -s --connect-timeout 10 -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' "$RPC_URL" | grep -q "ok"; then
                echo -e "${GREEN}✅ Remote endpoint is responding correctly${NC}"
                break
            else
                RETRY_COUNT=$((RETRY_COUNT + 1))
                if [ $RETRY_COUNT -lt $MAX_RETRIES ]; then
                    echo "   Retry $RETRY_COUNT/$MAX_RETRIES - waiting for validator..."
                    sleep 3
                else
                    echo -e "${RED}❌ Remote endpoint is not responding after $MAX_RETRIES attempts${NC}"
                    echo "   Please ensure the remote validator is running at $RPC_URL"
                    exit 1
                fi
            fi
        done
    else
        echo -e "${YELLOW}⚠️  curl not found. Cannot test endpoint automatically${NC}"
    fi
else
    echo -e "${BLUE}ℹ️  Skipping connectivity test (validator was just reset and verified)${NC}"
fi

# Step 3.5: Ensure Metaplex programs are deployed for local testing
echo -e "${YELLOW}🎨 Checking Metaplex programs deployment...${NC}"
echo "   Verifying Token Metadata Program for full token functionality"

METAPLEX_SCRIPT="$PROJECT_ROOT/scripts/metaplex/manage_metaplex.sh"
if [ -f "$METAPLEX_SCRIPT" ]; then
    # Check if Metaplex programs are already deployed
    if ! "$METAPLEX_SCRIPT" status >/dev/null 2>&1; then
        echo "   Metaplex programs not found, deploying..."
        if "$METAPLEX_SCRIPT" start; then
            echo -e "${GREEN}✅ Metaplex programs deployed successfully${NC}"
        else
            echo -e "${YELLOW}⚠️  Metaplex deployment failed, proceeding without full metadata support${NC}"
            echo "   Note: Token creation may not include metadata on this deployment"
        fi
    else
        echo -e "${GREEN}✅ Metaplex programs already deployed${NC}"
    fi
else
    echo -e "${YELLOW}⚠️  Metaplex management script not found, skipping metadata setup${NC}"
fi

echo ""

# Step 4: Check if build creates new changes
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

# Step 5: Initial build to check for changes
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

# Step 6: Determine if version should be incremented
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
    
    # Step 7: Rebuild with new version
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

# Step 8: Configure Solana CLI for remote endpoint
echo -e "${YELLOW}⚙️  Configuring Solana CLI for remote endpoint...${NC}"
solana config set --url $RPC_URL
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ CLI configured for remote validator${NC}"
else
    echo -e "${RED}❌ CLI configuration failed${NC}"
    exit 1
fi

# Step 9: Check/create keypair
if [ ! -f "$KEYPAIR_PATH" ]; then
    echo -e "${YELLOW}🔑 Creating new keypair...${NC}"
    solana-keygen new --no-bip39-passphrase --outfile $KEYPAIR_PATH
fi

# Step 10: Check Backpack wallet balance
echo -e "${YELLOW}💰 Checking Backpack wallet balance...${NC}"
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

# Step 11: Check if program exists on remote and compare versions
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

# Step 12: Get the actual deployed program ID and verify
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

# Step 12.5: Initialize system with program authority (for fresh deployments)
if [ "$DEPLOY_ACTION" = "CREATE" ]; then
    echo ""
    echo -e "${YELLOW}🔧 Initializing Fixed Ratio Trading system...${NC}"

    if command -v node &> /dev/null; then
        # Check if @solana/web3.js is available
        if [ -d "$PROJECT_ROOT/node_modules/@solana/web3.js" ]; then
            echo "  Using existing @solana/web3.js installation..."
            cd "$PROJECT_ROOT"
            
            # Use the consolidated initialization script
            node scripts/initialize_system.js "$PROGRAM_ID" "$RPC_URL" "$KEYPAIR_PATH"
            INIT_EXIT_CODE=$?
            
            if [ $INIT_EXIT_CODE -eq 0 ]; then
                echo -e "${GREEN}✅ System initialization completed successfully${NC}"
                INITIALIZATION_STATUS="success"
            else
                echo -e "${YELLOW}⚠️  System initialization failed, but deployment was successful${NC}"
                echo "   Try running manually: node scripts/initialize_system.js $PROGRAM_ID $RPC_URL"
                INITIALIZATION_STATUS="failed"
            fi
        else
            echo -e "${YELLOW}⚠️  @solana/web3.js not found, skipping automatic system initialization${NC}"
            echo "   Run 'npm install @solana/web3.js' and then:"
            echo "   node scripts/initialize_system.js $PROGRAM_ID $RPC_URL"
            INITIALIZATION_STATUS="skipped"
        fi
    else
        echo -e "${YELLOW}⚠️  Node.js not found, skipping automatic system initialization${NC}"
        echo "   Install Node.js and run: node scripts/initialize_system.js $PROGRAM_ID $RPC_URL"
        INITIALIZATION_STATUS="skipped"
    fi
else
    echo -e "${BLUE}ℹ️ Skipping initialization (upgrade deployment)${NC}"
    INITIALIZATION_STATUS="skipped"
fi

# Step 13: Save deployment info
echo -e "${YELLOW}💾 Saving deployment information...${NC}"
cat > "$PROJECT_ROOT/deployment_info.json" << EOF
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
  "deploy_result": "$DEPLOY_RESULT",
  "initialization_status": "$INITIALIZATION_STATUS",
  "initialization_transaction": "$INITIALIZATION_TX"
}
EOF

echo -e "${GREEN}✅ Deployment information saved to deployment_info.json${NC}"

# Generate program state data for dashboard
echo ""
echo "======================================================"
echo -e "${BLUE}📊 GENERATING PROGRAM STATE DATA FOR DASHBOARD${NC}"
echo "======================================================"

# Check if Node.js is available
if command -v node >/dev/null 2>&1; then
    echo -e "${BLUE}🔍 Querying program state data...${NC}"
    
    # Set environment variables for the query script
    export SOLANA_RPC_URL="$RPC_URL"
    export PROGRAM_ID="$PROGRAM_ID"
    export SOLANA_ENVIRONMENT="remote"
    
    # Run the state query script
    if node "$PROJECT_ROOT/scripts/query_program_state.js"; then
        echo -e "${GREEN}✅ Program state data generated successfully${NC}"
        echo -e "${BLUE}📁 State file: $PROJECT_ROOT/dashboard/state.json${NC}"
    else
        echo -e "${YELLOW}⚠️ Warning: Failed to generate program state data${NC}"
        echo -e "${YELLOW}   Dashboard will start with empty state${NC}"
    fi
else
    echo -e "${YELLOW}⚠️ Warning: Node.js not available, skipping state data generation${NC}"
    echo -e "${YELLOW}   Install Node.js to enable automatic state data generation${NC}"
fi

# Final status
echo ""
echo "======================================================"
echo -e "${GREEN}🎉 DIRECT ENDPOINT DEPLOYMENT COMPLETE!${NC}"
echo "======================================================"
echo -e "${BLUE}📊 Your Fixed Ratio Trading contract is deployed:${NC}"
echo ""
echo "  🌐 Direct RPC: $RPC_URL"
echo "  📊 Program ID: $PROGRAM_ID"
echo "  🔢 Version: $NEW_VERSION"
echo "  💳 Default Wallet: $DEFAULT_WALLET_ADDRESS"
echo "  🎒 Backpack Wallet: $BACKPACK_WALLET"
echo "  💰 Backpack Balance: $FINAL_BALANCE SOL"
echo ""
echo -e "${BLUE}📋 Deployment Details:${NC}"
echo "  📈 Action: $DEPLOY_ACTION"
echo "  ✅ Result: $DEPLOY_RESULT"
echo "  🏗️ Initialization: $INITIALIZATION_STATUS"
if [ "$INITIALIZATION_STATUS" = "success" ] && [ -n "$INITIALIZATION_TX" ]; then
    echo "  🔗 Init Transaction: $INITIALIZATION_TX"
fi
echo "  📊 Program Data: $PROGRAM_DATA_ADDRESS"
echo "  📏 Program Size: $PROGRAM_SIZE bytes"
echo ""
echo -e "${GREEN}💡 The contract is now live on the direct validator endpoint!${NC}"
echo -e "${YELLOW}📝 Next Steps:${NC}"
if [ "$INITIALIZATION_STATUS" = "success" ]; then
    echo "  1. ✅ Contract is deployed and initialized - ready for pools!"
    echo "  2. 🌐 Access via dashboard pointing to $RPC_URL"
    echo "  3. 🏊‍♂️ Create pools via dashboard (no manual initialization needed)"
    echo "  4. 📊 Monitor with: $PROJECT_ROOT/scripts/monitor_pools.sh"
elif [ "$INITIALIZATION_STATUS" = "failed" ]; then
    echo "  1. ✅ Contract is deployed but initialization failed"
    echo "  2. 🏗️ Initialize manually via dashboard before creating pools"
    echo "  3. 🌐 Access via dashboard pointing to $RPC_URL"
    echo "  4. 📊 Monitor with: $PROJECT_ROOT/scripts/monitor_pools.sh"
else
    echo "  1. ✅ Contract is upgraded and ready for use"
    echo "  2. 🌐 Access via dashboard pointing to $RPC_URL"
    echo "  3. 📊 Monitor with: $PROJECT_ROOT/scripts/monitor_pools.sh"
fi
echo ""
echo -e "${BLUE}🔗 Test connection:${NC}"
echo "  curl -X POST -H \"Content-Type: application/json\" \\"
echo "       -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getAccountInfo\",\"params\":[\"$PROGRAM_ID\"]}' \\"
echo "       \"$RPC_URL\""
echo "" 