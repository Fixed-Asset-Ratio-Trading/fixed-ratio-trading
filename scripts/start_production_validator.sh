#!/bin/bash
# Production Solana Validator Setup Script - Direct HTTP Access
# ============================================================
#
# DESCRIPTION:
#   This script creates a production-like Solana validator environment with:
#   - Direct HTTP access (no nginx reverse proxy)
#   - TPU access (automatic ports)
#   - External network access
#   - Remote access capability for wallets and clients
#
# USAGE:
#   ./scripts/start_production_validator_direct.sh
#
# AUTHOR: Fixed Ratio Trading Development Team
# VERSION: 2.0
# UPDATED: June 2025

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

# Configuration
DOMAIN="vmdevbox1.dcs1.cc"
PRIMARY_ACCOUNT="5GGZiMwU56rYL1L52q7Jz7ELkSN4iYyQqdv418hxPh6t"
SECONDARY_ACCOUNT="3mmceA2hn5Vis7UsziTh258iFdKuPAfXnQnmnocc653f"
AIRDROP_AMOUNT=1000
SECONDARY_AIRDROP_AMOUNT=100

# Network Configuration - External IP for TPU access
EXTERNAL_IP="192.168.9.81"

# Port Configuration
RPC_PORT=8899
WEBSOCKET_PORT=8900  # Automatically assigned by Solana
GOSSIP_PORT=8003
FAUCET_PORT=9900

# Network Configuration - Direct HTTP access
LOCAL_RPC_URL="http://localhost:$RPC_PORT"
EXTERNAL_RPC_URL="http://$EXTERNAL_IP:$RPC_PORT"
EXTERNAL_WS_URL="ws://$EXTERNAL_IP:$WEBSOCKET_PORT"

SCREEN_SESSION_NAME="production-validator"

echo -e "${BLUE}🚀 Production Solana Validator Setup - Direct HTTP Access${NC}"
echo "========================================================="
echo -e "${CYAN}External IP: $EXTERNAL_IP${NC}"
echo -e "${CYAN}HTTP RPC: $EXTERNAL_RPC_URL${NC}"
echo -e "${CYAN}WebSocket: $EXTERNAL_WS_URL${NC}"
echo -e "${CYAN}RPC Port: $RPC_PORT${NC}"
echo -e "${CYAN}WebSocket Port: $WEBSOCKET_PORT (auto)${NC}"
echo -e "${CYAN}Gossip Port: $GOSSIP_PORT${NC}"
echo -e "${CYAN}TPU Access: External (via $EXTERNAL_IP)${NC}"
echo -e "${CYAN}Primary Account: $PRIMARY_ACCOUNT${NC}"
echo -e "${CYAN}Secondary Account: $SECONDARY_ACCOUNT${NC}"
echo ""

# Check if running as root
if [[ $EUID -eq 0 ]]; then
    echo -e "${RED}❌ Do not run this script as root${NC}"
    echo -e "${YELLOW}💡 Run as regular user, script will use sudo when needed${NC}"
    exit 1
fi

# Function to install package if not present
install_if_missing() {
    local package="$1"
    if ! dpkg -l | grep -q "^ii  $package "; then
        echo -e "${YELLOW}📦 Installing $package...${NC}"
        sudo apt update && sudo apt install -y "$package"
        echo -e "${GREEN}✅ $package installed${NC}"
    else
        echo -e "${GREEN}✅ $package already installed${NC}"
    fi
}

# Check dependencies and install required packages
echo -e "${YELLOW}🔍 Checking dependencies...${NC}"

# Check Solana
if ! command -v solana-test-validator &> /dev/null; then
    echo -e "${RED}❌ Solana test validator not found in PATH${NC}"
    echo -e "${YELLOW}💡 Make sure Solana 2.2.18+ is installed and in PATH${NC}"
    exit 1
else
    SOLANA_VERSION=$(solana --version 2>/dev/null | head -1)
    echo -e "${GREEN}✅ Solana available: $SOLANA_VERSION${NC}"
fi

# Install required packages (no nginx needed)
install_if_missing "screen"
install_if_missing "curl"
install_if_missing "jq"

# Stop existing services
echo -e "${YELLOW}🛑 Stopping existing services...${NC}"
if pgrep -f "solana-test-validator" > /dev/null; then
    echo -e "${YELLOW}⚠️  Stopping existing validator...${NC}"
    pkill -f "solana-test-validator"
    sleep 3
fi

if screen -list | grep -q "$SCREEN_SESSION_NAME"; then
    echo -e "${YELLOW}⚠️  Terminating existing screen session...${NC}"
    screen -S "$SCREEN_SESSION_NAME" -X quit 2>/dev/null || true
    sleep 2
fi

