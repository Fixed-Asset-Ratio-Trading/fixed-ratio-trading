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

### Step 1: Deploy the Contract (Enhanced Script)
```bash
# Deploy to local testnet (this will now do everything automatically!)
./scripts/deploy_local.sh
```

**🎉 NEW FEATURES in deploy_local.sh:**
- ✅ **Auto-increment version** - Automatically bumps patch version on each deployment
- ✅ **Smart deployment logic** - Handles create/upgrade/redeploy/force scenarios intelligently  
- ✅ **Enhanced error handling** - No more hanging or unclear error messages
- ✅ **Automatic dashboard startup** - Launches web server and opens Firefox automatically
- ✅ **Clear status reporting** - Shows exactly what happened during deployment
- ✅ **Fixed macOS compatibility** - No more timeout command issues

### Step 2: Create Sample Pools
```bash
# Create test pools
./scripts/create_sample_pools.sh
```

### Step 3: Dashboard is Already Running!
After running `deploy_local.sh`, the dashboard should automatically open in Firefox at: **http://localhost:3000**

If it didn't open automatically:
```bash
# Manual dashboard startup (if needed)
./scripts/start_dashboard.sh
```

## 🔧 Enhanced Deployment Features

### Intelligent Deployment Logic
The enhanced `deploy_local.sh` script now handles multiple deployment scenarios:

#### 🆕 **CREATE** - First-time deployment
- Deploys new program with upgrade authority
- Sets up fresh program ID and deployment info
- Status: `Program successfully CREATED`

#### 📈 **UPGRADE** - Existing upgradeable program
- Detects existing upgradeable programs automatically
- Preserves program ID while updating contract code
- Status: `Program successfully UPGRADED`

#### 🔄 **REDEPLOY** - Non-upgradeable program replacement
- Closes old non-upgradeable program
- Deploys fresh program at same address
- Status: `Program successfully REDEPLOYED`

#### 🔧 **FORCE** - Account conflict resolution
- Uses `--force` flag to overwrite conflicting accounts
- Handles "Account already in use" errors automatically
- Status: `Program successfully CREATED (force deployment)`

#### ⚡ **NO_UPGRADE_NEEDED** - Already up-to-date
- Detects when bytecode hasn't changed
- Skips unnecessary redeployments
- Status: `No upgrade needed`

### Auto-Version Management
Every deployment automatically:
1. **Reads current version** from `Cargo.toml`
2. **Increments patch version** (e.g., 0.1.1002 → 0.1.1003)
3. **Updates Cargo.toml** with new version
4. **Builds with new version**
5. **Updates dashboard title** to show "Fixed Ratio Trading Dashboard v{version}"

### Enhanced Build Process
Build warnings have been completely eliminated:
- ✅ Added missing Solana features (`custom-heap`, `custom-panic`)
- ✅ Fixed conditional entrypoint compilation for target_os = "solana"
- ✅ Proper linting configuration for valid target_os values
- ✅ Clean compilation with zero warnings

## 🌐 Enhanced Web Dashboard

### 📊 Dynamic Version Display
The dashboard now features **live version detection**:
- 🔍 **Fetches version from smart contract** using GetVersion instruction
- 🏷️ **Updates title dynamically** to show "Fixed Ratio Trading Dashboard v{version}"
- 📡 **Real-time contract metadata** displayed in browser

### Auto-Launch Features
- 🦊 **Firefox auto-opening** on macOS (when available)
- 🌐 **Automatic web server startup** on port 3000
- 🔄 **Live reload capability** for development

### Original Dashboard Features
- **RPC Connection** - Local validator connectivity
- **Program Status** - Contract deployment status  
- **Block Height** - Current blockchain height
- **Pool Information** - Real-time pool data and metrics
- **Financial Metrics** - TVL, fees, swap activity
- **Auto-refresh** every 10 seconds with manual refresh option

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

### Program ID Management
The enhanced script automatically:
- 🔑 **Generates or reuses program keypair** from `target/deploy/fixed_ratio_trading-keypair.json`
- 📋 **Updates deployment_info.json** with all deployment details
- 🎯 **Maintains program ID consistency** across upgrades

## 🛠️ Manual Operations

### Start/Stop Validator
```bash
# Start validator manually
solana-test-validator --rpc-port 8899 --rpc-pubsub-enable --reset &

# Stop validator
pkill -f solana-test-validator
```

### Deploy Program Manually (Updated Process)
```bash
# Build the program (use modern SBF build)
cargo build-sbf

# Deploy scenarios:

# New deployment with upgrade authority
solana program deploy target/deploy/fixed_ratio_trading.so \
    --program-id target/deploy/fixed_ratio_trading-keypair.json \
    --upgrade-authority ~/.config/solana/id.json

# Upgrade existing program
solana program deploy target/deploy/fixed_ratio_trading.so \
    --program-id target/deploy/fixed_ratio_trading-keypair.json \
    --upgrade-authority ~/.config/solana/id.json

# Force deployment (conflict resolution)
solana program deploy target/deploy/fixed_ratio_trading.so \
    --program-id target/deploy/fixed_ratio_trading-keypair.json \
    --upgrade-authority ~/.config/solana/id.json \
    --force
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

#### ❌ Deployment Script Hanging (FIXED!)
**Old Problem:** Script would hang on program existence checks
**✅ Solution:** Enhanced script with proper error handling and no timeout dependencies

#### ❌ "Account already in use" Errors (FIXED!)
**Old Problem:** Deployment would fail with account conflicts
**✅ Solution:** Smart deployment logic automatically handles all conflict scenarios

#### ❌ Build Warnings (FIXED!)
**Old Problem:** Multiple compilation warnings about missing features
**✅ Solution:** Added proper Solana features and linting configuration

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

# Use the enhanced script (handles most issues automatically)
./scripts/deploy_local.sh
```

