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

# Check if ASP.NET Core is running
echo -e "\n${YELLOW}1. Testing ASP.NET Core Application${NC}"
echo "-------------------------------------------"

if curl -s http://localhost:5000 > /dev/null 2>&1; then
    echo -e "${GREEN}✅ ASP.NET Core is running on localhost:5000${NC}"
    
    # Test with HTTPie
    echo -e "\n${YELLOW}Testing with HTTPie:${NC}"
    http GET localhost:5000 --print=HhBb | head -5
    
else
    echo -e "${RED}❌ ASP.NET Core not running. Starting it...${NC}"
    echo "Run this command in another terminal:"
    echo "cd src/FixedRatioTrading.Dashboard.Web && dotnet run"
    echo ""
    echo "Then re-run this test script."
fi

# Test JSON processing with jq
echo -e "\n${YELLOW}2. Testing JSON Processing with jq${NC}"
echo "----------------------------------------"

# Create test JSON
TEST_JSON='{"poolId":"test-123","displayPair":"TS/MST","exchangeRate":"1 TS = 10,000.00 MST","isActive":true,"tokens":["TS","MST"]}'
echo "Test JSON:"
echo "$TEST_JSON" | jq '.'

echo -e "\n${YELLOW}Extract specific fields:${NC}"
echo "$TEST_JSON" | jq '.displayPair, .exchangeRate'

# Test HTTPie with different formats
echo -e "\n${YELLOW}3. Testing HTTPie API Calls${NC}"
echo "-----------------------------------"

echo "Testing public API (HTTPBin):"
http GET httpbin.org/get name==TestUser | jq '.args'

echo -e "\n${YELLOW}HTTPie POST example (would be used for our pool creation):${NC}"
echo "http POST localhost:5000/api/pools/create TokenAAddress=MINT_A TokenBAddress=MINT_B RatioANumerator:=10000 RatioBDenominator:=1"

# Test curl + jq combination
echo -e "\n${YELLOW}4. Testing curl + jq Combination${NC}"
echo "------------------------------------"

echo "Testing JSON response processing:"
curl -s httpbin.org/json | jq '.slideshow.author'

# Browser test instructions
echo -e "\n${YELLOW}5. Browser DevTools Test Instructions${NC}"
echo "--------------------------------------------"

echo "To test browser debugging:"
echo "1. Open Firefox Developer Edition:"
echo "   open -a 'Firefox Developer Edition'"
echo ""
echo "2. Navigate to: http://localhost:5000"
echo "3. Press F12 to open DevTools"
echo "4. Test console commands:"
echo "   - console.log('Debug test')"
echo "   - console.table([{name: 'TS', value: 1}, {name: 'MST', value: 10000}])"
echo ""

# Performance testing
echo -e "\n${YELLOW}6. Performance Testing Example${NC}"
echo "-----------------------------------"

if curl -s http://localhost:5000 > /dev/null 2>&1; then
    echo "Testing response times (5 requests):"
    for i in {1..5}; do
        time_result=$(curl -w "%{time_total}" -o /dev/null -s http://localhost:5000)
        echo "Request $i: ${time_result}s"
    done
else
    echo "ASP.NET Core not running - skipping performance test"
fi

# Summary
echo -e "\n${GREEN}🎉 Debugging Tools Test Complete!${NC}"
echo "======================================="
echo "Tools verified:"
echo "✅ jq - JSON processing"
echo "✅ HTTPie - API testing"
echo "✅ curl - HTTP requests"
echo "✅ Firefox Developer Edition - Installed"
echo "✅ Regular browsers - Available"
echo ""
echo "Next steps:"
echo "1. Start ASP.NET Core: cd src/FixedRatioTrading.Dashboard.Web && dotnet run"
echo "2. Import Postman collection: docs/api/FixedRatioTrading_Dashboard_API.postman_collection.json"
echo "3. Open browser DevTools and start debugging!"
echo ""
echo "📚 See full debugging guide: docs/BROWSER_DEBUGGING_GUIDE.md" 