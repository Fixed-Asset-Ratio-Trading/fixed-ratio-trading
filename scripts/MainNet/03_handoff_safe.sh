#!/bin/bash

# Fixed Ratio Trading - SAFE MainNet Handoff Script (Phase 3)
# Uses Squad's SAT (Safe Authority Transfer) feature to prevent authority loss

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROGRAM_ID="quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD"
DEPLOYMENT_AUTHORITY="3Li1ktauXzse1oHueYDAkD1d4o25u11jBT2yY61w4XbB"
ADMIN_AUTHORITY="4ekSqR4pNZ5hp4cRyicji1Yj7ZCphgkYQhwZf2ib9Wko"
SQUAD_ADDRESS="i8g7KyNHCv6MT8yD6R1TuPAap2VgUAm6r6uohc9vhMi"
RPC_URL="https://api.mainnet-beta.solana.com"
PROJECT_ROOT="/Users/davinci/code/fixed-ratio-trading"

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

print_critical() {
    echo -e "${RED}${YELLOW}[CRITICAL]${NC} $1"
}

# Function to verify prerequisites
verify_prerequisites() {
    print_info "Verifying deployment prerequisites..."
    
    # Check if Phase 2 completed
    PHASE2_INFO="$PROJECT_ROOT/temp/verification_info_mainnet_phase2.json"
    if [ ! -f "$PHASE2_INFO" ]; then
        print_error "Phase 2 verification not found. Run ./scripts/MainNet/02_verify.sh first"
        exit 1
    fi
    
    PHASE2_STATUS=$(jq -r '.status' "$PHASE2_INFO")
    if [ "$PHASE2_STATUS" != "phase2_complete_ready_for_handoff" ]; then
        print_error "Phase 2 not completed. Status: $PHASE2_STATUS"
        exit 1
    fi
    
    # Verify current upgrade authority
    CURRENT_AUTHORITY=$(solana program show "$PROGRAM_ID" --url "$RPC_URL" | grep "Authority" | awk '{print $2}')
    if [ "$CURRENT_AUTHORITY" != "$DEPLOYMENT_AUTHORITY" ]; then
        print_error "Unexpected upgrade authority: $CURRENT_AUTHORITY"
        print_error "Expected: $DEPLOYMENT_AUTHORITY"
        exit 1
    fi
    
    print_success "Prerequisites verified - ready for safe handoff"
}

# Function to provide SAT instructions
provide_sat_instructions() {
    echo ""
    print_critical "🛡️  SAFE AUTHORITY TRANSFER INSTRUCTIONS"
    echo "========================================================"
    echo ""
    print_warning "🚨 CRITICAL: Do NOT use manual authority transfer commands!"
    print_warning "Use Squad's SAT (Safe Authority Transfer) feature instead."
    echo ""
    
    print_info "📋 SAT Process Checklist:"
    echo ""
    echo "□ Step 1: Open Squad Interface"
    echo "   🔗 Go to: https://app.squads.so/squads/$SQUAD_ADDRESS/home"
    echo ""
    echo "□ Step 2: Navigate to Programs"
    echo "   • Click on 'Programs' section in Squad interface"
    echo ""
    echo "□ Step 3: Add Your Program"
    echo "   • Click 'Add Program' button"
    echo "   • Enter Program ID: $PROGRAM_ID"
    echo "   • Confirm program details"
    echo ""
    echo "□ Step 4: Create SAT (Safe Authority Transfer)"
    echo "   • Click 'Create SAT' button"
    echo "   • Squad will automatically calculate correct Vault PDA"
    echo "   • Review transaction details carefully"
    echo ""
    echo "□ Step 5: Get Multisig Signatures"
    echo "   • Share transaction with Squad members"
    echo "   • Collect required signatures based on Squad threshold"
    echo ""
    echo "□ Step 6: Execute SAT Transaction"
    echo "   • Execute when sufficient signatures obtained"
    echo "   • Wait for transaction confirmation"
    echo ""
    echo "□ Step 7: Verify Transfer"
    echo "   • Run verification command provided below"
    echo ""
}

# Function to provide verification commands
provide_verification_commands() {
    echo ""
    print_info "🔍 Verification Commands:"
    echo ""
    echo "After SAT execution, verify the transfer:"
    echo ""
    echo "# Check current upgrade authority"
    echo "solana program show $PROGRAM_ID --url $RPC_URL"
    echo ""
    echo "Expected output should show:"
    echo "  Upgrade Authority: [SQUAD_VAULT_PDA_ADDRESS]"
    echo "  (NOT the Squad address: $SQUAD_ADDRESS)"
    echo ""
    print_warning "⚠️  The upgrade authority should be a different address (Vault PDA)"
    print_warning "    If it shows $SQUAD_ADDRESS, the transfer was INCORRECT!"
    echo ""
}

# Function to provide safety reminders
provide_safety_reminders() {
    echo ""
    print_critical "🚨 SAFETY REMINDERS:"
    echo "===================="
    echo ""
    print_error "❌ NEVER run these dangerous commands:"
    echo "   solana program set-upgrade-authority $PROGRAM_ID \\"
    echo "     --new-upgrade-authority $SQUAD_ADDRESS"
    echo ""
    print_error "❌ NEVER transfer authority directly to Squad address"
    echo "   Squad Address: $SQUAD_ADDRESS"
    echo ""
    print_success "✅ ALWAYS use Squad's SAT feature"
    print_success "✅ Squad handles Vault PDA derivation automatically"
    print_success "✅ No risk of wrong address with SAT"
    echo ""
}

# Function to provide post-transfer instructions
provide_post_transfer_instructions() {
    echo ""
    print_info "📝 After Successful SAT Transfer:"
    echo ""
    echo "1. Test upgrade capability via Squad interface"
    echo "2. Verify all Squad members can participate in governance"
    echo "3. Document the final Vault PDA address"
    echo "4. Secure your deployment keypair"
    echo "5. Test emergency pause functionality (admin authority)"
    echo ""
    print_info "🎯 Your program will be safely controlled by Squad multisig"
    print_info "🔒 Admin authority remains with hardware wallet: $ADMIN_AUTHORITY"
    echo ""
}

# Function to show key information
show_key_information() {
    echo ""
    print_info "🔑 Key Information:"
    echo "==================="
    echo "Program ID: $PROGRAM_ID"
    echo "Squad Address: $SQUAD_ADDRESS"
    echo "Admin Authority: $ADMIN_AUTHORITY"
    echo "Deployment Authority: $DEPLOYMENT_AUTHORITY"
    echo ""
    print_info "🔗 Important Links:"
    echo "Squad Interface: https://app.squads.so/squads/$SQUAD_ADDRESS/home"
    echo "Program Explorer: https://explorer.solana.com/address/$PROGRAM_ID"
    echo "Squad Explorer: https://explorer.solana.com/address/$SQUAD_ADDRESS"
    echo ""
}

# Main execution
main() {
    echo ""
    print_success "🛡️  SAFE MAINNET HANDOFF - PHASE 3"
    echo "=================================================="
    print_info "Using Squad's SAT (Safe Authority Transfer) Feature"
    echo ""
    
    verify_prerequisites
    show_key_information
    provide_sat_instructions
    provide_verification_commands
    provide_safety_reminders
    provide_post_transfer_instructions
    
    echo ""
    print_success "🎉 Safe handoff instructions provided!"
    print_warning "⚠️  Follow the SAT process above to safely transfer authority"
    print_info "📖 See docs/deploy/SAFE_SQUAD_DEPLOYMENT_TESTING.md for details"
    echo ""
}

# Run main function
main
