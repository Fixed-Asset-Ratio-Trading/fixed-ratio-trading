 #!/bin/bash
# Deploy Fixed Ratio Trading Contract to Remote Solana Validator
# This script builds the contract and deploys/upgrades the program to the remote validator
# Targets the direct validator endpoint at http://192.168.2.88:8899
#
# Usage:
#   ./remote_build_and_deploy.sh [--reset|--noreset]
#
# Options:
#   --reset     Reset the validator before deployment
#   --noreset   Keep existing validator state (default behavior)
#   (no option) Keep existing validator state (default behavior)

set -e

# Parse command line arguments
VALIDATOR_RESET_OPTION="no_reset"  # Default to no reset (changed from auto_reset)
for arg in "$@"; do
    case $arg in
        --reset)
            VALIDATOR_RESET_OPTION="auto_reset"
            ;;
        --noreset)
            VALIDATOR_RESET_OPTION="no_reset"
            ;;
        *)
            echo "Unknown option: $arg"
            echo "Usage: $0 [--reset|--noreset]"
            exit 1
            ;;
    esac
done

# Find the project root directory (where Cargo.toml is located)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Verify we found the correct project directory
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    echo "❌ Error: Could not find Cargo.toml in project root: $PROJECT_ROOT"
    echo "   Please run this script from the fixed-ratio-trading project directory or its subdirectories"
    exit 1
fi

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "🚀 Fixed Ratio Trading - Remote Deployment Script"
echo "=================================================="
echo "📂 Project Root: $PROJECT_ROOT"
echo ""
echo -e "${BLUE}🌐 Targeting Direct Validator Endpoint: http://192.168.2.88:8899${NC}"
echo -e "${BLUE}🎒 Backpack Address: 5GGZiMwU56rYL1L52q7Jz7ELkSN4iYyQqdv418hxPh6t${NC}"
echo -e "${BLUE}   Run './scripts/setup_backpack_keypair.sh' first if you need the keypair file${NC}"

# Check for required tools
echo -e "${YELLOW}🔧 Checking required tools...${NC}"
MISSING_TOOLS=""
command -v solana >/dev/null 2>&1 || MISSING_TOOLS="$MISSING_TOOLS solana"
command -v solana-keygen >/dev/null 2>&1 || MISSING_TOOLS="$MISSING_TOOLS solana-keygen"
command -v jq >/dev/null 2>&1 || echo "  Warning: jq not found (JSON parsing will be limited)"
command -v curl >/dev/null 2>&1 || echo "  Warning: curl not found (endpoint testing will be limited)"

if [ -n "$MISSING_TOOLS" ]; then
    echo -e "${RED}❌ Missing required tools:$MISSING_TOOLS${NC}"
    echo "   Please install the Solana CLI tools first"
    exit 1
fi
echo -e "${GREEN}✅ All required tools found${NC}"

# Load shared configuration if available
SHARED_CONFIG="$PROJECT_ROOT/shared-config.json"
if [ -f "$SHARED_CONFIG" ] && command -v jq >/dev/null 2>&1; then
    echo -e "${BLUE}📋 Loading shared configuration...${NC}"
    RPC_URL=$(jq -r '.solana.rpcUrl' "$SHARED_CONFIG" 2>/dev/null || echo "http://192.168.2.88:8899")
    BACKPACK_WALLET=$(jq -r '.wallets.expectedBackpackWallet' "$SHARED_CONFIG" 2>/dev/null || echo "5GGZiMwU56rYL1L52q7Jz7ELkSN4iYyQqdv418hxPh6t")
    echo -e "${GREEN}✅ Configuration loaded from shared-config.json${NC}"
else
    echo -e "${YELLOW}⚠️ Using fallback configuration (shared-config.json not found or jq not available)${NC}"
    RPC_URL="http://192.168.2.88:8899"
    BACKPACK_WALLET="5GGZiMwU56rYL1L52q7Jz7ELkSN4iYyQqdv418hxPh6t"
fi

# Configuration - Ensure we use the correct program ID keypair
EXPECTED_PROGRAM_ID="4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn"
PROGRAM_KEYPAIR="$PROJECT_ROOT/target/deploy/fixed_ratio_trading-keypair.json"
BACKUP_KEYPAIR="$PROJECT_ROOT/target/deploy/LocalNet-4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn.json"
TEMP_KEYPAIR="$PROJECT_ROOT/temp/LocalNet-4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn.json"

# Function to restore correct keypair from backup
restore_correct_keypair() {
    echo -e "${YELLOW}🔧 Attempting to restore correct program ID keypair...${NC}"
    
    # Priority 1: Check temp directory first (most reliable backup)
    if [ -f "$TEMP_KEYPAIR" ]; then
        TEMP_PROGRAM_ID=$(solana-keygen pubkey "$TEMP_KEYPAIR")
        if [ "$TEMP_PROGRAM_ID" = "$EXPECTED_PROGRAM_ID" ]; then
            echo -e "${GREEN}✅ Found correct temp keypair, restoring...${NC}"
            cp "$TEMP_KEYPAIR" "$PROGRAM_KEYPAIR"
            echo -e "${GREEN}✅ Keypair restored from temp directory${NC}"
            return 0
        else
            echo -e "${YELLOW}⚠️  Temp keypair has wrong program ID: $TEMP_PROGRAM_ID${NC}"
        fi
    fi
    
    # Priority 2: Check if backup exists in target/deploy/
    if [ -f "$BACKUP_KEYPAIR" ]; then
        BACKUP_PROGRAM_ID=$(solana-keygen pubkey "$BACKUP_KEYPAIR")
        if [ "$BACKUP_PROGRAM_ID" = "$EXPECTED_PROGRAM_ID" ]; then
            echo -e "${GREEN}✅ Found correct backup keypair, restoring...${NC}"
            cp "$BACKUP_KEYPAIR" "$PROGRAM_KEYPAIR"
            echo -e "${GREEN}✅ Keypair restored successfully${NC}"
            return 0
        else
            echo -e "${YELLOW}⚠️  Backup keypair has wrong program ID: $BACKUP_PROGRAM_ID${NC}"
        fi
    fi
    
    # Priority 3: Check if backup exists in /Users/davinci/code/keys/
    EXTERNAL_BACKUP="/Users/davinci/code/keys/LocalNet-4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn.json"
    if [ -f "$EXTERNAL_BACKUP" ]; then
        EXTERNAL_PROGRAM_ID=$(solana-keygen pubkey "$EXTERNAL_BACKUP")
        if [ "$EXTERNAL_PROGRAM_ID" = "$EXPECTED_PROGRAM_ID" ]; then
            echo -e "${GREEN}✅ Found correct external backup keypair, restoring...${NC}"
            cp "$EXTERNAL_BACKUP" "$PROGRAM_KEYPAIR"
            echo -e "${GREEN}✅ Keypair restored from external backup${NC}"
            return 0
        else
            echo -e "${YELLOW}⚠️  External backup keypair has wrong program ID: $EXTERNAL_PROGRAM_ID${NC}"
        fi
    fi
    
    echo -e "${RED}❌ Cannot restore correct keypair automatically${NC}"
    echo "  No valid backup found with program ID: $EXPECTED_PROGRAM_ID"
    return 1
}

