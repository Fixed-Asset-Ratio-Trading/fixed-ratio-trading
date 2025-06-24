#!/bin/bash
# Remote Build and Deploy Script
# Syncs code to remote VM, builds there, and deploys locally

# Find the project root directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Verify we found the correct project directory
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    echo "❌ Error: Could not find Cargo.toml in project root: $PROJECT_ROOT"
    exit 1
fi

echo "🌐 Fixed Ratio Trading - Remote Build Deployment"
echo "==============================================="
echo "📂 Project Root: $PROJECT_ROOT"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Configuration - EDIT THESE VALUES
REMOTE_HOST="dev@vmdevbox1"  # Change to your VM details
REMOTE_SSH_KEY="~/.ssh/solana_build_vm"
REMOTE_PROJECT_PATH="/home/dev/code/fixed-ratio-trading"
PROGRAM_ID="quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD"
RPC_URL="http://localhost:8899"
KEYPAIR_PATH="$HOME/.config/solana/id.json"

echo -e "${BLUE}📋 Configuration:${NC}"
echo "  Remote Host: $REMOTE_HOST"
echo "  Remote Path: $REMOTE_PROJECT_PATH"
echo "  Program ID: $PROGRAM_ID"
echo "  RPC URL: $RPC_URL"
echo ""

# Check if SSH key exists
if [ ! -f "${REMOTE_SSH_KEY/#\~/$HOME}" ]; then
    echo -e "${RED}❌ SSH key not found: $REMOTE_SSH_KEY${NC}"
    echo "   Create one with: ssh-keygen -t ed25519 -f $REMOTE_SSH_KEY"
    exit 1
fi

# Step 1: Test SSH connection
echo -e "${YELLOW}🔐 Testing SSH connection...${NC}"
if ! ssh -i "$REMOTE_SSH_KEY" -o ConnectTimeout=5 "$REMOTE_HOST" "echo 'SSH connection successful'"; then
    echo -e "${RED}❌ SSH connection failed${NC}"
    echo "   Check your VM is running and SSH is configured"
    exit 1
fi
echo -e "${GREEN}✅ SSH connection successful${NC}"

# Step 1.5: Get current git commit hash
echo -e "${YELLOW}📋 Getting current git state...${NC}"
cd "$PROJECT_ROOT"
CURRENT_COMMIT=$(git rev-parse HEAD)
CURRENT_BRANCH=$(git branch --show-current)
echo "  Current Branch: $CURRENT_BRANCH"
echo "  Current Commit: ${CURRENT_COMMIT:0:8}"

# Step 2: Sync project files and git state to remote VM
echo -e "${YELLOW}📤 Syncing project files and git state to remote VM...${NC}"

# First sync the .git directory to ensure git state is available
rsync -avz --exclude 'target/' --exclude 'test-ledger/' \
    -e "ssh -i $REMOTE_SSH_KEY" \
    "$PROJECT_ROOT/" "$REMOTE_HOST:$REMOTE_PROJECT_PATH/"

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Files synced successfully${NC}"
else
    echo -e "${RED}❌ File sync failed${NC}"
    exit 1
fi

# Step 2.5: Ensure remote git is at the same commit
echo -e "${YELLOW}🔄 Syncing git state on remote...${NC}"
ssh -i "$REMOTE_SSH_KEY" "$REMOTE_HOST" << EOF
    cd $REMOTE_PROJECT_PATH
    
    # Ensure we're in a git repo
    if [ ! -d ".git" ]; then
        echo "❌ Not a git repository on remote"
        exit 1
    fi
    
    # Fetch latest and checkout the same commit
    echo "📡 Fetching latest git state..."
    git fetch --all
    
    echo "🔄 Checking out commit: $CURRENT_COMMIT"
    git checkout $CURRENT_COMMIT
    
    if [ \$? -eq 0 ]; then
        echo "✅ Git state synchronized"
        echo "   Remote commit: \$(git rev-parse HEAD | cut -c1-8)"
    else
        echo "❌ Failed to checkout commit $CURRENT_COMMIT"
        exit 1
    fi
