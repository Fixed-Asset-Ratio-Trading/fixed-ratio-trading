#!/bin/bash

# Fixed Ratio Trading - MainNet Deployment Script (Phase 1)
# This script handles compilation, verification, deployment, and system initialization
# 
# Usage:
#   ./scripts/MainNet/01_deploy.sh
#
# What it does:
#   1. Compiles the program for MainNet
#   2. Verifies all keypairs and addresses
#   3. Deploys the program to MainNet
#   4. Initializes the system state with admin authority
#   5. Records deployment information

set -e  # Exit on error

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROGRAM_ID="quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD"
PROGRAM_KEYPAIR="/Users/davinci/code/keys/MainNet-quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD.json"
DEPLOYMENT_AUTHORITY="3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB"
DEPLOYMENT_KEYPAIR="/Users/davinci/code/keys/3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB.json"
ADMIN_AUTHORITY="4ekSqR4pNZ5hp4cRyicji1Yj7ZCphgkYQhwZf2ib9Wko"
RPC_URL="https://api.mainnet-beta.solana.com"
PROJECT_ROOT="/Users/davinci/code/fixed-ratio-trading"
DEPLOYMENT_LOG="$PROJECT_ROOT/mainnet_deployment_phase1.log"
DEPLOYMENT_INFO="$PROJECT_ROOT/deployment_info_mainnet_phase1.json"

# Mode (MainNet default; use --test to target localnet with MainNet build)
TEST_MODE=0
if [ "${1:-}" = "--test" ]; then
    TEST_MODE=1
    RPC_URL="http://127.0.0.1:8899"
    DEPLOYMENT_LOG="$PROJECT_ROOT/mainnet_deployment_phase1_localnet.log"
    DEPLOYMENT_INFO="$PROJECT_ROOT/deployment_info_mainnet_phase1_localnet.json"
    BINARY_HASH_FILE=".mainnet_binary_hash_phase1_localnet"
    DEPLOY_TX_FILE=".mainnet_deploy_tx_phase1_localnet"
    INIT_INFO_PATH="$PROJECT_ROOT/.mainnet_init_info_phase1_localnet.json"
else
    BINARY_HASH_FILE=".mainnet_binary_hash_phase1"
    DEPLOY_TX_FILE=".mainnet_deploy_tx_phase1"
    INIT_INFO_PATH="$PROJECT_ROOT/.mainnet_init_info_phase1.json"
fi
print_info "Mode: $( [ $TEST_MODE -eq 1 ] && echo 'TEST (localnet)' || echo 'MAINNET' )"
print_info "RPC URL: $RPC_URL"

# Function to print colored messages
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to log messages
log_message() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" >> "$DEPLOYMENT_LOG"
}

# Function to check prerequisites
check_prerequisites() {
    print_info "Phase 1: Checking prerequisites..."
    log_message "Starting Phase 1 deployment prerequisites check"
    
    # Check if Solana CLI is installed
    if ! command -v solana &> /dev/null; then
        print_error "Solana CLI is not installed"
        exit 1
    fi
    
    # Check if Node.js is installed
    if ! command -v node &> /dev/null; then
        print_error "Node.js is not installed"
        exit 1
    fi
    
    # Check if program keypair exists and verify its public key
    if [ ! -f "$PROGRAM_KEYPAIR" ]; then
        print_error "Program keypair not found: $PROGRAM_KEYPAIR"
        print_error "This keypair must exist before running deployment"
        exit 1
    fi
    
    # Verify program keypair has correct public key
    ACTUAL_PROGRAM_PUBKEY=$(solana-keygen pubkey "$PROGRAM_KEYPAIR")
    if [ "$ACTUAL_PROGRAM_PUBKEY" != "$PROGRAM_ID" ]; then
        print_error "Program keypair public key mismatch!"
        print_error "  Expected: $PROGRAM_ID"
        print_error "  Actual:   $ACTUAL_PROGRAM_PUBKEY"
        print_error "Please provide the correct program keypair file"
        exit 1
    fi
    print_success "Program keypair verified: $PROGRAM_ID"
    
    # Check if deployment keypair exists and verify its public key
    if [ ! -f "$DEPLOYMENT_KEYPAIR" ]; then
        print_error "Deployment keypair not found: $DEPLOYMENT_KEYPAIR"
        print_error "Please transfer the deployment keypair to this location"
        print_error "This script does not generate keypairs - all keys must be provided"
        exit 1
    fi
    
    # Verify deployment keypair has correct public key
    ACTUAL_DEPLOY_PUBKEY=$(solana-keygen pubkey "$DEPLOYMENT_KEYPAIR")
    if [ "$ACTUAL_DEPLOY_PUBKEY" != "$DEPLOYMENT_AUTHORITY" ]; then
        print_error "Deployment keypair public key mismatch!"
        print_error "  Expected: $DEPLOYMENT_AUTHORITY"
        print_error "  Actual:   $ACTUAL_DEPLOY_PUBKEY"
        print_error "Please provide the correct deployment keypair file"
        exit 1
    fi
    print_success "Deployment keypair verified: $DEPLOYMENT_AUTHORITY"
    
    # Check deployment authority balance (require 7 SOL minimum)
    print_info "Checking deployment authority balance..."
    
    DEPLOYMENT_BALANCE=$(solana balance "$DEPLOYMENT_AUTHORITY" --url "$RPC_URL" | awk '{print $1}')
    print_info "Deployment authority balance: $DEPLOYMENT_BALANCE SOL"
    
    if (( $(echo "$DEPLOYMENT_BALANCE < 7" | bc -l) )); then
        print_error "Insufficient balance in deployment authority. Need at least 7 SOL, have $DEPLOYMENT_BALANCE SOL"
        print_info "Run: solana transfer $DEPLOYMENT_AUTHORITY 7 --url $RPC_URL"
        exit 1
    fi
    
    print_success "All Phase 1 prerequisites verified"
    log_message "Prerequisites check completed successfully"
}

