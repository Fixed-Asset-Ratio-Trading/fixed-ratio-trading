#!/bin/bash

# Metaplex Local Development Manager
# This script manages the deployment and lifecycle of Metaplex programs for local development

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
METAPLEX_DIR="$PROJECT_ROOT/.metaplex"
METAPLEX_PROGRAMS_DIR="$METAPLEX_DIR/programs"
METAPLEX_LOGS_DIR="$METAPLEX_DIR/logs"
METAPLEX_PID_FILE="$METAPLEX_DIR/metaplex.pid"

# Metaplex Program IDs (standard addresses used on mainnet/devnet)
TOKEN_METADATA_PROGRAM_ID="metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
CANDY_MACHINE_PROGRAM_ID="cndy3Z4yapfJBmL3ShUp5exZKqR3z33thTzeNMm2gRZ"
AUCTION_HOUSE_PROGRAM_ID="hausS13jsjafwWwGqZTUQRmWyvyxn9EQpqMwV1PBBmk"

# RPC Configuration
# Default to local validator for development, can be overridden with environment variables
RPC_URL="${RPC_URL:-http://localhost:8899}"
WEBSOCKET_URL="${WEBSOCKET_URL:-ws://localhost:8900}"

# Function to print colored output
print_status() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

# Function to create necessary directories
setup_directories() {
    mkdir -p "$METAPLEX_DIR"
    mkdir -p "$METAPLEX_PROGRAMS_DIR"
    mkdir -p "$METAPLEX_LOGS_DIR"
}

# Function to check if Solana validator is running
check_solana_validator() {
    if ! curl -s "$RPC_URL" -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' >/dev/null 2>&1; then
        print_status "$RED" "❌ Solana validator is not running at $RPC_URL"
        print_status "$YELLOW" "Please start the Solana validator first"
        exit 1
    fi
    print_status "$GREEN" "✅ Solana validator is running"
}

# Function to download Metaplex programs
download_metaplex_programs() {
    print_status "$BLUE" "📥 Getting Metaplex programs from network..."
    
    cd "$METAPLEX_PROGRAMS_DIR"
    
    # Use solana program dump to get programs from mainnet
    # This is more reliable than downloading binaries
    
    # Download Token Metadata Program from mainnet
    if [ ! -f "mpl_token_metadata.so" ]; then
        print_status "$YELLOW" "Dumping Token Metadata Program from mainnet..."
        print_status "$BLUE" "⏳ This may take a few minutes - downloading from mainnet..."
        
        # Use timeout if available to prevent hanging
        if command -v timeout &> /dev/null; then
            if timeout 300 solana program dump "$TOKEN_METADATA_PROGRAM_ID" mpl_token_metadata.so --url mainnet-beta; then
                print_status "$GREEN" "✅ Token Metadata Program downloaded"
            else
                print_status "$RED" "❌ Failed to download Token Metadata Program (timeout or network error)"
                print_status "$YELLOW" "💡 Will try to use canonical program ID instead"
                # Create a placeholder file to avoid re-attempting download
                touch "mpl_token_metadata.so.failed"
            fi
        else
            if solana program dump "$TOKEN_METADATA_PROGRAM_ID" mpl_token_metadata.so --url mainnet-beta; then
                print_status "$GREEN" "✅ Token Metadata Program downloaded"
            else
                print_status "$RED" "❌ Failed to download Token Metadata Program"
                print_status "$YELLOW" "💡 Will try to use canonical program ID instead"
                # Create a placeholder file to avoid re-attempting download
                touch "mpl_token_metadata.so.failed"
            fi
        fi
    fi
    
    # Download Candy Machine Program from mainnet (optional)
    if [ ! -f "mpl_candy_machine_core.so" ] && [ ! -f "mpl_candy_machine_core.so.failed" ]; then
        print_status "$YELLOW" "Dumping Candy Machine Program from mainnet..."
        print_status "$BLUE" "⏳ This is optional - downloading from mainnet..."
        
        # Use timeout if available to prevent hanging
        if command -v timeout &> /dev/null; then
            if timeout 180 solana program dump "$CANDY_MACHINE_PROGRAM_ID" mpl_candy_machine_core.so --url mainnet-beta; then
                print_status "$GREEN" "✅ Candy Machine Program downloaded"
            else
                print_status "$YELLOW" "⚠️  Failed to download Candy Machine Program (timeout or network error)"
                print_status "$YELLOW" "💡 This is optional - continuing without it"
                touch "mpl_candy_machine_core.so.failed"
            fi
        else
            if solana program dump "$CANDY_MACHINE_PROGRAM_ID" mpl_candy_machine_core.so --url mainnet-beta; then
                print_status "$GREEN" "✅ Candy Machine Program downloaded"
            else
                print_status "$YELLOW" "⚠️  Failed to download Candy Machine Program"
                print_status "$YELLOW" "💡 This is optional - continuing without it"
                touch "mpl_candy_machine_core.so.failed"
            fi
        fi
    fi
    
    # Download Auction House Program from mainnet (optional)
    if [ ! -f "mpl_auction_house.so" ] && [ ! -f "mpl_auction_house.so.failed" ]; then
        print_status "$YELLOW" "Dumping Auction House Program from mainnet..."
        print_status "$BLUE" "⏳ This is optional - downloading from mainnet..."
        
        # Use timeout if available to prevent hanging
        if command -v timeout &> /dev/null; then
            if timeout 180 solana program dump "$AUCTION_HOUSE_PROGRAM_ID" mpl_auction_house.so --url mainnet-beta; then
                print_status "$GREEN" "✅ Auction House Program downloaded"
            else
                print_status "$YELLOW" "⚠️  Failed to download Auction House Program (timeout or network error)"
                print_status "$YELLOW" "💡 This is optional - continuing without it"
                touch "mpl_auction_house.so.failed"
            fi
        else
            if solana program dump "$AUCTION_HOUSE_PROGRAM_ID" mpl_auction_house.so --url mainnet-beta; then
                print_status "$GREEN" "✅ Auction House Program downloaded"
            else
                print_status "$YELLOW" "⚠️  Failed to download Auction House Program"
                print_status "$YELLOW" "💡 This is optional - continuing without it"
                touch "mpl_auction_house.so.failed"
            fi
        fi
    fi
    
    print_status "$GREEN" "✅ Metaplex programs obtained successfully"
}

