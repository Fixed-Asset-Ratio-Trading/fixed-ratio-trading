# Metaplex Local Development Setup

This directory contains scripts and tools for setting up Metaplex programs on your local Solana validator, enabling full token metadata functionality in your development environment.

## 🎯 **Purpose**

The Metaplex programs provide token metadata functionality, allowing tokens to have:
- ✅ **Names and Symbols**: Proper token names instead of `TOKEN-ABC123`
- ✅ **Descriptions**: Rich token descriptions
- ✅ **Images**: Token logos and artwork
- ✅ **Metadata**: Additional token properties

## ⚠️ Critical Notes (Canonical ID & Preloading)

- Always use the canonical Token Metadata Program ID: `metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s`.
- Do NOT deploy a custom Token Metadata Program. Instead, preload the canonical binary at the canonical address when starting the validator using `--bpf-program`.
- The `manage_metaplex.sh` script is updated to skip on-chain deployment of Token Metadata and to write the canonical ID into `shared-config.json` only.
- Ensure your validator start command includes:

  ```bash
  solana-test-validator \
    --rpc-port 8899 \
    --bind-address 0.0.0.0 \
    --bpf-program metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s ~/.metaplex/programs/mpl_token_metadata.so \
    --reset
  ```

- Without preloading, attempts to create metadata may fail with "Incorrect account owner (Custom 57)".

## 📁 **Files**

- `manage_metaplex.sh` - Main Metaplex management script
- `README.md` - This documentation

## 🚀 **Quick Start**

### **Automatic Setup (Recommended)**
The deployment script automatically manages Metaplex:
```bash
./scripts/remote_build_and_deploy.sh
```

### **Manual Management**
You can also manage Metaplex manually:

```bash
# Start Metaplex (deploy programs)
./scripts/metaplex.sh start

# Check status
./scripts/metaplex.sh status

# Safe stop (preserves deployment state)
./scripts/metaplex.sh stop

# Full reset stop (clears tracking and program IDs)
./scripts/metaplex.sh stop --reset

# Safe restart (preserves state)
./scripts/metaplex.sh restart

# Full reset restart (clears tracking, then redeploys)
./scripts/metaplex.sh restart --reset
```

## 📊 **Programs Deployed**

| Program | ID | Purpose |
|---------|----|---------| 
| Token Metadata | `metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s` | Token names, symbols, descriptions |
| Candy Machine | `cndy3Z4yapfJBmL3ShUp5exZKqR3z33thTzeNMm2gRZ` | NFT creation and minting |
| Auction House | `hausS13jsjafwWwGqZTUQRmWyvyxn9EQpqMwV1PBBmk` | NFT marketplace functionality |

## 🔧 **Configuration**

### **Safety Features**
- **Safe Stop**: Default `stop` and `restart` commands preserve deployment state
- **Reset Option**: Use `--reset` flag for full cleanup of tracking and program IDs
- **Smart Detection**: Script detects existing programs and avoids unnecessary redeployment

### **Environment Variables**
- `RPC_URL` - Solana RPC endpoint (default: `http://localhost:8899` for local development)
- `WEBSOCKET_URL` - Solana WebSocket endpoint (default: `ws://localhost:8900` for local development)

### **Directories Created**
- `.metaplex/` - Main Metaplex directory (git-ignored)
- `.metaplex/programs/` - Downloaded program binaries
- `.metaplex/logs/` - Log files
- `.metaplex/metaplex.pid` - Process tracking file

## ✅ **Prerequisites**

1. **Solana CLI**: Installed and configured
2. **Solana Validator**: Running at `http://localhost:8899` (local)
3. **Internet Connection**: For downloading program binaries (first time only)
4. **curl**: For downloading programs and health checks

## 🔍 **How It Works**

1. **Downloads**: Dumps the Token Metadata binary from mainnet to `.metaplex/programs/mpl_token_metadata.so` (once).
2. **Preloading (Required)**: The validator must be started with `--bpf-program` to preload the Token Metadata binary at the canonical ID.
3. **Config Writes**: `manage_metaplex.sh` writes the canonical ID into `shared-config.json` only.
4. **Status**: Status checks verify accessibility and print the configured program IDs.

## 🐛 **Troubleshooting**

### **"Program not deployed" Error**
```bash
# Check validator is running (local)
curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' http://localhost:8899

# Restart Metaplex
./scripts/metaplex.sh restart
```

### **"Download failed" Error**
- Check internet connection
- Verify GitHub releases are accessible
- Try running with verbose output: `bash -x ./scripts/metaplex.sh start`

### **"Authority not found" Error**
- Ensure your Solana keypair is configured: `solana config get`
- Check keypair has sufficient SOL: `solana balance`

## 🔄 **Integration with Deployment**

- The `remote_build_and_deploy.sh` script now forces canonical Metaplex IDs into configs and does not auto-discover/override them.
- Update `start_production_validator.sh` to include the `--bpf-program` preloading flag (see Critical Notes). This ensures the RPC uses the canonical program at genesis without on-chain deploy.

## 📝 **Development Benefits**

### **Before Metaplex (Local)**
- Tokens show as `TOKEN-ABC123`
- No metadata support
- Poor wallet display
- Inconsistent with production

### **After Metaplex (Local)**
- ✅ Tokens show proper names (`TS`, `MST`)
- ✅ Full metadata support
- ✅ Rich wallet display with images
- ✅ Identical to mainnet/devnet behavior

## 🎯 **Use Cases**

- **Token Testing**: Create tokens with full metadata
- **Wallet Integration**: Test how tokens appear in wallets
- **Production Parity**: Ensure local matches mainnet behavior
- **Development**: Build features that rely on token metadata

## 🔗 **Related Files**

  (dashboard UI paths have been removed from this repo)
- `/scripts/remote_build_and_deploy.sh` - Main deployment script
- `/.gitignore` - Excludes `.metaplex/` directory 