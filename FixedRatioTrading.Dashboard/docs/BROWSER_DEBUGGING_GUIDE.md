# Browser & API Debugging Guide
**Fixed Ratio Trading Dashboard - Development Debugging Tools**

## 🛠️ **Installed Debugging Tools**

### **Browsers with Developer Tools**
- ✅ **Google Chrome** - Industry standard dev tools
- ✅ **Firefox** - Standard version with good debugging
- ✅ **Firefox Developer Edition** - Enhanced dev tools with superior network monitoring
- ✅ **Safari** - Native macOS browser with WebKit dev tools

### **Command-Line Debugging Tools**
- ✅ **jq** - JSON processor for pretty-printing API responses
- ✅ **HTTPie** - User-friendly HTTP client for API testing
- ✅ **curl** - Standard HTTP client (built-in)
- ✅ **Postman** - GUI API testing platform

## 🌐 **Quick Start Guide**

### **1. Start ASP.NET Core Application**
```bash
cd src/FixedRatioTrading.Dashboard.Web
dotnet run --environment Development
```

### **2. Open Firefox Developer Edition**
```bash
open -a "Firefox Developer Edition"
# Navigate to: http://localhost:5000
# Press F12 to open DevTools
```

### **3. Test API with HTTPie**
```bash
# Test health endpoint
http GET localhost:5000/health

# Test API endpoints as they're developed
http GET localhost:5000/api/pools
```

### **4. Debug JSON with jq**
```bash
# Pretty-print API responses
curl -s localhost:5000/api/pools | jq '.'

# Extract specific fields
curl -s localhost:5000/api/pools | jq '.[].displayPair'
```

## 🧪 **Testing Commands**

### **Run Debugging Test Script**
```bash
./scripts/test_debugging_tools.sh
```

### **Import Postman Collection**
1. Open Postman
2. Import: `docs/api/FixedRatioTrading_Dashboard_API.postman_collection.json`
3. Create environment with `baseUrl: http://localhost:5000`

## 🎯 **Integration Benefits**

This debugging setup perfectly complements our server-side C# development:

1. **Server-Side Debugging**: Visual Studio/VS Code breakpoints in C#
2. **Client-Side Debugging**: Browser DevTools for minimal JavaScript
3. **API Testing**: HTTPie/Postman for endpoint validation
4. **Performance Monitoring**: Both server-side metrics and client-side timing

The combination gives us **complete visibility** into both the server-side ASP.NET Core application and the client-side user interactions!
