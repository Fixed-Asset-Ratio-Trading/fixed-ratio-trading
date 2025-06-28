#!/bin/bash
# Solana Validator Firewall and Port Accessibility Checker
# ========================================================
#
# This script checks and configures firewall settings for optimal
# Solana validator external access including RPC, TPU, and WebSocket endpoints.
#
# AUTHOR: Fixed Ratio Trading Development Team
# VERSION: 1.0

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
EXTERNAL_IP="192.168.9.81"
DOMAIN="vmdevbox1.dcs1.cc"

# Solana Validator Ports
RPC_PORT=8899
WEBSOCKET_PORT=8900
GOSSIP_PORT=8003
FAUCET_PORT=9900
HTTPS_PORT=443
HTTP_PORT=80

echo -e "${BLUE}🔍 Solana Validator Firewall & Port Accessibility Check${NC}"
echo "========================================================"
echo -e "${CYAN}External IP: $EXTERNAL_IP${NC}"
echo -e "${CYAN}Domain: $DOMAIN${NC}"
echo ""

# Check if running as root
if [[ $EUID -eq 0 ]]; then
    echo -e "${RED}❌ Do not run this script as root${NC}"
    echo -e "${YELLOW}💡 Run as regular user, script will use sudo when needed${NC}"
    exit 1
fi

echo -e "${YELLOW}📊 Current Network Configuration${NC}"
echo "=================================="

# Check current port bindings
echo -e "${CYAN}🔗 Solana Validator Port Bindings:${NC}"
netstat -tulpn 2>/dev/null | grep solana | head -10 | while read line; do
    echo "  $line"
done

echo ""

# Check firewall status
echo -e "${YELLOW}🔥 Firewall Configuration${NC}"
echo "========================="

# Check iptables rules
echo -e "${CYAN}📋 Current iptables rules:${NC}"
IPTABLES_POLICY=$(sudo iptables -L INPUT | head -1 | grep -o "policy [A-Z]*" | cut -d' ' -f2)
echo -e "  INPUT policy: ${GREEN}$IPTABLES_POLICY${NC}"

if [ "$IPTABLES_POLICY" = "ACCEPT" ]; then
    echo -e "${GREEN}✅ Firewall allows all incoming connections (good for validator)${NC}"
else
    echo -e "${YELLOW}⚠️  Firewall has restrictive policy, checking specific rules...${NC}"
    
    # Check for specific port rules
    RULES_COUNT=$(sudo iptables -L INPUT -n | grep -E "(8899|8900|8003|9900|443|80)" | wc -l)
    if [ "$RULES_COUNT" -gt 0 ]; then
        echo -e "${GREEN}✅ Found specific rules for Solana ports${NC}"
        sudo iptables -L INPUT -n | grep -E "(8899|8900|8003|9900|443|80)" | while read rule; do
            echo "    $rule"
        done
    else
        echo -e "${RED}❌ No specific rules found for Solana ports${NC}"
    fi
fi

echo ""

# Test port accessibility
echo -e "${YELLOW}🧪 Port Accessibility Tests${NC}"
echo "============================"

# Function to test port accessibility
test_port() {
    local port=$1
    local protocol=$2
    local description=$3
    
    echo -e "${CYAN}Testing $description (port $port/$protocol):${NC}"
    
    # Check if port is listening
    if netstat -tulpn 2>/dev/null | grep -q ":$port "; then
        echo -e "  ${GREEN}✅ Port $port is listening${NC}"
        
        # Test local connectivity
        if [ "$protocol" = "tcp" ]; then
            if timeout 3 bash -c "</dev/tcp/localhost/$port" 2>/dev/null; then
                echo -e "  ${GREEN}✅ Local TCP connection successful${NC}"
            else
                echo -e "  ${RED}❌ Local TCP connection failed${NC}"
            fi
        fi
        
        # Test external connectivity (if not localhost)
        if [ "$EXTERNAL_IP" != "127.0.0.1" ] && [ "$protocol" = "tcp" ]; then
            if timeout 3 bash -c "</dev/tcp/$EXTERNAL_IP/$port" 2>/dev/null; then
                echo -e "  ${GREEN}✅ External TCP connection successful${NC}"
            else
                echo -e "  ${YELLOW}⚠️  External TCP connection failed (may be network/firewall)${NC}"
            fi
        fi
    else
        echo -e "  ${RED}❌ Port $port is not listening${NC}"
    fi
    echo ""
}

