#!/bin/bash
# Deploy Fixed Ratio Trading Contract to Local Solana Testnet
# This script builds the contract, starts a local validator, and deploys the program

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

echo "🚀 Fixed Ratio Trading - Local Deployment Script"
echo "================================================="
echo "📂 Project Root: $PROJECT_ROOT"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

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

# Step 1: Build the program
echo -e "${YELLOW}🔨 Building Solana program...${NC}"
cd "$PROJECT_ROOT"
RUSTFLAGS="-C link-arg=-zstack-size=131072" cargo build-sbf || true
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Build successful${NC}"
else
    echo -e "${RED}❌ Build failed${NC}"
    exit 1
fi

# Step 2: Check if validator is running
echo -e "${YELLOW}🔍 Checking for running validator...${NC}"
if pgrep -f "solana-test-validator" > /dev/null; then
    echo -e "${YELLOW}⚠️  Validator already running. Stopping existing validator...${NC}"
    pkill -f "solana-test-validator"
    sleep 3
fi

# Step 3: Start local validator
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

# Step 4: Configure Solana CLI
echo -e "${YELLOW}⚙️  Configuring Solana CLI...${NC}"
solana config set --url $RPC_URL
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ CLI configured for local testnet${NC}"
else
    echo -e "${RED}❌ CLI configuration failed${NC}"
    kill $VALIDATOR_PID
    exit 1
fi

# Step 5: Check/create keypair
if [ ! -f "$KEYPAIR_PATH" ]; then
    echo -e "${YELLOW}🔑 Creating new keypair...${NC}"
    solana-keygen new --no-bip39-passphrase --outfile $KEYPAIR_PATH
fi

# Step 6: Airdrop SOL
echo -e "${YELLOW}💰 Airdropping SOL...${NC}"
WALLET_ADDRESS=$(solana-keygen pubkey $KEYPAIR_PATH)
echo "  Wallet: $WALLET_ADDRESS"
solana airdrop 100 $WALLET_ADDRESS
sleep 2

# Skip program airdrop during initial deployment (program ID not yet known)
if [ "$PROGRAM_ID" != "Will be generated during build" ]; then
    echo "  Program ID: $PROGRAM_ID"
    solana airdrop 10 $PROGRAM_ID
    sleep 2
fi

# Check balances
BALANCE=$(solana balance $WALLET_ADDRESS --output json | jq -r '.value')
echo -e "${GREEN}  Wallet Balance: $BALANCE SOL${NC}"
if [ "$PROGRAM_ID" != "Will be generated during build" ]; then
    PROGRAM_BALANCE=$(solana balance $PROGRAM_ID --output json | jq -r '.value')
    echo -e "${GREEN}  Program Balance: $PROGRAM_BALANCE SOL${NC}"
fi

# Step 7: Deploy the program
echo -e "${YELLOW}🚀 Deploying program...${NC}"
solana program deploy "$PROJECT_ROOT/target/deploy/fixed_ratio_trading.so"
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Program deployed successfully!${NC}"
    echo -e "${GREEN}   Program ID: $PROGRAM_ID${NC}"
else
    echo -e "${RED}❌ Deployment failed${NC}"
    kill $VALIDATOR_PID
    exit 1
fi

# Step 8: Get the actual deployed program ID and verify
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

# Step 9: Save deployment info
echo -e "${YELLOW}💾 Saving deployment information...${NC}"
cat > "$PROJECT_ROOT/deployment_info.json" << EOF
{
  "program_id": "$PROGRAM_ID",
  "rpc_url": "$RPC_URL",
  "wallet_address": "$WALLET_ADDRESS",
  "deployment_timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "validator_pid": $VALIDATOR_PID,
  "program_data_address": "$PROGRAM_DATA_ADDRESS",
  "program_size": $PROGRAM_SIZE
}
EOF

echo -e "${GREEN}✅ Deployment information saved to deployment_info.json${NC}"

# Final status
echo ""
echo -e "${GREEN}🎉 DEPLOYMENT COMPLETE!${NC}"
echo -e "${GREEN}================================${NC}"
echo -e "${BLUE}📊 Access your deployment:${NC}"
echo "  🌐 Web Dashboard: http://localhost:3000"
echo "  🔗 RPC Endpoint: $RPC_URL"
echo "  📋 Program ID: $PROGRAM_ID"
echo "  💳 Wallet: $WALLET_ADDRESS"
echo ""
echo -e "${YELLOW}📝 Next Steps:${NC}"
echo "  1. Open web dashboard: $PROJECT_ROOT/scripts/start_dashboard.sh"
echo "  2. Create test pools: $PROJECT_ROOT/scripts/create_sample_pools.sh"
echo "  3. Monitor pools: $PROJECT_ROOT/scripts/monitor_pools.sh"
echo ""
echo -e "${YELLOW}🛑 To stop validator:${NC}"
echo "  kill $VALIDATOR_PID"
echo ""

# Keep the script running so validator stays up
echo -e "${BLUE}🔄 Validator running in background (PID: $VALIDATOR_PID)${NC}"
echo -e "${BLUE}   Press Ctrl+C to stop validator and exit${NC}"

# Trap Ctrl+C to clean up
trap "echo -e '\\n${YELLOW}🛑 Stopping validator...${NC}'; kill $VALIDATOR_PID; exit 0" INT

# Wait for user to stop
while true; do
    sleep 10
    # Check if validator is still running
    if ! kill -0 $VALIDATOR_PID 2>/dev/null; then
        echo -e "${RED}❌ Validator stopped unexpectedly${NC}"
        exit 1
    fi
done 