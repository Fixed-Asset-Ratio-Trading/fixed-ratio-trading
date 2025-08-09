#!/bin/bash
# Production Environment Setup Script - Complete Automation
# ========================================================
# FileName: start_production_validator.sh
# 
# DESCRIPTION:
#   This script sets up a complete production environment with:
#   ✅ Auto-detection of host IP address
#   ✅ Automatic Solana validator startup
#   ✅ Smart ngrok management (preserves existing tunnels)
#   ✅ Dedicated screen sessions (ngrok + validator)
#   ✅ Clean validator initialization (--reset)
#   ✅ Intelligent error detection and auto-recovery
#   ✅ Automatic retry with cleanup on startup failures
#   ✅ Global tunnel access (https://fixed.ngrok.app)
#   ✅ Automatic SOL airdrops to configured accounts
#   ✅ Real-time status monitoring
#   ✅ Comprehensive logging
#   ✅ Reset status reporting
#
# USAGE:
#   ./scripts/start_production_validator.sh [--reset]
#   
#   Options:
#     --reset    Force clean blockchain reset (removes all existing accounts/state)
#   
#   Environment variables:
#     EXTERNAL_IP=192.168.1.100 ./scripts/start_production_validator.sh
#
# AUTHOR: Fixed Ratio Trading Development Team
# VERSION: 4.1 - Enhanced Error Recovery Update
# UPDATED: January 2025

set -e