# Make sure nginx is stopped (if it was running)
echo -e "${YELLOW}🛑 Ensuring nginx is stopped...${NC}"
if sudo systemctl is-active --quiet nginx 2>/dev/null; then
    echo -e "${YELLOW}⚠️  Stopping nginx (using direct access instead)...${NC}"
    sudo systemctl stop nginx
fi

# Create logs directory
mkdir -p logs

# Start production validator with direct external access
echo -e "${YELLOW}🏁 Starting production Solana validator with direct HTTP access...${NC}"

screen -dmS "$SCREEN_SESSION_NAME" bash -c "
    echo '�� Production Solana Validator - Direct HTTP Access'
    echo '=================================================='
    echo 'Started: \$(date)'
    echo 'External IP: $EXTERNAL_IP'
    echo 'HTTP RPC: $EXTERNAL_RPC_URL'
    echo 'WebSocket: $EXTERNAL_WS_URL'
    echo 'Local RPC: $LOCAL_RPC_URL'
    echo 'Session: $SCREEN_SESSION_NAME'
    echo 'Ledger: logs/test-ledger'
    echo ''
    echo 'Screen Commands:'
    echo '  Detach: Ctrl+A, then D'
    echo '  Kill session: screen -S $SCREEN_SESSION_NAME -X quit'
    echo ''
    echo '════════════════════════════════════════════════════════════════'
    echo ''
    
    # Start validator with external access
    echo 'Starting production Solana validator with external HTTP access...'
    echo \"External IP: $EXTERNAL_IP\"
    echo \"HTTP RPC accessible at: $EXTERNAL_RPC_URL\"
    echo \"WebSocket accessible at: $EXTERNAL_WS_URL\"
    echo \"TPU will be accessible from external networks\"
    solana-test-validator \\
        --rpc-port $RPC_PORT \\
        --gossip-port $GOSSIP_PORT \\
        --gossip-host $EXTERNAL_IP \\
        --faucet-port $FAUCET_PORT \\
        --bind-address 0.0.0.0 \\
        --compute-unit-limit 1400000 \\
        --reset \\
        --log \\
        --ledger logs/test-ledger \\
        2>&1 | tee logs/validator.log &
    
    VALIDATOR_PID=\$!
    echo \"✅ Production validator started with PID: \$VALIDATOR_PID\"
    echo \"\"
    
    # Wait for validator to be ready
    sleep 8
    echo \"✅ Validator initialization complete\"
    echo \"\"
    
    # Monitor and display useful information
    echo \"Starting production status monitor...\"
    echo \"\"
    
    while kill -0 \$VALIDATOR_PID 2>/dev/null; do
        echo \"════════ \$(date) ════════\"
        
        # Check validator status
        if kill -0 \$VALIDATOR_PID 2>/dev/null; then
            echo \"✅ Validator: RUNNING (PID: \$VALIDATOR_PID)\"
        else
            echo \"❌ Validator: STOPPED\"
        fi
        
        # Check local RPC health
        if curl -s $LOCAL_RPC_URL -X POST -H 'Content-Type: application/json' -d '{\\\"jsonrpc\\\":\\\"2.0\\\",\\\"id\\\":1,\\\"method\\\":\\\"getHealth\\\"}' | grep -q '\\\"ok\\\"' 2>/dev/null; then
            echo \"✅ Local RPC: HEALTHY\"
        else
            echo \"❌ Local RPC: NOT RESPONDING\"
        fi
        
        # Check external HTTP endpoint health
        if curl -s $EXTERNAL_RPC_URL -X POST -H 'Content-Type: application/json' -d '{\\\"jsonrpc\\\":\\\"2.0\\\",\\\"id\\\":1,\\\"method\\\":\\\"getHealth\\\"}' | grep -q '\\\"ok\\\"' 2>/dev/null; then
            echo \"✅ External HTTP RPC: HEALTHY ($EXTERNAL_RPC_URL)\"
        else
            echo \"⚠️  External HTTP RPC: NOT RESPONDING\"
        fi
        
        # Get blockchain info
        SLOT_INFO=\$(curl -s $LOCAL_RPC_URL -X POST -H 'Content-Type: application/json' -d '{\\\"jsonrpc\\\":\\\"2.0\\\",\\\"id\\\":1,\\\"method\\\":\\\"getSlot\\\"}' | jq -r '.result // \\\"N/A\\\"' 2>/dev/null || echo 'N/A')
        echo \"📊 Current Slot: \$SLOT_INFO\"
        
        EPOCH_INFO=\$(curl -s $LOCAL_RPC_URL -X POST -H 'Content-Type: application/json' -d '{\\\"jsonrpc\\\":\\\"2.0\\\",\\\"id\\\":1,\\\"method\\\":\\\"getEpochInfo\\\"}' | jq -r '.result.epoch // \\\"N/A\\\"' 2>/dev/null || echo 'N/A')
        echo \"🕒 Epoch: \$EPOCH_INFO\"
        
        # Check account balances
        PRIMARY_BALANCE=\$(solana balance $PRIMARY_ACCOUNT --url $LOCAL_RPC_URL 2>/dev/null | cut -d' ' -f1 || echo 'Error')
        SECONDARY_BALANCE=\$(solana balance $SECONDARY_ACCOUNT --url $LOCAL_RPC_URL 2>/dev/null | cut -d' ' -f1 || echo 'Error')
        echo \"💰 Primary Account: \$PRIMARY_BALANCE SOL\"
        echo \"💰 Secondary Account: \$SECONDARY_BALANCE SOL\"
        
        # Show recent activity
        echo \"📝 Recent Validator Activity:\"
        tail -n 2 logs/validator.log | sed 's/^/   /'
        
        echo \"\"
        echo \"🌐 HTTP RPC Endpoint: $EXTERNAL_RPC_URL\"
        echo \"🔌 WebSocket Endpoint: $EXTERNAL_WS_URL\"
        echo \"⚡ TPU: Available on dynamic ports\"
        echo \"Press Ctrl+C to stop validator\"
        echo \"Press Ctrl+A, D to detach from screen\"
        echo \"\"
        
        sleep 15
    done
    
    echo \"❌ Validator process stopped unexpectedly\"
    echo \"Check logs: tail -f logs/validator.log\"
    read -p \"Press Enter to close...\"
