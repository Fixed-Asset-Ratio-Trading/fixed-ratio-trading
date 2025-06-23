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

### Apple Silicon (M1/M2/M3) Requirements
If you're using an Apple Silicon Mac, you may encounter compatibility issues with the standard Solana toolchain. We provide two deployment options:

#### Option A: Docker-based Deployment (Recommended for Apple Silicon)
- **Docker Desktop** (latest version)
- **8GB+ RAM** available for Docker
- **ARM64 Linux container support**

#### Option B: Native Deployment (May have issues on Apple Silicon)
- Standard Rust/Solana installation
- Potential compatibility workarounds required

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

# Install Docker Desktop (macOS - for Apple Silicon compatibility)
brew install --cask docker
```

## 🚀 Quick Start

### For Standard Systems (Intel/AMD64)

#### Step 1: Deploy the Contract
```bash
# Deploy to local testnet (this will start the validator and deploy)
./scripts/deploy_local.sh
```

### For Apple Silicon (M1/M2/M3) - Docker Method ⭐ Recommended

#### Step 1: Install and Start Docker
```bash
# Install Docker Desktop
brew install --cask docker

# Start Docker Desktop
open /Applications/Docker.app
# Wait for Docker to start (whale icon in menu bar)
```

#### Step 2: Deploy with Docker
```bash
# Deploy using Docker-based compilation
./scripts/deploy_local_docker.sh
```

**First-time setup:** The initial build takes 10-15 minutes as it downloads Ubuntu 24.04 ARM64 and builds the complete Solana toolchain. Subsequent builds are much faster (30-60 seconds).

### Common Steps for All Systems

#### Step 3: Create Sample Pools
```bash
# In a new terminal, create test pools
./scripts/create_sample_pools.sh
```

#### Step 4: Launch the Dashboard
```bash
# In another terminal, start the web dashboard
./scripts/start_dashboard.sh
```

Then open your browser to: **http://localhost:3000**

## 🐳 Docker Deployment Details

### How Docker Compilation Works

**For Apple Silicon Users:** The Docker approach solves compatibility issues by:

1. **Container Environment**: Uses Ubuntu 24.04 ARM64 Linux container
2. **Isolated Toolchain**: Builds Solana from source with compatible Rust version
3. **Cross-compilation**: Your Mac runs the validator, Docker compiles the program
4. **File Sharing**: Project directory mounted into container for compilation

### Docker Build Process
```bash
# What happens during first-time Docker deployment:
1. 📦 Downloads Ubuntu 24.04 ARM64 base image (~100MB)
2. 🔧 Installs build dependencies (GCC, Clang, OpenSSL, etc.)
3. 🦀 Installs fresh Rust toolchain compatible with ARM64
4. ⚡ Clones and builds Solana toolchain from source (~10 minutes)
5. 🎯 Compiles your project using the containerized environment
6. 💾 Caches everything for faster subsequent builds

# Subsequent builds:
1. 🔄 Uses cached Docker image and dependencies
2. 🔨 Only compiles your project changes (~30-60 seconds)
3. 🚀 Deploys to your Mac's local validator
```

### Resource Requirements
- **RAM**: 6-8GB allocated to Docker
- **Disk**: 4-6GB for Docker image and cache
- **CPU**: 2-4 cores recommended for build speed
- **Time**: 10-15 minutes first build, 30-60 seconds subsequent

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

### Docker Configuration
Customize Docker resources in Docker Desktop:
```
Docker Desktop → Settings → Resources
├── Memory: 6-8GB (recommended)
├── CPUs: 4 cores (recommended)  
└── Disk: 64GB (for images and cache)
```

### Dashboard Customization
The dashboard files are in the `dashboard/` directory:
- `index.html` - Main dashboard interface
- `dashboard.js` - JavaScript logic and RPC calls

### Program ID
The contract uses a fixed Program ID: `quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD`

## 🛠️ Manual Operations

### Start/Stop Validator
```bash
# Start validator manually (with Apple Silicon compatibility)
solana-test-validator --rpc-port 8899 --rpc-pubsub-enable --no-bpf-jit --reset &

# Stop validator
pkill -f solana-test-validator
```

### Deploy Program Manually

#### Standard Deployment
```bash
# Build the program
cargo build-bpf

# Deploy to local testnet
solana program deploy target/deploy/fixed_ratio_trading.so \
    --program-id quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD \
    --url http://localhost:8899
```

#### Docker-based Build (Apple Silicon)
```bash
# Build using Docker container
docker run --rm -v "$PWD:/workspace" -w /workspace solana-m1-dev \
    cargo build-bpf --manifest-path Cargo.toml --bpf-out-dir target/deploy

# Deploy the compiled program
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

### Docker-Specific Issues

#### Docker Not Running
```bash
# Check Docker status
docker --version
docker info

# Start Docker Desktop
open /Applications/Docker.app

# Verify Docker is running
docker run hello-world
```

