# Metaplex Local Development Setup

This directory contains scripts and tools for setting up Metaplex programs on your local Solana validator, enabling full token metadata functionality in your development environment.

## 🎯 **Purpose**

The Metaplex programs provide token metadata functionality, allowing tokens to have:
- ✅ **Names and Symbols**: Proper token names instead of `TOKEN-ABC123`
- ✅ **Descriptions**: Rich token descriptions
- ✅ **Images**: Token logos and artwork
- ✅ **Metadata**: Additional token properties

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

# Stop/cleanup
./scripts/metaplex.sh stop

# Restart
./scripts/metaplex.sh restart
```

## 📊 **Programs Deployed**

| Program | ID | Purpose |
|---------|----|---------| 
| Token Metadata | `metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s` | Token names, symbols, descriptions |
| Candy Machine | `cndy3Z4yapfJBmL3ShUp5exZKqR3z33thTzeNMm2gRZ` | NFT creation and minting |
| Auction House | `hausS13jsjafwWwGqZTUQRmWyvyxn9EQpqMwV1PBBmk` | NFT marketplace functionality |

## 🔧 **Configuration**

### **Environment Variables**
- `RPC_URL` - Solana RPC endpoint (default: `http://localhost:8899`)
- `WEBSOCKET_URL` - Solana WebSocket endpoint (default: `ws://localhost:8900`)

### **Directories Created**
- `.metaplex/` - Main Metaplex directory (git-ignored)
- `.metaplex/programs/` - Downloaded program binaries
- `.metaplex/logs/` - Log files
- `.metaplex/metaplex.pid` - Process tracking file

## ✅ **Prerequisites**

1. **Solana CLI**: Installed and configured
2. **Local Validator**: Running at `http://localhost:8899`
3. **Internet Connection**: For downloading program binaries (first time only)
4. **curl**: For downloading programs and health checks

## 🔍 **How It Works**

1. **Downloads**: Gets official Metaplex program binaries from GitHub releases
2. **Deploys**: Uses `solana program deploy` with correct Program IDs
3. **Verifies**: Checks that programs are accessible via RPC
4. **Tracks**: Maintains state in `.metaplex/metaplex.pid`

## 🐛 **Troubleshooting**

### **"Program not deployed" Error**
```bash
# Check validator is running
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

The `remote_build_and_deploy.sh` script automatically:
1. ✅ **Checks** if Metaplex programs are deployed
2. ✅ **Deploys** them if missing
3. ✅ **Continues** with normal deployment if successful
4. ✅ **Warns** if deployment fails but continues

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

- `/dashboard/token-creation.js` - Token creation with metadata
- `/scripts/remote_build_and_deploy.sh` - Main deployment script
- `/.gitignore` - Excludes `.metaplex/` directory 