#!/bin/bash
# Update Dashboard State from Solana Devnet
# This script queries the Solana devnet for Fixed Ratio Trading program state
# and updates the dashboard/state.json file with the latest data
#
# Usage:
#   ./update_state_from_devnet.sh [--program-id PROGRAM_ID]
#
# Options:
#   --program-id    Specify a custom program ID (default from shared-config.json)

set -e

# Find the project root directory (where Cargo.toml is located)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Verify we found the correct project directory
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    echo "❌ Error: Could not find Cargo.toml in project root: $PROJECT_ROOT"
    exit 1
fi

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Default configuration
DEVNET_RPC_URL="https://api.devnet.solana.com"
PROGRAM_ID=""
COMMITMENT="confirmed"

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --program-id)
            PROGRAM_ID="$2"
            shift 2
            ;;
        --help|-h)
            echo "Update Dashboard State from Solana Devnet"
            echo ""
            echo "Usage: $0 [--program-id PROGRAM_ID]"
            echo ""
            echo "Options:"
            echo "  --program-id   Specify a custom program ID"
            echo "  --help, -h     Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

echo "🌐 Fixed Ratio Trading - Devnet State Update Script"
echo "==================================================="
echo "📂 Project Root: $PROJECT_ROOT"
echo ""

# Load program ID from shared config if not provided
if [ -z "$PROGRAM_ID" ]; then
    SHARED_CONFIG="$PROJECT_ROOT/shared-config.json"
    if [ -f "$SHARED_CONFIG" ] && command -v jq >/dev/null 2>&1; then
        echo -e "${BLUE}📋 Loading program ID from shared configuration...${NC}"
        PROGRAM_ID=$(jq -r '.program.programId' "$SHARED_CONFIG" 2>/dev/null || echo "")
        if [ -n "$PROGRAM_ID" ] && [ "$PROGRAM_ID" != "null" ]; then
            echo -e "${GREEN}✅ Program ID loaded from shared-config.json: $PROGRAM_ID${NC}"
        else
            echo -e "${RED}❌ No valid program ID found in shared-config.json${NC}"
            exit 1
        fi
    else
        echo -e "${RED}❌ No program ID specified and shared-config.json not found or jq not available${NC}"
        echo "Usage: $0 --program-id YOUR_PROGRAM_ID"
        exit 1
    fi
fi

echo -e "${BLUE}📋 Configuration:${NC}"
echo "  🌐 Devnet RPC: $DEVNET_RPC_URL"
echo "  🆔 Program ID: $PROGRAM_ID"
echo "  📁 Output File: $PROJECT_ROOT/dashboard/state.json"
echo ""

# Check for required tools
echo -e "${YELLOW}🔧 Checking required tools...${NC}"
MISSING_TOOLS=""
command -v node >/dev/null 2>&1 || MISSING_TOOLS="$MISSING_TOOLS node"
command -v npm >/dev/null 2>&1 || MISSING_TOOLS="$MISSING_TOOLS npm"

if [ -n "$MISSING_TOOLS" ]; then
    echo -e "${RED}❌ Missing required tools:$MISSING_TOOLS${NC}"
    echo "   Please install Node.js and npm first"
    exit 1
fi
echo -e "${GREEN}✅ All required tools found${NC}"

# Check if @solana/web3.js is available
if [ ! -d "$PROJECT_ROOT/node_modules/@solana/web3.js" ]; then
    echo -e "${YELLOW}📦 Installing required dependencies...${NC}"
    cd "$PROJECT_ROOT"
    if ! npm install @solana/web3.js; then
        echo -e "${RED}❌ Failed to install @solana/web3.js${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ Dependencies installed${NC}"
fi

# Test devnet connectivity
echo -e "${YELLOW}🔍 Testing devnet connectivity...${NC}"
if command -v curl >/dev/null 2>&1; then
    if curl -s --connect-timeout 10 -X POST -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' \
        "$DEVNET_RPC_URL" | grep -q "ok"; then
        echo -e "${GREEN}✅ Devnet is accessible${NC}"
    else
        echo -e "${RED}❌ Cannot connect to devnet${NC}"
        echo "   Please check your internet connection"
        exit 1
    fi
else
    echo -e "${YELLOW}⚠️ curl not found, skipping connectivity test${NC}"
fi

