#!/bin/bash
# Build BPF script - Latest Solana 2.2.x with proper BPF tools
# Install latest Solana toolchain with enhanced BPF compatibility

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔨 Solana 2.2.x BPF Tools Setup and Build${NC}"
echo "========================================"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}❌ Error: Cargo.toml not found. Please run this script from the project root.${NC}"
    exit 1
fi

# Create deploy directory
mkdir -p target/deploy

echo -e "${YELLOW}🔍 Checking current Solana installation...${NC}"

# Check current Solana installation
if command -v solana &> /dev/null; then
    SOLANA_VERSION=$(solana --version 2>/dev/null | head -1)
    echo "  Current Solana: $SOLANA_VERSION"
else
    echo "  No Solana found"
fi

# Always install/update to latest Solana 2.2.x for best BPF compatibility
echo -e "${YELLOW}🚀 Installing/updating to latest Solana 2.2.x...${NC}"

# Remove any old installation
echo "  Cleaning any existing Solana installation..."
rm -rf ~/.local/share/solana 2>/dev/null || true

# Install latest stable Solana (2.2.x)
echo "  Installing latest stable Solana toolchain..."
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"

# Add to PATH for this session
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"

# Update shell profile for future sessions
if [ -f "$HOME/.bashrc" ]; then
    echo 'export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"' >> ~/.bashrc
fi
if [ -f "$HOME/.zshrc" ]; then
    echo 'export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"' >> ~/.zshrc
fi

echo "  ✅ Latest Solana toolchain installed"

# Verify installation
echo "  Verifying installation..."
solana --version

# Check for build-sbf (should be available in 2.2.x)
echo -e "${YELLOW}🔍 Checking BPF build tools...${NC}"
if cargo build-sbf --help &> /dev/null; then
    echo "  ✅ cargo build-sbf: Available and working"
    BUILD_TOOL="cargo build-sbf"
elif cargo build-bpf --help &> /dev/null; then
    echo "  ✅ cargo build-bpf: Available and working"
    BUILD_TOOL="cargo build-bpf"
else
    echo -e "${RED}❌ No BPF build tools available${NC}"
    echo "  This is unexpected for Solana 2.2.x"
    exit 1
fi

# Now build using proper BPF tools
echo -e "${YELLOW}🔨 Building with $BUILD_TOOL...${NC}"

if [ "$BUILD_TOOL" = "cargo build-sbf" ]; then
    echo "  Using modern SBF build system..."
    cargo build-sbf --manifest-path Cargo.toml --sbf-out-dir target/deploy
    BUILD_SUCCESS=$?
else
    echo "  Using legacy BPF build system..."
    cargo build-bpf --manifest-path Cargo.toml --bpf-out-dir target/deploy
    BUILD_SUCCESS=$?
fi

if [ $BUILD_SUCCESS -eq 0 ]; then
    echo -e "${GREEN}✅ Build successful with $BUILD_TOOL!${NC}"
    
    # Check the output
    if [ -f "target/deploy/fixed_ratio_trading.so" ]; then
        echo -e "${GREEN}✅ Program built successfully${NC}"
        
        echo -e "${BLUE}📊 Build Information:${NC}"
        echo "  Program Size: $(ls -lh target/deploy/fixed_ratio_trading.so | awk '{print $5}')"
        echo "  Build Location: target/deploy/fixed_ratio_trading.so"
        echo "  Build Tool: $BUILD_TOOL"
        echo "  Solana Version: $(solana --version | head -1)"
        echo "  Build Time: $(date)"
        
        # Verify it's a proper BPF program
        if command -v file &> /dev/null; then
            echo "  File type: $(file target/deploy/fixed_ratio_trading.so)"
        fi
        
    else
        echo -e "${RED}❌ Build succeeded but output file not found${NC}"
        echo "  Expected: target/deploy/fixed_ratio_trading.so"
        echo "  Available files:"
        find target -name "*.so" -type f 2>/dev/null || echo "  No .so files found"
        exit 1
    fi
else
    echo -e "${RED}❌ Build failed with $BUILD_TOOL${NC}"
    exit 1
fi

echo -e "${GREEN}🎉 Solana 2.2.x BPF Build Complete!${NC}"
echo -e "${BLUE}   This should work perfectly with: solana program deploy target/deploy/fixed_ratio_trading.so${NC}" 