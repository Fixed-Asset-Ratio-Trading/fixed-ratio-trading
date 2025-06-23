#!/bin/bash
# Monitor Fixed Ratio Trading Pools from Command Line
# Displays real-time pool information and statistics

echo "📊 Fixed Ratio Trading Pool Monitor"
echo "===================================="

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Configuration
RPC_URL="http://localhost:8899"
PROGRAM_ID="quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD"
REFRESH_INTERVAL=5

# Function to clear screen and show header
show_header() {
    clear
    echo -e "${BOLD}${BLUE}📊 Fixed Ratio Trading Pool Monitor${NC}"
    echo -e "${BLUE}====================================${NC}"
    echo -e "${CYAN}RPC: $RPC_URL${NC}"
    echo -e "${CYAN}Program: $PROGRAM_ID${NC}"
    echo -e "${CYAN}Updated: $(date)${NC}"
    echo ""
}

# Function to check if validator is running
check_validator() {
    if ! curl -s $RPC_URL -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' > /dev/null 2>&1; then
        echo -e "${RED}❌ Validator not running at $RPC_URL${NC}"
        echo "   Please start the validator first: ./deploy_local.sh"
        return 1
    fi
    return 0
}

# Function to check if program is deployed
check_program() {
    if ! solana program show $PROGRAM_ID --url $RPC_URL > /dev/null 2>&1; then
        echo -e "${RED}❌ Program not deployed${NC}"
        echo "   Please deploy the program first: ./deploy_local.sh"
        return 1
    fi
    return 0
}

# Function to get block height
get_block_height() {
    curl -s $RPC_URL -X POST -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","id":1,"method":"getBlockHeight"}' | \
        grep -o '"result":[0-9]*' | \
        cut -d':' -f2
}

# Function to get program accounts
get_program_accounts() {
    curl -s $RPC_URL -X POST -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getProgramAccounts\",\"params\":[\"$PROGRAM_ID\",{\"encoding\":\"base64\"}]}" | \
        grep -o '"pubkey":"[^"]*"' | \
        wc -l
}

# Function to display system status
show_system_status() {
    echo -e "${BOLD}🌐 System Status${NC}"
    echo "----------------"
    
    if check_validator; then
        echo -e "${GREEN}✅ Validator: Online${NC}"
        BLOCK_HEIGHT=$(get_block_height)
        echo -e "${GREEN}📊 Block Height: $BLOCK_HEIGHT${NC}"
    else
        echo -e "${RED}❌ Validator: Offline${NC}"
        return 1
    fi
    
    if check_program; then
        echo -e "${GREEN}✅ Program: Deployed${NC}"
    else
        echo -e "${RED}❌ Program: Not Found${NC}"
        return 1
    fi
    
    echo ""
}

# Function to display pool statistics
show_pool_stats() {
    echo -e "${BOLD}🏊‍♂️ Pool Statistics${NC}"
    echo "-------------------"
    
    # Get program accounts count (approximate pool count)
    ACCOUNT_COUNT=$(get_program_accounts)
    echo -e "${CYAN}📊 Program Accounts: $ACCOUNT_COUNT${NC}"
    
    # Note: More detailed pool parsing would require proper RPC calls
    # For now, show basic information
    if [ "$ACCOUNT_COUNT" -gt 0 ]; then
        echo -e "${GREEN}✅ Pools detected on chain${NC}"
        echo -e "${YELLOW}💡 For detailed pool info, use the web dashboard${NC}"
    else
        echo -e "${YELLOW}📭 No pools found${NC}"
        echo -e "${YELLOW}   Create pools with: ./create_sample_pools.sh${NC}"
    fi
    
    echo ""
}

# Function to display helpful information
show_info() {
    echo -e "${BOLD}🔗 Quick Links${NC}"
    echo "---------------"
    echo -e "${CYAN}🌐 Web Dashboard: http://localhost:3000${NC}"
    echo -e "${CYAN}📊 Start Dashboard: ./start_dashboard.sh${NC}"
    echo -e "${CYAN}🏊‍♂️ Create Pools: ./create_sample_pools.sh${NC}"
    echo -e "${CYAN}🛑 Stop Validator: pkill -f solana-test-validator${NC}"
    echo ""
    
    echo -e "${BOLD}📝 Commands${NC}"
    echo "------------"
    echo -e "${CYAN}r - Refresh now${NC}"
    echo -e "${CYAN}q - Quit monitor${NC}"
    echo -e "${CYAN}h - Show this help${NC}"
    echo ""
}

# Function to run monitoring loop
run_monitor() {
    # Set up non-blocking input
    stty -echo
    
    while true; do
        show_header
        
        # Show status
        if ! show_system_status; then
            echo -e "${RED}❌ Cannot monitor - system not ready${NC}"
            echo ""
            show_info
            echo -e "${YELLOW}Press 'q' to quit, 'r' to retry...${NC}"
        else
            show_pool_stats
            show_info
            echo -e "${YELLOW}Auto-refresh in ${REFRESH_INTERVAL}s... (r=refresh, q=quit, h=help)${NC}"
        fi
        
        # Wait for input or timeout
        for i in $(seq 1 $REFRESH_INTERVAL); do
            # Check for input (non-blocking)
            read -t 1 -n 1 input 2>/dev/null
            
            case $input in
                'q'|'Q')
                    echo ""
                    echo -e "${GREEN}👋 Monitor stopped${NC}"
                    stty echo
                    exit 0
                    ;;
                'r'|'R')
                    break 2
                    ;;
                'h'|'H')
                    show_header
                    show_info
                    echo -e "${YELLOW}Press any key to continue...${NC}"
                    read -n 1
                    break 2
                    ;;
            esac
            
            # Clear input
            input=""
        done
    done
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -i, --interval SECONDS    Set refresh interval (default: 5)"
    echo "  -u, --url URL            Set RPC URL (default: http://localhost:8899)"
    echo "  -h, --help               Show this help"
    echo ""
    echo "Examples:"
    echo "  $0                       Start monitor with default settings"
    echo "  $0 -i 10                 Refresh every 10 seconds"
    echo "  $0 -u http://localhost:8900  Use different RPC endpoint"
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -i|--interval)
            REFRESH_INTERVAL="$2"
            if ! [[ "$REFRESH_INTERVAL" =~ ^[0-9]+$ ]] || [ "$REFRESH_INTERVAL" -lt 1 ]; then
                echo "Error: Refresh interval must be a positive integer"
                exit 1
            fi
            shift 2
            ;;
        -u|--url)
            RPC_URL="$2"
            shift 2
            ;;
        -h|--help)
            show_usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Main execution
echo -e "${BOLD}📊 Starting Pool Monitor...${NC}"
echo -e "${CYAN}Refresh interval: ${REFRESH_INTERVAL}s${NC}"
echo -e "${CYAN}RPC URL: $RPC_URL${NC}"
echo ""
echo -e "${YELLOW}Press Enter to continue...${NC}"
read

# Trap Ctrl+C to cleanup
trap "echo ''; echo -e '${GREEN}👋 Monitor stopped${NC}'; stty echo; exit 0" INT

# Start monitoring
run_monitor 