use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
    program_pack::Pack,
};
use spl_token::{
    instruction as token_instruction,
    state::{Account as TokenAccount, Mint as MintAccount},
};
use spl_associated_token_account::instruction as associated_token_instruction;

// Constants for fees
const REGISTRATION_FEE: u64 = 1_150_000_000; // 1.15 SOL
const DEPOSIT_WITHDRAWAL_FEE: u64 = 1_300_000; // 0.0013 SOL
const SWAP_FEE: u64 = 12_500; // 0.0000125 SOL

// PDA Seeds
const POOL_STATE_SEED_PREFIX: &[u8] = b"pool_state";
const PRIMARY_VAULT_SEED_PREFIX: &[u8] = b"primary_vault";
const BASE_VAULT_SEED_PREFIX: &[u8] = b"base_vault";

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct PoolState {
    pub owner: Pubkey,
    pub primary_token_mint: Pubkey,
    pub base_token_mint: Pubkey,
    pub primary_token_vault: Pubkey,
    pub base_token_vault: Pubkey,
    pub lp_token_mint: Pubkey,
    pub ratio: u64,
    pub total_primary_tokens: u64,
    pub total_base_tokens: u64,
    pub pool_authority_bump_seed: u8,
    pub primary_vault_bump_seed: u8,
    pub base_vault_bump_seed: u8,
    pub is_initialized: bool,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum FixedRatioInstruction {
    InitializePool {
        ratio: u64,
        pool_authority_bump_seed: u8,
        primary_vault_bump_seed: u8,
        base_vault_bump_seed: u8,
    },
    Deposit {
        amount: u64,
    },
    Withdraw {
        amount: u64,
    },
    SwapPrimaryToBase {
        amount: u64,
    },
    SwapBaseToPrimary {
        amount: u64,
    },
    WithdrawFees,
}

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = FixedRatioInstruction::try_from_slice(instruction_data)?;
    
    match instruction {
        FixedRatioInstruction::InitializePool { ratio, pool_authority_bump_seed, primary_vault_bump_seed, base_vault_bump_seed } => {
            process_initialize_pool(program_id, accounts, ratio, pool_authority_bump_seed, primary_vault_bump_seed, base_vault_bump_seed)
        }
        FixedRatioInstruction::Deposit { amount } => {
            process_deposit(program_id, accounts, amount)
        }
        FixedRatioInstruction::Withdraw { amount } => {
            process_withdraw(program_id, accounts, amount)
        }
        FixedRatioInstruction::SwapPrimaryToBase { amount } => {
            process_swap_primary_to_base(program_id, accounts, amount)
        }
        FixedRatioInstruction::SwapBaseToPrimary { amount } => {
            process_swap_base_to_primary(program_id, accounts, amount)
        }
        FixedRatioInstruction::WithdrawFees => {
            process_withdraw_fees(program_id, accounts)
        }
    }
}

