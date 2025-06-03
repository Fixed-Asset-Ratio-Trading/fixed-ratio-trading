# Fixed Ratio Trading Smart Contract (Solana)

A Solana smart contract that enables trustless token swaps at a pre-determined, immutable exchange ratio between a pair of tokens. This version introduces a dual LP token model and token pair normalization.

## Architecture

### Two-Instruction Pool Initialization Pattern

This program implements a **two-instruction pattern** for pool initialization as a workaround for a known Solana AccountInfo.data issue.

#### Background: The AccountInfo.data Issue

When using `system_instruction::create_account` via CPI to create a PDA account within a Solana program instruction, the `AccountInfo.data` slice for that account does not get updated to reflect the newly allocated memory buffer within the same instruction context.

**Symptoms:**
- `AccountInfo.data.borrow().len()` returns `0` even after successful account creation
- Serialization operations report "OK" but write to a detached buffer
- `banks_client.get_account()` returns empty data for the account
- Pool initialization appears successful but pool state data is not persisted

**Root Cause:**
This issue is documented in:
- [Solana GitHub Issue #31960](https://github.com/solana-labs/solana/issues/31960)
- Various Solana Stack Exchange discussions
- Community reports of similar CPI account creation issues

#### Solution: Two-Instruction Pattern

**Instruction 1: `CreatePoolStateAccount`**
```rust
CreatePoolStateAccount {
    ratio_primary_per_base: u64,
    pool_authority_bump_seed: u8,
    primary_token_vault_bump_seed: u8,
    base_token_vault_bump_seed: u8,
}
```

**What it does:**
- Creates Pool State PDA account with correct size allocation
- Creates LP token mints and transfers authority to pool
- Creates token vault PDAs and initializes them  
- Transfers registration fees
- **Does NOT attempt to serialize PoolState data**

**Instruction 2: `InitializePoolData`**
```rust
InitializePoolData {
    ratio_primary_per_base: u64,
    pool_authority_bump_seed: u8,
    primary_token_vault_bump_seed: u8,
    base_token_vault_bump_seed: u8,
}
```

**What it does:**
- Runs in fresh transaction context with proper AccountInfo references
- Validates Pool State PDA account exists with correct size
- Creates and populates PoolState struct with configuration data
- Uses buffer serialization approach for additional safety
- Writes pool state data that properly persists on-chain

#### Buffer Serialization Technique

Even with the two-instruction pattern, we employ an additional safeguard:

```rust
// Step 1: Serialize to temporary buffer
let mut serialized_data = Vec::new();
pool_state_data.serialize(&mut serialized_data)?;

// Step 2: Atomic copy to account data
let mut account_data = pool_state_pda_account.data.borrow_mut();
account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
```

This approach ensures:
- Serialization success is verified before writing to account
- Copy operation is atomic (all-or-nothing)
- Data integrity and persistence are guaranteed

#### Usage in Client Code

```typescript
// Transaction 1: Create accounts
const createTx = new Transaction().add(
  createPoolStateAccountInstruction(...)
);
await sendAndConfirmTransaction(connection, createTx, [payer, ...]);

// Transaction 2: Initialize data  
const initTx = new Transaction().add(
  initializePoolDataInstruction(...)
);
await sendAndConfirmTransaction(connection, initTx, [payer]);
```

#### Testing Implementation

The integration tests in `tests/integration_test.rs` demonstrate this pattern:

```rust
// Transaction 1: Account creation
let mut create_tx = Transaction::new_with_payer(&[create_ix], Some(&payer.pubkey()));
create_tx.sign(&signers_for_create_tx[..], recent_blockhash);
banks_client.process_transaction(create_tx).await?;

// Transaction 2: Data initialization
let mut init_data_tx = Transaction::new_with_payer(&[init_data_ix], Some(&payer.pubkey()));
init_data_tx.sign(&[&payer], recent_blockhash);
banks_client.process_transaction(init_data_tx).await?;
```

This workaround ensures reliable pool initialization across all Solana environments and runtime versions.

## Features

-   **Fixed Exchange Ratio**: Each pool maintains an immutable ratio between two tokens (Token A and Token B).
-   **Token Pair Normalization**: Pools are uniquely identified by the pair of token mints and their ratio, regardless of the order in which the tokens are specified during initialization. Internally, tokens are normalized (e.g., by lexicographical order of their mint addresses) to prevent duplicate pools for the same pair and ratio.
-   **Dual LP Token Model**:
    -   Each liquidity pool issues two distinct LP tokens:
        -   `LP-Token-A`: Represents a claim on Token A in the pool.
        -   `LP-Token-B`: Represents a claim on Token B in the pool.
-   **One-Sided Liquidity Provision**: Users can provide liquidity for *either* Token A or Token B individually and receive the corresponding LP token (`LP-Token-A` or `LP-Token-B`).
-   **One-Sided Liquidity Withdrawal**: Users can burn *either* `LP-Token-A` to withdraw Token A, or `LP-Token-B` to withdraw Token B.
-   **Bi-Directional Swaps**: Trade Token A for Token B, or Token B for Token A, at the pool's fixed ratio, utilizing the combined liquidity.
-   **PDA-Based Pool Accounts**: Each unique pool (defined by normalized token pair and ratio) is managed by a Program Derived Address (PDA). Vaults and LP mints are also PDAs or controlled by the pool PDA.
-   **Flat Fee Structure**: Fixed fees for pool registration, deposits, withdrawals, and swaps.

## Fee Structure

-   Registration Fee: 1.15 SOL (one-time, paid when a new pool is created)
-   Deposit/Withdrawal Fee: 0.0013 SOL (per transaction)
-   Swap Fee: 0.0000125 SOL (per transaction)

## Instructions

### 1. `InitializePool`

-   **Purpose**: Creates a new trading pool for a pair of tokens with a specified fixed exchange ratio.
-   **Details**:
    -   Takes two token mints (e.g., Primary Token, Base Token) and a ratio (e.g., X units of Primary per 1 unit of Base).
    -   Normalizes the token pair (Token A, Token B) and the ratio (`ratio_A_numerator` : `ratio_B_denominator`).
    -   Creates a unique Pool State PDA based on the normalized pair and ratio.
    -   Creates two new SPL Token mints: `LP-Token-A` (for Token A liquidity) and `LP-Token-B` (for Token B liquidity). The Pool State PDA is the mint authority.
    -   Creates two token vaults (for Token A and Token B), owned by the Pool State PDA.
-   **Accounts Required**: Payer (signer), Pool State PDA (to be created), Primary Token Mint, Base Token Mint, LP Token A Mint (to be created), LP Token B Mint (to be created), Token A Vault PDA (to be created), Token B Vault PDA (to be created), System Program, Token Program, Rent Sysvar.

### 2. `Deposit`

-   **Purpose**: Allows a user to deposit one of the pool's tokens (either Token A or Token B) and receive a corresponding amount of LP tokens for that specific token.
-   **Details**:
    -   User specifies the `deposit_token_mint` (must be one of the pool's `token_a_mint` or `token_b_mint`) and the `amount`.
    -   The specified `amount` of the `deposit_token_mint` is transferred from the user to the pool's corresponding vault (Token A vault or Token B vault).
    -   An equivalent `amount` of the corresponding LP tokens (`LP-Token-A` or `LP-Token-B`) is minted to the user. (1:1 minting for one-sided deposit).
-   **Accounts Required**: User (signer), User's Source Token Account (for the token being deposited), Pool State PDA, Token A Mint (for PDA seed verification), Token B Mint (for PDA seed verification), Pool's Token A Vault, Pool's Token B Vault, LP Token A Mint, LP Token B Mint, User's Destination LP Token Account (for the corresponding LP token), System Program, Token Program.

### 3. `Withdraw`

-   **Purpose**: Allows a user to burn one type of LP token (`LP-Token-A` or `LP-Token-B`) and withdraw a corresponding amount of the underlying token (Token A or Token B) from the pool.
-   **Details**:
    -   User specifies the `withdraw_token_mint` (the underlying token they want to receive, either Token A or Token B) and the `lp_amount_to_burn`.
    -   The specified `lp_amount_to_burn` of the corresponding LP tokens (`LP-Token-A` or `LP-Token-B`) is burned from the user's account.
    -   An equivalent `lp_amount_to_burn` of the `withdraw_token_mint` is transferred from the pool's corresponding vault to the user.
-   **Accounts Required**: User (signer), User's Source LP Token Account (for the LP token being burned), User's Destination Token Account (for the underlying token), Pool State PDA, Token A Mint (for PDA seed verification), Token B Mint (for PDA seed verification), Pool's Token A Vault, Pool's Token B Vault, LP Token A Mint, LP Token B Mint, System Program, Token Program.

### 4. `Swap`

-   **Purpose**: Allows a user to swap a specified amount of one token from the pool (e.g., Token A) for an equivalent amount of the other token (e.g., Token B) based on the pool's fixed ratio.
-   **Details**:
    -   User specifies the `input_token_mint` (the token they are giving) and the `amount_in`.
    -   The contract calculates the `amount_out` of the other token based on the pool's `ratio_A_numerator` and `ratio_B_denominator`.
    -   `amount_in` is transferred from the user to the pool's vault for the input token.
    -   `amount_out` is transferred from the pool's vault for the output token to the user.
-   **Accounts Required**: User (signer), User's Input Token Account, User's Output Token Account, Pool State PDA, Token A Mint (for PDA seed verification), Token B Mint (for PDA seed verification), Pool's Token A Vault, Pool's Token B Vault, System Program, Token Program.

### 5. `WithdrawFees`

-   **Purpose**: Allows the designated owner of the pool (set during `InitializePool`) to withdraw accumulated SOL fees from the Pool State PDA.
-   **Details**: Transfers the SOL balance of the Pool State PDA to the owner's account.
-   **Accounts Required**: Owner (signer), Pool State PDA, System Program.

## Example Use Case (Dual LP Model)

**Scenario:**
A pool is desired for USDC and MSOL with a fixed ratio where 1 MSOL = 150 USDC.

**1. Pool Initialization (`InitializePool`):**
*   **Creator**: Pays the registration fee (1.15 SOL).
*   **Inputs**:
    *   Primary Token Mint: MSOL Mint Address
    *   Base Token Mint: USDC Mint Address
    *   Ratio (Primary per Base): 1 MSOL per 150 USDC. (The contract will normalize this. If MSOL < USDC lexicographically, Token A becomes MSOL, Token B becomes USDC. Ratio A:B becomes 1 MSOL : 150 USDC. If USDC < MSOL, Token A becomes USDC, Token B becomes MSOL. Ratio A:B becomes 150 USDC : 1 MSOL).
*   **Outcome**:
    *   A new Pool State PDA is created, uniquely identifying this MSOL/USDC pool with the specified ratio.
    *   Two LP Mints are created: `LP-MSOL-Mint` and `LP-USDC-Mint`.
    *   Two Vaults are created: `MSOL-Vault` and `USDC-Vault`.

**2. Alice's Liquidity Provision (One-Sided - MSOL) (`Deposit`):**
*   Alice wants to provide 10 MSOL.
*   She pays the deposit fee (0.0013 SOL).
*   **Action**: Calls `Deposit` with `deposit_token_mint = MSOL-Mint-Address`, `amount = 10 MSOL`.
*   **Outcome**:
    *   10 MSOL transferred from Alice's MSOL account to the pool's `MSOL-Vault`.
    *   Alice receives 10 `LP-MSOL` tokens from `LP-MSOL-Mint`.
    *   Pool Liquidity: 10 MSOL, 0 USDC.

**3. Bob's Liquidity Provision (One-Sided - USDC) (`Deposit`):**
*   Bob wants to provide 3000 USDC.
*   He pays the deposit fee (0.0013 SOL).
*   **Action**: Calls `Deposit` with `deposit_token_mint = USDC-Mint-Address`, `amount = 3000 USDC`.
*   **Outcome**:
    *   3000 USDC transferred from Bob's USDC account to the pool's `USDC-Vault`.
    *   Bob receives 3000 `LP-USDC` tokens from `LP-USDC-Mint`.
    *   Pool Liquidity: 10 MSOL, 3000 USDC.

**4. Charlie's Swap (USDC for MSOL) (`Swap`):**
*   Charlie has 1500 USDC and wants MSOL. The pool ratio is 1 MSOL : 150 USDC.
*   He pays the swap fee (0.0000125 SOL).
*   **Action**: Calls `Swap` with `input_token_mint = USDC-Mint-Address`, `amount_in = 1500 USDC`.
*   **Calculation**: `amount_out_MSOL = (1500 USDC / 150 USDC_per_MSOL) = 10 MSOL`.
*   **Outcome**:
    *   1500 USDC transferred from Charlie to the pool's `USDC-Vault`.
    *   10 MSOL transferred from the pool's `MSOL-Vault` to Charlie.
    *   Pool Liquidity: 0 MSOL (10 - 10), 4500 USDC (3000 + 1500).

**5. Alice's Withdrawal (MSOL) (`Withdraw`):**
*   Alice wants to withdraw her MSOL liquidity. She has 10 `LP-MSOL` tokens.
*   The pool now has 0 MSOL.
*   **Action**: Alice calls `Withdraw` with `withdraw_token_mint = MSOL-Mint-Address`, `lp_amount_to_burn = 10 LP-MSOL`.
*   **Outcome**:
    *   This withdrawal will FAIL because the `MSOL-Vault` is empty (`pool_state_data.total_token_a_liquidity` for MSOL is 0). Alice cannot get her MSOL back until someone swaps USDC for MSOL, replenishing the MSOL vault.
    *   This highlights the risk of one-sided liquidity: if the token you provided is heavily swapped out, you might not be able to withdraw it immediately.

**6. Bob's Withdrawal (Partial USDC) (`Withdraw`):**
*   Bob wants to withdraw 1000 USDC. He has 3000 `LP-USDC` tokens.
*   Pool `USDC-Vault` has 4500 USDC.
*   He pays the withdrawal fee (0.0013 SOL).
*   **Action**: Bob calls `Withdraw` with `withdraw_token_mint = USDC-Mint-Address`, `lp_amount_to_burn = 1000 LP-USDC`.
*   **Outcome**:
    *   1000 `LP-USDC` tokens burned from Bob.
    *   1000 USDC transferred from pool's `USDC-Vault` to Bob.
    *   Bob now has 2000 `LP-USDC` tokens.
    *   Pool Liquidity: 0 MSOL, 3500 USDC (4500 - 1000).

**Simulating "Original" Balanced Liquidity Provision:**
To provide liquidity similar to the previous single LP token model (where a user provides both tokens in ratio), a user would perform two separate `Deposit` operations:
1.  Deposit Token A and receive `LP-Token-A`.
2.  Deposit Token B (in the correct ratio to Token A) and receive `LP-Token-B`.

Withdrawal would also require two separate `Withdraw` operations if they wish to get both tokens back.

## Building

```bash
cargo build-bpf
```

## Testing

```bash
cargo test-bpf
```

## Security Considerations

-   All operations aim to be atomic.
-   Comprehensive overflow checks are implemented for arithmetic operations.
-   Fee collection is enforced.
-   Pool state and vaults are protected by PDA ownership and derivation logic.
-   Token transfers rely on the security of the SPL Token program.
-   **New Considerations for Dual LP Model**:
    -   The 1:1 minting of LP tokens for one-sided deposits is simple but means LP token value is directly tied to the specific token deposited, not a share of the *overall* pool value in the same way as a traditional AMM LP token.
    -   Users must understand that providing liquidity for one side (e.g., Token A) means their ability to withdraw Token A depends on Token A being present in the pool, which can be depleted by swaps.
    -   The ratio is fixed, so there is no impermanent loss in the traditional sense, but liquidity can become imbalanced. 