# Function to deploy Metaplex programs
deploy_metaplex_programs() {
    print_status "$BLUE" "🚀 Configuring Metaplex programs..."
    
    cd "$METAPLEX_PROGRAMS_DIR"
    
    #================================================================================
    # IMPORTANT: Do NOT deploy a custom Token Metadata Program. Always use canonical ID.
    # We rely on the validator to preload the binary at the canonical address via --bpf-program.
    print_status "$YELLOW" "⏭️  Skipping on-chain deployment of Token Metadata Program"
    print_status "$BLUE" "🔗 Using canonical Token Metadata Program ID: $TOKEN_METADATA_PROGRAM_ID"
    echo "$TOKEN_METADATA_PROGRAM_ID" > "$METAPLEX_DIR/token_metadata_program_id.txt"
    update_shared_config_with_program_id "$TOKEN_METADATA_PROGRAM_ID"
    #================================================================================
    
    # Deploy Candy Machine Program (optional)
    if [ -f "mpl_candy_machine_core.so" ] && [ ! -f "mpl_candy_machine_core.so.failed" ]; then
        print_status "$YELLOW" "Deploying Candy Machine Program..."
        CANDY_MACHINE_DEPLOYMENT=$(solana program deploy mpl_candy_machine_core.so \
            --url "$RPC_URL" \
            --upgrade-authority ~/.config/solana/id.json 2>&1)
        
        if echo "$CANDY_MACHINE_DEPLOYMENT" | grep -q "Program Id:"; then
            DEPLOYED_CANDY_MACHINE_ID=$(echo "$CANDY_MACHINE_DEPLOYMENT" | grep "Program Id:" | awk '{print $3}')
            print_status "$GREEN" "✅ Candy Machine Program deployed at: $DEPLOYED_CANDY_MACHINE_ID"
            echo "$DEPLOYED_CANDY_MACHINE_ID" > "$METAPLEX_DIR/candy_machine_program_id.txt"
        else
            print_status "$YELLOW" "⚠️ Candy Machine Program deployment failed (non-critical)"
            echo "$CANDY_MACHINE_PROGRAM_ID" > "$METAPLEX_DIR/candy_machine_program_id.txt"
        fi
    else
        print_status "$YELLOW" "⚠️ Candy Machine Program binary not available, using canonical program ID"
        echo "$CANDY_MACHINE_PROGRAM_ID" > "$METAPLEX_DIR/candy_machine_program_id.txt"
    fi
    
    # Deploy Auction House Program (optional)
    if [ -f "mpl_auction_house.so" ] && [ ! -f "mpl_auction_house.so.failed" ]; then
        print_status "$YELLOW" "Deploying Auction House Program..."
        AUCTION_HOUSE_DEPLOYMENT=$(solana program deploy mpl_auction_house.so \
            --url "$RPC_URL" \
            --upgrade-authority ~/.config/solana/id.json 2>&1)
        
        if echo "$AUCTION_HOUSE_DEPLOYMENT" | grep -q "Program Id:"; then
            DEPLOYED_AUCTION_HOUSE_ID=$(echo "$AUCTION_HOUSE_DEPLOYMENT" | grep "Program Id:" | awk '{print $3}')
            print_status "$GREEN" "✅ Auction House Program deployed at: $DEPLOYED_AUCTION_HOUSE_ID"
            echo "$DEPLOYED_AUCTION_HOUSE_ID" > "$METAPLEX_DIR/auction_house_program_id.txt"
        else
            print_status "$YELLOW" "⚠️ Auction House Program deployment failed (non-critical)"
            echo "$AUCTION_HOUSE_PROGRAM_ID" > "$METAPLEX_DIR/auction_house_program_id.txt"
        fi
    else
        print_status "$YELLOW" "⚠️ Auction House Program binary not available, using canonical program ID"
        echo "$AUCTION_HOUSE_PROGRAM_ID" > "$METAPLEX_DIR/auction_house_program_id.txt"
    fi
    
    print_status "$GREEN" "✅ Metaplex programs configured successfully"
    print_status "$BLUE" "📋 Deployed Program IDs:"
    if [ -f "$METAPLEX_DIR/token_metadata_program_id.txt" ]; then
        print_status "$BLUE" "  Token Metadata: $(cat "$METAPLEX_DIR/token_metadata_program_id.txt")"
    fi
    if [ -f "$METAPLEX_DIR/candy_machine_program_id.txt" ]; then
        print_status "$BLUE" "  Candy Machine: $(cat "$METAPLEX_DIR/candy_machine_program_id.txt")"
    fi
    if [ -f "$METAPLEX_DIR/auction_house_program_id.txt" ]; then
        print_status "$BLUE" "  Auction House: $(cat "$METAPLEX_DIR/auction_house_program_id.txt")"
    fi
}

