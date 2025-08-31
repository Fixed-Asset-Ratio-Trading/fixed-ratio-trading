//! Pool Fee Update Tests
//! 
//! Tests for the UpdatePoolFees instruction functionality

use {
    fixed_ratio_trading::{
        constants::*,
        types::instructions::PoolInstruction,
        state::{
            pool_state::PoolState,
            system_state::SystemState,
        },
    },
    solana_program::{
        pubkey::Pubkey,
        account_info::AccountInfo,
        entrypoint::ProgramResult,
    },
    solana_program_test::*,
    solana_sdk::{
        instruction::{AccountMeta, Instruction, InstructionError},
        signature::{Keypair, Signer},
        transaction::{Transaction, TransactionError},
        account::Account,
        system_instruction,
    },
    borsh::{BorshSerialize, BorshDeserialize},
};

// Simple adapter function to bridge lifetime signature differences for tests
// The test framework expects independent lifetimes, but our secure function requires linked lifetimes
// This is safe in tests because accounts remain valid for the duration of the function call
fn test_adapter(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // SAFETY: In test environments, account references remain valid for the function duration
    // The lifetime cast is safe because we're not storing references beyond this call
    unsafe {
        let accounts_with_lifetime: &[AccountInfo] = std::mem::transmute(accounts);
        fixed_ratio_trading::process_instruction(program_id, accounts_with_lifetime, instruction_data)
    }
}

mod common;



type TestResult = Result<(), Box<dyn std::error::Error>>;



/// Helper function to create a fee update instruction
fn create_fee_update_instruction(
    pool_state_pda: Pubkey,
    authority: &Keypair,
    update_flags: u8,
    new_liquidity_fee: u64,
    new_swap_fee: u64,
) -> Result<Instruction, Box<dyn std::error::Error>> {
    let program_id = fixed_ratio_trading::id();
    
    // Derive the system state PDA
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &program_id
    );
    
    // Derive the correct program data account
    let (program_data_account, _bump) = Pubkey::find_program_address(
        &[program_id.as_ref()],
        &solana_program::bpf_loader_upgradeable::id()
    );

    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(authority.pubkey(), true), // Program authority signer
            AccountMeta::new_readonly(system_state_pda, false), // System state PDA
            AccountMeta::new(pool_state_pda, false), // Pool state PDA (writable)
            AccountMeta::new_readonly(program_data_account, false), // Program data account
        ],
        data: PoolInstruction::UpdatePoolFees {
            update_flags,
            new_liquidity_fee,
            new_swap_fee,
        }
        .try_to_vec()?,
    })
}