fn process_initialize_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ratio: u64,
    pool_authority_bump_seed: u8,
    primary_vault_bump_seed: u8,
    base_vault_bump_seed: u8,
) -> ProgramResult {
    msg!("Processing InitializePool");
    let account_info_iter = &mut accounts.iter();

    let payer = next_account_info(account_info_iter)?;
    let pool_state_pda_account = next_account_info(account_info_iter)?;
    let primary_token_mint_account = next_account_info(account_info_iter)?;
    let base_token_mint_account = next_account_info(account_info_iter)?;
    let primary_token_vault_pda_account = next_account_info(account_info_iter)?;
    let base_token_vault_pda_account = next_account_info(account_info_iter)?;
    let lp_token_mint_account = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;
    let token_program_account = next_account_info(account_info_iter)?;
    let rent_sysvar_account = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_sysvar_account)?;

    if !payer.is_signer {
        msg!("Payer must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        primary_token_mint_account.key.as_ref(),
        base_token_mint_account.key.as_ref(),
        &[pool_authority_bump_seed],
    ];
    let expected_pool_state_pda = Pubkey::create_program_address(pool_state_pda_seeds, program_id)?;
    if *pool_state_pda_account.key != expected_pool_state_pda {
        msg!("Invalid Pool State PDA address");
        return Err(ProgramError::InvalidArgument);
    }

    let primary_vault_pda_seeds = &[
        PRIMARY_VAULT_SEED_PREFIX,
        pool_state_pda_account.key.as_ref(),
        &[primary_vault_bump_seed],
    ];
    let expected_primary_vault_pda = Pubkey::create_program_address(primary_vault_pda_seeds, program_id)?;
    if *primary_token_vault_pda_account.key != expected_primary_vault_pda {
        msg!("Invalid Primary Token Vault PDA address");
        return Err(ProgramError::InvalidArgument);
    }

    let base_vault_pda_seeds = &[
        BASE_VAULT_SEED_PREFIX,
        pool_state_pda_account.key.as_ref(),
        &[base_vault_bump_seed],
    ];
    let expected_base_vault_pda = Pubkey::create_program_address(base_vault_pda_seeds, program_id)?;
    if *base_token_vault_pda_account.key != expected_base_vault_pda {
        msg!("Invalid Base Token Vault PDA address");
        return Err(ProgramError::InvalidArgument);
    }
    
    if ratio == 0 {
        msg!("Ratio cannot be zero");
        return Err(ProgramError::InvalidArgument);
    }

    if payer.lamports() < REGISTRATION_FEE {
        msg!("Insufficient SOL for registration fee");
        return Err(ProgramError::InsufficientFunds);
    }
    invoke(
        &system_instruction::transfer(payer.key, program_id, REGISTRATION_FEE),
        &[
            payer.clone(),
            program_id,
            system_program_account.clone(),
        ],
    )?;
    msg!("Registration fee transferred");

    let pool_state_account_size = PoolState::get_packed_len();
    let rent_for_pool_state = rent.minimum_balance(pool_state_account_size);
    msg!("Creating Pool State PDA account");
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
    msg!("Pool State PDA account created");

    let vault_account_size = TokenAccount::LEN;
    let rent_for_vault = rent.minimum_balance(vault_account_size);
    msg!("Creating Primary Token Vault PDA account");
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            primary_token_vault_pda_account.key,
            rent_for_vault,
            vault_account_size as u64,
            token_program_account.key,
        ),
        &[
            payer.clone(),
            primary_token_vault_pda_account.clone(),
            system_program_account.clone(),
        ],
        &[primary_vault_pda_seeds],
    )?;
    msg!("Primary Token Vault PDA account created. Initializing...");
    invoke_signed(
        &token_instruction::initialize_account(
            token_program_account.key,
            primary_token_vault_pda_account.key,
            primary_token_mint_account.key,
            pool_state_pda_account.key,
        )?,
        &[
            primary_token_vault_pda_account.clone(),
            primary_token_mint_account.clone(),
            pool_state_pda_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ],
        &[primary_vault_pda_seeds],
    )?;
    invoke(
        &token_instruction::initialize_account(
            token_program_account.key,
            primary_token_vault_pda_account.key,
            primary_token_mint_account.key,
            pool_state_pda_account.key,
        )?,
        &[
            primary_token_vault_pda_account.clone(),
            primary_token_mint_account.clone(),
            pool_state_pda_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ]
    )?;
    msg!("Primary Token Vault PDA initialized");

    msg!("Creating Base Token Vault PDA account");
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            base_token_vault_pda_account.key,
            rent_for_vault,
            vault_account_size as u64,
            token_program_account.key,
        ),
        &[
            payer.clone(),
            base_token_vault_pda_account.clone(),
            system_program_account.clone(),
        ],
        &[base_vault_pda_seeds],
    )?;
    msg!("Base Token Vault PDA account created. Initializing...");
    invoke(
        &token_instruction::initialize_account(
            token_program_account.key,
            base_token_vault_pda_account.key,
            base_token_mint_account.key,
            pool_state_pda_account.key,
        )?,
        &[
            base_token_vault_pda_account.clone(),
            base_token_mint_account.clone(),
            pool_state_pda_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
        ]
    )?;
    msg!("Base Token Vault PDA initialized");

    msg!("Setting LP Token Mint Authority");
    invoke_signed(
        &token_instruction::set_authority(
            token_program_account.key,
            lp_token_mint_account.key,
            Some(pool_state_pda_account.key),
            token_instruction::AuthorityType::MintTokens,
            payer.key,
            &[payer.key],
        )?,
        &[
            lp_token_mint_account.clone(),
            payer.clone(),
            token_program_account.clone(),
        ],
    )?;
    msg!("LP Token Mint Authority set to Pool State PDA");

    msg!("Initializing Pool State data");
    let mut pool_state_data = PoolState::try_from_slice(&pool_state_pda_account.data.borrow())?;
    if pool_state_data.is_initialized {
        msg!("Pool already initialized");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    pool_state_data.owner = *payer.key;
    pool_state_data.primary_token_mint = *primary_token_mint_account.key;
    pool_state_data.base_token_mint = *base_token_mint_account.key;
    pool_state_data.primary_token_vault = *primary_token_vault_pda_account.key;
    pool_state_data.base_token_vault = *base_token_vault_pda_account.key;
    pool_state_data.lp_token_mint = *lp_token_mint_account.key;
    pool_state_data.ratio = ratio;
    pool_state_data.total_primary_tokens = 0;
    pool_state_data.total_base_tokens = 0;
    pool_state_data.pool_authority_bump_seed = pool_authority_bump_seed;
    pool_state_data.primary_vault_bump_seed = primary_vault_bump_seed;
    pool_state_data.base_vault_bump_seed = base_vault_bump_seed;
    pool_state_data.is_initialized = true;

    pool_state_data.serialize(&mut *pool_state_pda_account.data.borrow_mut())?;
    msg!("Pool State PDA initialized with data");

    Ok(())
}

