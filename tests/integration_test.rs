use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar, clock::Clock},
    program_pack::Pack,
};
use solana_sdk::account::Account as SdkAccount;
use spl_token::{
    instruction as token_instruction,
    state::{Account as TokenAccount, Mint as MintAccount},
};
use std::fmt;
use bincode;

// Import your contract's instruction enum and PoolState struct
use fixed_ratio_trading::{FixedRatioInstruction, PoolState};

// Helper function to create a token mint
async fn create_token_mint<'a>(
    payer: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    authority: &Pubkey,
    decimals: u8,
    token_program: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    rent: &AccountInfo<'a>,
) -> ProgramResult {
    invoke(
        &system_instruction::create_account(
            payer.key,
            mint.key,
            Rent::from_account_info(rent)?.minimum_balance(MintAccount::LEN),
            MintAccount::LEN as u64,
            token_program.key,
        ),
        &[payer.clone(), mint.clone(), system_program.clone()],
    )?;
    invoke(
        &token_instruction::initialize_mint(
            token_program.key,
            mint.key,
            authority,
            None,
            decimals,
        )?,
        &[mint.clone(), rent.clone(), token_program.clone()],
    )?;
    Ok(())
}

// Helper function to create a token account
async fn create_token_account<'a>(
    payer: &AccountInfo<'a>,
    account: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    owner: &Pubkey,
    token_program: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    rent: &AccountInfo<'a>,
) -> ProgramResult {
    invoke(
        &system_instruction::create_account(
            payer.key,
            account.key,
            Rent::from_account_info(rent)?.minimum_balance(TokenAccount::LEN),
            TokenAccount::LEN as u64,
            token_program.key,
        ),
        &[payer.clone(), account.clone(), system_program.clone()],
    )?;
    invoke(
        &token_instruction::initialize_account(
            token_program.key,
            account.key,
            mint.key,
            owner,
        )?,
        &[account.clone(), mint.clone(), rent.clone(), token_program.clone()],
    )?;
    Ok(())
}