# Ensure the keypair file exists and matches the expected program ID
if [ -f "$PROGRAM_KEYPAIR" ]; then
    CURRENT_PROGRAM_ID=$(solana-keygen pubkey "$PROGRAM_KEYPAIR")
    if [ "$CURRENT_PROGRAM_ID" = "$EXPECTED_PROGRAM_ID" ]; then
        PROGRAM_ID="$CURRENT_PROGRAM_ID"
        echo -e "${GREEN}✅ Using correct program ID keypair: $PROGRAM_ID${NC}"
        
        # Verify it matches the temp file if temp file exists
        if [ -f "$TEMP_KEYPAIR" ]; then
            TEMP_PROGRAM_ID=$(solana-keygen pubkey "$TEMP_KEYPAIR")
            if [ "$TEMP_PROGRAM_ID" != "$CURRENT_PROGRAM_ID" ]; then
                echo -e "${YELLOW}⚠️  Target keypair doesn't match temp keypair!${NC}"
                echo "  Target keypair ID: $CURRENT_PROGRAM_ID"
                echo "  Temp keypair ID: $TEMP_PROGRAM_ID"
                echo -e "${YELLOW}🔧 Overwriting target with correct temp keypair...${NC}"
                cp "$TEMP_KEYPAIR" "$PROGRAM_KEYPAIR"
                echo -e "${GREEN}✅ Target keypair synchronized with temp keypair${NC}"
            else
                echo -e "${GREEN}✅ Target and temp keypairs are synchronized${NC}"
            fi
        fi
        
        # Create backup to preserve the correct keypair
        if [ ! -f "$BACKUP_KEYPAIR" ]; then
            cp "$PROGRAM_KEYPAIR" "$BACKUP_KEYPAIR"
            echo -e "${BLUE}📋 Created backup of correct keypair${NC}"
        fi
    else
        echo -e "${YELLOW}⚠️  Program ID mismatch detected!${NC}"
        echo "  Current keypair ID: $CURRENT_PROGRAM_ID"
        echo "  Expected ID: $EXPECTED_PROGRAM_ID"
        
        # Backup the incorrect keypair
        mv "$PROGRAM_KEYPAIR" "$PROGRAM_KEYPAIR.backup.$(date +%s)"
        
        # Try to restore correct keypair
        if restore_correct_keypair; then
            PROGRAM_ID="$EXPECTED_PROGRAM_ID"
            echo -e "${GREEN}✅ Successfully restored correct program ID keypair${NC}"
        else
            echo -e "${RED}❌ DEPLOYMENT FAILED: Cannot restore correct keypair${NC}"
            echo ""
            echo -e "${RED}🚨 CRITICAL ERROR: Required program ID keypair not found${NC}"
            echo "  Expected program ID: $EXPECTED_PROGRAM_ID"
            echo ""
            echo -e "${YELLOW}📋 REQUIRED ACTION: Restore the correct keypair file${NC}"
            echo "  The script requires temp/LocalNet-4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn.json"
            echo "  This file must contain the keypair that generates program ID: $EXPECTED_PROGRAM_ID"
            echo ""
            echo -e "${BLUE}🔍 Checked these locations (none found or wrong program ID):${NC}"
            echo "    1. $TEMP_KEYPAIR (PRIORITY - required for deployment)"
            echo "    2. $BACKUP_KEYPAIR"
            echo "    3. /Users/davinci/code/keys/LocalNet-4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn.json"
            echo ""
            echo -e "${RED}❌ DEPLOYMENT CANNOT PROCEED WITHOUT CORRECT KEYPAIR${NC}"
            exit 1
        fi
    fi
else
    echo -e "${YELLOW}⚠️  Program keypair not found, attempting to restore...${NC}"
    
    # Try to restore from backup
    if restore_correct_keypair; then
        PROGRAM_ID="$EXPECTED_PROGRAM_ID"
        echo -e "${GREEN}✅ Successfully restored program ID keypair${NC}"
    else
        echo -e "${RED}❌ DEPLOYMENT FAILED: Cannot create or restore keypair${NC}"
        echo ""
        echo -e "${RED}🚨 CRITICAL ERROR: Program keypair file missing${NC}"
        echo "  Target file: $PROGRAM_KEYPAIR"
        echo "  Expected program ID: $EXPECTED_PROGRAM_ID"
        echo ""
        echo -e "${YELLOW}📋 REQUIRED ACTION: Provide the correct keypair file${NC}"
        echo "  The script requires temp/LocalNet-4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn.json"
        echo "  This file must contain the keypair that generates program ID: $EXPECTED_PROGRAM_ID"
        echo ""
        echo -e "${BLUE}🔍 Checked these locations (none found):${NC}"
        echo "    1. $TEMP_KEYPAIR (PRIORITY - required for deployment)"
        echo "    2. $BACKUP_KEYPAIR"
        echo "    3. /Users/davinci/code/keys/LocalNet-4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn.json"
        echo ""
        echo -e "${RED}❌ DEPLOYMENT CANNOT PROCEED WITHOUT CORRECT KEYPAIR${NC}"
        exit 1
    fi
fi
KEYPAIR_PATH="$HOME/.config/solana/id.json"

echo -e "${BLUE}📋 Configuration:${NC}"
echo "  Program ID: $PROGRAM_ID"
echo "  Remote RPC URL: $RPC_URL"
echo "  Keypair: $KEYPAIR_PATH"
echo "  Backpack Wallet: $BACKPACK_WALLET"
echo ""

# Step 1: Run all tests before deployment
echo -e "${YELLOW}🧪 Running comprehensive test suite...${NC}"
echo "   This ensures code quality before deployment"
cd "$PROJECT_ROOT"

echo "   Running cargo tests..."
if ! cargo test --lib; then
    echo -e "${RED}❌ Unit tests failed! Deployment aborted.${NC}"
    echo "   Please fix failing tests before deploying"
    exit 1
fi

echo "   Running integration tests..."
if ! cargo test --test '*'; then
    echo -e "${RED}❌ Integration tests failed! Deployment aborted.${NC}"
    echo "   Please fix failing tests before deploying"
    exit 1
fi

echo -e "${GREEN}✅ All tests passed successfully${NC}"
echo ""

# Step 2: Determine validator reset action
VALIDATOR_RESET=false

if [ "$VALIDATOR_RESET_OPTION" = "auto_reset" ]; then
    VALIDATOR_RESET=true
    echo -e "${YELLOW}🔄 Resetting validator (--reset specified)${NC}"
else
    # Default no reset
    VALIDATOR_RESET=false
    echo -e "${BLUE}🔄 Keeping existing validator state (default behavior)${NC}"
fi

