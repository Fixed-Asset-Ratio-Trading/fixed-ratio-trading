#!/bin/bash
# Build BPF script - Workaround for cargo build-bpf
# This script provides BPF build functionality using existing Solana tools

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔨 Building Solana BPF Program${NC}"
echo "======================================"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}❌ Error: Cargo.toml not found. Please run this script from the project root.${NC}"
    exit 1
fi

# Create deploy directory if it doesn't exist
mkdir -p target/deploy

echo -e "${YELLOW}📦 Building with cdylib target for Solana...${NC}"

# Build the program as a cdylib (which is what Solana programs need)
cargo build --release --lib --target x86_64-unknown-linux-gnu

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Build successful${NC}"
    
    # Copy the built library to the deploy directory
    if [ -f "target/release/libfixed_ratio_trading.so" ]; then
        cp target/release/libfixed_ratio_trading.so target/deploy/fixed_ratio_trading.so
        echo -e "${GREEN}✅ Program copied to target/deploy/fixed_ratio_trading.so${NC}"
        
        # Show file info
        echo -e "${BLUE}📊 Build Information:${NC}"
        echo "  Program Size: $(ls -lh target/deploy/fixed_ratio_trading.so | awk '{print $5}')"
        echo "  Build Location: target/deploy/fixed_ratio_trading.so"
        echo "  Build Time: $(date)"
        
    else
        echo -e "${RED}❌ Error: Built library not found${NC}"
        exit 1
    fi
else
    echo -e "${RED}❌ Build failed${NC}"
    exit 1
fi

echo -e "${GREEN}🎉 BPF Build Complete!${NC}"
echo -e "${BLUE}   You can now deploy using: solana program deploy target/deploy/fixed_ratio_trading.so${NC}" 