fn process_deposit(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    msg!("Processing Deposit");
    let account_info_iter = &mut accounts.iter();
    let user = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;
    let user_token_account = next_account_info(account_info_iter)?;
    let pool_token_vault = next_account_info(account_info_iter)?;
    let lp_token_mint = next_account_info(account_info_iter)?;
    let user_lp_token_account = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    // Verify deposit fee
    if user.lamports() < DEPOSIT_WITHDRAWAL_FEE {
        msg!("Insufficient SOL for deposit fee");
        return Err(ProgramError::InsufficientFunds);
    }

    // Get pool state data to access bump seed
    let pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if !pool_state_data.is_initialized {
        msg!("Pool not initialized");
        return Err(ProgramError::UninitializedAccount);
    }

    // Create seeds for pool state PDA
    let pool_state_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.primary_token_mint.as_ref(),
        pool_state_data.base_token_mint.as_ref(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    // Transfer primary tokens from user to pool vault
    invoke(
        &token_instruction::transfer(
            token_program.key,
            user_token_account.key,
            pool_token_vault.key,
            user.key,
            &[],
            amount,
        )?,
        &[
            user_token_account.clone(),
            pool_token_vault.clone(),
            user.clone(),
            token_program.clone(),
        ],
    )?;
    msg!("Primary tokens transferred to pool vault");

    // Mint LP tokens to user using pool state PDA as authority
    invoke_signed(
        &token_instruction::mint_to(
            token_program.key,
            lp_token_mint.key,
            user_lp_token_account.key,
            pool_state.key, // Authority is the pool state PDA
            &[],
            amount,
        )?,
        &[
            lp_token_mint.clone(),
            user_lp_token_account.clone(),
            pool_state.clone(),
            token_program.clone(),
        ],
        &[pool_state_seeds],
    )?;
    msg!("LP tokens minted to user");

    // Update pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    pool_state_data.total_primary_tokens = pool_state_data.total_primary_tokens.checked_add(amount)
        .ok_or(ProgramError::Overflow)?;
    pool_state_data.serialize(&mut *pool_state.data.borrow_mut())?;
    msg!("Pool state updated");

    // Transfer deposit fee
    invoke(
        &system_instruction::transfer(user.key, program_id, DEPOSIT_WITHDRAWAL_FEE),
        &[user.clone(), system_program.clone()],
    )?;
    msg!("Deposit fee transferred");

    Ok(())
}

fn process_withdraw(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    msg!("Processing Withdraw");
    let account_info_iter = &mut accounts.iter();
    let user = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;
    let user_token_account = next_account_info(account_info_iter)?;
    let pool_token_vault = next_account_info(account_info_iter)?;
    let lp_token_mint = next_account_info(account_info_iter)?;
    let user_lp_token_account = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    // Verify withdrawal fee
    if user.lamports() < DEPOSIT_WITHDRAWAL_FEE {
        msg!("Insufficient SOL for withdrawal fee");
        return Err(ProgramError::InsufficientFunds);
    }

    // Get pool state data to access bump seed
    let pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if !pool_state_data.is_initialized {
        msg!("Pool not initialized");
        return Err(ProgramError::UninitializedAccount);
    }

    // Create seeds for pool state PDA
    let pool_state_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.primary_token_mint.as_ref(),
        pool_state_data.base_token_mint.as_ref(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    // Burn LP tokens from user
    invoke(
        &token_instruction::burn(
            token_program.key,
            user_lp_token_account.key,
            lp_token_mint.key,
            user.key,
            &[],
            amount,
        )?,
        &[
            user_lp_token_account.clone(),
            lp_token_mint.clone(),
            user.clone(),
            token_program.clone(),
        ],
    )?;
    msg!("LP tokens burned from user");

    // Transfer primary tokens from pool vault to user using pool state PDA as authority
    invoke_signed(
        &token_instruction::transfer(
            token_program.key,
            pool_token_vault.key,
            user_token_account.key,
            pool_state.key, // Authority is the pool state PDA
            &[],
            amount,
        )?,
        &[
            pool_token_vault.clone(),
            user_token_account.clone(),
            pool_state.clone(),
            token_program.clone(),
        ],
        &[pool_state_seeds],
    )?;
    msg!("Primary tokens transferred to user");

    // Update pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    pool_state_data.total_primary_tokens = pool_state_data.total_primary_tokens.checked_sub(amount)
        .ok_or(ProgramError::Overflow)?;
    pool_state_data.serialize(&mut *pool_state.data.borrow_mut())?;
    msg!("Pool state updated");

    // Transfer withdrawal fee
    invoke(
        &system_instruction::transfer(user.key, program_id, DEPOSIT_WITHDRAWAL_FEE),
        &[user.clone(), system_program.clone()],
    )?;
    msg!("Withdrawal fee transferred");

    Ok(())
}

fn process_swap_primary_to_base(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    msg!("Processing Swap Primary to Base");
    let account_info_iter = &mut accounts.iter();
    let user = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;
    let user_primary_token_account = next_account_info(account_info_iter)?;
    let user_base_token_account = next_account_info(account_info_iter)?;
    let pool_primary_token_vault = next_account_info(account_info_iter)?;
    let pool_base_token_vault = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    // Verify swap fee
    if user.lamports() < SWAP_FEE {
        msg!("Insufficient SOL for swap fee");
        return Err(ProgramError::InsufficientFunds);
    }

    // Get pool state data to access bump seed
    let pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if !pool_state_data.is_initialized {
        msg!("Pool not initialized");
        return Err(ProgramError::UninitializedAccount);
    }

    // Create seeds for pool state PDA
    let pool_state_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.primary_token_mint.as_ref(),
        pool_state_data.base_token_mint.as_ref(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    let base_amount = amount.checked_mul(pool_state_data.ratio)
        .ok_or(ProgramError::Overflow)?;

    // Verify pool has enough base tokens
    if base_amount > pool_state_data.total_base_tokens {
        msg!("Insufficient base tokens in pool");
        return Err(ProgramError::InsufficientFunds);
    }

    // Transfer primary tokens from user to pool
    invoke(
        &token_instruction::transfer(
            token_program.key,
            user_primary_token_account.key,
            pool_primary_token_vault.key,
            user.key,
            &[],
            amount,
        )?,
        &[
            user_primary_token_account.clone(),
            pool_primary_token_vault.clone(),
            user.clone(),
            token_program.clone(),
        ],
    )?;
    msg!("Primary tokens transferred to pool");

    // Transfer base tokens from pool to user using pool state PDA as authority
    invoke_signed(
        &token_instruction::transfer(
            token_program.key,
            pool_base_token_vault.key,
            user_base_token_account.key,
            pool_state.key, // Authority is the pool state PDA
            &[],
            base_amount,
        )?,
        &[
            pool_base_token_vault.clone(),
            user_base_token_account.clone(),
            pool_state.clone(),
            token_program.clone(),
        ],
        &[pool_state_seeds],
    )?;
    msg!("Base tokens transferred to user");

    // Update pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    pool_state_data.total_primary_tokens = pool_state_data.total_primary_tokens.checked_add(amount)
        .ok_or(ProgramError::Overflow)?;
    pool_state_data.total_base_tokens = pool_state_data.total_base_tokens.checked_sub(base_amount)
        .ok_or(ProgramError::Overflow)?;
    pool_state_data.serialize(&mut *pool_state.data.borrow_mut())?;
    msg!("Pool state updated");

    // Transfer swap fee
    invoke(
        &system_instruction::transfer(user.key, program_id, SWAP_FEE),
        &[user.clone(), system_program.clone()],
    )?;
    msg!("Swap fee transferred");

    Ok(())
}

fn process_swap_base_to_primary(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    msg!("Processing Swap Base to Primary");
    let account_info_iter = &mut accounts.iter();
    let user = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;
    let user_primary_token_account = next_account_info(account_info_iter)?;
    let user_base_token_account = next_account_info(account_info_iter)?;
    let pool_primary_token_vault = next_account_info(account_info_iter)?;
    let pool_base_token_vault = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    // Verify swap fee
    if user.lamports() < SWAP_FEE {
        msg!("Insufficient SOL for swap fee");
        return Err(ProgramError::InsufficientFunds);
    }

    // Get pool state data to access bump seed
    let pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if !pool_state_data.is_initialized {
        msg!("Pool not initialized");
        return Err(ProgramError::UninitializedAccount);
    }

    // Create seeds for pool state PDA
    let pool_state_seeds = &[
        POOL_STATE_SEED_PREFIX,
        pool_state_data.primary_token_mint.as_ref(),
        pool_state_data.base_token_mint.as_ref(),
        &[pool_state_data.pool_authority_bump_seed],
    ];

    let primary_amount = amount.checked_div(pool_state_data.ratio)
        .ok_or(ProgramError::Overflow)?;

    // Verify pool has enough primary tokens
    if primary_amount > pool_state_data.total_primary_tokens {
        msg!("Insufficient primary tokens in pool");
        return Err(ProgramError::InsufficientFunds);
    }

    // Transfer base tokens from user to pool
    invoke(
        &token_instruction::transfer(
            token_program.key,
            user_base_token_account.key,
            pool_base_token_vault.key,
            user.key,
            &[],
            amount,
        )?,
        &[
            user_base_token_account.clone(),
            pool_base_token_vault.clone(),
            user.clone(),
            token_program.clone(),
        ],
    )?;
    msg!("Base tokens transferred to pool");

    // Transfer primary tokens from pool to user using pool state PDA as authority
    invoke_signed(
        &token_instruction::transfer(
            token_program.key,
            pool_primary_token_vault.key,
            user_primary_token_account.key,
            pool_state.key, // Authority is the pool state PDA
            &[],
            primary_amount,
        )?,
        &[
            pool_primary_token_vault.clone(),
            user_primary_token_account.clone(),
            pool_state.clone(),
            token_program.clone(),
        ],
        &[pool_state_seeds],
    )?;
    msg!("Primary tokens transferred to user");

    // Update pool state
    let mut pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    pool_state_data.total_primary_tokens = pool_state_data.total_primary_tokens.checked_sub(primary_amount)
        .ok_or(ProgramError::Overflow)?;
    pool_state_data.total_base_tokens = pool_state_data.total_base_tokens.checked_add(amount)
        .ok_or(ProgramError::Overflow)?;
    pool_state_data.serialize(&mut *pool_state.data.borrow_mut())?;
    msg!("Pool state updated");

    // Transfer swap fee
    invoke(
        &system_instruction::transfer(user.key, program_id, SWAP_FEE),
        &[user.clone(), system_program.clone()],
    )?;
    msg!("Swap fee transferred");

    Ok(())
}

fn process_withdraw_fees(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    if owner.key != program_id {
        return Err(ProgramError::InvalidAccountData);
    }

    let fees = program_id.lamports();
    invoke(
        &system_instruction::transfer(program_id, owner.key, fees),
        &[program_id.clone(), owner.clone()],
    )?;

    Ok(())
}

impl PoolState {
    pub fn get_packed_len() -> usize {
        (32 * 6) + (8 * 3) + (1 * 3) + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
