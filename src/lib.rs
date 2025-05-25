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
    clock::Clock,
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
const POOL_STATE_SEED_PREFIX: &[u8] = b"pool_state_v2";
const TOKEN_A_VAULT_SEED_PREFIX: &[u8] = b"token_a_vault";
const TOKEN_B_VAULT_SEED_PREFIX: &[u8] = b"token_b_vault";

// Add constant for SPL Token Program ID
const SPL_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

// Add after the existing constants
const MINIMUM_RENT_BUFFER: u64 = 1000; // Additional buffer for rent to account for potential rent increases

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct RentRequirements {
    pub pool_state_rent: u64,
    pub token_vault_rent: u64,
    pub lp_mint_rent: u64,
    pub last_update_slot: u64,
}

impl RentRequirements {
    pub fn new(rent: &Rent) -> Self {
        Self {
            pool_state_rent: rent.minimum_balance(PoolState::get_packed_len()),
            token_vault_rent: rent.minimum_balance(TokenAccount::LEN),
            lp_mint_rent: rent.minimum_balance(MintAccount::LEN),
            last_update_slot: 0,
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
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum FixedRatioInstruction {
    InitializePool {
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
    },
    WithdrawFees,
}

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
    // ... other error types with context
}

impl std::fmt::Display for PoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
            // ... other error variants
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
            // ... other error codes
        }
    }
}

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = FixedRatioInstruction::try_from_slice(instruction_data)?;
    
    match instruction {
        FixedRatioInstruction::InitializePool { 
            ratio_primary_per_base, 
            pool_authority_bump_seed, 
            primary_token_vault_bump_seed, 
            base_token_vault_bump_seed 
        } => {
            process_initialize_pool(
                program_id, 
                accounts, 
                ratio_primary_per_base, 
                pool_authority_bump_seed, 
                primary_token_vault_bump_seed, 
                base_token_vault_bump_seed
            )
        }
        FixedRatioInstruction::Deposit { deposit_token_mint, amount } => {
            process_deposit(program_id, accounts, deposit_token_mint, amount)
        }
        FixedRatioInstruction::Withdraw { withdraw_token_mint, lp_amount_to_burn } => {
            process_withdraw(program_id, accounts, withdraw_token_mint, lp_amount_to_burn)
        }
        FixedRatioInstruction::Swap { input_token_mint, amount_in } => {
            process_swap(program_id, accounts, input_token_mint, amount_in)
        }
        FixedRatioInstruction::WithdrawFees => {
            process_withdraw_fees(program_id, accounts)
        }
    }
}

