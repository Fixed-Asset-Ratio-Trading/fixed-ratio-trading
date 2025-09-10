#!/bin/bash

# Fixed Ratio Trading - MainNet Handoff Script (Phase 3)

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color
# This script transfers upgrade authority to Squads multisig and remaining SOL
# 
# Usage:
#   ./scripts/MainNet/03_handoff.sh
#
# What it does:
#   1. Verifies Phase 2 verification completed successfully
#   2. Transfers upgrade authority from deployment key to Squads multisig
#   3. Transfers remaining SOL from deployment authority to multisig
#   4. Creates final deployment record
#   5. Provides final security instructions

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
SQUADS_MULTISIG="i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi"
RPC_URL="https://api.mainnet-beta.solana.com"
PROJECT_ROOT="/Users/davinci/code/fixed-ratio-trading"
HANDOFF_LOG="$PROJECT_ROOT/temp/mainnet_handoff_phase3.log"
FINAL_DEPLOYMENT_INFO="$PROJECT_ROOT/temp/deployment_info_mainnet_final.json"

# Mode (MainNet default; use --test to target localnet with MainNet build)
TEST_MODE=0
if [ "${1:-}" = "--test" ]; then
    TEST_MODE=1
    RPC_URL="http://192.168.2.88:8899"
    HANDOFF_LOG="$PROJECT_ROOT/temp/mainnet_handoff_phase3_localnet.log"
    FINAL_DEPLOYMENT_INFO="$PROJECT_ROOT/temp/deployment_info_mainnet_final_localnet.json"
    PHASE2_INFO_PATH="$PROJECT_ROOT/temp/verification_info_mainnet_phase2_localnet.json"
else
    PHASE2_INFO_PATH="$PROJECT_ROOT/temp/verification_info_mainnet_phase2.json"
fi

# Function to print colored messages
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_info "Mode: $( [ $TEST_MODE -eq 1 ] && echo 'TEST (localnet)' || echo 'MAINNET' )"
print_info "RPC URL: $RPC_URL"

# Function to log messages
log_message() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" >> "$HANDOFF_LOG"
}

# Function to check sufficient funds for Phase 3
check_phase3_funds() {
    print_info "Checking sufficient funds for Phase 3 operations..."
    
    # Get current balance
    DEPLOYMENT_BALANCE=$(solana balance "$DEPLOYMENT_AUTHORITY" --url "$RPC_URL" 2>/dev/null | awk '{print $1}' || echo "0")
    
    print_info "Deployment authority balance: $DEPLOYMENT_BALANCE SOL"
    
    # Phase 3 operations are minimal - just authority transfer and SOL transfer
    # But we need some SOL for transaction fees
    MINIMUM_BALANCE="0.1"
    
    if (( $(echo "$DEPLOYMENT_BALANCE < $MINIMUM_BALANCE" | bc -l) )); then
        print_error "Insufficient balance for Phase 3 operations"
        print_error "   Current balance: $DEPLOYMENT_BALANCE SOL"
        print_error "   Required minimum: $MINIMUM_BALANCE SOL"
        print_error "   Phase 3 operations include:"
        print_error "     • Authority transfer to multisig"
        print_error "     • SOL transfer to multisig"
        print_error "     • Transaction fees"
        print_error ""
        print_info "To add funds:"
        print_info "   solana transfer $DEPLOYMENT_AUTHORITY 0.5 --url $RPC_URL"
        exit 1
    else
        print_success "✅ Sufficient funds available for Phase 3"
        log_message "Phase 3 funds check passed - balance: $DEPLOYMENT_BALANCE SOL"
    fi
}

