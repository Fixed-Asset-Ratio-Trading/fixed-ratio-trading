# Fixed Ratio Trading Dashboard API Documentation

## Overview
The Fixed Ratio Trading Dashboard provides REST API endpoints for managing and monitoring liquidity pools on Solana blockchain. The API is organized into two main controllers:

- **Pool Controller** (`/api/pool`) - Pool operations and data retrieval
- **System Controller** (`/api/system`) - System monitoring and health checks

All API responses follow a consistent format with a `success` field and standardized error handling.

## Base URL
```
http://localhost:5000/api  (Development)
https://your-domain.com/api  (Production)
```

## Response Format
All API responses follow this structure:
```json
{
  "success": true,       // Indicates whether the API request was processed successfully
  "data": { /* response data */ },  // Main response payload containing requested information
  "pagination": { /* only for paginated endpoints */ },  // Pagination metadata for list endpoints
  "error": "error message"  // Error description (only when success is false)
}
```

---

## Pool Status System

The API uses a simplified `status` field that clearly indicates what operations are allowed on each pool. This combines all pause/active states into one easy-to-understand field.

### Pool Status Values
| Status | Description | Operations Allowed |
|--------|-------------|-------------------|
| `Operational` | Pool is fully functional | All operations (swaps, liquidity) |
| `Inactive` | Pool is deprecated/failed in database | None |
| `SystemPaused` | Entire system is paused | None |
| `PoolPaused` | This specific pool is paused by owner | None |
| `SwapsPaused` | Only swaps are paused | Liquidity operations only |

### Usage Example
```javascript
if (pool.status === "Operational") {
  // All operations available
} else if (pool.status === "SwapsPaused") {
  // Only liquidity operations available
} else {
  // No operations available
}
```

---

## Pool Controller Endpoints

### 1. Get All Pools
**GET** `/api/pool`

Retrieve all pools with optional filtering and pagination. Results are sorted by creation date (newest to oldest).

#### Query Parameters
| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `network` | string | No | null | Filter by network (testnet, mainnet, devnet) |
| `isActive` | boolean | No | null | Filter by active status |
| `page` | integer | No | 1 | Page number for pagination |
| `pageSize` | integer | No | 20 | Page size (max: 100) |

