# OTC Vesting Conversions Use Case

**Document Version:** 1.0  
**Date:** January 2025  
**Use Case Category:** Token Distribution & Compliance  
**Complexity:** Intermediate  

---

## Overview

Enable team and investor vesting claims to be converted into liquid tokens at **fixed rates without slippage** via dedicated fixed-ratio pools. This creates a **fair, predictable conversion mechanism** that coexists with vesting schedules and off-chain agreements.

## The Problem

- **OTC Liquidity Friction**: Private investors often need structured exits; public markets suffer slippage.
- **Price Impact**: Unlock events can cause large sell pressure and unpredictable pricing.
- **Compliance Constraints**: Conversions require clear, auditable processes aligned with agreements.
- **Coordination Overhead**: Manual OTC deals are operationally heavy and opaque.

## The Solution: Fixed-Rate Vesting Conversion Pools

Create fixed-ratio pools where vesting claim tokens (or claim vouchers) convert to liquid tokens at pre-agreed rates—**zero slippage**, **auditable**, **time-restricted**, and **budget-limited**.

## Architecture

### Token Model Options
1. **Claim Voucher Tokens**: Mint claim tokens representing a right to convert (e.g., `VEST-CLAIM`).
2. **Direct Vesting Mints**: Vesting escrow releases tokens directly to the conversion pool flow.

### Pool Parameters
- **Token A**: Vesting claim token (non-transferable optional)
- **Token B**: Liquid token (public trading token)
- **Fixed Ratio**: Pre-agreed OTC rate (e.g., `1 claim = 0.85 liquid`)
- **Controls**: 
  - Pause/unpause to align with vesting windows
  - Liquidity budget caps to throttle exits
  - Optional allowlist UI enforcement

## Implementation Example

```rust
// Example: Convert vested claim to liquid at 0.85 rate
InitializePool {
    token_a: VEST_CLAIM_MINT,     // Claim/voucher representing vested rights
    token_b: LIQUID_TOKEN_MINT,   // Public liquid token
    ratio_a: 1_000_000_000,       // 1 claim (9 decimals)
    ratio_b: 850_000_000,         // 0.85 liquid (9 decimals)
}

// Treasury or market maker provides liquidity to satisfy OTC conversions
Deposit {
    deposit_token_mint: LIQUID_TOKEN_MINT,
    amount: 50_000_000_000_000, // Budget for conversions
}
```

## Operational Policies

- **Windows**: Limit conversion periods to vesting unlock dates.
- **Budgets**: Cap liquidity per window to prevent shocks.
- **Pricing**: OTC discount can reflect lockups, early exits, or strategic agreements.
- **Transparency**: On-chain conversions with auditable records.

## Benefits

- **Predictable Conversions**: Investors know exact rates; no slippage risk.
- **Market Protection**: Reduce order-book dumping and price impact.
- **Compliance Friendly**: Clear, rule-based, auditable OTC mechanism.
- **Operational Efficiency**: Replace manual OTC coordination with on-chain rails.

## Risks & Mitigations

- **Adverse Signaling**: Discounted rates seen as bearish.
  - Mitigation: Clear communication; time-bound windows; limited budgets.
- **Speculative Gaming**: Attempts to acquire claim tokens indirectly.
  - Mitigation: Non-transferable claim tokens; allowlisted UI/process.
- **Budget Exhaustion**: Conversions exceed planned liquidity.
  - Mitigation: Tiered windows; dynamic budget reallocation.

## Success Metrics

- **Conversion Throughput**: Claims converted vs vested volume.
- **Market Stability**: Reduced volatility around unlock events.
- **Stakeholder Satisfaction**: Investors and team report smoother processes.
- **Compliance Outcomes**: Clean audit posture for OTC conversions.

## Example Scenarios

- **Team Unlocks**: Quarterly windows for team claim conversions at pre-agreed rates.
- **Investor Exits**: Structured exits over 6–12 months with fixed discounts.
- **Strategic Partnerships**: Partners receive claim vouchers convertible at fixed strategic rates.

---

**Related Documentation:**
- [Fixed Ratio Trading API](../api/A_FIXED_RATIO_TRADING_API.md)
- [Security & Compliance](../security/SECURITY_ASSESSMENT_REPORT.md)
- [Governance Controls](../security/FUTURE_GOVERNANCE_CONTRACT_DESIGN.md)
