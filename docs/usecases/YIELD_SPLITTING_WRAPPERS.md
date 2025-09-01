# Yield Splitting & Wrappers Use Case

**Document Version:** 1.0  
**Date:** January 2025  
**Use Case Category:** Structured Products  
**Complexity:** Advanced  

---

## Overview

Create structured tokens that separate **principal** and **yield** into distinct instruments. Use Fixed Ratio Trading pools to allow **fixed conversion of the principal leg at par** (e.g., `1 Principal = 1 Underlying` at maturity or policy-controlled times), while the yield leg floats.

## The Problem

- **Complex Yield Instruments**: Hard to provide predictable redemption for principal.
- **Liquidity Fragmentation**: Principal and yield tokens suffer from uncertain AMM pricing.
- **Transparency & Redemption**: Investors need credible par redemption for principal.
- **Market Access**: Retail needs simple, auditable pathways to principal redemption.

## The Solution: Par Principal Redemption Pools

Deploy fixed-ratio pools that guarantee conversion of **Principal tokens** back to the **Underlying token at par** (e.g., `1 pToken = 1 underlying`) at defined checkpoints, while yield tokens (`yToken`) trade freely.

## Architecture

### Token Split Model
- **Underlying Token (U)**: Interest-bearing or yield-generating asset.
- **Principal Token (pU)**: Claim on principal only, redeemable at par.
- **Yield Token (yU)**: Claim on accrued yield, freely tradable.

### Pool Configuration (Principal Redemption)
- **Token A**: pU (Principal token)
- **Token B**: U (Underlying)
- **Fixed Ratio**: 1:1 at redemption checkpoints (or policy schedule)
- **Windows**: Redemption windows at intervals/maturity; pools paused otherwise

## Implementation Example

```rust
// Guarantee par redemption for principal token
InitializePool {
    token_a: P_UNDERLYING_MINT,   // pU principal token
    token_b: UNDERLYING_MINT,     // U base token
    ratio_a: 1_000_000_000,       // 1 pU (9 decimals)
    ratio_b: 1_000_000_000,       // 1 U  (9 decimals)
}

// Treasury or product issuer funds pool with Underlying
Deposit {
    deposit_token_mint: UNDERLYING_MINT,
    amount: 10_000_000_000_000, // 10,000 U budget
}
```

## Redemption Policy

- **Maturity-Based**: Par redemption available at or after maturity.
- **Interval Windows**: Weekly/monthly par redemption windows.
- **Early Redemption**: Optional policy to provide discounted early redemption.
- **Pause Controls**: Pause pools outside redemption periods.

## Benefits

- **Predictable Principal**: Investors guaranteed par redemption for principal.
- **Transparent Structure**: Clear split between principal and yield.
- **Market Access**: pU and yU trade separately; different risk appetites.
- **Composability**: Other protocols can build on pU/yU instruments.

## Risks & Mitigations

- **Underfunded Redemption**: Insufficient Underlying in pool.
  - Mitigation: Match budgets to outstanding pU; staged windows; governance alerts.
- **Yield Volatility**: yU price volatility.
  - Mitigation: Education; AMM/market maker support for yU liquidity.
- **Timing Attacks**: Front-running windows.
  - Mitigation: Short windows; randomized starts; KYC gates if needed.

## Use Case Scenarios

- **Fixed Income Wrappers**: Tokenized bonds with par principal redemption and floating coupons.
- **Staking Derivatives**: Split staked asset into principal and yield accrual token.
- **Savings Products**: Retail-friendly products guaranteeing principal redemption dates.

## Success Metrics

- **Redemption Throughput**: % of pU redeemed at par during windows.
- **Market Depth**: Liquidity for both pU and yU.
- **User Adoption**: Growth in structured product holders.
- **Protocol Integrations**: Composability in other DeFi protocols.

---

**Related Documentation:**
- [Fixed Ratio Trading API](../api/A_FIXED_RATIO_TRADING_API.md)
- [Security Considerations](../security/SECURITY_ASSESSMENT_REPORT.md)
- [Product Governance](../security/FUTURE_GOVERNANCE_CONTRACT_DESIGN.md)
