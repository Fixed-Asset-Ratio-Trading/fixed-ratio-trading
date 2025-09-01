# Token Redemption Payouts Use Case

**Document Version:** 1.0  
**Date:** January 2025  
**Use Case Category:** Payouts & Incentives  
**Complexity:** Intermediate  

---

## Overview

Execute deterministic payout programs using Fixed Ratio Trading pools that convert rebate/voucher/claim tokens into payout assets (e.g., USDC) at **fixed, auditable rates**. Covers insurance-like payouts, DAO rebate programs, and token wind-downs at pre-announced rates.

## The Problem

- **Unpredictable Payouts**: AMM-based redemptions suffer slippage and manipulation.
- **Operational Overhead**: Manual distribution is error-prone and opaque.
- **Trust Deficit**: Recipients need transparent, on-chain redemption mechanics.
- **Wind-Down Complexity**: Token sunsets require credible close-out processes.

## The Solution: Fixed-Rate Redemption Pools

Establish pools where **payout tokens** (rebateToken, claimToken, closeoutToken) redeem into **payout assets** (USDC/USDT/SOL) at a **fixed ratio** (e.g., `1 rebateToken = 10 USDC`). Time-boxed windows and budget caps align payouts with treasury capacity.

## Architecture

### Payout Models
- **Rebate Programs**: Users redeem `rebateToken` for `USDC` at fixed rates.
- **Insurance Payouts**: `policyToken` redeemable for `payoutAsset` after covered events.
- **Closing/Wind-Down**: Legacy `projectToken` redeemable into `payoutAsset` at fixed sunset rate.

### Pool Parameters
- **Token A**: Payout token (rebate/policy/closeout)
- **Token B**: Payout asset (USDC/USDT/SOL)
- **Fixed Ratio**: Defines payout amount per redemption unit
- **Windowing**: Time-limited redemption periods (optional)
- **Budget Caps**: Treasury-controlled payout budgets via provided liquidity

## Implementation Example

```rust
// DAO rebate: 1 rebateToken = 10 USDC
InitializePool {
    token_a: REBATE_MINT,         // Issued rebate tokens
    token_b: USDC_MINT,           // Payout asset
    ratio_a: 1_000_000_000,       // 1 rebateToken (9 decimals)
    ratio_b: 10_000_000,          // 10.00 USDC (6 decimals)
}

// Treasury funds payout budget
Deposit {
    deposit_token_mint: USDC_MINT,
    amount: 2_000_000_000_000,    // 2,000,000 USDC budget
}
```

## Operational Policies

- **Eligibility**: Off-chain allowlist, merkle proofs, or UI controls as needed.
- **Windows**: Monthly/quarterly payout cycles or event-driven windows.
- **Reporting**: Post-window reports: redemptions, remaining budget, beneficiaries.
- **Sunset Programs**: Announce final redemption pools and deadlines.

## Benefits

- **Deterministic Payouts**: Recipients know exact amounts in advance.
- **Zero Slippage**: No AMM dependency or price impact.
- **Auditable**: On-chain redemption records and treasury budgeting.
- **Operational Simplicity**: Single pool replaces manual payout flows.

## Risks & Mitigations

- **Budget Shortfall**: Claims exceed budgeted payouts.
  - Mitigation: Tiered windows, pro-rata top-ups, transparent communication.
- **Token Leakage**: Payout tokens circulate outside intended recipients.
  - Mitigation: UI allowlists, non-transferable payout tokens, claim verification.
- **Timing Attacks**: Sniping windows.
  - Mitigation: Short windows, randomized activation within notified ranges.

## Success Metrics

- **Redemption Rate**: % of payout tokens redeemed per window.
- **Program Coverage**: Total recipients served vs planned.
- **Budget Utilization**: Funds distributed vs budgeted.
- **User Satisfaction**: Support metrics and sentiment.

## Example Scenarios

- **Insurance Claims**: `policyToken` redeemable for stablecoins after verified incident.
- **DAO Rebates**: Ecosystem incentives distributed as rebateToken redeemable at fixed USDC amount.
- **Token Sunsets**: Project winds down old token at fixed redemption rate for holders.

---

**Related Documentation:**
- [Fixed Ratio Trading API](../api/A_FIXED_RATIO_TRADING_API.md)
- [Security Considerations](../security/SECURITY_ASSESSMENT_REPORT.md)
- [Governance Controls](../security/FUTURE_GOVERNANCE_CONTRACT_DESIGN.md)