if [ "$VALIDATOR_RESET" = true ]; then
    echo -e "${YELLOW}🔄 Resetting remote validator...${NC}"
    
    # Check if SSH is available
    if ! command -v ssh >/dev/null 2>&1; then
        echo -e "${RED}❌ SSH not found. Cannot reset remote validator.${NC}"
        exit 1
    fi
    
    echo "   Connecting to dev@vmdevbox1..."
    echo "   Starting fresh validator (script will handle stopping previous one)..."
    
    # Start fresh validator and show output
    echo "   Running: cd ~/code/fixed-ratio-trading && ./scripts/remote_server_only/start_production_validator.sh --reset"
    if ssh dev@vmdevbox1 'cd ~/code/fixed-ratio-trading && ./scripts/remote_server_only/start_production_validator.sh --reset'; then
        echo -e "${GREEN}✅ Validator start script completed${NC}"
        
        # Verify validator is actually running by testing connectivity
        echo "   Verifying validator is responding..."
        VALIDATOR_CHECK_COUNT=0
        MAX_VALIDATOR_CHECKS=10
        
        while [ $VALIDATOR_CHECK_COUNT -lt $MAX_VALIDATOR_CHECKS ]; do
            if curl -s --connect-timeout 5 -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' "$RPC_URL" | grep -q "ok"; then
                echo -e "${GREEN}✅ Validator is running and responding${NC}"
                
                # Get some basic validator info to confirm it's working
                echo "   Getting validator status..."
                SLOT_INFO=$(curl -s -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getSlot"}' "$RPC_URL" 2>/dev/null)
                if echo "$SLOT_INFO" | grep -q '"result"'; then
                    CURRENT_SLOT=$(echo "$SLOT_INFO" | grep -o '"result":[0-9]*' | cut -d':' -f2)
                    echo -e "${GREEN}   Current slot: $CURRENT_SLOT${NC}"
                fi
                
                # Check if we can get account balance (basic functionality test)
                BALANCE_CHECK=$(solana balance $BACKPACK_WALLET 2>/dev/null | head -1)
                if [ $? -eq 0 ]; then
                    echo -e "${GREEN}   Balance check successful: $BALANCE_CHECK${NC}"
                else
                    echo -e "${YELLOW}   Balance check failed, but validator is responding${NC}"
                fi
                break
            else
                VALIDATOR_CHECK_COUNT=$((VALIDATOR_CHECK_COUNT + 1))
                if [ $VALIDATOR_CHECK_COUNT -lt $MAX_VALIDATOR_CHECKS ]; then
                    echo "   Validator check $VALIDATOR_CHECK_COUNT/$MAX_VALIDATOR_CHECKS - waiting..."
                    sleep 2
                else
                    echo -e "${RED}❌ Validator not responding after $MAX_VALIDATOR_CHECKS attempts${NC}"
                    echo "   The start script completed but validator may not be ready yet"
                    echo "   You may need to wait a bit longer or check vmdevbox1 manually"
                    exit 1
                fi
            fi
        done
    else
        echo -e "${RED}❌ Failed to start fresh validator${NC}"
        echo "   You may need to manually start the validator on vmdevbox1"
        exit 1
    fi
    
    echo -e "${GREEN}✅ Remote validator reset completed${NC}"
    
    # Step 2.5: Update Metaplex configuration after reset
    echo -e "${YELLOW}🎨 Updating Metaplex configuration after reset...${NC}"
    echo "   Forcing canonical Metaplex program IDs (no auto-discovery)"

    TOKEN_METADATA_PROGRAM_ID="metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
    CANDY_MACHINE_PROGRAM_ID="cndy3Z4yapfJBmL3ShUp5exZKqR3z33thTzeNMm2gRZ"
    AUCTION_HOUSE_PROGRAM_ID="hausS13jsjafwWwGqZTUQRmWyvyxn9EQpqMwV1PBBmk"

    echo "   Updating shared-config.json with canonical Metaplex program IDs..."
    echo "   Token Metadata Program: $TOKEN_METADATA_PROGRAM_ID"
    echo "   Candy Machine Program: $CANDY_MACHINE_PROGRAM_ID"
    echo "   Auction House Program: $AUCTION_HOUSE_PROGRAM_ID"

    if [ -f "$PROJECT_ROOT/shared-config.json" ] && command -v jq >/dev/null 2>&1; then
        TEMP_CONFIG=$(mktemp)
        jq --arg tokenMetadata "$TOKEN_METADATA_PROGRAM_ID" \
           --arg candyMachine "$CANDY_MACHINE_PROGRAM_ID" \
           --arg auctionHouse "$AUCTION_HOUSE_PROGRAM_ID" \
           --arg lastUpdated "$(date -u +%Y-%m-%d)" \
           '.metaplex.tokenMetadataProgramId = $tokenMetadata |
            .metaplex.candyMachineProgramId = $candyMachine |
            .metaplex.auctionHouseProgramId = $auctionHouse |
            .metaplex.lastUpdated = $lastUpdated |
            .metaplex.deploymentType = "remote" |
            .metaplex.remoteRpcUrl = "http://192.168.2.88:8899"' "$PROJECT_ROOT/shared-config.json" > "$TEMP_CONFIG"
        mv "$TEMP_CONFIG" "$PROJECT_ROOT/shared-config.json"
        echo -e "${GREEN}✅ Updated main shared-config.json file${NC}"
    else
        echo -e "${YELLOW}⚠️  Could not update shared-config.json (file not found or jq not available)${NC}"
        echo "   Please manually update Metaplex program IDs to canonical values."
    fi
    echo -e "${GREEN}✅ Metaplex configuration update completed${NC}"
    
else
    echo -e "${BLUE}ℹ️  Keeping existing validator state${NC}"
fi

echo ""

# Step 3: Test remote endpoint connectivity (skip if we just reset validator)
if [[ $VALIDATOR_RESET == false ]]; then
    echo -e "${YELLOW}🔍 Testing remote endpoint connectivity...${NC}"
    if command -v curl >/dev/null 2>&1; then
        # Test endpoint with retry logic
        RETRY_COUNT=0
        MAX_RETRIES=5
        
        while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
            if curl -s --connect-timeout 10 -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' "$RPC_URL" | grep -q "ok"; then
                echo -e "${GREEN}✅ Remote endpoint is responding correctly${NC}"
                break
            else
                RETRY_COUNT=$((RETRY_COUNT + 1))
                if [ $RETRY_COUNT -lt $MAX_RETRIES ]; then
                    echo "   Retry $RETRY_COUNT/$MAX_RETRIES - waiting for validator..."
                    sleep 3
                else
                    echo -e "${RED}❌ Remote endpoint is not responding after $MAX_RETRIES attempts${NC}"
                    echo "   Please ensure the remote validator is running at $RPC_URL"
                    exit 1
                fi
            fi
        done
    else
        echo -e "${YELLOW}⚠️  curl not found. Cannot test endpoint automatically${NC}"
    fi
else
    echo -e "${BLUE}ℹ️  Skipping connectivity test (validator was just reset and verified)${NC}"
fi

# Step 3.5: Ensure Metaplex programs are deployed for local testing
echo -e "${YELLOW}🎨 Checking Metaplex programs deployment...${NC}"
echo "   Verifying Token Metadata Program for full token functionality"

METAPLEX_SCRIPT="$PROJECT_ROOT/scripts/metaplex/manage_metaplex.sh"
if [ -f "$METAPLEX_SCRIPT" ]; then
    # Check if Metaplex programs are already deployed
    if ! "$METAPLEX_SCRIPT" status >/dev/null 2>&1; then
        echo "   Metaplex programs not found, deploying..."
        if "$METAPLEX_SCRIPT" start; then
            echo -e "${GREEN}✅ Metaplex programs deployed successfully${NC}"
        else
            echo -e "${YELLOW}⚠️  Metaplex deployment failed, proceeding without full metadata support${NC}"
            echo "   Note: Token creation may not include metadata on this deployment"
        fi
    else
        echo -e "${GREEN}✅ Metaplex programs already deployed${NC}"
    fi
else
    echo -e "${YELLOW}⚠️  Metaplex management script not found, skipping metadata setup${NC}"
fi

echo ""

# Step 4: Check if build creates new changes
echo -e "${YELLOW}🔍 Checking if app was modified...${NC}"

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep '^version = ' "$PROJECT_ROOT/Cargo.toml" | head -1 | sed 's/version = "\(.*\)"/\1/')
echo "  Current version: $CURRENT_VERSION"