// Add helper function for rent-exempt checks
fn check_rent_exempt(account: &AccountInfo, rent: &Rent, current_slot: u64) -> ProgramResult {
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

fn process_initialize_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ratio_primary_per_base: u64,
    pool_authority_bump_seed: u8,
    primary_token_vault_bump_seed: u8,
    base_token_vault_bump_seed: u8,
) -> ProgramResult {
    msg!("Processing InitializePool v2");
    let account_info_iter = &mut accounts.iter();

    let payer = next_account_info(account_info_iter)?;
    let pool_state_pda_account = next_account_info(account_info_iter)?;
    let primary_token_mint_account = next_account_info(account_info_iter)?;
    let base_token_mint_account = next_account_info(account_info_iter)?;
    let lp_token_a_mint_account = next_account_info(account_info_iter)?;
    let lp_token_b_mint_account = next_account_info(account_info_iter)?;
    let token_a_vault_pda_account = next_account_info(account_info_iter)?;
    let token_b_vault_pda_account = next_account_info(account_info_iter)?;
    let system_program_account = next_account_iter(account_info_iter)?;
    let token_program_account = next_account_iter(account_info_iter)?;
    let rent_sysvar_account = next_account_iter(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_sysvar_account)?;
    let clock_sysvar = next_account_iter(account_info_iter)?;
    let clock = &Clock::from_account_info(clock_sysvar)?;

    if !payer.is_signer {
        return Err(PoolError::InvalidTokenAccount {
            account: *payer.key,
            reason: "Payer must be a signer".to_string(),
        }.into());
    }

    if ratio_primary_per_base == 0 {
        return Err(PoolError::InvalidRatio {
            ratio: ratio_primary_per_base,
            min_ratio: 1,
            max_ratio: u64::MAX,
        }.into());
    }

    // Token Normalization & Ratio Conversion
    let (token_a_mint_key, token_b_mint_key,
         ratio_a_numerator, ratio_b_denominator,
         token_a_is_primary, _token_b_is_primary) =
        if primary_token_mint_account.key.to_bytes() < base_token_mint_account.key.to_bytes() {
            (primary_token_mint_account.key, base_token_mint_account.key,
             ratio_primary_per_base, 1, 
             true, false)
        } else if primary_token_mint_account.key.to_bytes() > base_token_mint_account.key.to_bytes() {
            (base_token_mint_account.key, primary_token_mint_account.key, 
             1, ratio_primary_per_base, 
             false, true)
        } else {
            msg!("Primary and Base token mints cannot be the same");
            return Err(ProgramError::InvalidArgument);
        };
    
    // Determine AccountInfo references for normalized mints
    let token_a_mint_account_info_ref = if token_a_is_primary {
        primary_token_mint_account
    } else {
        base_token_mint_account
    };
    let token_b_mint_account_info_ref = if token_a_is_primary {
        base_token_mint_account
    } else {
        primary_token_mint_account
    };

    // Ensure mints are actually mints
    if !primary_token_mint_account.owner.eq(&spl_token::id()) || primary_token_mint_account.data_len() != MintAccount::LEN {
        msg!("Primary token mint account is not a valid mint account");
        return Err(ProgramError::InvalidAccountData);
    }
    if !base_token_mint_account.owner.eq(&spl_token::id()) || base_token_mint_account.data_len() != MintAccount::LEN {
        msg!("Base token mint account is not a valid mint account");
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify the pool state PDA is derived correctly using normalized values
    let pool_state_pda_seeds = &[
        POOL_STATE_SEED_PREFIX,
        token_a_mint_key.as_ref(),
        token_b_mint_key.as_ref(),
        &ratio_a_numerator.to_le_bytes(),
        &ratio_b_denominator.to_le_bytes(),
        &[pool_authority_bump_seed],
    ];
    let expected_pool_state_pda = Pubkey::create_program_address(pool_state_pda_seeds, program_id)?;
    if *pool_state_pda_account.key != expected_pool_state_pda {
        msg!("Invalid Pool State PDA address. Expected {}, got {}", expected_pool_state_pda, pool_state_pda_account.key);
        return Err(ProgramError::InvalidArgument);
    }

    // Check if pool state is already initialized
    if pool_state_pda_account.data_len() > 0 && !pool_state_pda_account.data_is_empty() {
         match PoolState::try_from_slice(&pool_state_pda_account.data.borrow()) {
            Ok(pool_state_data) => {
                if pool_state_data.is_initialized {
                    msg!("Pool state already initialized");
                    return Err(ProgramError::AccountAlreadyInitialized);
                }
            }
            Err(_) => {
                // If deserialization fails for a non-empty account, it might be corrupt or not what we expect.
                // For safety, treat as an error if not zeroed. If it was zeroed, create_account will handle it.
                // This check mainly prevents re-initialization over existing *valid* pool state.
                let is_zeroed = pool_state_pda_account.data.borrow().iter().all(|&x| x == 0);
                if !is_zeroed {
                    msg!("Pool state account has data but is not a valid PoolState struct and not zeroed.");
                    return Err(ProgramError::InvalidAccountData);
                }
            }
        }
    }

    // Map input vault bump seeds to normalized token_a and token_b vault bump seeds
    let (token_a_vault_bump, token_b_vault_bump) = if token_a_is_primary {
        (primary_token_vault_bump_seed, base_token_vault_bump_seed)
    } else {
        (base_token_vault_bump_seed, primary_token_vault_bump_seed)
    };

    // Verify token_a_vault PDA
    let token_a_vault_pda_seeds = &[
        TOKEN_A_VAULT_SEED_PREFIX,
        pool_state_pda_account.key.as_ref(),
        &[token_a_vault_bump],
    ];
    let expected_token_a_vault_pda = Pubkey::create_program_address(token_a_vault_pda_seeds, program_id)?;
    if *token_a_vault_pda_account.key != expected_token_a_vault_pda {
        msg!("Invalid Token A Vault PDA address. Expected {}, got {}", expected_token_a_vault_pda, token_a_vault_pda_account.key);
        return Err(ProgramError::InvalidArgument);
    }

    // Verify token_b_vault PDA
    let token_b_vault_pda_seeds = &[
        TOKEN_B_VAULT_SEED_PREFIX,
        pool_state_pda_account.key.as_ref(),
        &[token_b_vault_bump],
    ];
    let expected_token_b_vault_pda = Pubkey::create_program_address(token_b_vault_pda_seeds, program_id)?;
    if *token_b_vault_pda_account.key != expected_token_b_vault_pda {
        msg!("Invalid Token B Vault PDA address. Expected {}, got {}", expected_token_b_vault_pda, token_b_vault_pda_account.key);
        return Err(ProgramError::InvalidArgument);
    }
    
    if payer.lamports() < REGISTRATION_FEE {
        msg!("Insufficient SOL for registration fee");
        return Err(ProgramError::InsufficientFunds);
    }

    // Transfer registration fee to pool state PDA (this account will be created shortly)
    invoke(
        &system_instruction::transfer(payer.key, pool_state_pda_account.key, REGISTRATION_FEE),
        &[
            payer.clone(),
            pool_state_pda_account.clone(),
            system_program_account.clone(),
        ],
    )?;
    msg!("Registration fee transferred to pool state PDA (pending creation)");

    // Create Pool State PDA account
    let pool_state_account_size = PoolState::get_packed_len();
    let rent_for_pool_state = rent.minimum_balance(pool_state_account_size);
    msg!("Creating Pool State PDA account: {}", pool_state_pda_account.key);
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

    // Create and Initialize LP Token A Mint
    let rent_for_mint = rent.minimum_balance(MintAccount::LEN);
    msg!("Creating LP Token A Mint account: {}", lp_token_a_mint_account.key);
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
    msg!("LP Token A Mint account created. Initializing...");
    invoke_signed(
        &token_instruction::initialize_mint(
            token_program_account.key,
            lp_token_a_mint_account.key,
            pool_state_pda_account.key,
            None,
            9,
        )?,
        &[
            lp_token_a_mint_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
            pool_state_pda_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;
    msg!("LP Token A Mint initialized");

    // Create and Initialize LP Token B Mint
    msg!("Creating LP Token B Mint account: {}", lp_token_b_mint_account.key);
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
    msg!("LP Token B Mint account created. Initializing...");
    invoke_signed(
        &token_instruction::initialize_mint(
            token_program_account.key,
            lp_token_b_mint_account.key,
            pool_state_pda_account.key,
            None,
            9,
        )?,
        &[
            lp_token_b_mint_account.clone(),
            rent_sysvar_account.clone(),
            token_program_account.clone(),
            pool_state_pda_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;
    msg!("LP Token B Mint initialized");

    // Create and Initialize Token A Vault PDA
    let vault_account_size = TokenAccount::LEN;
    let rent_for_vault = rent.minimum_balance(vault_account_size);
    msg!("Creating Token A Vault PDA account: {}", token_a_vault_pda_account.key);
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
    msg!("Token A Vault PDA account created. Initializing...");
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
    msg!("Token A Vault PDA initialized");

    // Create and Initialize Token B Vault PDA
    msg!("Creating Token B Vault PDA account: {}", token_b_vault_pda_account.key);
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
    msg!("Token B Vault PDA account created. Initializing...");
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
    msg!("Token B Vault PDA initialized");

    msg!("Initializing Pool State data");
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

    // Initialize rent requirements
    let rent_requirements = RentRequirements::new(rent);
    
    // Update pool state data
    pool_state_data.rent_requirements = rent_requirements;
    
    pool_state_data.serialize(&mut *pool_state_pda_account.data.borrow_mut())?;
    msg!("Pool State PDA initialized with data: {:?}", pool_state_data);

    Ok(())
}

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
    let clock_sysvar = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(clock_sysvar)?;

    // Check rent-exempt status for pool accounts
    check_rent_exempt(pool_state_account, rent, clock.slot)?;
    check_rent_exempt(pool_token_a_vault_account, rent, clock.slot)?;
    check_rent_exempt(pool_token_b_vault_account, rent, clock.slot)?;
    check_rent_exempt(lp_token_a_mint_account, rent, clock.slot)?;
    check_rent_exempt(lp_token_b_mint_account, rent, clock.slot)?;

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
            .ok_or(ProgramError::Overflow)?;
    } else {
        pool_state_data.total_token_b_liquidity = pool_state_data.total_token_b_liquidity.checked_add(amount)
            .ok_or(ProgramError::Overflow)?;
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

fn process_withdraw(
    _program_id: &Pubkey, // Not directly used unless for PDA derivation if not passed in accounts
    accounts: &[AccountInfo],
    withdraw_token_mint_key: Pubkey, // Key of the underlying token user wants to withdraw
    lp_amount_to_burn: u64,         // Amount of LP tokens to burn (equals underlying amount out)
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
    let clock_sysvar = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(clock_sysvar)?;

    // Check rent-exempt status for pool accounts
    check_rent_exempt(pool_state_account, rent, clock.slot)?;
    check_rent_exempt(pool_token_a_vault_account, rent, clock.slot)?;
    check_rent_exempt(pool_token_b_vault_account, rent, clock.slot)?;
    check_rent_exempt(lp_token_a_mint_account, rent, clock.slot)?;
    check_rent_exempt(lp_token_b_mint_account, rent, clock.slot)?;

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
    let (source_pool_vault_account, source_lp_mint_account, is_withdrawing_token_a) = 
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
        if pool_state_data.total_token_b_liquidity < lp_amount_to_burn {
            msg!("Insufficient token B liquidity in the pool for withdrawal.");
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
           lp_amount_to_burn, withdraw_token_mint_key, source_pool_vault_account.key, user_destination_token_account.key);
    invoke_signed(
        &token_instruction::transfer(
            token_program_account.key,
            source_pool_vault_account.key,         // Pool's vault (source)
            user_destination_token_account.key,    // User's token account (destination)
            pool_state_account.key,                // Pool PDA is the authority over its vault
            &[],
            lp_amount_to_burn,                     // Amount of underlying token to transfer (equals LP burned)
        )?,
        &[
            source_pool_vault_account.clone(),
            user_destination_token_account.clone(),
            pool_state_account.clone(),
            token_program_account.clone(),
        ],
        &[pool_state_pda_seeds],
    )?;

    // Update pool state liquidity
    if is_withdrawing_token_a {
        pool_state_data.total_token_a_liquidity = pool_state_data.total_token_a_liquidity.checked_sub(lp_amount_to_burn)
            .ok_or(ProgramError::Overflow)?;
    } else {
        pool_state_data.total_token_b_liquidity = pool_state_data.total_token_b_liquidity.checked_sub(lp_amount_to_burn)
            .ok_or(ProgramError::Overflow)?;
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

fn process_swap(
    _program_id: &Pubkey, // Not directly used unless for PDA derivation if not passed in accounts
    accounts: &[AccountInfo],
    input_token_mint_key: Pubkey, // Mint of the token user is providing for the swap
    amount_in: u64,               // Amount of the input token
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
    let clock_sysvar = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(clock_sysvar)?;

    // Check rent-exempt status for pool accounts
    check_rent_exempt(pool_state_account, rent, clock.slot)?;
    check_rent_exempt(pool_token_a_vault_account, rent, clock.slot)?;
    check_rent_exempt(pool_token_b_vault_account, rent, clock.slot)?;

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
            // Swapping Token A for Token B
            if *pool_token_a_vault_account.key != pool_state_data.token_a_vault || 
               *pool_token_b_vault_account.key != pool_state_data.token_b_vault {
                msg!("Invalid pool vault accounts provided for A -> B swap.");
                return Err(ProgramError::InvalidAccountData);
            }
            (pool_token_a_vault_account, pool_token_b_vault_account, pool_state_data.token_b_mint, true)
        } else if input_token_mint_key == pool_state_data.token_b_mint {
            // Swapping Token B for Token A
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
            .ok_or(ProgramError::Overflow)?
            .checked_div(pool_state_data.ratio_a_numerator)
            .ok_or(ProgramError::ArithmeticOverflow)? // Using ArithmeticOverflow for division issues
    } else {
        // Swapping B for A: amount_out_A = (amount_in_B * ratio_A_numerator) / ratio_B_denominator
        if pool_state_data.ratio_b_denominator == 0 {
            msg!("Pool ratio_b_denominator is zero, cannot perform swap.");
            return Err(ProgramError::InvalidAccountData);
        }
        amount_in.checked_mul(pool_state_data.ratio_a_numerator)
            .ok_or(ProgramError::Overflow)?
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

    // Transfer input tokens from user to pool vault
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

    // Update pool state liquidity
    if input_is_token_a {
        pool_state_data.total_token_a_liquidity = pool_state_data.total_token_a_liquidity.checked_add(amount_in)
            .ok_or(ProgramError::Overflow)?;
        pool_state_data.total_token_b_liquidity = pool_state_data.total_token_b_liquidity.checked_sub(amount_out)
            .ok_or(ProgramError::Overflow)?;
    } else {
        pool_state_data.total_token_b_liquidity = pool_state_data.total_token_b_liquidity.checked_add(amount_in)
            .ok_or(ProgramError::Overflow)?;
        pool_state_data.total_token_a_liquidity = pool_state_data.total_token_a_liquidity.checked_sub(amount_out)
            .ok_or(ProgramError::Overflow)?;
    }
    pool_state_data.serialize(&mut *pool_state_account.data.borrow_mut())?;
    msg!("Pool liquidity updated after swap. Token A: {}, Token B: {}", 
           pool_state_data.total_token_a_liquidity, pool_state_data.total_token_b_liquidity);

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

fn process_withdraw_fees(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Processing WithdrawFees");
    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let pool_state = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let rent_sysvar = next_account_info(account_info_iter)?; // Add rent sysvar account

    if !owner.is_signer {
        msg!("Owner must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Get pool state data to verify ownership
    let pool_state_data = PoolState::try_from_slice(&pool_state.data.borrow())?;
    if !pool_state_data.is_initialized {
        msg!("Pool not initialized");
        return Err(ProgramError::UninitializedAccount);
    }

    // Verify the caller is the pool owner
    if owner.key != &pool_state_data.owner {
        msg!("Only pool owner can withdraw fees");
        return Err(ProgramError::InvalidAccountData);
    }

    // Get the current balance of the pool state PDA
    let fees = pool_state.lamports();
    if fees == 0 {
        msg!("No fees to withdraw");
        return Ok(());
    }

    // Calculate minimum rent-exempt balance
    let rent = &Rent::from_account_info(rent_sysvar)?;
    let minimum_rent = rent.minimum_balance(pool_state.data_len());
    
    // Calculate maximum withdrawable amount (current balance minus minimum rent)
    let withdrawable_amount = fees.checked_sub(minimum_rent)
        .ok_or(ProgramError::InsufficientFunds)?;

    if withdrawable_amount == 0 {
        msg!("Cannot withdraw fees - would leave account below rent-exempt threshold");
        return Err(ProgramError::InsufficientFunds);
    }

    // Transfer fees from pool state PDA to owner, leaving enough for rent
    invoke(
        &system_instruction::transfer(pool_state.key, owner.key, withdrawable_amount),
        &[pool_state.clone(), owner.clone(), system_program.clone()],
    )?;
    msg!("Fees transferred to owner: {} lamports ({} lamports reserved for rent)", 
         withdrawable_amount, minimum_rent);

    Ok(())
}

// Add helper function for rent management
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
        8 +  // pool_state_rent
        8 +  // token_vault_rent
        8 +  // lp_mint_rent
        8    // last_update_slot
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
