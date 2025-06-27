#!/bin/bash
# Ubuntu 24 Solana Validator Startup Script
# ===========================================
#
# DESCRIPTION:
#   This script starts a local Solana test validator specifically optimized for Ubuntu 24.
#   It runs the validator in a detachable screen session with an interactive monitoring
#   interface that displays real-time validator status, blockchain progress, and account
#   balances. The script automatically airdrops SOL to a specified target account.
#
# FEATURES:
#   ✅ Ubuntu 24 compatibility with automatic dependency installation
#   ✅ Interactive monitoring dashboard in screen session
#   ✅ Real-time RPC health status monitoring
#   ✅ Live blockchain slot and epoch information
#   ✅ Automatic SOL airdrop to target account (1000 SOL)
#   ✅ Background validator with foreground monitoring
#   ✅ Comprehensive logging to files
#   ✅ Clean session management and cleanup
#
# USAGE:
#   ./scripts/start_validator_ubuntu24.sh
#
# SCREEN COMMANDS:
#   View session:    screen -r solana-validator
#   Detach session:  Ctrl+A, then D (while in screen)
#   List sessions:   screen -list
#   Kill session:    screen -S solana-validator -X quit
#
# OUTPUTS:
#   Screen Session:  solana-validator (interactive monitor)
#   Validator Logs:  logs/validator.log (full verbose output)
#   Ledger Data:     logs/test-ledger/ (blockchain data)
#
# MONITORING DISPLAY:
#   The screen session shows:
#   - RPC health status (HEALTHY/NOT RESPONDING)
#   - Current blockchain slot and epoch
#   - Target account balance (live updates)
#   - Recent validator activity (last 3 log lines)
#   - Timestamp updates every 10 seconds
#
# CONFIGURATION:
#   Target Account:  5GGZiMwU56rYL1L52q7Jz7ELkSN4iYyQqdv418hxPh6t
#   Airdrop Amount:  1000 SOL
#   RPC Endpoint:    http://localhost:8899
#   WebSocket:       ws://localhost:8900
#
# REQUIREMENTS:
#   - Ubuntu 24.04 LTS
#   - Solana CLI 2.2.18+ (Agave)
#   - screen package (auto-installed if missing)
#   - curl and jq for monitoring
#
# AUTHOR: Fixed Ratio Trading Development Team
# VERSION: 1.1
# UPDATED: June 2025

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
TARGET_ACCOUNT="5GGZiMwU56rYL1L52q7Jz7ELkSN4iYyQqdv418hxPh6t"
AIRDROP_AMOUNT=1000
RPC_URL="http://localhost:8899"
SCREEN_SESSION_NAME="solana-validator"

echo -e "${BLUE}🚀 Ubuntu 24 Solana Validator Startup${NC}"
echo "======================================"
echo -e "${CYAN}Target Account: $TARGET_ACCOUNT${NC}"
echo -e "${CYAN}Airdrop Amount: $AIRDROP_AMOUNT SOL${NC}"
echo -e "${CYAN}RPC URL: $RPC_URL${NC}"
echo -e "${CYAN}Screen Session: $SCREEN_SESSION_NAME${NC}"
echo ""

# Check dependencies
echo -e "${YELLOW}🔍 Checking dependencies...${NC}"

# Check if screen is installed
if ! command -v screen &> /dev/null; then
    echo -e "${RED}❌ Screen is not installed${NC}"
    echo -e "${YELLOW}💡 Installing screen...${NC}"
    sudo apt update && sudo apt install -y screen
    echo -e "${GREEN}✅ Screen installed${NC}"
else
    echo -e "${GREEN}✅ Screen is available${NC}"
fi

# Check if Solana is available
if ! command -v solana-test-validator &> /dev/null; then
    echo -e "${RED}❌ Solana test validator not found in PATH${NC}"
    echo -e "${YELLOW}💡 Make sure Solana 2.2.18+ is installed and in PATH${NC}"
    echo -e "${YELLOW}   Current PATH includes Solana: $(which solana 2>/dev/null || echo 'Not found')${NC}"
    exit 1
else
    SOLANA_VERSION=$(solana --version 2>/dev/null | head -1)
    echo -e "${GREEN}✅ Solana available: $SOLANA_VERSION${NC}"