#### Response Structure
```json
{
  // Indicates whether the API request was processed successfully
  "success": true,
  
  // Array of pool summary objects containing essential pool information
  "data": [
    {
      // Unique identifier for the pool in the database
      "id": "123e4567-e89b-12d3-a456-426614174000",
      
      // The Solana program-derived address (PDA) of this pool on the blockchain
      "poolAddress": "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU",
      
      // Mint address of the first token on the Solana blockchain
      "tokenAMint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
      
      // Mint address of the second token on the Solana blockchain
      "tokenBMint": "So11111111111111111111111111111111111111112",
      
      // Symbol of the first token in the trading pair (e.g., "BTC", "SOL")
      "tokenASymbol": "BTC",
      
      // Symbol of the second token in the trading pair (e.g., "USDC", "ETH")
      "tokenBSymbol": "USDC",
      
      // Full display name of the first token (e.g., "Bitcoin")
      "tokenAName": "Bitcoin",
      
      // Full display name of the second token (e.g., "USD Coin")
      "tokenBName": "USD Coin",
      
      // Human-readable ratio string in format: "1 (Symbol of base) = ratio (Symbol of multiple)"
      // When tokenAIsTheMultiple=false: TokenA=base, TokenB=multiple, display "1 TokenA = ratio TokenB"
      // Example: "1 BTC = 10000 USDC" means 1 unit of base token equals ratio units of multiple token
      "ratioDisplay": "1 BTC = 10000 USDC",
        
      // Trading ratio representing how many units of TokenA per 1 unit of TokenB
      "ratio": 10000,
      
      // Whether TokenA is the multiple token in the trading pair calculation
      // When true: TokenA is multiple (abundant), TokenB is base (valuable), display "1 TokenB = ratio TokenA"
      // When false: TokenA is base (valuable), TokenB is multiple (abundant), display "1 TokenA = ratio TokenB"  
      // Used to construct correct human-readable ratio strings
      "tokenAIsTheMultiple": false,
      
      // Current total liquidity amount of TokenA in the pool (in smallest token units, e.g., satoshis for BTC)
      "totalTokenALiquidity": 50000000000,
      
      // Current total liquidity amount of TokenB in the pool (in smallest token units, e.g., micro-USDC for USDC)
      "totalTokenBLiquidity": 1000000000,
      
      // Total trading volume of TokenA that has passed through this pool since creation
      "totalVolumeTokenA": 500000000000,
      
      // Total trading volume of TokenB that has passed through this pool since creation
      "totalVolumeTokenB": 10000000000,
      
      // Pool operational status combining all pause/active states
      "status": "Operational",
      
      // Human-readable description of the current pool status
      "statusDescription": "Pool is fully operational",
      
      // UTC timestamp when this pool was created on the blockchain
      "createdAt": "2024-01-15T10:30:00Z",
      
      // UTC timestamp when pool data was last synchronized from the blockchain
      "lastUpdated": "2024-01-20T14:45:30Z",
      
      // Blockchain network where this pool exists (mainnet-beta, testnet, or devnet)
      "network": "testnet"
    }
  ],
  
  // Pagination information for navigating through multiple pages of results
  "pagination": {
    // Current page number being returned (1-based indexing)
    "currentPage": 1,
    
    // Number of pools returned per page (maximum 100, default 20)
    "pageSize": 20,
    
    // Total number of pools matching the search criteria across all pages
    "totalCount": 156,
    
    // Total number of pages available based on totalCount and pageSize
    "totalPages": 8
  }
}
```

### 2. Get Pool by ID
**GET** `/api/pool/{id}`

Retrieve detailed information for a specific pool.

#### Path Parameters
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | GUID | Yes | Pool unique identifier |