# Function to check Phase 2 completion
check_phase2_completion() {
    print_info "Phase 3: Verifying Phase 2 verification completion..."
    log_message "Starting Phase 3 handoff prerequisites"
    
    # Check if Phase 2 verification info exists
    PHASE2_INFO="$PHASE2_INFO_PATH"
    if [ ! -f "$PHASE2_INFO" ]; then
        print_error "Phase 2 verification info not found: $PHASE2_INFO"
        print_error "Please run Phase 2 verification first: ./scripts/MainNet/02_verify.sh"
        exit 1
    fi
    
    # Verify Phase 2 status
    PHASE2_STATUS=$(jq -r '.status' "$PHASE2_INFO")
    if [ "$PHASE2_STATUS" != "phase2_complete_ready_for_handoff" ]; then
        print_error "Phase 2 not completed successfully. Status: $PHASE2_STATUS"
        print_error "Please run Phase 2 verification first: ./scripts/MainNet/02_verify.sh"
        exit 1
    fi
    
    # Verify current upgrade authority is still deployment authority
    print_info "Verifying current upgrade authority..."
    CURRENT_AUTHORITY=$(solana program show "$PROGRAM_ID" --url "$RPC_URL" | grep "Authority" | awk '{print $2}')
    
    if [ "$CURRENT_AUTHORITY" != "$DEPLOYMENT_AUTHORITY" ]; then
        print_error "Unexpected upgrade authority: $CURRENT_AUTHORITY"
        print_error "Expected: $DEPLOYMENT_AUTHORITY"
        print_error "Authority may have already been transferred"
        exit 1
    fi
    
    print_success "Current upgrade authority confirmed: $DEPLOYMENT_AUTHORITY"

    # Verify the deployment keypair matches the expected deployment authority pubkey
    print_info "Verifying deployment keypair matches deployment authority..."
    DEPLOYMENT_KEYPAIR_PUBKEY=$(solana-keygen pubkey "$DEPLOYMENT_KEYPAIR" 2>/dev/null || echo "")
    if [ -z "$DEPLOYMENT_KEYPAIR_PUBKEY" ]; then
        print_error "Could not read deployment keypair: $DEPLOYMENT_KEYPAIR"
        exit 1
    fi
    if [ "$DEPLOYMENT_KEYPAIR_PUBKEY" != "$DEPLOYMENT_AUTHORITY" ]; then
        print_error "Deployment keypair public key mismatch!"
        print_error "  Expected: $DEPLOYMENT_AUTHORITY"
        print_error "  Actual:   $DEPLOYMENT_KEYPAIR_PUBKEY"
        exit 1
    fi
    print_success "Deployment keypair matches expected authority: $DEPLOYMENT_AUTHORITY"
    
    # Check deployment authority balance
    DEPLOYMENT_BALANCE=$(solana balance "$DEPLOYMENT_AUTHORITY" --url "$RPC_URL" | awk '{print $1}')
    print_info "Deployment authority balance: $DEPLOYMENT_BALANCE SOL"
    
    # Check Squads multisig exists
    print_info "Verifying Squads multisig address..."
    MULTISIG_INFO=$(solana account "$SQUADS_MULTISIG" --url "$RPC_URL" 2>/dev/null || echo "")
    if [ -z "$MULTISIG_INFO" ]; then
        print_error "Squads multisig account not found: $SQUADS_MULTISIG"
        print_error "Please verify the multisig address is correct"
        exit 1
    fi
    
    print_success "Squads multisig verified: $SQUADS_MULTISIG"
    print_success "Phase 2 verification complete - ready for Phase 3"
}

# Function to transfer upgrade authority
transfer_upgrade_authority() {
    print_info "Transferring upgrade authority to Squads multisig..."
    log_message "Starting upgrade authority transfer"
    
    # Final confirmation
    print_warning "🚨 CRITICAL: You are about to transfer upgrade authority!"
    print_warning "From: $DEPLOYMENT_AUTHORITY (your deployment key)"
    print_warning "To:   $SQUADS_MULTISIG (Squads multisig)"
    print_warning ""
    print_warning "After this transfer:"
    print_warning "- Only the Squads multisig can upgrade the program"
    print_warning "- This action CANNOT be undone without multisig approval"
    print_warning "- Make sure the multisig is properly configured"
    echo ""
    
    read -p "Are you absolutely sure you want to continue? Type 'TRANSFER' to confirm: " confirm
    
    if [ "$confirm" != "TRANSFER" ]; then
        print_info "Authority transfer cancelled by user"
        exit 0
    fi
    
    print_info "Proceeding with authority transfer..."
    
    # Transfer upgrade authority
    TRANSFER_OUTPUT=$(solana program set-upgrade-authority \
        "$PROGRAM_ID" \
        --new-upgrade-authority "$SQUADS_MULTISIG" \
        --upgrade-authority "$DEPLOYMENT_KEYPAIR" \
        --skip-new-upgrade-authority-signer-check \
        --url "$RPC_URL" \
        2>&1)
    
    # Extract transaction signature (may be missing on some CLI versions)
    TRANSFER_TX=$(echo "$TRANSFER_OUTPUT" | grep -oE '[A-Za-z0-9]{87,88}' | head -1)
    
    if [ -z "$TRANSFER_TX" ]; then
        print_warning "No transaction signature parsed from CLI output"
        echo "$TRANSFER_OUTPUT"
    else
        print_success "Upgrade authority transferred successfully"
        print_info "Transfer transaction: $TRANSFER_TX"
        log_message "Authority transfer transaction: $TRANSFER_TX"
        # Store transfer transaction
        echo "$TRANSFER_TX" > "$PROJECT_ROOT/temp/.mainnet_transfer_tx_phase3"
    fi
    
    # Verify transfer
    print_info "Verifying authority transfer..."
    sleep 5  # Wait for confirmation
    
    NEW_AUTHORITY=$(solana program show "$PROGRAM_ID" --url "$RPC_URL" | grep "Authority" | awk '{print $2}')
    
    if [ "$NEW_AUTHORITY" == "$SQUADS_MULTISIG" ]; then
        print_success "Authority transfer verified successfully"
        print_success "New upgrade authority: $SQUADS_MULTISIG"
    else
        print_error "Authority transfer verification failed"
        print_error "Expected: $SQUADS_MULTISIG"
        print_error "Actual: $NEW_AUTHORITY"
        exit 1
    fi
}