# Function to check if Metaplex programs are deployed
check_metaplex_programs() {
    local token_metadata_deployed=false
    local candy_machine_deployed=false
    local auction_house_deployed=false
    
    # Check deployed Token Metadata Program (from our deployment)
    if [ -f "$METAPLEX_DIR/token_metadata_program_id.txt" ]; then
        local deployed_token_metadata_id=$(cat "$METAPLEX_DIR/token_metadata_program_id.txt")
        if solana program show "$deployed_token_metadata_id" --url "$RPC_URL" >/dev/null 2>&1; then
            if [ "$deployed_token_metadata_id" = "$TOKEN_METADATA_PROGRAM_ID" ]; then
                print_status "$GREEN" "✅ Token Metadata Program is deployed (canonical: $deployed_token_metadata_id)"
            else
                print_status "$GREEN" "✅ Token Metadata Program is deployed (custom: $deployed_token_metadata_id)"
            fi
            token_metadata_deployed=true
        else
            print_status "$RED" "❌ Token Metadata Program not accessible ($deployed_token_metadata_id)"
        fi
    else
        # Check canonical program ID as fallback
        if solana program show "$TOKEN_METADATA_PROGRAM_ID" --url "$RPC_URL" >/dev/null 2>&1; then
            print_status "$GREEN" "✅ Token Metadata Program is deployed (canonical)"
            token_metadata_deployed=true
        else
            print_status "$RED" "❌ Token Metadata Program not deployed"
        fi
    fi
    
    # Check deployed Candy Machine Program (optional)
    if [ -f "$METAPLEX_DIR/candy_machine_program_id.txt" ]; then
        local deployed_candy_machine_id=$(cat "$METAPLEX_DIR/candy_machine_program_id.txt")
        if solana program show "$deployed_candy_machine_id" --url "$RPC_URL" >/dev/null 2>&1; then
            print_status "$GREEN" "✅ Candy Machine Program is deployed ($deployed_candy_machine_id)"
            candy_machine_deployed=true
        else
            print_status "$YELLOW" "⚠️ Candy Machine Program not accessible ($deployed_candy_machine_id)"
        fi
    else
        # Check canonical program ID as fallback
        if solana program show "$CANDY_MACHINE_PROGRAM_ID" --url "$RPC_URL" >/dev/null 2>&1; then
            print_status "$GREEN" "✅ Candy Machine Program is deployed (canonical)"
            candy_machine_deployed=true
        else
            print_status "$YELLOW" "⚠️ Candy Machine Program not deployed (optional)"
        fi
    fi
    
    # Check deployed Auction House Program (optional)
    if [ -f "$METAPLEX_DIR/auction_house_program_id.txt" ]; then
        local deployed_auction_house_id=$(cat "$METAPLEX_DIR/auction_house_program_id.txt")
        if solana program show "$deployed_auction_house_id" --url "$RPC_URL" >/dev/null 2>&1; then
            print_status "$GREEN" "✅ Auction House Program is deployed ($deployed_auction_house_id)"
            auction_house_deployed=true
        else
            print_status "$YELLOW" "⚠️ Auction House Program not accessible ($deployed_auction_house_id)"
        fi
    else
        # Check canonical program ID as fallback
        if solana program show "$AUCTION_HOUSE_PROGRAM_ID" --url "$RPC_URL" >/dev/null 2>&1; then
            print_status "$GREEN" "✅ Auction House Program is deployed (canonical)"
            auction_house_deployed=true
        else
            print_status "$YELLOW" "⚠️ Auction House Program not deployed (optional)"
        fi
    fi
    
    # Return success if at least Token Metadata Program is deployed (most important)
    if [ "$token_metadata_deployed" = true ]; then
        return 0
    else
        return 1
    fi
}

