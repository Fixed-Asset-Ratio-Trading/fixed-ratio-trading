/*
MIT License

Copyright (c) 2024 Davinci

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

// This is the main library for the fixed-ratio-trading program
// It contains the program's instructions, error handling, and other functionality
// It also contains the program's constants and PDA seeds
// It is used by the program's entrypoint and other modules


use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
    program_pack::Pack,
    clock::Clock,
    declare_id,
};
use spl_token::{
    instruction as token_instruction,
    state::{Account as TokenAccount, Mint as MintAccount},
};
use std::fmt;

declare_id!("quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD");

// Constants for fees
const REGISTRATION_FEE: u64 = 1_150_000_000; // 1.15 SOL
const DEPOSIT_WITHDRAWAL_FEE: u64 = 1_300_000; // 0.0013 SOL
const SWAP_FEE: u64 = 12_500; // 0.0000125 SOL

// Swap fee configuration constants
const MAX_SWAP_FEE_BASIS_POINTS: u64 = 50; // 0.5% maximum
const FEE_BASIS_POINTS_DENOMINATOR: u64 = 10000; // 1 basis point = 0.01%

// Delegate system constants
const MAX_DELEGATES: usize = 3;
// REMOVED: const DELEGATE_CHANGE_COOLDOWN_SLOTS: u64 = 216_000; // Approximately 24 hours at 400ms slots

// PDA Seeds
pub const POOL_STATE_SEED_PREFIX: &[u8] = b"pool_state_v2";
pub const TOKEN_A_VAULT_SEED_PREFIX: &[u8] = b"token_a_vault";
pub const TOKEN_B_VAULT_SEED_PREFIX: &[u8] = b"token_b_vault";

// Add constant for SPL Token Program ID
// const SPL_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

// Add after the existing constants
pub const MINIMUM_RENT_BUFFER: u64 = 1000; // Additional buffer for rent to account for potential rent increases

#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
pub struct RentRequirements {
    pub last_update_slot: u64,
    pub rent_exempt_minimum: u64,
    pub pool_state_rent: u64,
    pub token_vault_rent: u64,
    pub lp_mint_rent: u64,
}

impl RentRequirements {
    pub fn new(rent: &Rent) -> Self {
        Self {
            last_update_slot: 0,
            rent_exempt_minimum: rent.minimum_balance(0),
            pool_state_rent: rent.minimum_balance(PoolState::get_packed_len()),
            token_vault_rent: rent.minimum_balance(TokenAccount::LEN),
            lp_mint_rent: rent.minimum_balance(MintAccount::LEN),
        }
    }

    pub fn update_if_needed(&mut self, rent: &Rent, current_slot: u64) -> bool {
        // Update rent requirements if they've changed or if it's been a while
        let needs_update = self.last_update_slot == 0 || 
                          current_slot - self.last_update_slot > 1000 || // Update every ~1000 slots
                          self.pool_state_rent != rent.minimum_balance(PoolState::get_packed_len()) ||
                          self.token_vault_rent != rent.minimum_balance(TokenAccount::LEN) ||
                          self.lp_mint_rent != rent.minimum_balance(MintAccount::LEN);

        if needs_update {
            self.pool_state_rent = rent.minimum_balance(PoolState::get_packed_len());
            self.token_vault_rent = rent.minimum_balance(TokenAccount::LEN);
            self.lp_mint_rent = rent.minimum_balance(MintAccount::LEN);
            self.last_update_slot = current_slot;
        }

        needs_update
    }

    pub fn get_total_required_rent(&self) -> u64 {
        self.pool_state_rent + 
        (2 * self.token_vault_rent) + // Two token vaults
        (2 * self.lp_mint_rent) + // Two LP mints
        MINIMUM_RENT_BUFFER // Additional buffer
    }

    pub fn get_packed_len() -> usize {
        8 + // last_update_slot
        8 + // rent_exempt_minimum
        8 + // pool_state_rent
        8 + // token_vault_rent
        8   // lp_mint_rent
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
pub struct PoolState {
    pub owner: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub token_a_vault: Pubkey,
    pub token_b_vault: Pubkey,
    pub lp_token_a_mint: Pubkey,
    pub lp_token_b_mint: Pubkey,
    pub ratio_a_numerator: u64,
    pub ratio_b_denominator: u64,
    pub total_token_a_liquidity: u64,
    pub total_token_b_liquidity: u64,
    pub pool_authority_bump_seed: u8,
    pub token_a_vault_bump_seed: u8,
    pub token_b_vault_bump_seed: u8,
    pub is_initialized: bool,
    pub rent_requirements: RentRequirements,
    pub is_paused: bool,
    pub delegate_management: DelegateManagement,
    pub collected_fees_token_a: u64,
    pub collected_fees_token_b: u64,
    pub total_fees_withdrawn_token_a: u64,
    pub total_fees_withdrawn_token_b: u64,
    pub swap_fee_basis_points: u64, // Fee in basis points (0-50, representing 0%-0.5%)
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum FixedRatioInstruction {
    /// WORKAROUND FOR SOLANA ACCOUNTINFO.DATA ISSUE:
    /// 
    /// The following two instructions implement a workaround for a known issue in Solana
    /// where AccountInfo.data is not properly updated after CPI account creation within
    /// the same instruction. This issue was documented in:
    /// - GitHub Issue #31960: https://github.com/solana-labs/solana/issues/31960
    /// - Solana Stack Exchange discussions on AccountInfo.data not reflecting CPI changes
    /// 
    /// ROOT CAUSE:
    /// When using system_instruction::create_account via CPI to create a PDA account,
    /// the AccountInfo.data slice for that account does not get updated to reflect the
    /// newly allocated memory buffer. This causes:
    /// 1. AccountInfo.data.borrow().len() to return 0 even after successful account creation
    /// 2. Serialization to AccountInfo.data.borrow_mut() to report "OK" but write to a 
    ///    detached buffer that doesn't represent the actual on-chain account data
    /// 3. Subsequent account fetches (e.g., banks_client.get_account()) to return empty data
    /// 
    /// SOLUTION:
    /// Split the operation into two separate instructions to ensure AccountInfo references
    /// are fresh and properly point to the allocated on-chain account data.
    
    /// Step 1: Creates the Pool State PDA account and all related accounts (LP mints, vaults)
    /// 
    /// This instruction performs all CPI account creation operations:
    /// - Creates Pool State PDA with correct size allocation
    /// - Creates LP token mints and transfers authority to pool
    /// - Creates token vault PDAs and initializes them
    /// - Transfers registration fees
    /// 
    /// IMPORTANT: This instruction does NOT attempt to serialize PoolState data to avoid
    /// the AccountInfo.data issue described above.
    CreatePoolStateAccount {
        ratio_primary_per_base: u64,
        pool_authority_bump_seed: u8,
        primary_token_vault_bump_seed: u8,
        base_token_vault_bump_seed: u8,
    },
    
    /// Step 2: Initializes the data in the already-created Pool State PDA account
    /// 
    /// This instruction runs in a fresh transaction context where:
    /// - AccountInfo.data properly references the on-chain allocated account buffer
    /// - Serialization operations work correctly
    /// - Pool state data can be safely written and will persist on-chain
    /// 
    /// Uses a buffer-copy approach as an additional safeguard against any remaining
    /// AccountInfo.data inconsistencies.
    InitializePoolData {
        ratio_primary_per_base: u64,
        pool_authority_bump_seed: u8,
        primary_token_vault_bump_seed: u8,
        base_token_vault_bump_seed: u8,
    },
    Deposit {
        deposit_token_mint: Pubkey,
        amount: u64,
    },
    Withdraw {
        withdraw_token_mint: Pubkey,
        lp_amount_to_burn: u64,
    },
    Swap {
        input_token_mint: Pubkey,
        amount_in: u64,
        minimum_amount_out: u64,
    },
    WithdrawFees,
    UpdateSecurityParams {
        max_withdrawal_percentage: Option<u64>,
        withdrawal_cooldown: Option<u64>,
        is_paused: Option<bool>,
    },
    /// Delegate Management Instructions
    AddDelegate {
        delegate: Pubkey,
    },
    RemoveDelegate {
        delegate: Pubkey,
    },
    /// Fee withdrawal by delegates
    WithdrawFeesToDelegate {
        token_mint: Pubkey,
        amount: u64,
    },
    /// Set swap fee configuration (owner only, max 0.5%)
    SetSwapFee {
        fee_basis_points: u64, // Fee in basis points (0-50)
    },
    /// Get withdrawal history (for transparency)
    GetWithdrawalHistory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolError {
    InvalidTokenPair {
        token_a: Pubkey,
        token_b: Pubkey,
        reason: String,
    },
    InvalidRatio {
        ratio: u64,
        min_ratio: u64,
        max_ratio: u64,
    },
    InsufficientFunds {
        required: u64,
        available: u64,
        account: Pubkey,
    },
    InvalidTokenAccount {
        account: Pubkey,
        reason: String,
    },
    InvalidSwapAmount {
        amount: u64,
        min_amount: u64,
        max_amount: u64,
    },
    RentExemptError {
        account: Pubkey,
        required: u64,
        available: u64,
    },
    WithdrawalTooLarge,
    WithdrawalCooldown,
    PoolPaused,
    DelegateLimitExceeded,
    DelegateAlreadyExists { delegate: Pubkey },
    DelegateNotFound { delegate: Pubkey },
}

impl fmt::Display for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PoolError::InvalidTokenPair { token_a, token_b, reason } => {
                write!(f, "Invalid token pair: {} and {}. Reason: {}", token_a, token_b, reason)
            },
            PoolError::InvalidRatio { ratio, min_ratio, max_ratio } => {
                write!(f, "Invalid ratio: {}. Must be between {} and {}", ratio, min_ratio, max_ratio)
            },
            PoolError::InsufficientFunds { required, available, account } => {
                write!(f, "Insufficient funds: Required {}, Available {}, Account {}", required, available, account)
            },
            PoolError::InvalidTokenAccount { account, reason } => {
                write!(f, "Invalid token account: Account {}, Reason: {}", account, reason)
            },
            PoolError::InvalidSwapAmount { amount, min_amount, max_amount } => {
                write!(f, "Invalid swap amount: {} is not between {} and {}", amount, min_amount, max_amount)
            },
            PoolError::RentExemptError { account, required, available } => {
                write!(f, "Insufficient funds: Required {}, Available {}, Account {}", required, available, account)
            },
            PoolError::WithdrawalTooLarge => write!(f, "Withdrawal amount exceeds maximum allowed percentage"),
            PoolError::WithdrawalCooldown => write!(f, "Withdrawal is currently in cooldown period"),
            PoolError::PoolPaused => write!(f, "Pool operations are currently paused"),
            PoolError::DelegateLimitExceeded => write!(f, "Delegate limit exceeded"),
            PoolError::DelegateAlreadyExists { delegate } => write!(f, "Delegate already exists: {}", delegate),
            PoolError::DelegateNotFound { delegate } => write!(f, "Delegate not found: {}", delegate),
        }
    }
}

impl PoolError {
    pub fn error_code(&self) -> u32 {
        match self {
            PoolError::InvalidTokenPair { .. } => 1001,
            PoolError::InvalidRatio { .. } => 1002,
            PoolError::InsufficientFunds { .. } => 1003,
            PoolError::InvalidTokenAccount { .. } => 1004,
            PoolError::InvalidSwapAmount { .. } => 1005,
            PoolError::RentExemptError { .. } => 1006,
            PoolError::WithdrawalTooLarge => 1007,
            PoolError::WithdrawalCooldown => 1008,
            PoolError::PoolPaused => 1009,
            PoolError::DelegateLimitExceeded => 1010,
            PoolError::DelegateAlreadyExists { .. } => 1011,
            PoolError::DelegateNotFound { .. } => 1012,
        }
    }
}

impl From<PoolError> for ProgramError {
    fn from(e: PoolError) -> Self {
        ProgramError::Custom(e.error_code())
    }
}

/// Entry point for Solana program instructions.
///
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for the operation
/// * `instruction_data` - The instruction data containing the operation to perform
///
/// # Returns
/// * `ProgramResult` - Success or error code
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("DEBUG: process_instruction: Entered. Program ID: {}, Instruction data len: {}", program_id, instruction_data.len());
    let instruction = match FixedRatioInstruction::try_from_slice(instruction_data) {
        Ok(instr) => {
            msg!("DEBUG: process_instruction: Successfully deserialized instruction.");
            instr
        }
        Err(e) => {
            msg!("DEBUG: process_instruction: Failed to deserialize instruction_data: {:?}", e);
            return Err(e.into());
        }
    };
    
    // Check if pool is paused for all instructions except WithdrawFees, UpdateSecurityParams, CreatePoolStateAccount, and InitializePoolData
    if let FixedRatioInstruction::WithdrawFees 
        | FixedRatioInstruction::UpdateSecurityParams { .. }
        | FixedRatioInstruction::CreatePoolStateAccount { .. }
        | FixedRatioInstruction::InitializePoolData { .. } = instruction {
        msg!("DEBUG: process_instruction: Skipping pause check for pool creation/management instructions.");
    } else {
        msg!("DEBUG: process_instruction: Checking pause state for relevant instruction.");
        let account_info_iter_for_pause_check = &mut accounts.iter();
        let pool_state_account_for_pause_check = next_account_info(account_info_iter_for_pause_check)?;
        match PoolState::try_from_slice(&pool_state_account_for_pause_check.data.borrow()) {
            Ok(pool_state_data_for_pause) => {
                if pool_state_data_for_pause.is_paused {
                    msg!("DEBUG: process_instruction: Pool is paused. Instruction prohibited.");
                    return Err(PoolError::PoolPaused.into());
                }
                msg!("DEBUG: process_instruction: Pool is not paused or instruction allows paused state.");
            }
            Err(e) => {
                msg!("DEBUG: process_instruction: Failed to deserialize PoolState for pause check: {:?}. Key: {}", e, pool_state_account_for_pause_check.key);
            }
        }
    }
    
    match instruction {
        FixedRatioInstruction::CreatePoolStateAccount { 
            ratio_primary_per_base, 
            pool_authority_bump_seed, 
            primary_token_vault_bump_seed, 
            base_token_vault_bump_seed 
        } => {
            msg!("DEBUG: process_instruction: Dispatching to process_create_pool_state_account");
            process_create_pool_state_account(
                program_id, 
                accounts, 
                ratio_primary_per_base, 
                pool_authority_bump_seed, 
                primary_token_vault_bump_seed, 
                base_token_vault_bump_seed
            )
        }
        FixedRatioInstruction::InitializePoolData { 
            ratio_primary_per_base, 
            pool_authority_bump_seed, 
            primary_token_vault_bump_seed, 
            base_token_vault_bump_seed 
        } => {
            msg!("DEBUG: process_instruction: Dispatching to process_initialize_pool_data");
            process_initialize_pool_data(
                program_id, 
                accounts, 
                ratio_primary_per_base, 
                pool_authority_bump_seed, 
                primary_token_vault_bump_seed, 
                base_token_vault_bump_seed
            )
        }
        FixedRatioInstruction::Deposit { deposit_token_mint, amount } => {
            msg!("DEBUG: process_instruction: Dispatching to process_deposit");
            process_deposit(program_id, accounts, deposit_token_mint, amount)
        }
        FixedRatioInstruction::Withdraw { withdraw_token_mint, lp_amount_to_burn } => {
            msg!("DEBUG: process_instruction: Dispatching to process_withdraw");
            process_withdraw(program_id, accounts, withdraw_token_mint, lp_amount_to_burn)
        }
        FixedRatioInstruction::Swap { input_token_mint, amount_in, minimum_amount_out } => {
            msg!("DEBUG: process_instruction: Dispatching to process_swap");
            process_swap(program_id, accounts, input_token_mint, amount_in, minimum_amount_out)
        }
        FixedRatioInstruction::WithdrawFees => {
            msg!("DEBUG: process_instruction: Dispatching to process_withdraw_fees");
            process_withdraw_fees(program_id, accounts)
        }
        FixedRatioInstruction::UpdateSecurityParams { 
            max_withdrawal_percentage, 
            withdrawal_cooldown, 
            is_paused 
        } => {
            msg!("DEBUG: process_instruction: Dispatching to process_update_security_params");
            process_update_security_params(
                program_id,
                accounts,
                max_withdrawal_percentage,
                withdrawal_cooldown,
                is_paused
            )
        }
        FixedRatioInstruction::AddDelegate { delegate } => {
            msg!("DEBUG: process_instruction: Dispatching to process_add_delegate");
            process_add_delegate(program_id, accounts, delegate)
        }
        FixedRatioInstruction::RemoveDelegate { delegate } => {
            msg!("DEBUG: process_instruction: Dispatching to process_remove_delegate");
            process_remove_delegate(program_id, accounts, delegate)
        }
        FixedRatioInstruction::WithdrawFeesToDelegate { token_mint, amount } => {
            msg!("DEBUG: process_instruction: Dispatching to process_withdraw_fees_to_delegate");
            process_withdraw_fees_to_delegate(program_id, accounts, token_mint, amount)
        }
        FixedRatioInstruction::SetSwapFee { fee_basis_points } => {
            msg!("DEBUG: process_instruction: Dispatching to process_set_swap_fee");
            process_set_swap_fee(program_id, accounts, fee_basis_points)
        }
        FixedRatioInstruction::GetWithdrawalHistory => {
            msg!("DEBUG: process_instruction: Dispatching to process_get_withdrawal_history");
            process_get_withdrawal_history(program_id, accounts)
        }
    }
}

/// Checks if an account is rent-exempt. For program-owned accounts, uses rent tracking; otherwise, checks minimum balance.
///
/// # Arguments
/// * `account` - The account to check
/// * `program_id` - The program ID
/// * `rent` - The rent sysvar
/// * `current_slot` - The current slot
///
/// # Returns
/// * `ProgramResult` - Success or error code
pub fn check_rent_exempt(account: &AccountInfo, program_id: &Pubkey, rent: &Rent, current_slot: u64) -> ProgramResult {
    // Check if the account is owned by the program
    if account.owner == program_id {
        // For program-owned accounts, use the new rent tracking mechanism
        ensure_rent_exempt(account, rent, current_slot)
    } else {
        // For other accounts, use the simple check
        let minimum_balance = rent.minimum_balance(account.data_len());
        if account.lamports() < minimum_balance {
            msg!("Account {} below rent-exempt threshold. Required: {}, Current: {}", 
                 account.key, minimum_balance, account.lamports());
            return Err(ProgramError::InsufficientFunds);
        }
        Ok(())
    }
}

/// Creates the Pool State PDA account and all related accounts (LP mints, vaults).
/// This is Step 1 of the two-instruction pool initialization pattern.
///
/// WORKAROUND CONTEXT:
/// This function implements the first part of a workaround for Solana AccountInfo.data
/// issue where AccountInfo.data doesn't get updated after CPI account creation within
/// the same instruction. See GitHub Issue #31960 and related community discussions.
///
/// WHY THIS APPROACH:
/// 1. Creates all required accounts via CPI (Pool State PDA, LP mints, token vaults)
/// 2. Deliberately AVOIDS writing PoolState data to prevent AccountInfo.data issues
/// 3. Allows the second instruction (InitializePoolData) to run with fresh AccountInfo
///    references that properly point to the allocated on-chain account buffers
///
/// WHAT THIS FUNCTION DOES:
/// - Validates all input parameters and PDA derivations
/// - Creates Pool State PDA account with correct size via system_instruction::create_account
/// - Creates and initializes LP token mints, transfers authority to pool
/// - Creates and initializes token vault PDAs
/// - Transfers registration fees to pool
/// - Does NOT serialize any PoolState data (that's done in Step 2)
///
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for pool creation
/// * `ratio_primary_per_base` - The ratio of primary tokens per base token
/// * `pool_authority_bump_seed` - Bump seed for pool authority PDA
/// * `primary_token_vault_bump_seed` - Bump seed for primary token vault PDA
/// * `base_token_vault_bump_seed` - Bump seed for base token vault PDA
///
/// # Returns
/// * `ProgramResult` - Success or error code
fn process_create_pool_state_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ratio_primary_per_base: u64,
    pool_authority_bump_seed: u8,
    primary_token_vault_bump_seed: u8,
    base_token_vault_bump_seed: u8,
) -> ProgramResult {
    msg!("DEBUG: process_create_pool_state_account: Entered");
    let account_info_iter = &mut accounts.iter();

    let payer = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Payer: {}", payer.key);
    let pool_state_pda_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Pool State PDA Account (from client): {}", pool_state_pda_account.key);
    let primary_token_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Primary Token Mint Account: {}", primary_token_mint_account.key);
    let base_token_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Base Token Mint Account: {}", base_token_mint_account.key);
    let lp_token_a_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: LP Token A Mint Account: {}", lp_token_a_mint_account.key);
    let lp_token_b_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: LP Token B Mint Account: {}", lp_token_b_mint_account.key);
    let token_a_vault_pda_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Token A Vault PDA Account (from client): {}", token_a_vault_pda_account.key);
    let token_b_vault_pda_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Token B Vault PDA Account (from client): {}", token_b_vault_pda_account.key);
    let system_program_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: System Program Account: {}", system_program_account.key);
    let token_program_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Token Program Account: {}", token_program_account.key);
    let rent_sysvar_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_create_pool_state_account: Rent Sysvar Account: {}", rent_sysvar_account.key);
    
    msg!("DEBUG: process_create_pool_state_account: Parsed all accounts");

    let rent = &Rent::from_account_info(rent_sysvar_account)?;

    // Verify that payer is a signer
    if !payer.is_signer {
        msg!("DEBUG: process_create_pool_state_account: Payer is not a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }
    msg!("DEBUG: process_create_pool_state_account: Payer is signer check passed");

    // Verify ratio is non-zero
    if ratio_primary_per_base == 0 {
        msg!("DEBUG: process_create_pool_state_account: Ratio cannot be zero");
        return Err(ProgramError::InvalidArgument);
    }
    msg!("DEBUG: process_create_pool_state_account: Ratio is non-zero check passed");

    // Normalize tokens and ratio
    msg!("DEBUG: process_create_pool_state_account: Normalizing tokens and ratio...");
    let (token_a_mint_key, token_b_mint_key, ratio_a_numerator, ratio_b_denominator, token_a_is_primary) = 
        if primary_token_mint_account.key < base_token_mint_account.key {
            msg!("DEBUG: process_create_pool_state_account: Primary mint < Base mint");
            (primary_token_mint_account.key, base_token_mint_account.key, ratio_primary_per_base, 1, true)
        } else {
            msg!("DEBUG: process_create_pool_state_account: Primary mint > Base mint");
            (base_token_mint_account.key, primary_token_mint_account.key, 1, ratio_primary_per_base, false)
        };

    msg!("DEBUG: process_create_pool_state_account: Normalized: token_a_mint_key={}, token_b_mint_key={}, ratio_a_num={}, ratio_b_den={}", 
         token_a_mint_key, token_b_mint_key, ratio_a_numerator, ratio_b_denominator);

    let token_a_mint_account_info_ref = if token_a_is_primary { primary_token_mint_account } else { base_token_mint_account };
    let token_b_mint_account_info_ref = if token_a_is_primary { base_token_mint_account } else { primary_token_mint_account };
    msg!("DEBUG: process_create_pool_state_account: Set token_a/b_mint_account_info_refs");

    // Validate mint accounts
    if !primary_token_mint_account.owner.eq(&spl_token::id()) || primary_token_mint_account.data_len() != MintAccount::LEN {
        msg!("DEBUG: process_create_pool_state_account: Primary token mint account is not a valid mint account");
        return Err(ProgramError::InvalidAccountData);
    }

    if !base_token_mint_account.owner.eq(&spl_token::id()) || base_token_mint_account.data_len() != MintAccount::LEN {
        msg!("DEBUG: process_create_pool_state_account: Base token mint account is not a valid mint account");
        return Err(ProgramError::InvalidAccountData);
    }
    msg!("DEBUG: process_create_pool_state_account: Mint account validations passed");

    // Verify the pool state PDA is derived correctly using normalized values
    msg!("DEBUG: process_create_pool_state_account: Verifying Pool State PDA. Pool Auth Bump Seed from instr: {}", pool_authority_bump_seed);
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        token_a_mint_key.as_ref(),
        token_b_mint_key.as_ref(),
        &ratio_a_numerator.to_le_bytes(),
        &ratio_b_denominator.to_le_bytes(),
        &[pool_authority_bump_seed],
    ];
    let expected_pool_state_pda = Pubkey::create_program_address(pool_state_pda_seeds, program_id)?;
    msg!("DEBUG: process_create_pool_state_account: Expected Pool State PDA (program derived): {}", expected_pool_state_pda);
    if *pool_state_pda_account.key != expected_pool_state_pda {
        msg!("DEBUG: process_create_pool_state_account: Invalid Pool State PDA address. Expected {}, got {}", expected_pool_state_pda, pool_state_pda_account.key);
        return Err(ProgramError::InvalidArgument);
    }
    msg!("DEBUG: process_create_pool_state_account: Pool State PDA address verification passed.");

    // Check if pool state already exists
    msg!("DEBUG: process_create_pool_state_account: Checking if pool state already exists. Data len: {}", pool_state_pda_account.data_len());
    if pool_state_pda_account.data_len() > 0 && !pool_state_pda_account.data_is_empty() {
        msg!("DEBUG: process_create_pool_state_account: Pool state account already exists");
        return Err(ProgramError::AccountAlreadyInitialized);
    } else {
        msg!("DEBUG: process_create_pool_state_account: Pool state PDA account is empty, proceeding with creation.");
    }

    // Map vault bump seeds
    msg!("DEBUG: process_create_pool_state_account: Mapping vault bump seeds. Primary Vault Bump: {}, Base Vault Bump: {}", primary_token_vault_bump_seed, base_token_vault_bump_seed);
    let (token_a_vault_bump, token_b_vault_bump) = if token_a_is_primary {
        (primary_token_vault_bump_seed, base_token_vault_bump_seed)
    } else {
        (base_token_vault_bump_seed, primary_token_vault_bump_seed)
    };
    msg!("DEBUG: process_create_pool_state_account: Normalized token_a_vault_bump: {}, token_b_vault_bump: {}", token_a_vault_bump, token_b_vault_bump);

    // Verify vault PDAs
    msg!("DEBUG: process_create_pool_state_account: Verifying Token A Vault PDA...");
    let token_a_vault_pda_seeds = &[
        TOKEN_A_VAULT_SEED_PREFIX,
        pool_state_pda_account.key.as_ref(),
        &[token_a_vault_bump],
    ];
    let expected_token_a_vault_pda = Pubkey::create_program_address(token_a_vault_pda_seeds, program_id)?;
    msg!("DEBUG: process_create_pool_state_account: Expected Token A Vault PDA (program derived): {}", expected_token_a_vault_pda);
    if *token_a_vault_pda_account.key != expected_token_a_vault_pda {
        msg!("DEBUG: process_create_pool_state_account: Invalid Token A Vault PDA address. Expected {}, got {}", expected_token_a_vault_pda, token_a_vault_pda_account.key);
        return Err(ProgramError::InvalidArgument);
    }
    msg!("DEBUG: process_create_pool_state_account: Token A Vault PDA address verification passed.");

    msg!("DEBUG: process_create_pool_state_account: Verifying Token B Vault PDA...");
    let token_b_vault_pda_seeds = &[
        TOKEN_B_VAULT_SEED_PREFIX,
        pool_state_pda_account.key.as_ref(),
        &[token_b_vault_bump],
    ];
    let expected_token_b_vault_pda = Pubkey::create_program_address(token_b_vault_pda_seeds, program_id)?;
    msg!("DEBUG: process_create_pool_state_account: Expected Token B Vault PDA (program derived): {}", expected_token_b_vault_pda);
    if *token_b_vault_pda_account.key != expected_token_b_vault_pda {
        msg!("DEBUG: process_create_pool_state_account: Invalid Token B Vault PDA address. Expected {}, got {}", expected_token_b_vault_pda, token_b_vault_pda_account.key);
        return Err(ProgramError::InvalidArgument);
    }
    msg!("DEBUG: process_create_pool_state_account: Token B Vault PDA address verification passed.");
    
    // Create the Pool State PDA account
    let pool_state_account_size = PoolState::get_packed_len();
    let rent_for_pool_state = rent.minimum_balance(pool_state_account_size);
    msg!("DEBUG: process_create_pool_state_account: Creating Pool State PDA account: {}. Size: {}. Rent: {}", pool_state_pda_account.key, pool_state_account_size, rent_for_pool_state);
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            pool_state_pda_account.key,
            rent_for_pool_state,
            pool_state_account_size as u64,
            program_id,
        ),
        &[
            payer.clone(),
            pool_state_pda_account.clone(),
            system_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;
    msg!("DEBUG: process_create_pool_state_account: Pool State PDA account created");

    // Transfer registration fee to pool state PDA
    if payer.lamports() < REGISTRATION_FEE {
        msg!("DEBUG: process_create_pool_state_account: Insufficient SOL for registration fee. Required: {}, Payer has: {}", REGISTRATION_FEE, payer.lamports());
        return Err(ProgramError::InsufficientFunds);
    }
    msg!("DEBUG: process_create_pool_state_account: Payer SOL for registration fee check passed. Payer lamports: {}", payer.lamports());

    msg!("DEBUG: process_create_pool_state_account: Transferring registration fee: {} from {} to {}", REGISTRATION_FEE, payer.key, pool_state_pda_account.key);
    invoke(
        &system_instruction::transfer(payer.key, pool_state_pda_account.key, REGISTRATION_FEE),
        &[
            payer.clone(),
            pool_state_pda_account.clone(),
            system_program_account.clone(),
        ],
    )?;
    msg!("DEBUG: process_create_pool_state_account: Registration fee transferred to pool state PDA.");

    // Create LP Token mints
    let rent_for_mint = rent.minimum_balance(MintAccount::LEN);
    msg!("DEBUG: process_create_pool_state_account: Creating LP Token A Mint account: {}. Rent: {}", lp_token_a_mint_account.key, rent_for_mint);
    invoke(
        &system_instruction::create_account(
            payer.key,
            lp_token_a_mint_account.key,
            rent_for_mint,
            MintAccount::LEN as u64,
            token_program_account.key,
        ),
        &[
            payer.clone(), 
            lp_token_a_mint_account.clone(), 
            system_program_account.clone()
        ],
    )?;
    msg!("DEBUG: process_create_pool_state_account: LP Token A Mint account created. Initializing...");
    invoke(
        &token_instruction::initialize_mint(
            token_program_account.key,
            lp_token_a_mint_account.key,
            payer.key,
            None,
            9,
        )?,
        &[
            lp_token_a_mint_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
    )?;
    msg!("DEBUG: process_create_pool_state_account: LP Token A Mint initialized");

    msg!("DEBUG: process_create_pool_state_account: Creating LP Token B Mint account: {}. Rent: {}", lp_token_b_mint_account.key, rent_for_mint);
    invoke(
        &system_instruction::create_account(
            payer.key,
            lp_token_b_mint_account.key,
            rent_for_mint,
            MintAccount::LEN as u64,
            token_program_account.key,
        ),
        &[
            payer.clone(), 
            lp_token_b_mint_account.clone(), 
            system_program_account.clone()
        ],
    )?;
    msg!("DEBUG: process_create_pool_state_account: LP Token B Mint account created. Initializing...");
    invoke(
        &token_instruction::initialize_mint(
            token_program_account.key,
            lp_token_b_mint_account.key,
            payer.key,
            None,
            9,
        )?,
        &[
            lp_token_b_mint_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
    )?;
    msg!("DEBUG: process_create_pool_state_account: LP Token B Mint initialized");

    // Transfer authority of LP token mints to pool state PDA
    msg!("DEBUG: process_create_pool_state_account: Transferring authority of LP Token A Mint to pool state PDA");
    invoke(
        &token_instruction::set_authority(
            token_program_account.key,
            lp_token_a_mint_account.key,
            Some(pool_state_pda_account.key),
            token_instruction::AuthorityType::MintTokens,
            payer.key,
            &[],
        )?,
        &[
            lp_token_a_mint_account.clone(),
            pool_state_pda_account.clone(),
            payer.clone(),
            token_program_account.clone(),
        ],
    )?;

    msg!("DEBUG: process_create_pool_state_account: Transferring authority of LP Token B Mint to pool state PDA");
    invoke(
        &token_instruction::set_authority(
            token_program_account.key,
            lp_token_b_mint_account.key,
            Some(pool_state_pda_account.key),
            token_instruction::AuthorityType::MintTokens,
            payer.key,
            &[],
        )?,
        &[
            lp_token_b_mint_account.clone(),
            pool_state_pda_account.clone(),
            payer.clone(),
            token_program_account.clone(),
        ],
    )?;

    // Create token vaults
    let vault_account_size = TokenAccount::LEN;
    let rent_for_vault = rent.minimum_balance(vault_account_size);
    msg!("DEBUG: process_create_pool_state_account: Creating Token A Vault PDA account: {}. Size: {}. Rent: {}. Mint: {}", token_a_vault_pda_account.key, vault_account_size, rent_for_vault, token_a_mint_account_info_ref.key);
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            token_a_vault_pda_account.key,
            rent_for_vault,
            vault_account_size as u64,
            token_program_account.key,
        ),
        &[
            payer.clone(),
            token_a_vault_pda_account.clone(),
            system_program_account.clone(),
        ],
        &[token_a_vault_pda_seeds],
    )?;
    msg!("DEBUG: process_create_pool_state_account: Token A Vault PDA account created. Initializing...");
    invoke_signed(
        &token_instruction::initialize_account(
            token_program_account.key,
            token_a_vault_pda_account.key,
            token_a_mint_account_info_ref.key,
            pool_state_pda_account.key,
        )?,
        &[
            token_a_vault_pda_account.clone(),
            token_a_mint_account_info_ref.clone(),
            pool_state_pda_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;
    msg!("DEBUG: process_create_pool_state_account: Token A Vault PDA initialized");

    msg!("DEBUG: process_create_pool_state_account: Creating Token B Vault PDA account: {}. Size: {}. Rent: {}. Mint: {}", token_b_vault_pda_account.key, vault_account_size, rent_for_vault, token_b_mint_account_info_ref.key);
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            token_b_vault_pda_account.key,
            rent_for_vault,
            vault_account_size as u64,
            token_program_account.key,
        ),
        &[
            payer.clone(),
            token_b_vault_pda_account.clone(),
            system_program_account.clone(),
        ],
        &[token_b_vault_pda_seeds],
    )?;
    msg!("DEBUG: process_create_pool_state_account: Token B Vault PDA account created. Initializing...");
    invoke_signed(
        &token_instruction::initialize_account(
            token_program_account.key,
            token_b_vault_pda_account.key,
            token_b_mint_account_info_ref.key,
            pool_state_pda_account.key,
        )?,
        &[
            token_b_vault_pda_account.clone(),
            token_b_mint_account_info_ref.clone(),
            pool_state_pda_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;
    msg!("DEBUG: process_create_pool_state_account: Token B Vault PDA initialized");

    msg!("DEBUG: process_create_pool_state_account: All accounts created successfully");
    Ok(())
}

/// Initializes the data in the already-created Pool State PDA account.
/// This is Step 2 of the two-instruction pool initialization pattern.
///
/// WORKAROUND CONTEXT:
/// This function implements the second part of a workaround for Solana AccountInfo.data
/// issue. It runs in a fresh transaction context where AccountInfo.data properly
/// references the on-chain allocated account buffer created in Step 1.
///
/// BUFFER SERIALIZATION APPROACH:
/// Even with the two-instruction pattern, we use an additional safeguard against
/// potential AccountInfo.data inconsistencies:
/// 1. Serialize PoolState to a temporary Vec<u8> buffer first
/// 2. Verify serialization succeeds and check buffer size
/// 3. Copy the serialized data directly to AccountInfo.data using copy_from_slice
/// 
/// This approach is more robust than direct serialization to AccountInfo.data.borrow_mut()
/// because it ensures we have a valid serialized representation before attempting to
/// write to the account, and the copy operation is atomic.
///
/// WHY THIS IS NEEDED:
/// - Direct serialization with pool_state_data.serialize(&mut *account.data.borrow_mut())
///   was reporting "OK" but the data wasn't persisting to the on-chain account
/// - AccountInfo.data.borrow().len() was returning 0 even after "successful" serialization
/// - This buffer-copy approach ensures data integrity and persistence
///
/// WHAT THIS FUNCTION DOES:
/// - Validates the Pool State PDA account exists with correct size
/// - Checks if pool is already initialized (prevents double-initialization)
/// - Creates and populates PoolState struct with all configuration data
/// - Serializes to buffer, then copies to account data
/// - Verifies the operation succeeded
///
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for pool data initialization
/// * `ratio_primary_per_base` - The ratio of primary tokens per base token
/// * `pool_authority_bump_seed` - Bump seed for pool authority PDA
/// * `primary_token_vault_bump_seed` - Bump seed for primary token vault PDA
/// * `base_token_vault_bump_seed` - Bump seed for base token vault PDA
///
/// # Returns
/// * `ProgramResult` - Success or error code
fn process_initialize_pool_data(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ratio_primary_per_base: u64,
    pool_authority_bump_seed: u8,
    primary_token_vault_bump_seed: u8,
    base_token_vault_bump_seed: u8,
) -> ProgramResult {
    msg!("DEBUG: process_initialize_pool_data: Entered");
    let account_info_iter = &mut accounts.iter();

    let payer = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: Payer: {}", payer.key);
    let pool_state_pda_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: Pool State PDA Account (from client): {}", pool_state_pda_account.key);
    let primary_token_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: Primary Token Mint Account: {}", primary_token_mint_account.key);
    let base_token_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: Base Token Mint Account: {}", base_token_mint_account.key);
    let lp_token_a_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: LP Token A Mint Account: {}", lp_token_a_mint_account.key);
    let lp_token_b_mint_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: LP Token B Mint Account: {}", lp_token_b_mint_account.key);
    let token_a_vault_pda_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: Token A Vault PDA Account (from client): {}", token_a_vault_pda_account.key);
    let token_b_vault_pda_account = next_account_info(account_info_iter)?;
    msg!("DEBUG: process_initialize_pool_data: Token B Vault PDA Account (from client): {}", token_b_vault_pda_account.key);
    let _system_program_account = next_account_info(account_info_iter)?;
    let _token_program_account = next_account_info(account_info_iter)?;
    let rent_sysvar_account = next_account_info(account_info_iter)?;
    
    msg!("DEBUG: process_initialize_pool_data: Parsed all accounts");

    let rent = &Rent::from_account_info(rent_sysvar_account)?;

    // Verify that payer is a signer
    if !payer.is_signer {
        msg!("DEBUG: process_initialize_pool_data: Payer is not a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }
    msg!("DEBUG: process_initialize_pool_data: Payer is signer check passed");

    // Verify ratio is non-zero
    if ratio_primary_per_base == 0 {
        msg!("DEBUG: process_initialize_pool_data: Ratio cannot be zero");
        return Err(ProgramError::InvalidArgument);
    }
    msg!("DEBUG: process_initialize_pool_data: Ratio is non-zero check passed");

    // Normalize tokens and ratio
    msg!("DEBUG: process_initialize_pool_data: Normalizing tokens and ratio...");
    let (token_a_mint_key, token_b_mint_key, ratio_a_numerator, ratio_b_denominator, token_a_is_primary) = 
        if primary_token_mint_account.key < base_token_mint_account.key {
            msg!("DEBUG: process_initialize_pool_data: Primary mint < Base mint");
            (primary_token_mint_account.key, base_token_mint_account.key, ratio_primary_per_base, 1, true)
        } else {
            msg!("DEBUG: process_initialize_pool_data: Primary mint > Base mint");
            (base_token_mint_account.key, primary_token_mint_account.key, 1, ratio_primary_per_base, false)
        };

    msg!("DEBUG: process_initialize_pool_data: Normalized: token_a_mint_key={}, token_b_mint_key={}, ratio_a_num={}, ratio_b_den={}", 
         token_a_mint_key, token_b_mint_key, ratio_a_numerator, ratio_b_denominator);

    // Verify the pool state PDA is derived correctly using normalized values
    msg!("DEBUG: process_initialize_pool_data: Verifying Pool State PDA. Pool Auth Bump Seed from instr: {}", pool_authority_bump_seed);
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        token_a_mint_key.as_ref(),
        token_b_mint_key.as_ref(),
        &ratio_a_numerator.to_le_bytes(),
        &ratio_b_denominator.to_le_bytes(),
        &[pool_authority_bump_seed],
    ];
    let expected_pool_state_pda = Pubkey::create_program_address(pool_state_pda_seeds, program_id)?;
    msg!("DEBUG: process_initialize_pool_data: Expected Pool State PDA (program derived): {}", expected_pool_state_pda);
    if *pool_state_pda_account.key != expected_pool_state_pda {
        msg!("DEBUG: process_initialize_pool_data: Invalid Pool State PDA address. Expected {}, got {}", expected_pool_state_pda, pool_state_pda_account.key);
        return Err(ProgramError::InvalidArgument);
    }
    msg!("DEBUG: process_initialize_pool_data: Pool State PDA address verification passed.");

    // Check if pool state account exists and has the correct size
    msg!("DEBUG: process_initialize_pool_data: Checking pool state account. Data len: {}", pool_state_pda_account.data_len());
    if pool_state_pda_account.data_len() != PoolState::get_packed_len() {
        msg!("DEBUG: process_initialize_pool_data: Pool state account has incorrect size. Expected: {}, Got: {}", 
             PoolState::get_packed_len(), pool_state_pda_account.data_len());
        return Err(ProgramError::InvalidAccountData);
    }

    // Check if pool state is already initialized
    if !pool_state_pda_account.data_is_empty() {
        match PoolState::try_from_slice(&pool_state_pda_account.data.borrow()) {
            Ok(pool_state_data) => {
                if pool_state_data.is_initialized {
                    msg!("DEBUG: process_initialize_pool_data: Pool state already initialized");
                    return Err(ProgramError::AccountAlreadyInitialized);
                }
                msg!("DEBUG: process_initialize_pool_data: Pool state data found but not initialized, proceeding.");
            }
            Err(_) => {
                // If we can't deserialize, check if it's all zeros (uninitialized)
                let is_zeroed = pool_state_pda_account.data.borrow().iter().all(|&x| x == 0);
                if !is_zeroed {
                    msg!("DEBUG: process_initialize_pool_data: Pool state account has data but is not a valid PoolState struct and not zeroed.");
                    return Err(ProgramError::InvalidAccountData);
                }
                msg!("DEBUG: process_initialize_pool_data: Pool state account data is zeroed, proceeding.");
            }
        }
    }

    // Map vault bump seeds
    msg!("DEBUG: process_initialize_pool_data: Mapping vault bump seeds. Primary Vault Bump: {}, Base Vault Bump: {}", primary_token_vault_bump_seed, base_token_vault_bump_seed);
    let (token_a_vault_bump, token_b_vault_bump) = if token_a_is_primary {
        (primary_token_vault_bump_seed, base_token_vault_bump_seed)
    } else {
        (base_token_vault_bump_seed, primary_token_vault_bump_seed)
    };
    msg!("DEBUG: process_initialize_pool_data: Normalized token_a_vault_bump: {}, token_b_vault_bump: {}", token_a_vault_bump, token_b_vault_bump);

    // Initialize Pool State data struct
    msg!("DEBUG: process_initialize_pool_data: Initializing Pool State data struct");
    let mut pool_state_data = PoolState::default();
    
    pool_state_data.owner = *payer.key;
    pool_state_data.token_a_mint = *token_a_mint_key;
    pool_state_data.token_b_mint = *token_b_mint_key;
    pool_state_data.token_a_vault = *token_a_vault_pda_account.key;
    pool_state_data.token_b_vault = *token_b_vault_pda_account.key;
    pool_state_data.lp_token_a_mint = *lp_token_a_mint_account.key;
    pool_state_data.lp_token_b_mint = *lp_token_b_mint_account.key;
    pool_state_data.ratio_a_numerator = ratio_a_numerator;
    pool_state_data.ratio_b_denominator = ratio_b_denominator;
    pool_state_data.total_token_a_liquidity = 0;
    pool_state_data.total_token_b_liquidity = 0;
    pool_state_data.pool_authority_bump_seed = pool_authority_bump_seed;
    pool_state_data.token_a_vault_bump_seed = token_a_vault_bump;
    pool_state_data.token_b_vault_bump_seed = token_b_vault_bump;
    pool_state_data.is_initialized = true;

    // Initialize security parameters
    pool_state_data.is_paused = false;

    // Initialize rent requirements
    let rent_requirements = RentRequirements::new(rent);
    pool_state_data.rent_requirements = rent_requirements;

    // Initialize delegate management system (owner is first delegate)
    let current_slot = 0; // Will be updated when clock is available
    pool_state_data.delegate_management = DelegateManagement::new(*payer.key, current_slot);
    
    // Initialize fee tracking
    pool_state_data.collected_fees_token_a = 0;
    pool_state_data.collected_fees_token_b = 0;
    pool_state_data.total_fees_withdrawn_token_a = 0;
    pool_state_data.total_fees_withdrawn_token_b = 0;
    
    // Initialize swap fee to 0% as per requirements
    pool_state_data.swap_fee_basis_points = 0;
    
    // BUFFER SERIALIZATION WORKAROUND:
    // Instead of directly serializing to AccountInfo.data.borrow_mut(), we use a two-step process:
    // 1. Serialize to a temporary buffer to ensure the operation succeeds
    // 2. Copy the buffer contents to the account data
    // This approach prevents issues where serialization reports "OK" but data doesn't persist.
    
    // Step 1: Serialize the pool state data to a temporary buffer
    let mut serialized_data = Vec::new();
    match pool_state_data.serialize(&mut serialized_data) {
        Ok(_) => {
            msg!("DEBUG: process_initialize_pool_data: Serialization to buffer successful. Buffer len: {}", serialized_data.len());
        }
        Err(e) => {
            msg!("DEBUG: process_initialize_pool_data: Serialization to buffer FAILED: {:?}", e);
            return Err(e.into());
        }
    }
    
    // Step 2: Copy the serialized data to the account data
    msg!("DEBUG: process_initialize_pool_data: Copying {} bytes to account data", serialized_data.len());
    let account_data_len = pool_state_pda_account.data_len();
    if serialized_data.len() > account_data_len {
        msg!("DEBUG: process_initialize_pool_data: Serialized data too large for account. Need: {}, Have: {}", 
             serialized_data.len(), account_data_len);
        return Err(ProgramError::AccountDataTooSmall);
    }
    
    // Perform the atomic copy operation
    // This ensures that either all data is written correctly or the operation fails cleanly
    {
        let mut account_data = pool_state_pda_account.data.borrow_mut();
        account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
        msg!("DEBUG: process_initialize_pool_data: Data copied to account successfully");
    }
    
    msg!("DEBUG: process_initialize_pool_data: Pool State PDA data len after copy: {}", pool_state_pda_account.data.borrow().len());
    msg!("DEBUG: process_initialize_pool_data: Pool State PDA initialized with data: {:?}", pool_state_data);
    msg!("DEBUG: process_initialize_pool_data: Exiting successfully");

    Ok(())
}

/// Handles user deposits into the trading pool.
///
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for deposit
/// * `deposit_token_mint_key` - The mint of the token being deposited
/// * `amount` - The amount to deposit
///
/// # Returns
/// * `ProgramResult` - Success or error code
fn process_deposit(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    deposit_token_mint_key: Pubkey,
    amount: u64,
) -> ProgramResult {
    msg!("Processing Deposit v2");
    let account_info_iter = &mut accounts.iter();

    let user_signer = next_account_info(account_info_iter)?;
    let user_source_token_account = next_account_info(account_info_iter)?;
    let pool_state_account = next_account_info(account_info_iter)?;
    let token_a_mint_for_pda_seeds = next_account_info(account_info_iter)?;
    let token_b_mint_for_pda_seeds = next_account_info(account_info_iter)?;
    
    let pool_token_a_vault_account = next_account_info(account_info_iter)?;
    let pool_token_b_vault_account = next_account_info(account_info_iter)?;
    let lp_token_a_mint_account = next_account_info(account_info_iter)?;
    let lp_token_b_mint_account = next_account_info(account_info_iter)?;
    let user_destination_lp_token_account = next_account_info(account_info_iter)?;
    
    let system_program_account = next_account_info(account_info_iter)?;
    let token_program_account = next_account_info(account_info_iter)?;
    let rent_sysvar_account = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_sysvar_account)?;
    let _clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;

    // Check rent-exempt status for pool accounts
    check_rent_exempt(pool_state_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(pool_token_a_vault_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(pool_token_b_vault_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(lp_token_a_mint_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(lp_token_b_mint_account, program_id, rent, _clock.slot)?;

    if !user_signer.is_signer {
        msg!("User must be a signer for deposit");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut pool_state_data = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    if !pool_state_data.is_initialized {
        msg!("Pool not initialized");
        return Err(ProgramError::UninitializedAccount);
    }

    // Verify that the provided token_a_mint_for_pda_seeds and token_b_mint_for_pda_seeds match pool state
    if *token_a_mint_for_pda_seeds.key != pool_state_data.token_a_mint {
        msg!("Provided token_a_mint_for_pda_seeds does not match pool state");
        return Err(ProgramError::InvalidAccountData);
    }
    if *token_b_mint_for_pda_seeds.key != pool_state_data.token_b_mint {
        msg!("Provided token_b_mint_for_pda_seeds does not match pool state");
        return Err(ProgramError::InvalidAccountData);
    }

    // Determine which token (A or B) is being deposited and set target accounts
    let (target_pool_vault_account, target_lp_mint_account, is_depositing_token_a) = 
        if deposit_token_mint_key == pool_state_data.token_a_mint {
            // Depositing Token A
            if *pool_token_a_vault_account.key != pool_state_data.token_a_vault {
                msg!("Invalid pool_token_a_vault_account provided for token A deposit.");
                return Err(ProgramError::InvalidAccountData);
            }
            if *lp_token_a_mint_account.key != pool_state_data.lp_token_a_mint {
                msg!("Invalid lp_token_a_mint_account provided for token A deposit.");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_a_vault_account, lp_token_a_mint_account, true)
        } else if deposit_token_mint_key == pool_state_data.token_b_mint {
            // Depositing Token B
            if *pool_token_b_vault_account.key != pool_state_data.token_b_vault {
                msg!("Invalid pool_token_b_vault_account provided for token B deposit.");
                return Err(ProgramError::InvalidAccountData);
            }
            if *lp_token_b_mint_account.key != pool_state_data.lp_token_b_mint {
                msg!("Invalid lp_token_b_mint_account provided for token B deposit.");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_b_vault_account, lp_token_b_mint_account, false)
        } else {
            msg!("Deposit token mint does not match either of the pool's tokens");
            return Err(ProgramError::InvalidArgument);
        };

    // Validate user's source token account
    let user_source_token_account_data = TokenAccount::unpack_from_slice(&user_source_token_account.data.borrow())?;
    if user_source_token_account_data.mint != deposit_token_mint_key {
        msg!("User source token account mint mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_source_token_account_data.owner != *user_signer.key {
        msg!("User source token account owner mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_source_token_account_data.amount < amount {
        msg!("Insufficient funds in user source token account");
        return Err(ProgramError::InsufficientFunds);
    }

    // Validate user's destination LP token account
    let user_dest_lp_token_account_data = TokenAccount::unpack_from_slice(&user_destination_lp_token_account.data.borrow())?;
    if user_dest_lp_token_account_data.mint != *target_lp_mint_account.key {
        msg!("User destination LP token account mint mismatch with target LP mint");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_dest_lp_token_account_data.owner != *user_signer.key {
        msg!("User destination LP token account owner mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Validate SPL Token Program ID
    if *token_program_account.key != Pubkey::new_from_array(spl_token::id().to_bytes()) {
        msg!("Invalid SPL Token Program ID");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Transfer tokens from user to pool vault
    msg!("Transferring {} of token {} from user to pool", amount, deposit_token_mint_key);
    invoke(
        &token_instruction::transfer(
            token_program_account.key,
            user_source_token_account.key,
            target_pool_vault_account.key,
            user_signer.key,
            &[],
            amount,
        )?,
        &[
            user_source_token_account.clone(),
            target_pool_vault_account.clone(),
            user_signer.clone(),
            token_program_account.clone(),
        ],
    )?;

    // Mint LP tokens to user
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.token_a_mint.as_ref(),
        pool_state_data.token_b_mint.as_ref(),
        &pool_state_data.ratio_a_numerator.to_le_bytes(),
        &pool_state_data.ratio_b_denominator.to_le_bytes(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    msg!("Minting {} LP tokens for {} to user", amount, target_lp_mint_account.key);
    invoke_signed(
        &token_instruction::mint_to(
            token_program_account.key,
            target_lp_mint_account.key,
            user_destination_lp_token_account.key,
            pool_state_account.key,
            &[], 
            amount,
        )?,
        &[
            target_lp_mint_account.clone(),
            user_destination_lp_token_account.clone(),
            pool_state_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;

    // Update pool state liquidity
    if is_depositing_token_a {
        pool_state_data.total_token_a_liquidity = pool_state_data.total_token_a_liquidity.checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    } else {
        pool_state_data.total_token_b_liquidity = pool_state_data.total_token_b_liquidity.checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }
    pool_state_data.serialize(&mut *pool_state_account.data.borrow_mut())?;
    msg!("Pool liquidity updated. Token A: {}, Token B: {}", pool_state_data.total_token_a_liquidity, pool_state_data.total_token_b_liquidity);

    // Transfer deposit fee to pool state PDA
    if user_signer.lamports() < DEPOSIT_WITHDRAWAL_FEE {
        msg!("Insufficient SOL for deposit fee after token transfer. User lamports: {}", user_signer.lamports());
        return Err(ProgramError::InsufficientFunds); 
    }
    invoke(
        &system_instruction::transfer(user_signer.key, pool_state_account.key, DEPOSIT_WITHDRAWAL_FEE),
        &[user_signer.clone(), pool_state_account.clone(), system_program_account.clone()],
    )?;
    msg!("Deposit fee {} transferred to pool state PDA", DEPOSIT_WITHDRAWAL_FEE);

    Ok(())
}

/// Handles user withdrawals from the trading pool.
///
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for withdrawal
/// * `withdraw_token_mint_key` - The mint of the token being withdrawn
/// * `lp_amount_to_burn` - The amount of LP tokens to burn
///
/// # Returns
/// * `ProgramResult` - Success or error code
fn process_withdraw(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    withdraw_token_mint_key: Pubkey,
    lp_amount_to_burn: u64,
) -> ProgramResult {
    msg!("Processing Withdraw v2");
    let account_info_iter = &mut accounts.iter();

    let user_signer = next_account_info(account_info_iter)?;                     // User making the withdrawal (signer)
    let user_source_lp_token_account = next_account_info(account_info_iter)?;   // User's LP token account (source of burn)
    let user_destination_token_account = next_account_info(account_info_iter)?; // User's account for receiving underlying tokens
    let pool_state_account = next_account_info(account_info_iter)?;              // Pool state PDA
    
    // Accounts needed for Pool State PDA seeds derivation for signing
    let token_a_mint_for_pda_seeds = next_account_info(account_info_iter)?;    // Pool's token_a_mint (must match pool_state_data.token_a_mint)
    let token_b_mint_for_pda_seeds = next_account_info(account_info_iter)?;    // Pool's token_b_mint (must match pool_state_data.token_b_mint)
    
    let pool_token_a_vault_account = next_account_info(account_info_iter)?;     // Pool's vault for token A
    let pool_token_b_vault_account = next_account_info(account_info_iter)?;     // Pool's vault for token B
    let lp_token_a_mint_account = next_account_info(account_info_iter)?;         // Pool's LP token A mint
    let lp_token_b_mint_account = next_account_info(account_info_iter)?;         // Pool's LP token B mint
    
    let system_program_account = next_account_info(account_info_iter)?;         // System program
    let token_program_account = next_account_info(account_info_iter)?;           // SPL Token program
    let rent_sysvar_account = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_sysvar_account)?;
    let _clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;

    // Check rent-exempt status for pool accounts
    check_rent_exempt(pool_state_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(pool_token_a_vault_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(pool_token_b_vault_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(lp_token_a_mint_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(lp_token_b_mint_account, program_id, rent, _clock.slot)?;

    if !user_signer.is_signer {
        msg!("User must be a signer for withdraw");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut pool_state_data = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    if !pool_state_data.is_initialized {
        msg!("Pool not initialized");
        return Err(ProgramError::UninitializedAccount);
    }

    // Verify that the provided token_a_mint_for_pda_seeds and token_b_mint_for_pda_seeds match pool state
    if *token_a_mint_for_pda_seeds.key != pool_state_data.token_a_mint {
        msg!("Provided token_a_mint_for_pda_seeds does not match pool state");
        return Err(ProgramError::InvalidAccountData);
    }
    if *token_b_mint_for_pda_seeds.key != pool_state_data.token_b_mint {
        msg!("Provided token_b_mint_for_pda_seeds does not match pool state");
        return Err(ProgramError::InvalidAccountData);
    }

    // Determine which token (A or B) is being withdrawn and set relevant accounts
    let (source_pool_vault_acc, source_lp_mint_account, is_withdrawing_token_a) = 
        if withdraw_token_mint_key == pool_state_data.token_a_mint {
            // Withdrawing Token A, so burning LP Token A
            if *pool_token_a_vault_account.key != pool_state_data.token_a_vault {
                msg!("Invalid pool_token_a_vault_account provided for token A withdrawal.");
                return Err(ProgramError::InvalidAccountData);
            }
            if *lp_token_a_mint_account.key != pool_state_data.lp_token_a_mint {
                msg!("Invalid lp_token_a_mint_account provided for token A withdrawal.");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_a_vault_account, lp_token_a_mint_account, true)
        } else if withdraw_token_mint_key == pool_state_data.token_b_mint {
            // Withdrawing Token B, so burning LP Token B
            if *pool_token_b_vault_account.key != pool_state_data.token_b_vault {
                msg!("Invalid pool_token_b_vault_account provided for token B withdrawal.");
                return Err(ProgramError::InvalidAccountData);
            }
            if *lp_token_b_mint_account.key != pool_state_data.lp_token_b_mint {
                msg!("Invalid lp_token_b_mint_account provided for token B withdrawal.");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_b_vault_account, lp_token_b_mint_account, false)
        } else {
            msg!("Withdraw token mint does not match either of the pool's tokens");
            return Err(ProgramError::InvalidArgument);
        };

    // Validate user's source LP token account
    let user_source_lp_token_account_data = TokenAccount::unpack_from_slice(&user_source_lp_token_account.data.borrow())?;
    if user_source_lp_token_account_data.mint != *source_lp_mint_account.key {
        msg!("User source LP token account mint mismatch with identified LP mint for withdrawal.");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_source_lp_token_account_data.owner != *user_signer.key {
        msg!("User source LP token account owner mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_source_lp_token_account_data.amount < lp_amount_to_burn {
        msg!("Insufficient LP tokens in user source account");
        return Err(ProgramError::InsufficientFunds);
    }

    // Validate user's destination token account (for underlying tokens)
    let user_dest_token_account_data = TokenAccount::unpack_from_slice(&user_destination_token_account.data.borrow())?;
    if user_dest_token_account_data.mint != withdraw_token_mint_key {
        msg!("User destination token account mint mismatch with withdraw_token_mint_key");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_dest_token_account_data.owner != *user_signer.key {
        msg!("User destination token account owner mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate SPL Token Program ID
    if *token_program_account.key != Pubkey::new_from_array(spl_token::id().to_bytes()) {
        msg!("Invalid SPL Token Program ID");
        return Err(ProgramError::IncorrectProgramId);
    }
    
    // Check if pool has enough liquidity for the withdrawal
    if is_withdrawing_token_a {
        if pool_state_data.total_token_a_liquidity < lp_amount_to_burn {
            msg!("Insufficient token A liquidity in the pool for withdrawal.");
            return Err(ProgramError::InsufficientFunds);
        }
    } else {
        // Output is Token A
        if pool_state_data.total_token_b_liquidity < lp_amount_to_burn {
            msg!("Insufficient Token A liquidity in the pool for swap output.");
            return Err(ProgramError::InsufficientFunds);
        }
    }

    // Burn LP tokens from user
    msg!("Burning {} LP tokens from account {}", lp_amount_to_burn, user_source_lp_token_account.key);
    invoke(
        &token_instruction::burn(
            token_program_account.key,
            user_source_lp_token_account.key, // Account to burn from
            source_lp_mint_account.key,       // Mint of the LP tokens being burned
            user_signer.key,                  // Authority (owner of the LP token account)
            &[],
            lp_amount_to_burn,
        )?,
        &[
            user_source_lp_token_account.clone(),
            source_lp_mint_account.clone(),
            user_signer.clone(),
            token_program_account.clone(),
        ],
    )?;

    // Transfer underlying tokens from pool vault to user
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.token_a_mint.as_ref(),
        pool_state_data.token_b_mint.as_ref(),
        &pool_state_data.ratio_a_numerator.to_le_bytes(),
        &pool_state_data.ratio_b_denominator.to_le_bytes(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    msg!("Transferring {} of token {} from pool vault {} to user account {}", 
           lp_amount_to_burn, withdraw_token_mint_key, source_pool_vault_acc.key, user_destination_token_account.key);
    invoke_signed(
        &token_instruction::transfer(
            token_program_account.key,
            source_pool_vault_acc.key,          // Pool's vault (source)
            user_destination_token_account.key,      // User's output account (destination)
            pool_state_account.key,             // Pool PDA is the authority over its vault
            &[],
            lp_amount_to_burn,                        // Amount of underlying token to transfer (equals LP burned)
        )?,
        &[
            source_pool_vault_acc.clone(),
            user_destination_token_account.clone(),
            pool_state_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;

    // Update pool state liquidity
    if is_withdrawing_token_a {
        pool_state_data.total_token_a_liquidity = pool_state_data.total_token_a_liquidity.checked_sub(lp_amount_to_burn)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    } else {
        pool_state_data.total_token_b_liquidity = pool_state_data.total_token_b_liquidity.checked_sub(lp_amount_to_burn)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }
    pool_state_data.serialize(&mut *pool_state_account.data.borrow_mut())?;
    msg!("Pool liquidity updated. Token A: {}, Token B: {}", pool_state_data.total_token_a_liquidity, pool_state_data.total_token_b_liquidity);

    // Transfer withdrawal fee to pool state PDA
    if user_signer.lamports() < DEPOSIT_WITHDRAWAL_FEE {
        msg!("Insufficient SOL for withdrawal fee. User lamports: {}", user_signer.lamports());
        return Err(ProgramError::InsufficientFunds);
    }
    invoke(
        &system_instruction::transfer(user_signer.key, pool_state_account.key, DEPOSIT_WITHDRAWAL_FEE),
        &[user_signer.clone(), pool_state_account.clone(), system_program_account.clone()],
    )?;
    msg!("Withdrawal fee {} transferred to pool state PDA", DEPOSIT_WITHDRAWAL_FEE);

    Ok(())
}

/// Handles token swaps within the trading pool.
///
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for swap
/// * `input_token_mint_key` - The mint of the input token
/// * `amount_in` - The amount of input token to swap
///
/// # Returns
/// * `ProgramResult` - Success or error code
fn process_swap(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input_token_mint_key: Pubkey,
    amount_in: u64,
    minimum_amount_out: u64,
) -> ProgramResult {
    msg!("Processing Swap v2");
    let account_info_iter = &mut accounts.iter();

    let user_signer = next_account_info(account_info_iter)?;                     // User initiating the swap (signer)
    let user_input_token_account = next_account_info(account_info_iter)?;      // User's token account for the input token
    let user_output_token_account = next_account_info(account_info_iter)?;     // User's token account to receive the output token
    let pool_state_account = next_account_info(account_info_iter)?;              // Pool state PDA

    // Accounts needed for Pool State PDA seeds derivation for signing
    let token_a_mint_for_pda_seeds = next_account_info(account_info_iter)?;    // Pool's token_a_mint (must match pool_state_data.token_a_mint)
    let token_b_mint_for_pda_seeds = next_account_info(account_info_iter)?;    // Pool's token_b_mint (must match pool_state_data.token_b_mint)
    
    let pool_token_a_vault_account = next_account_info(account_info_iter)?;     // Pool's vault for token A
    let pool_token_b_vault_account = next_account_info(account_info_iter)?;     // Pool's vault for token B
    
    let system_program_account = next_account_info(account_info_iter)?;         // System program
    let token_program_account = next_account_info(account_info_iter)?;           // SPL Token program
    let rent_sysvar_account = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_sysvar_account)?;
    let _clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;

    // Check rent-exempt status for pool accounts
    check_rent_exempt(pool_state_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(pool_token_a_vault_account, program_id, rent, _clock.slot)?;
    check_rent_exempt(pool_token_b_vault_account, program_id, rent, _clock.slot)?;

    if !user_signer.is_signer {
        msg!("User must be a signer for swap");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut pool_state_data = PoolState::try_from_slice(&pool_state_account.data.borrow())?;
    if !pool_state_data.is_initialized {
        msg!("Pool not initialized");
        return Err(ProgramError::UninitializedAccount);
    }

    // Verify that the provided token_a_mint_for_pda_seeds and token_b_mint_for_pda_seeds match pool state
    if *token_a_mint_for_pda_seeds.key != pool_state_data.token_a_mint {
        msg!("Provided token_a_mint_for_pda_seeds does not match pool state");
        return Err(ProgramError::InvalidAccountData);
    }
    if *token_b_mint_for_pda_seeds.key != pool_state_data.token_b_mint {
        msg!("Provided token_b_mint_for_pda_seeds does not match pool state");
        return Err(ProgramError::InvalidAccountData);
    }

    // Determine swap direction and relevant accounts
    let (input_pool_vault_acc, output_pool_vault_acc, output_token_mint_key, input_is_token_a) = 
        if input_token_mint_key == pool_state_data.token_a_mint {
            // Swapping A for B
            if *pool_token_a_vault_account.key != pool_state_data.token_a_vault || 
               *pool_token_b_vault_account.key != pool_state_data.token_b_vault {
                msg!("Invalid pool vault accounts provided for A -> B swap.");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_a_vault_account, pool_token_b_vault_account, pool_state_data.token_b_mint, true)
        } else if input_token_mint_key == pool_state_data.token_b_mint {
            // Swapping B for A
            if *pool_token_b_vault_account.key != pool_state_data.token_b_vault || 
               *pool_token_a_vault_account.key != pool_state_data.token_a_vault {
                msg!("Invalid pool vault accounts provided for B -> A swap.");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_b_vault_account, pool_token_a_vault_account, pool_state_data.token_a_mint, false)
        } else {
            msg!("Input token mint does not match either of the pool's tokens");
            return Err(ProgramError::InvalidArgument);
        };

    // Validate user's input token account
    let user_input_token_account_data = TokenAccount::unpack_from_slice(&user_input_token_account.data.borrow())?;
    if user_input_token_account_data.mint != input_token_mint_key {
        msg!("User input token account mint mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_input_token_account_data.owner != *user_signer.key {
        msg!("User input token account owner mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_input_token_account_data.amount < amount_in {
        msg!("Insufficient funds in user input token account");
        return Err(ProgramError::InsufficientFunds);
    }

    // Validate user's output token account
    let user_output_token_account_data = TokenAccount::unpack_from_slice(&user_output_token_account.data.borrow())?;
    if user_output_token_account_data.mint != output_token_mint_key {
        msg!("User output token account mint mismatch with expected output token");
        return Err(ProgramError::InvalidAccountData);
    }
    if user_output_token_account_data.owner != *user_signer.key {
        msg!("User output token account owner mismatch");
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Validate SPL Token Program ID
    if *token_program_account.key != Pubkey::new_from_array(spl_token::id().to_bytes()) {
        msg!("Invalid SPL Token Program ID");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Calculate amount_out
    let amount_out = if input_is_token_a {
        // Swapping A for B: amount_out_B = (amount_in_A * ratio_B_denominator) / ratio_A_numerator
        if pool_state_data.ratio_a_numerator == 0 {
            msg!("Pool ratio_a_numerator is zero, cannot perform swap.");
            return Err(ProgramError::InvalidAccountData); // Or a more specific error
        }
        amount_in.checked_mul(pool_state_data.ratio_b_denominator)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .checked_div(pool_state_data.ratio_a_numerator)
            .ok_or(ProgramError::ArithmeticOverflow)? // Using ArithmeticOverflow for division issues
    } else {
        // Swapping B for A: amount_out_A = (amount_in_B * ratio_A_numerator) / ratio_B_denominator
        if pool_state_data.ratio_b_denominator == 0 {
            msg!("Pool ratio_b_denominator is zero, cannot perform swap.");
            return Err(ProgramError::InvalidAccountData);
        }
        amount_in.checked_mul(pool_state_data.ratio_a_numerator)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .checked_div(pool_state_data.ratio_b_denominator)
            .ok_or(ProgramError::ArithmeticOverflow)?
    };

    if amount_out == 0 {
        return Err(PoolError::InvalidSwapAmount {
            amount: amount_out,
            min_amount: 1,
            max_amount: u64::MAX,
        }.into());
    }

    // Check slippage protection
    if amount_out < minimum_amount_out {
        msg!("Slippage tolerance exceeded. Expected minimum: {}, Got: {}", minimum_amount_out, amount_out);
        return Err(PoolError::InvalidSwapAmount {
            amount: amount_out,
            min_amount: minimum_amount_out,
            max_amount: u64::MAX,
        }.into());
    }

    // Calculate and collect trading fees using configurable rate
    let fee_amount = if pool_state_data.swap_fee_basis_points == 0 {
        0u64 // No fee if set to 0%
    } else {
        amount_in
            .checked_mul(pool_state_data.swap_fee_basis_points)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .checked_div(FEE_BASIS_POINTS_DENOMINATOR)
            .ok_or(ProgramError::ArithmeticOverflow)?
    };
    
    let amount_after_fee = amount_in
        .checked_sub(fee_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    msg!("Swap calculation: Input: {}, Fee: {} ({:.2}% rate), After fee: {}, Output: {}", 
         amount_in, fee_amount, pool_state_data.swap_fee_basis_points as f64 / 100.0, amount_after_fee, amount_out);

    // Check pool liquidity for output token
    if input_is_token_a {
        // Output is Token B
        if pool_state_data.total_token_b_liquidity < amount_out {
            msg!("Insufficient Token B liquidity in the pool for swap output.");
            return Err(ProgramError::InsufficientFunds);
        }
    } else {
        // Output is Token A
        if pool_state_data.total_token_a_liquidity < amount_out {
            msg!("Insufficient Token A liquidity in the pool for swap output.");
            return Err(ProgramError::InsufficientFunds);
        }
    }

    // Transfer input tokens from user to pool vault (including fee)
    msg!("Transferring {} of input token {} from user to pool vault {}", 
           amount_in, input_token_mint_key, input_pool_vault_acc.key);
    invoke(
        &token_instruction::transfer(
            token_program_account.key,
            user_input_token_account.key,
            input_pool_vault_acc.key,
            user_signer.key, // User is the authority over their input account
            &[],
            amount_in,
        )?,
        &[
            user_input_token_account.clone(),
            input_pool_vault_acc.clone(),
            user_signer.clone(),
            token_program_account.clone(),
        ],
    )?;

    // Transfer output tokens from pool vault to user
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.token_a_mint.as_ref(),
        pool_state_data.token_b_mint.as_ref(),
        &pool_state_data.ratio_a_numerator.to_le_bytes(),
        &pool_state_data.ratio_b_denominator.to_le_bytes(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    msg!("Transferring {} of output token {} from pool vault {} to user account {}", 
           amount_out, output_token_mint_key, output_pool_vault_acc.key, user_output_token_account.key);
    invoke_signed(
        &token_instruction::transfer(
            token_program_account.key,
            output_pool_vault_acc.key,          // Pool's output vault (source)
            user_output_token_account.key,      // User's output account (destination)
            pool_state_account.key,             // Pool PDA is the authority over its vault
            &[],
            amount_out,
        )?,
        &[
            output_pool_vault_acc.clone(),
            user_output_token_account.clone(),
            pool_state_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;

    // Update pool state liquidity and fee tracking
    if input_is_token_a {
        // Add input tokens (minus fee) to liquidity, track fee separately
        pool_state_data.total_token_a_liquidity = pool_state_data.total_token_a_liquidity.checked_add(amount_after_fee)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        pool_state_data.total_token_b_liquidity = pool_state_data.total_token_b_liquidity.checked_sub(amount_out)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        // Track collected fee
        pool_state_data.collected_fees_token_a = pool_state_data.collected_fees_token_a.checked_add(fee_amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    } else {
        // Add input tokens (minus fee) to liquidity, track fee separately
        pool_state_data.total_token_b_liquidity = pool_state_data.total_token_b_liquidity.checked_add(amount_after_fee)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        pool_state_data.total_token_a_liquidity = pool_state_data.total_token_a_liquidity.checked_sub(amount_out)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        // Track collected fee
        pool_state_data.collected_fees_token_b = pool_state_data.collected_fees_token_b.checked_add(fee_amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }
    pool_state_data.serialize(&mut *pool_state_account.data.borrow_mut())?;
    msg!("Pool liquidity updated after swap. Token A: {}, Token B: {}", 
           pool_state_data.total_token_a_liquidity, pool_state_data.total_token_b_liquidity);
    msg!("Fees collected - Token A: {}, Token B: {}", 
           pool_state_data.collected_fees_token_a, pool_state_data.collected_fees_token_b);

    // Transfer swap fee to pool state PDA
    if user_signer.lamports() < SWAP_FEE {
        msg!("Insufficient SOL for swap fee. User lamports: {}", user_signer.lamports());
        return Err(ProgramError::InsufficientFunds);
    }
    invoke(
        &system_instruction::transfer(user_signer.key, pool_state_account.key, SWAP_FEE),
        &[user_signer.clone(), pool_state_account.clone(), system_program_account.clone()],
    )?;
    msg!("Swap fee {} transferred to pool state PDA", SWAP_FEE);

    Ok(())
}

/// Allows the pool owner to withdraw accumulated fees.
///
/// # Arguments
/// * `_program_id` - The program ID of the contract
/// * `accounts` - The accounts required for fee withdrawal
///
/// # Returns
/// * `ProgramResult` - Success or error code
fn process_withdraw_fees(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing WithdrawFees");
    let account_info_iter = &mut accounts.iter();

    let owner = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify pool state
    let pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if *owner.key != pool_state_data.owner {
        msg!("Only pool owner can withdraw fees");
        return Err(ProgramError::InvalidAccountData);
    }

    // Calculate withdrawable amount
    let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;
    let minimum_rent = rent.minimum_balance(pool_state.data_len());
    let withdrawable_amount = pool_state.lamports().checked_sub(minimum_rent)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    if withdrawable_amount == 0 {
        msg!("No fees available to withdraw");
        return Ok(());
    }

    // Get PDA seeds for signing
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.token_a_mint.as_ref(),
        pool_state_data.token_b_mint.as_ref(),
        &pool_state_data.ratio_a_numerator.to_le_bytes(),
        &pool_state_data.ratio_b_denominator.to_le_bytes(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    // Transfer fees using invoke_signed
    invoke_signed(
        &system_instruction::transfer(pool_state.key, owner.key, withdrawable_amount),
        &[pool_state.clone(), owner.clone(), system_program.clone()],
        &[pool_state_pda_seeds],
    )?;
    msg!("Fees transferred to owner: {} lamports ({} lamports reserved for rent)", 
         withdrawable_amount, minimum_rent);

    Ok(())
}

/// Ensures an account has enough lamports to be rent exempt.
///
/// # Arguments
/// * `pool_state` - The pool state account
/// * `rent` - The rent sysvar
/// * `current_slot` - The current slot
///
/// # Returns
/// * `ProgramResult` - Success or error code
fn ensure_rent_exempt(
    pool_state: &AccountInfo,
    rent: &Rent,
    current_slot: u64,
) -> ProgramResult {
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    
    // Update rent requirements if needed
    if pool_state_data.rent_requirements.update_if_needed(rent, current_slot) {
        pool_state_data.serialize(&mut *pool_state.data.borrow_mut())?;
    }

    // Calculate total required rent
    let total_required_rent = pool_state_data.rent_requirements.get_total_required_rent();
    
    // Check if we have enough balance
    if pool_state.lamports() < total_required_rent {
        return Err(PoolError::RentExemptError {
            account: *pool_state.key,
            required: total_required_rent,
            available: pool_state.lamports(),
        }.into());
    }

    Ok(())
}

/// Updates the pool's security parameters.
///
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for the update
/// * `max_withdrawal_percentage` - Optional new maximum withdrawal percentage (e.g., 1000 for 10%)
/// * `withdrawal_cooldown` - Optional new withdrawal cooldown in slots
/// * `is_paused` - Optional new pause state
///
/// # Returns
/// * `ProgramResult` - Success or error code
fn process_update_security_params(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _max_withdrawal_percentage: Option<u64>,
    _withdrawal_cooldown: Option<u64>,
    is_paused: Option<bool>,
) -> ProgramResult {
    msg!("Processing UpdateSecurityParams");
    let account_info_iter = &mut accounts.iter();

    let owner = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if *owner.key != pool_state_data.owner {
        msg!("Only pool owner can update security parameters");
        return Err(ProgramError::InvalidAccountData);
    }

    // Only update is_paused if provided
    if let Some(paused) = is_paused {
        pool_state_data.is_paused = paused;
    }

    // Save updated state
    pool_state_data.serialize(&mut *pool_state.data.borrow_mut())?;
    msg!("Security parameters updated successfully");

    Ok(())
}

impl PoolState {
    pub fn get_packed_len() -> usize {
        32 + // owner
        32 + // token_a_mint
        32 + // token_b_mint
        32 + // token_a_vault
        32 + // token_b_vault
        32 + // lp_token_a_mint
        32 + // lp_token_b_mint
        8 +  // ratio_a_numerator
        8 +  // ratio_b_denominator
        8 +  // total_token_a_liquidity
        8 +  // total_token_b_liquidity
        1 +  // pool_authority_bump_seed
        1 +  // token_a_vault_bump_seed
        1 +  // token_b_vault_bump_seed
        1 +  // is_initialized
        RentRequirements::get_packed_len() + // rent_requirements
        1 +  // is_paused
        DelegateManagement::get_packed_len() + // delegate_management
        8 +  // collected_fees_token_a
        8 +  // collected_fees_token_b
        8 +  // total_fees_withdrawn_token_a
        8 +  // total_fees_withdrawn_token_b
        8    // swap_fee_basis_points
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Default, Clone, Copy)]
pub struct WithdrawalRecord {
    pub delegate: Pubkey,
    pub token_mint: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
    pub slot: u64,
}

impl WithdrawalRecord {
    pub fn new(delegate: Pubkey, token_mint: Pubkey, amount: u64, timestamp: i64, slot: u64) -> Self {
        Self {
            delegate,
            token_mint,
            amount,
            timestamp,
            slot,
        }
    }

    pub fn get_packed_len() -> usize {
        32 + // delegate
        32 + // token_mint
        8 +  // amount
        8 +  // timestamp
        8    // slot
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
pub struct DelegateManagement {
    pub delegates: [Pubkey; MAX_DELEGATES],
    pub delegate_count: u8,
    pub withdrawal_history: [WithdrawalRecord; 10], // Last 10 withdrawals
    pub withdrawal_history_index: u8,
}

impl DelegateManagement {
    pub fn new(owner: Pubkey, _current_slot: u64) -> Self {
        let mut delegates = [Pubkey::default(); MAX_DELEGATES];
        delegates[0] = owner; // Owner is the first delegate
        
        Self {
            delegates,
            delegate_count: 1,
            withdrawal_history: [WithdrawalRecord::default(); 10],
            withdrawal_history_index: 0,
        }
    }

    pub fn is_delegate(&self, pubkey: &Pubkey) -> bool {
        for i in 0..self.delegate_count as usize {
            if self.delegates[i] == *pubkey {
                return true;
            }
        }
        false
    }

    pub fn add_delegate(&mut self, delegate: Pubkey) -> Result<(), PoolError> {
        if self.delegate_count as usize >= MAX_DELEGATES {
            return Err(PoolError::DelegateLimitExceeded);
        }

        // Check if already a delegate
        if self.is_delegate(&delegate) {
            return Err(PoolError::DelegateAlreadyExists { delegate });
        }

        self.delegates[self.delegate_count as usize] = delegate;
        self.delegate_count += 1;
        Ok(())
    }

    pub fn remove_delegate(&mut self, delegate: Pubkey) -> Result<(), PoolError> {
        let mut found_index = None;
        for i in 0..self.delegate_count as usize {
            if self.delegates[i] == delegate {
                found_index = Some(i);
                break;
            }
        }

        if let Some(index) = found_index {
            // Shift remaining delegates
            for i in index..(self.delegate_count as usize - 1) {
                self.delegates[i] = self.delegates[i + 1];
            }
            self.delegates[self.delegate_count as usize - 1] = Pubkey::default();
            self.delegate_count -= 1;
            Ok(())
        } else {
            Err(PoolError::DelegateNotFound { delegate })
        }
    }

    pub fn add_withdrawal_record(&mut self, record: WithdrawalRecord) {
        let index = self.withdrawal_history_index as usize;
        self.withdrawal_history[index] = record;
        self.withdrawal_history_index = (self.withdrawal_history_index + 1) % 10;
    }

    pub fn get_packed_len() -> usize {
        (32 * MAX_DELEGATES) + // delegates array
        1 +  // delegate_count
        (WithdrawalRecord::get_packed_len() * 10) + // withdrawal_history
        1    // withdrawal_history_index
    }
}

/// Allows the pool owner to add a new delegate.
///
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for adding a delegate
/// * `delegate` - The public key of the new delegate
///
/// # Returns
/// * `ProgramResult` - Success or error code
fn process_add_delegate(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    delegate: Pubkey,
) -> ProgramResult {
    msg!("Processing AddDelegate for: {}", delegate);
    let account_info_iter = &mut accounts.iter();

    let owner = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer to add delegate");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if *owner.key != pool_state_data.owner {
        msg!("Only pool owner can add delegates");
        return Err(ProgramError::InvalidAccountData);
    }

    // Add the delegate
    pool_state_data.delegate_management.add_delegate(delegate)?;
    
    // Save updated state
    pool_state_data.serialize(&mut *pool_state.data.borrow_mut())?;
    
    // Log the change for transparency
    msg!("Delegate added successfully: {}. Total delegates: {}", 
         delegate, pool_state_data.delegate_management.delegate_count);

    Ok(())
}

/// Allows the pool owner to remove a delegate.
///
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for removing a delegate
/// * `delegate` - The public key of the delegate to remove
///
/// # Returns
/// * `ProgramResult` - Success or error code
fn process_remove_delegate(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    delegate: Pubkey,
) -> ProgramResult {
    msg!("Processing RemoveDelegate for: {}", delegate);
    let account_info_iter = &mut accounts.iter();

    let owner = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer to remove delegate");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if *owner.key != pool_state_data.owner {
        msg!("Only pool owner can remove delegates");
        return Err(ProgramError::InvalidAccountData);
    }

    // Remove the delegate
    pool_state_data.delegate_management.remove_delegate(delegate)?;
    
    // Save updated state
    pool_state_data.serialize(&mut *pool_state.data.borrow_mut())?;
    
    // Log the change for transparency
    msg!("Delegate removed successfully: {}. Remaining delegates: {}", 
         delegate, pool_state_data.delegate_management.delegate_count);

    Ok(())
}

/// Allows delegates to withdraw collected fees.
///
/// # Arguments
/// * `program_id` - The program ID of the contract
/// * `accounts` - The accounts required for fee withdrawal
/// * `token_mint` - The mint of the token to withdraw
/// * `amount` - The amount to withdraw
///
/// # Returns
/// * `ProgramResult` - Success or error code
fn process_withdraw_fees_to_delegate(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    token_mint: Pubkey,
    amount: u64,
) -> ProgramResult {
    msg!("Processing WithdrawFeesToDelegate for token: {}, amount: {}", token_mint, amount);
    let account_info_iter = &mut accounts.iter();

    let delegate = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;
    let token_vault = next_account_info(account_info_iter)?;
    let delegate_token_account = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let rent_sysvar = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;

    // Verify delegate is signer
    if !delegate.is_signer {
        msg!("Delegate must be a signer for fee withdrawal");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    
    // Verify pool is not paused
    if pool_state_data.is_paused {
        msg!("Fee withdrawals are paused");
        return Err(PoolError::PoolPaused.into());
    }

    // Verify caller is a delegate
    if !pool_state_data.delegate_management.is_delegate(delegate.key) {
        msg!("Caller is not an authorized delegate: {}", delegate.key);
        return Err(PoolError::DelegateNotFound { delegate: *delegate.key }.into());
    }

    // Determine token index (0 for token_a, 1 for token_b)
    let (token_index, vault_key, collected_fees) = if token_mint == pool_state_data.token_a_mint {
        (0, pool_state_data.token_a_vault, pool_state_data.collected_fees_token_a)
    } else if token_mint == pool_state_data.token_b_mint {
        (1, pool_state_data.token_b_vault, pool_state_data.collected_fees_token_b)
    } else {
        msg!("Invalid token mint for withdrawal: {}", token_mint);
        return Err(ProgramError::InvalidArgument);
    };

    // Verify vault account
    if *token_vault.key != vault_key {
        msg!("Invalid token vault provided");
        return Err(ProgramError::InvalidAccountData);
    }

    // Check if enough fees collected
    if amount > collected_fees {
        msg!("Insufficient collected fees. Available: {}, Requested: {}", collected_fees, amount);
        return Err(ProgramError::InsufficientFunds);
    }

    // Check rent exempt requirements
    let rent = &Rent::from_account_info(rent_sysvar)?;
    check_rent_exempt(pool_state, program_id, rent, clock.slot)?;

    // Transfer fees to delegate
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.token_a_mint.as_ref(),
        pool_state_data.token_b_mint.as_ref(),
        &pool_state_data.ratio_a_numerator.to_le_bytes(),
        &pool_state_data.ratio_b_denominator.to_le_bytes(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    invoke_signed(
        &token_instruction::transfer(
            token_program.key,
            token_vault.key,
            delegate_token_account.key,
            pool_state.key,
            &[],
            amount,
        )?,
        &[
            token_vault.clone(),
            delegate_token_account.clone(),
            pool_state.clone(),
            token_program.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;

    // Update pool state
    if token_index == 0 {
        pool_state_data.collected_fees_token_a = pool_state_data.collected_fees_token_a
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        pool_state_data.total_fees_withdrawn_token_a = pool_state_data.total_fees_withdrawn_token_a
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    } else {
        pool_state_data.collected_fees_token_b = pool_state_data.collected_fees_token_b
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        pool_state_data.total_fees_withdrawn_token_b = pool_state_data.total_fees_withdrawn_token_b
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }

    // Add withdrawal record
    let withdrawal_record = WithdrawalRecord::new(
        *delegate.key,
        token_mint,
        amount,
        clock.unix_timestamp,
        clock.slot,
    );
    pool_state_data.delegate_management.add_withdrawal_record(withdrawal_record);

    // Save updated state
    pool_state_data.serialize(&mut *pool_state.data.borrow_mut())?;

    // Log the withdrawal for transparency
    msg!("Fee withdrawal completed: Delegate: {}, Token: {}, Amount: {}, Timestamp: {}", 
         delegate.key, token_mint, amount, clock.unix_timestamp);

    Ok(())
}

/// Returns withdrawal history for transparency.
///
/// # Arguments
/// * `_program_id` - The program ID of the contract
/// * `accounts` - The accounts required for getting withdrawal history
///
/// # Returns
/// * `ProgramResult` - Success or error code
fn process_get_withdrawal_history(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing GetWithdrawalHistory");
    let account_info_iter = &mut accounts.iter();

    let pool_state = next_account_info(account_info_iter)?;

    // Load pool state
    let pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;

    // Log withdrawal history for transparency
    msg!("Withdrawal History (last 10 withdrawals):");
    for (i, record) in pool_state_data.delegate_management.withdrawal_history.iter().enumerate() {
        if record.delegate != Pubkey::default() {
            msg!("Record {}: Delegate: {}, Token: {}, Amount: {}, Timestamp: {}, Slot: {}", 
                 i, record.delegate, record.token_mint, record.amount, record.timestamp, record.slot);
        }
    }

    msg!("Total fees withdrawn - Token A: {}, Token B: {}", 
         pool_state_data.total_fees_withdrawn_token_a,
         pool_state_data.total_fees_withdrawn_token_b);

    msg!("Current delegates ({}):", pool_state_data.delegate_management.delegate_count);
    for i in 0..pool_state_data.delegate_management.delegate_count as usize {
        msg!("Delegate {}: {}", i, pool_state_data.delegate_management.delegates[i]);
    }

    Ok(())
}

/// Allows the pool owner to set the swap fee configuration.
///
/// # Arguments
/// * `_program_id` - The program ID of the contract
/// * `accounts` - The accounts required for setting swap fee
/// * `fee_basis_points` - The fee in basis points (0-50, max 0.5%)
///
/// # Returns
/// * `ProgramResult` - Success or error code
fn process_set_swap_fee(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    fee_basis_points: u64,
) -> ProgramResult {
    msg!("Processing SetSwapFee: {} basis points", fee_basis_points);
    let account_info_iter = &mut accounts.iter();

    let owner = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner.is_signer {
        msg!("Owner must be a signer to set swap fee");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if *owner.key != pool_state_data.owner {
        msg!("Only pool owner can set swap fees");
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate fee is within allowed range (0-50 basis points = 0%-0.5%)
    if fee_basis_points > MAX_SWAP_FEE_BASIS_POINTS {
        msg!("Swap fee {} basis points exceeds maximum of {} basis points (0.5%)", 
             fee_basis_points, MAX_SWAP_FEE_BASIS_POINTS);
        return Err(ProgramError::InvalidArgument);
    }

    // Update swap fee
    let old_fee = pool_state_data.swap_fee_basis_points;
    pool_state_data.swap_fee_basis_points = fee_basis_points;

    // Save updated state
    pool_state_data.serialize(&mut *pool_state.data.borrow_mut())?;
    
    // Log the change for transparency
    msg!("Swap fee updated: {} -> {} basis points ({:.2}% -> {:.2}%)", 
         old_fee, fee_basis_points,
         old_fee as f64 / 100.0, fee_basis_points as f64 / 100.0);

    Ok(())
}