# Get timestamp of current build artifact (if it exists)
BUILD_ARTIFACT="$PROJECT_ROOT/target/deploy/fixed_ratio_trading.so"
if [ -f "$BUILD_ARTIFACT" ]; then
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS stat format
        OLD_TIMESTAMP=$(stat -f %m "$BUILD_ARTIFACT" 2>/dev/null || echo "0")
    else
        # Linux stat format
        OLD_TIMESTAMP=$(stat -c %Y "$BUILD_ARTIFACT" 2>/dev/null || echo "0")
    fi
    echo "  Previous build timestamp: $OLD_TIMESTAMP"
else
    OLD_TIMESTAMP="0"
    echo "  No previous build found"
fi

# Step 5: Ensure correct keypair before build
echo -e "${YELLOW}🔧 Ensuring correct program keypair before build...${NC}"

# Critical: Ensure the correct keypair is in place BEFORE build
if [ ! -f "$PROGRAM_KEYPAIR" ]; then
    echo -e "${YELLOW}⚠️  Program keypair missing, restoring from temp...${NC}"
    if [ -f "$TEMP_KEYPAIR" ]; then
        TEMP_PROGRAM_ID=$(solana-keygen pubkey "$TEMP_KEYPAIR")
        if [ "$TEMP_PROGRAM_ID" = "$EXPECTED_PROGRAM_ID" ]; then
            echo -e "${GREEN}✅ Copying correct keypair from temp directory${NC}"
            mkdir -p "$(dirname "$PROGRAM_KEYPAIR")"
            cp "$TEMP_KEYPAIR" "$PROGRAM_KEYPAIR"
            echo -e "${GREEN}✅ Program keypair restored: $EXPECTED_PROGRAM_ID${NC}"
        else
            echo -e "${RED}❌ Temp keypair has wrong program ID: $TEMP_PROGRAM_ID${NC}"
            echo -e "${RED}❌ Expected: $EXPECTED_PROGRAM_ID${NC}"
            exit 1
        fi
    else
        echo -e "${RED}❌ CRITICAL: Temp keypair not found at $TEMP_KEYPAIR${NC}"
        echo -e "${RED}❌ Cannot proceed without correct program keypair${NC}"
        exit 1
    fi
else
    # Verify existing keypair is correct
    CURRENT_PROGRAM_ID=$(solana-keygen pubkey "$PROGRAM_KEYPAIR")
    if [ "$CURRENT_PROGRAM_ID" != "$EXPECTED_PROGRAM_ID" ]; then
        echo -e "${YELLOW}⚠️  Existing keypair has wrong program ID: $CURRENT_PROGRAM_ID${NC}"
        echo -e "${YELLOW}🔧 Replacing with correct keypair from temp...${NC}"
        if [ -f "$TEMP_KEYPAIR" ]; then
            cp "$TEMP_KEYPAIR" "$PROGRAM_KEYPAIR"
            echo -e "${GREEN}✅ Program keypair corrected: $EXPECTED_PROGRAM_ID${NC}"
        else
            echo -e "${RED}❌ CRITICAL: Temp keypair not found at $TEMP_KEYPAIR${NC}"
            exit 1
        fi
    else
        echo -e "${GREEN}✅ Program keypair is correct: $EXPECTED_PROGRAM_ID${NC}"
    fi
fi

# Step 5: Initial build to check for changes
echo -e "${YELLOW}🔨 Running initial build to detect changes...${NC}"
cd "$PROJECT_ROOT"
RUSTFLAGS="-C link-arg=-zstack-size=131072" cargo build-sbf || true
if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Initial build failed${NC}"
    exit 1
fi

# Check if build artifact timestamp changed
if [ -f "$BUILD_ARTIFACT" ]; then
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS stat format
        NEW_TIMESTAMP=$(stat -f %m "$BUILD_ARTIFACT" 2>/dev/null || echo "0")
    else
        # Linux stat format
        NEW_TIMESTAMP=$(stat -c %Y "$BUILD_ARTIFACT" 2>/dev/null || echo "0")
    fi
    echo "  New build timestamp: $NEW_TIMESTAMP"
else
    NEW_TIMESTAMP="0"
fi

# Step 6: Determine if version should be incremented
VERSION_UPDATED=false
if [ "$NEW_TIMESTAMP" != "$OLD_TIMESTAMP" ] && [ "$NEW_TIMESTAMP" != "0" ]; then
    echo -e "${GREEN}✅ Changes detected - updating version number${NC}"
    
    # Parse version components (major.minor.patch)
    IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"
    
    # Increment patch version
    NEW_PATCH=$((PATCH + 1))
    NEW_VERSION="$MAJOR.$MINOR.$NEW_PATCH"
    
    echo "  New version: $NEW_VERSION"
    
    # Update Cargo.toml
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS sed
        sed -i '' "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "$PROJECT_ROOT/Cargo.toml"
    else
        # Linux sed
        sed -i "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "$PROJECT_ROOT/Cargo.toml"
    fi
    
    echo -e "${GREEN}✅ Version updated: $CURRENT_VERSION → $NEW_VERSION${NC}"
    VERSION_UPDATED=true
    
    # Step 7: Ensure correct keypair before rebuild
    echo -e "${YELLOW}🔧 Ensuring correct program keypair before rebuild...${NC}"
    if [ ! -f "$PROGRAM_KEYPAIR" ] || [ "$(solana-keygen pubkey "$PROGRAM_KEYPAIR")" != "$EXPECTED_PROGRAM_ID" ]; then
        echo -e "${YELLOW}🔧 Restoring correct keypair before rebuild...${NC}"
        if [ -f "$TEMP_KEYPAIR" ]; then
            cp "$TEMP_KEYPAIR" "$PROGRAM_KEYPAIR"
            echo -e "${GREEN}✅ Program keypair restored for rebuild: $EXPECTED_PROGRAM_ID${NC}"
        else
            echo -e "${RED}❌ CRITICAL: Cannot rebuild without correct keypair${NC}"
            exit 1
        fi
    fi
    
    # Step 7: Rebuild with new version
    echo -e "${YELLOW}🔨 Rebuilding with updated version...${NC}"
    RUSTFLAGS="-C link-arg=-zstack-size=131072" cargo build-sbf || true
    if [ $? -ne 0 ]; then
        echo -e "${RED}❌ Rebuild failed${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ Final build successful${NC}"
    
