# Technical Implementation Details

This document contains detailed technical implementation information for the Fixed Ratio Trading Pool smart contract.

## Table of Contents
- [Anti-Liquidity Fragmentation Implementation](#anti-liquidity-fragmentation-implementation)
- [Token Normalization Algorithm](#token-normalization-algorithm)
- [PDA Derivation](#pda-derivation)
- [Pool State Structure](#pool-state-structure)
- [Account Validation](#account-validation)
- [Mathematical Formulas](#mathematical-formulas)

## Anti-Liquidity Fragmentation Implementation

### Problem Statement

Market fragmentation occurs when multiple pools exist for the same economic relationship, splitting liquidity and creating inefficiencies:

```rust
// ❌ THESE SCENARIOS ARE PREVENTED:
// Pool 1: TokenA/TokenB with ratio 3:1 (3 A per 1 B)  
// Pool 2: TokenB/TokenA with ratio 1:3 (1 B per 3 A) ← Same economic rate!

// Pool 1: USDC/SOL with ratio 100:1 (100 USDC per 1 SOL)
// Pool 2: SOL/USDC with any ratio ← BLOCKED - same token pair!
```

### Technical Solution

The contract uses **enhanced normalization** to detect and prevent economically equivalent pools:

1. **Token Normalization**: All token pairs are ordered lexicographically (A < B)
2. **Canonical Mapping**: Both "A/B" and "B/A" pools normalize to the same configuration
3. **PDA Collision**: Duplicate pools result in the same PDA, causing creation to fail
4. **Atomic Prevention**: Second pool creation fails at the account level

### Implementation Details

```rust
// Both of these pool configurations:
normalize_pool_config(&token_a, &token_b, ratio_x)  // Pool 1
normalize_pool_config(&token_b, &token_a, ratio_x)  // Pool 2

// Result in IDENTICAL normalized configuration:
// - Same token_a_mint (lexicographically smaller)
// - Same token_b_mint (lexicographically larger)  
// - Same pool_state_pda
// - Same canonical ratio representation
```

### For Developers

When creating pools, remember:
- ✅ **First pool created**: Successfully establishes the token pair
- ❌ **Second pool attempt**: Will fail with account already exists error
- 🔍 **Pool lookup**: Use either token order - both resolve to same pool
- 🎯 **Integration**: No need to worry about multiple pools for same pair

```rust
// Example: All these attempts reference the same pool
let pool_usdc_sol = derive_pool_pda(&usdc_mint, &sol_mint, ratio);
let pool_sol_usdc = derive_pool_pda(&sol_mint, &usdc_mint, ratio);
// pool_usdc_sol == pool_sol_usdc ✅

// Only the first creation succeeds
create_pool(&usdc_mint, &sol_mint, ratio_1); // ✅ Success
create_pool(&sol_mint, &usdc_mint, ratio_2); // ❌ Fails - same token pair
```

## Token Normalization Algorithm

### Core Algorithm

```rust
pub fn normalize_pool_tokens(
    input_token_a: &Pubkey,
    input_token_b: &Pubkey,
    input_ratio: u64,
) -> Result<(Pubkey, Pubkey, u64, bool), ProgramError> {
    // Step 1: Lexicographic ordering
    let (token_a, token_b, token_a_is_the_multiple) = if input_token_a < input_token_b {
        (*input_token_a, *input_token_b, determine_multiple_token(input_token_a, input_ratio))
    } else {
        (*input_token_b, *input_token_a, determine_multiple_token(input_token_b, input_ratio))
    };
    
    // Step 2: Calculate canonical ratio
    let canonical_ratio = if token_a_is_the_multiple {
        input_ratio  // TokenA is multiple, ratio is correct
    } else {
        // TokenB is multiple, ratio needs adjustment
        calculate_inverse_ratio(input_ratio)?
    };
    
    Ok((token_a, token_b, canonical_ratio, token_a_is_the_multiple))
}
```

### Ratio Normalization

The contract normalizes ratios to always represent "multiple tokens per base token":

```rust
// Input: 1 SOL = 100 USDC
// If SOL < USDC lexicographically:
//   - token_a = SOL (base), token_b = USDC (multiple)
//   - ratio = 100 (USDC per SOL)
//   - token_a_is_the_multiple = false

// Input: 100 USDC = 1 SOL  
// If SOL < USDC lexicographically:
//   - token_a = SOL (base), token_b = USDC (multiple)
//   - ratio = 100 (USDC per SOL) 
//   - token_a_is_the_multiple = false
```

## PDA Derivation

### Pool State PDA

```rust
pub fn derive_pool_state_pda(
    token_a_mint: &Pubkey,
    token_b_mint: &Pubkey,
    multiple_per_base: u64,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    let seeds = &[
        b"pool_state",
        token_a_mint.as_ref(),
        token_b_mint.as_ref(),
        &multiple_per_base.to_le_bytes(),
    ];
    
    Pubkey::find_program_address(seeds, program_id)
}
```

### Token Vault PDAs

```rust
pub fn derive_token_vault_pda(
    pool_state_pda: &Pubkey,
    token_mint: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    let seeds = &[
        b"token_vault",
        pool_state_pda.as_ref(),
        token_mint.as_ref(),
    ];
    
    Pubkey::find_program_address(seeds, program_id)
}
```

### LP Token Mint PDA

```rust
pub fn derive_lp_token_mint_pda(
    pool_state_pda: &Pubkey,
    token_mint: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    let seeds = &[
        b"lp_token_mint",
        pool_state_pda.as_ref(),
        token_mint.as_ref(),
    ];
    
    Pubkey::find_program_address(seeds, program_id)
}
```

## Pool State Structure

### Account Layout

```rust
#[account]
pub struct PoolState {
    /// Pool authority (owner)
    pub pool_authority: Pubkey,
    
    /// Token A mint (lexicographically smaller)
    pub token_a_mint: Pubkey,
    
    /// Token B mint (lexicographically larger) 
    pub token_b_mint: Pubkey,
    
    /// Multiple tokens per base token ratio
    pub multiple_per_base: u64,
    
    /// Whether token A is the multiple token
    pub token_a_is_the_multiple: bool,
    
    /// Token vault accounts
    pub multiple_token_vault: Pubkey,
    pub base_token_vault: Pubkey,
    
    /// LP token mints
    pub lp_token_mint_a: Pubkey,
    pub lp_token_mint_b: Pubkey,
    
    /// Pool fees
    pub swap_fee_basis_points: u16,
    pub collected_fee_a: u64,
    pub collected_fee_b: u64,
    
    /// Pool controls
    pub is_swap_paused: bool,
    
    /// PDA bump seeds
    pub pool_authority_bump_seed: u8,
    pub multiple_token_vault_bump_seed: u8,
    pub base_token_vault_bump_seed: u8,
    pub lp_token_mint_a_bump_seed: u8,
    pub lp_token_mint_b_bump_seed: u8,
}
```

### Size Calculation

```rust
impl PoolState {
    pub const LEN: usize = 8 + // discriminator
        32 + // pool_authority
        32 + // token_a_mint  
        32 + // token_b_mint
        8 +  // multiple_per_base
        1 +  // token_a_is_the_multiple
        32 + // multiple_token_vault
        32 + // base_token_vault
        32 + // lp_token_mint_a
        32 + // lp_token_mint_b
        2 +  // swap_fee_basis_points
        8 +  // collected_fee_a
        8 +  // collected_fee_b
        1 +  // is_swap_paused
        1 +  // pool_authority_bump_seed
        1 +  // multiple_token_vault_bump_seed
        1 +  // base_token_vault_bump_seed
        1 +  // lp_token_mint_a_bump_seed
        1;   // lp_token_mint_b_bump_seed
}
```

## Account Validation

### Pool State Validation

```rust
pub fn validate_pool_state_account(
    pool_state_account: &AccountInfo,
    expected_pda: &Pubkey,
    program_id: &Pubkey,
) -> ProgramResult {
    // Check account owner
    if pool_state_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    
    // Check PDA address
    if pool_state_account.key != expected_pda {
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Check account is initialized
    if pool_state_account.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }
    
    Ok(())
}
```

### Token Account Validation

```rust
pub fn validate_token_account(
    token_account: &AccountInfo,
    expected_mint: &Pubkey,
    expected_owner: &Pubkey,
) -> ProgramResult {
    let account_data = TokenAccount::unpack(&token_account.data.borrow())?;
    
    // Check mint
    if account_data.mint != *expected_mint {
        return Err(TokenError::MintMismatch.into());
    }
    
    // Check owner
    if account_data.owner != *expected_owner {
        return Err(TokenError::OwnerMismatch.into());
    }
    
    Ok(())
}
```

## Mathematical Formulas

### Swap Calculations

#### Multiple to Base Token Swap

```rust
pub fn calculate_multiple_to_base_output(
    input_multiple_amount: u64,
    multiple_per_base_ratio: u64,
    fee_basis_points: u16,
) -> Result<u64, ProgramError> {
    // Apply trading fee
    let fee_amount = input_multiple_amount
        .checked_mul(fee_basis_points as u64)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_div(10_000)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    let effective_input = input_multiple_amount
        .checked_sub(fee_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    // Calculate base token output
    let base_output = effective_input
        .checked_div(multiple_per_base_ratio)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    Ok(base_output)
}
```

#### Base to Multiple Token Swap

```rust
pub fn calculate_base_to_multiple_output(
    input_base_amount: u64,
    multiple_per_base_ratio: u64,
    fee_basis_points: u16,
) -> Result<u64, ProgramError> {
    // Calculate multiple token output before fee
    let gross_output = input_base_amount
        .checked_mul(multiple_per_base_ratio)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    // Apply trading fee
    let fee_amount = gross_output
        .checked_mul(fee_basis_points as u64)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_div(10_000)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    let net_output = gross_output
        .checked_sub(fee_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    Ok(net_output)
}
```

### Liquidity Calculations

#### LP Token Minting

```rust
pub fn calculate_lp_tokens_to_mint(
    deposit_amount: u64,
    current_token_balance: u64,
    total_lp_supply: u64,
) -> Result<u64, ProgramError> {
    if total_lp_supply == 0 {
        // First deposit: mint LP tokens equal to deposit amount
        Ok(deposit_amount)
    } else {
        // Subsequent deposits: proportional to existing liquidity
        let lp_tokens = deposit_amount
            .checked_mul(total_lp_supply)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .checked_div(current_token_balance)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        
        Ok(lp_tokens)
    }
}
```

#### Withdrawal Calculations

```rust
pub fn calculate_withdrawal_amount(
    lp_tokens_to_burn: u64,
    total_lp_supply: u64,
    current_token_balance: u64,
) -> Result<u64, ProgramError> {
    let withdrawal_amount = lp_tokens_to_burn
        .checked_mul(current_token_balance)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_div(total_lp_supply)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    Ok(withdrawal_amount)
}
```

### Fee Calculations

```rust
pub fn calculate_trading_fee(
    amount: u64,
    fee_basis_points: u16,
) -> Result<u64, ProgramError> {
    let fee = amount
        .checked_mul(fee_basis_points as u64)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_div(10_000)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    Ok(fee)
}
``` 