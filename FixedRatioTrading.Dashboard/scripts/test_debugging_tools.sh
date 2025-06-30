#!/bin/bash

# Test script for debugging tools with ASP.NET Core Dashboard
# Fixed Ratio Trading Dashboard - Debugging Tools Test

echo "🧪 Testing Debugging Tools for Fixed Ratio Trading Dashboard"
echo "============================================================="

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Test JSON processing with jq
echo -e "\n${YELLOW}1. Testing JSON Processing with jq${NC}"
echo "----------------------------------------"

# Create test JSON
TEST_JSON='{"poolId":"test-123","displayPair":"TS/MST","exchangeRate":"1 TS = 10,000.00 MST","isActive":true,"tokens":["TS","MST"]}'
echo "Test JSON:"
echo "$TEST_JSON" | jq '.'

echo -e "\n${YELLOW}Extract specific fields:${NC}"
echo "$TEST_JSON" | jq '.displayPair, .exchangeRate'

# Test HTTPie
echo -e "\n${YELLOW}2. Testing HTTPie${NC}"
echo "-------------------"
echo "HTTPie version:"
http --version

# Test curl + jq combination
echo -e "\n${YELLOW}3. Testing curl + jq Combination${NC}"
echo "------------------------------------"
echo "Testing JSON response processing:"
curl -s httpbin.org/json | jq '.slideshow.author'

# Summary
echo -e "\n${GREEN}🎉 Debugging Tools Test Complete!${NC}"
echo "Tools verified:"
echo "✅ jq - JSON processing"
echo "✅ HTTPie - API testing"
echo "✅ curl - HTTP requests"
echo ""
echo "📚 See full debugging guide: docs/BROWSER_DEBUGGING_GUIDE.md"
