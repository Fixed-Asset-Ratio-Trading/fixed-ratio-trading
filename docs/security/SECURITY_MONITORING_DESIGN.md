# Security Monitoring Off-Chain Application - Design Document

**Version:** 1.0  
**Date:** August 14, 2025  
**Status:** Design Phase  

## Executive Summary

This document outlines the design for an off-chain security monitoring application for the Fixed Ratio Trading program. The initial version (V1) focuses on critical monitoring capabilities: treasury withdrawal timing validation and pool state consistency verification. The system will use Pushover for real-time notifications and operates in a read-only monitoring capacity.

## Table of Contents

1. [System Overview](#system-overview)
2. [Architecture](#architecture)
3. [Core Features - Version 1](#core-features-version-1)
4. [Implementation Details](#implementation-details)
5. [Alert Types and Thresholds](#alert-types-and-thresholds)
6. [Notification System](#notification-system)
7. [Pool Monitoring Logic](#pool-monitoring-logic)
8. [Treasury Monitoring Logic](#treasury-monitoring-logic)
9. [Future Version Recommendations](#future-version-recommendations)
10. [Security Considerations](#security-considerations)
11. [Deployment Guide](#deployment-guide)

## System Overview

### Purpose
The Security Monitoring Application is a dedicated off-chain service that continuously monitors the Fixed Ratio Trading program for:
- Unauthorized or suspicious treasury withdrawals
- Pool state inconsistencies
- Ratio violations
- Deposit/swap mismatches

### Key Principles
- **Read-Only Operations**: The monitor has no ability to modify on-chain state
- **Real-Time Monitoring**: Continuous blockchain monitoring with minimal latency
- **Selective Pool Monitoring**: Pools must be explicitly added for monitoring
- **Alert Prioritization**: Emergency vs. general notifications
- **Audit Trail**: All alerts and monitoring events are logged

## Architecture

### High-Level Architecture
```
┌─────────────────────────────────────────────────────────────┐
│                   Security Monitor Service                   │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌──────────────┐  ┌───────────────┐ │
│  │  RPC Connection │  │ State Cache  │  │  Alert Queue  │ │
│  │    Manager      │  │   Manager    │  │    Manager    │ │
│  └────────┬────────┘  └──────┬───────┘  └───────┬───────┘ │
│           │                   │                   │         │
│  ┌────────▼────────────────────▼──────────────────▼──────┐ │
│  │              Core Monitoring Engine                    │ │
│  ├────────────────────────────────────────────────────────┤ │
│  │  ┌───────────────┐  ┌────────────────┐               │ │
│  │  │   Treasury    │  │      Pool      │               │ │
│  │  │   Monitor     │  │    Monitor     │               │ │
│  │  └───────────────┘  └────────────────┘               │ │
│  └────────────────────────┬───────────────────────────────┘ │
│                           │                                  │
│  ┌────────────────────────▼────────────────────────────────┐│
│  │            Notification Service (Pushover)               ││
│  └──────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

### Components

1. **RPC Connection Manager**
   - Manages Solana RPC connections
   - Handles reconnection logic
   - Load balances across multiple RPC endpoints

2. **State Cache Manager**
   - Caches pool states for efficiency
   - Tracks historical states for comparison
   - Manages memory efficiently

3. **Alert Queue Manager**
   - Queues alerts for delivery
   - Handles retry logic
   - Prevents alert flooding

4. **Core Monitoring Engine**
   - Orchestrates monitoring activities
   - Schedules periodic checks
   - Manages monitoring rules

5. **Treasury Monitor**
   - Monitors treasury withdrawal transactions
   - Validates withdrawal timing
   - Tracks withdrawal amounts

6. **Pool Monitor**
   - Monitors pool state changes
   - Validates ratios
   - Tracks deposit/swap consistency

7. **Notification Service**
   - Integrates with Pushover API
   - Manages notification templates
   - Handles delivery confirmation

## Core Features - Version 1

### 1. Treasury Withdrawal Monitoring
- **Allowed Window**: 1st, 2nd, and 3rd of each month (GMT)
- **Emergency Alert**: Any withdrawal outside allowed window
- **General Notification**: Withdrawals within allowed window
- **Tracked Information**:
  - Withdrawal timestamp
  - Amount withdrawn
  - Recipient address
  - Transaction signature

### 2. Pool State Monitoring
- **Per-Pool Basis**: Each pool must be explicitly added
- **Ratio Validation**: Ensures pool maintains correct ratio
- **Deposit/Swap Matching**: Verifies all deposits match swaps correctly
- **Count Accuracy**: Tracks token counts remain consistent
- **Emergency Alert Triggers**:
  - Ratio deviation beyond threshold
  - Deposit/swap mismatch
  - Unexpected token count changes

### 3. Notification System
- **Platform**: Pushover
- **Alert Levels**:
  - Emergency (Priority 2, requires acknowledgment)
  - Warning (Priority 1)
  - Info (Priority 0)
- **Rate Limiting**: Prevents notification spam
- **Alert Grouping**: Groups related alerts

## Implementation Details

### Technology Stack
```yaml
Language: TypeScript/Node.js
Runtime: Node.js 18+
Key Libraries:
  - @solana/web3.js: Solana interaction
  - node-pushover: Pushover notifications
  - winston: Logging
  - node-cron: Scheduled tasks
  - redis: State caching (optional)
Database: PostgreSQL for alert history
Configuration: Environment variables + JSON config
```

### Configuration Structure
```json
{
  "monitoring": {
    "rpcEndpoints": [
      "https://api.mainnet-beta.solana.com",
      "https://solana-api.projectserum.com"
    ],
    "pollIntervalMs": 5000,
    "cacheExpiryMs": 60000
  },
  "treasury": {
    "address": "TREASURY_PUBKEY",
    "allowedWithdrawalDays": [1, 2, 3],
    "timezone": "GMT"
  },
  "pools": [
    {
      "id": "POOL_1",
      "address": "POOL_PUBKEY_1",
      "ratioThresholdPercent": 0.1,
      "enabled": true
    }
  ],
  "pushover": {
    "userKey": "PUSHOVER_USER_KEY",
    "apiToken": "PUSHOVER_API_TOKEN",
    "emergencyRetryInterval": 60,
    "emergencyExpireTime": 3600
  },
  "alerts": {
    "cooldownMinutes": 15,
    "maxAlertsPerHour": 20
  }
}
```

### Core Monitoring Loop
```typescript
interface MonitoringCycle {
  // 1. Fetch latest blockchain state
  fetchLatestState(): Promise<BlockchainState>;
  
  // 2. Compare with cached state
  detectChanges(current: BlockchainState, previous: BlockchainState): Changes[];
  
  // 3. Validate changes against rules
  validateChanges(changes: Changes[]): ValidationResult[];
  
  // 4. Generate alerts for violations
  generateAlerts(violations: ValidationResult[]): Alert[];
  
  // 5. Send notifications
  sendNotifications(alerts: Alert[]): Promise<void>;
  
  // 6. Update cache and logs
  updateState(state: BlockchainState): Promise<void>;
}
```

## Alert Types and Thresholds

### Emergency Alerts (Priority 2)
| Alert Type | Condition | Cooldown |
|------------|-----------|----------|
| Unauthorized Treasury Withdrawal | Withdrawal outside 1st-3rd GMT | None |
| Pool Ratio Violation | Ratio deviates > 0.1% | 15 min |
| Deposit/Swap Mismatch | Counts don't match after settlement | 15 min |
| Pool State Corruption | Invalid state detected | None |

### Warning Alerts (Priority 1)
| Alert Type | Condition | Cooldown |
|------------|-----------|----------|
| High Volume Activity | > 100 txs/minute on pool | 30 min |
| Treasury Withdrawal (Valid) | Within allowed window | Per event |
| Pool Approaching Ratio Limit | Within 0.05% of threshold | 60 min |

### Info Notifications (Priority 0)
| Alert Type | Condition | Cooldown |
|------------|-----------|----------|
| Monitor Started | Service initialization | N/A |
| Pool Added/Removed | Configuration change | N/A |
| Daily Summary | 00:00 GMT daily | 24 hours |

## Notification System

### Pushover Integration
```typescript
interface PushoverConfig {
  user: string;
  token: string;
  device?: string;
  title: string;
  message: string;
  priority: -2 | -1 | 0 | 1 | 2;
  expire?: number;  // For emergency alerts
  retry?: number;   // For emergency alerts
  sound?: string;   // Alert sound
  timestamp: number;
}

// Emergency Alert Example
{
  title: "🚨 EMERGENCY: Unauthorized Treasury Withdrawal",
  message: "Treasury withdrawal detected outside allowed window\n" +
           "Amount: 1000 USDC\n" +
           "Time: 2024-12-15 14:30:00 GMT\n" +
           "Tx: 3xY9k2...",
  priority: 2,
  expire: 3600,
  retry: 60,
  sound: "siren"
}
```

### Alert Templates
```typescript
const alertTemplates = {
  TREASURY_UNAUTHORIZED: {
    title: "🚨 EMERGENCY: Unauthorized Treasury Withdrawal",
    priority: 2,
    template: "Treasury withdrawal detected outside allowed window\n" +
              "Amount: {amount} {token}\n" +
              "Time: {timestamp}\n" +
              "Tx: {signature}"
  },
  POOL_RATIO_VIOLATION: {
    title: "⚠️ EMERGENCY: Pool Ratio Violation",
    priority: 2,
    template: "Pool {poolId} ratio deviation detected\n" +
              "Expected: {expectedRatio}\n" +
              "Actual: {actualRatio}\n" +
              "Deviation: {deviation}%"
  },
  TREASURY_AUTHORIZED: {
    title: "ℹ️ Treasury Withdrawal (Authorized)",
    priority: 0,
    template: "Authorized treasury withdrawal\n" +
              "Amount: {amount} {token}\n" +
              "Time: {timestamp}"
  }
};
```

## Pool Monitoring Logic

### Pool State Validation
```typescript
interface PoolValidation {
  // 1. Ratio Validation
  validateRatio(pool: PoolState): RatioValidation {
    const expectedRatio = pool.baseAmount / pool.quoteAmount;
    const actualRatio = calculateActualRatio(pool);
    const deviation = Math.abs(expectedRatio - actualRatio) / expectedRatio;
    
    return {
      valid: deviation <= pool.ratioThreshold,
      deviation,
      expectedRatio,
      actualRatio
    };
  }
  
  // 2. Deposit/Swap Consistency
  validateConsistency(pool: PoolState, transactions: Transaction[]): ConsistencyValidation {
    const deposits = filterDeposits(transactions);
    const swaps = filterSwaps(transactions);
    
    // Verify all deposits are accounted for in swaps
    const unaccountedDeposits = findUnaccountedDeposits(deposits, swaps);
    
    return {
      valid: unaccountedDeposits.length === 0,
      unaccountedDeposits
    };
  }
  
  // 3. Token Count Accuracy
  validateTokenCounts(pool: PoolState): CountValidation {
    const vaultBalances = fetchVaultBalances(pool);
    const expectedBalances = calculateExpectedBalances(pool);
    
    return {
      valid: vaultBalances.equals(expectedBalances),
      vaultBalances,
      expectedBalances
    };
  }
}
```

### Pool Monitoring Workflow
1. **Subscribe to Pool Updates**: Monitor all transactions affecting monitored pools
2. **Batch Process**: Process transactions in batches every 5 seconds
3. **State Reconstruction**: Rebuild pool state from transactions
4. **Validation**: Run all validation checks
5. **Alert Generation**: Create alerts for any violations
6. **State Caching**: Update cached pool state

## Treasury Monitoring Logic

### Treasury Withdrawal Detection
```typescript
interface TreasuryMonitor {
  // Check if withdrawal is authorized
  isAuthorizedTime(timestamp: number): boolean {
    const date = new Date(timestamp * 1000);
    const dayOfMonth = date.getUTCDate();
    return [1, 2, 3].includes(dayOfMonth);
  }
  
  // Process treasury transaction
  processTreasuryTransaction(tx: Transaction): Alert | null {
    if (!isTreasuryWithdrawal(tx)) return null;
    
    const withdrawal = parseWithdrawal(tx);
    const isAuthorized = isAuthorizedTime(withdrawal.timestamp);
    
    return {
      type: isAuthorized ? 'TREASURY_AUTHORIZED' : 'TREASURY_UNAUTHORIZED',
      priority: isAuthorized ? 0 : 2,
      data: withdrawal
    };
  }
}
```

## Future Version Recommendations

### Version 2.0 - Enhanced Monitoring
1. **MEV Detection**
   - Monitor for sandwich attacks
   - Detect unusual transaction ordering
   - Track profit extraction patterns

2. **Liquidity Monitoring**
   - Track liquidity depth changes
   - Monitor large liquidity movements
   - Detect liquidity attacks

3. **Performance Monitoring**
   - Transaction success rates
   - Compute unit usage trends
   - Network congestion impact

4. **User Behavior Analysis**
   - Unusual user patterns
   - Whale activity tracking
   - New user onboarding rates

### Version 3.0 - Predictive Monitoring
1. **Machine Learning Integration**
   - Anomaly detection models
   - Predictive risk scoring
   - Pattern recognition

2. **Automated Response Capabilities**
   - Automatic pool pausing (with authority)
   - Dynamic fee adjustments
   - Liquidity rebalancing

3. **Advanced Analytics**
   - Real-time dashboards
   - Historical trend analysis
   - Risk heat maps

### Version 4.0 - Comprehensive Security Platform
1. **Multi-Protocol Support**
   - Cross-protocol monitoring
   - DEX aggregation tracking
   - Cross-chain monitoring

2. **Regulatory Compliance**
   - AML/KYC monitoring
   - Compliance reporting
   - Audit trail generation

3. **Integration Ecosystem**
   - Webhook support
   - API for third-party integration
   - Mobile app support

## Security Considerations

### Monitor Security
1. **Access Control**
   - API keys stored in secure vault
   - Read-only RPC access
   - No private keys in monitor

2. **Data Protection**
   - Encrypted alert storage
   - Secure communication channels
   - PII data minimization

3. **Operational Security**
   - Monitor health checks
   - Redundant deployments
   - Automatic failover

### Alert Security
1. **Authentication**
   - Signed alerts
   - Alert verification
   - Anti-spoofing measures

2. **Rate Limiting**
   - Prevent alert flooding
   - DoS protection
   - Smart batching

## Deployment Guide

### Prerequisites
```bash
# System Requirements
- Node.js 18+
- PostgreSQL 14+
- Redis 6+ (optional)
- 2GB RAM minimum
- 20GB disk space

# Network Requirements
- Stable internet connection
- Access to Solana RPC endpoints
- Outbound HTTPS for Pushover
```

### Installation Steps
```bash
# 1. Clone repository
git clone https://github.com/your-org/frt-security-monitor
cd frt-security-monitor

# 2. Install dependencies
npm install

# 3. Configure environment
cp .env.example .env
# Edit .env with your configuration

# 4. Initialize database
npm run db:migrate

# 5. Build application
npm run build

# 6. Start monitor
npm run start:prod
```

### Configuration Checklist
- [ ] Configure RPC endpoints
- [ ] Set Pushover credentials
- [ ] Add treasury address
- [ ] Configure monitoring pools
- [ ] Set alert thresholds
- [ ] Configure notification recipients
- [ ] Set up logging
- [ ] Configure database connection

### Monitoring the Monitor
```bash
# Health check endpoint
curl http://localhost:3000/health

# Metrics endpoint
curl http://localhost:3000/metrics

# Recent alerts
curl http://localhost:3000/alerts/recent
```

## Conclusion

This design provides a robust foundation for security monitoring of the Fixed Ratio Trading program. The modular architecture allows for incremental feature additions while maintaining system stability. The focus on read-only operations ensures the monitor itself cannot become a security risk while providing critical visibility into program operations.

The phased approach allows for immediate deployment of critical monitoring capabilities while planning for more sophisticated features in future versions. By starting with treasury and pool monitoring, we address the most critical security concerns identified in the security assessment report.