fi

# Check if validator is already running
echo -e "${YELLOW}🔍 Checking for existing validator...${NC}"
if pgrep -f "solana-test-validator" > /dev/null; then
    echo -e "${YELLOW}⚠️  Validator already running. Stopping existing validator...${NC}"
    pkill -f "solana-test-validator"
    sleep 3
    echo -e "${GREEN}✅ Existing validator stopped${NC}"
fi

# Check if screen session already exists
if screen -list | grep -q "$SCREEN_SESSION_NAME"; then
    echo -e "${YELLOW}⚠️  Screen session '$SCREEN_SESSION_NAME' already exists. Terminating...${NC}"
    screen -S "$SCREEN_SESSION_NAME" -X quit 2>/dev/null || true
    sleep 2
    echo -e "${GREEN}✅ Existing screen session terminated${NC}"
fi

# Create logs directory
mkdir -p logs

echo ""
echo -e "${YELLOW}🏁 Starting Solana validator in screen session...${NC}"

# Start validator in screen with interactive monitoring
screen -dmS "$SCREEN_SESSION_NAME" bash -c "
    echo '🚀 Solana Test Validator - Interactive Monitor'
    echo '=============================================='
    echo 'Started: \$(date)'
    echo 'RPC URL: $RPC_URL'
    echo 'Session: $SCREEN_SESSION_NAME'
    echo 'Ledger: logs/test-ledger'
    echo ''
    echo 'Screen Commands:'
    echo '  Detach: Ctrl+A, then D'
    echo '  Kill session: screen -S $SCREEN_SESSION_NAME -X quit'
    echo ''
    echo '════════════════════════════════════════════════════════════════'
    echo ''
    
    # Start validator in background
    solana-test-validator \\
        --rpc-port 8899 \\
        --compute-unit-limit 1000000 \\
        --reset \\
        --log \\
        --ledger logs/test-ledger \\
        --bind-address 0.0.0.0 \\
        2>&1 | tee logs/validator.log &
    
    VALIDATOR_PID=\$!
    echo \"Validator started with PID: \$VALIDATOR_PID\"
    echo \"Logs: tail -f logs/validator.log\"
    echo \"\"
    
    # Monitor and display useful information
    sleep 3
    echo \"Starting status monitor...\"
    echo \"\"
    
    while kill -0 \$VALIDATOR_PID 2>/dev/null; do
        echo \"════════ \$(date) ════════\"
        
        # Check if RPC is responding
        if curl -s $RPC_URL -X POST -H 'Content-Type: application/json' -d '{\\\"jsonrpc\\\":\\\"2.0\\\",\\\"id\\\":1,\\\"method\\\":\\\"getHealth\\\"}' | grep -q '\\\"ok\\\"'; then
            echo \"✅ RPC Status: HEALTHY\"
        else
            echo \"❌ RPC Status: NOT RESPONDING\"
        fi
        
        # Get slot info
        SLOT_INFO=\$(curl -s $RPC_URL -X POST -H 'Content-Type: application/json' -d '{\\\"jsonrpc\\\":\\\"2.0\\\",\\\"id\\\":1,\\\"method\\\":\\\"getSlot\\\"}' | jq -r '.result // \\\"N/A\\\"' 2>/dev/null || echo 'N/A')
        echo \"📊 Current Slot: \$SLOT_INFO\"
        
        # Get epoch info
        EPOCH_INFO=\$(curl -s $RPC_URL -X POST -H 'Content-Type: application/json' -d '{\\\"jsonrpc\\\":\\\"2.0\\\",\\\"id\\\":1,\\\"method\\\":\\\"getEpochInfo\\\"}' | jq -r '.result.epoch // \\\"N/A\\\"' 2>/dev/null || echo 'N/A')
        echo \"🕒 Epoch: \$EPOCH_INFO\"
        
        # Check target account balance
        TARGET_BALANCE=\$(solana balance $TARGET_ACCOUNT 2>/dev/null | cut -d' ' -f1 || echo 'Error')
        echo \"💰 Target Account Balance: \$TARGET_BALANCE SOL\"
        
        # Show recent log entries (last 3 lines)
        echo \"📝 Recent Validator Activity:\"
        tail -n 3 logs/validator.log | sed 's/^/   /'
        
        echo \"\"
        echo \"Press Ctrl+C to stop monitoring (validator will continue)\"
        echo \"Press Ctrl+A, D to detach from screen\"
        echo \"\"
        
        sleep 10
    done
    
    echo \"❌ Validator process stopped unexpectedly\"
    echo \"Check logs: tail -f logs/validator.log\"
    read -p \"Press Enter to close...\"