# Create a temporary Node.js script to query devnet
TEMP_SCRIPT="$PROJECT_ROOT/temp_devnet_query.js"
cat > "$TEMP_SCRIPT" << 'EOF'
#!/usr/bin/env node

const { Connection, PublicKey } = require('@solana/web3.js');
const fs = require('fs');
const path = require('path');

// Configuration from command line arguments
const DEVNET_RPC_URL = process.argv[2];
const PROGRAM_ID = process.argv[3];
const OUTPUT_FILE = process.argv[4];

// PDA seed constants (must match smart contract)
const SEEDS = {
    POOL_STATE: 'pool_state',
    MAIN_TREASURY: 'main_treasury',
    SYSTEM_STATE: 'system_state'
};

/**
 * Parse PoolState account data (simplified version)
 */
function parsePoolState(data, address) {
    try {
        const dataArray = new Uint8Array(data);
        let offset = 0;
        
        const readPubkey = () => {
            const pubkey = new PublicKey(dataArray.slice(offset, offset + 32));
            offset += 32;
            return pubkey.toString();
        };

        const readU64 = () => {
            const value = dataArray.slice(offset, offset + 8);
            offset += 8;
            let result = 0n;
            for (let i = 7; i >= 0; i--) {
                result = (result << 8n) + BigInt(value[i]);
            }
            return Number(result);
        };

        const readU8 = () => {
            const value = dataArray[offset];
            offset += 1;
            return value;
        };

        // Parse main pool fields
        const poolState = {
            address: address.toString(),
            owner: readPubkey(),
            token_a_mint: readPubkey(),
            token_b_mint: readPubkey(),
            token_a_vault: readPubkey(),
            token_b_vault: readPubkey(),
            lp_token_a_mint: readPubkey(),
            lp_token_b_mint: readPubkey(),
            ratio_a_numerator: readU64(),
            ratio_b_denominator: readU64(),
            total_token_a_liquidity: readU64(),
            total_token_b_liquidity: readU64(),
            pool_authority_bump_seed: readU8(),
            token_a_vault_bump_seed: readU8(),
            token_b_vault_bump_seed: readU8(),
            lp_token_a_mint_bump_seed: readU8(),
            lp_token_b_mint_bump_seed: readU8(),
            flags: readU8(),
        };
        
        // Skip to fee fields (approximate offset)
        offset = 200; // Approximate position
        if (offset + 40 < dataArray.length) {
            try {
                poolState.collected_fees_token_a = readU64();
                poolState.collected_fees_token_b = readU64();
                poolState.total_fees_withdrawn_token_a = readU64();
                poolState.total_fees_withdrawn_token_b = readU64();
                poolState.total_sol_fees_collected = readU64();
            } catch (e) {
                // Use defaults if parsing fails
                poolState.collected_fees_token_a = 0;
                poolState.collected_fees_token_b = 0;
                poolState.total_fees_withdrawn_token_a = 0;
                poolState.total_fees_withdrawn_token_b = 0;
                poolState.total_sol_fees_collected = 0;
            }
        }
        
        // Decode flags
        const flags = poolState.flags || 0;
        poolState.flags_decoded = {
            one_to_many_ratio: (flags & 1) !== 0,
            liquidity_paused: (flags & 2) !== 0,
            swaps_paused: (flags & 4) !== 0,
            withdrawal_protection: (flags & 8) !== 0,
            single_lp_token_mode: (flags & 16) !== 0
        };
        
        return poolState;
    } catch (error) {
        console.error(`Error parsing PoolState for ${address}:`, error);
        return null;
    }
}

/**
 * Parse MainTreasuryState account data
 */
function parseMainTreasuryState(data) {
    try {
        const dataArray = new Uint8Array(data);
        let offset = 0;
        
        const readU64 = () => {
            const value = dataArray.slice(offset, offset + 8);
            offset += 8;
            let result = 0n;
            for (let i = 7; i >= 0; i--) {
                result = (result << 8n) + BigInt(value[i]);
            }
            return Number(result);
        };

        const readI64 = () => {
            const value = dataArray.slice(offset, offset + 8);
            offset += 8;
            const view = new DataView(value.buffer, value.byteOffset, value.byteLength);
            return Number(view.getBigInt64(0, true));
        };

        return {
            total_balance: readU64(),
            rent_exempt_minimum: readU64(),
            total_withdrawn: readU64(),
            pool_creation_count: readU64(),
            liquidity_operation_count: readU64(),
            regular_swap_count: readU64(),
            treasury_withdrawal_count: readU64(),
            failed_operation_count: readU64(),
            total_pool_creation_fees: readU64(),
            total_liquidity_fees: readU64(),
            total_regular_swap_fees: readU64(),
            total_swap_contract_fees: readU64(),
            last_update_timestamp: readI64(),
            total_consolidations_performed: readU64(),
            last_consolidation_timestamp: readI64()
        };
    } catch (error) {
        console.error('Error parsing MainTreasuryState:', error);
        return null;
    }
}

