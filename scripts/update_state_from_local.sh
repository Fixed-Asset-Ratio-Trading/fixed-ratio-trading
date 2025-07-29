#!/bin/bash
# Update Dashboard State from Local/Remote Testnet
# This script queries the local/remote testnet for Fixed Ratio Trading program state
# and updates the dashboard/state.json file with the latest data
#
# Usage:
#   ./update_state_from_local.sh [--program-id PROGRAM_ID] [--rpc-url RPC_URL]

set -e

# Find the project root directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Default configuration
LOCAL_RPC_URL="http://192.168.2.88:8899"
PROGRAM_ID=""

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --program-id)
            PROGRAM_ID="$2"
            shift 2
            ;;
        --rpc-url)
            LOCAL_RPC_URL="$2"
            shift 2
            ;;
        --help|-h)
            echo "Update Dashboard State from Local/Remote Testnet"
            echo ""
            echo "Usage: $0 [--program-id PROGRAM_ID] [--rpc-url RPC_URL]"
            echo ""
            echo "Options:"
            echo "  --program-id   Specify a custom program ID"
            echo "  --rpc-url      Specify a custom RPC URL (default: http://192.168.2.88:8899)"
            echo "  --help, -h     Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo "🏠 Fixed Ratio Trading - Local/Remote Testnet State Update"
echo "========================================================="
echo "📂 Project Root: $PROJECT_ROOT"
echo ""

# Load program ID from shared config if not provided
if [ -z "$PROGRAM_ID" ]; then
    SHARED_CONFIG="$PROJECT_ROOT/shared-config.json"
    if [ -f "$SHARED_CONFIG" ] && command -v jq >/dev/null 2>&1; then
        echo -e "${BLUE}📋 Loading program ID from shared configuration...${NC}"
        PROGRAM_ID=$(jq -r '.program.programId' "$SHARED_CONFIG" 2>/dev/null || echo "")
        if [ -n "$PROGRAM_ID" ] && [ "$PROGRAM_ID" != "null" ]; then
            echo -e "${GREEN}✅ Program ID loaded: $PROGRAM_ID${NC}"
        else
            echo -e "${RED}❌ No valid program ID found in shared-config.json${NC}"
            exit 1
        fi
    else
        echo -e "${RED}❌ No program ID specified and shared-config.json not found${NC}"
        echo "Usage: $0 --program-id YOUR_PROGRAM_ID"
        exit 1
    fi
fi

echo -e "${BLUE}📋 Configuration:${NC}"
echo "  🌐 Local/Remote RPC: $LOCAL_RPC_URL"
echo "  🆔 Program ID: $PROGRAM_ID"
echo "  📁 Output File: $PROJECT_ROOT/dashboard/state.json"
echo ""

# Check for required tools
echo -e "${YELLOW}🔧 Checking required tools...${NC}"
if ! command -v node >/dev/null 2>&1; then
    echo -e "${RED}❌ Node.js is required${NC}"
    exit 1
fi
echo -e "${GREEN}✅ All required tools found${NC}"

# Test connectivity
echo -e "${YELLOW}🔍 Testing testnet connectivity...${NC}"
if command -v curl >/dev/null 2>&1; then
    if curl -s --connect-timeout 10 -X POST -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' \
        "$LOCAL_RPC_URL" | grep -q "ok"; then
        echo -e "${GREEN}✅ Local/Remote testnet is accessible${NC}"
    else
        echo -e "${RED}❌ Cannot connect to local/remote testnet${NC}"
        echo "   Please check if the validator is running at $LOCAL_RPC_URL"
        exit 1
    fi
else
    echo -e "${YELLOW}⚠️ curl not found, skipping connectivity test${NC}"
fi

# Create the query script
echo -e "${YELLOW}🔍 Querying local/remote testnet for program state data...${NC}"

node -e "
const { Connection, PublicKey } = require('@solana/web3.js');
const fs = require('fs');
const path = require('path');

const LOCAL_RPC_URL = '$LOCAL_RPC_URL';
const PROGRAM_ID = '$PROGRAM_ID';
const OUTPUT_FILE = '$PROJECT_ROOT/dashboard/state.json';

// PDA seed constants
const SEEDS = {
    MAIN_TREASURY: 'main_treasury',
    SYSTEM_STATE: 'system_state'
};

