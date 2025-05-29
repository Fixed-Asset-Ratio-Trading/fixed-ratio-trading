#!/bin/bash

# Start the Solana test validator in the background
solana-test-validator &
VALIDATOR_PID=$!

# Wait for the validator to start
sleep 5

# Set Solana CLI to use the local validator
solana config set --url http://localhost:8899

# Airdrop SOL to the default wallet
solana airdrop 10

# Run the integration tests
cargo test -- --test-threads=1

# Capture the test result
TEST_RESULT=$?

# Stop the validator
kill $VALIDATOR_PID

# Exit with the test result
exit $TEST_RESULT 