#### Docker Build Fails
```bash
# Check available disk space
docker system df

# Clean up Docker cache
docker system prune -a

# Rebuild Docker image
docker build -f Dockerfile.solana -t solana-m1-dev .
```

#### Out of Memory During Docker Build
```bash
# Increase Docker memory allocation
# Docker Desktop → Settings → Resources → Memory: 8GB

# Or build with reduced parallelism
docker build --memory=6g -f Dockerfile.solana -t solana-m1-dev .
```

### Apple Silicon Specific Issues

#### cargo build-bpf Fails
```bash
# Error: "lock file version 4 was found, but this version of Cargo does not understand"
# Solution: Use Docker deployment method

./scripts/deploy_local_docker.sh  # Use this instead of deploy_local.sh
```

#### BPF JIT Errors
```bash
# Error: BPF just-in-time compilation not supported
# Solution: Add --no-bpf-jit flag to validator

solana-test-validator --no-bpf-jit --rpc-port 8899 --reset &
```

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

### Performance Issues in VM
If running in a virtual machine:
- Allocate 16GB+ RAM to the VM
- Enable virtualization features
- Use Docker with reduced resource limits
- Consider native development environment for better performance

### Log Files
- **Validator logs**: Check terminal where deployment script is running
- **Dashboard logs**: Check browser developer console (F12)
- **Program logs**: Use `solana logs` command
- **Docker logs**: `docker logs <container-id>`

## 📁 File Structure

```
fixed-ratio-trading/
├── scripts/                     # All deployment and utility scripts
│   ├── deploy_local.sh          # Standard deployment script
│   ├── deploy_local_docker.sh   # Docker-based deployment (Apple Silicon)
│   ├── start_dashboard.sh       # Web dashboard server
│   ├── create_sample_pools.sh   # Test pool creation
│   ├── monitor_pools.sh         # Command-line monitoring
│   ├── check_wallet.sh          # Wallet status and info
│   ├── run_integration_tests.sh # Test suite runner
│   └── test_script_paths.sh     # Script portability verification
├── Dockerfile.solana            # Ubuntu 24.04 ARM64 container for Solana
├── dashboard/
│   ├── index.html              # Dashboard interface
│   └── dashboard.js            # Dashboard logic
├── deployment_info.json        # Deployment details (auto-generated)
└── src/                        # Contract source code
```

## 🔄 Regular Workflow

### Daily Development Workflow

#### Standard Systems
```bash
1. ./scripts/deploy_local.sh          # Start validator & deploy
2. ./scripts/create_sample_pools.sh   # Create test data
3. ./scripts/start_dashboard.sh       # Open dashboard
4. # Develop and test your application
5. # Monitor with dashboard or CLI tools
```

#### Apple Silicon Systems
```bash
1. open /Applications/Docker.app      # Ensure Docker is running
2. ./scripts/deploy_local_docker.sh   # Docker-based deployment
3. ./scripts/create_sample_pools.sh   # Create test data  
4. ./scripts/start_dashboard.sh       # Open dashboard
5. # Develop and test your application
```

### Testing New Features
1. Make code changes
2. Run tests: `cargo test`
3. Redeploy: `./scripts/deploy_local.sh` or `./scripts/deploy_local_docker.sh`
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

# Stop Docker containers (if using Docker deployment)
docker stop $(docker ps -q --filter ancestor=solana-m1-dev)
```

## 🎯 Deployment Method Comparison

| Feature | Standard Deployment | Docker Deployment |
|---------|-------------------|------------------|
| **Compatibility** | Intel/AMD64 ✅ | Intel/AMD64 ✅<br/>Apple Silicon ✅ |
| **Setup Time** | 1-2 minutes | 10-15 min (first time)<br/>30-60 sec (subsequent) |
| **Resource Usage** | Low | Higher (Docker overhead) |
| **Reliability** | Good on compatible systems | Excellent on all systems |
| **Isolation** | Uses system toolchain | Containerized environment |
| **Recommended For** | Intel Macs, Linux, Windows | Apple Silicon Macs, VMs |

## 📞 Support

If you encounter issues:

1. **Check your system**: Intel/AMD64 → use standard deployment, Apple Silicon → use Docker
2. **Verify prerequisites**: All required software installed and running
3. **Check logs**: Terminal output, browser console, Docker logs
4. **Try alternative method**: Switch between standard and Docker deployment
5. **Clean restart**: Stop all processes and restart from Step 1

## 🎯 Next Steps

Once you have the local setup working:

1. **Explore the Dashboard** - Understand all the metrics displayed
2. **Create Custom Pools** - Modify the test scripts for your use case
3. **Monitor Performance** - Use the monitoring tools to track activity
4. **Integrate Your App** - Use the free RPC methods in your application
5. **Deploy to Devnet** - When ready, deploy to Solana devnet

---

**🎉 Congratulations!** You now have a fully functional Fixed Ratio Trading deployment with comprehensive monitoring capabilities, optimized for your system architecture. 