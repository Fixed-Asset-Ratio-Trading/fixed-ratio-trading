#!/bin/bash
# Setup Backpack Wallet Keypair for Local Testing
# This script creates a keypair file from the provided private key

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "🎒 Backpack Wallet Keypair Setup"
echo "================================"

# Backpack wallet details
BACKPACK_ADDRESS="5GGZiMwU56rYL1L52q7Jz7ELkSN4iYyQqdv418hxPh6t"
BACKPACK_PRIVATE_KEY="26uwjawj1t3SQz1NzgQZ4TEQyBUdsH7xVLXLpaf4zXU9bqe9Gx1i18YY2d58RrGgo3WZesWqN3d6WZXD4wBH617r"

# Find the project root directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
KEYPAIR_DIR="$PROJECT_ROOT/.keypairs"
BACKPACK_KEYPAIR_PATH="$KEYPAIR_DIR/backpack-keypair.json"

echo "📂 Project Root: $PROJECT_ROOT"
echo "🎒 Backpack Address: $BACKPACK_ADDRESS"
echo "📁 Keypair Directory: $KEYPAIR_DIR"

# Create keypair directory if it doesn't exist
if [ ! -d "$KEYPAIR_DIR" ]; then
    echo -e "${YELLOW}📁 Creating keypair directory...${NC}"
    mkdir -p "$KEYPAIR_DIR"
fi

# Check if solana CLI is available
if ! command -v solana >/dev/null 2>&1; then
    echo -e "${RED}❌ Error: solana CLI not found${NC}"
    echo "   Please install Solana CLI tools first"
    exit 1
fi

# Create temporary file with private key
echo -e "${YELLOW}🔑 Creating Backpack keypair file...${NC}"

# Create the keypair file from the private key
# Note: This creates a keypair file in the standard Solana format
echo "[$BACKPACK_PRIVATE_KEY]" | base58 -d > /tmp/backpack_key_bytes 2>/dev/null || {
    # If base58 is not available, use Python to decode
    python3 -c "
import base58
import sys
private_key = '$BACKPACK_PRIVATE_KEY'
decoded = base58.b58decode(private_key)
sys.stdout.buffer.write(decoded)
" > /tmp/backpack_key_bytes 2>/dev/null || {
        echo -e "${RED}❌ Error: Could not decode private key${NC}"
        echo "   Please ensure base58 or Python with base58 library is available"
        exit 1
    }
}

# Convert to JSON format expected by Solana
python3 -c "
import json
with open('/tmp/backpack_key_bytes', 'rb') as f:
    key_bytes = f.read()
key_array = list(key_bytes)
with open('$BACKPACK_KEYPAIR_PATH', 'w') as f:
    json.dump(key_array, f)
" || {
    echo -e "${RED}❌ Error: Could not create keypair JSON file${NC}"
    exit 1
}

# Clean up temporary file
rm -f /tmp/backpack_key_bytes

# Verify the keypair
echo -e "${YELLOW}✅ Verifying keypair...${NC}"
GENERATED_ADDRESS=$(solana-keygen pubkey "$BACKPACK_KEYPAIR_PATH")

if [ "$GENERATED_ADDRESS" = "$BACKPACK_ADDRESS" ]; then
    echo -e "${GREEN}✅ Success! Keypair created and verified${NC}"
    echo "   Address: $GENERATED_ADDRESS"
    echo "   Keypair file: $BACKPACK_KEYPAIR_PATH"
else
    echo -e "${RED}❌ Error: Generated address doesn't match expected address${NC}"
    echo "   Expected: $BACKPACK_ADDRESS"
    echo "   Generated: $GENERATED_ADDRESS"
    exit 1
fi

echo ""
echo -e "${BLUE}📋 Summary:${NC}"
echo "  ✅ Backpack keypair created successfully"
echo "  📁 Location: $BACKPACK_KEYPAIR_PATH"
echo "  🎒 Address: $BACKPACK_ADDRESS"
echo ""
echo -e "${GREEN}🎉 Setup complete! You can now use this keypair with Solana CLI${NC}"
echo "   Example: solana balance $BACKPACK_ADDRESS --keypair $BACKPACK_KEYPAIR_PATH" 