// Function to get token decimals from mint account
async function getTokenDecimals(connection, mintAddress) {
    try {
        const mintPublicKey = new PublicKey(mintAddress);
        const mintAccount = await connection.getAccountInfo(mintPublicKey, 'confirmed');
        
        if (!mintAccount) {
            console.warn(\`⚠️ Token mint account not found: \${mintAddress}\`);
            return 0;
        }
        
        // Parse token mint data to get decimals (at offset 44)
        const decimals = mintAccount.data[44];
        return decimals;
    } catch (error) {
        console.warn(\`⚠️ Error fetching decimals for \${mintAddress}:\`, error.message);
        return 0;
    }
}

// Function to convert basis points to display units
function basisPointsToDisplay(basisPoints, decimals) {
    const factor = Math.pow(10, decimals);
    return basisPoints / factor;
}

async function parsePoolState(data, address, connection) {
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
        const owner = readPubkey();
        const token_a_mint = readPubkey();
        const token_b_mint = readPubkey();
        const token_a_vault = readPubkey();
        const token_b_vault = readPubkey();
        const lp_token_a_mint = readPubkey();
        const lp_token_b_mint = readPubkey();
        const ratio_a_numerator = readU64();
        const ratio_b_denominator = readU64();
        
        // Fetch token decimals
        console.log(\`🔍 Fetching decimals for pool \${address.toString().slice(0, 8)}...\`);
        const ratio_a_decimal = await getTokenDecimals(connection, token_a_mint);
        const ratio_b_decimal = await getTokenDecimals(connection, token_b_mint);
        
        // Calculate actual ratios (display units)
        const ratio_a_actual = basisPointsToDisplay(ratio_a_numerator, ratio_a_decimal);
        const ratio_b_actual = basisPointsToDisplay(ratio_b_denominator, ratio_b_decimal);
        
        const poolState = {
            address: address.toString(),
            owner: owner,
            token_a_mint: token_a_mint,
            token_b_mint: token_b_mint,
            token_a_vault: token_a_vault,
            token_b_vault: token_b_vault,
            lp_token_a_mint: lp_token_a_mint,
            lp_token_b_mint: lp_token_b_mint,
            ratio_a_numerator: ratio_a_numerator,
            ratio_a_decimal: ratio_a_decimal,
            ratio_a_actual: ratio_a_actual,
            ratio_b_denominator: ratio_b_denominator,
            ratio_b_decimal: ratio_b_decimal,
            ratio_b_actual: ratio_b_actual,
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
        console.error(\`Error parsing PoolState for \${address}:\`, error);
        return null;
    }
}

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

async function main() {
    try {
        console.log('🏠 Connecting to Local/Remote Testnet...');
        const connection = new Connection(LOCAL_RPC_URL, 'confirmed');
        
        // Test connection
        const version = await connection.getVersion();
        console.log(\`✅ Connected to testnet, Solana version: \${version['solana-core']}\`);
        
        // Derive PDA addresses
        const pdaAddresses = await derivePDAAddresses(PROGRAM_ID);
        console.log(\`🔑 Main Treasury PDA: \${pdaAddresses.mainTreasuryPda.toString()}\`);
        console.log(\`🔑 System State PDA: \${pdaAddresses.systemStatePda.toString()}\`);
        
        // Query all pool accounts
        console.log('🔍 Querying pool accounts...');
        const programPubkey = new PublicKey(PROGRAM_ID);
        const accounts = await connection.getProgramAccounts(programPubkey, {
            commitment: 'confirmed',
            encoding: 'base64'
        });
        
        console.log(\`📊 Found \${accounts.length} program accounts\`);
        
        // Parse pools
        const pools = [];
        for (const account of accounts) {
            if (account.account.data.length > 300) { // Minimum size for PoolState
                const poolState = await parsePoolState(account.account.data, account.pubkey, connection);
                if (poolState) {
                    pools.push(poolState);
                    console.log(\`✅ Parsed pool: \${account.pubkey.toString()}\`);
                    console.log(\`   📊 Token A Decimals: \${poolState.ratio_a_decimal}, Actual Ratio: \${poolState.ratio_a_actual}\`);
                    console.log(\`   📊 Token B Decimals: \${poolState.ratio_b_decimal}, Actual Ratio: \${poolState.ratio_b_actual}\`);
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
                rpc_url: LOCAL_RPC_URL,
                script_version: '1.0.0',
                solana_environment: 'local/remote-testnet'
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
        console.log('📊 LOCAL/REMOTE TESTNET STATE UPDATE SUMMARY');
        console.log('===========================================');
        console.log(\`🏊 Pools found: \${pools.length}\`);
        console.log(\`💰 Treasury state: \${mainTreasuryState ? '✅' : '❌'}\`);
        console.log(\`⚙️ System state: \${systemState ? '✅' : '❌'}\`);
        console.log(\`📁 Output file: \${OUTPUT_FILE}\`);
        console.log('');
        console.log('🎉 Dashboard state updated from local/remote testnet successfully!');
        
    } catch (error) {
        console.error('💥 Error updating state from testnet:', error);
        process.exit(1);
    }
}

main();
" && echo -e "${GREEN}✅ State query completed successfully!${NC}"

echo ""
echo -e "${GREEN}🎉 Local/Remote testnet state update completed!${NC}"
echo -e "${BLUE}💡 Your dashboard will now show the latest testnet data with pools!${NC}"
echo ""
echo -e "${YELLOW}📝 Next steps:${NC}"
echo "  1. 🌐 Open your dashboard"
echo "  2. 🔄 Refresh the page to see updated data"
echo "  3. 🏊‍♂️ View your pools and treasury/system state"
echo "" 