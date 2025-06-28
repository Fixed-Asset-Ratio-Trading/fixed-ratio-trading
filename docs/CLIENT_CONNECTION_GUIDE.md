# Solana Validator Client Connection Guide

## 🌐 Network Architecture Overview

Your Solana validator provides multiple connection endpoints with different protocols:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Client Connections                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  🌐 HTTPS RPC (Port 443)     🔒 SSL Certificate Required        │
│  └─ https://192.168.9.81     └─ *.dcs1.cc wildcard cert        │
│      │                                                          │
│      ▼                                                          │
│  📡 Nginx Reverse Proxy  ────────────────────────────────────▶  │
│      │                                                          │
│      ▼                                                          │
│  🔗 HTTP RPC (Port 8899)     🚫 No SSL                         │
│  └─ http://192.168.9.81:8899                                   │
│                                                                 │
│  ⚡ TPU (Port 1026)          🚫 No SSL (UDP Protocol)          │
│  └─ 192.168.9.81:1026                                          │
│                                                                 │
│  🔌 WebSocket (Port 8900)    🔒 SSL via Nginx WSS              │
│  └─ wss://192.168.9.81/ws                                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 📋 Connection Endpoints Summary

| Service | Protocol | Endpoint | SSL/Certificate | Use Case |
|---------|----------|----------|-----------------|----------|
| **RPC** | HTTPS | `https://192.168.9.81` | ✅ Required | Wallet connections, dApp frontend |
| **RPC** | HTTP | `http://192.168.9.81:8899` | ❌ None | Local development, internal tools |
| **TPU** | UDP | `192.168.9.81:1026` | ❌ None | High-performance transaction submission |
| **WebSocket** | WSS | `wss://192.168.9.81/ws` | ✅ Required | Real-time subscriptions |
| **Gossip** | UDP | `192.168.9.81:8003` | ❌ None | Validator network communication |

## 🔧 Client Configuration Examples

### 1. **Backpack Wallet Configuration**
```
RPC URL: https://192.168.9.81
WebSocket URL: wss://192.168.9.81/ws
Network: Custom
```

### 2. **Solana CLI Configuration**
```bash
# For HTTPS (production)
solana config set --url https://192.168.9.81

# For HTTP (development)
solana config set --url http://192.168.9.81:8899
```

### 3. **JavaScript/TypeScript (web3.js)**
```javascript
import { Connection } from '@solana/web3.js';

// HTTPS connection (for production)
const connection = new Connection('https://192.168.9.81', 'confirmed');

// HTTP connection (for development)
const connection = new Connection('http://192.168.9.81:8899', 'confirmed');
```

### 4. **Rust Client**
```rust
use solana_client::rpc_client::RpcClient;

// HTTPS connection
let rpc_client = RpcClient::new("https://192.168.9.81");

// HTTP connection
let rpc_client = RpcClient::new("http://192.168.9.81:8899");
```

### 5. **High-Performance TPU Client (Rust)**
```rust
use solana_client::tpu_client::TpuClient;
use solana_client::rpc_client::RpcClient;

let rpc_client = RpcClient::new("http://192.168.9.81:8899");
let tpu_client = TpuClient::new(
    rpc_client,
    &websocket_url,
    solana_client::tpu_client::TpuClientConfig::default(),
).unwrap();
```

## ❌ Common SSL Certificate Issues & Solutions

### Issue 1: "SSL Certificate Error" with TPU
**Problem**: Trying to use SSL/TLS with TPU connections
```
❌ Wrong: https://192.168.9.81:1026 (TPU doesn't use HTTPS)
✅ Right: 192.168.9.81:1026 (Raw UDP connection)
```

### Issue 2: "Certificate doesn't match hostname"
**Problem**: Using domain name that doesn't resolve in DNS
```
❌ Wrong: https://vmdevbox1.dcs1.cc (DNS doesn't resolve)
✅ Right: https://192.168.9.81 (Direct IP with certificate)
```

### Issue 3: "Wrong port for HTTPS"
**Problem**: Trying to use HTTPS on RPC port 8899
```
❌ Wrong: https://192.168.9.81:8899 (Port 8899 is HTTP only)
✅ Right: https://192.168.9.81 (Port 443 via nginx)
```

## 🛠️ DNS Configuration (Optional)

To use `vmdevbox1.dcs1.cc` instead of IP addresses, add to your client's `/etc/hosts`:

```bash
# Add this line to /etc/hosts on client machines
192.168.9.81 vmdevbox1.dcs1.cc

# Then you can use:
# https://vmdevbox1.dcs1.cc
# wss://vmdevbox1.dcs1.cc/ws
```

## 🔍 Testing & Verification

### Test RPC Connections
```bash
# Test HTTPS RPC (with certificate)
curl -k https://192.168.9.81 -X POST -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'

# Test HTTP RPC (no certificate)
curl http://192.168.9.81:8899 -X POST -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
```

### Test TPU Endpoints
```bash
# Check TPU endpoint information
curl -s http://192.168.9.81:8899 -X POST -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"getClusterNodes"}' | jq '.result[] | {rpc: .rpc, tpu: .tpu, gossip: .gossip}'
```

### Test WebSocket
```bash
# Install wscat if needed: npm install -g wscat
wscat -c wss://192.168.9.81/ws
```

## 🚨 Security Considerations

1. **Production Use**: Always use HTTPS endpoints (`https://192.168.9.81`)
2. **Development Use**: HTTP endpoints are acceptable (`http://192.168.9.81:8899`)
3. **TPU Connections**: Always UDP, never encrypted
4. **Certificate Validation**: Use `-k` flag in curl only for testing

## 📞 Support

If you encounter certificate issues:

1. **Check endpoint protocol**: TPU = UDP (no SSL), RPC = HTTP/HTTPS
2. **Verify port numbers**: 443 (HTTPS), 8899 (HTTP), 1026 (TPU)
3. **Test with curl**: Use examples above to isolate issues
4. **Check certificate**: `openssl x509 -in ../dcs1/dcs1.crt -text -noout`

---

**Certificate Info**: Wildcard `*.dcs1.cc` (Valid until March 2026)
**Validator Version**: Solana 2.2.18 (Agave)
**Last Updated**: June 2025 