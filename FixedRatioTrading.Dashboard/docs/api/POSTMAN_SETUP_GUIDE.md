# Postman Setup Guide for Fixed Ratio Trading Dashboard

## 🚀 Installation Verification

You should now have Postman installed at `/Applications/Postman.app`. 

## 📥 Import the API Collection

1. **Launch Postman** from Applications or Spotlight
2. **Import Collection**:
   - Click "Import" button (top left)
   - Select "Upload Files"
   - Navigate to: `docs/api/FixedRatioTrading_Dashboard_API.postman_collection.json`
   - Click "Import"

## ⚙️ Environment Setup

1. **Create Environment** named: `Fixed Ratio Trading - Local`
2. **Set Variables**:
   - `baseUrl`: `http://localhost:5000`
   - `testnet_rpc`: `https://api.testnet.solana.com`

## 🧪 Testing Workflow

### Phase 1: System Health
- Health Check - Verify API is running

### Phase 2: Development Testing
- Create Test Token - For testnet development
- Get All Pools - Verify pool creation

### Phase 3: Core Operations
- Add/Remove Liquidity - Test pool operations
- Token Swapping - Test exchange functionality

## 🔧 Development Tips

- Use environment variables for dynamic data
- Add test scripts to verify responses
- Save successful responses as examples
- Keep collection updated as API evolves

## 🚀 Next Steps

1. Import the collection into Postman
2. Set up your local environment
3. Start with health checks
4. Follow testing workflow as you build features

The collection will be expanded as we implement the 7 MVP features!