#### Response Structure
```json
{
  // Indicates whether the API request was processed successfully
  "success": true,
  
  // Detailed pool information object
  "data": {
    // Unique identifier for the pool in the database
    "id": "123e4567-e89b-12d3-a456-426614174000",
    
    // The Solana program-derived address (PDA) of this pool on the blockchain
    "poolAddress": "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU",
    
    // Pool owner (creator) public key - READ-ONLY field for display purposes
    "owner": "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
    
    // First token mint address (TokenA) on the Solana blockchain
    "tokenAMint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    
    // Second token mint address (TokenB) on the Solana blockchain
    "tokenBMint": "So11111111111111111111111111111111111111112",
    
    // TokenA vault PDA address where TokenA liquidity is stored
    "tokenAVault": "4qeKyDVLWAdxKzd3h3VAaKNJa8Z4fF8k9LL8LqGBRKhe",
    
    // TokenB vault PDA address where TokenB liquidity is stored
    "tokenBVault": "8FfMrLPjqCkHSBRJrqTz4Z3vYr8LKtR1QDpbsRVYGwc",
    
    // LP Token A mint address for liquidity providers in TokenA
    "lpTokenAMint": "LPAbc123def456ghi789jkl012mno345pqr678stu",
    
    // LP Token B mint address for liquidity providers in TokenB
    "lpTokenBMint": "LPBdef456ghi789jkl012mno345pqr678stu901vwx",
    
    // Symbol of the first token (e.g., "BTC", "SOL")
    "tokenASymbol": "BTC",
    
    // Symbol of the second token (e.g., "USDC", "ETH")
    "tokenBSymbol": "USDC",
    
    // Full display name of the first token
    "tokenAName": "Bitcoin",
    
    // Full display name of the second token
    "tokenBName": "USD Coin",
    
    // Human-readable ratio string (e.g., "1 BTC = 10000 USDC")
    "ratioDisplay": "1 BTC = 10000 USDC",
    
    // Trading ratio representing how many units of TokenA per 1 unit of TokenB
    "ratio": 10000,
    
    // Whether TokenA is the multiple token in the trading pair calculation
    // When true: TokenA is multiple (abundant), TokenB is base (valuable), display "1 TokenB = ratio TokenA"
    // When false: TokenA is base (valuable), TokenB is multiple (abundant), display "1 TokenA = ratio TokenB"
    // Used to construct correct human-readable ratio strings
    "tokenAIsTheMultiple": false,
    
    // Current total liquidity amount of TokenA in the pool (smallest units)
    "totalTokenALiquidity": 50000000000,
    
    // Current total liquidity amount of TokenB in the pool (smallest units)
    "totalTokenBLiquidity": 1000000000,
    
    // Total trading volume of TokenA since pool creation
    "totalVolumeTokenA": 500000000000,
    
    // Total trading volume of TokenB since pool creation
    "totalVolumeTokenB": 10000000000,
    
    // Pool operational status combining all pause/active states
    "status": "SwapsPaused",
    
    // Human-readable description of the current pool status
    "statusDescription": "Swaps are paused - liquidity operations available",
    
    // Collected fees in TokenA awaiting withdrawal by owner (READ-ONLY, in smallest units)
    "collectedFeesTokenA": 100000000,
    
    // Collected fees in TokenB awaiting withdrawal by owner (READ-ONLY, in smallest units)
    "collectedFeesTokenB": 2000000,
    
    // Current swap fee rate in basis points (e.g., 30 = 0.3%) (READ-ONLY)
    "swapFeeBasisPoints": 30,
    
    // Collected SOL fees in lamports (READ-ONLY)
    "collectedSolFees": 50000000,
    
    // Number of unique addresses that have provided liquidity to this pool
    "uniqueLiquidityProviders": 25,
    
    // UTC timestamp when this pool was created
    "createdAt": "2024-01-15T10:30:00Z",
    
    // UTC timestamp when pool data was last synchronized from blockchain
    "lastUpdated": "2024-01-20T14:45:30Z",
    
    // Blockchain network where this pool exists
    "network": "testnet",
    
    // Array of recent transactions for this pool (up to 10 most recent)
    "recentTransactions": [
      {
        // Unique identifier for the transaction in the database
        "id": "456e7890-f12b-34c5-d678-901234567890",
        
        // Type of transaction (1=Swap, 2=AddLiquidity, 3=RemoveLiquidity, 7=PoolCreation)
        "type": 1,
        
        // Human-readable transaction type (e.g., "Swap", "AddLiquidity")
        "typeDisplay": "Swap",
        
        // Solana transaction signature for this transaction
        "transactionSignature": "4Kj8B3mN9pQ2rF5xL7cV1tE6wG8hS4dA2zM9nP1qR5xL",
        
        // Public key of the user who initiated this transaction
        "userAddress": "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
        
        // Amount of TokenA involved in this transaction (0 if not applicable, in smallest units)
        "tokenAAmount": 1000000000,
        
        // Amount of TokenB involved in this transaction (0 if not applicable, in smallest units)
        "tokenBAmount": 10000000,
        
        // Amount of LP tokens involved (for liquidity operations, in smallest units)
        "lpTokenAmount": 0,
        
        // UTC timestamp when this transaction was processed on the blockchain
        "processedAt": "2024-01-20T14:30:00Z",
        
        // Whether the transaction completed successfully
        "isSuccessful": true,
        
        // Error message if transaction failed (null if successful)
        "errorMessage": null,
        
        // Gas fees paid for this transaction (in lamports)
        "gasFee": 5000,
        
        // Exchange rate at time of swap (TokenA per TokenB, null for non-swap transactions)
        "swapPrice": 100.50,
        
        // Human-readable description of what this transaction accomplished
        "description": "Swapped 0.01 BTC for 1005.0 USDC"
      }
    ]
  }
}
```

### 3. Get Pool by Address
**GET** `/api/pool/address/{address}`

