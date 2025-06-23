#!/bin/bash
# Deploy Fixed Ratio Trading Contract using Docker for Apple Silicon compatibility
# This script builds the contract using Docker, then deploys locally

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

echo "🚀 Fixed Ratio Trading - Docker-based Local Deployment (Apple Silicon)"
echo "=================================================================="
echo "📂 Project Root: $PROJECT_ROOT"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Configuration
PROGRAM_ID="quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD"
RPC_URL="http://localhost:8899"
KEYPAIR_PATH="$HOME/.config/solana/id.json"
DOCKER_IMAGE="solana-m1-dev"

echo -e "${BLUE}📋 Configuration:${NC}"
echo "  Program ID: $PROGRAM_ID"
echo "  RPC URL: $RPC_URL"
echo "  Keypair: $KEYPAIR_PATH"
echo "  Docker Image: $DOCKER_IMAGE"
echo ""

# Step 1: Build Docker image if it doesn't exist
echo -e "${YELLOW}🐳 Checking Docker image...${NC}"
if ! docker image inspect $DOCKER_IMAGE >/dev/null 2>&1; then
    echo -e "${YELLOW}🔨 Building Docker image for Solana development (this may take 10-15 minutes first time)...${NC}"
    cd "$PROJECT_ROOT"
    docker build -f Dockerfile.solana -t $DOCKER_IMAGE .
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ Docker image built successfully${NC}"
    else
        echo -e "${RED}❌ Docker image build failed${NC}"
        exit 1
    fi
else
    echo -e "${GREEN}✅ Docker image already exists${NC}"
fi

# Step 2: Build the program using Docker
echo -e "${YELLOW}🔨 Building Solana program in Docker...${NC}"
docker run --rm \
    -v "$PROJECT_ROOT:/workspace" \
    -w /workspace \
    $DOCKER_IMAGE \
    bash -c "cargo build-bpf --manifest-path Cargo.toml --bpf-out-dir target/deploy"

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

# Step 4: Start local validator with Apple Silicon compatibility
echo -e "${YELLOW}🏁 Starting local Solana validator (Apple Silicon compatible)...${NC}"
solana-test-validator \
    --rpc-port 8899 \
    --rpc-pubsub-enable \
    --compute-unit-limit 1000000 \
    --no-bpf-jit \
    --reset \
    --quiet &

VALIDATOR_PID=$!
echo "  Validator PID: $VALIDATOR_PID"

# Wait for validator to start
echo -e "${YELLOW}⏳ Waiting for validator to start...${NC}"
sleep 10

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

# Check balance
BALANCE=$(solana balance $WALLET_ADDRESS --output json | jq -r '.value')
echo -e "${GREEN}  Balance: $BALANCE SOL${NC}"

# Step 8: Deploy the program
echo -e "${YELLOW}🚀 Deploying program...${NC}"
solana program deploy "$PROJECT_ROOT/target/deploy/fixed_ratio_trading.so" --program-id $PROGRAM_ID
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Program deployed successfully!${NC}"
    echo -e "${GREEN}   Program ID: $PROGRAM_ID${NC}"
else
    echo -e "${RED}❌ Deployment failed${NC}"
    kill $VALIDATOR_PID
    exit 1
fi

# Step 9: Verify deployment
echo -e "${YELLOW}🔍 Verifying deployment...${NC}"
PROGRAM_INFO=$(solana program show $PROGRAM_ID --output json)
if [ $? -eq 0 ]; then
    PROGRAM_DATA_ADDRESS=$(echo $PROGRAM_INFO | jq -r '.programdataAddress')
    PROGRAM_SIZE=$(echo $PROGRAM_INFO | jq -r '.dataLen')
    echo -e "${GREEN}✅ Program verification successful${NC}"
    echo "  Program Data Address: $PROGRAM_DATA_ADDRESS"
    echo "  Program Size: $PROGRAM_SIZE bytes"
else
    echo -e "${RED}❌ Program verification failed${NC}"
fi

# Step 10: Save deployment info
echo -e "${YELLOW}💾 Saving deployment information...${NC}"
cat > "$PROJECT_ROOT/deployment_info.json" << EOF
{
  "program_id": "$PROGRAM_ID",
  "rpc_url": "$RPC_URL",
  "wallet_address": "$WALLET_ADDRESS",
  "deployment_timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "validator_pid": $VALIDATOR_PID,
  "program_data_address": "$PROGRAM_DATA_ADDRESS",
  "program_size": $PROGRAM_SIZE,
  "deployment_method": "docker_apple_silicon"
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