# Function to start Metaplex (deploy programs)
start_metaplex() {
    print_status "$BLUE" "🏁 Starting Metaplex local deployment..."
    
    setup_directories
    check_solana_validator
    
    if check_metaplex_programs; then
        print_status "$GREEN" "✅ Metaplex programs are already deployed"
        echo $$ > "$METAPLEX_PID_FILE"
        return 0
    fi
    
    download_metaplex_programs
    deploy_metaplex_programs
    
    # Create PID file to track deployment
    echo $$ > "$METAPLEX_PID_FILE"
    
    print_status "$GREEN" "🎉 Metaplex local deployment completed successfully!"
    print_status "$BLUE" "Program IDs:"
    print_status "$BLUE" "  Token Metadata: $TOKEN_METADATA_PROGRAM_ID"
    print_status "$BLUE" "  Candy Machine:  $CANDY_MACHINE_PROGRAM_ID"
    print_status "$BLUE" "  Auction House:  $AUCTION_HOUSE_PROGRAM_ID"
}

# Function to stop Metaplex (remove programs)
stop_metaplex() {
    local reset_flag="${1:-false}"
    
    print_status "$BLUE" "🛑 Stopping Metaplex local deployment..."
    
    # Note: In a local validator, programs are automatically cleared when validator restarts
    # This function mainly cleans up tracking files
    
    if [ "$reset_flag" = "true" ]; then
        if [ -f "$METAPLEX_PID_FILE" ]; then
            rm "$METAPLEX_PID_FILE"
            print_status "$GREEN" "✅ Metaplex tracking cleared"
        fi
        
        # Also clean up program ID files and failed download markers when resetting
        if [ -f "$METAPLEX_DIR/token_metadata_program_id.txt" ]; then
            rm "$METAPLEX_DIR/token_metadata_program_id.txt"
            print_status "$GREEN" "✅ Token Metadata Program ID cleared"
        fi
        if [ -f "$METAPLEX_DIR/candy_machine_program_id.txt" ]; then
            rm "$METAPLEX_DIR/candy_machine_program_id.txt"
            print_status "$GREEN" "✅ Candy Machine Program ID cleared"
        fi
        if [ -f "$METAPLEX_DIR/auction_house_program_id.txt" ]; then
            rm "$METAPLEX_DIR/auction_house_program_id.txt"
            print_status "$GREEN" "✅ Auction House Program ID cleared"
        fi
        
        # Clean up failed download markers to allow retry
        cd "$METAPLEX_PROGRAMS_DIR" 2>/dev/null || mkdir -p "$METAPLEX_PROGRAMS_DIR"
        rm -f *.so.failed 2>/dev/null || true
        if [ -f "mpl_token_metadata.so.failed" ] || [ -f "mpl_candy_machine_core.so.failed" ] || [ -f "mpl_auction_house.so.failed" ]; then
            print_status "$GREEN" "✅ Failed download markers cleared"
        fi
        
        print_status "$YELLOW" "ℹ️  Metaplex programs will be cleared when Solana validator restarts"
    else
        print_status "$YELLOW" "ℹ️  Metaplex deployment preserved (use --reset to clear tracking)"
        print_status "$YELLOW" "ℹ️  Metaplex programs will be cleared when Solana validator restarts"
    fi
}