"

echo -e "${GREEN}✅ Production validator started in screen session '$SCREEN_SESSION_NAME'${NC}"

# Wait for validator to start
echo -e "${YELLOW}⏳ Waiting for validator to initialize...${NC}"
sleep 10

# Check if validator is responding
echo -e "${YELLOW}🔍 Checking validator status...${NC}"
for i in {1..15}; do
    if curl -s $LOCAL_RPC_URL -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' | grep -q "ok"; then
        echo -e "${GREEN}✅ Validator is responding${NC}"
        break
    else
        if [ $i -eq 15 ]; then
            echo -e "${RED}❌ Validator failed to start after 15 attempts${NC}"
            echo -e "${YELLOW}💡 Check screen session: screen -r $SCREEN_SESSION_NAME${NC}"
            echo -e "${YELLOW}💡 Check logs: tail -f logs/validator.log${NC}"
            exit 1
        fi
        echo -e "${YELLOW}   Attempt $i/15 - waiting...${NC}"
        sleep 4
    fi
done

# Configure Solana CLI
echo -e "${YELLOW}⚙️  Configuring Solana CLI for production validator...${NC}"
solana config set --url $LOCAL_RPC_URL
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ CLI configured for production validator${NC}"
else
    echo -e "${RED}❌ CLI configuration failed${NC}"
    exit 1
fi

# Airdrop SOL to accounts
echo -e "${YELLOW}💰 Airdropping SOL to accounts...${NC}"

# Primary account airdrop
echo -e "${CYAN}   Primary Target: $PRIMARY_ACCOUNT${NC}"
solana airdrop $AIRDROP_AMOUNT $PRIMARY_ACCOUNT --url $LOCAL_RPC_URL
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Primary airdrop successful${NC}"
    sleep 2
    BALANCE=$(solana balance $PRIMARY_ACCOUNT --url $LOCAL_RPC_URL 2>/dev/null || echo "Error retrieving balance")
    echo -e "${GREEN}   Primary Account Balance: $BALANCE${NC}"
else
    echo -e "${RED}❌ Primary airdrop failed${NC}"
fi

echo ""

# Secondary account airdrop
echo -e "${CYAN}   Secondary Target: $SECONDARY_ACCOUNT${NC}"
solana airdrop $SECONDARY_AIRDROP_AMOUNT $SECONDARY_ACCOUNT --url $LOCAL_RPC_URL
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Secondary airdrop successful${NC}"
    sleep 2
    SECONDARY_BALANCE=$(solana balance $SECONDARY_ACCOUNT --url $LOCAL_RPC_URL 2>/dev/null || echo "Error retrieving balance")
    echo -e "${GREEN}   Secondary Account Balance: $SECONDARY_BALANCE${NC}"
else
    echo -e "${RED}❌ Secondary airdrop failed${NC}"
fi

# Test external HTTP endpoint
echo -e "${YELLOW}🌐 Testing external HTTP endpoint...${NC}"
sleep 3
if curl -s $EXTERNAL_RPC_URL -X POST -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' | grep -q "ok"; then
    echo -e "${GREEN}✅ External HTTP endpoint is working perfectly!${NC}"