# Function to build the program
build_program() {
    print_info "Building program for MainNet..."
    log_message "Starting MainNet build"
    
    cd "$PROJECT_ROOT"
    
    # Clean previous builds
    print_info "Cleaning previous builds..."
    cargo clean
    
    # Build with MainNet feature (disable default features to ensure only mainnet is used)
    print_info "Building with MainNet feature flag..."
    cargo build-bpf --features mainnet --no-default-features
    
    # Verify the binary was created
    if [ ! -f "target/deploy/fixed_ratio_trading.so" ]; then
        print_error "Build failed - binary not found"
        exit 1
    fi
    
    # Calculate and store hash
    BINARY_HASH=$(sha256sum target/deploy/fixed_ratio_trading.so | awk '{print $1}')
    print_success "Program built successfully"
    print_info "Binary hash: $BINARY_HASH"
    log_message "Binary hash: $BINARY_HASH"
    
    # Store hash in temp file for later use
    echo "$BINARY_HASH" > "$PROJECT_ROOT/$BINARY_HASH_FILE"
}

# Function to deploy the program
deploy_program() {
    print_info "Deploying program to MainNet..."
    log_message "Starting MainNet program deployment"
    
    cd "$PROJECT_ROOT"
    
    # Deploy the program
    print_info "Deploying with authority: $DEPLOYMENT_AUTHORITY"
    print_warning "This will deploy the program to MainNet - confirm this is correct!"
    
    DEPLOY_OUTPUT=$(solana program deploy \
        target/deploy/fixed_ratio_trading.so \
        --program-id "$PROGRAM_KEYPAIR" \
        --url "$RPC_URL" \
        --keypair "$DEPLOYMENT_KEYPAIR" \
        2>&1)
    
    # Extract transaction signature
    DEPLOY_TX=$(echo "$DEPLOY_OUTPUT" | grep -oE '[A-Za-z0-9]{87,88}' | head -1)
    
    if [ -z "$DEPLOY_TX" ]; then
        print_error "Failed to deploy program"
        echo "$DEPLOY_OUTPUT"
        exit 1
    fi
    
    print_success "Program deployed successfully to MainNet"
    print_info "Deployment transaction: $DEPLOY_TX"
    log_message "Deployment transaction: $DEPLOY_TX"
    
    # Store deployment transaction
    echo "$DEPLOY_TX" > "$PROJECT_ROOT/$DEPLOY_TX_FILE"
    
    # Verify deployment
    print_info "Verifying deployment..."
    solana program show "$PROGRAM_ID" --url "$RPC_URL"
    
    # Dump and verify binary hash matches
    print_info "Verifying binary hash on-chain..."
    solana program dump "$PROGRAM_ID" dumped_mainnet_phase1.so --url "$RPC_URL"
    ONCHAIN_HASH=$(sha256sum dumped_mainnet_phase1.so | awk '{print $1}')
    LOCAL_HASH=$(cat "$PROJECT_ROOT/$BINARY_HASH_FILE")
    
    if [ "$ONCHAIN_HASH" == "$LOCAL_HASH" ]; then
        print_success "Binary hash verification successful"
        rm dumped_mainnet_phase1.so
    else
        print_error "Binary hash mismatch!"
        print_error "  Local:    $LOCAL_HASH"
        print_error "  On-chain: $ONCHAIN_HASH"
        exit 1
    fi
}

