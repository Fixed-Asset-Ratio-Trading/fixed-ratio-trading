# Postman Setup Guide for Fixed Ratio Trading Dashboard

## 🚀 **Installation Verification**

You should now have Postman installed at `/Applications/Postman.app`. You can launch it from:
- **Spotlight**: Press `Cmd + Space`, type "Postman"
- **Applications folder**: Open Finder → Applications → Postman.app
- **Dock**: Add to dock for quick access

## 📥 **Import the API Collection**

### 1. **Launch Postman**
- Open Postman application
- Create a free account or sign in (recommended for sync across devices)

### 2. **Import Collection**
- Click **"Import"** button (top left)
- Select **"Upload Files"**
- Navigate to: `docs/api/FixedRatioTrading_Dashboard_API.postman_collection.json`
- Click **"Import"**

### 3. **Verify Import**
You should see the collection organized as:
```
📁 Fixed Ratio Trading Dashboard API
├── 🏠 System Health
├── 🪙 1. Token Creation (Testnet Only)
├── 🏊 2. Pool Creation  
├── 💧 3. Liquidity Management
├── 🔄 4. Token Swapping
├── 💰 5. Fee Withdrawal
└── ⏸️ 6. System Pause/Unpause
```

## ⚙️ **Environment Setup**

### 1. **Create Environment**
- Click **"Environments"** tab (left sidebar)
- Click **"+"** to create new environment
- Name it: `Fixed Ratio Trading - Local`

### 2. **Set Environment Variables**
Configure these key variables:

| Variable | Initial Value | Current Value | Description |
|----------|---------------|---------------|-------------|
| `baseUrl` | `http://localhost:5000` | `http://localhost:5000` | Local development server |
| `testnet_rpc` | `https://api.testnet.solana.com` | `https://api.testnet.solana.com` | Solana testnet RPC |
| `poolId` | `POOL_ID_PLACEHOLDER` | `POOL_ID_PLACEHOLDER` | Dynamic pool ID |
| `userWallet` | `USER_WALLET_PLACEHOLDER` | `USER_WALLET_PLACEHOLDER` | Test wallet address |

| `systemAuthority` | `SYSTEM_AUTHORITY_PLACEHOLDER` | `SYSTEM_AUTHORITY_PLACEHOLDER` | System authority wallet |

### 3. **Select Environment**
- In the top-right dropdown, select your created environment
- This makes variables available to all requests

## 🧪 **Testing Workflow**

### **Phase 1: System Health Checks**
Start with basic connectivity tests:
1. **Health Check** - Verify API is running
2. **System Status** - Check if system is paused/active

### **Phase 2: Token & Pool Setup**
For development testing:
1. **Create Test Token** - Create tokens for testing
2. **Get Available Tokens** - Verify tokens are available
3. **Create Pool** - Create a test pool
4. **Get All Pools** - Verify pool creation

### **Phase 3: Core Operations**
Test main functionality:
1. **Add Liquidity** - Test liquidity provision
2. **Calculate Swap** - Test swap calculations
3. **Execute Swap** - Test actual swapping
4. **Remove Liquidity** - Test liquidity removal

### **Phase 4: Advanced Features**
Test system management:
1. **Withdraw Fees** - Test fee collection
2. **System Pause/Unpause** - Test emergency controls

## 🔧 **Development Best Practices**

### **1. Environment Management**
- **Local Development**: `http://localhost:5000`
- **Staging**: `https://staging.your-domain.com`
- **Production**: `https://api.your-domain.com`

### **2. Variable Management**
Use Postman variables for dynamic data:
```javascript
// Set pool ID from response
pm.collectionVariables.set("poolId", responseJson.poolId);

// Set transaction signature for tracking
pm.collectionVariables.set("lastTransaction", responseJson.transactionSignature);
```

### **3. Automated Testing**
Add test scripts to requests:
```javascript
// Verify successful response
pm.test("Pool created successfully", function () {
    pm.response.to.have.status(200);
    pm.expect(pm.response.json()).to.have.property('poolAddress');
});

// Extract and store values
pm.test("Extract pool ID", function () {
    const response = pm.response.json();
    pm.collectionVariables.set("poolId", response.poolId);
});
```

### **4. Request Organization**
- **Folders**: Organize by MVP feature
- **Descriptions**: Document each endpoint's purpose
- **Examples**: Save successful responses as examples

## 📊 **Collection Features**

### **Pre-request Scripts**
Automatically sets up dynamic variables:
- Pool ID placeholders
- User wallet placeholders
- Timestamp generation

### **Test Scripts**
Global validation for all requests:
- Response time validation (< 5 seconds)
- Status code validation (no 4xx/5xx errors)
- Custom tests per endpoint

### **Variable Scoping**
- **Collection Variables**: Shared across all requests
- **Environment Variables**: Environment-specific settings
- **Global Variables**: Cross-collection sharing

## 🐛 **Debugging Tips**

### **1. Response Inspection**
- **Body tab**: View JSON responses
- **Headers tab**: Check response headers
- **Cookies tab**: Inspect session cookies
- **Test Results**: View test outcomes

### **2. Console Debugging**
- Open Postman Console (View → Show Postman Console)
- Add console logs in scripts:
```javascript
console.log("Pool ID:", pm.collectionVariables.get("poolId"));
console.log("Response:", pm.response.json());
```

### **3. Network Issues**
Common problems and solutions:
- **CORS Errors**: Configure API to allow localhost origins
- **Port Issues**: Verify ASP.NET is running on port 5000
- **SSL Issues**: Use HTTP for local development

## 🔄 **Integration with Development**

### **1. API Documentation**
- Export collection as documentation
- Share with team members
- Keep updated with API changes

### **2. CI/CD Integration**
Use Newman (Postman CLI) for automated testing:
```bash
# Install Newman
npm install -g newman

# Run collection
newman run docs/api/FixedRatioTrading_Dashboard_API.postman_collection.json \
  --environment docs/api/Local-Environment.json \
  --reporters cli,html
```

### **3. Mock Servers**
Create mock responses for frontend development:
- Right-click collection → "Mock Collection"
- Use mock URL for frontend testing
- Develop UI before backend is ready

## 📝 **Maintenance**

### **Regular Updates**
- Update collection when APIs change
- Add new endpoints as features develop
- Keep environment variables current

### **Version Control**
- Export collections regularly
- Store in project repository
- Track changes with meaningful commits

### **Team Collaboration**
- Share collections via Postman workspace
- Use team environments for shared testing
- Document API changes in collection descriptions

---

## 🚀 **Next Steps**

1. **Import the collection** into Postman
2. **Set up your environment** with local variables
3. **Start with health checks** to verify connectivity
4. **Follow the testing workflow** as you develop features
5. **Customize requests** as your API evolves

This setup provides a solid foundation for testing all 7 MVP features of your Fixed Ratio Trading Dashboard as you build them out with ASP.NET Core! 