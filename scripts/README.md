# Scripts Directory

This directory contains utility scripts for managing the Fixed Ratio Trading project deployment and operations.

## 🚀 Deployment Scripts

### `deploy_local.sh`
Deploys the Fixed Ratio Trading program to a local Solana validator.

**Features:**
- Starts local Solana validator with proper configuration
- Builds and deploys the Rust program
- Automatically initializes the system with program authority
- Starts dashboard server on port 3000
- Configures ngrok tunnel for external access

**Usage:**
```bash
./scripts/deploy_local.sh [--reset|--noreset]
```

### `remote_build_and_deploy.sh`
Deploys the Fixed Ratio Trading program to a remote Solana validator.

**Features:**
- Targets remote validator at `http://192.168.2.88:8899`
- Builds and deploys/upgrades the program
- Automatically initializes the system for fresh deployments
- Supports both fresh deployments and upgrades

**Usage:**
```bash
./scripts/remote_build_and_deploy.sh [--reset|--noreset]
```

## 🔧 System Management Scripts

### `initialize_system.js`
**NEW:** Consolidated system initialization script that creates essential system PDAs.

**Purpose:**
- Creates SystemState PDA (global pause controls and authority management)
- Creates MainTreasury PDA (pool creation fee collection)
- Must be run by the program upgrade authority
- Required before users can create pools

**Usage:**
```bash
# Basic usage (uses default RPC and keypair)
node scripts/initialize_system.js <PROGRAM_ID>

# With custom RPC URL
node scripts/initialize_system.js <PROGRAM_ID> http://localhost:8899

# With custom RPC and keypair
node scripts/initialize_system.js <PROGRAM_ID> http://localhost:8899 ./keypair.json
```

**Examples:**
```bash
# Initialize for local deployment
node scripts/initialize_system.js 4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn

# Initialize for remote deployment
node scripts/initialize_system.js 4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn http://192.168.2.88:8899

# Initialize with custom keypair
node scripts/initialize_system.js 4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn http://localhost:8899 ./my-keypair.json
```

**Requirements:**
- Node.js installed
- `@solana/web3.js` package (`npm install @solana/web3.js`)
- Program upgrade authority keypair access
- Sufficient SOL balance (0.1+ SOL recommended)

**Architecture Note:**
System initialization is the responsibility of the program upgrade authority (deployment authority), NOT regular users. This script should only be run by whoever deployed the program.

## 🌐 Networking Scripts

### `setup_backpack_keypair.sh`
Sets up the Backpack wallet keypair for testing.

### `test_ngrok_endpoint.sh`
Tests the ngrok tunnel endpoint connectivity.

### `start_dashboard.sh`
Starts the dashboard server independently.

## 🧪 Testing Scripts

### `test_debugging_tools.sh`
Runs debugging and diagnostic tools.

### `test_tpu_vmdevbox.sh`
Tests TPU VM development box connectivity.

## 📁 Metaplex Scripts

### `metaplex/manage_metaplex.sh`
Manages deployment of Metaplex programs (Token Metadata, Candy Machine, etc.) to local validator.

**Usage:**
```bash
RPC_URL=http://192.168.2.88:8899 ./scripts/metaplex/manage_metaplex.sh
```

## 🔍 Utility Scripts

### `query_program_state.js`
Queries the current state of the Fixed Ratio Trading program.

### `update_state_from_devnet.sh`
**NEW:** Updates the dashboard state.json file with data from Solana devnet.

**Features:**
- Connects to Solana devnet and queries program state
- Retrieves all pool states, treasury state, and system state
- Updates the dashboard/state.json file with live devnet data
- Enables dashboard to display real devnet program state

**Usage:**
```bash
# Use program ID from shared-config.json
./scripts/update_state_from_devnet.sh

# Use custom program ID
./scripts/update_state_from_devnet.sh --program-id YOUR_PROGRAM_ID
```

**Requirements:**
- Node.js and npm installed
- Internet connection to access devnet
- Valid program ID (either in shared-config.json or specified via --program-id)

**Purpose:**
This script is useful when you want to view and monitor the state of your program deployed on devnet through your local dashboard. Instead of creating local test data, you can pull real state data from devnet.

---

## 🏗️ Architecture Notes

### System Initialization Flow

1. **Program Deployment**: Deploy the Fixed Ratio Trading smart contract
2. **System Initialization**: Run `initialize_system.js` with program authority
3. **User Operations**: Users can now create pools via dashboard

### Authority Hierarchy

- **Program Upgrade Authority**: Can deploy, upgrade, and initialize the system
- **System Authority**: Can pause/unpause the system (initially the upgrade authority)
- **Pool Creators**: Any user can create pools after system initialization
- **Traders**: Any user can trade in existing pools

### Security Model

- Only the program upgrade authority can initialize the system
- System initialization creates essential infrastructure PDAs
- All authority validations happen on-chain via program data account verification
- The dashboard cannot and should not allow regular users to initialize the system

---

## 📝 Prerequisites

All deployment scripts require:
- Rust and Cargo installed
- Solana CLI tools installed and configured
- Node.js for system initialization
- Sufficient SOL in the authority wallet

For local deployment:
- Available ports 8899 (validator) and 3000 (dashboard)

For remote deployment:
- Network access to the remote validator
- Properly configured Solana CLI pointing to the remote RPC 