# Function to initialize system state
initialize_system() {
    print_info "Initializing system state with admin authority: $ADMIN_AUTHORITY"
    log_message "Starting system initialization"
    
    cd "$PROJECT_ROOT"
    
    # Create initialization script for MainNet Phase 1
    cat > "$PROJECT_ROOT/scripts/MainNet/initialize_phase1.js" << 'EOF'
#!/usr/bin/env node

const { PublicKey, Connection, Transaction, TransactionInstruction, Keypair, SystemProgram, sendAndConfirmTransaction } = require('@solana/web3.js');
const fs = require('fs');
const path = require('path');

// Configuration from arguments
const PROGRAM_ID = process.argv[2];
const RPC_URL = process.argv[3];
const KEYPAIR_PATH = process.argv[4];
const ADMIN_AUTHORITY = process.argv[5];

async function initializeSystem() {
    try {
        console.log('🔧 Fixed Ratio Trading - MainNet System Initialization (Phase 1)');
        console.log('================================================================');
        console.log(`📋 Program ID: ${PROGRAM_ID}`);
        console.log(`🌐 RPC URL: ${RPC_URL}`);
        console.log(`🔑 Admin Authority: ${ADMIN_AUTHORITY}`);
        
        const connection = new Connection(RPC_URL, 'confirmed');
        
        // Load deployment authority keypair
        const keypairData = JSON.parse(fs.readFileSync(KEYPAIR_PATH, 'utf-8'));
        const authority = Keypair.fromSecretKey(new Uint8Array(keypairData));
        console.log(`🔐 Deployment Authority: ${authority.publicKey.toBase58()}`);
        
        // Derive PDAs
        const programId = new PublicKey(PROGRAM_ID);
        const adminAuthority = new PublicKey(ADMIN_AUTHORITY);
        
        const [systemStatePda] = PublicKey.findProgramAddressSync(
            [Buffer.from('system_state')],
            programId
        );
        
        const [mainTreasuryPda] = PublicKey.findProgramAddressSync(
            [Buffer.from('main_treasury')],
            programId
        );
        
        const [programDataAddress] = PublicKey.findProgramAddressSync(
            [programId.toBuffer()],
            new PublicKey('BPFLoaderUpgradeab1e11111111111111111111111')
        );
        
        console.log(`\n📍 Derived PDAs:`);
        console.log(`   SystemState: ${systemStatePda.toBase58()}`);
        console.log(`   MainTreasury: ${mainTreasuryPda.toBase58()}`);
        console.log(`   ProgramData: ${programDataAddress.toBase58()}`);
        
        // Create initialization instruction (InitializeProgram = instruction 0)
        const initInstruction = new TransactionInstruction({
            programId: programId,
            keys: [
                { pubkey: authority.publicKey, isSigner: true, isWritable: true },
                { pubkey: systemStatePda, isSigner: false, isWritable: true },
                { pubkey: mainTreasuryPda, isSigner: false, isWritable: true },
                { pubkey: programDataAddress, isSigner: false, isWritable: false },
                { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
            ],
            data: Buffer.concat([
                Buffer.from([0]), // InitializeProgram instruction
                adminAuthority.toBuffer(), // Admin authority to set (32 bytes)
            ])
        });
        
        // Create and send transaction
        const transaction = new Transaction().add(initInstruction);
        
        console.log(`\n📤 Sending system initialization transaction...`);
        const signature = await sendAndConfirmTransaction(
            connection,
            transaction,
            [authority],
            { 
                commitment: 'confirmed',
                preflightCommitment: 'confirmed'
            }
        );
        
        console.log(`\n✅ System initialized successfully!`);
        console.log(`📝 Transaction signature: ${signature}`);
        console.log(`🔍 View on Solana Explorer: https://explorer.solana.com/tx/${signature}`);
        
        // Verify system state was created correctly
        console.log(`\n🔍 Verifying system state...`);
        const systemStateAccount = await connection.getAccountInfo(systemStatePda);
        if (systemStateAccount) {
            console.log(`✅ SystemState PDA created successfully`);
            console.log(`   Account owner: ${systemStateAccount.owner.toBase58()}`);
            console.log(`   Data length: ${systemStateAccount.data.length} bytes`);
        } else {
            throw new Error('SystemState PDA was not created');
        }
        
        // Save initialization info
        const initInfo = {
            programId: PROGRAM_ID,
            systemStatePda: systemStatePda.toBase58(),
            mainTreasuryPda: mainTreasuryPda.toBase58(),
            adminAuthority: ADMIN_AUTHORITY,
            deploymentAuthority: authority.publicKey.toBase58(),
            initTransaction: signature,
            timestamp: new Date().toISOString(),
            phase: 'phase1_initialization'
        };
        
        fs.writeFileSync(
            path.join(process.cwd(), process.env.INIT_INFO_PATH || '.mainnet_init_info_phase1.json'),
            JSON.stringify(initInfo, null, 2)
        );
        
        console.log(`\n💾 Initialization info saved to .mainnet_init_info_phase1.json`);
        process.exit(0);
    } catch (error) {
        console.error('❌ System initialization failed:', error);
        if (error.logs) {
            console.error('Transaction logs:', error.logs);
        }
        process.exit(1);
    }
}

initializeSystem();
EOF
    
    chmod +x "$PROJECT_ROOT/scripts/MainNet/initialize_phase1.js"
    
    # Run initialization
    print_info "Running system initialization..."
    INIT_INFO_PATH="$INIT_INFO_PATH" node "$PROJECT_ROOT/scripts/MainNet/initialize_phase1.js" \
        "$PROGRAM_ID" \
        "$RPC_URL" \
        "$DEPLOYMENT_KEYPAIR" \
        "$ADMIN_AUTHORITY"
    
    if [ $? -eq 0 ]; then
        print_success "System initialized successfully"
        log_message "System initialization completed successfully"
        
        # Read initialization info
        if [ -f "$INIT_INFO_PATH" ]; then
            INIT_TX=$(jq -r '.initTransaction' "$INIT_INFO_PATH")
            log_message "Initialization transaction: $INIT_TX"
            print_info "System is now ready for verification testing (Phase 2)"
        fi
    else
        print_error "System initialization failed"
        exit 1
    fi
}

# Function to create deployment record
create_deployment_record() {
    print_info "Creating Phase 1 deployment record..."
    
    # Gather all information
    BINARY_HASH=$(cat "$PROJECT_ROOT/$BINARY_HASH_FILE" 2>/dev/null || echo "unknown")
    DEPLOY_TX=$(cat "$PROJECT_ROOT/$DEPLOY_TX_FILE" 2>/dev/null || echo "unknown")
    INIT_INFO=$(cat "$INIT_INFO_PATH" 2>/dev/null || echo "{}")
    
    # Create deployment info JSON
    cat > "$DEPLOYMENT_INFO" << EOF
{
  "phase": "phase1_deployment_and_initialization",
  "network": "mainnet-beta",
  "programId": "$PROGRAM_ID",
  "deploymentAuthority": "$DEPLOYMENT_AUTHORITY",
  "adminAuthority": "$ADMIN_AUTHORITY",
  "upgradeAuthority": "$DEPLOYMENT_AUTHORITY",
  "binaryHash": "$BINARY_HASH",
  "deploymentTransaction": "$DEPLOY_TX",
  "initializationInfo": $INIT_INFO,
  "deploymentTimestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "solanaVersion": "$(solana --version | awk '{print $2}')",
  "rustVersion": "$(rustc --version | awk '{print $2}')",
  "status": "phase1_complete_ready_for_verification",
  "nextStep": "Run Phase 2 verification script: ./scripts/MainNet/02_verify.sh"
}
EOF
    
    print_success "Phase 1 deployment record created: $DEPLOYMENT_INFO"
    log_message "Phase 1 deployment record created"
    
    # Clean up temporary files
    rm -f "$PROJECT_ROOT/$BINARY_HASH_FILE"
    rm -f "$PROJECT_ROOT/$DEPLOY_TX_FILE"
    # Keep init info for Phase 2
}

# Function to display next steps
show_next_steps() {
    print_success "🎉 Phase 1 Deployment Complete!"
    echo ""
    print_info "What was accomplished:"
    echo "  ✅ Program compiled for MainNet"
    echo "  ✅ All keypairs and addresses verified"
    echo "  ✅ Program deployed to MainNet"
    echo "  ✅ System state initialized with admin authority"
    echo "  ✅ Deployment information recorded"
    echo ""
    print_warning "⚠️  IMPORTANT: Upgrade authority is still with deployment key!"
    print_warning "   Do NOT transfer authority until Phase 3 (after verification)"
    echo ""
    print_info "Next Steps:"
    echo "  1. Run Phase 2 verification: ./scripts/MainNet/02_verify.sh"
    echo "  2. If verification passes, run Phase 3 handoff: ./scripts/MainNet/03_handoff.sh"
    echo ""
    print_info "Logs and info:"
    print_info "  Deployment log: $DEPLOYMENT_LOG"
    print_info "  Deployment info: $DEPLOYMENT_INFO"
}

# Main execution
main() {
    print_info "Starting Fixed Ratio Trading MainNet Deployment - Phase 1"
    print_info "========================================================"
    log_message "Starting Phase 1 deployment"
    
    check_prerequisites
    build_program
    deploy_program
    initialize_system
    create_deployment_record
    show_next_steps
    
    print_success "Phase 1 deployment completed successfully!"
}

# Run main function
main
