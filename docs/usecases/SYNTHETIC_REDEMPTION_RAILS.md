# Synthetic Redemption Rails Use Case

**Document Version:** 1.0  
**Date:** January 2025  
**Use Case Category:** Synthetic Assets & Collateral  
**Complexity:** Advanced  

---

## Overview

Provide deterministic redemption paths for synthetic assets (synths) to their backing assets using **fixed-ratio pools**, avoiding volatile AMM pricing and ensuring **predictable, auditable redemptions** at par or policy-defined rates. Example: `1 USDT = 1 USDC` par redemption via a fixed-ratio pool (no slippage), guaranteed by the pool’s configured ratio and available backing liquidity.

## The Problem

- **AMM Dependency**: Synth redemption via AMMs introduces slippage and price manipulation.
- **Peg Instability**: Lack of deterministic redemption undermines peg confidence.
- **Settlement Ambiguity**: Users cannot rely on guaranteed redemption values.
- **Oracle/Latency Risk**: Oracle updates can lag during high volatility.

## The Solution: Fixed-Ratio Redemption Pools

Establish one or more fixed-ratio pools that guarantee redemption from synths to backing assets (and optionally, reverse minting) at **transparent, fixed rates**. These pools act as **on-chain redemption rails**, not speculative markets.

## Architecture

### Pool Configurations
- **Synth → Backing (Par Redemption)**: `1 Synth = 1 BackingUnit` (e.g., 1 sUSD = 1 USDC).
- **Synth → Basket**: Split redemption into multiple backing tokens at defined ratios.
- **Policy Ratios**: Governance can set controlled off-par ratios during stress events (e.g., 0.98 recovery rate).

### Redemption Flow
1. User provides Synth token to the redemption pool.
2. Pool returns backing asset at fixed ratio.
3. (Optional) Burn redeemed Synth on receipt to retire liability.

## Implementation Example

```rust
// Par redemption: 1 sUSD = 1 USDC
InitializePool {
    token_a: SUSD_MINT,          // Synthetic USD
    token_b: USDC_MINT,          // Backing stablecoin
    ratio_a: 1_000_000_000,      // 1 sUSD (9 decimals)
    ratio_b: 1_000_000,          // 1.00 USDC (6 decimals)
}

// Treasury/issuer funds USDC side to guarantee redemption capacity
Deposit {
    deposit_token_mint: USDC_MINT,
    amount: 5_000_000_000_000,   // 5,000,000 USDC
}
```

## Policy Controls

- **Pause/Unpause**: Temporarily halt redemptions during audits or attacks.
- **Tiered Rates**: Introduce recovery rates (e.g., 0.98) during under-collateralization.
- **Budget Caps**: Redemption capacity limited to provided backing asset liquidity.
- **Windowing**: Redemption windows to manage inflows during market stress.

## Benefits

- **Deterministic Redemption**: Users always know the redemption value.
- **Peg Confidence**: Credible, on-chain redemption improves stability.
- **Oracle Independence**: Redemption does not depend on time-sensitive oracles.
- **Auditability**: Full transparency of redemption flows and budgets.

## Risks & Mitigations

- **Liquidity Exhaustion**: Backing asset budget depleted.
  - Mitigation: Tiered windows, auto-replenish policies, circuit breakers.
- **Arbitrage Drain**: Off-par ratios exploited during stress.
  - Mitigation: Dynamic policy rates, governance controls, layered caps.
- **Collateral Attacks**: Backing asset volatility.
  - Mitigation: Basket backing with multiple pools and assets.

## Success Metrics

- **Peg Stability**: Reduced deviation in synth price.
- **Redemption Rate**: % of synth supply redeemed at par.
- **User Confidence**: Increased holding duration and reduced panic sells.
- **Operational Resilience**: Smooth redemptions during market stress.

## Example Scenarios

- **Stable Synths**: sUSD → USDC par redemption with emergency recovery ratio.
- **Commodity Synths**: sGOLD → wGOLD with exact weight-based ratios.
- **Index Synths**: sDEFI → basket redemption using multiple pools.

---

**Related Documentation:**
- [Fixed Ratio Trading API](../api/A_FIXED_RATIO_TRADING_API.md)
- [Security & Collateral](../security/SECURITY_ASSESSMENT_REPORT.md)
- [Governance Controls](../security/FUTURE_GOVERNANCE_CONTRACT_DESIGN.md)
