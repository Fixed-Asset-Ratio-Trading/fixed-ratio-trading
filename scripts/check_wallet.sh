#!/bin/bash
# Check Wallet and Deployment Information
# Shows current keypair status, balance, and deployment details

# Find the project root directory (where Cargo.toml is located)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Verify we found the correct project directory
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    echo "❌ Error: Could not find Cargo.toml in project root: $PROJECT_ROOT"
    echo "   Please run this script from the fixed-ratio-trading project directory or its subdirectories"
    exit 1
fi

echo "🔑 Fixed Ratio Trading - Wallet Information"
echo "============================================"
echo "📂 Project Root: $PROJECT_ROOT"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration paths
KEYPAIR_PATH="$HOME/.config/solana/id.json"
CONFIG_PATH="$HOME/.config/solana/cli/config.yml"
DEPLOYMENT_INFO="$PROJECT_ROOT/deployment_info.json"

echo -e "${BLUE}📁 File Locations:${NC}"
echo "  Keypair: $KEYPAIR_PATH"
echo "  Config: $CONFIG_PATH"
echo "  Deployment Info: $DEPLOYMENT_INFO"
echo ""

# Check Solana CLI configuration
echo -e "${BLUE}⚙️  Solana CLI Configuration:${NC}"
if command -v solana &> /dev/null; then
    solana config get
else
    echo -e "${RED}❌ Solana CLI not installed${NC}"
    exit 1
fi
echo ""

# Check if keypair exists
echo -e "${BLUE}🔑 Keypair Status:${NC}"
if [ -f "$KEYPAIR_PATH" ]; then
    echo -e "${GREEN}✅ Keypair exists: $KEYPAIR_PATH${NC}"
    
    # Get wallet address
    WALLET_ADDRESS=$(solana-keygen pubkey $KEYPAIR_PATH 2>/dev/null)
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}📍 Wallet Address: $WALLET_ADDRESS${NC}"
        
        # Check balance
        echo -e "${BLUE}💰 Checking wallet balance...${NC}"
        BALANCE=$(solana balance $WALLET_ADDRESS 2>/dev/null)
        if [ $? -eq 0 ]; then
            echo -e "${GREEN}💵 Balance: $BALANCE${NC}"
        else
            echo -e "${YELLOW}⚠️  Could not check balance (validator may not be running)${NC}"
        fi
    else
        echo -e "${RED}❌ Could not read wallet address from keypair${NC}"
    fi
else
    echo -e "${YELLOW}⚠️  Keypair does not exist yet${NC}"
    echo -e "${CYAN}   Will be created during deployment: $PROJECT_ROOT/scripts/deploy_local.sh${NC}"
fi
echo ""

# Check deployment information
echo -e "${BLUE}🚀 Deployment Information:${NC}"
if [ -f "$DEPLOYMENT_INFO" ]; then
    echo -e "${GREEN}✅ Deployment info found: $DEPLOYMENT_INFO${NC}"
    echo ""
    echo -e "${CYAN}📋 Deployment Details:${NC}"
    
    if command -v jq &> /dev/null; then
        # Pretty print with jq if available
        cat $DEPLOYMENT_INFO | jq '.'
    else
        # Fallback to basic display
        cat $DEPLOYMENT_INFO
    fi
else
    echo -e "${YELLOW}⚠️  No deployment info found${NC}"
    echo -e "${CYAN}   Will be created after deployment: $PROJECT_ROOT/scripts/deploy_local.sh${NC}"
fi
echo ""

# Security recommendations
echo -e "${BLUE}🛡️  Security Notes:${NC}"
echo -e "${CYAN}Local Testnet (Safe):${NC}"
echo "  ✅ This is a LOCAL testnet keypair"
echo "  ✅ Only used for development/testing"
echo "  ✅ Contains test SOL (no real value)"
echo "  ✅ Safe to share for debugging"
echo ""
echo -e "${YELLOW}⚠️  For Mainnet (Important):${NC}"
echo "  🔒 NEVER share your mainnet keypair"
echo "  🔒 Backup your keypair securely"
echo "  🔒 Use hardware wallets for large amounts"
echo ""

# Quick actions
echo -e "${BLUE}🔧 Quick Actions:${NC}"
echo "  📊 Check this info: $PROJECT_ROOT/scripts/check_wallet.sh"
echo "  🚀 Deploy contract: $PROJECT_ROOT/scripts/deploy_local.sh"
echo "  🌐 Open dashboard: $PROJECT_ROOT/scripts/start_dashboard.sh"
echo "  🏊‍♂️ Create pools: $PROJECT_ROOT/scripts/create_sample_pools.sh"
echo ""

# Backup instructions
if [ -f "$KEYPAIR_PATH" ]; then
    echo -e "${BLUE}💾 Backup Instructions:${NC}"
    echo "  To backup your keypair:"
    echo "    cp $KEYPAIR_PATH $PROJECT_ROOT/my_wallet_backup.json"
    echo ""
    echo "  To view your private key (for importing elsewhere):"
    echo "    cat $KEYPAIR_PATH"
    echo ""
fi 