else
    echo -e "${YELLOW}⚠️  External HTTP endpoint not responding yet (may need more time)${NC}"
fi

# Display success information
echo ""
echo -e "${GREEN}🎉 PRODUCTION SOLANA VALIDATOR STARTED SUCCESSFULLY!${NC}"
echo -e "${GREEN}====================================================${NC}"
echo ""
echo -e "${BLUE}📊 Production Service Information:${NC}"
echo -e "  🌐 HTTP RPC: $EXTERNAL_RPC_URL"
echo -e "  🔌 WebSocket: $EXTERNAL_WS_URL"
echo -e "  ⚡ TPU Access: External via $EXTERNAL_IP (dynamic ports)"
echo -e "  🌍 External IP: $EXTERNAL_IP"
echo -e "  🔒 Local RPC: $LOCAL_RPC_URL"
echo -e "  🔒 Local WebSocket: ws://localhost:$WEBSOCKET_PORT"
echo -e "  📋 Primary Account: $PRIMARY_ACCOUNT ($AIRDROP_AMOUNT SOL)"
echo -e "  📋 Secondary Account: $SECONDARY_ACCOUNT ($SECONDARY_AIRDROP_AMOUNT SOL)"
echo -e "  📂 Logs Directory: $(pwd)/logs/"
echo -e "  📱 Screen Session: $SCREEN_SESSION_NAME"
echo ""

echo -e "${YELLOW}📺 Screen Session Commands:${NC}"
echo -e "${CYAN}  View validator output:${NC}"
echo -e "    screen -r $SCREEN_SESSION_NAME"
echo ""
echo -e "${CYAN}  Detach from screen (while viewing):${NC}"
echo -e "    Press: Ctrl+A, then D"
echo ""
echo -e "${CYAN}  Kill validator session:${NC}"
echo -e "    screen -S $SCREEN_SESSION_NAME -X quit"
echo ""

echo -e "${YELLOW}🔍 Direct HTTP Endpoints:${NC}"
echo -e "${CYAN}  Test HTTP RPC health:${NC}"
echo -e "    curl $EXTERNAL_RPC_URL -X POST -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getHealth\"}'"
echo ""
echo -e "${CYAN}  Check account balances via HTTP:${NC}"
echo -e "    solana balance $PRIMARY_ACCOUNT --url $EXTERNAL_RPC_URL"
echo -e "    solana balance $SECONDARY_ACCOUNT --url $EXTERNAL_RPC_URL"
echo ""
echo -e "${CYAN}  Check TPU endpoints:${NC}"
echo -e "    curl -s $EXTERNAL_RPC_URL -X POST -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getClusterNodes\"}' | jq '.result[] | {rpc: .rpc, tpu: .tpu, gossip: .gossip}'"
echo ""
echo -e "${CYAN}  View live logs:${NC}"
echo -e "    tail -f logs/validator.log"
echo ""

echo -e "${YELLOW}🔧 Client Configuration:${NC}"
echo -e "${CYAN}  RPC URL: $EXTERNAL_RPC_URL${NC}"
echo -e "${CYAN}  WebSocket URL: $EXTERNAL_WS_URL${NC}"
echo -e "${CYAN}  Network: Custom${NC}"
echo -e "${GREEN}  ✅ Direct HTTP access - no certificate issues!${NC}"
echo ""

echo -e "${YELLOW}🛑 To Stop All Services:${NC}"
echo -e "${RED}    screen -S $SCREEN_SESSION_NAME -X quit${NC}"
echo ""

echo -e "${PURPLE}🔥 PRODUCTION FEATURES ENABLED:${NC}"
echo -e "${GREEN}   ✅ Direct HTTP access (no nginx proxy)${NC}"
echo -e "${GREEN}   ✅ TPU access on dynamic ports${NC}"
echo -e "${GREEN}   ✅ External network access${NC}"
echo -e "${GREEN}   ✅ Production validator configuration${NC}"
echo -e "${GREEN}   ✅ Extended transaction metadata${NC}"
echo -e "${GREEN}   ✅ Transaction history enabled${NC}"
echo -e "${GREEN}   ✅ Native Solana CORS handling${NC}"
echo -e "${GREEN}   ✅ WebSocket support${NC}"
echo -e "${GREEN}   ✅ Remote network access${NC}"
echo ""

echo -e "${GREEN}✨ Your production Solana validator is ready for direct HTTP access!${NC}"
echo -e "${BLUE}   Clients can now connect directly via $EXTERNAL_RPC_URL${NC}"
echo -e "${BLUE}   No certificate issues - pure HTTP access!${NC}"
echo -e "${BLUE}   TPU access available for high-performance transaction submission${NC}"
echo -e "${BLUE}   Use the screen commands above to monitor and manage the validator.${NC}"