else
    echo -e "${BLUE}ℹ️  No changes detected - keeping current version${NC}"
    NEW_VERSION="$CURRENT_VERSION"
    echo -e "${GREEN}✅ Build successful (no changes)${NC}"
fi

# 🔧 FIX: Update deployment_info.json with new version EARLY for test compatibility
# This ensures that test_contract_version_matches_deployment_info has the correct expected version
if [ -f "$PROJECT_ROOT/deployment_info.json" ]; then
    echo -e "${YELLOW}🔄 Updating deployment_info.json with new version for test compatibility...${NC}"
    
    # Read current deployment_info.json and update version field
    TEMP_DEPLOYMENT_INFO=$(mktemp)
    
    # Use sed to update the version field while preserving the rest
    sed "s/\"version\": \"[^\"]*\"/\"version\": \"$NEW_VERSION\"/" "$PROJECT_ROOT/deployment_info.json" > "$TEMP_DEPLOYMENT_INFO"
    
    # Also update previous_version field
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS sed
        sed -i '' "s/\"previous_version\": \"[^\"]*\"/\"previous_version\": \"$CURRENT_VERSION\"/" "$TEMP_DEPLOYMENT_INFO"
    else
        # Linux sed
        sed -i "s/\"previous_version\": \"[^\"]*\"/\"previous_version\": \"$CURRENT_VERSION\"/" "$TEMP_DEPLOYMENT_INFO"
    fi
    
    # Replace original with updated version
    mv "$TEMP_DEPLOYMENT_INFO" "$PROJECT_ROOT/deployment_info.json"

    
    echo -e "${GREEN}✅ deployment_info.json pre-updated for test compatibility${NC}"
else
    echo -e "${BLUE}ℹ️  deployment_info.json doesn't exist yet - will be created after deployment${NC}"
fi

echo ""

# Step 8: Configure Solana CLI for remote endpoint
echo -e "${YELLOW}⚙️  Configuring Solana CLI for remote endpoint...${NC}"
solana config set --url $RPC_URL
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ CLI configured for remote validator${NC}"
else
    echo -e "${RED}❌ CLI configuration failed${NC}"
    exit 1
fi

# Step 9: Check/create keypair
if [ ! -f "$KEYPAIR_PATH" ]; then
    echo -e "${YELLOW}🔑 Creating new keypair...${NC}"
    solana-keygen new --no-bip39-passphrase --outfile $KEYPAIR_PATH
fi

# Step 10: Check wallet balances
echo -e "${YELLOW}💰 Checking wallet balances...${NC}"
DEFAULT_WALLET_ADDRESS=$(solana-keygen pubkey $KEYPAIR_PATH)
echo "  Default Wallet: $DEFAULT_WALLET_ADDRESS"
echo "  Backpack Wallet: $BACKPACK_WALLET"

# Check Backpack wallet balance
BACKPACK_BALANCE=$(solana balance $BACKPACK_WALLET 2>/dev/null | awk '{print $1}' | head -1)
# Fallback if balance command fails
if [ -z "$BACKPACK_BALANCE" ] || [ "$BACKPACK_BALANCE" = "Error:" ]; then
    BACKPACK_BALANCE="0"
fi

# Check default wallet balance
DEFAULT_WALLET_BALANCE=$(solana balance $DEFAULT_WALLET_ADDRESS 2>/dev/null | awk '{print $1}' | head -1)
# Fallback if balance command fails
if [ -z "$DEFAULT_WALLET_BALANCE" ] || [ "$DEFAULT_WALLET_BALANCE" = "Error:" ]; then
    DEFAULT_WALLET_BALANCE="0"
fi

echo -e "${GREEN}  Current Backpack Wallet Balance: $BACKPACK_BALANCE SOL${NC}"
echo -e "${GREEN}  Current Default Wallet Balance: $DEFAULT_WALLET_BALANCE SOL${NC}"

# Display current balances (no automatic funding)
echo -e "${GREEN}✅ Current Backpack wallet balance: $BACKPACK_BALANCE SOL${NC}"
echo -e "${GREEN}✅ Current Default wallet balance: $DEFAULT_WALLET_BALANCE SOL${NC}"
FINAL_BALANCE=$BACKPACK_BALANCE

# Step 11: Check if program exists on remote and compare versions
echo -e "${YELLOW}🔍 Checking remote program status...${NC}"

DEPLOY_ACTION=""
DEPLOY_RESULT=""
REMOTE_VERSION=""

