# Fixed Ratio Trading Smart Contract (Solana)

A Solana smart contract that enables trustless token swaps at a pre-determined, immutable exchange ratio between tokens.

## Features

- Fixed Exchange Ratio: Each pool maintains an immutable ratio of base tokens per primary token
- Bi-Directional Swaps: Trade in both directions (primary → base or base → primary) at the fixed ratio
- PDA-Based Pool Accounts: Each pool is uniquely identified by a Program Derived Address
- One-Sided Liquidity Provision: LPs deposit only the primary token and receive LP tokens
- LP Token Redemption: Burn LP tokens to withdraw primary tokens from the pool
- Flat Fee Structure: Fixed fees for registration, deposits, withdrawals, and swaps

## Fee Structure

- Registration Fee: 1.15 SOL (one-time)
- Deposit/Withdrawal Fee: 0.0013 SOL
- Swap Fee: 0.0000125 SOL

## Instructions

### Initialize Pool
Creates a new trading pool with a fixed ratio between primary and base tokens.

### Deposit
Deposit primary tokens into the pool and receive LP tokens.

### Withdraw
Burn LP tokens to withdraw primary tokens from the pool.

### Swap Primary to Base
Swap primary tokens for base tokens at the fixed ratio.

### Swap Base to Primary
Swap base tokens for primary tokens at the fixed ratio.

### Withdraw Fees
Contract owner can withdraw accumulated fees.

## Example Use Case

**Scenario:**

Alice has issued MYT tokens with a guarantee of redemption at a value of 10 USDT tokens each, contingent upon her maintained collateral. To uphold this commitment and ensure the perpetual exchange of 1 MYT for 10 USDT, she has established a liquidity pool comprising USDT (Primary token) and MYT (base token) at a fixed ratio of 0.1. This means every 10 USDT in the pool will be worth 1 MYT token. In this pool, 10 USDT will be equivalent in value to 1 MYT token.

**Steps:**

1.  **Alice Creates the Pool (One-time action):**
    *   A new pool for "USDT" (Primary) and "MYT" (Base) is created with a 10 USDT : 1 MYT ratio.
    *   Alice (or the pool creator) pays the one-time registration fee of 1.15 SOL.

2.  **Alice's (or anyone's) Liquidity Provision:**
    *   Alice decides to deposit 100 USDT into the pool.
    *   She pays a deposit fee of 0.0013 SOL.
    *   Alice receives LP tokens representing her 100 USDT deposit. These LP tokens are a claim on the USDT reserves of the pool.

3.  **Bob's Swap (MYT to USDT):**
    *   Bob wants to acquire USDT and has 1 MYT.
    *   He interacts with the pool to swap his 1 MYT for USDT.
    *   Bob pays a swap fee of 0.0000125 SOL.
    *   The contract uses the 10:1 ratio. Bob provides 1 MYT to the pool.
    *   Bob receives 10 USDT from the pool's USDT vault.
    *   The pool now holds 90 USDT (100 - 10) and 1 MYT.

4.  **Alice's Withdrawal:**
    *   Later, Alice decides to withdraw her liquidity.
    *   She initiates a withdrawal by providing her LP tokens (representing her initial 100 USDT deposit).
    *   Alice pays a withdrawal fee of 0.0013 SOL.
    *   The contract burns her LP tokens.
    *   Alice receives the remaining USDT from the pool that corresponds to her share. In this simplified case, if no other LPs exist and considering Bob's trade, she would receive 90 USDT. (Note: The exact amount depends on the total liquidity and her share, but for this example, she gets back the 90 USDT remaining from her initial deposit after Bob's trade).

5.  **Post-Alice's Withdrawal & Further Swaps:**
    *   The pool now has 0 USDT (if Alice was the sole LP and withdrew everything she could) and 1 MYT (from Bob's swap).
    *   No further MYT to USDT swaps can occur as the USDT vault is empty.
    *   However, if someone wants MYT and the market value of MYT exceeds 10 USDT, they can swap 10 USDT for the 1 MYT held in the contract (assuming Alice did not withdraw the full 100 USDT, or other LPs exist).
    *   If Alice had, for instance, LP tokens equivalent to 10 USDT still in the pool (perhaps from fees or a partial withdrawal), she could withdraw that remaining 10 USDT.

**Important Note:** Once a token vault in the pool (e.g., the USDT vault) is empty, trades requiring that token as output cannot occur until new liquidity for that token is provided. Anyone (not just the initial creator) can deposit the primary token (USDT in this case) to receive LP tokens and enable further trades.

## Building

```bash
cargo build-bpf
```

## Testing

```bash
cargo test-bpf
```

## Security Considerations

- All operations are atomic
- Overflow checks are implemented for all arithmetic operations
- Fee collection is enforced for all operations
- Pool state is protected by PDA ownership
- Token transfers use SPL Token program for security 