#### Dashboard Shows No Pools
1. Make sure the validator is running
2. Verify the program is deployed
3. Create test pools with `./scripts/create_sample_pools.sh`
4. Check browser console for errors

#### Dashboard Won't Auto-Open
- **Firefox not installed:** Install Firefox or open http://localhost:3000 manually
- **Port 3000 in use:** The script will automatically find an alternative port
- **Manual fallback:** Run `./scripts/start_dashboard.sh` separately

## 📁 File Structure

```
fixed-ratio-trading/
├── scripts/                     # All deployment and utility scripts
│   ├── deploy_local.sh          # ⭐ ENHANCED deployment script with auto-versioning
│   ├── start_dashboard.sh       # Web dashboard server
│   ├── create_sample_pools.sh   # Test pool creation
│   ├── monitor_pools.sh         # Command-line monitoring
│   ├── check_wallet.sh          # Wallet status and info
│   ├── run_integration_tests.sh # Test suite runner
│   ├── test_script_paths.sh     # Script portability verification
│   └── build-bpf.sh            # Enhanced BPF build script
├── dashboard/
│   ├── index.html              # Dashboard interface
│   └── dashboard.js            # ⭐ Enhanced dashboard with version detection
├── deployment_info.json        # ⭐ Enhanced deployment details (auto-generated)
├── Cargo.toml                  # ⭐ Updated with proper Solana features
└── src/                        # Contract source code with GetVersion support
```

## 🔄 Enhanced Workflow

### New Streamlined Development Workflow
```bash
1. ./scripts/deploy_local.sh          # Does EVERYTHING automatically:
   #   ✅ Auto-increments version
   #   ✅ Builds contract 
   #   ✅ Starts validator
   #   ✅ Handles smart deployment
   #   ✅ Starts dashboard
   #   ✅ Opens Firefox
   
2. ./scripts/create_sample_pools.sh   # Create test data (if needed)

3. # Develop and test your application
   # Dashboard shows live version: "Fixed Ratio Trading Dashboard v0.1.1003"

4. # Make changes and redeploy
   ./scripts/deploy_local.sh          # Automatically handles upgrade!
```

### Testing New Features
1. Make code changes
2. Run tests: `cargo test`
3. Redeploy: `./scripts/deploy_local.sh` (auto-upgrades!)
4. Verify in dashboard (version automatically updates)

## 🚫 Stopping Everything

To stop all services:

```bash
# The deployment script provides clear stop instructions:
# "To stop everything: kill [VALIDATOR_PID] [DASHBOARD_PID]"

# Quick cleanup
pkill -f solana-test-validator
pkill -f "python.*http.server.*3000"

# From deployment script terminal
Ctrl+C  # Stops both validator and dashboard automatically
```

## 📞 Support

If you encounter issues:

1. **Try the enhanced script first**: `./scripts/deploy_local.sh` handles most common issues
2. **Check terminal output**: Enhanced error messages provide clear guidance
3. **Verify prerequisites**: All required software installed and running
4. **Clean restart**: Stop all processes and restart from Step 1

## 🎯 Next Steps

Once you have the local setup working:

1. **Explore the Dashboard** - Notice the live version display and real-time metrics
2. **Test Auto-Upgrade** - Make code changes and redeploy to see smart upgrade in action
3. **Create Custom Pools** - Modify the test scripts for your use case
4. **Monitor Performance** - Use the monitoring tools to track activity
5. **Integrate Your App** - Use the free RPC methods and version detection in your application
6. **Deploy to Devnet** - When ready, adapt the enhanced script for devnet deployment

---

## 🎓 Key Learnings - Enhanced Solana Deployment

### ✅ What Now Works Perfectly (Enhanced Method)
1. **Smart Deployment**: Script automatically handles all deployment scenarios
2. **Auto-Versioning**: Every deployment increments version and updates dashboard
3. **Zero Warnings**: Clean compilation with proper Solana features
4. **Enhanced UX**: Automatic dashboard and browser launch
5. **Robust Error Handling**: Clear status messages and conflict resolution

### �� Enhanced Features
1. **Version Management**: Automatic version bumping with live dashboard display
2. **Deployment Intelligence**: CREATE → UPGRADE → REDEPLOY → FORCE as needed
3. **Build Optimization**: Zero-warning compilation with proper feature flags
4. **Dashboard Integration**: Live version fetching from smart contract
5. **Developer Experience**: One-command deployment with automatic environment setup

### 🔄 New Optimal Workflow
1. **One Command Deployment**: `./scripts/deploy_local.sh` does everything
2. **Smart Version Tracking**: Contract version automatically managed and displayed
3. **Conflict-Free Deployments**: Automatic handling of all account conflict scenarios
4. **Instant Feedback**: Clear status reporting and automatic browser launch
5. **Seamless Upgrades**: Preserves program ID while updating contract code

**🎉 Congratulations!** You now have a completely enhanced Fixed Ratio Trading deployment system with automatic versioning, intelligent deployment logic, zero build warnings, and seamless developer experience. 