# Check if account/program already exists on remote
if [ "$PROGRAM_ID" != "Will be generated during build" ]; then
    echo "  Checking if account $PROGRAM_ID exists on remote..."
    
    # Check if any account exists at this address
    if solana account $PROGRAM_ID >/dev/null 2>&1; then
        echo "  Account exists on remote! Checking what type..."
        
        # Check if it's a program
        if solana program show $PROGRAM_ID >/dev/null 2>&1; then
            echo "  It's a program! Checking if it's upgradeable..."
            
            # Try to get program info for upgrade check
            if command -v jq >/dev/null 2>&1; then
                PROGRAM_INFO=$(solana program show $PROGRAM_ID --output json 2>/dev/null)
                if [ $? -eq 0 ]; then
                    IS_UPGRADEABLE=$(echo "$PROGRAM_INFO" | jq -r '.programdataAddress != null' 2>/dev/null)
                    echo "  Upgradeable check result: $IS_UPGRADEABLE"
                else
                    echo "  Could not get program info, assuming upgradeable"
                    IS_UPGRADEABLE="true"
                fi
            else
                echo "  jq not found, assuming program is upgradeable"
                IS_UPGRADEABLE="true"
            fi
            
            if [ "$IS_UPGRADEABLE" = "true" ]; then
                DEPLOY_ACTION="UPGRADE"
                echo -e "${BLUE}📈 UPGRADING existing program on remote...${NC}"
                echo "  Program exists and is upgradeable. Attempting upgrade..."
                
                # Attempt upgrade
                DEPLOY_OUTPUT=$(solana program deploy "$PROJECT_ROOT/target/deploy/fixed_ratio_trading.so" --program-id "$PROGRAM_KEYPAIR" --upgrade-authority "$KEYPAIR_PATH" 2>&1)
                DEPLOY_EXIT_CODE=$?
                
                # Check if Solana detected no changes
                if echo "$DEPLOY_OUTPUT" | grep -q "Program was not upgraded"; then
                    DEPLOY_RESULT="NO_UPGRADE_NEEDED"
                elif [ $DEPLOY_EXIT_CODE -eq 0 ]; then
                    DEPLOY_RESULT="UPGRADED"
                else
                    DEPLOY_RESULT="FAILED"
                fi
            else
                echo -e "${RED}❌ Program exists but is not upgradeable${NC}"
                echo "   Cannot upgrade immutable program on remote validator"
                echo "   A new program ID would be required"
                exit 1
            fi
        else
            echo -e "${RED}❌ Account exists but is not a program${NC}"
            echo "   Cannot deploy to existing non-program account on remote validator"
            exit 1
        fi
    else
        echo "  No account exists at this address on remote (expected for first deployment)"
        
        DEPLOY_ACTION="CREATE"
        echo -e "${BLUE}🆕 CREATING new program on remote...${NC}"
        echo "  Using initial deployment with upgrade authority..."
        
        DEPLOY_OUTPUT=$(solana program deploy "$PROJECT_ROOT/target/deploy/fixed_ratio_trading.so" --program-id "$PROGRAM_KEYPAIR" --upgrade-authority "$KEYPAIR_PATH" 2>&1)
        DEPLOY_EXIT_CODE=$?
        DEPLOY_RESULT=$([ $DEPLOY_EXIT_CODE -eq 0 ] && echo "CREATED" || echo "FAILED")
    fi
else
    DEPLOY_ACTION="CREATE"
    echo -e "${BLUE}🆕 CREATING new program on remote...${NC}"
    echo "  Using initial deployment with upgrade authority..."
    
    DEPLOY_OUTPUT=$(solana program deploy "$PROJECT_ROOT/target/deploy/fixed_ratio_trading.so" --upgrade-authority "$KEYPAIR_PATH" 2>&1)
    DEPLOY_EXIT_CODE=$?
    DEPLOY_RESULT=$([ $DEPLOY_EXIT_CODE -eq 0 ] && echo "CREATED" || echo "FAILED")
fi

# Display results with clear status
echo ""
echo -e "${BLUE}📋 REMOTE DEPLOYMENT SUMMARY${NC}"
echo "====================================="

case $DEPLOY_RESULT in
    "CREATED")
        echo -e "${GREEN}✅ STATUS: Program successfully CREATED on remote${NC}"
        echo -e "${GREEN}   🆕 New program deployed with upgrade authority${NC}"
        ;;
    "UPGRADED")
        echo -e "${GREEN}✅ STATUS: Program successfully UPGRADED on remote${NC}"
        echo -e "${GREEN}   📈 Contract code updated, program ID preserved${NC}"
        ;;
    "NO_UPGRADE_NEEDED")
        echo -e "${YELLOW}⚡ STATUS: No upgrade needed on remote${NC}"
        echo -e "${YELLOW}   📊 Program bytecode is already up-to-date${NC}"
        ;;
    "FAILED")
        echo -e "${RED}❌ STATUS: Remote deployment FAILED${NC}"
        echo -e "${RED}   💥 See error details below${NC}"
        echo ""
        echo "Error output:"
        echo "$DEPLOY_OUTPUT"
        exit 1
        ;;
esac

echo "   Action: $DEPLOY_ACTION"
echo "   Program ID: $PROGRAM_ID"
echo "   Remote RPC: $RPC_URL"
echo ""

if [ "$DEPLOY_RESULT" != "FAILED" ]; then
    echo -e "${GREEN}✅ Remote program deployment completed successfully!${NC}"
else
    echo -e "${RED}❌ Remote deployment failed${NC}"
    exit 1
fi

# Step 12: Get the actual deployed program ID and verify
echo -e "${YELLOW}🔍 Getting deployed program ID and verifying on remote...${NC}"
if [ -f "$PROGRAM_KEYPAIR" ]; then
    DEPLOYED_PROGRAM_ID=$(solana-keygen pubkey "$PROGRAM_KEYPAIR")
    echo -e "${GREEN}✅ Program ID: $DEPLOYED_PROGRAM_ID${NC}"
    
    # Verify deployment on remote
    PROGRAM_INFO=$(solana program show $DEPLOYED_PROGRAM_ID --output json 2>/dev/null)
    if [ $? -eq 0 ]; then
        PROGRAM_DATA_ADDRESS=$(echo $PROGRAM_INFO | jq -r '.programdataAddress // "N/A"')
        PROGRAM_SIZE=$(echo $PROGRAM_INFO | jq -r '.dataLen // "N/A"')
        echo -e "${GREEN}✅ Remote program verification successful${NC}"
        echo "  Program Data Address: $PROGRAM_DATA_ADDRESS"
        echo "  Program Size: $PROGRAM_SIZE bytes"
    else
        echo -e "${YELLOW}⚠️  Program deployed but verification data not immediately available${NC}"
    fi
    PROGRAM_ID=$DEPLOYED_PROGRAM_ID
else
    echo -e "${RED}❌ Program keypair not found${NC}"
fi

# Step 12.5: Initialize system with program authority (for fresh deployments)
if [ "$DEPLOY_ACTION" = "CREATE" ]; then
    echo ""
    echo -e "${YELLOW}🔧 Initializing Fixed Ratio Trading system...${NC}"

    if command -v node &> /dev/null; then
        # Check if @solana/web3.js is available
        if [ -d "$PROJECT_ROOT/node_modules/@solana/web3.js" ]; then
            echo "  Using existing @solana/web3.js installation..."
            cd "$PROJECT_ROOT"
            
            # Use the consolidated initialization script
            node scripts/initialize_system.js "$PROGRAM_ID" "$RPC_URL" "$KEYPAIR_PATH"
            INIT_EXIT_CODE=$?
            
            if [ $INIT_EXIT_CODE -eq 0 ]; then
                echo -e "${GREEN}✅ System initialization completed successfully${NC}"
                INITIALIZATION_STATUS="success"
            else
                echo -e "${YELLOW}⚠️  System initialization failed, but deployment was successful${NC}"
                echo "   Try running manually: node scripts/initialize_system.js $PROGRAM_ID $RPC_URL"
                INITIALIZATION_STATUS="failed"
            fi
        else
            echo -e "${YELLOW}⚠️  @solana/web3.js not found, skipping automatic system initialization${NC}"
            echo "   Run 'npm install @solana/web3.js' and then:"
            echo "   node scripts/initialize_system.js $PROGRAM_ID $RPC_URL"
            INITIALIZATION_STATUS="skipped"
        fi
    else
        echo -e "${YELLOW}⚠️  Node.js not found, skipping automatic system initialization${NC}"
        echo "   Install Node.js and run: node scripts/initialize_system.js $PROGRAM_ID $RPC_URL"
        INITIALIZATION_STATUS="skipped"
    fi
