# Floor-Price Buybacks & Treasury Windows Use Case

**Document Version:** 1.0  
**Date:** January 2025  
**Use Case Category:** Treasury & Market Defense  
**Complexity:** Intermediate  

---

## Overview

Deploy time-boxed, fixed-ratio buyback pools to defend a floor price for a token or to conduct controlled treasury buyback windows. The protocol guarantees a fixed buyback ratio during the window, providing a credible floor and predictable liquidity pathway for holders.

## The Problem

- **Price Crashes & Panic Selling**: Market selloffs can break confidence and spiral lower.
- **Unreliable Order Books**: Thin liquidity makes floors easy to breach.
- **Treasury Tools Are Blunt**: Manual buys are slow, opaque, and operationally risky.
- **Credibility Gap**: Announced floors often fail without automated, on-chain enforcement.

## The Solution: Time-Boxed Fixed-Ratio Pools

Create a fixed-ratio pool (e.g., `1 ProjectToken = 1.00 USDC`) that is funded by the treasury and enabled for a defined time window. Holders can redeem at the floor rate during the window, establishing a credible, on-chain buyback mechanism.

## Architecture

### Pool Parameters
- **Token A (Project Token)**: The token with the defended floor
- **Token B (Reserve Asset)**: USDC/USDT/SOL—treasury-controlled buyback currency
- **Fixed Ratio**: Floor rate (e.g., `1 Token = 1.00 USDC`)
- **Windowing**: Start/end timestamps (enforced off-chain by the window manager or by pausing/unpausing at specific times)
- **Cap Controls**: Treasury defines maximum buyback budget (liquidity provided)

### Operational Flow
1. Treasury funds the pool with reserve asset at the floor ratio budget.
2. System unpauses pool for the window duration (or creates and later closes the pool).
3. Holders redeem tokens at the floor, receiving reserve asset at exact ratio.
4. At window end, treasury pauses pool and withdraws remaining reserves.

## Implementation Example

```rust
// Example: Defend 1.00 USDC floor for PROJECT token
// Ratio expressed in basis points with decimal alignment
InitializePool {
    token_a: PROJECT_MINT,      // Redeemed by users (sell to treasury)
    token_b: USDC_MINT,         // Treasury reserve asset paid out
    ratio_a: 1_000_000_000,     // 1 PROJECT (9 decimals)
    ratio_b: 1_000_000,         // 1.00 USDC (6 decimals)
}

// Treasury provides liquidity equal to buyback budget
Deposit {
    deposit_token_mint: USDC_MINT,
    amount: 1_000_000_000_000, // 1,000,000 USDC budget
}
```

## Treasury Window Management

- **Activation**: Unpause the pool at `T0` to start the buyback window.
- **Deactivation**: Pause the pool at `T1` to end the window.
- **Budget Discipline**: Treasury deposits only the amount it is willing to spend.
- **Multiple Tranches**: Stagger multiple windows to manage expectations and liquidity.

## Benefits

- **Credible Floor**: On-chain, enforceable, and auditable.
- **Predictable Execution**: Zero slippage at the guaranteed ratio.
- **Fast Stabilization**: Absorbs sell pressure transparently.
- **Community Trust**: Clear rules reduce panic and rumor.
- **Treasury Control**: Exact budget control via provided liquidity.

## Risks & Mitigations

- **Budget Exhaustion**: Window can deplete quickly.
  - Mitigation: Tiered tranches, short windows, dynamic announcements.
- **Moral Hazard**: Traders may rely on floors.
  - Mitigation: Use sparingly during extreme events; communicate policy clearly.
- **Timing Attacks**: Bots snipe windows.
  - Mitigation: Short windows, randomized start within a public interval, rate limits (off-chain UI), KYC if applicable.
- **Liquidity Migration**: Secondary markets adjust to floor.
  - Mitigation: Announce scope and limits, avoid perpetual floors.

## Operational Playbook

1. **Policy Definition**: Conditions to activate a buyback window (e.g., 20% deviation from NAV or governance trigger).
2. **Budget Allocation**: Define tranche size (e.g., $250k per window).
3. **Window Announcement**: Communicate timing and rules.
4. **Execution**: Unpause pool; monitor fills; pause at end.
5. **Post-Analysis**: Publish report with amounts redeemed and remaining budget.

## Success Metrics

- **Redemption Volume**: Tokens redeemed vs budget used.
- **Floor Adherence**: Market price stabilization around floor.
- **Community Sentiment**: Reduced panic metrics/social signals.
- **Volatility Reduction**: Pre- vs post-window volatility stats.

## Example Scenarios

- **Emergency Crash Response**: 4-hour window at 1.00 USDC floor after 30% drawdown.
- **Quarterly Capital Return**: Pre-announced buyback windows to return value to holders.
- **Event-Linked Floor**: Temporary floor during token unlock events to reduce shock.

---

**Related Documentation:**
- [Fixed Ratio Trading API](../api/A_FIXED_RATIO_TRADING_API.md)
- [Security Considerations](../security/SECURITY_ASSESSMENT_REPORT.md)
- [Governance Protocols](../security/FUTURE_GOVERNANCE_CONTRACT_DESIGN.md)