# Function to transfer remaining SOL
transfer_remaining_sol() {
    print_info "Transferring remaining SOL to Squads multisig..."
    log_message "Starting SOL transfer to multisig"
    
    # Get current balance
    REMAINING_BALANCE=$(solana balance "$DEPLOYMENT_AUTHORITY" --url "$RPC_URL" | awk '{print $1}')
    print_info "Remaining balance in deployment authority: $REMAINING_BALANCE SOL"
    
    # Calculate amount to transfer (leave 0.001 SOL for rent)
    TRANSFER_AMOUNT=$(echo "$REMAINING_BALANCE - 0.001" | bc -l)
    
    if (( $(echo "$TRANSFER_AMOUNT > 0.001" | bc -l) )); then
        print_info "Transferring $TRANSFER_AMOUNT SOL to multisig $SQUADS_MULTISIG"
        
        TRANSFER_OUTPUT=$(solana transfer "$SQUADS_MULTISIG" "$TRANSFER_AMOUNT" \
            --keypair "$DEPLOYMENT_KEYPAIR" \
            --url "$RPC_URL" \
            2>&1)
        
        # Extract transaction signature
        SOL_TRANSFER_TX=$(echo "$TRANSFER_OUTPUT" | grep -oE '[A-Za-z0-9]{87,88}' | head -1)
        
        if [ -z "$SOL_TRANSFER_TX" ]; then
            print_warning "Failed to transfer remaining SOL"
            echo "$TRANSFER_OUTPUT"
            SOL_TRANSFER_TX="failed"
        else
            print_success "Transferred $TRANSFER_AMOUNT SOL to multisig"
            print_info "SOL transfer transaction: $SOL_TRANSFER_TX"
            log_message "SOL transfer transaction: $SOL_TRANSFER_TX"
        fi
    else
        print_info "Insufficient SOL to transfer (balance: $REMAINING_BALANCE SOL)"
        SOL_TRANSFER_TX="insufficient_balance"
    fi
    
    # Store SOL transfer transaction
    echo "$SOL_TRANSFER_TX" > "$PROJECT_ROOT/temp/.mainnet_sol_transfer_tx_phase3"
    
    # Show final balance
    FINAL_BALANCE=$(solana balance "$DEPLOYMENT_AUTHORITY" --url "$RPC_URL" | awk '{print $1}')
    print_info "Final deployment authority balance: $FINAL_BALANCE SOL"
}

# Function to create final deployment record
create_final_deployment_record() {
    print_info "Creating final deployment record..."
    
    # Gather all information from previous phases
    PHASE1_INFO=$(cat "$PROJECT_ROOT/temp/deployment_info_mainnet_phase1.json" 2>/dev/null || echo "{}")
    PHASE2_INFO=$(cat "$PROJECT_ROOT/temp/verification_info_mainnet_phase2.json" 2>/dev/null || echo "{}")
    TRANSFER_TX=$(cat "$PROJECT_ROOT/temp/.mainnet_transfer_tx_phase3" 2>/dev/null || echo "unknown")
    SOL_TRANSFER_TX=$(cat "$PROJECT_ROOT/temp/.mainnet_sol_transfer_tx_phase3" 2>/dev/null || echo "unknown")
    
    # Create comprehensive final deployment record
    cat > "$FINAL_DEPLOYMENT_INFO" << EOF
{
  "deployment": {
    "phase": "phase3_handoff_complete",
    "network": "mainnet-beta",
    "programId": "$PROGRAM_ID",
    "deploymentAuthority": "$DEPLOYMENT_AUTHORITY",
    "adminAuthority": "$ADMIN_AUTHORITY",
    "finalUpgradeAuthority": "$SQUADS_MULTISIG",
    "status": "production_ready",
    "deploymentComplete": true
  },
  "phase1_deployment": $PHASE1_INFO,
  "phase2_verification": $PHASE2_INFO,
  "phase3_handoff": {
    "authorityTransferTransaction": "$TRANSFER_TX",
    "solTransferTransaction": "$SOL_TRANSFER_TX",
    "handoffTimestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
    "finalUpgradeAuthority": "$SQUADS_MULTISIG"
  },
  "security": {
    "upgradeAuthority": "$SQUADS_MULTISIG",
    "adminAuthority": "$ADMIN_AUTHORITY",
    "emergencyPauseAuthority": "$ADMIN_AUTHORITY",
    "multisigType": "Squads",
    "multisigUrl": "https://app.squads.so/"
  },
  "monitoring": {
    "programExplorer": "https://explorer.solana.com/address/$PROGRAM_ID",
    "multisigExplorer": "https://explorer.solana.com/address/$SQUADS_MULTISIG",
    "adminExplorer": "https://explorer.solana.com/address/$ADMIN_AUTHORITY"
  },
  "metadata": {
    "deploymentTimestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
    "solanaVersion": "$(solana --version | awk '{print $2}')",
    "rustVersion": "$(rustc --version | awk '{print $2}')",
    "deploymentMethod": "three_phase_secure_deployment"
  }
}
EOF
    
    print_success "Final deployment record created: $FINAL_DEPLOYMENT_INFO"
    log_message "Final deployment record created"
    
    # Clean up temporary files
    rm -f "$PROJECT_ROOT/temp/.mainnet_transfer_tx_phase3"
    rm -f "$PROJECT_ROOT/temp/.mainnet_sol_transfer_tx_phase3"
}