/// Test successful fee update for liquidity fee only
#[tokio::test]
async fn test_update_liquidity_fee_only() -> TestResult {
    // Use minimal setup approach like the working tests
    let program_id = fixed_ratio_trading::id();
    let (program_data_account, _bump) = Pubkey::find_program_address(
        &[program_id.as_ref()],
        &solana_program::bpf_loader_upgradeable::id()
    );
    
    let mut program_test = ProgramTest::new(
        "fixed-ratio-trading",
        program_id,
        processor!(test_adapter),
    );
    
    // Create the upgrade authority keypair for testing
    let upgrade_authority = Keypair::new();
    
    // Create the program data account data
    let account_type: u32 = 3; // ProgramData type
    let has_upgrade_authority: u8 = 1; // true
    let slot: u64 = 0;
    
    let mut account_data = Vec::new();
    account_data.extend_from_slice(&account_type.to_le_bytes());
    account_data.push(has_upgrade_authority);
    account_data.extend_from_slice(upgrade_authority.pubkey().as_ref());
    account_data.extend_from_slice(&slot.to_le_bytes());
    account_data.extend_from_slice(&[0u8; 100]);
    
    // Add the program data account to the test environment
    program_test.add_account(
        program_data_account,
        Account {
            lamports: 1_000_000_000,
            data: account_data,
            owner: solana_program::bpf_loader_upgradeable::id(),
            executable: false,
            rent_epoch: 0,
        },
    );
    
    // Create a mock pool state account for testing with proper PDA derivation
    let token_a_mint = Pubkey::new_unique();
    let token_b_mint = Pubkey::new_unique(); 
    
    // Derive the pool state PDA correctly
    let pool_state_pda = {
        let seeds = &[
            b"pool_state",
            token_a_mint.as_ref(),
            token_b_mint.as_ref(),
            &[1u64.to_le_bytes(), 1u64.to_le_bytes()].concat(), // ratio_a:ratio_b = 1:1
        ];
        Pubkey::find_program_address(seeds, &program_id).0
    };
    
    let mut initial_pool_state = PoolState::default();
    initial_pool_state.token_a_mint = token_a_mint;
    initial_pool_state.token_b_mint = token_b_mint;
    initial_pool_state.ratio_a_numerator = 1;
    initial_pool_state.ratio_b_denominator = 1;
    initial_pool_state.contract_liquidity_fee = DEPOSIT_WITHDRAWAL_FEE;
    initial_pool_state.swap_contract_fee = SWAP_CONTRACT_FEE;
    
    // Create a proper system state account
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &program_id
    );
    
    let system_state = SystemState::new(upgrade_authority.pubkey()); // Creates unpaused state with upgrade authority as admin
    
    program_test.add_account(
        system_state_pda,
        Account {
            lamports: 1_000_000,
            data: system_state.try_to_vec().unwrap(),
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );
    
    program_test.add_account(
        pool_state_pda,
        Account {
            lamports: 10_000_000,
            data: initial_pool_state.try_to_vec().unwrap(),
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );
    
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Fund the upgrade authority
    let fund_upgrade_authority_ix = system_instruction::transfer(
        &payer.pubkey(),
        &upgrade_authority.pubkey(),
        1_000_000_000,
    );
    
    let fund_upgrade_authority_tx = Transaction::new_signed_with_payer(
        &[fund_upgrade_authority_ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    
    banks_client.process_transaction(fund_upgrade_authority_tx).await.map_err(|e| format!("Failed to fund upgrade authority: {:?}", e))?;
    
    // Get initial pool state
    let initial_liquidity_fee = initial_pool_state.contract_liquidity_fee;
    let initial_swap_fee = initial_pool_state.swap_contract_fee;
    
    // Define new liquidity fee (increase by 50%)
    let new_liquidity_fee = initial_liquidity_fee + (initial_liquidity_fee / 2);
    let new_swap_fee = initial_swap_fee; // Keep swap fee unchanged
    
    // Create fee update instruction using the upgrade authority
    let update_instruction = create_fee_update_instruction(
        pool_state_pda,
        &upgrade_authority, // Use the proper upgrade authority
        FEE_UPDATE_FLAG_LIQUIDITY,
        new_liquidity_fee,
        new_swap_fee,
    ).map_err(|e| format!("Failed to create instruction: {:?}", e))?;
    
    // Execute the transaction
    let transaction = Transaction::new_signed_with_payer(
        &[update_instruction],
        Some(&upgrade_authority.pubkey()),
        &[&upgrade_authority],
        recent_blockhash,
    );
    
    banks_client.process_transaction(transaction).await.map_err(|e| format!("Failed to process transaction: {:?}", e))?;
    
    // Verify the fee was updated
    let pool_account = banks_client.get_account(pool_state_pda).await
        .map_err(|e| format!("Failed to get account: {:?}", e))?
        .ok_or("Pool state account not found")?;
    let updated_pool_state = PoolState::try_from_slice(&pool_account.data)
        .map_err(|e| format!("Failed to deserialize pool state: {:?}", e))?;
    
    assert_eq!(updated_pool_state.contract_liquidity_fee, new_liquidity_fee, "Liquidity fee should be updated");
    assert_eq!(updated_pool_state.swap_contract_fee, initial_swap_fee, "Swap fee should remain unchanged");
    
    println!("✅ Liquidity fee successfully updated from {} to {}", initial_liquidity_fee, new_liquidity_fee);
    Ok(())
}

/// Test successful fee update for swap fee only
#[tokio::test]
async fn test_update_swap_fee_only() -> TestResult {
    // Use minimal setup approach like the working tests
    let program_id = fixed_ratio_trading::id();
    let (program_data_account, _bump) = Pubkey::find_program_address(
        &[program_id.as_ref()],
        &solana_program::bpf_loader_upgradeable::id()
    );
    
    let mut program_test = ProgramTest::new(
        "fixed-ratio-trading",
        program_id,
        processor!(test_adapter),
    );
    
    // Create the upgrade authority keypair for testing
    let upgrade_authority = Keypair::new();
    
    // Create the program data account data
    let account_type: u32 = 3; // ProgramData type
    let has_upgrade_authority: u8 = 1; // true
    let slot: u64 = 0;
    
    let mut account_data = Vec::new();
    account_data.extend_from_slice(&account_type.to_le_bytes());
    account_data.push(has_upgrade_authority);
    account_data.extend_from_slice(upgrade_authority.pubkey().as_ref());
    account_data.extend_from_slice(&slot.to_le_bytes());
    account_data.extend_from_slice(&[0u8; 100]);
    
    // Add the program data account to the test environment
    program_test.add_account(
        program_data_account,
        Account {
            lamports: 1_000_000_000,
            data: account_data,
            owner: solana_program::bpf_loader_upgradeable::id(),
            executable: false,
            rent_epoch: 0,
        },
    );
    
    // Create a mock pool state account for testing with proper PDA derivation
    let token_a_mint = Pubkey::new_unique();
    let token_b_mint = Pubkey::new_unique(); 
    
    // Derive the pool state PDA correctly
    let pool_state_pda = {
        let seeds = &[
            b"pool_state",
            token_a_mint.as_ref(),
            token_b_mint.as_ref(),
            &[1u64.to_le_bytes(), 1u64.to_le_bytes()].concat(), // ratio_a:ratio_b = 1:1
        ];
        Pubkey::find_program_address(seeds, &program_id).0
    };
    
    let mut initial_pool_state = PoolState::default();
    initial_pool_state.token_a_mint = token_a_mint;
    initial_pool_state.token_b_mint = token_b_mint;
    initial_pool_state.ratio_a_numerator = 1;
    initial_pool_state.ratio_b_denominator = 1;
    initial_pool_state.contract_liquidity_fee = DEPOSIT_WITHDRAWAL_FEE;
    initial_pool_state.swap_contract_fee = SWAP_CONTRACT_FEE;
    
    // Create a proper system state account
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &program_id
    );
    
    let system_state = SystemState::new(upgrade_authority.pubkey()); // Creates unpaused state with upgrade authority as admin
    
    program_test.add_account(
        system_state_pda,
        Account {
            lamports: 1_000_000,
            data: system_state.try_to_vec().unwrap(),
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );
    
    program_test.add_account(
        pool_state_pda,
        Account {
            lamports: 10_000_000,
            data: initial_pool_state.try_to_vec().unwrap(),
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );
    
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Fund the upgrade authority
    let fund_upgrade_authority_ix = system_instruction::transfer(
        &payer.pubkey(),
        &upgrade_authority.pubkey(),
        1_000_000_000,
    );
    
    let fund_upgrade_authority_tx = Transaction::new_signed_with_payer(
        &[fund_upgrade_authority_ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    
    banks_client.process_transaction(fund_upgrade_authority_tx).await.map_err(|e| format!("Failed to fund upgrade authority: {:?}", e))?;
    
    // Get initial pool state
    let initial_liquidity_fee = initial_pool_state.contract_liquidity_fee;
    let initial_swap_fee = initial_pool_state.swap_contract_fee;
    
    // Define new swap fee (double it)
    let new_liquidity_fee = initial_liquidity_fee; // Keep liquidity fee unchanged
    let new_swap_fee = initial_swap_fee * 2;
    
    // Create fee update instruction using the upgrade authority
    let update_instruction = create_fee_update_instruction(
        pool_state_pda,
        &upgrade_authority,
        FEE_UPDATE_FLAG_SWAP,
        new_liquidity_fee,
        new_swap_fee,
    ).map_err(|e| format!("Failed to create instruction: {:?}", e))?;
    
    // Execute the transaction
    let transaction = Transaction::new_signed_with_payer(
        &[update_instruction],
        Some(&upgrade_authority.pubkey()),
        &[&upgrade_authority],
        recent_blockhash,
    );
    
    banks_client.process_transaction(transaction).await.map_err(|e| format!("Failed to process transaction: {:?}", e))?;
    
    // Verify the fee was updated
    let pool_account = banks_client.get_account(pool_state_pda).await
        .map_err(|e| format!("Failed to get account: {:?}", e))?
        .ok_or("Pool state account not found")?;
    let updated_pool_state = PoolState::try_from_slice(&pool_account.data)
        .map_err(|e| format!("Failed to deserialize pool state: {:?}", e))?;
    
    assert_eq!(updated_pool_state.contract_liquidity_fee, initial_liquidity_fee, "Liquidity fee should remain unchanged");
    assert_eq!(updated_pool_state.swap_contract_fee, new_swap_fee, "Swap fee should be updated");
    
    println!("✅ Swap fee successfully updated from {} to {}", initial_swap_fee, new_swap_fee);
    Ok(())
}

/// Test successful fee update for both fees (using minimal setup)
#[tokio::test]
async fn test_update_both_fees() -> TestResult {
    // Use minimal setup approach like the working tests
    let program_id = fixed_ratio_trading::id();
    let (program_data_account, _bump) = Pubkey::find_program_address(
        &[program_id.as_ref()],
        &solana_program::bpf_loader_upgradeable::id()
    );
    
    let mut program_test = ProgramTest::new(
        "fixed-ratio-trading",
        program_id,
        processor!(test_adapter),
    );
    
    // Create the upgrade authority keypair for testing
    let upgrade_authority = Keypair::new();
    
    // Create the program data account data
    let account_type: u32 = 3; // ProgramData type
    let has_upgrade_authority: u8 = 1; // true
    let slot: u64 = 0;
    
    let mut account_data = Vec::new();
    account_data.extend_from_slice(&account_type.to_le_bytes());
    account_data.push(has_upgrade_authority);
    account_data.extend_from_slice(upgrade_authority.pubkey().as_ref());
    account_data.extend_from_slice(&slot.to_le_bytes());
    account_data.extend_from_slice(&[0u8; 100]);
    
    // Add the program data account to the test environment
    program_test.add_account(
        program_data_account,
        Account {
            lamports: 1_000_000_000,
            data: account_data,
            owner: solana_program::bpf_loader_upgradeable::id(),
            executable: false,
            rent_epoch: 0,
        },
    );
    
    // Create a mock pool state account for testing with proper PDA derivation
    let token_a_mint = Pubkey::new_unique();
    let token_b_mint = Pubkey::new_unique(); 
    
    // Derive the pool state PDA correctly
    let pool_state_pda = {
        let seeds = &[
            b"pool_state",
            token_a_mint.as_ref(),
            token_b_mint.as_ref(),
            &[1u64.to_le_bytes(), 1u64.to_le_bytes()].concat(), // ratio_a:ratio_b = 1:1
        ];
        Pubkey::find_program_address(seeds, &program_id).0
    };
    
    let mut initial_pool_state = PoolState::default();
    initial_pool_state.token_a_mint = token_a_mint;
    initial_pool_state.token_b_mint = token_b_mint;
    initial_pool_state.ratio_a_numerator = 1;
    initial_pool_state.ratio_b_denominator = 1;
    initial_pool_state.contract_liquidity_fee = DEPOSIT_WITHDRAWAL_FEE;
    initial_pool_state.swap_contract_fee = SWAP_CONTRACT_FEE;
    
    // Create a proper system state account
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &program_id
    );
    
    let system_state = SystemState::new(upgrade_authority.pubkey()); // Creates unpaused state with upgrade authority as admin
    
    program_test.add_account(
        system_state_pda,
        Account {
            lamports: 1_000_000,
            data: system_state.try_to_vec().unwrap(),
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );
    
    program_test.add_account(
        pool_state_pda,
        Account {
            lamports: 10_000_000,
            data: initial_pool_state.try_to_vec().unwrap(),
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );
    
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Fund the upgrade authority
    let fund_upgrade_authority_ix = system_instruction::transfer(
        &payer.pubkey(),
        &upgrade_authority.pubkey(),
        1_000_000_000,
    );
    
    let fund_upgrade_authority_tx = Transaction::new_signed_with_payer(
        &[fund_upgrade_authority_ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    
    banks_client.process_transaction(fund_upgrade_authority_tx).await.map_err(|e| format!("Failed to fund upgrade authority: {:?}", e))?;
    
    // Get initial pool state
    let initial_liquidity_fee = initial_pool_state.contract_liquidity_fee;
    let initial_swap_fee = initial_pool_state.swap_contract_fee;
    
    // Define new fees
    let new_liquidity_fee = initial_liquidity_fee + 100_000; // Add 0.1 SOL
    let new_swap_fee = initial_swap_fee + 10_000; // Add 0.01 SOL
    
    // Create fee update instruction using the upgrade authority
    let update_instruction = create_fee_update_instruction(
        pool_state_pda,
        &upgrade_authority,
        FEE_UPDATE_FLAG_BOTH,
        new_liquidity_fee,
        new_swap_fee,
    ).map_err(|e| format!("Failed to create instruction: {:?}", e))?;
    
    // Execute the transaction
    let transaction = Transaction::new_signed_with_payer(
        &[update_instruction],
        Some(&upgrade_authority.pubkey()),
        &[&upgrade_authority],
        recent_blockhash,
    );
    
    banks_client.process_transaction(transaction).await.map_err(|e| format!("Failed to process transaction: {:?}", e))?;
    
    // Verify both fees were updated
    let pool_account = banks_client.get_account(pool_state_pda).await
        .map_err(|e| format!("Failed to get account: {:?}", e))?
        .ok_or("Pool state account not found")?;
    let updated_pool_state = PoolState::try_from_slice(&pool_account.data)
        .map_err(|e| format!("Failed to deserialize pool state: {:?}", e))?;
    
    assert_eq!(updated_pool_state.contract_liquidity_fee, new_liquidity_fee, "Liquidity fee should be updated");
    assert_eq!(updated_pool_state.swap_contract_fee, new_swap_fee, "Swap fee should be updated");
    
    println!("✅ Both fees successfully updated");
    println!("   Liquidity fee: {} -> {}", initial_liquidity_fee, new_liquidity_fee);
    println!("   Swap fee: {} -> {}", initial_swap_fee, new_swap_fee);
    Ok(())
}

/// Test that unauthorized users cannot update fees
#[tokio::test]
async fn test_unauthorized_fee_update() -> TestResult {
    // Use minimal setup approach like the working tests
    let program_id = fixed_ratio_trading::id();
    let (program_data_account, _bump) = Pubkey::find_program_address(
        &[program_id.as_ref()],
        &solana_program::bpf_loader_upgradeable::id()
    );
    
    let mut program_test = ProgramTest::new(
        "fixed-ratio-trading",
        program_id,
        processor!(test_adapter),
    );
    
    // Create the upgrade authority keypair for testing
    let upgrade_authority = Keypair::new();
    
    // Create the program data account data
    let account_type: u32 = 3; // ProgramData type
    let has_upgrade_authority: u8 = 1; // true
    let slot: u64 = 0;
    
    let mut account_data = Vec::new();
    account_data.extend_from_slice(&account_type.to_le_bytes());
    account_data.push(has_upgrade_authority);
    account_data.extend_from_slice(upgrade_authority.pubkey().as_ref());
    account_data.extend_from_slice(&slot.to_le_bytes());
    account_data.extend_from_slice(&[0u8; 100]);
    
    // Add the program data account to the test environment
    program_test.add_account(
        program_data_account,
        Account {
            lamports: 1_000_000_000,
            data: account_data,
            owner: solana_program::bpf_loader_upgradeable::id(),
            executable: false,
            rent_epoch: 0,
        },
    );
    
    // Create a mock pool state account for testing with proper PDA derivation
    let token_a_mint = Pubkey::new_unique();
    let token_b_mint = Pubkey::new_unique(); 
    
    // Derive the pool state PDA correctly
    let pool_state_pda = {
        let seeds = &[
            b"pool_state",
            token_a_mint.as_ref(),
            token_b_mint.as_ref(),
            &[1u64.to_le_bytes(), 1u64.to_le_bytes()].concat(), // ratio_a:ratio_b = 1:1
        ];
        Pubkey::find_program_address(seeds, &program_id).0
    };
    
    let mut initial_pool_state = PoolState::default();
    initial_pool_state.token_a_mint = token_a_mint;
    initial_pool_state.token_b_mint = token_b_mint;
    initial_pool_state.ratio_a_numerator = 1;
    initial_pool_state.ratio_b_denominator = 1;
    initial_pool_state.contract_liquidity_fee = DEPOSIT_WITHDRAWAL_FEE;
    initial_pool_state.swap_contract_fee = SWAP_CONTRACT_FEE;
    
    // Create a proper system state account
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &program_id
    );
    
    let system_state = SystemState::new(upgrade_authority.pubkey()); // Creates unpaused state with upgrade authority as admin
    
    program_test.add_account(
        system_state_pda,
        Account {
            lamports: 1_000_000,
            data: system_state.try_to_vec().unwrap(),
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );
    
    program_test.add_account(
        pool_state_pda,
        Account {
            lamports: 10_000_000,
            data: initial_pool_state.try_to_vec().unwrap(),
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );
    
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Fund the upgrade authority
    let fund_upgrade_authority_ix = system_instruction::transfer(
        &payer.pubkey(),
        &upgrade_authority.pubkey(),
        1_000_000_000,
    );
    
    let fund_upgrade_authority_tx = Transaction::new_signed_with_payer(
        &[fund_upgrade_authority_ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    
    banks_client.process_transaction(fund_upgrade_authority_tx).await.map_err(|e| format!("Failed to fund upgrade authority: {:?}", e))?;
    
    // Create unauthorized user
    let unauthorized_user = Keypair::new();
    
    // Fund the unauthorized user
    let fund_instruction = system_instruction::transfer(
        &payer.pubkey(),
        &unauthorized_user.pubkey(),
        1_000_000_000, // 1 SOL
    );
    
    let fund_transaction = Transaction::new_signed_with_payer(
        &[fund_instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    
    banks_client.process_transaction(fund_transaction).await.map_err(|e| format!("Failed to fund unauthorized user: {:?}", e))?;
    
    // Try to update fees with unauthorized user (not the upgrade authority)
    let update_instruction = create_fee_update_instruction(
        pool_state_pda,
        &unauthorized_user, // This should fail
        FEE_UPDATE_FLAG_BOTH,
        2_000_000, // 2 SOL
        50_000,    // 0.05 SOL
    ).map_err(|e| format!("Failed to create instruction: {:?}", e))?;
    
    let transaction = Transaction::new_signed_with_payer(
        &[update_instruction],
        Some(&unauthorized_user.pubkey()),
        &[&unauthorized_user],
        recent_blockhash,
    );
    
    let result = banks_client.process_transaction(transaction).await;
    
    // This should fail
    assert!(result.is_err(), "Unauthorized fee update should fail");
    
    println!("✅ Unauthorized fee update properly rejected");
    Ok(())
}

/// Test invalid fee update flags
#[tokio::test]
async fn test_invalid_fee_update_flags() -> TestResult {
    // Use minimal setup approach like the working tests
    let program_id = fixed_ratio_trading::id();
    let (program_data_account, _bump) = Pubkey::find_program_address(
        &[program_id.as_ref()],
        &solana_program::bpf_loader_upgradeable::id()
    );
    
    let mut program_test = ProgramTest::new(
        "fixed-ratio-trading",
        program_id,
        processor!(test_adapter),
    );
    
    // Create the upgrade authority keypair for testing
    let upgrade_authority = Keypair::new();
    
    // Create the program data account data
    let account_type: u32 = 3; // ProgramData type
    let has_upgrade_authority: u8 = 1; // true
    let slot: u64 = 0;
    
    let mut account_data = Vec::new();
    account_data.extend_from_slice(&account_type.to_le_bytes());
    account_data.push(has_upgrade_authority);
    account_data.extend_from_slice(upgrade_authority.pubkey().as_ref());
    account_data.extend_from_slice(&slot.to_le_bytes());
    account_data.extend_from_slice(&[0u8; 100]);
    
    // Add the program data account to the test environment
    program_test.add_account(
        program_data_account,
        Account {
            lamports: 1_000_000_000,
            data: account_data,
            owner: solana_program::bpf_loader_upgradeable::id(),
            executable: false,
            rent_epoch: 0,
        },
    );
    
    // Create a mock pool state account for testing with proper PDA derivation
    let token_a_mint = Pubkey::new_unique();
    let token_b_mint = Pubkey::new_unique(); 
    
    // Derive the pool state PDA correctly
    let pool_state_pda = {
        let seeds = &[
            b"pool_state",
            token_a_mint.as_ref(),
            token_b_mint.as_ref(),
            &[1u64.to_le_bytes(), 1u64.to_le_bytes()].concat(), // ratio_a:ratio_b = 1:1
        ];
        Pubkey::find_program_address(seeds, &program_id).0
    };
    
    let mut initial_pool_state = PoolState::default();
    initial_pool_state.token_a_mint = token_a_mint;
    initial_pool_state.token_b_mint = token_b_mint;
    initial_pool_state.ratio_a_numerator = 1;
    initial_pool_state.ratio_b_denominator = 1;
    initial_pool_state.contract_liquidity_fee = DEPOSIT_WITHDRAWAL_FEE;
    initial_pool_state.swap_contract_fee = SWAP_CONTRACT_FEE;
    
    // Create a proper system state account
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &program_id
    );
    
    let system_state = SystemState::new(upgrade_authority.pubkey()); // Creates unpaused state with upgrade authority as admin
    
    program_test.add_account(
        system_state_pda,
        Account {
            lamports: 1_000_000,
            data: system_state.try_to_vec().unwrap(),
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );
    
    program_test.add_account(
        pool_state_pda,
        Account {
            lamports: 10_000_000,
            data: initial_pool_state.try_to_vec().unwrap(),
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );
    
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Fund the upgrade authority
    let fund_upgrade_authority_ix = system_instruction::transfer(
        &payer.pubkey(),
        &upgrade_authority.pubkey(),
        1_000_000_000,
    );
    
    let fund_upgrade_authority_tx = Transaction::new_signed_with_payer(
        &[fund_upgrade_authority_ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    
    banks_client.process_transaction(fund_upgrade_authority_tx).await.map_err(|e| format!("Failed to fund upgrade authority: {:?}", e))?;
    
    // Try invalid flag (should be 1, 2, or 3) using the upgrade authority
    let update_instruction = create_fee_update_instruction(
        pool_state_pda,
        &upgrade_authority,
        4, // Invalid flag
        1_000_000,
        50_000,
    ).map_err(|e| format!("Failed to create instruction: {:?}", e))?;
    
    let transaction = Transaction::new_signed_with_payer(
        &[update_instruction],
        Some(&upgrade_authority.pubkey()),
        &[&upgrade_authority],
        recent_blockhash,
    );
    
    let result = banks_client.process_transaction(transaction).await;
    
    // This should fail
    assert!(result.is_err(), "Invalid flag should be rejected");
    
    println!("✅ Invalid fee update flag properly rejected");
    Ok(())
} 

/// Test successful fee update with minimal setup (bypasses treasury issues)
#[tokio::test]
async fn test_update_fees_minimal() {
    // Create a minimal test environment without complex treasury setup
    let program_id = fixed_ratio_trading::id();
    let (program_data_account, _bump) = Pubkey::find_program_address(
        &[program_id.as_ref()],
        &solana_program::bpf_loader_upgradeable::id()
    );
    
    let mut program_test = ProgramTest::new(
        "fixed-ratio-trading",
        program_id,
        processor!(test_adapter),
    );
    
    // Create the upgrade authority keypair for testing
    let upgrade_authority = Keypair::new();
    
    // Create the program data account data
    let account_type: u32 = 3; // ProgramData type
    let has_upgrade_authority: u8 = 1; // true
    let slot: u64 = 0;
    
    let mut account_data = Vec::new();
    account_data.extend_from_slice(&account_type.to_le_bytes());
    account_data.push(has_upgrade_authority);
    account_data.extend_from_slice(upgrade_authority.pubkey().as_ref());
    account_data.extend_from_slice(&slot.to_le_bytes());
    account_data.extend_from_slice(&[0u8; 100]);
    
    // Add the program data account to the test environment
    program_test.add_account(
        program_data_account,
        Account {
            lamports: 1_000_000_000,
            data: account_data,
            owner: solana_program::bpf_loader_upgradeable::id(),
            executable: false,
            rent_epoch: 0,
        },
    );
    
    // Create a mock pool state account for testing with proper PDA derivation
    let token_a_mint = Pubkey::new_unique();
    let token_b_mint = Pubkey::new_unique(); 
    
    // Derive the pool state PDA correctly
    let pool_state_pda = {
        let seeds = &[
            b"pool_state",
            token_a_mint.as_ref(),
            token_b_mint.as_ref(),
            &[1u64.to_le_bytes(), 1u64.to_le_bytes()].concat(), // ratio_a:ratio_b = 1:1
        ];
        Pubkey::find_program_address(seeds, &program_id).0
    };
    
    let mut initial_pool_state = PoolState::default();
    initial_pool_state.token_a_mint = token_a_mint;
    initial_pool_state.token_b_mint = token_b_mint;
    initial_pool_state.ratio_a_numerator = 1;
    initial_pool_state.ratio_b_denominator = 1;
    initial_pool_state.contract_liquidity_fee = DEPOSIT_WITHDRAWAL_FEE;
    initial_pool_state.swap_contract_fee = SWAP_CONTRACT_FEE;
    
    // Create a proper system state account
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &program_id
    );
    
    let system_state = SystemState::new(upgrade_authority.pubkey()); // Creates unpaused state with upgrade authority as admin
    
    program_test.add_account(
        system_state_pda,
        Account {
            lamports: 1_000_000,
            data: system_state.try_to_vec().unwrap(),
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );
    
    program_test.add_account(
        pool_state_pda,
        Account {
            lamports: 10_000_000,
            data: initial_pool_state.try_to_vec().unwrap(),
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );
    
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Fund the upgrade authority
    let fund_ix = system_instruction::transfer(
        &payer.pubkey(),
        &upgrade_authority.pubkey(),
        1_000_000_000,
    );
    
    let fund_tx = Transaction::new_signed_with_payer(
        &[fund_ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    
    banks_client.process_transaction(fund_tx).await.expect("Failed to fund upgrade authority");
    
    // Test the fee update
    let new_liquidity_fee = DEPOSIT_WITHDRAWAL_FEE * 2;
    let new_swap_fee = SWAP_CONTRACT_FEE * 2;
    
    let update_instruction = create_fee_update_instruction(
        pool_state_pda,
        &upgrade_authority,
        FEE_UPDATE_FLAG_BOTH,
        new_liquidity_fee,
        new_swap_fee,
    ).expect("Failed to create instruction");
    
    let transaction = Transaction::new_signed_with_payer(
        &[update_instruction],
        Some(&upgrade_authority.pubkey()),
        &[&upgrade_authority],
        recent_blockhash,
    );
    
    let result = banks_client.process_transaction(transaction).await;
    
    // Check if the transaction succeeded
    match result {
        Ok(_) => {
            // Verify the fees were updated
            let updated_account = banks_client.get_account(pool_state_pda).await
                .expect("Failed to get account")
                .expect("Pool state account not found");
            let updated_pool_state = PoolState::try_from_slice(&updated_account.data)
                .expect("Failed to deserialize pool state");
            
            assert_eq!(updated_pool_state.contract_liquidity_fee, new_liquidity_fee);
            assert_eq!(updated_pool_state.swap_contract_fee, new_swap_fee);
            
            println!("🎉 SUCCESS: Fee update functionality works correctly!");
            println!("   New liquidity fee: {} lamports", new_liquidity_fee);
            println!("   New swap fee: {} lamports", new_swap_fee);
            println!("✅ Program authority validation is working properly");
        },
        Err(e) => {
            println!("❌ Fee update failed with error: {:?}", e);
            println!("   This indicates an issue with the fee update logic or validation");
            panic!("Fee update test failed: {:?}", e);
        }
    }
} 

/// Test that unauthorized users cannot update fees (minimal setup)
#[tokio::test]
async fn test_unauthorized_fee_update_minimal() {
    // Create a minimal test environment without complex treasury setup
    let program_id = fixed_ratio_trading::id();
    let (program_data_account, _bump) = Pubkey::find_program_address(
        &[program_id.as_ref()],
        &solana_program::bpf_loader_upgradeable::id()
    );
    
    let mut program_test = ProgramTest::new(
        "fixed-ratio-trading",
        program_id,
        processor!(test_adapter),
    );
    
    // Create the upgrade authority keypair for testing
    let upgrade_authority = Keypair::new();
    
    // Create the program data account data
    let account_type: u32 = 3; // ProgramData type
    let has_upgrade_authority: u8 = 1; // true
    let slot: u64 = 0;
    
    let mut account_data = Vec::new();
    account_data.extend_from_slice(&account_type.to_le_bytes());
    account_data.push(has_upgrade_authority);
    account_data.extend_from_slice(upgrade_authority.pubkey().as_ref());
    account_data.extend_from_slice(&slot.to_le_bytes());
    account_data.extend_from_slice(&[0u8; 100]);
    
    // Add the program data account to the test environment
    program_test.add_account(
        program_data_account,
        Account {
            lamports: 1_000_000_000,
            data: account_data,
            owner: solana_program::bpf_loader_upgradeable::id(),
            executable: false,
            rent_epoch: 0,
        },
    );
    
    // Create a mock pool state account for testing with proper PDA derivation
    let token_a_mint = Pubkey::new_unique();
    let token_b_mint = Pubkey::new_unique(); 
    
    // Derive the pool state PDA correctly
    let pool_state_pda = {
        let seeds = &[
            b"pool_state",
            token_a_mint.as_ref(),
            token_b_mint.as_ref(),
            &[1u64.to_le_bytes(), 1u64.to_le_bytes()].concat(), // ratio_a:ratio_b = 1:1
        ];
        Pubkey::find_program_address(seeds, &program_id).0
    };
    
    let mut initial_pool_state = PoolState::default();
    initial_pool_state.token_a_mint = token_a_mint;
    initial_pool_state.token_b_mint = token_b_mint;
    initial_pool_state.ratio_a_numerator = 1;
    initial_pool_state.ratio_b_denominator = 1;
    initial_pool_state.contract_liquidity_fee = DEPOSIT_WITHDRAWAL_FEE;
    initial_pool_state.swap_contract_fee = SWAP_CONTRACT_FEE;
    
    // Create a proper system state account
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &program_id
    );
    
    let system_state = SystemState::new(upgrade_authority.pubkey()); // Creates unpaused state with upgrade authority as admin
    
    program_test.add_account(
        system_state_pda,
        Account {
            lamports: 1_000_000,
            data: system_state.try_to_vec().unwrap(),
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );
    
    program_test.add_account(
        pool_state_pda,
        Account {
            lamports: 10_000_000,
            data: initial_pool_state.try_to_vec().unwrap(),
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );
    
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Fund the upgrade authority
    let fund_upgrade_authority_ix = system_instruction::transfer(
        &payer.pubkey(),
        &upgrade_authority.pubkey(),
        1_000_000_000,
    );
    
    let fund_upgrade_authority_tx = Transaction::new_signed_with_payer(
        &[fund_upgrade_authority_ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    
    banks_client.process_transaction(fund_upgrade_authority_tx).await.expect("Failed to fund upgrade authority");
    
    // Create an unauthorized user (not the upgrade authority)
    let unauthorized_user = Keypair::new();
    
    // Fund the unauthorized user
    let fund_unauthorized_ix = system_instruction::transfer(
        &payer.pubkey(),
        &unauthorized_user.pubkey(),
        1_000_000_000,
    );
    
    let fund_unauthorized_tx = Transaction::new_signed_with_payer(
        &[fund_unauthorized_ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    
    banks_client.process_transaction(fund_unauthorized_tx).await.expect("Failed to fund unauthorized user");
    
    // Test the fee update with unauthorized user (should fail)
    let new_liquidity_fee = DEPOSIT_WITHDRAWAL_FEE * 2;
    let new_swap_fee = SWAP_CONTRACT_FEE * 2;
    
    let update_instruction = create_fee_update_instruction(
        pool_state_pda,
        &unauthorized_user, // Using unauthorized user instead of upgrade_authority
        FEE_UPDATE_FLAG_BOTH,
        new_liquidity_fee,
        new_swap_fee,
    ).expect("Failed to create instruction");
    
    let transaction = Transaction::new_signed_with_payer(
        &[update_instruction],
        Some(&unauthorized_user.pubkey()),
        &[&unauthorized_user],
        recent_blockhash,
    );
    
    let result = banks_client.process_transaction(transaction).await;
    
    // Check that the transaction failed (this should fail)
    match result {
        Ok(_) => {
            panic!("🚨 SECURITY BREACH: Unauthorized user was able to update fees!");
        },
        Err(e) => {
            println!("✅ SUCCESS: Unauthorized fee update properly rejected");
            println!("   Error: {:?}", e);
            println!("✅ Program authority validation is working correctly");
            
            // Verify the pool state was not modified
            let pool_account = banks_client.get_account(pool_state_pda).await
                .expect("Failed to get account")
                .expect("Pool state account not found");
            let pool_state = PoolState::try_from_slice(&pool_account.data)
                .expect("Failed to deserialize pool state");
            
            // Fees should remain unchanged
            assert_eq!(pool_state.contract_liquidity_fee, DEPOSIT_WITHDRAWAL_FEE);
            assert_eq!(pool_state.swap_contract_fee, SWAP_CONTRACT_FEE);
            println!("✅ Pool state unchanged - fees remain at original values");
        }
    }
} 

/// Test that invalid fee update flags are rejected (minimal setup)
#[tokio::test]
async fn test_invalid_fee_update_flags_minimal() {
    // Create a minimal test environment without complex treasury setup
    let program_id = fixed_ratio_trading::id();
    let (program_data_account, _bump) = Pubkey::find_program_address(
        &[program_id.as_ref()],
        &solana_program::bpf_loader_upgradeable::id()
    );
    
    let mut program_test = ProgramTest::new(
        "fixed-ratio-trading",
        program_id,
        processor!(test_adapter),
    );
    
    // Create the upgrade authority keypair for testing
    let upgrade_authority = Keypair::new();
    
    // Create the program data account data
    let account_type: u32 = 3; // ProgramData type
    let has_upgrade_authority: u8 = 1; // true
    let slot: u64 = 0;
    
    let mut account_data = Vec::new();
    account_data.extend_from_slice(&account_type.to_le_bytes());
    account_data.push(has_upgrade_authority);
    account_data.extend_from_slice(upgrade_authority.pubkey().as_ref());
    account_data.extend_from_slice(&slot.to_le_bytes());
    account_data.extend_from_slice(&[0u8; 100]);
    
    // Add the program data account to the test environment
    program_test.add_account(
        program_data_account,
        Account {
            lamports: 1_000_000_000,
            data: account_data,
            owner: solana_program::bpf_loader_upgradeable::id(),
            executable: false,
            rent_epoch: 0,
        },
    );
    
    // Create a mock pool state account for testing with proper PDA derivation
    let token_a_mint = Pubkey::new_unique();
    let token_b_mint = Pubkey::new_unique(); 
    
    // Derive the pool state PDA correctly
    let pool_state_pda = {
        let seeds = &[
            b"pool_state",
            token_a_mint.as_ref(),
            token_b_mint.as_ref(),
            &[1u64.to_le_bytes(), 1u64.to_le_bytes()].concat(), // ratio_a:ratio_b = 1:1
        ];
        Pubkey::find_program_address(seeds, &program_id).0
    };
    
    let mut initial_pool_state = PoolState::default();
    initial_pool_state.token_a_mint = token_a_mint;
    initial_pool_state.token_b_mint = token_b_mint;
    initial_pool_state.ratio_a_numerator = 1;
    initial_pool_state.ratio_b_denominator = 1;
    initial_pool_state.contract_liquidity_fee = DEPOSIT_WITHDRAWAL_FEE;
    initial_pool_state.swap_contract_fee = SWAP_CONTRACT_FEE;
    
    // Create a proper system state account
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &program_id
    );
    
    let system_state = SystemState::new(upgrade_authority.pubkey()); // Creates unpaused state with upgrade authority as admin
    
    program_test.add_account(
        system_state_pda,
        Account {
            lamports: 1_000_000,
            data: system_state.try_to_vec().unwrap(),
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );
    
    program_test.add_account(
        pool_state_pda,
        Account {
            lamports: 10_000_000,
            data: initial_pool_state.try_to_vec().unwrap(),
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );
    
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Fund the upgrade authority
    let fund_upgrade_authority_ix = system_instruction::transfer(
        &payer.pubkey(),
        &upgrade_authority.pubkey(),
        1_000_000_000,
    );
    
    let fund_upgrade_authority_tx = Transaction::new_signed_with_payer(
        &[fund_upgrade_authority_ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    
    banks_client.process_transaction(fund_upgrade_authority_tx).await.expect("Failed to fund upgrade authority");
    
    // Test invalid flags: 0 (no flags set) and 4 (invalid flag)
    let invalid_flags = [0u8, 4u8, 5u8, 255u8]; // Various invalid flag combinations
    
    for invalid_flag in invalid_flags.iter() {
        println!("Testing invalid flag: {}", invalid_flag);
        
        let update_instruction = create_fee_update_instruction(
            pool_state_pda,
            &upgrade_authority,
            *invalid_flag, // Using invalid flag
            DEPOSIT_WITHDRAWAL_FEE,
            SWAP_CONTRACT_FEE,
        ).expect("Failed to create instruction");
        
        let transaction = Transaction::new_signed_with_payer(
            &[update_instruction],
            Some(&upgrade_authority.pubkey()),
            &[&upgrade_authority],
            recent_blockhash,
        );
        
        let result = banks_client.process_transaction(transaction).await;
        
        // Check that the transaction failed due to invalid flags
        match result {
            Ok(_) => {
                panic!("🚨 ERROR: Invalid flag {} was accepted when it should be rejected!", invalid_flag);
            },
            Err(e) => {
                println!("✅ SUCCESS: Invalid flag {} properly rejected", invalid_flag);
                println!("   Error: {:?}", e);
                
                // Verify it's the correct error (InvalidFeeUpdateFlags = 1043)
                if let BanksClientError::TransactionError(TransactionError::InstructionError(_, InstructionError::Custom(error_code))) = e {
                    assert_eq!(error_code, 1043, "Expected InvalidFeeUpdateFlags error (1043), got {}", error_code);
                    println!("✅ Correct error code: InvalidFeeUpdateFlags ({})", error_code);
                } else {
                    panic!("🚨 Unexpected error type: {:?}", e);
                }
            }
        }
    }
    
    // Verify the pool state was not modified
    let pool_account = banks_client.get_account(pool_state_pda).await
        .expect("Failed to get account")
        .expect("Pool state account not found");
    let pool_state = PoolState::try_from_slice(&pool_account.data)
        .expect("Failed to deserialize pool state");
    
    // Fees should remain unchanged
    assert_eq!(pool_state.contract_liquidity_fee, DEPOSIT_WITHDRAWAL_FEE);
    assert_eq!(pool_state.swap_contract_fee, SWAP_CONTRACT_FEE);
    println!("✅ Pool state unchanged - fees remain at original values");
} 

/// Test updating both liquidity and swap fees together (minimal setup)
#[tokio::test]
async fn test_update_both_fees_minimal() {
    // Create a minimal test environment without complex treasury setup
    let program_id = fixed_ratio_trading::id();
    let (program_data_account, _bump) = Pubkey::find_program_address(
        &[program_id.as_ref()],
        &solana_program::bpf_loader_upgradeable::id()
    );
    
    let mut program_test = ProgramTest::new(
        "fixed-ratio-trading",
        program_id,
        processor!(test_adapter),
    );
    
    // Create the upgrade authority keypair for testing
    let upgrade_authority = Keypair::new();
    
    // Create the program data account data
    let account_type: u32 = 3; // ProgramData type
    let has_upgrade_authority: u8 = 1; // true
    let slot: u64 = 0;
    
    let mut account_data = Vec::new();
    account_data.extend_from_slice(&account_type.to_le_bytes());
    account_data.push(has_upgrade_authority);
    account_data.extend_from_slice(upgrade_authority.pubkey().as_ref());
    account_data.extend_from_slice(&slot.to_le_bytes());
    account_data.extend_from_slice(&[0u8; 100]);
    
    // Add the program data account to the test environment
    program_test.add_account(
        program_data_account,
        Account {
            lamports: 1_000_000_000,
            data: account_data,
            owner: solana_program::bpf_loader_upgradeable::id(),
            executable: false,
            rent_epoch: 0,
        },
    );
    
    // Create a mock pool state account for testing with proper PDA derivation
    let token_a_mint = Pubkey::new_unique();
    let token_b_mint = Pubkey::new_unique(); 
    
    // Derive the pool state PDA correctly
    let pool_state_pda = {
        let seeds = &[
            b"pool_state",
            token_a_mint.as_ref(),
            token_b_mint.as_ref(),
            &[1u64.to_le_bytes(), 1u64.to_le_bytes()].concat(), // ratio_a:ratio_b = 1:1
        ];
        Pubkey::find_program_address(seeds, &program_id).0
    };
    
    let mut initial_pool_state = PoolState::default();
    initial_pool_state.token_a_mint = token_a_mint;
    initial_pool_state.token_b_mint = token_b_mint;
    initial_pool_state.ratio_a_numerator = 1;
    initial_pool_state.ratio_b_denominator = 1;
    initial_pool_state.contract_liquidity_fee = DEPOSIT_WITHDRAWAL_FEE;
    initial_pool_state.swap_contract_fee = SWAP_CONTRACT_FEE;
    
    // Create a proper system state account
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &program_id
    );
    
    let system_state = SystemState::new(upgrade_authority.pubkey()); // Creates unpaused state with upgrade authority as admin
    
    program_test.add_account(
        system_state_pda,
        Account {
            lamports: 1_000_000,
            data: system_state.try_to_vec().unwrap(),
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );
    
    program_test.add_account(
        pool_state_pda,
        Account {
            lamports: 10_000_000,
            data: initial_pool_state.try_to_vec().unwrap(),
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );
    
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Fund the upgrade authority
    let fund_upgrade_authority_ix = system_instruction::transfer(
        &payer.pubkey(),
        &upgrade_authority.pubkey(),
        1_000_000_000,
    );
    
    let fund_upgrade_authority_tx = Transaction::new_signed_with_payer(
        &[fund_upgrade_authority_ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    
    banks_client.process_transaction(fund_upgrade_authority_tx).await.expect("Failed to fund upgrade authority");
    
    // Test updating both fees with new values
    let new_liquidity_fee = DEPOSIT_WITHDRAWAL_FEE * 2; // Double the original
    let new_swap_fee = SWAP_CONTRACT_FEE * 3; // Triple the original
    
    println!("Original liquidity fee: {} lamports", DEPOSIT_WITHDRAWAL_FEE);
    println!("Original swap fee: {} lamports", SWAP_CONTRACT_FEE);
    println!("New liquidity fee: {} lamports", new_liquidity_fee);
    println!("New swap fee: {} lamports", new_swap_fee);
    
    let update_instruction = create_fee_update_instruction(
        pool_state_pda,
        &upgrade_authority,
        FEE_UPDATE_FLAG_BOTH, // Update both fees
        new_liquidity_fee,
        new_swap_fee,
    ).expect("Failed to create instruction");
    
    let transaction = Transaction::new_signed_with_payer(
        &[update_instruction],
        Some(&upgrade_authority.pubkey()),
        &[&upgrade_authority],
        recent_blockhash,
    );
    
    let result = banks_client.process_transaction(transaction).await;
    
    // Check that the transaction succeeded
    match result {
        Ok(_) => {
            println!("✅ SUCCESS: Both fees updated successfully");
            
            // Verify the pool state was properly updated
            let pool_account = banks_client.get_account(pool_state_pda).await
                .expect("Failed to get account")
                .expect("Pool state account not found");
            let pool_state = PoolState::try_from_slice(&pool_account.data)
                .expect("Failed to deserialize pool state");
            
            // Verify both fees were updated correctly
            assert_eq!(pool_state.contract_liquidity_fee, new_liquidity_fee, 
                      "Liquidity fee should be updated to {}", new_liquidity_fee);
            assert_eq!(pool_state.swap_contract_fee, new_swap_fee, 
                      "Swap fee should be updated to {}", new_swap_fee);
            
            println!("✅ Liquidity fee updated: {} → {} lamports", 
                    DEPOSIT_WITHDRAWAL_FEE, pool_state.contract_liquidity_fee);
            println!("✅ Swap fee updated: {} → {} lamports", 
                    SWAP_CONTRACT_FEE, pool_state.swap_contract_fee);
            println!("✅ Both fee updates verified on blockchain");
        },
        Err(e) => {
            panic!("🚨 ERROR: Fee update transaction failed: {:?}", e);
        }
    }
} 

/// Test updating only the liquidity fee (minimal setup)
#[tokio::test]
async fn test_update_liquidity_fee_only_minimal() {
    // Create a minimal test environment without complex treasury setup
    let program_id = fixed_ratio_trading::id();
    let (program_data_account, _bump) = Pubkey::find_program_address(
        &[program_id.as_ref()],
        &solana_program::bpf_loader_upgradeable::id()
    );
    
    let mut program_test = ProgramTest::new(
        "fixed-ratio-trading",
        program_id,
        processor!(test_adapter),
    );
    
    // Create the upgrade authority keypair for testing
    let upgrade_authority = Keypair::new();
    
    // Create the program data account data
    let account_type: u32 = 3; // ProgramData type
    let has_upgrade_authority: u8 = 1; // true
    let slot: u64 = 0;
    
    let mut account_data = Vec::new();
    account_data.extend_from_slice(&account_type.to_le_bytes());
    account_data.push(has_upgrade_authority);
    account_data.extend_from_slice(upgrade_authority.pubkey().as_ref());
    account_data.extend_from_slice(&slot.to_le_bytes());
    account_data.extend_from_slice(&[0u8; 100]);
    
    // Add the program data account to the test environment
    program_test.add_account(
        program_data_account,
        Account {
            lamports: 1_000_000_000,
            data: account_data,
            owner: solana_program::bpf_loader_upgradeable::id(),
            executable: false,
            rent_epoch: 0,
        },
    );
    
    // Create a mock pool state account for testing with proper PDA derivation
    let token_a_mint = Pubkey::new_unique();
    let token_b_mint = Pubkey::new_unique(); 
    
    // Derive the pool state PDA correctly
    let pool_state_pda = {
        let seeds = &[
            b"pool_state",
            token_a_mint.as_ref(),
            token_b_mint.as_ref(),
            &[1u64.to_le_bytes(), 1u64.to_le_bytes()].concat(), // ratio_a:ratio_b = 1:1
        ];
        Pubkey::find_program_address(seeds, &program_id).0
    };
    
    let mut initial_pool_state = PoolState::default();
    initial_pool_state.token_a_mint = token_a_mint;
    initial_pool_state.token_b_mint = token_b_mint;
    initial_pool_state.ratio_a_numerator = 1;
    initial_pool_state.ratio_b_denominator = 1;
    initial_pool_state.contract_liquidity_fee = DEPOSIT_WITHDRAWAL_FEE;
    initial_pool_state.swap_contract_fee = SWAP_CONTRACT_FEE;
    
    // Create a proper system state account
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &program_id
    );
    
    let system_state = SystemState::new(upgrade_authority.pubkey()); // Creates unpaused state with upgrade authority as admin
    
    program_test.add_account(
        system_state_pda,
        Account {
            lamports: 1_000_000,
            data: system_state.try_to_vec().unwrap(),
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );
    
    program_test.add_account(
        pool_state_pda,
        Account {
            lamports: 10_000_000,
            data: initial_pool_state.try_to_vec().unwrap(),
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        },
    );
    
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Fund the upgrade authority
    let fund_upgrade_authority_ix = system_instruction::transfer(
        &payer.pubkey(),
        &upgrade_authority.pubkey(),
        1_000_000_000,
    );
    
    let fund_upgrade_authority_tx = Transaction::new_signed_with_payer(
        &[fund_upgrade_authority_ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    
    banks_client.process_transaction(fund_upgrade_authority_tx).await.expect("Failed to fund upgrade authority");
    
    // Test updating only the liquidity fee
    let new_liquidity_fee = DEPOSIT_WITHDRAWAL_FEE * 4; // Quadruple the original
    let unchanged_swap_fee = SWAP_CONTRACT_FEE; // This should remain unchanged
    
    println!("Original liquidity fee: {} lamports", DEPOSIT_WITHDRAWAL_FEE);
    println!("Original swap fee: {} lamports", SWAP_CONTRACT_FEE);
    println!("New liquidity fee: {} lamports", new_liquidity_fee);
    println!("Swap fee should remain: {} lamports", unchanged_swap_fee);
    
    let update_instruction = create_fee_update_instruction(
        pool_state_pda,
        &upgrade_authority,
        FEE_UPDATE_FLAG_LIQUIDITY, // Update only liquidity fee
        new_liquidity_fee,
        unchanged_swap_fee, // This value should be ignored since we're only updating liquidity
    ).expect("Failed to create instruction");
    
    let transaction = Transaction::new_signed_with_payer(
        &[update_instruction],
        Some(&upgrade_authority.pubkey()),
        &[&upgrade_authority],
        recent_blockhash,
    );
    
    let result = banks_client.process_transaction(transaction).await;
    
    // Check that the transaction succeeded
    match result {
        Ok(_) => {
            println!("✅ SUCCESS: Liquidity fee updated successfully");
            
            // Verify the pool state was properly updated
            let pool_account = banks_client.get_account(pool_state_pda).await
                .expect("Failed to get account")
                .expect("Pool state account not found");
            let pool_state = PoolState::try_from_slice(&pool_account.data)
                .expect("Failed to deserialize pool state");
            
            // Verify only the liquidity fee was updated
            assert_eq!(pool_state.contract_liquidity_fee, new_liquidity_fee, 
                      "Liquidity fee should be updated to {}", new_liquidity_fee);
            assert_eq!(pool_state.swap_contract_fee, SWAP_CONTRACT_FEE, 
                      "Swap fee should remain unchanged at {}", SWAP_CONTRACT_FEE);
            
            println!("✅ Liquidity fee updated: {} → {} lamports", 
                    DEPOSIT_WITHDRAWAL_FEE, pool_state.contract_liquidity_fee);
            println!("✅ Swap fee unchanged: {} lamports (as expected)", 
                    pool_state.swap_contract_fee);
            println!("✅ Selective fee update verified on blockchain");
        },
        Err(e) => {
            panic!("🚨 ERROR: Liquidity fee update transaction failed: {:?}", e);
        }
    }
} 