# Function to get Metaplex status
status_metaplex() {
    print_status "$BLUE" "📊 Metaplex Status:"
    
    if [ -f "$METAPLEX_PID_FILE" ]; then
        print_status "$GREEN" "✅ Metaplex tracking file exists"
    else
        print_status "$YELLOW" "⚠️  Metaplex tracking file not found"
    fi
    
    print_status "$BLUE" "Program deployment status:"
    check_metaplex_programs
}

# Function to update shared-config.json with new program ID
update_shared_config_with_program_id() {
    local program_id="$1"
    local config_file="$PROJECT_ROOT/shared-config.json"
    local dashboard_config="$PROJECT_ROOT/dashboard/shared-config.json"
    
    print_status "$BLUE" "📝 Updating shared-config.json with new Token Metadata Program ID"
    
    if [ -f "$config_file" ]; then
        # Use jq to update the JSON if available, otherwise use sed
        if command -v jq &> /dev/null; then
            # Update using jq (preserves JSON formatting)
            local temp_file=$(mktemp)
            jq ".metaplex.tokenMetadataProgramId = \"$program_id\" | .metaplex.lastUpdated = \"$(date -u +%Y-%m-%d)\"" "$config_file" > "$temp_file"
            mv "$temp_file" "$config_file"
            print_status "$GREEN" "✅ Updated shared-config.json using jq"
        else
            # Fallback to sed-based update
            sed -i.bak "s/\"tokenMetadataProgramId\": \"[^\"]*\"/\"tokenMetadataProgramId\": \"$program_id\"/" "$config_file"
            sed -i.bak "s/\"lastUpdated\": \"[^\"]*\"/\"lastUpdated\": \"$(date -u +%Y-%m-%d)\"/" "$config_file"
            rm -f "$config_file.bak"
            print_status "$GREEN" "✅ Updated shared-config.json using sed"
        fi
        
        # Copy to dashboard directory
        if [ -f "$config_file" ]; then
            cp "$config_file" "$dashboard_config"
            print_status "$GREEN" "✅ Copied updated config to dashboard directory"
        fi
    else
        print_status "$RED" "❌ shared-config.json not found at $config_file"
    fi
}

# Function to show usage
show_usage() {
    echo "Usage: $0 {start|stop [--reset]|status|restart [--reset]}"
    echo ""
    echo "Commands:"
    echo "  start          - Deploy Metaplex programs to local validator"
    echo "  stop           - Preserve Metaplex deployment state (safe stop)"
    echo "  stop --reset   - Clear Metaplex tracking and program IDs (full reset)"
    echo "  status         - Show current Metaplex deployment status"
    echo "  restart        - Stop and start Metaplex deployment (preserves state)"
    echo "  restart --reset - Reset and restart Metaplex deployment (full reset)"
    echo ""
    echo "Options:"
    echo "  --reset        - Clear all tracking files and program IDs (use with stop/restart)"
    echo ""
    echo "Environment Variables:"
    echo "  RPC_URL        - Solana RPC endpoint (default: http://localhost:8899)"
    echo "  WEBSOCKET_URL  - Solana WebSocket endpoint (default: ws://localhost:8900)"
}

# Main script logic
case "${1:-}" in
    start)
        start_metaplex
        ;;
    stop)
        # Check if --reset flag is provided as second argument
        if [ "${2:-}" = "--reset" ]; then
            stop_metaplex true
        else
            stop_metaplex false
        fi
        ;;
    status)
        status_metaplex
        ;;
    restart)
        # Check if --reset flag is provided as second argument
        if [ "${2:-}" = "--reset" ]; then
            stop_metaplex true
        else
            stop_metaplex false
        fi
        start_metaplex
        ;;
    *)
        show_usage
        exit 1
        ;;
esac 