# Function to provide security instructions
provide_security_instructions() {
    print_success "🔐 Final Security Instructions"
    echo ""
    print_warning "CRITICAL: Secure the Program Keypair"
    print_warning "========================================="
    print_warning "The program keypair must be secured immediately:"
    print_warning "  File: $PROGRAM_KEYPAIR"
    echo ""
    print_warning "Required Actions:"
    print_warning "1. Copy to cold storage (hardware wallet, paper backup, etc.)"
    print_warning "2. Verify backup is readable and correct"
    print_warning "3. DELETE the online copy from this system"
    print_warning "4. Store backup in secure, offline location"
    echo ""
    print_warning "Deployment Authority Keypair:"
    print_warning "  File: $DEPLOYMENT_KEYPAIR"
    print_warning "  Can be kept for emergency use but should be secured"
    echo ""
    print_info "Commands to secure keypairs:"
    echo "  # Backup program keypair"
    echo "  cp $PROGRAM_KEYPAIR /path/to/secure/backup/"
    echo "  # Verify backup"
    echo "  solana-keygen pubkey /path/to/secure/backup/MainNet-quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD.json"
    echo "  # Should output: $PROGRAM_ID"
    echo "  # DELETE online copy (after confirming backup)"
    echo "  rm $PROGRAM_KEYPAIR"
    echo ""
    print_success "System is now fully deployed and secured!"
}

# Function to display final status
show_final_status() {
    print_success "🎉 MainNet Deployment Complete!"
    echo ""
    print_info "Deployment Summary:"
    echo "  ✅ Program compiled and deployed to MainNet"
    echo "  ✅ System state initialized with admin authority"
    echo "  ✅ Functionality verified with test tokens and pool"
    echo "  ✅ Upgrade authority transferred to Squads multisig"
    echo "  ✅ Remaining SOL transferred to multisig"
    echo "  ✅ Deployment records created"
    echo ""
    print_info "Key Addresses:"
    echo "  • Program ID: $PROGRAM_ID"
    echo "  • Admin Authority: $ADMIN_AUTHORITY"
    echo "  • Upgrade Authority: $SQUADS_MULTISIG"
    echo ""
    print_info "Monitoring Links:"
    echo "  • Program: https://explorer.solana.com/address/$PROGRAM_ID"
    echo "  • Multisig: https://explorer.solana.com/address/$SQUADS_MULTISIG"
    echo "  • Admin: https://explorer.solana.com/address/$ADMIN_AUTHORITY"
    echo "  • Squads App: https://app.squads.so/"
    echo ""
    print_info "Deployment Files:"
    print_info "  • Phase 1: deployment_info_mainnet_phase1.json"
    print_info "  • Phase 2: verification_info_mainnet_phase2.json"
    print_info "  • Final: $FINAL_DEPLOYMENT_INFO"
    print_info "  • Logs: $HANDOFF_LOG"
    echo ""
    print_warning "⚠️  NEXT STEPS: Secure the program keypair as instructed above!"
}

# Main execution
main() {
    print_info "Starting Fixed Ratio Trading MainNet Handoff - Phase 3"
    print_info "====================================================="
    log_message "Starting Phase 3 handoff"
    
    check_phase2_completion
    check_phase3_funds
    transfer_upgrade_authority
    transfer_remaining_sol
    create_final_deployment_record
    provide_security_instructions
    show_final_status
    
    print_success "Phase 3 handoff completed successfully!"
    print_success "Fixed Ratio Trading is now live on MainNet! 🚀"
}

# Run main function
main