/**
 * Parse SystemState account data
 */
function parseSystemState(data) {
    try {
        const dataArray = new Uint8Array(data);
        let offset = 0;
        
        const readU8 = () => {
            const value = dataArray[offset];
            offset += 1;
            return value;
        };

        const readBool = () => {
            const value = dataArray[offset] !== 0;
            offset += 1;
            return value;
        };

        const readI64 = () => {
            const value = dataArray.slice(offset, offset + 8);
            offset += 8;
            const view = new DataView(value.buffer, value.byteOffset, value.byteLength);
            return Number(view.getBigInt64(0, true));
        };

        const pauseReasonCode = readU8();
        
        return {
            pause_reason_code: pauseReasonCode,
            is_paused: readBool(),
            pause_timestamp: readI64()
        };
    } catch (error) {
        console.error('Error parsing SystemState:', error);
        return null;
    }
}

/**
 * Derive PDA addresses
 */
async function derivePDAAddresses(programId) {
    const programPubkey = new PublicKey(programId);
    
    const [mainTreasuryPda] = await PublicKey.findProgramAddress(
        [Buffer.from(SEEDS.MAIN_TREASURY)],
        programPubkey
    );
    
    const [systemStatePda] = await PublicKey.findProgramAddress(
        [Buffer.from(SEEDS.SYSTEM_STATE)],
        programPubkey
    );
    
    return { mainTreasuryPda, systemStatePda };
}

/**
 * Main execution
 */
async function main() {
    try {
        console.log('🌐 Connecting to Solana Devnet...');
        const connection = new Connection(DEVNET_RPC_URL, 'confirmed');
        
        // Test connection
        const version = await connection.getVersion();
        console.log(`✅ Connected to devnet, Solana version: ${version['solana-core']}`);
        
        // Derive PDA addresses
        const pdaAddresses = await derivePDAAddresses(PROGRAM_ID);
        console.log(`🔑 Main Treasury PDA: ${pdaAddresses.mainTreasuryPda.toString()}`);
        console.log(`🔑 System State PDA: ${pdaAddresses.systemStatePda.toString()}`);
        
        // Query all pool accounts
        console.log('🔍 Querying pool accounts...');
        const programPubkey = new PublicKey(PROGRAM_ID);
        const accounts = await connection.getProgramAccounts(programPubkey, {
            commitment: 'confirmed',
            encoding: 'base64'
        });
        
        console.log(`📊 Found ${accounts.length} program accounts`);
        
        // Parse pools
        const pools = [];
        for (const account of accounts) {
            if (account.account.data.length > 300) { // Minimum size for PoolState
                const poolState = parsePoolState(account.account.data, account.pubkey);
                if (poolState) {
                    pools.push(poolState);
                    console.log(`✅ Parsed pool: ${account.pubkey.toString()}`);
                }
            }
        }
        
        // Query treasury state
        console.log('🏛️ Querying treasury state...');
        let mainTreasuryState = null;
        try {
            const treasuryAccount = await connection.getAccountInfo(pdaAddresses.mainTreasuryPda, 'confirmed');
            if (treasuryAccount) {
                mainTreasuryState = parseMainTreasuryState(treasuryAccount.data);
                console.log('✅ Parsed MainTreasuryState');
            } else {
                console.log('⚠️ MainTreasuryState account not found');
            }
        } catch (error) {
            console.warn('⚠️ Error querying treasury state:', error.message);
        }
        
        // Query system state
        console.log('⚙️ Querying system state...');
        let systemState = null;
        try {
            const systemAccount = await connection.getAccountInfo(pdaAddresses.systemStatePda, 'confirmed');
            if (systemAccount) {
                systemState = parseSystemState(systemAccount.data);
                console.log('✅ Parsed SystemState');
            } else {
                console.log('⚠️ SystemState account not found');
            }
        } catch (error) {
            console.warn('⚠️ Error querying system state:', error.message);
        }
        
        // Compile final state data
        const stateData = {
            metadata: {
                generated_at: new Date().toISOString(),
                program_id: PROGRAM_ID,
                rpc_url: DEVNET_RPC_URL,
                script_version: '1.0.0',
                solana_environment: 'devnet'
            },
            pools: pools,
            main_treasury_state: mainTreasuryState,
            system_state: systemState,
            pda_addresses: {
                main_treasury: pdaAddresses.mainTreasuryPda.toString(),
                system_state: pdaAddresses.systemStatePda.toString()
            }
        };
        
        // Save to file
        const outputDir = path.dirname(OUTPUT_FILE);
        if (!fs.existsSync(outputDir)) {
            fs.mkdirSync(outputDir, { recursive: true });
        }
        
        fs.writeFileSync(OUTPUT_FILE, JSON.stringify(stateData, null, 2), 'utf8');
        
        console.log('');
        console.log('📊 DEVNET STATE UPDATE SUMMARY');
        console.log('==============================');
        console.log(`🏊 Pools found: ${pools.length}`);
        console.log(`💰 Treasury state: ${mainTreasuryState ? '✅' : '❌'}`);
        console.log(`⚙️ System state: ${systemState ? '✅' : '❌'}`);
        console.log(`📁 Output file: ${OUTPUT_FILE}`);
        console.log('');
        console.log('🎉 Dashboard state updated from devnet successfully!');
        
    } catch (error) {
        console.error('💥 Error updating state from devnet:', error);
        process.exit(1);
    }
}