"

echo -e "${GREEN}✅ Validator started in screen session '$SCREEN_SESSION_NAME'${NC}"

# Wait for validator to start
echo -e "${YELLOW}⏳ Waiting for validator to initialize...${NC}"
sleep 8

# Check if validator is responding
echo -e "${YELLOW}🔍 Checking validator status...${NC}"
for i in {1..10}; do
    if curl -s $RPC_URL -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' | grep -q "ok"; then
        echo -e "${GREEN}✅ Validator is responding${NC}"
        break
    else
        if [ $i -eq 10 ]; then
            echo -e "${RED}❌ Validator failed to start after 10 attempts${NC}"
            echo -e "${YELLOW}💡 Check screen session: screen -r $SCREEN_SESSION_NAME${NC}"
            exit 1
        fi
        echo -e "${YELLOW}   Attempt $i/10 - waiting...${NC}"
        sleep 3
    fi
done

# Configure Solana CLI
echo -e "${YELLOW}⚙️  Configuring Solana CLI for local validator...${NC}"
solana config set --url $RPC_URL
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ CLI configured for local validator${NC}"
else
    echo -e "${RED}❌ CLI configuration failed${NC}"
    exit 1
fi

# Airdrop SOL to target account
echo -e "${YELLOW}💰 Airdropping $AIRDROP_AMOUNT SOL to target account...${NC}"
echo -e "${CYAN}   Target: $TARGET_ACCOUNT${NC}"

# Perform airdrop
solana airdrop $AIRDROP_AMOUNT $TARGET_ACCOUNT
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Airdrop successful${NC}"
    
    # Verify balance
    sleep 2
    BALANCE=$(solana balance $TARGET_ACCOUNT 2>/dev/null || echo "Error retrieving balance")
    echo -e "${GREEN}   Account Balance: $BALANCE${NC}"
else
    echo -e "${RED}❌ Airdrop failed${NC}"
    echo -e "${YELLOW}💡 The validator might need more time to initialize${NC}"
fi

# Display success information
echo ""
echo -e "${GREEN}🎉 SOLANA VALIDATOR STARTED SUCCESSFULLY!${NC}"
echo -e "${GREEN}=========================================${NC}"
echo ""
echo -e "${BLUE}📊 Validator Information:${NC}"
echo -e "  🌐 RPC Endpoint: $RPC_URL"
echo -e "  📋 Target Account: $TARGET_ACCOUNT"
echo -e "  💰 Airdropped: $AIRDROP_AMOUNT SOL"
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
echo -e "${CYAN}  List all screen sessions:${NC}"
echo -e "    screen -list"
echo ""
echo -e "${CYAN}  Kill validator session:${NC}"
echo -e "    screen -S $SCREEN_SESSION_NAME -X quit"
echo ""

echo -e "${YELLOW}🔍 Useful Commands:${NC}"
echo -e "${CYAN}  Check validator health:${NC}"
echo -e "    curl $RPC_URL -X POST -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getHealth\"}'"
echo ""
echo -e "${CYAN}  Check account balance:${NC}"
echo -e "    solana balance $TARGET_ACCOUNT"
echo ""
echo -e "${CYAN}  View live logs:${NC}"
echo -e "    tail -f logs/validator.log"
echo ""

echo -e "${YELLOW}🛑 To Stop Everything:${NC}"
echo -e "${RED}    screen -S $SCREEN_SESSION_NAME -X quit${NC}"
echo ""

echo -e "${GREEN}✨ Validator is now running in verbose mode!${NC}"
echo -e "${BLUE}   Use the screen commands above to monitor and manage the validator.${NC}" 