Retrieve pool information using blockchain address.

#### Path Parameters
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `address` | string | Yes | Pool blockchain address |

#### Response Structure
Same as Get Pool by ID - returns detailed pool information with the `status` field.

### 4. Search Pools
**GET** `/api/pool/search`

Search pools by token symbols, names, or token pairs. Supports both individual token search and token pair search with slash notation. Results are sorted by creation date (newest to oldest).

#### Query Parameters
| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `q` | string | Yes | - | Search query - supports individual tokens ("BTC") or token pairs ("BTC/USDC", "USDC/BTC") |
| `network` | string | No | null | Network filter |
| `page` | integer | No | 1 | Page number |
| `pageSize` | integer | No | 20 | Page size |

#### Search Query Examples
- **Individual token symbols**: `"BTC"`, `"USDC"`, `"SOL"`
- **Individual token names**: `"Bitcoin"`, `"USD Coin"`
- **Token pairs (order independent)**: `"BTC/USDC"`, `"USDC/BTC"`, `"SOL/USDT"`
- **Pool addresses**: Partial or full blockchain addresses

**Note**: Token pair searches work regardless of how pools are stored in the database. Pools are internally stored in lexicographic order for technical consistency, but the API supports searching by the user-friendly display format (most valuable token first) as defined in UX_DESIGN_TOKEN_PAIR_DISPLAY guidelines.

#### Response Structure
```json
{
  // Indicates whether the search was processed successfully
  "success": true,
  
  // Array of pool summary objects matching the search criteria
  // Search supports:
  // - Individual token symbols: "BTC", "USDC", "SOL"
  // - Individual token names: "Bitcoin", "USD Coin"
  // - Token pairs: "BTC/USDC", "USDC/BTC" (order independent)
  // - Pool addresses: partial or full blockchain addresses
  "data": [
    // Pool objects with same structure as Get All Pools response
  ],
  
  // Pagination information (same structure as Get All Pools)
  "pagination": {
    "currentPage": 1,
    "pageSize": 20,
    "totalCount": 42,
    "totalPages": 3
  },
  
  // The search query that was executed against token symbols, names, and token pairs
  "query": "BTC/USDC"
}
```

### 5. Get Pool Statistics
**GET** `/api/pool/statistics`

Retrieve aggregated statistics across all pools.

#### Query Parameters
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `network` | string | No | Network filter (optional) |

#### Response Structure
```json
{
  // Indicates whether statistics were retrieved successfully
  "success": true,
  
  // Aggregated statistics object
  "data": {
    // Total number of pools across all networks (or filtered network)
    "totalPools": 156,
    
    // Number of pools currently accepting transactions
    "activePools": 142,
    
    // Number of pools that are paused (either globally or swaps-only)
    "pausedPools": 8,
    
    // Total value locked across all pools (sum of all token liquidity in smallest units)
    "totalValueLocked": 500000000000,
    
    // Total trading volume in the last 24 hours (in smallest units)
    "volume24h": 10000000000,
    
    // Number of unique user addresses that traded in the last 24 hours
    "uniqueUsers24h": 1247,
    
    // Total number of transactions across all pools since inception
    "totalTransactions": 45623,
    
    // UTC timestamp when these statistics were last calculated
    "lastUpdated": "2024-01-20T14:45:30Z"
  }
}
```

### 6. Get Pool Transactions
**GET** `/api/pool/{id}/transactions`

Retrieve recent transactions for a specific pool.

#### Path Parameters
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | GUID | Yes | Pool unique identifier |

#### Query Parameters
| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `limit` | integer | No | 50 | Max transactions to return (max: 100) |