// Run the script
main();
EOF

# Execute the temporary script
echo -e "${YELLOW}🔍 Querying devnet for program state data...${NC}"
cd "$PROJECT_ROOT"

# Set environment and run the query
if node "$TEMP_SCRIPT" "$DEVNET_RPC_URL" "$PROGRAM_ID" "$PROJECT_ROOT/dashboard/state.json"; then
    echo -e "${GREEN}✅ Dashboard state file updated successfully${NC}"
    
    # Display file info
    if [ -f "$PROJECT_ROOT/dashboard/state.json" ]; then
        FILE_SIZE=$(ls -lh "$PROJECT_ROOT/dashboard/state.json" | awk '{print $5}')
        echo -e "${BLUE}📁 State file size: $FILE_SIZE${NC}"
        
        # Show a summary of what was found using jq if available
        if command -v jq >/dev/null 2>&1; then
            echo -e "${BLUE}📊 Quick summary:${NC}"
            POOL_COUNT=$(jq -r '.pools | length' "$PROJECT_ROOT/dashboard/state.json" 2>/dev/null || echo "N/A")
            TREASURY_STATUS=$(jq -r 'if .main_treasury_state then "✅ Available" else "❌ Not found" end' "$PROJECT_ROOT/dashboard/state.json" 2>/dev/null || echo "N/A")
            SYSTEM_STATUS=$(jq -r 'if .system_state then "✅ Available" else "❌ Not found" end' "$PROJECT_ROOT/dashboard/state.json" 2>/dev/null || echo "N/A")
            
            echo "  🏊 Pools: $POOL_COUNT"
            echo "  🏛️ Treasury: $TREASURY_STATUS"
            echo "  ⚙️ System: $SYSTEM_STATUS"
        fi
    fi
    
    echo ""
    echo -e "${GREEN}🎉 Devnet state update completed successfully!${NC}"
    echo -e "${BLUE}💡 Your dashboard will now show the latest devnet data${NC}"
    echo ""
    echo -e "${YELLOW}📝 Next steps:${NC}"
    echo "  1. 🌐 Open your dashboard (pointing to devnet)"
    echo "  2. 🔄 Refresh the page to see updated data"
    echo "  3. 🏊‍♂️ View pools and treasury state from devnet"
    echo ""
    
else
    echo -e "${RED}❌ Failed to update dashboard state from devnet${NC}"
    rm -f "$TEMP_SCRIPT"
    exit 1
fi

# Clean up temporary script
rm -f "$TEMP_SCRIPT"

echo -e "${BLUE}🧹 Temporary files cleaned up${NC}"
echo -e "${GREEN}✅ Devnet state update script completed${NC}" 