// Helper function to mint tokens
async fn mint_tokens<'a>(
    mint: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    amount: u64,
    token_program: &AccountInfo<'a>,
) -> ProgramResult {
    invoke(
        &token_instruction::mint_to(
            token_program.key,
            mint.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?,
        &[mint.clone(), destination.clone(), authority.clone(), token_program.clone()],
    )?;
    Ok(())
}

#[tokio::test]
async fn test_initialize_pool() {
    // Setup test accounts
    let program_id_val = Pubkey::new_unique();
    let program_id = program_id_val;
    let payer_key = Pubkey::new_unique();
    let mut payer_lamports = 100_000_000_000; // Ensure enough for all fees and rents
    let payer_owner_id = solana_program::system_program::id(); 
    let mut payer_data = vec![];
    let payer = AccountInfo::new(
        &payer_key,
        true, // Payer must be signer
        true, // Payer must be writable for lamport transfers
        &mut payer_lamports,
        &mut payer_data, // Payer data is usually empty
        &payer_owner_id,
        false,
        0,
    );

    // Mints - key can be unique, owner must be spl_token::id()
    let token_a_mint_key = Pubkey::new_unique();
    let mut token_a_mint_lamports = 0; // Will be funded by create_account in create_token_mint
    let mut token_a_mint_data = vec![0; MintAccount::LEN]; // Initialize with some data for mint
    let token_a_mint_owner = spl_token::id();
    let token_a_mint = AccountInfo::new(
        &token_a_mint_key,
        false, // Not signer for this AccountInfo instance itself
        true, // Mints are writable during initialization by create_token_mint
        &mut token_a_mint_lamports,
        &mut token_a_mint_data,
        &token_a_mint_owner,
        false,
        0,
    );

    let token_b_mint_key = Pubkey::new_unique();
    let mut token_b_mint_lamports = 0;
    let mut token_b_mint_data = vec![0; MintAccount::LEN]; 
    let token_b_mint_owner = spl_token::id();
    let token_b_mint = AccountInfo::new(
        &token_b_mint_key,
        false,
        true, 
        &mut token_b_mint_lamports,
        &mut token_b_mint_data,
        &token_b_mint_owner,
        false,
        0,
    );
    
    let lp_token_a_mint_key = Pubkey::new_unique();
    let mut lp_token_a_mint_lamports = 0;
    let mut lp_token_a_mint_data = vec![0; MintAccount::LEN];
    let lp_token_a_mint_owner = spl_token::id(); // Program will initialize
    let lp_token_a_mint = AccountInfo::new(&lp_token_a_mint_key, false, true, &mut lp_token_a_mint_lamports, &mut lp_token_a_mint_data, &lp_token_a_mint_owner, false, 0);

    let lp_token_b_mint_key = Pubkey::new_unique();
    let mut lp_token_b_mint_lamports = 0;
    let mut lp_token_b_mint_data = vec![0; MintAccount::LEN];
    let lp_token_b_mint_owner = spl_token::id();
    let lp_token_b_mint = AccountInfo::new(&lp_token_b_mint_key, false, true, &mut lp_token_b_mint_lamports, &mut lp_token_b_mint_data, &lp_token_b_mint_owner, false, 0);

    // System Accounts
    let system_program_key = solana_program::system_program::id();
    let mut system_program_lamports = 0; 
    let mut system_program_data = vec![];
    let system_program_owner = solana_program::system_program::id();
    let system_program = AccountInfo::new(
        &system_program_key,
        false,
        false,
        &mut system_program_lamports,
        &mut system_program_data,
        &system_program_owner, // Owner is self for programs
        true, // System program is executable
        0,
    );

    let token_program_key = spl_token::id();
    let mut token_program_lamports = 0;
    let mut token_program_data = vec![];
    let token_program_owner = spl_token::id(); // Owner is self for token program
    let token_program = AccountInfo::new(
        &token_program_key,
        false,
        false,
        &mut token_program_lamports,
        &mut token_program_data,
        &token_program_owner,
        true, // Token program is executable
        0,
    );

    let rent_key = solana_program::sysvar::rent::id();
    let rent_default_instance = Rent::default();
    let space = Rent::size_of(); 
    let mut lamports_for_rent_sysvar = rent_default_instance.minimum_balance(space);
    let mut rent_data = bincode::serialize(&rent_default_instance).unwrap();
    let rent_owner_id = solana_program::system_program::id(); 

    let rent_sysvar_account = AccountInfo::new(
        &rent_key, false, false, 
        &mut lamports_for_rent_sysvar, 
        &mut rent_data,    
        &rent_owner_id, 
        false, 0 );
    
    let clock_key = solana_program::sysvar::clock::id();
    let clock_default_instance = Clock::default();
    let space_clock = Clock::size_of();
    let mut lamports_for_clock_sysvar = Rent::default().minimum_balance(space_clock);
    let mut clock_data = bincode::serialize(&clock_default_instance).unwrap();
    let clock_owner_id = solana_program::system_program::id();

    let clock_sysvar_account = AccountInfo::new(
        &clock_key, false, false, 
        &mut lamports_for_clock_sysvar, 
        &mut clock_data, 
        &clock_owner_id, 
        false, 0 );
    
    create_token_mint(&payer, &token_a_mint, payer.key, 9, &token_program, &system_program, &rent_sysvar_account).await.unwrap();
    create_token_mint(&payer, &token_b_mint, payer.key, 9, &token_program, &system_program, &rent_sysvar_account).await.unwrap();

    let ratio_primary_per_base = 1u64;
    let (norm_token_a_mint_key, norm_token_b_mint_key, norm_ratio_a_numerator, norm_ratio_b_denominator, _token_a_is_primary) =
        if token_a_mint_key.to_bytes() < token_b_mint_key.to_bytes() {
            (token_a_mint_key, token_b_mint_key, ratio_primary_per_base, 1u64, true)
        } else {
            (token_b_mint_key, token_a_mint_key, 1u64, ratio_primary_per_base, false)
        };
        
    let (expected_pool_state_key, found_pool_authority_bump_seed) = Pubkey::find_program_address(
        &[fixed_ratio_trading::POOL_STATE_SEED_PREFIX, norm_token_a_mint_key.as_ref(), norm_token_b_mint_key.as_ref(), &norm_ratio_a_numerator.to_le_bytes(), &norm_ratio_b_denominator.to_le_bytes()],
        &program_id,
    );
    let mut pool_state_lamports = 0;
    let pool_state_serialized_len = PoolState::default().try_to_vec().unwrap().len();
    let mut pool_state_data_vec = vec![0; pool_state_serialized_len];
    let pool_state_account = AccountInfo::new(&expected_pool_state_key, false, true, &mut pool_state_lamports, &mut pool_state_data_vec, &program_id, false, 0);
    
    let (expected_token_a_vault_key, found_token_a_vault_bump_seed) = Pubkey::find_program_address(&[fixed_ratio_trading::TOKEN_A_VAULT_SEED_PREFIX, expected_pool_state_key.as_ref()], &program_id);
    let mut token_a_vault_lamports = 0;
    let mut token_a_vault_data_vec = vec![0; TokenAccount::LEN];
    let spl_token_id_a_vault = spl_token::id();
    let token_a_vault_account = AccountInfo::new(&expected_token_a_vault_key, false, true, &mut token_a_vault_lamports, &mut token_a_vault_data_vec, &spl_token_id_a_vault, false, 0);

    let (expected_token_b_vault_key, found_token_b_vault_bump_seed) = Pubkey::find_program_address(&[fixed_ratio_trading::TOKEN_B_VAULT_SEED_PREFIX, expected_pool_state_key.as_ref()], &program_id);
    let mut token_b_vault_lamports = 0;
    let mut token_b_vault_data_vec = vec![0; TokenAccount::LEN];
    let spl_token_id_b_vault = spl_token::id();
    let token_b_vault_account = AccountInfo::new(&expected_token_b_vault_key, false, true, &mut token_b_vault_lamports, &mut token_b_vault_data_vec, &spl_token_id_b_vault, false, 0);

    let (final_primary_vault_bump, final_base_vault_bump) = if _token_a_is_primary { (found_token_a_vault_bump_seed, found_token_b_vault_bump_seed) } else { (found_token_b_vault_bump_seed, found_token_a_vault_bump_seed) };

    let instruction = FixedRatioInstruction::InitializePool {
        ratio_primary_per_base, pool_authority_bump_seed: found_pool_authority_bump_seed, primary_token_vault_bump_seed: final_primary_vault_bump, base_token_vault_bump_seed: final_base_vault_bump,
    };
    let instruction_data = instruction.try_to_vec().unwrap();
    
    let accounts = &[ payer.clone(), pool_state_account.clone(), token_a_mint.clone(), token_b_mint.clone(), lp_token_a_mint.clone(), lp_token_b_mint.clone(), token_a_vault_account.clone(), token_b_vault_account.clone(), system_program.clone(), token_program.clone(), rent_sysvar_account.clone(), clock_sysvar_account.clone()];
    fixed_ratio_trading::process_instruction(&program_id, accounts, &instruction_data).unwrap();

    let pool_state_data_loaded = PoolState::try_from_slice(&pool_state_account.data.borrow()).unwrap();
    assert!(pool_state_data_loaded.is_initialized);
    assert_eq!(pool_state_data_loaded.owner, *payer.key);
    assert_eq!(pool_state_data_loaded.token_a_mint, norm_token_a_mint_key);
    assert_eq!(pool_state_data_loaded.token_b_mint, norm_token_b_mint_key);
    assert_eq!(pool_state_data_loaded.token_a_vault, expected_token_a_vault_key);
    assert_eq!(pool_state_data_loaded.token_b_vault, expected_token_b_vault_key);
    assert_eq!(pool_state_data_loaded.lp_token_a_mint, *lp_token_a_mint.key);
    assert_eq!(pool_state_data_loaded.lp_token_b_mint, *lp_token_b_mint.key);
    assert_eq!(pool_state_data_loaded.ratio_a_numerator, norm_ratio_a_numerator);
    assert_eq!(pool_state_data_loaded.ratio_b_denominator, norm_ratio_b_denominator);
    assert_eq!(pool_state_data_loaded.pool_authority_bump_seed, found_pool_authority_bump_seed);
    assert_eq!(pool_state_data_loaded.token_a_vault_bump_seed, found_token_a_vault_bump_seed);
    assert_eq!(pool_state_data_loaded.token_b_vault_bump_seed, found_token_b_vault_bump_seed);
}

#[tokio::test]
async fn test_deposit() {
    let program_id_val = Pubkey::new_unique();
    let program_id = program_id_val;
    let payer_key = Pubkey::new_unique();
    let mut payer_lamports = 100_000_000_000; // Increased for safety
    let payer_owner_id = solana_program::system_program::id();
    let mut payer_data = vec![]; // Payer data is usually empty
    let payer = AccountInfo::new(
        &payer_key,
        true, // Signer
        true, // Writable
        &mut payer_lamports,
        &mut payer_data,
        &payer_owner_id,
        false,
        0
    );

    let token_a_mint_key = Pubkey::new_unique();
    let mut token_a_mint_lamports = 0;
    let mut token_a_mint_data = vec![0; MintAccount::LEN];
    let token_a_mint_owner = spl_token::id();
    let token_a_mint = AccountInfo::new(&token_a_mint_key, false, true, &mut token_a_mint_lamports, &mut token_a_mint_data, &token_a_mint_owner, false, 0);

    let token_b_mint_key = Pubkey::new_unique();
    let mut token_b_mint_lamports = 0;
    let mut token_b_mint_data = vec![0; MintAccount::LEN];
    let token_b_mint_owner = spl_token::id();
    let token_b_mint = AccountInfo::new(&token_b_mint_key, false, true, &mut token_b_mint_lamports, &mut token_b_mint_data, &token_b_mint_owner, false, 0);
    
    let lp_token_a_mint_key = Pubkey::new_unique();
    let mut lp_token_a_mint_lamports = 0;
    let mut lp_token_a_mint_data = vec![0; MintAccount::LEN];
    let lp_token_a_mint_owner = spl_token::id();
    let lp_token_a_mint = AccountInfo::new(&lp_token_a_mint_key, false, true, &mut lp_token_a_mint_lamports, &mut lp_token_a_mint_data, &lp_token_a_mint_owner, false, 0);

    let lp_token_b_mint_key = Pubkey::new_unique();
    let mut lp_token_b_mint_lamports = 0;
    let mut lp_token_b_mint_data = vec![0; MintAccount::LEN];
    let lp_token_b_mint_owner = spl_token::id();
    let lp_token_b_mint = AccountInfo::new(&lp_token_b_mint_key, false, true, &mut lp_token_b_mint_lamports, &mut lp_token_b_mint_data, &lp_token_b_mint_owner, false, 0);

    let system_program_key = solana_program::system_program::id();
    let mut system_program_lamports = 0;
    let mut system_program_data = vec![];
    let system_program = AccountInfo::new(&system_program_key, false, false, &mut system_program_lamports, &mut system_program_data, &system_program_key, true, 0);

    let token_program_key = spl_token::id();
    let mut token_program_lamports = 0;
    let mut token_program_data = vec![];
    let token_program = AccountInfo::new(&token_program_key, false, false, &mut token_program_lamports, &mut token_program_data, &token_program_key, true, 0);

    let rent_key = solana_program::sysvar::rent::id();
    let rent_default_instance_deposit = Rent::default();
    let space_rent_deposit = Rent::size_of();
    let mut lamports_for_rent_sysvar_deposit = rent_default_instance_deposit.minimum_balance(space_rent_deposit);
    let mut rent_data_deposit = bincode::serialize(&rent_default_instance_deposit).unwrap();
    let rent_owner_id_deposit = solana_program::system_program::id();

    let rent_sysvar_account = AccountInfo::new(
        &rent_key, false, false, 
        &mut lamports_for_rent_sysvar_deposit, 
        &mut rent_data_deposit, 
        &rent_owner_id_deposit, 
        false, 0 );
    
    let clock_key = solana_program::sysvar::clock::id();
    let clock_default_instance_deposit = Clock::default();
    let space_clock_deposit = Clock::size_of();
    let mut lamports_for_clock_sysvar_deposit = Rent::default().minimum_balance(space_clock_deposit);
    let mut clock_data_deposit = bincode::serialize(&clock_default_instance_deposit).unwrap();
    let clock_owner_id_deposit = solana_program::system_program::id();

    let clock_sysvar_account = AccountInfo::new(
        &clock_key, false, false, 
        &mut lamports_for_clock_sysvar_deposit, 
        &mut clock_data_deposit, 
        &clock_owner_id_deposit, 
        false, 0 );
    
    // Initialize mints (using the AccountInfo created above)
    create_token_mint(&payer, &token_a_mint, payer.key, 9, &token_program, &system_program, &rent_sysvar_account).await.unwrap();
    create_token_mint(&payer, &token_b_mint, payer.key, 9, &token_program, &system_program, &rent_sysvar_account).await.unwrap();

    let ratio_primary_per_base = 1u64;
    let (norm_token_a_mint_key, norm_token_b_mint_key, norm_ratio_a_numerator, norm_ratio_b_denominator, _token_a_is_primary) =
        if token_a_mint_key.to_bytes() < token_b_mint_key.to_bytes() {
            (token_a_mint_key, token_b_mint_key, ratio_primary_per_base, 1u64, true)
        } else {
            (token_b_mint_key, token_a_mint_key, 1u64, ratio_primary_per_base, false)
        };
        
    let (expected_pool_state_key, found_pool_authority_bump_seed) = Pubkey::find_program_address(
        &[fixed_ratio_trading::POOL_STATE_SEED_PREFIX, norm_token_a_mint_key.as_ref(), norm_token_b_mint_key.as_ref(), &norm_ratio_a_numerator.to_le_bytes(), &norm_ratio_b_denominator.to_le_bytes()],
        &program_id,
    );
    let mut pool_state_lamports = 0;
    let pool_state_serialized_len = PoolState::default().try_to_vec().unwrap().len();
    let mut pool_state_data = vec![0; pool_state_serialized_len];
    let pool_state_account = AccountInfo::new(&expected_pool_state_key, false, true, &mut pool_state_lamports, &mut pool_state_data, &program_id, false, 0);
    
    let (expected_token_a_vault_key, found_token_a_vault_bump_seed) = Pubkey::find_program_address(&[fixed_ratio_trading::TOKEN_A_VAULT_SEED_PREFIX, expected_pool_state_key.as_ref()], &program_id);
    let mut token_a_vault_lamports = 0;
    let mut token_a_vault_data = vec![0; TokenAccount::LEN];
    let spl_token_id_a_vault_deposit = spl_token::id();
    let token_a_vault_account = AccountInfo::new(&expected_token_a_vault_key, false, true, &mut token_a_vault_lamports, &mut token_a_vault_data, &spl_token_id_a_vault_deposit, false, 0);

    let (expected_token_b_vault_key, found_token_b_vault_bump_seed) = Pubkey::find_program_address(&[fixed_ratio_trading::TOKEN_B_VAULT_SEED_PREFIX, expected_pool_state_key.as_ref()], &program_id);
    let mut token_b_vault_lamports = 0;
    let mut token_b_vault_data = vec![0; TokenAccount::LEN];
    let spl_token_id_b_vault_deposit = spl_token::id();
    let token_b_vault_account = AccountInfo::new(&expected_token_b_vault_key, false, true, &mut token_b_vault_lamports, &mut token_b_vault_data, &spl_token_id_b_vault_deposit, false, 0);

    let (final_primary_vault_bump, final_base_vault_bump) = if _token_a_is_primary { (found_token_a_vault_bump_seed, found_token_b_vault_bump_seed) } else { (found_token_b_vault_bump_seed, found_token_a_vault_bump_seed) };

    let init_instruction = FixedRatioInstruction::InitializePool {
        ratio_primary_per_base, pool_authority_bump_seed: found_pool_authority_bump_seed, primary_token_vault_bump_seed: final_primary_vault_bump, base_token_vault_bump_seed: final_base_vault_bump,
    };
    let init_instruction_data = init_instruction.try_to_vec().unwrap();
    let init_accounts = &[ payer.clone(), pool_state_account.clone(), token_a_mint.clone(), token_b_mint.clone(), lp_token_a_mint.clone(), lp_token_b_mint.clone(), token_a_vault_account.clone(), token_b_vault_account.clone(), system_program.clone(), token_program.clone(), rent_sysvar_account.clone(), clock_sysvar_account.clone()];
    fixed_ratio_trading::process_instruction(&program_id, init_accounts, &init_instruction_data).unwrap();

    let user_signer_key = payer_key;

    let user_source_token_a_key = Pubkey::new_unique();
    let mut user_source_token_a_lamports = 0;
    let mut user_source_token_a_data = vec![0; TokenAccount::LEN];
    let user_source_token_account_owner = spl_token::id();
    let user_source_token_account = AccountInfo::new(&user_source_token_a_key, false, true, &mut user_source_token_a_lamports, &mut user_source_token_a_data, &user_source_token_account_owner, false, 0);
    create_token_account(&payer, &user_source_token_account, &token_a_mint, &user_signer_key, &token_program, &system_program, &rent_sysvar_account).await.unwrap();

    let user_dest_lp_a_key = Pubkey::new_unique();
    let mut user_dest_lp_a_lamports = 0;
    let mut user_dest_lp_a_data = vec![0; TokenAccount::LEN];
    let user_dest_lp_a_owner = spl_token::id();
    let user_destination_lp_token_account = AccountInfo::new(&user_dest_lp_a_key, false, true, &mut user_dest_lp_a_lamports, &mut user_dest_lp_a_data, &user_dest_lp_a_owner, false, 0);
    create_token_account(&payer, &user_destination_lp_token_account, &lp_token_a_mint, &user_signer_key, &token_program, &system_program, &rent_sysvar_account).await.unwrap();

    let deposit_amount = 1000u64;
    mint_tokens(&token_a_mint, &user_source_token_account, &payer, deposit_amount, &token_program).await.unwrap();

    let deposit_instruction = FixedRatioInstruction::Deposit {
        deposit_token_mint: token_a_mint_key,
        amount: deposit_amount,
    };
    let deposit_instruction_data = deposit_instruction.try_to_vec().unwrap();

    let deposit_accounts = &[
        payer.clone(),
        user_source_token_account.clone(),
        pool_state_account.clone(),
        token_a_mint.clone(), 
        token_b_mint.clone(), 
        token_a_vault_account.clone(),
        token_b_vault_account.clone(),
        lp_token_a_mint.clone(),
        lp_token_b_mint.clone(),
        user_destination_lp_token_account.clone(),
        system_program.clone(),
        token_program.clone(),
        rent_sysvar_account.clone(),
        clock_sysvar_account.clone(),
    ];

    fixed_ratio_trading::process_instruction(&program_id, deposit_accounts, &deposit_instruction_data).unwrap();

    let pool_state_data_after_deposit = PoolState::try_from_slice(&pool_state_account.data.borrow()).unwrap();
    assert_eq!(pool_state_data_after_deposit.total_token_a_liquidity, deposit_amount);
    assert_eq!(pool_state_data_after_deposit.total_token_b_liquidity, 0);

    let user_lp_token_data = TokenAccount::unpack_from_slice(&user_destination_lp_token_account.data.borrow()).unwrap();
    assert_eq!(user_lp_token_data.amount, deposit_amount);
} 