#### Response Structure
```json
{
  // Indicates whether transactions were retrieved successfully
  "success": true,
  
  // Array of transaction objects for this pool
  "data": [
    {
      // Unique identifier for the transaction in the database
      "id": "456e7890-f12b-34c5-d678-901234567890",
      
      // Type of transaction as enum value (1=Swap, 2=AddLiquidity, 3=RemoveLiquidity, 7=PoolCreation)
      "type": 1,
      
      // Human-readable transaction type string
      "typeDisplay": "Swap",
      
      // Solana transaction signature for verification on blockchain
      "transactionSignature": "4Kj8B3mN9pQ2rF5xL7cV1tE6wG8hS4dA2zM9nP1qR5xL",
      
      // Public key of the user who initiated this transaction
      "userAddress": "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
      
      // Amount of TokenA involved (0 if not applicable, in smallest token units)
      "tokenAAmount": 1000000000,
      
      // Amount of TokenB involved (0 if not applicable, in smallest token units)
      "tokenBAmount": 10000000,
      
      // Amount of LP tokens minted/burned (for liquidity operations, in smallest units)
      "lpTokenAmount": 0,
      
      // UTC timestamp when transaction was processed on blockchain
      "processedAt": "2024-01-20T14:30:00Z",
      
      // Whether the transaction completed successfully on blockchain
      "isSuccessful": true,
      
      // Error message if transaction failed (null if successful)
      "errorMessage": null,
      
      // Gas fees paid for this transaction (in lamports)
      "gasFee": 5000,
      
      // Exchange rate at time of swap (TokenA per TokenB, null for non-swap transactions)
      "swapPrice": 100.50,
      
      // Human-readable description of what this transaction accomplished
      "description": "Swapped 0.01 BTC for 1005.0 USDC"
    }
  ],
  
  // The pool ID these transactions belong to
  "poolId": "123e4567-e89b-12d3-a456-426614174000",
  
  // The limit parameter that was applied to this query
  "limit": 50
}
```

### 7. Sync Pool (Manual Refresh)
**POST** `/api/pool/sync/{address}`

Manually trigger synchronization of pool data from blockchain.

#### Path Parameters
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `address` | string | Yes | Pool blockchain address |

#### Response Structure
```json
{
  // Indicates whether the sync operation was initiated successfully
  "success": true,
  
  // Sync operation result object
  "data": {
    // Whether the blockchain sync operation completed successfully
    "success": true,
    
    // Error message if sync failed (null if successful)
    "errorMessage": null,
    
    // UTC timestamp when the sync operation was performed
    "syncedAt": "2024-01-20T14:45:30Z",
    
    // The updated pool data after sync (includes status field)
    "pool": {
      // Full pool details object (same as Get Pool by ID response data)
    }
  },
  
  // Human-readable message about the sync operation result
  "message": "Pool synchronized successfully"
}
```

### 8. Get Top Pools
**GET** `/api/pool/top`

Retrieve top-performing pools by various criteria.

#### Query Parameters
| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `sortBy` | string | No | "volume" | Sort criteria: "volume", "liquidity", "recent" |
| `count` | integer | No | 10 | Number of pools to return (max: 50) |
| `network` | string | No | null | Network filter (optional) |

#### Response Structure
```json
{
  // Indicates whether top pools were retrieved successfully
  "success": true,
  
  // Array of top pool summary objects (includes status field)
  "data": [
    // Pool summary objects ordered by the specified criteria
  ],
  
  // The sorting criteria that was applied ("volume", "liquidity", or "recent")
  "sortBy": "volume",
  
  // The number of pools returned
  "count": 10,
  
  // The network filter applied (null if no filter)
  "network": null
}
```

---

## System Controller Endpoints

### 1. Get System State
**GET** `/api/system/state/{network}`

Retrieve system state for a specific network.

#### Path Parameters
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `network` | string | Yes | Network name (testnet, mainnet-beta, devnet) |

