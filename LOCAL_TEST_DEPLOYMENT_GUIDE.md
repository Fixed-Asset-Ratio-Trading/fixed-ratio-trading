# 🚀 Fixed Ratio Trading - Local Deployment Guide

This guide will help you deploy the Fixed Ratio Trading contract to a local Solana testnet and monitor it with a comprehensive web dashboard.

## 📋 Prerequisites

Before starting, make sure you have the following installed:

### Required Software
- **Rust & Cargo** (latest stable version)
- **Solana CLI** (v1.18.26 or later)
- **Python 3** (for web server)
- **curl** (for API calls)
- **jq** (for JSON parsing) - Optional but recommended

### Installation Commands

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/v1.18.26/install)"

# Install jq (Ubuntu/Debian)
sudo apt install jq

# Install jq (macOS)
brew install jq
```

## 🚀 Quick Start

### Step 1: Deploy the Contract
```bash
# Deploy to local testnet (this will start the validator and deploy)
./scripts/deploy_local.sh
```

### Step 2: Create Sample Pools
```bash
# Create test pools
./scripts/create_sample_pools.sh
```

### Step 3: Launch the Dashboard
```bash
# Start the web dashboard
./scripts/start_dashboard.sh
```

Then open your browser to: **http://localhost:3000**

## 🌐 Web Dashboard Features

The dashboard provides real-time monitoring of:

### 📊 System Status
- **RPC Connection** - Local validator connectivity
- **Program Status** - Contract deployment status  
- **Block Height** - Current blockchain height

### 🏊‍♂️ Pool Information
- **Total Pools** - Number of active trading pools
- **Pool Liquidity** - Token amounts in each pool
- **Exchange Rates** - Current token ratios
- **Pause Status** - Which pools are active/paused

### 💰 Financial Metrics
- **Total Value Locked (TVL)** - All liquidity across pools
- **Collected Fees** - SOL and token fees earned
- **Swap Activity** - Transaction volume and frequency
- **Delegate Activity** - Governance and management actions

### 🔄 Real-time Updates
- **Auto-refresh** every 10 seconds
- **Manual refresh** button
- **Live status indicators**
- **Error handling and notifications**

## 📊 Command Line Monitoring

For command-line monitoring, use:

```bash
# Start real-time pool monitor
./scripts/monitor_pools.sh

# Monitor with custom refresh interval
./scripts/monitor_pools.sh -i 30  # Refresh every 30 seconds

# Monitor on different RPC endpoint
./scripts/monitor_pools.sh -u http://localhost:8900
```

## 🔧 Advanced Configuration

### Custom RPC Port
If you need to use a different port:

```bash
# Edit the deployment script
nano scripts/deploy_local.sh

# Change the RPC_URL and --rpc-port values
RPC_URL="http://localhost:8900"
solana-test-validator --rpc-port 8900 ...
```

### Program ID
The contract uses a fixed Program ID: `quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD`

## 🛠️ Manual Operations

### Start/Stop Validator
```bash
# Start validator manually
solana-test-validator --rpc-port 8899 --rpc-pubsub-enable --reset &

# Stop validator
pkill -f solana-test-validator
```

### Deploy Program Manually
```bash
# Build the program
cargo build-bpf

# Deploy to local testnet
solana program deploy target/deploy/fixed_ratio_trading.so \
    --program-id quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD \
    --url http://localhost:8899
```

### Create Test Pools Manually
```bash
# Run specific tests to create pools
cargo test test_initialize_pool_new_pattern --lib
cargo test test_basic_deposit_success --lib
```

## 🔍 Data Access Methods

The dashboard uses **FREE** RPC calls to read pool data:

### Free Account Reading (Used by Dashboard)
```javascript
// No cost - direct RPC account reads
const poolAccount = await connection.getAccountInfo(poolStateAddress);
const poolData = PoolState.deserialize(poolAccount.data);
```

### Pool Scanning
The dashboard automatically scans for pools by:
1. Getting all accounts owned by the program
2. Filtering for pool state accounts
3. Parsing pool data structures
4. Displaying real-time information

## 🐛 Troubleshooting

### Common Issues

#### Validator Won't Start
```bash
# Check if port is in use
lsof -i :8899

# Kill existing validator
pkill -f solana-test-validator

# Clean up test ledger
rm -rf test-ledger/
```

#### Program Deployment Fails
```bash
# Check wallet balance
solana balance

# Airdrop more SOL
solana airdrop 10

# Check Solana CLI configuration
solana config get
```

#### Dashboard Shows No Pools
1. Make sure the validator is running
2. Verify the program is deployed
3. Create test pools with `./scripts/create_sample_pools.sh`
4. Check browser console for errors

#### Web Server Won't Start
```bash
# Check if Python is installed
python3 --version

# Check if port 3000 is free
lsof -i :3000

# Use different port
cd dashboard && python3 -m http.server 3001
```

## 📁 File Structure

```
fixed-ratio-trading/
├── scripts/                     # All deployment and utility scripts
│   ├── deploy_local.sh          # Standard deployment script
│   ├── start_dashboard.sh       # Web dashboard server
│   ├── create_sample_pools.sh   # Test pool creation
│   ├── monitor_pools.sh         # Command-line monitoring
│   ├── check_wallet.sh          # Wallet status and info
│   ├── run_integration_tests.sh # Test suite runner
│   └── test_script_paths.sh     # Script portability verification
├── dashboard/
│   ├── index.html              # Dashboard interface
│   └── dashboard.js            # Dashboard logic
├── deployment_info.json        # Deployment details (auto-generated)
└── src/                        # Contract source code
```

## 🔄 Regular Workflow

### Daily Development Workflow
```bash
1. ./scripts/deploy_local.sh          # Start validator & deploy
2. ./scripts/create_sample_pools.sh   # Create test data
3. ./scripts/start_dashboard.sh       # Open dashboard
4. # Develop and test your application
5. # Monitor with dashboard or CLI tools
```

### Testing New Features
1. Make code changes
2. Run tests: `cargo test`
3. Redeploy: `./scripts/deploy_local.sh`
4. Verify in dashboard

## 🚫 Stopping Everything

To stop all services:

```bash
# Stop validator (from deployment script terminal)
Ctrl+C

# Stop dashboard (from start_dashboard.sh terminal)  
Ctrl+C

# Stop command-line monitor
Ctrl+C

# Clean up any remaining processes
pkill -f solana-test-validator
pkill -f "python.*http.server"
```

## 📞 Support

If you encounter issues:

1. **Verify prerequisites**: All required software installed and running
2. **Check logs**: Terminal output, browser console
3. **Clean restart**: Stop all processes and restart from Step 1

## 🎯 Next Steps

Once you have the local setup working:

1. **Explore the Dashboard** - Understand all the metrics displayed
2. **Create Custom Pools** - Modify the test scripts for your use case
3. **Monitor Performance** - Use the monitoring tools to track activity
4. **Integrate Your App** - Use the free RPC methods in your application
5. **Deploy to Devnet** - When ready, deploy to Solana devnet

---

**🎉 Congratulations!** You now have a fully functional Fixed Ratio Trading deployment with comprehensive monitoring capabilities. 