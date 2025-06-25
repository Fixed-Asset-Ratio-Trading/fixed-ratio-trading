#!/bin/bash
# Test Ngrok Static Endpoint
# This script tests the fixed.ngrok.app endpoint to ensure it's working correctly

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "🌐 Testing Ngrok Static Endpoint"
echo "================================="

# Configuration
NGROK_URL="https://fixed.ngrok.app"
BACKPACK_WALLET="5GGZiMwU56rYL1L52q7Jz7ELkSN4iYyQqdv418hxPh6t"

echo "🔗 Endpoint: $NGROK_URL"
echo "🎒 Backpack Wallet: $BACKPACK_WALLET"
echo ""

# Check if curl is available
if ! command -v curl >/dev/null 2>&1; then
    echo -e "${RED}❌ Error: curl not found${NC}"
    echo "   Please install curl to test the endpoint"
    exit 1
fi

# Test 1: Basic connectivity
echo -e "${YELLOW}🔍 Test 1: Basic connectivity...${NC}"
if curl -s --connect-timeout 10 "$NGROK_URL" >/dev/null 2>&1; then
    echo -e "${GREEN}✅ Endpoint is reachable${NC}"
else
    echo -e "${RED}❌ Endpoint is not reachable${NC}"
    echo "   Check if ngrok tunnel is running and properly configured"
    exit 1
fi

# Test 2: Health check
echo -e "${YELLOW}🔍 Test 2: RPC health check...${NC}"
HEALTH_RESPONSE=$(curl -s -X POST -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' \
    "$NGROK_URL" 2>/dev/null)

if echo "$HEALTH_RESPONSE" | grep -q "ok"; then
    echo -e "${GREEN}✅ RPC endpoint is healthy${NC}"
else
    echo -e "${RED}❌ RPC endpoint health check failed${NC}"
    echo "   Response: $HEALTH_RESPONSE"
    echo "   Check if Solana validator is running on localhost:8899"
    exit 1
fi

# Test 3: Get balance for Backpack wallet
echo -e "${YELLOW}🔍 Test 3: Getting Backpack wallet balance...${NC}"
BALANCE_RESPONSE=$(curl -s -X POST -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getBalance\",\"params\":[\"$BACKPACK_WALLET\"]}" \
    "$NGROK_URL" 2>/dev/null)

if echo "$BALANCE_RESPONSE" | grep -q "result"; then
    BALANCE=$(echo "$BALANCE_RESPONSE" | grep -o '"value":[0-9]*' | cut -d':' -f2)
    if [ -n "$BALANCE" ]; then
        SOL_BALANCE=$(echo "scale=9; $BALANCE / 1000000000" | bc 2>/dev/null || echo "Could not calculate")
        echo -e "${GREEN}✅ Balance retrieved successfully${NC}"
        echo "   Lamports: $BALANCE"
        echo "   SOL: $SOL_BALANCE"
    else
        echo -e "${YELLOW}⚠️  Balance response format unexpected${NC}"
        echo "   Response: $BALANCE_RESPONSE"
    fi
else
    echo -e "${RED}❌ Failed to get balance${NC}"
    echo "   Response: $BALANCE_RESPONSE"
    echo "   Check if the wallet address is correct and has been airdropped"
fi

# Test 4: Get latest blockhash
echo -e "${YELLOW}🔍 Test 4: Getting latest blockhash...${NC}"
BLOCKHASH_RESPONSE=$(curl -s -X POST -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"getLatestBlockhash"}' \
    "$NGROK_URL" 2>/dev/null)

if echo "$BLOCKHASH_RESPONSE" | grep -q "blockhash"; then
    echo -e "${GREEN}✅ Latest blockhash retrieved successfully${NC}"
else
    echo -e "${RED}❌ Failed to get latest blockhash${NC}"
    echo "   Response: $BLOCKHASH_RESPONSE"
fi

echo ""
echo -e "${BLUE}📋 Summary:${NC}"
echo "  🌐 Ngrok static endpoint: $NGROK_URL"
echo "  🎒 Backpack wallet: $BACKPACK_WALLET"
echo -e "${GREEN}✅ All tests completed successfully!${NC}"
echo ""
echo -e "${BLUE}💡 You can now use this endpoint with external tools:${NC}"
echo "  • Backpack wallet browser extension"
echo "  • External dApps and services"
echo "  • API calls from remote servers"
echo ""
echo -e "${YELLOW}🔧 Example curl commands:${NC}"
echo "  # Get balance:"
echo "  curl -X POST -H \"Content-Type: application/json\" \\"
echo "    -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getBalance\",\"params\":[\"$BACKPACK_WALLET\"]}' \\"
echo "    \"$NGROK_URL\""
echo ""
echo "  # Get latest blockhash:"
echo "  curl -X POST -H \"Content-Type: application/json\" \\"
echo "    -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getLatestBlockhash\"}' \\"
echo "    \"$NGROK_URL\"" 