#### Response Structure
```json
{
  // Indicates whether system state was retrieved successfully
  "success": true,
  
  // System state information object
  "data": {
    // Unique identifier for this system state record in the database
    "id": "789e0123-f45b-67c8-d901-234567890123",
    
    // Public key of the authority that can pause/unpause the system (READ-ONLY)
    "authority": "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
    
    // Whether the entire system is currently paused (READ-ONLY)
    "isPaused": false,
    
    // Unix timestamp when system was paused (0 if not paused, READ-ONLY)
    "pauseTimestamp": 0,
    
    // Human-readable reason for system pause (empty if not paused, READ-ONLY)
    "pauseReason": "",
    
    // Blockchain network this system state applies to
    "network": "testnet",
    
    // UTC timestamp when this state record was last updated in dashboard
    "updatedAt": "2024-01-20T14:45:30Z",
    
    // UTC timestamp of last synchronization with blockchain
    "lastSyncAt": "2024-01-20T14:45:30Z",
    
    // Last blockchain slot number that was synchronized
    "lastSyncSlot": 123456789,
    
    // Transaction signature of the last system operation (null if none)
    "lastOperationTxSignature": null,
    
    // Type of last system operation (null if none, values: 1=Pause, 2=Unpause, 3=Upgrade, 4=EmergencyStop, 5=Configuration)
    "lastOperationType": null
  }
}
```

### 2. System Health Check
**GET** `/api/system/health`

Retrieve comprehensive health status of all system components.

#### Response Structure
```json
{
  // Indicates whether health check completed (may be false if critical failure)
  "success": true,
  
  // Health status information object
  "data": {
    // Overall system health status ("healthy", "degraded", or "unhealthy")
    "status": "healthy",
    
    // UTC timestamp when health check was performed
    "timestamp": "2024-01-20T14:45:30Z",
    
    // Individual service health status
    "services": {
      // Database connectivity health
      "database": {
        // Whether database connection is working
        "healthy": true,
        
        // Human-readable status message
        "message": "Database connection successful"
      },
      
      // Solana RPC connection health
      "solanaRpc": {
        // Whether Solana RPC connection is working
        "healthy": true,
        
        // Human-readable status message
        "message": "Solana RPC connection successful",
        
        // Which network the RPC is connected to
        "network": "testnet"
      },
      
      // Polling service health
      "polling": {
        // Whether polling service is running properly
        "healthy": true,
        
        // Human-readable status message
        "message": "Polling service is running",
        
        // Whether the polling service is currently active
        "isRunning": true
      }
    }
  }
}
```

**Note**: If any service is unhealthy, the endpoint returns HTTP 503 (Service Unavailable) instead of 200.

### 3. Get Polling Statistics
**GET** `/api/system/polling/statistics`

Retrieve detailed statistics about the polling service.

#### Response Structure
```json
{
  // Indicates whether polling statistics were retrieved successfully
  "success": true,
  
  // Polling service statistics object
  "data": {
    // Whether the polling service is currently running
    "isRunning": true,
    
    // Polling service configuration
    "configuration": {
      // How often the polling service runs (TimeSpan format)
      "pollInterval": "00:01:00",
      
      // Number of pools processed in each batch
      "batchSize": 100,
      
      // Number of concurrent requests allowed
      "concurrentRequests": 5
    },
    
    // Detailed runtime statistics
    "statistics": {
      // Total number of polling cycles completed since service start
      "totalPollCycles": 1000,
      
      // Number of polling cycles that completed successfully
      "successfulCycles": 995,
      
      // Number of polling cycles that failed
      "failedCycles": 5,
      
      // Average time each polling cycle takes (TimeSpan format)
      "averageCycleTime": "00:00:30",
      
      // UTC timestamp of last successful polling cycle
      "lastSuccessfulPoll": "2024-01-20T14:45:30Z",
      
      // Number of consecutive failed cycles (0 indicates healthy)
      "consecutiveFailures": 0,
      
      // Total number of pools synchronized since service start
      "poolsSynced": 500,
      
      // Total number of transactions synchronized since service start
      "transactionsSynced": 10000,
      
      // Number of new pools discovered and added since service start
      "newPoolsDiscovered": 25,
      
      // Total runtime since polling service started (TimeSpan format)
      "totalRuntime": "7.12:30:00"
    }
  }
}
```