# Parse command line arguments
FORCE_RESET=false
while [[ $# -gt 0 ]]; do
    case $1 in
        --reset)
            FORCE_RESET=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [--reset]"
            echo ""
            echo "Options:"
            echo "  --reset    Force clean blockchain reset (removes all existing accounts/state)"
            echo "  -h, --help Show this help message"
            echo ""
            echo "Environment variables:"
            echo "  EXTERNAL_IP=<ip>  Specify custom external IP address"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

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
VALIDATOR_SESSION_NAME="solana-validator"

# Account Configuration
PRIMARY_ACCOUNT="5GGZiMwU56rYL1L52q7Jz7ELkSN4iYyQqdv418hxPh6t"
SECONDARY_ACCOUNT="3mmceA2hn5Vis7UsziTh258iFdKuPAfXnQnmnocc653f"
AIRDROP_AMOUNT=105
SECONDARY_AIRDROP_AMOUNT=1000

# Reset tracking
RESET_PERFORMED=false

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

# Function to detect and add Solana to PATH for SSH sessions
detect_and_setup_solana() {
    # If Solana is already in PATH, we're good
    if command -v solana &> /dev/null; then
        SOLANA_VERSION=$(solana --version 2>/dev/null | head -1)
        echo -e "${GREEN}✅ solana CLI available: $SOLANA_VERSION${NC}"
        return 0
    fi
    
    echo -e "${YELLOW}⚠️  Solana not in PATH (common in SSH sessions), checking common locations...${NC}"
    
    # Common Solana installation paths (static paths)
    SOLANA_PATHS=(
        "/home/$USER/.local/share/solana/install/active_release/bin"
        "/home/dev/.local/share/solana/install/active_release/bin"  
        "$HOME/.local/share/solana/install/active_release/bin"
        "/root/.local/share/solana/install/active_release/bin"
        "/usr/local/bin"
        "/opt/solana/bin"
    )
    
    SOLANA_FOUND=false
    
    # First check static paths
    for solana_path in "${SOLANA_PATHS[@]}"; do
        if [ -f "$solana_path/solana" ] && [ -f "$solana_path/solana-test-validator" ]; then
            echo -e "${GREEN}✅ Found Solana at: $solana_path${NC}"
            export PATH="$solana_path:$PATH"
            SOLANA_FOUND=true
            break
        fi
    done
    
    # If not found, check release directories with wildcards
    if [ "$SOLANA_FOUND" = false ]; then
        for base_dir in "/home/$USER/.local/share/solana/install/releases" "/home/dev/.local/share/solana/install/releases" "$HOME/.local/share/solana/install/releases"; do
            if [ -d "$base_dir" ]; then
                for release_dir in "$base_dir"/*/solana-release/bin; do
                    if [ -f "$release_dir/solana" ] && [ -f "$release_dir/solana-test-validator" ]; then
                        echo -e "${GREEN}✅ Found Solana at: $release_dir${NC}"
                        export PATH="$release_dir:$PATH"
                        SOLANA_FOUND=true
                        break 2
                    fi
                done
            fi
        done
    fi
    
    if [ "$SOLANA_FOUND" = true ]; then
        SOLANA_VERSION=$(solana --version 2>/dev/null | head -1)
        echo -e "${GREEN}✅ solana CLI available: $SOLANA_VERSION${NC}"
        return 0
    else
        echo -e "${RED}❌ Solana CLI not found in PATH or common locations${NC}"
        echo -e "${YELLOW}💡 Please install Solana CLI or add it to PATH${NC}"
        echo -e "${YELLOW}💡 Common locations checked:${NC}"
        for path in "${SOLANA_PATHS[@]}"; do
            echo -e "${YELLOW}   - $path${NC}"
        done
        echo -e "${YELLOW}💡 To install: sh -c \"\$(curl -sSfL https://release.solana.com/stable/install)\"${NC}"
        return 1
    fi
}

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

# Detect and setup Solana (handles SSH sessions)
if ! detect_and_setup_solana; then
    exit 1
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

# Check for Solana validator and start if needed
echo -e "${YELLOW}🔍 Checking for Solana validator...${NC}"

VALIDATOR_RUNNING=false
NEED_VALIDATOR_START=false

if curl -s $LOCAL_RPC_URL -X POST -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' | grep -q "ok" 2>/dev/null; then
    if [ "$FORCE_RESET" = true ]; then
        echo -e "${YELLOW}⚠️  Validator detected but --reset flag specified${NC}"
        echo -e "${YELLOW}🔄 Stopping existing validator to force clean reset...${NC}"
        
        # Stop existing validator for clean reset
        if screen -list | grep -q "$VALIDATOR_SESSION_NAME"; then
            screen -S "$VALIDATOR_SESSION_NAME" -X quit 2>/dev/null || true
        fi
        if pgrep -f "solana-test-validator" > /dev/null; then
            pkill -f "solana-test-validator" 2>/dev/null || true
            sleep 3
        fi
        VALIDATOR_RUNNING=false
        NEED_VALIDATOR_START=true
    else
        echo -e "${GREEN}✅ Solana validator already running and responding${NC}"
        echo -e "${CYAN}💡 Use --reset flag to force clean blockchain reset${NC}"
        VALIDATOR_RUNNING=true
    fi
else
    echo -e "${YELLOW}⚠️  No Solana validator detected on port $RPC_PORT${NC}"
    NEED_VALIDATOR_START=true
fi

# Start validator if needed
if [ "$NEED_VALIDATOR_START" = true ]; then
    echo -e "${YELLOW}🚀 Starting Solana validator automatically...${NC}"
    
    # Stop any existing validator screen session
    if screen -list | grep -q "$VALIDATOR_SESSION_NAME"; then
        echo -e "${YELLOW}⚠️  Terminating existing validator screen session...${NC}"
        screen -S "$VALIDATOR_SESSION_NAME" -X quit 2>/dev/null || true
        sleep 2
    fi
    
    # Kill any existing validator processes
    if pgrep -f "solana-test-validator" > /dev/null; then
        echo -e "${YELLOW}⚠️  Stopping existing validator processes...${NC}"
        pkill -f "solana-test-validator" 2>/dev/null || true
        sleep 3
    fi
    
    # Function to clean up validator state
    cleanup_validator_state() {
        echo -e "${YELLOW}🧹 Cleaning up validator state...${NC}"
        
        # Remove old test ledger to ensure clean start
        if [ -d "test-ledger" ]; then
            echo -e "${YELLOW}🧹 Removing old ledger data...${NC}"
            rm -rf test-ledger
        fi
        
        # Also remove any backup ledger data
        for ledger_dir in test-ledger-backup* .ledger-backup*; do
            if [ -d "$ledger_dir" ]; then
                echo -e "${YELLOW}🧹 Cleaning up backup ledger: $ledger_dir${NC}"
                rm -rf "$ledger_dir"
            fi
        done
        
        RESET_PERFORMED=true
    }
    
    # Function to check for validator startup errors
    check_validator_errors() {
        if [ -f "test-ledger/validator.log" ]; then
            if grep -q "Address already in use" test-ledger/validator.log; then
                echo -e "${RED}❌ Detected 'Address already in use' error${NC}"
                return 1
            fi
            if grep -q "Faucet failed to start" test-ledger/validator.log; then
                echo -e "${RED}❌ Detected faucet startup failure${NC}"
                return 1
            fi
        fi
        return 0
    }
    
    # Handle reset logic
    RESET_FLAG=""
    if [ "$FORCE_RESET" = true ]; then
        echo -e "${CYAN}🔄 --reset flag detected: Forcing clean blockchain state${NC}"
        RESET_FLAG="--reset"
        cleanup_validator_state
    else
        echo -e "${CYAN}💡 No --reset flag: Preserving existing blockchain state (if any)${NC}"
        # Only remove ledger if it exists and seems corrupted
        if [ -d "test-ledger" ]; then
            if [ ! -f "test-ledger/genesis.bin" ]; then
                echo -e "${YELLOW}🧹 Removing corrupted ledger data...${NC}"
                cleanup_validator_state
                RESET_FLAG="--reset"
            fi
        fi
    fi
    
    # Function to start validator with retry logic
    start_validator_with_retry() {
        local attempt=1
        local max_attempts=3
        local current_reset_flag="$1"
        
        while [ $attempt -le $max_attempts ]; do
            echo -e "${YELLOW}🚀 Validator startup attempt $attempt/$max_attempts${NC}"
            
            # Start validator in screen session
            screen -dmS "$VALIDATOR_SESSION_NAME" bash -c "
                echo '⛓️  Solana Test Validator Manager'
                echo '================================'
                echo 'Started: \$(date)'
                echo 'RPC Port: $RPC_PORT'
                echo 'Bind Address: 0.0.0.0'
                echo 'Session: $VALIDATOR_SESSION_NAME'
                echo 'Attempt: $attempt/$max_attempts'
                echo ''
                echo 'Screen Commands:'
                echo '  Detach: Ctrl+A, then D'
                echo '  Kill session: screen -S $VALIDATOR_SESSION_NAME -X quit'
                echo ''
                echo '════════════════════════════════════════════════════════════════'
                echo ''
                
                echo 'Starting Solana test validator...'
                echo 'RPC URL: http://localhost:$RPC_PORT'
                echo 'WebSocket URL: ws://localhost:$((RPC_PORT + 1))'
                echo 'Logs: test-ledger/validator.log'
                echo 'Reset Flag: $current_reset_flag'
                echo ''
                
                # Start the validator
                solana-test-validator \\
                    --rpc-port $RPC_PORT \\
                    --bind-address 0.0.0.0 \\
                    $current_reset_flag \\
                    --quiet
            "
            
            echo -e "${GREEN}✅ Solana validator started in screen session '$VALIDATOR_SESSION_NAME' (attempt $attempt)${NC}"
            
            # Wait for validator to initialize
            echo -e "${YELLOW}⏳ Waiting for validator to initialize...${NC}"
            RETRY_COUNT=0
            MAX_RETRIES=5
            
            # Wait for startup
            while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
                if curl -s $LOCAL_RPC_URL -X POST -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' | grep -q "ok" 2>/dev/null; then
                    echo -e "${GREEN}✅ Validator is now responding to RPC calls${NC}"
                    VALIDATOR_RUNNING=true
                    return 0
                fi
                
                echo -e "${CYAN}   Attempt $((RETRY_COUNT + 1))/$MAX_RETRIES - waiting...${NC}"
                sleep 2
                RETRY_COUNT=$((RETRY_COUNT + 1))
            done
            
            # Check for specific errors if startup failed
            echo -e "${YELLOW}🔍 Checking for startup errors...${NC}"
            sleep 2  # Give time for logs to be written
            
            if ! check_validator_errors; then
                echo -e "${YELLOW}🔄 Detected startup error - attempting recovery...${NC}"
                
                # Stop the failed validator
                if screen -list | grep -q "$VALIDATOR_SESSION_NAME"; then
                    screen -S "$VALIDATOR_SESSION_NAME" -X quit 2>/dev/null || true
                fi
                if pgrep -f "solana-test-validator" > /dev/null; then
                    pkill -f "solana-test-validator" 2>/dev/null || true
                    sleep 3
                fi
                
                # Clean up state and force reset for next attempt
                cleanup_validator_state
                current_reset_flag="--reset"
                
                echo -e "${CYAN}💡 State cleaned up, will retry with --reset flag${NC}"
            else
                echo -e "${RED}❌ Validator failed to start (attempt $attempt/$max_attempts)${NC}"
                
                if [ -f "test-ledger/validator.log" ]; then
                    echo -e "${YELLOW}📋 Last 10 lines of validator log:${NC}"
                    echo -e "${CYAN}════════════════════════════════════════${NC}"
                    tail -10 test-ledger/validator.log | sed 's/^/   /'
                    echo -e "${CYAN}════════════════════════════════════════${NC}"
                fi
            fi
            
            attempt=$((attempt + 1))
            
            if [ $attempt -le $max_attempts ]; then
                echo -e "${YELLOW}⏳ Waiting 5 seconds before retry...${NC}"
                sleep 5
            fi
        done
        
        # All attempts failed
        echo -e "${RED}❌ Validator failed to start after $max_attempts attempts${NC}"
        echo ""
        echo -e "${YELLOW}💡 Additional debugging options:${NC}"
        echo -e "${YELLOW}   View live logs: screen -r $VALIDATOR_SESSION_NAME${NC}"
        echo -e "${YELLOW}   Follow log file: tail -f test-ledger/validator.log${NC}"
        echo -e "${YELLOW}   Check processes: ps aux | grep solana-test-validator${NC}"
        echo -e "${YELLOW}   Manual cleanup: rm -rf test-ledger && ./scripts/start_production_validator.sh --reset${NC}"
        
        return 1
    }
    
    # Start validator with enhanced error handling
    start_validator_with_retry "$RESET_FLAG"
fi

# Perform CLI configuration and airdrops if validator is running
if [ "$VALIDATOR_RUNNING" = true ]; then
    # Verify Solana CLI is still available (PATH persistence check)
    if ! command -v solana &> /dev/null; then
        echo -e "${RED}❌ Solana CLI lost from PATH - please restart script${NC}"
        exit 1
    fi
    
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
    
    # Verify reset worked if --reset was used
    if [ "$FORCE_RESET" = true ]; then
        echo -e "${YELLOW}🔍 Verifying blockchain reset...${NC}"
        CURRENT_BALANCE=$(solana balance $PRIMARY_ACCOUNT --url $LOCAL_RPC_URL 2>/dev/null | cut -d' ' -f1 || echo "Error")
        if [ "$CURRENT_BALANCE" = "$AIRDROP_AMOUNT" ]; then
            echo -e "${GREEN}✅ Reset verification passed: Account has exactly $CURRENT_BALANCE SOL${NC}"
        elif [ "$CURRENT_BALANCE" = "Error" ]; then
            echo -e "${RED}❌ Reset verification failed: Could not check account balance${NC}"
        else
            echo -e "${RED}❌ Reset verification failed: Account has $CURRENT_BALANCE SOL (expected $AIRDROP_AMOUNT SOL)${NC}"
            echo -e "${YELLOW}⚠️  This indicates the blockchain state was not properly reset${NC}"
        fi
    fi
    
    echo ""
else
    echo -e "${RED}❌ Validator not running - airdrops skipped${NC}"
    echo -e "${YELLOW}💡 To manually start: screen -r $VALIDATOR_SESSION_NAME${NC}"
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
echo -e "  ⛓️  Validator Screen: $VALIDATOR_SESSION_NAME"
echo ""
echo -e "${BLUE}💰 Account Configuration:${NC}"
echo -e "  🥇 Primary Account: $PRIMARY_ACCOUNT ($AIRDROP_AMOUNT SOL)"
echo -e "  🥈 Secondary Account: $SECONDARY_ACCOUNT ($SECONDARY_AIRDROP_AMOUNT SOL)"
echo ""

echo -e "${YELLOW}📺 Screen Session Commands:${NC}"
echo -e "${CYAN}  View ngrok status:${NC}"
echo -e "    screen -r $NGROK_SESSION_NAME"
echo ""
echo -e "${CYAN}  View validator status:${NC}"
echo -e "    screen -r $VALIDATOR_SESSION_NAME"
echo ""
echo -e "${CYAN}  Detach from screen:${NC}"
echo -e "    Press: Ctrl+A, then D"
echo ""
echo -e "${CYAN}  Stop ngrok tunnel:${NC}"
echo -e "    screen -S $NGROK_SESSION_NAME -X quit"
echo ""
echo -e "${CYAN}  Stop validator:${NC}"
echo -e "    screen -S $VALIDATOR_SESSION_NAME -X quit"
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
echo -e "${GREEN}   ✅ Automatic Solana validator startup${NC}"
echo -e "${GREEN}   ✅ Smart ngrok management (preserves existing tunnels)${NC}"
echo -e "${GREEN}   ✅ Dedicated screen sessions (ngrok + validator)${NC}"
echo -e "${GREEN}   ✅ Clean validator initialization (--reset)${NC}"
echo -e "${GREEN}   ✅ Intelligent error detection and auto-recovery${NC}"
echo -e "${GREEN}   ✅ Automatic retry with cleanup on startup failures${NC}"
echo -e "${GREEN}   ✅ Global tunnel access ($NGROK_URL)${NC}"
echo -e "${GREEN}   ✅ Automatic SOL airdrops to configured accounts${NC}"
echo -e "${GREEN}   ✅ Real-time status monitoring${NC}"
echo -e "${GREEN}   ✅ Comprehensive logging${NC}"
echo -e "${GREEN}   ✅ Reset status reporting${NC}"
echo ""

echo -e "${GREEN}✨ Production environment is now fully operational!${NC}"
echo -e "${BLUE}   🌐 Global access: $NGROK_URL${NC}"
echo -e "${BLUE}   ⛓️  Validator: Running in screen session${NC}"
echo -e "${BLUE}   📱 Monitor ngrok: screen -r $NGROK_SESSION_NAME${NC}"
echo -e "${BLUE}   📱 Monitor validator: screen -r $VALIDATOR_SESSION_NAME${NC}"
echo -e "${BLUE}   💰 Accounts funded and ready for transactions${NC}"

# Reset status report
echo ""
echo -e "${PURPLE}🔄 RESET STATUS REPORT:${NC}"
if [ "$RESET_PERFORMED" = true ]; then
    echo -e "${YELLOW}   ⚠️  VALIDATOR WAS RESET during this session${NC}"
    echo -e "${YELLOW}   📋 Blockchain state was cleaned and reinitialized${NC}"
    echo -e "${YELLOW}   💡 All previous transactions and accounts were cleared${NC}"
    if [ "$FORCE_RESET" = true ]; then
        echo -e "${CYAN}   🎯 Reset was requested via --reset flag${NC}"
    else
        echo -e "${CYAN}   🔧 Reset was performed automatically to fix startup issues${NC}"
    fi
else
    echo -e "${GREEN}   ✅ NO RESET PERFORMED - blockchain state preserved${NC}"
    echo -e "${GREEN}   📋 Existing transactions and accounts remain intact${NC}"
    echo -e "${CYAN}   💡 Use --reset flag to force clean blockchain state${NC}"
fi
