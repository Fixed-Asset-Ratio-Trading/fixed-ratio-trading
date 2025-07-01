#!/bin/bash
# Production Environment Setup Script - ngrok Focus
# =================================================
#
# DESCRIPTION:
#   This script sets up a production environment with:
#   - Auto-detection of host IP address
#   - ngrok tunnel in dedicated screen session
#   - Independent operation (works with or without validator)
#   - Smart ngrok management (doesn't restart if already running)
#
# USAGE:
#   ./scripts/start_production_validator.sh
#   
#   Or with custom IP:
#   EXTERNAL_IP=192.168.1.100 ./scripts/start_production_validator.sh
#
# AUTHOR: Fixed Ratio Trading Development Team
# VERSION: 3.0
# UPDATED: January 2025

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

# Function to auto-detect external IP
detect_external_ip() {
    local detected_ip=""
    
    # Method 1: Get IP from default route interface
    if command -v ip &> /dev/null; then
        detected_ip=$(ip route get 8.8.8.8 2>/dev/null | grep -oP 'src \K\S+' | head -1)
        if [[ -n "$detected_ip" && "$detected_ip" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            echo "$detected_ip"
            return 0
        fi
    fi
    
    # Method 2: Get primary interface IP
    if command -v hostname &> /dev/null; then
        detected_ip=$(hostname -I | awk '{print $1}')
        if [[ -n "$detected_ip" && "$detected_ip" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            echo "$detected_ip"
            return 0
        fi
    fi
    
    # Method 3: Check common network interfaces
    for interface in eth0 enp0s3 enp0s8 wlan0 ens33; do
        if command -v ip &> /dev/null; then
            detected_ip=$(ip addr show $interface 2>/dev/null | grep 'inet ' | awk '{print $2}' | cut -d/ -f1 | head -1)
            if [[ -n "$detected_ip" && "$detected_ip" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
                echo "$detected_ip"
                return 0
            fi
        fi
    done
    
    # Method 4: Fallback to ifconfig if available
    if command -v ifconfig &> /dev/null; then
        detected_ip=$(ifconfig | grep 'inet ' | grep -v '127.0.0.1' | awk '{print $2}' | head -1)
        if [[ -n "$detected_ip" && "$detected_ip" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            echo "$detected_ip"
            return 0
        fi
    fi
    
    echo "127.0.0.1"
    return 1
}

# Configuration
RPC_PORT=8899
NGROK_URL="https://fixed.ngrok.app"
NGROK_SESSION_NAME="ngrok-tunnel"

# Account Configuration
PRIMARY_ACCOUNT="5GGZiMwU56rYL1L52q7Jz7ELkSN4iYyQqdv418hxPh6t"
SECONDARY_ACCOUNT="3mmceA2hn5Vis7UsziTh258iFdKuPAfXnQnmnocc653f"
AIRDROP_AMOUNT=100
SECONDARY_AIRDROP_AMOUNT=10

# Network Configuration - Auto-detect or use environment variable
if [[ -n "$EXTERNAL_IP" ]]; then
    echo -e "${CYAN}🔧 Using provided EXTERNAL_IP: $EXTERNAL_IP${NC}"
    if [[ ! "$EXTERNAL_IP" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo -e "${RED}❌ Invalid IP address format: $EXTERNAL_IP${NC}"
        exit 1
    fi
else
    echo -e "${YELLOW}🔍 Auto-detecting external IP address...${NC}"
    EXTERNAL_IP=$(detect_external_ip)
    if [[ "$EXTERNAL_IP" == "127.0.0.1" ]]; then
        echo -e "${RED}❌ Could not auto-detect IP address${NC}"
        echo -e "${YELLOW}⚠️  Using localhost - remote access will be limited${NC}"
        echo -e "${YELLOW}💡 To specify a custom IP: EXTERNAL_IP=your.ip.address ./scripts/start_production_validator.sh${NC}"
    else
        echo -e "${GREEN}✅ Auto-detected IP: $EXTERNAL_IP${NC}"
    fi
fi

LOCAL_RPC_URL="http://localhost:$RPC_PORT"
EXTERNAL_RPC_URL="http://$EXTERNAL_IP:$RPC_PORT"

echo -e "${BLUE}🚀 Production Environment Setup - ngrok Focus${NC}"
echo "================================================"
echo -e "${CYAN}Auto-detected IP: $EXTERNAL_IP${NC}"
echo -e "${CYAN}Local HTTP RPC: $LOCAL_RPC_URL${NC}"
echo -e "${CYAN}External HTTP RPC: $EXTERNAL_RPC_URL${NC}"
echo -e "${CYAN}ngrok Tunnel: $NGROK_URL${NC}"
echo ""

# Check dependencies
echo -e "${YELLOW}🔍 Checking dependencies...${NC}"

if ! command -v screen &> /dev/null; then
    echo -e "${RED}❌ screen not found - please install: apt install screen${NC}"
    exit 1
else
    echo -e "${GREEN}✅ screen available${NC}"
fi

if ! command -v curl &> /dev/null; then
    echo -e "${RED}❌ curl not found - please install: apt install curl${NC}"
    exit 1
else
    echo -e "${GREEN}✅ curl available${NC}"
fi

if ! command -v ngrok &> /dev/null; then
    echo -e "${RED}❌ ngrok not found - please install ngrok${NC}"
    exit 1
else
    NGROK_VERSION=$(ngrok version 2>/dev/null | head -1)
    echo -e "${GREEN}✅ ngrok available: $NGROK_VERSION${NC}"
fi

if ! command -v solana &> /dev/null; then
    echo -e "${RED}❌ solana CLI not found - please install Solana CLI${NC}"
    exit 1
else
    SOLANA_VERSION=$(solana --version 2>/dev/null | head -1)
    echo -e "${GREEN}✅ solana CLI available: $SOLANA_VERSION${NC}"
fi

# Check if ngrok is already running
echo -e "${YELLOW}🔍 Checking ngrok status...${NC}"
if pgrep -f "ngrok.*8899" > /dev/null; then
    echo -e "${GREEN}✅ ngrok already running on port $RPC_PORT${NC}"
    NGROK_ALREADY_RUNNING=true
else
    echo -e "${YELLOW}⚠️  ngrok not running on port $RPC_PORT${NC}"
    NGROK_ALREADY_RUNNING=false
fi

# Create logs directory
mkdir -p logs

# Handle ngrok setup
if [ "$NGROK_ALREADY_RUNNING" = true ]; then
    echo -e "${CYAN}🔄 Keeping existing ngrok tunnel running${NC}"
    echo -e "${YELLOW}💡 To restart ngrok: pkill -f ngrok && run this script again${NC}"
else
    echo -e "${YELLOW}🌐 Starting ngrok tunnel in dedicated screen session...${NC}"
    
    # Stop any existing ngrok screen session
    if screen -list | grep -q "$NGROK_SESSION_NAME"; then
        echo -e "${YELLOW}⚠️  Terminating existing ngrok screen session...${NC}"
        screen -S "$NGROK_SESSION_NAME" -X quit 2>/dev/null || true
        sleep 2
    fi
    
    # Start ngrok in screen session
    screen -dmS "$NGROK_SESSION_NAME" bash -c "
        echo '🌐 ngrok Tunnel Manager'
        echo '======================'
        echo 'Started: \$(date)'
        echo 'Local Port: $RPC_PORT'
        echo 'Public URL: $NGROK_URL'
        echo 'Log File: logs/ngrok.log'
        echo 'Session: $NGROK_SESSION_NAME'
        echo ''
        echo 'Screen Commands:'
        echo '  Detach: Ctrl+A, then D'
        echo '  Kill session: screen -S $NGROK_SESSION_NAME -X quit'
        echo ''
        echo '════════════════════════════════════════════════════════════════'
        echo ''
        
        echo 'Starting ngrok tunnel...'
        echo \"Target: http://localhost:$RPC_PORT\"
        echo \"Public URL: $NGROK_URL\"
        echo \"Log: logs/ngrok.log\"
        echo ''
        
        ngrok http $RPC_PORT --hostname=fixed.ngrok.app --log=logs/ngrok.log &
        NGROK_PID=\$!
        
        echo \"✅ ngrok started with PID: \$NGROK_PID\"
        echo \"\"
        
        # Wait for ngrok to initialize
        sleep 5
        echo \"✅ ngrok tunnel ready\"
        echo \"\"
        
        # Monitor ngrok status
        echo \"Starting ngrok status monitor...\"
        echo \"\"
        
        while kill -0 \$NGROK_PID 2>/dev/null; do
            echo \"════════ \$(date) ════════\"
            
            # Check ngrok process status
            if kill -0 \$NGROK_PID 2>/dev/null; then
                echo \"✅ ngrok Process: RUNNING (PID: \$NGROK_PID)\"
            else
                echo \"❌ ngrok Process: STOPPED\"
                break
            fi
            
            # Check local port accessibility
            if curl -s http://localhost:$RPC_PORT > /dev/null 2>&1; then
                echo \"✅ Local Port $RPC_PORT: ACCESSIBLE\"
            else
                echo \"⚠️  Local Port $RPC_PORT: NOT ACCESSIBLE (no service running)\"
            fi
            
            # Check tunnel health
            if curl -s $NGROK_URL > /dev/null 2>&1; then
                echo \"✅ ngrok Tunnel: ACTIVE ($NGROK_URL)\"
            else
                echo \"⚠️  ngrok Tunnel: NOT RESPONDING\"
            fi
            
            # Show recent ngrok activity
            echo \"📝 Recent ngrok Activity:\"
            tail -n 2 logs/ngrok.log 2>/dev/null | sed 's/^/   /' || echo '   (no recent activity)'
            
            echo \"\"
            echo \"🌐 Tunnel URL: $NGROK_URL\"
            echo \"📍 Local Target: http://localhost:$RPC_PORT\"
            echo \"📱 Screen Session: $NGROK_SESSION_NAME\"
            echo \"Press Ctrl+C to stop tunnel\"
            echo \"Press Ctrl+A, D to detach from screen\"
            echo \"\"
            
            sleep 15
        done
        
        echo \"❌ ngrok process stopped unexpectedly\"
        echo \"Check logs: tail -f logs/ngrok.log\"
        read -p \"Press Enter to close...\"
    "
    
    echo -e "${GREEN}✅ ngrok tunnel started in screen session '$NGROK_SESSION_NAME'${NC}"
    
    # Wait for ngrok to initialize
    echo -e "${YELLOW}⏳ Waiting for ngrok to initialize...${NC}"
    sleep 8
fi

# Test ngrok tunnel
echo -e "${YELLOW}🧪 Testing ngrok tunnel...${NC}"
if curl -s $NGROK_URL > /dev/null 2>&1; then
    echo -e "${GREEN}✅ ngrok tunnel is accessible at $NGROK_URL${NC}"
else
    echo -e "${YELLOW}⚠️  ngrok tunnel not responding yet (may need more time or no service on port $RPC_PORT)${NC}"
fi

# Check for Solana validator and perform airdrops
echo -e "${YELLOW}🔍 Checking for Solana validator...${NC}"
if curl -s $LOCAL_RPC_URL -X POST -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' | grep -q "ok" 2>/dev/null; then
    echo -e "${GREEN}✅ Solana validator detected and responding${NC}"
    
    # Configure Solana CLI
    echo -e "${YELLOW}⚙️  Configuring Solana CLI...${NC}"
    solana config set --url $LOCAL_RPC_URL >/dev/null 2>&1
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ CLI configured for local validator${NC}"
    else
        echo -e "${RED}❌ CLI configuration failed${NC}"
    fi
    
    # Perform airdrops
    echo -e "${YELLOW}💰 Performing SOL airdrops...${NC}"
    
    # Primary account airdrop
    echo -e "${CYAN}   Primary Target: $PRIMARY_ACCOUNT${NC}"
    if solana airdrop $AIRDROP_AMOUNT $PRIMARY_ACCOUNT --url $LOCAL_RPC_URL >/dev/null 2>&1; then
        sleep 2
        BALANCE=$(solana balance $PRIMARY_ACCOUNT --url $LOCAL_RPC_URL 2>/dev/null | cut -d' ' -f1 || echo "Error")
        echo -e "${GREEN}✅ Primary airdrop successful: $BALANCE SOL${NC}"
    else
        echo -e "${RED}❌ Primary airdrop failed${NC}"
    fi
    
    # Secondary account airdrop
    echo -e "${CYAN}   Secondary Target: $SECONDARY_ACCOUNT${NC}"
    if solana airdrop $SECONDARY_AIRDROP_AMOUNT $SECONDARY_ACCOUNT --url $LOCAL_RPC_URL >/dev/null 2>&1; then
        sleep 2
        SECONDARY_BALANCE=$(solana balance $SECONDARY_ACCOUNT --url $LOCAL_RPC_URL 2>/dev/null | cut -d' ' -f1 || echo "Error")
        echo -e "${GREEN}✅ Secondary airdrop successful: $SECONDARY_BALANCE SOL${NC}"
    else
        echo -e "${RED}❌ Secondary airdrop failed${NC}"
    fi
    
    echo ""
else
    echo -e "${YELLOW}⚠️  No Solana validator detected on port $RPC_PORT${NC}"
    echo -e "${YELLOW}💡 Start a validator manually to enable airdrops:${NC}"
    echo -e "${CYAN}    solana-test-validator --rpc-port $RPC_PORT --bind-address 0.0.0.0 --reset${NC}"
    echo ""
fi

# Display service information
echo ""
echo -e "${GREEN}🎉 ENVIRONMENT SETUP COMPLETE!${NC}"
echo -e "${GREEN}===============================${NC}"
echo ""
echo -e "${BLUE}📊 Service Information:${NC}"
echo -e "  🌍 Auto-detected IP: $EXTERNAL_IP"
echo -e "  🔗 Local RPC: $LOCAL_RPC_URL"
echo -e "  🌐 External RPC: $EXTERNAL_RPC_URL"
echo -e "  🌍 Global ngrok URL: $NGROK_URL"
echo -e "  📂 Logs Directory: $(pwd)/logs/"
echo -e "  📱 ngrok Screen: $NGROK_SESSION_NAME"
echo ""
echo -e "${BLUE}💰 Account Configuration:${NC}"
echo -e "  🥇 Primary Account: $PRIMARY_ACCOUNT ($AIRDROP_AMOUNT SOL)"
echo -e "  🥈 Secondary Account: $SECONDARY_ACCOUNT ($SECONDARY_AIRDROP_AMOUNT SOL)"
echo ""

echo -e "${YELLOW}📺 ngrok Screen Commands:${NC}"
echo -e "${CYAN}  View ngrok status:${NC}"
echo -e "    screen -r $NGROK_SESSION_NAME"
echo ""
echo -e "${CYAN}  Detach from screen:${NC}"
echo -e "    Press: Ctrl+A, then D"
echo ""
echo -e "${CYAN}  Stop ngrok tunnel:${NC}"
echo -e "    screen -S $NGROK_SESSION_NAME -X quit"
echo ""

echo -e "${YELLOW}🧪 Testing Commands:${NC}"
echo -e "${CYAN}  Test tunnel health:${NC}"
echo -e "    curl $NGROK_URL"
echo ""
echo -e "${CYAN}  Test with mock server:${NC}"
echo -e "    python3 -m http.server $RPC_PORT &"
echo -e "    curl $NGROK_URL"
echo ""
echo -e "${CYAN}  View ngrok logs:${NC}"
echo -e "    tail -f logs/ngrok.log"
echo ""

echo -e "${YELLOW}🔧 For Solana Validator:${NC}"
echo -e "${CYAN}  Start validator manually (clean environment):${NC}"
echo -e "    solana-test-validator --rpc-port $RPC_PORT --bind-address 0.0.0.0 --reset"
echo -e "${YELLOW}    Note: --reset ensures fresh blockchain state each startup${NC}"
echo ""
echo -e "${CYAN}  Test RPC via tunnel:${NC}"
echo -e "    curl $NGROK_URL -X POST -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getHealth\"}'"
echo ""
echo -e "${CYAN}  Check account balances:${NC}"
echo -e "    solana balance $PRIMARY_ACCOUNT"
echo -e "    solana balance $SECONDARY_ACCOUNT"
echo ""
echo -e "${CYAN}  Manual airdrop (if needed):${NC}"
echo -e "    solana airdrop $AIRDROP_AMOUNT $PRIMARY_ACCOUNT"
echo -e "    solana airdrop $SECONDARY_AIRDROP_AMOUNT $SECONDARY_ACCOUNT"
echo ""

echo -e "${PURPLE}🔥 FEATURES ENABLED:${NC}"
echo -e "${GREEN}   ✅ Auto-detection of host IP address${NC}"
echo -e "${GREEN}   ✅ Smart ngrok management (preserves existing tunnels)${NC}"
echo -e "${GREEN}   ✅ Dedicated ngrok screen session${NC}"
echo -e "${GREEN}   ✅ Independent operation (works with/without validator)${NC}"
echo -e "${GREEN}   ✅ Global tunnel access ($NGROK_URL)${NC}"
echo -e "${GREEN}   ✅ Automatic SOL airdrops (when validator detected)${NC}"
echo -e "${GREEN}   ✅ Real-time status monitoring${NC}"
echo -e "${GREEN}   ✅ Comprehensive logging${NC}"
echo ""

echo -e "${GREEN}✨ ngrok tunnel is now running independently!${NC}"
echo -e "${BLUE}   Global access: $NGROK_URL${NC}"
echo -e "${BLUE}   Works with any service on port $RPC_PORT${NC}"
echo -e "${BLUE}   Monitor status: screen -r $NGROK_SESSION_NAME${NC}"
echo -e "${BLUE}   The tunnel will persist and work with or without a validator${NC}"