### 4. Trigger Manual Polling
**POST** `/api/system/polling/trigger`

Manually trigger a polling cycle.

#### Response Structure
```json
{
  // Indicates whether the polling trigger was successful
  "success": true,
  
  // Human-readable message about the operation
  "message": "Polling cycle triggered successfully",
  
  // UTC timestamp when the trigger was initiated
  "timestamp": "2024-01-20T14:45:30Z"
}
```

**Note**: Returns HTTP 400 (Bad Request) if polling service is not running.

### 5. Get System Configuration
**GET** `/api/system/config`

Retrieve system configuration (safe values only, no sensitive data).

#### Response Structure
```json
{
  // Indicates whether configuration was retrieved successfully
  "success": true,
  
  // System configuration object (sensitive values excluded)
  "data": {
    // Solana blockchain configuration
    "solana": {
      // Which Solana network the system is connected to
      "network": "testnet"
    },
    
    // Polling service configuration
    "polling": {
      // Whether polling service is currently running
      "isRunning": true,
      
      // Polling configuration details
      "configuration": {
        // How often polling cycles run (TimeSpan format)
        "pollInterval": "00:01:00",
        
        // Number of pools processed per batch
        "batchSize": 100
      }
    },
    
    // Version and environment information
    "version": {
      // API version string
      "api": "1.0.0",
      
      // Runtime environment (Development, Staging, Production)
      "environment": "Development"
    }
  }
}
```

### 6. Get System Metrics
**GET** `/api/system/metrics`

Retrieve comprehensive system performance metrics.

#### Response Structure
```json
{
  // Indicates whether metrics were retrieved successfully
  "success": true,
  
  // System performance metrics object
  "data": {
    // Total time the system has been running (TimeSpan format)
    "uptime": "7.12:30:00",
    
    // Polling service performance metrics
    "polling": {
      // Total polling cycles completed
      "totalCycles": 1000,
      
      // Number of successful polling cycles
      "successfulCycles": 995,
      
      // Number of failed polling cycles
      "failedCycles": 5,
      
      // Average time per polling cycle (TimeSpan format)
      "averageCycleTime": "00:00:30",
      
      // UTC timestamp of last successful poll
      "lastSuccessfulPoll": "2024-01-20T14:45:30Z",
      
      // Current consecutive failure count (0 = healthy)
      "consecutiveFailures": 0
    },
    
    // Data synchronization metrics
    "synchronization": {
      // Total pools synchronized from blockchain
      "poolsSynced": 500,
      
      // Total transactions synchronized from blockchain
      "transactionsSynced": 10000,
      
      // New pools discovered and added to database
      "newPoolsDiscovered": 25
    },
    
    // UTC timestamp when these metrics were calculated
    "lastUpdated": "2024-01-20T14:45:30Z"
  }
}
```

---

## Error Handling

All endpoints use consistent error handling:

### HTTP Status Codes
- `200` - Success (operation completed successfully)
- `400` - Bad Request (invalid parameters or request format)
- `404` - Not Found (requested resource doesn't exist)
- `500` - Internal Server Error (unexpected server error)
- `503` - Service Unavailable (health check failures or system down)

### Error Response Format
```json
{
  // Always false when an error occurs
  "success": false,
  
  // Human-readable error message describing what went wrong
  "error": "Error message describing what went wrong"
}
```

### Common Error Scenarios
- **Invalid GUID**: "Invalid pool ID format"
- **Pool Not Found**: "Pool not found"
- **Invalid Parameters**: "Search query is required"
- **System Down**: "Polling service is not running"
- **Network Issues**: "Solana RPC connection failed"

---

## Data Types Reference

### Pool Status Values
| Value | Enum | Description | Operations Allowed |
|-------|------|-------------|-------------------|
| `Operational` | 1 | Pool is fully functional | All operations |
| `Inactive` | 2 | Pool is deprecated/failed | None |
| `SystemPaused` | 3 | System-wide pause active | None |
| `PoolPaused` | 4 | Pool paused by owner | None |
| `SwapsPaused` | 5 | Only swaps are paused | Liquidity operations only |

### Transaction Types
| Value | Name | Description |
|-------|------|-------------|
| 1 | Swap | Token exchange between TokenA and TokenB |
| 2 | AddLiquidity | Adding liquidity to the pool |
| 3 | RemoveLiquidity | Removing liquidity from the pool |
| 7 | PoolCreation | Initial pool creation transaction |

### System Operation Types
| Value | Name | Description |
|-------|------|-------------|
| 1 | Pause | System pause operation |
| 2 | Unpause | System unpause operation |
| 3 | Upgrade | System upgrade operation |
| 4 | EmergencyStop | Emergency stop operation |
| 5 | Configuration | Configuration change operation |

### Network Types
- `testnet` - Solana testnet
- `mainnet-beta` - Solana mainnet
- `devnet` - Solana devnet

---

## Rate Limiting & Best Practices

1. **Pagination**: Use pagination for list endpoints to avoid large responses
2. **Caching**: Pool data is cached and updated via polling service every minute
3. **Manual Sync**: Use sync endpoints sparingly as they perform direct blockchain calls
4. **Health Monitoring**: Monitor `/api/system/health` regularly for system status
5. **Error Handling**: Always check the `success` field before processing `data`
6. **Pool Status**: Use the `status` field to determine allowed operations
7. **Timeouts**: API calls timeout after 30 seconds
8. **Rate Limits**: No strict rate limits, but avoid excessive concurrent requests

---

## Example Usage

### JavaScript/Fetch API
```javascript
// Get all pools and check their status
const response = await fetch('/api/pool?page=1&pageSize=10&network=testnet');
const result = await response.json();

if (result.success) {
  console.log(`Found ${result.pagination.totalCount} pools`);
  result.data.forEach(pool => {
    console.log(`${pool.tokenASymbol}/${pool.tokenBSymbol}: ${pool.status} - ${pool.statusDescription}`);
    
    // Simple status checking
    if (pool.status === 'Operational') {
      console.log('✅ Can perform all operations');
    } else if (pool.status === 'SwapsPaused') {
      console.log('⚠️ Only liquidity operations available');
    } else {
      console.log('❌ No operations available');
    }
  });
} else {
  console.error('Error:', result.error);
}

// Get pool details with status
const poolResponse = await fetch('/api/pool/123e4567-e89b-12d3-a456-426614174000');
const poolResult = await poolResponse.json();

if (poolResult.success) {
  const pool = poolResult.data;
  console.log(`Pool ${pool.tokenASymbol}/${pool.tokenBSymbol}`);
  console.log(`Status: ${pool.status} - ${pool.statusDescription}`);
  console.log(`Liquidity: ${pool.totalTokenALiquidity} / ${pool.totalTokenBLiquidity}`);
  
  // Check what operations are allowed
  switch (pool.status) {
    case 'Operational':
      console.log('All operations available: swap, add/remove liquidity');
      break;
    case 'SwapsPaused':
      console.log('Liquidity operations only: add/remove liquidity');
      break;
    default:
      console.log('No operations available');
  }
}

// Check system health
const healthResponse = await fetch('/api/system/health');
const healthResult = await healthResponse.json();

if (healthResult.success && healthResult.data.status === 'healthy') {
  console.log('All systems operational');
} else {
  console.warn('System health issues detected');
}
```

### cURL Examples
```bash
# Get all pools with status field
curl "http://localhost:5000/api/pool?page=1&pageSize=20"

# Search for operational BTC pools
curl "http://localhost:5000/api/pool/search?q=BTC&network=testnet"

# Get pool statistics
curl "http://localhost:5000/api/pool/statistics?network=testnet"

# Check system health
curl "http://localhost:5000/api/system/health"

# Trigger manual polling
curl -X POST "http://localhost:5000/api/system/polling/trigger"
``` 