EOF

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Git state synchronized on remote${NC}"
else
    echo -e "${RED}❌ Git sync failed${NC}"
    exit 1
fi

# Step 3: Build on remote VM using build-bpf.sh
echo -e "${YELLOW}🔨 Building on remote VM using build-bpf.sh...${NC}"
ssh -i "$REMOTE_SSH_KEY" "$REMOTE_HOST" << EOF
    cd $REMOTE_PROJECT_PATH
    
    # Make sure build script is executable
    chmod +x scripts/build-bpf.sh
    
    echo "🦀 Building Solana program on x64 using build-bpf.sh..."
    
    # Run the build script
    ./scripts/build-bpf.sh
    
    # Check if the build was successful
    if [ \$? -eq 0 ] && [ -f "target/deploy/fixed_ratio_trading.so" ]; then
        echo "✅ Build completed successfully"
        echo "📊 Built program info:"
        ls -lh target/deploy/fixed_ratio_trading.so
    else
        echo "❌ Build failed or output not found"
        exit 1
    fi
EOF

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Remote build successful${NC}"
else
    echo -e "${RED}❌ Remote build failed${NC}"
    exit 1
fi

# Step 4: Download compiled binary
echo -e "${YELLOW}📥 Downloading compiled binary...${NC}"
mkdir -p "$PROJECT_ROOT/target/deploy"
scp -i "$REMOTE_SSH_KEY" \
    "$REMOTE_HOST:$REMOTE_PROJECT_PATH/target/deploy/fixed_ratio_trading.so" \
    "$PROJECT_ROOT/target/deploy/"

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Binary downloaded successfully${NC}"
    echo "  Local binary: $(ls -lh "$PROJECT_ROOT/target/deploy/fixed_ratio_trading.so" | awk '{print $5}')"
else
    echo -e "${RED}❌ Binary download failed${NC}"
    exit 1
fi

# Step 5: Check if validator is running locally
echo -e "${YELLOW}🔍 Checking for local validator...${NC}"
if pgrep -f "solana-test-validator" > /dev/null; then
    echo -e "${YELLOW}⚠️  Validator already running. Stopping existing validator...${NC}"
    pkill -f "solana-test-validator"
    sleep 3
fi

# Step 6: Start local validator
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

# Step 7: Configure Solana CLI
echo -e "${YELLOW}⚙️  Configuring Solana CLI...${NC}"
solana config set --url $RPC_URL
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ CLI configured for local testnet${NC}"
else
    echo -e "${RED}❌ CLI configuration failed${NC}"
    kill $VALIDATOR_PID
    exit 1
fi

# Step 8: Check/create keypair
if [ ! -f "$KEYPAIR_PATH" ]; then
    echo -e "${YELLOW}🔑 Creating new keypair...${NC}"
    solana-keygen new --no-bip39-passphrase --outfile $KEYPAIR_PATH
fi

# Step 9: Airdrop SOL
echo -e "${YELLOW}💰 Airdropping SOL...${NC}"
WALLET_ADDRESS=$(solana-keygen pubkey $KEYPAIR_PATH)
echo "  Wallet: $WALLET_ADDRESS"
solana airdrop 100 $WALLET_ADDRESS
sleep 2

# Step 10: Deploy the program
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

# Step 11: Verify deployment
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

# Step 12: Save deployment info
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
  "deployment_method": "remote_build_x64",
  "build_commit": "$CURRENT_COMMIT",
  "build_branch": "$CURRENT_BRANCH"
}
EOF

echo -e "${GREEN}✅ Deployment information saved to deployment_info.json${NC}"

# Final status
echo ""
echo -e "${GREEN}🎉 REMOTE BUILD DEPLOYMENT COMPLETE!${NC}"
echo -e "${GREEN}======================================${NC}"
echo -e "${BLUE}📊 Build Details:${NC}"
echo "  🖥️  Compilation: Remote x64 VM"
echo "  🏠 Execution: Local Mac validator"
echo "  📋 Program ID: $PROGRAM_ID"
echo "  💳 Wallet: $WALLET_ADDRESS"
echo "  📝 Git Commit: ${CURRENT_COMMIT:0:8}"
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