else
    echo -e "${BLUE}ℹ️ Skipping initialization (upgrade deployment)${NC}"
    INITIALIZATION_STATUS="skipped"
fi

# Step 13: Save deployment info
echo -e "${YELLOW}💾 Saving deployment information...${NC}"
cat > "$PROJECT_ROOT/deployment_info.json" << EOF
{
  "program_id": "$PROGRAM_ID",
  "version": "$NEW_VERSION",
  "previous_version": "$CURRENT_VERSION",
  "rpc_url": "$RPC_URL",
  "wallet_address": "$DEFAULT_WALLET_ADDRESS",
  "deployment_timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "deployment_type": "remote",
  "program_data_address": "$PROGRAM_DATA_ADDRESS",
  "program_size": "$PROGRAM_SIZE",
  "backpack_wallet": "$BACKPACK_WALLET",
  "backpack_wallet_balance": "$FINAL_BALANCE",
  "default_wallet_balance": "$DEFAULT_WALLET_BALANCE",
  "deploy_action": "$DEPLOY_ACTION",
  "deploy_result": "$DEPLOY_RESULT",
  "initialization_status": "$INITIALIZATION_STATUS",
  "initialization_transaction": "$INITIALIZATION_TX"
}
EOF

echo -e "${GREEN}✅ Deployment information saved to deployment_info.json${NC}"



# Final status
echo ""
echo "======================================================"
echo -e "${GREEN}🎉 DIRECT ENDPOINT DEPLOYMENT COMPLETE!${NC}"
echo "======================================================"
echo -e "${BLUE}📊 Your Fixed Ratio Trading contract is deployed:${NC}"
echo ""
echo "  🌐 Direct RPC: $RPC_URL"
echo "  📊 Program ID: $PROGRAM_ID"
echo "  🔢 Version: $NEW_VERSION"
echo "  💳 Default Wallet: $DEFAULT_WALLET_ADDRESS"
echo "  💰 Default Balance: $DEFAULT_WALLET_BALANCE SOL"
echo "  🎒 Backpack Wallet: $BACKPACK_WALLET"
echo "  💰 Backpack Balance: $FINAL_BALANCE SOL"
echo ""
echo -e "${BLUE}📋 Deployment Details:${NC}"
echo "  📈 Action: $DEPLOY_ACTION"
echo "  ✅ Result: $DEPLOY_RESULT"
echo "  🏗️ Initialization: $INITIALIZATION_STATUS"
if [ "$INITIALIZATION_STATUS" = "success" ] && [ -n "$INITIALIZATION_TX" ]; then
    echo "  🔗 Init Transaction: $INITIALIZATION_TX"
fi
echo "  📊 Program Data: $PROGRAM_DATA_ADDRESS"
echo "  📏 Program Size: $PROGRAM_SIZE bytes"
if [ "$VERSION_VERIFIED" = "true" ]; then
    echo "  🔍 Version Verified: ✅ YES - Contract responding correctly"
elif [ "$VERSION_VERIFIED" = "false" ]; then
    echo "  🔍 Version Verified: ❌ NO - CRITICAL ISSUE DETECTED"
else
    echo "  🔍 Version Verified: ⚠️  SKIPPED - Script not executed"
fi
echo ""
echo -e "${GREEN}💡 The contract is now live on the direct validator endpoint!${NC}"
echo -e "${YELLOW}📝 Next Steps:${NC}"
if [ "$VERSION_VERIFIED" = "false" ]; then
    echo -e "${RED}🚨 CRITICAL: Version verification FAILED - DO NOT USE IN PRODUCTION${NC}"
    echo "  1. ❌ URGENT: Fix deployment issues before proceeding"
    echo "  2. 🔍 Run diagnostic tests: cargo test --test 54_test_get_version"
    echo "  3. 🔄 Redeploy contract if tests fail"
    echo "  4. 🚫 DO NOT create pools or use dashboard until fixed"
    echo "  5. 📞 Contact development team if issues persist"
elif [ "$INITIALIZATION_STATUS" = "success" ]; then
    echo "  1. ✅ Contract is deployed and initialized - ready for pools!"
    echo "  2. 🌐 Access via dashboard pointing to $RPC_URL"
    echo "  3. 🏊‍♂️ Create pools via dashboard (no manual initialization needed)"
    echo "  4. 📊 Monitor with: $PROJECT_ROOT/scripts/monitor_pools.sh"
    if [ "$VERSION_VERIFIED" = "true" ]; then
        echo "  5. ✅ Version verification passed - deployment confirmed"
    else
        echo "  5. ⚠️  Version verification skipped - manual check recommended"
    fi
elif [ "$INITIALIZATION_STATUS" = "failed" ]; then
    echo "  1. ✅ Contract is deployed but initialization failed"
    echo "  2. 🏗️ Initialize manually via dashboard before creating pools"
    echo "  3. 🌐 Access via dashboard pointing to $RPC_URL"
    echo "  4. 📊 Monitor with: $PROJECT_ROOT/scripts/monitor_pools.sh"
    if [ "$VERSION_VERIFIED" = "true" ]; then
        echo "  5. ✅ Version verification passed - deployment confirmed"
    else
        echo "  5. ⚠️  Version verification skipped - manual check recommended"
    fi
else
    echo "  1. ✅ Contract is upgraded and ready for use"
    echo "  2. 🌐 Access via dashboard pointing to $RPC_URL"
    echo "  3. 📊 Monitor with: $PROJECT_ROOT/scripts/monitor_pools.sh"
    if [ "$VERSION_VERIFIED" = "true" ]; then
        echo "  4. ✅ Version verification passed - deployment confirmed"
    else
        echo "  4. ⚠️  Version verification skipped - manual check recommended"
    fi
fi
echo ""
echo -e "${BLUE}🔗 Test connection:${NC}"
echo "  curl -X POST -H \"Content-Type: application/json\" \\"
echo "       -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getAccountInfo\",\"params\":[\"$PROGRAM_ID\"]}' \\"
echo "       \"$RPC_URL\""
echo "" 

echo "======================================================"
echo "🔍 VERIFYING DEPLOYED CONTRACT VERSION"
echo "======================================================"
echo "🔍 Verifying deployed contract version matches build..."

# Create a robust Node.js script to call GetVersion instruction (using the actual deployment keypair)
cat > "$PROJECT_ROOT/temp_version_check.js" << EOF
const { Connection, PublicKey, Transaction, TransactionInstruction, Keypair } = require('@solana/web3.js');
const fs = require('fs');

