#!/bin/bash
#filename run_integration_tests.sh
# This bash script is for running the solana-program-test code intergration_test.rs in the tests folder
# It will start a local validator, airdrop SOL to the default wallet, and run the tests
# It will then stop the validator and exit with the test result
# quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD is the program ID for the fixed-ratio-trading program

# Find the project root directory (where Cargo.toml is located)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Verify we found the correct project directory
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    echo "❌ Error: Could not find Cargo.toml in project root: $PROJECT_ROOT"
    echo "   Please run this script from the fixed-ratio-trading project directory or its subdirectories"
    exit 1
fi

echo "🧪 Running integration tests from: $PROJECT_ROOT"

# Change to project directory for cargo commands
cd "$PROJECT_ROOT"

# Start the Solana test validator in the background with increased compute budget
solana-test-validator --rpc-pubsub-enable --rpc-port 8899 --compute-unit-limit 1000000 &
VALIDATOR_PID=$!

# Wait for the validator to start
sleep 5

# Set Solana CLI to use the local validator
solana config set --url http://localhost:8899

# Airdrop SOL to the default wallet
solana airdrop 10

# Run the integration tests with increased timeout and compute budget
RUST_LOG=error cargo test

# Capture the test result
TEST_RESULT=$?

# Stop the validator
kill $VALIDATOR_PID

# Exit with the test result
exit $TEST_RESULT 