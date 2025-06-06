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
-   **Trading Fee: 0.3% of input amount** (collected on swaps, withdrawable by delegates)

## Instructions

### Core Trading Instructions

### 1. `CreatePoolStateAccount`

-   **Purpose**: Creates the pool state PDA account and all related accounts (LP mints, vaults).
-   **Details**: First step of the two-instruction initialization pattern. Creates accounts without serializing data.
-   **Accounts Required**: Payer, Pool State PDA, Primary Token Mint, Base Token Mint, LP Token A Mint, LP Token B Mint, Token A Vault PDA, Token B Vault PDA, System Program, Token Program, Rent Sysvar.

### 2. `InitializePoolData`

-   **Purpose**: Initializes the data in the already-created Pool State PDA account.
-   **Details**: Second step of initialization. Populates the pool state with configuration data and sets owner as initial delegate.
-   **Accounts Required**: Same as CreatePoolStateAccount.

### 3. `Deposit`

-   **Purpose**: Allows a user to deposit one of the pool's tokens (either Token A or Token B) and receive a corresponding amount of LP tokens for that specific token.
-   **Details**:
    -   User specifies the `deposit_token_mint` (must be one of the pool's `token_a_mint` or `token_b_mint`) and the `amount`.
    -   The specified `amount` of the `deposit_token_mint` is transferred from the user to the pool's corresponding vault (Token A vault or Token B vault).
    -   An equivalent `amount` of the corresponding LP tokens (`LP-Token-A` or `LP-Token-B`) is minted to the user. (1:1 minting for one-sided deposit).
-   **Accounts Required**: User (signer), User's Source Token Account (for the token being deposited), Pool State PDA, Token A Mint (for PDA seed verification), Token B Mint (for PDA seed verification), Pool's Token A Vault, Pool's Token B Vault, LP Token A Mint, LP Token B Mint, User's Destination LP Token Account (for the corresponding LP token), System Program, Token Program.

### 4. `Withdraw`

-   **Purpose**: Allows a user to burn one type of LP token (`LP-Token-A` or `LP-Token-B`) and withdraw a corresponding amount of the underlying token (Token A or Token B) from the pool.
-   **Details**:
    -   User specifies the `withdraw_token_mint` (the underlying token they want to receive, either Token A or Token B) and the `lp_amount_to_burn`.
    -   The specified `lp_amount_to_burn` of the corresponding LP tokens (`LP-Token-A` or `LP-Token-B`) is burned from the user's account.
    -   An equivalent `lp_amount_to_burn` of the `withdraw_token_mint` is transferred from the pool's corresponding vault to the user.
-   **Accounts Required**: User (signer), User's Source LP Token Account (for the LP token being burned), User's Destination Token Account (for the underlying token), Pool State PDA, Token A Mint (for PDA seed verification), Token B Mint (for PDA seed verification), Pool's Token A Vault, Pool's Token B Vault, LP Token A Mint, LP Token B Mint, System Program, Token Program.

### 5. `Swap`

-   **Purpose**: Allows a user to swap a specified amount of one token from the pool (e.g., Token A) for an equivalent amount of the other token (e.g., Token B) based on the pool's fixed ratio.
-   **Details**:
    -   User specifies the `input_token_mint` (the token they are giving) and the `amount_in`.
    -   **0.3% trading fee is deducted from the input amount** and tracked separately for delegate withdrawal.
    -   The contract calculates the `amount_out` of the other token based on the pool's `ratio_A_numerator` and `ratio_B_denominator`.
    -   `amount_in` is transferred from the user to the pool's vault for the input token.
    -   `amount_out` is transferred from the pool's vault for the output token to the user.
-   **Accounts Required**: User (signer), User's Input Token Account, User's Output Token Account, Pool State PDA, Token A Mint (for PDA seed verification), Token B Mint (for PDA seed verification), Pool's Token A Vault, Pool's Token B Vault, System Program, Token Program.

### 6. `WithdrawFees`

-   **Purpose**: Allows the designated owner of the pool (set during `InitializePool`) to withdraw accumulated SOL fees from the Pool State PDA.
-   **Details**: Transfers the SOL balance of the Pool State PDA to the owner's account.
-   **Accounts Required**: Owner (signer), Pool State PDA, System Program.

### 7. `UpdateSecurityParams`

-   **Purpose**: Allows the pool owner to update security parameters like withdrawal limits, cooldown periods, and pause state.
-   **Details**: Only the pool owner can modify these parameters to maintain pool security.
-   **Accounts Required**: Owner (signer), Pool State PDA.

### Delegate Management Instructions

### 8. `AddDelegate`

-   **Purpose**: Allows the pool owner to add up to 3 authorized delegates who can withdraw collected trading fees.
-   **Details**:
    -   Only the pool owner can add delegates
    -   Maximum of 3 delegates allowed at any time
    -   24-hour cooldown period enforced between delegate changes
    -   Pool owner is automatically set as the first delegate upon pool creation
-   **Accounts Required**: Owner (signer), Pool State PDA, Clock Sysvar.

### 9. `RemoveDelegate`

-   **Purpose**: Allows the pool owner to remove an existing delegate.
-   **Details**:
    -   Only the pool owner can remove delegates
    -   24-hour cooldown period enforced between delegate changes
    -   Cannot remove all delegates (at least one must remain)
-   **Accounts Required**: Owner (signer), Pool State PDA, Clock Sysvar.

### 10. `WithdrawFeesToDelegate`

-   **Purpose**: Allows authorized delegates to withdraw collected trading fees from the pool.
-   **Details**:
    -   Only authorized delegates can call this instruction
    -   Daily withdrawal limits capped at 15% of total pool balance per token
    -   Blocked when pool is paused
    -   All withdrawals are logged with timestamp and delegate information
    -   Separate tracking of collected fees vs. pool liquidity
-   **Accounts Required**: Delegate (signer), Pool State PDA, Token Vault, Delegate Token Account, Token Program, Rent Sysvar, Clock Sysvar.

### 11. `GetWithdrawalHistory`

-   **Purpose**: Returns withdrawal history for transparency and auditing.
-   **Details**:
    -   Logs the last 10 withdrawal transactions
    -   Shows delegate addresses, amounts, timestamps, and slot numbers
    -   Displays total fees withdrawn by token type
    -   Lists current active delegates
-   **Accounts Required**: Pool State PDA.

## Delegate Withdrawal System

### Overview

The Fixed Ratio Trading contract includes a comprehensive delegate withdrawal system that allows authorized delegates to withdraw trading fees collected from swaps. This system is designed with multiple security layers and full transparency.

### Key Features

#### **Multi-Delegate Support**
- **Up to 3 delegates** can be assigned by the pool owner
- **Pool owner is automatically** the first delegate upon pool creation
- **Flexible delegate management** with add/remove capabilities

#### **Security Controls**
- **24-hour cooldown period** between delegate changes (216,000 slots)
- **Daily withdrawal limits** capped at 15% of total pool balance per token
- **Pause protection** - all withdrawals blocked when pool is paused
- **Owner-only delegate management** with signature verification

#### **Fee Collection & Tracking**
- **0.3% trading fee** automatically collected on all swaps
- **Separate fee tracking** from pool liquidity
- **Real-time fee accumulation** during trading activity
- **Withdrawal limits based on collected fees and daily caps**

#### **Transparency & Auditing**
- **All operations logged publicly** with timestamp and slot information
- **Withdrawal history tracking** (last 10 transactions stored on-chain)
- **Total withdrawal tracking** for full accountability
- **Public access to withdrawal history** via `GetWithdrawalHistory`

#### **Daily Limits & Reset**
- **15% daily limit** calculated as percentage of total token balances
- **Automatic daily reset** based on slot time progression
- **Per-token tracking** (separate limits for Token A and Token B)
- **Real-time limit updates** based on current pool liquidity

### Usage Workflow

1. **Pool Creation**: Owner becomes first delegate automatically
2. **Delegate Addition**: Owner adds up to 2 additional delegates (24h cooldown)
3. **Fee Collection**: 0.3% fees collected automatically during swaps
4. **Fee Withdrawal**: Authorized delegates withdraw within daily limits
5. **Transparency**: All operations logged and publicly accessible

### Security Considerations

#### **Strong Protections**
- ✅ Owner-only delegate management with signature verification
- ✅ 24-hour cooldown prevents rapid delegate changes
- ✅ Daily withdrawal limits prevent fund drainage
- ✅ Pause functionality halts all withdrawals in emergencies
- ✅ Comprehensive logging ensures full transparency
- ✅ Rent-exempt checks prevent account closure attacks

#### **Potential Risks & Mitigations**
- ⚠️ **Owner Key Compromise**: If compromised, attacker can add malicious delegates
  - *Mitigation*: Consider multi-sig for owner operations
- ⚠️ **Delegate Collusion**: Multiple delegates could coordinate rapid withdrawals
  - *Mitigation*: Daily limits and pause functionality provide protection
- ⚠️ **Emergency Situations**: Cannot revoke delegates immediately due to cooldown
  - *Mitigation*: Pause functionality stops all withdrawals instantly

### Integration for Rewards Contracts

This system provides a foundation for future rewards contracts:

1. **Rewards contract can be assigned as a delegate**
2. **Read on-chain trade data from transaction logs**
3. **Calculate rewards based on LP token staking activity**
4. **Withdraw collected fees within daily limits**
5. **Distribute rewards to staked LP token holders**

The design enables clean separation of concerns while maintaining security and transparency.

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

### Delegate Withdrawal System Security

#### **Robust Security Features**
-   **Multi-layered Access Control**: Only pool owner can manage delegates, only delegates can withdraw fees
-   **Time-based Protections**: 24-hour cooldown between delegate changes prevents rapid manipulation
-   **Daily Withdrawal Limits**: 15% cap on daily withdrawals prevents fund drainage
-   **Emergency Controls**: Pause functionality immediately halts all withdrawals
-   **Comprehensive Auditing**: All operations logged with timestamps and delegate identification
-   **Separation of Concerns**: Trading fees tracked separately from pool liquidity
-   **Rent-exempt Validation**: Prevents account closure attacks

#### **Access Control Hierarchy**
1. **Pool Owner**: Can add/remove delegates, update security parameters, pause operations
2. **Authorized Delegates**: Can withdraw collected fees within daily limits
3. **General Users**: Can trade and provide liquidity normally

#### **Fee Collection Security**
-   **Automatic Collection**: 0.3% trading fee collected on every swap transaction
-   **Segregated Tracking**: Fees tracked separately from pool liquidity
-   **Limit Enforcement**: Withdrawals cannot exceed collected fees or daily limits
-   **Real-time Validation**: All fee calculations use checked arithmetic operations

#### **Withdrawal Controls**
-   **Daily Reset Mechanism**: Limits automatically reset based on slot progression
-   **Per-token Limits**: Separate withdrawal tracking for Token A and Token B
-   **Pause Protection**: All withdrawals blocked when pool is paused
-   **Balance Verification**: Cannot withdraw more than available collected fees

#### **Transparency & Accountability**
-   **On-chain History**: Last 10 withdrawal transactions stored on blockchain
-   **Public Access**: Anyone can query withdrawal history via `GetWithdrawalHistory`
-   **Complete Logging**: All delegate changes and withdrawals logged with full details
-   **Immutable Records**: All operations recorded in Solana transaction logs

#### **Risk Mitigation Strategies**
-   **Cooldown Periods**: Prevent rapid delegate changes and manipulation
-   **Multiple Validation Layers**: Each operation verified through multiple security checks
-   **Emergency Pause**: Owner can immediately halt all withdrawals if needed
-   **Limited Delegate Count**: Maximum 3 delegates reduces coordination attack surface
-   **Owner Privilege Separation**: Critical functions require owner signature verification

#### **Potential Attack Vectors & Defenses**
1. **Owner Key Compromise**: 
   - *Risk*: Attacker could add malicious delegates
   - *Defense*: 24-hour cooldown, daily limits, pause functionality, consider multi-sig
2. **Delegate Collusion**: 
   - *Risk*: Multiple delegates coordinate rapid withdrawals
   - *Defense*: Daily limits, pause functionality, withdrawal history tracking
3. **Flash Loan Attacks**: 
   - *Risk*: Manipulate pool to increase withdrawal limits
   - *Defense*: Limits based on actual liquidity, not borrowed funds
4. **Reentrancy**: 
   - *Risk*: Multiple calls to withdrawal function
   - *Defense*: Solana's account-based model prevents traditional reentrancy
5. **Time Manipulation**: 
   - *Risk*: Exploit slot time for limit resets
   - *Defense*: Conservative slot-to-time conversion, blockchain time immutability

#### **Recommended Security Practices**
-   **Multi-signature Implementation**: Use multi-sig for pool owner operations
-   **Regular Monitoring**: Monitor withdrawal patterns and delegate activity
-   **Delegate Vetting**: Carefully select and verify delegate addresses
-   **Emergency Procedures**: Establish clear protocols for using pause functionality
-   **Audit Trail Review**: Regularly review withdrawal history for anomalies
-   **Key Management**: Secure storage and backup of owner keys 