# Test all Solana ports
test_port $RPC_PORT "tcp" "RPC (HTTP)"
test_port $WEBSOCKET_PORT "tcp" "WebSocket"
test_port $GOSSIP_PORT "tcp" "Gossip"
test_port $FAUCET_PORT "tcp" "Faucet"
test_port $HTTPS_PORT "tcp" "HTTPS (nginx)"
test_port $HTTP_PORT "tcp" "HTTP (nginx)"

# Test TPU port (dynamic, check from validator info)
echo -e "${CYAN}Testing TPU port (dynamic):${NC}"
TPU_PORT=$(curl -s http://localhost:$RPC_PORT -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getClusterNodes"}' 2>/dev/null | jq -r '.result[0].tpu' 2>/dev/null | cut -d':' -f2)

if [ "$TPU_PORT" != "null" ] && [ -n "$TPU_PORT" ]; then
    echo -e "  ${GREEN}✅ TPU port detected: $TPU_PORT${NC}"
    if netstat -tulpn 2>/dev/null | grep -q ":$TPU_PORT "; then
        echo -e "  ${GREEN}✅ TPU port $TPU_PORT is listening (UDP)${NC}"
    else
        echo -e "  ${RED}❌ TPU port $TPU_PORT is not listening${NC}"
    fi
else
    echo -e "  ${YELLOW}⚠️  Could not detect TPU port (validator may not be running)${NC}"
fi

echo ""

# Test HTTPS endpoints
echo -e "${YELLOW}🌐 HTTPS Endpoint Tests${NC}"
echo "======================="

# Test domain resolution
echo -e "${CYAN}Testing DNS resolution:${NC}"
if nslookup $DOMAIN >/dev/null 2>&1; then
    RESOLVED_IP=$(nslookup $DOMAIN | grep "Address:" | tail -1 | awk '{print $2}')
    if [ "$RESOLVED_IP" = "$EXTERNAL_IP" ]; then
        echo -e "  ${GREEN}✅ DNS resolves correctly: $DOMAIN → $EXTERNAL_IP${NC}"
    else
        echo -e "  ${YELLOW}⚠️  DNS resolves to different IP: $DOMAIN → $RESOLVED_IP (expected: $EXTERNAL_IP)${NC}"
    fi
else
    echo -e "  ${RED}❌ DNS resolution failed for $DOMAIN${NC}"
fi

# Test HTTPS connectivity
echo -e "${CYAN}Testing HTTPS connectivity:${NC}"

# Test with certificate validation
if curl -s --max-time 5 https://$DOMAIN -X POST -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' >/dev/null 2>&1; then
    echo -e "  ${GREEN}✅ HTTPS connection with certificate validation successful${NC}"
    CERT_VALID=true
else
    echo -e "  ${YELLOW}⚠️  HTTPS connection with certificate validation failed${NC}"
    CERT_VALID=false
fi

# Test without certificate validation
if curl -k -s --max-time 5 https://$DOMAIN -X POST -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' >/dev/null 2>&1; then
    echo -e "  ${GREEN}✅ HTTPS connection without certificate validation successful${NC}"
    if [ "$CERT_VALID" = "false" ]; then
        echo -e "  ${YELLOW}💡 Certificate validation issue detected${NC}"
    fi
else
    echo -e "  ${RED}❌ HTTPS connection failed completely${NC}"
fi

# Test direct IP HTTPS
echo -e "${CYAN}Testing direct IP HTTPS:${NC}"
if curl -k -s --max-time 5 https://$EXTERNAL_IP -X POST -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' >/dev/null 2>&1; then
    echo -e "  ${GREEN}✅ Direct IP HTTPS connection successful${NC}"
else
    echo -e "  ${RED}❌ Direct IP HTTPS connection failed${NC}"
fi

echo ""

# Check certificate details
echo -e "${YELLOW}🔐 SSL Certificate Analysis${NC}"
echo "==========================="

CERT_PATH="../dcs1/dcs1.crt"
if [ -f "$CERT_PATH" ]; then
    echo -e "${CYAN}Certificate information:${NC}"
    
    # Get certificate details
    CERT_SUBJECT=$(openssl x509 -in "$CERT_PATH" -subject -noout | cut -d'=' -f2-)
    CERT_EXPIRY=$(openssl x509 -in "$CERT_PATH" -enddate -noout | cut -d'=' -f2)
    CERT_SAN=$(openssl x509 -in "$CERT_PATH" -text -noout | grep -A1 "Subject Alternative Name" | tail -1 | sed 's/^[[:space:]]*//')
    
    echo -e "  Subject: ${GREEN}$CERT_SUBJECT${NC}"
    echo -e "  Expires: ${GREEN}$CERT_EXPIRY${NC}"
    echo -e "  SAN: ${GREEN}$CERT_SAN${NC}"
    
    # Check if certificate covers the domain
    if echo "$CERT_SAN" | grep -q "$DOMAIN\|*.dcs1.cc"; then
        echo -e "  ${GREEN}✅ Certificate covers domain $DOMAIN${NC}"
    else
        echo -e "  ${RED}❌ Certificate does not cover domain $DOMAIN${NC}"
    fi
else
    echo -e "  ${RED}❌ Certificate file not found: $CERT_PATH${NC}"
fi

echo ""

# Recommendations
echo -e "${YELLOW}💡 Recommendations${NC}"
echo "=================="

if [ "$IPTABLES_POLICY" != "ACCEPT" ]; then
    echo -e "${CYAN}🔥 Firewall Configuration:${NC}"
    echo "  Consider opening specific ports if needed:"
    echo "  sudo iptables -A INPUT -p tcp --dport $RPC_PORT -j ACCEPT"
    echo "  sudo iptables -A INPUT -p tcp --dport $WEBSOCKET_PORT -j ACCEPT"
    echo "  sudo iptables -A INPUT -p tcp --dport $GOSSIP_PORT -j ACCEPT"
    echo "  sudo iptables -A INPUT -p tcp --dport $HTTPS_PORT -j ACCEPT"
    echo "  sudo iptables -A INPUT -p udp --dport 1024:65535 -j ACCEPT  # TPU range"
    echo ""
fi

if [ "$CERT_VALID" = "false" ]; then
    echo -e "${CYAN}🔐 Certificate Issues:${NC}"
    echo "  • Certificate validation is failing"
    echo "  • This may be due to:"
    echo "    - Self-signed certificate"
    echo "    - Hostname mismatch"
    echo "    - Certificate chain issues"
    echo "  • For testing, use: curl -k https://$DOMAIN"
    echo "  • For production, ensure proper certificate installation"
    echo ""
fi

echo -e "${CYAN}🌐 Client Configuration:${NC}"
echo "  Use these endpoints for external access:"
echo "  • HTTPS RPC: https://$DOMAIN"
echo "  • WebSocket: wss://$DOMAIN/ws"
echo "  • TPU: $EXTERNAL_IP:$TPU_PORT (UDP, no SSL)"
echo ""

echo -e "${CYAN}🧪 Testing Commands:${NC}"
echo "  # Test RPC health"
echo "  curl -k https://$DOMAIN -X POST -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getHealth\"}'"
echo ""
echo "  # Check TPU endpoints"
echo "  curl -s http://localhost:$RPC_PORT -X POST -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getClusterNodes\"}' | jq '.result[] | {rpc: .rpc, tpu: .tpu, gossip: .gossip}'"
echo ""

echo -e "${GREEN}🎉 Firewall and port accessibility check complete!${NC}"
echo -e "${BLUE}📋 See recommendations above for any required actions.${NC}" 