async function verifyContractVersion() {
    try {
        // Load configuration
        const config = JSON.parse(fs.readFileSync('./shared-config.json', 'utf8'));
        const deploymentInfo = JSON.parse(fs.readFileSync('./deployment_info.json', 'utf8'));
        
        console.log('🔍 Calling GetVersion instruction to verify deployment...');
        console.log('📋 Expected version from deployment_info.json:', deploymentInfo.version);
        
        const connection = new Connection(config.solana.rpcUrl, 'confirmed');
        // Use the actual deployed program ID from deployment_info.json, not shared-config
        const programId = new PublicKey(deploymentInfo.program_id);
        
        // Load the same keypair used for deployment (program upgrade authority)
        const keypairPath = '$KEYPAIR_PATH';
        console.log('🔑 Loading deployment keypair from:', keypairPath);
        
        if (!fs.existsSync(keypairPath)) {
            console.error('❌ CRITICAL ERROR: Deployment keypair not found at:', keypairPath);
            console.error('   Cannot verify version without the deployment keypair');
            process.exit(1);
        }
        
        const keypairData = JSON.parse(fs.readFileSync(keypairPath, 'utf8'));
        const keypair = Keypair.fromSecretKey(new Uint8Array(keypairData));
        
        console.log('🔑 Using deployment keypair:', keypair.publicKey.toString());
        
        // Create GetVersion instruction (1-byte discriminator for unit enum!)
        const instructionData = Buffer.from([14]); // GetVersion = discriminator 14 (1 byte only!)
        const instruction = new TransactionInstruction({
            keys: [], // GetVersion requires no accounts
            programId: programId,
            data: instructionData,
        });
        
        // Get recent blockhash for proper transaction structure
        const { blockhash } = await connection.getLatestBlockhash();
        
        // Create signed transaction (this is the key - must be signed!)
        const signedTransaction = new Transaction().add(instruction);
        signedTransaction.recentBlockhash = blockhash;
        signedTransaction.feePayer = keypair.publicKey;
        signedTransaction.sign(keypair); // This is what makes it work!
        
        console.log('📡 Calling GetVersion instruction on smart contract...');
        console.log('🔍 Debug info:');
        console.log('  Program ID:', deploymentInfo.program_id);
        console.log('  Instruction data:', Array.from(instructionData));
        console.log('  Recent blockhash:', blockhash);
        console.log('  Fee payer:', keypair.publicKey.toString());
        
        // Simulate the signed transaction
        const result = await connection.simulateTransaction(signedTransaction);
        
        console.log('📋 Smart contract simulation result:');
        console.log('  Error:', result?.value?.err);
        console.log('  Logs available:', !!result?.value?.logs);
        if (result?.value?.logs) {
            console.log('  Logs:', result.value.logs);
        }
        
        // Parse version from logs if we got a result
        if (result && !result.value.err && result.value.logs) {
            console.log('📋 Contract version logs from GetVersion:');
            result.value.logs.forEach(log => console.log('   ', log));
            
            // Look for "Contract Version: X.X.X" in logs
            const versionLog = result.value.logs.find(log => log.includes('Contract Version:'));
            if (versionLog) {
                const versionMatch = versionLog.match(/Contract Version:\s*([0-9\.]+)/);
                if (versionMatch) {
                    const deployedVersion = versionMatch[1];
                    console.log('🎯 Deployed contract version:', deployedVersion);
                    console.log('📋 Expected version from deployment_info.json:', deploymentInfo.version);
                    
                    if (deployedVersion === deploymentInfo.version) {
                        console.log('✅ VERSION VERIFICATION SUCCESSFUL!');
                        console.log('🎯 Deployed version matches deployment_info.json version');
                        console.log('🛡️ Deployment integrity confirmed');
                        process.exit(0);
                    } else {
                        console.log('❌ VERSION MISMATCH DETECTED!');
                        console.log('🚨 This indicates a critical deployment issue');
                        console.log('   Expected (deployment_info.json):', deploymentInfo.version);
                        console.log('   Deployed (contract GetVersion): ', deployedVersion);
                        console.log('💡 Possible causes:');
                        console.log('   - Previous version still cached in validator');
                        console.log('   - Deployment process failed silently');
                        console.log('   - Build artifacts out of sync');
                        console.error('❌ DEPLOYMENT VERIFICATION FAILED');
                        process.exit(1);
                    }
                }
            }
            
            console.log('❌ Could not extract version from contract logs');
            console.log('📋 Available logs:', result.value.logs);
            console.error('❌ VERSION EXTRACTION FAILED');
            process.exit(1);
        }
        
        // If we reach here, contract call failed
        console.error('❌ RPC simulation failed to call GetVersion instruction');
        if (result && result.value.err) {
            console.error('   Simulation error:', result.value.err);
        }
        console.error('🚨 CRITICAL: Cannot verify deployed contract version');
        console.error('   This indicates a serious deployment issue');
        console.error('   The contract may not be working correctly');
        console.error('');
        console.error('🛡️  RECOMMENDATION: Check deployment manually:');
        console.error('   cargo test --test 54_test_get_version test_contract_version_matches_deployment_info');
        console.error('');
        console.error('❌ DEPLOYMENT VERIFICATION FAILED');
        process.exit(1);
        
    } catch (error) {
        console.error('❌ Version verification encountered a critical error:', error.message);
        console.error('🔍 Error details:', error.name, error.message);
        console.error('🚨 CRITICAL: Cannot verify deployed contract version');
        console.error('   This indicates a serious deployment or script issue');
        console.error('');
        console.error('🛡️  RECOMMENDATION: Check deployment manually:');
        console.error('   cargo test --test 54_test_get_version test_contract_version_matches_deployment_info');
        console.error('');
        console.error('❌ DEPLOYMENT VERIFICATION FAILED');
        process.exit(1);
    }
}

verifyContractVersion();
EOF

# Run version verification
echo "🚀 Running contract version verification..."
echo "   Using deployment keypair for authentication..."

if node "$PROJECT_ROOT/temp_version_check.js"; then
    echo ""
    echo "✅ CONTRACT VERSION VERIFICATION SUCCESSFUL!"
    echo "🎯 Deployed contract version matches expected version"
    echo "🛡️ Deployment integrity confirmed"
    VERSION_VERIFIED=true
else
    echo ""
    echo "❌ CONTRACT VERSION VERIFICATION FAILED!"
    echo "🚨 CRITICAL DEPLOYMENT ISSUE DETECTED"
    echo ""
    echo "⚠️ POSSIBLE CAUSES:"
    echo "   1. Deployment didn't include latest changes"
    echo "   2. GetVersion instruction is not working correctly"
    echo "   3. Contract deployment was incomplete"
    echo "   4. Build artifacts are out of sync"
    echo "   5. Network connectivity or RPC issues"
    echo ""
    echo "🛠️ REQUIRED ACTIONS:"
    echo "   - Check deployment logs above for errors"
    echo "   - Run unit tests to verify GetVersion works:"
    echo "     cargo test --test 54_test_get_version"
    echo "   - Redeploy if necessary"
    echo "   - Do not use this deployment in production"
    echo ""
    VERSION_VERIFIED=false
fi

# Clean up temporary script
rm -f "$PROJECT_ROOT/temp_version_check.js" 