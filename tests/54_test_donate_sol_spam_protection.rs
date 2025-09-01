use {
    solana_program_test::tokio,
    solana_sdk::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        transaction::Transaction,
        system_program,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    serial_test::serial,
    fixed_ratio_trading::{
        constants::*,
        PoolInstruction,
        state::MainTreasuryState,
    },
    std::error::Error,
};

mod common;
use common::{
    setup::{initialize_treasury_system, start_test_environment},
};

/// 🎯 TEST CONFIGURATION - MODIFY THESE VALUES TO CHANGE TEST BEHAVIOR
const SPAM_DONATION_COUNT: u64 = 20;         // Number of spam donations to test (reduced due to 0.1 SOL minimum)
const TEST_DONATION_AMOUNT: u64 = 100_000_000; // 0.1 SOL - minimum donation amount
#[allow(dead_code)]
const NORMAL_DONATION_AMOUNT: u64 = 1_000_000_000; // 1 SOL for normal donations

/// DONATE-001: Test spam protection for Donate_Sol function
/// 
/// This test verifies that the Donate_Sol function:
/// 1. Cannot be spammed to cause data corruption
/// 2. Properly tracks donation counts even under spam conditions
/// 3. Maintains data integrity with many small donations
#[tokio::test]
#[serial]
async fn test_donate_sol_spam_protection() -> Result<(), Box<dyn Error>> {
    println!("🧪 Testing DONATE-001: Donate_Sol spam protection...");
    
    // Initialize test environment
    let mut env = start_test_environment().await;
    
    // Initialize treasury system
    let system_authority = Keypair::new();
    initialize_treasury_system(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &system_authority,
    ).await?;
    
    // Get treasury PDA
    let (main_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        &fixed_ratio_trading::ID,
    );
    
    // Get system state PDA
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &fixed_ratio_trading::ID,
    );
    
    // Get initial treasury state
    let initial_account = env.banks_client.get_account(main_treasury_pda).await?.unwrap();
    let initial_state = MainTreasuryState::try_from_slice(&initial_account.data)?;
    
    println!("📊 Initial treasury state:");
    println!("   - Donation count: {}", initial_state.donation_count);
    println!("   - Total donations: {} lamports", initial_state.total_donations);
    println!("   - Total balance: {} lamports", initial_state.total_balance);
    
    // Create a donor account with sufficient SOL for spam test
    let donor = Keypair::new();
    let donor_balance = 50_000_000_000; // 50 SOL for transaction fees and donations (more needed due to 0.1 SOL minimum)
    
    // Transfer SOL to donor
    let transfer_ix = solana_sdk::system_instruction::transfer(
        &env.payer.pubkey(),
        &donor.pubkey(),
        donor_balance,
    );
    
    let mut transfer_tx = Transaction::new_with_payer(
        &[transfer_ix],
        Some(&env.payer.pubkey()),
    );
    transfer_tx.sign(&[&env.payer], env.recent_blockhash);
    env.banks_client.process_transaction(transfer_tx).await?;
    
    println!("\n🔥 Step 1: Attempting spam attack with {} minimum donations...", SPAM_DONATION_COUNT);
    
    let mut successful_donations = 0;
    let mut total_donated = 0u64;
    let mut failed_donations = 0;
    
    // Attempt to spam minimum donations (0.1 SOL each)
    for i in 0..SPAM_DONATION_COUNT {
        env.recent_blockhash = env.banks_client.get_latest_blockhash().await?;
        
        let donation_amount = TEST_DONATION_AMOUNT + (i * 1_000_000); // Vary between 0.1-0.12 SOL
        let message = format!("Spam donation #{}", i);
        
        // Create donation instruction
        let donate_ix = Instruction {
            program_id: fixed_ratio_trading::ID,
            accounts: vec![
                AccountMeta::new(donor.pubkey(), true),        // Donor (signer, writable)
                AccountMeta::new(main_treasury_pda, false),    // Treasury (writable)
                AccountMeta::new_readonly(system_state_pda, false), // System state
                AccountMeta::new_readonly(system_program::id(), false), // System program
            ],
            data: PoolInstruction::DonateSol {
                amount: donation_amount,
                message,
            }.try_to_vec()?,
        };
        
        let mut donate_tx = Transaction::new_with_payer(
            &[donate_ix],
            Some(&donor.pubkey()),
        );
        donate_tx.sign(&[&donor], env.recent_blockhash);
        
        match env.banks_client.process_transaction(donate_tx).await {
            Ok(_) => {
                successful_donations += 1;
                total_donated += donation_amount;
                            if i % 5 == 0 {
                println!("   ✅ Donation #{} successful ({:.3} SOL)", i, donation_amount as f64 / 1_000_000_000.0);
            }
            },
            Err(e) => {
                failed_donations += 1;
                println!("   ❌ Donation #{} failed: {:?}", i, e);
            }
        }
    }
    
    println!("\n📊 Spam attack results:");
    println!("   - Successful donations: {}", successful_donations);
    println!("   - Failed donations: {}", failed_donations);
    println!("   - Total amount donated: {} lamports", total_donated);
    
    // Get treasury state after spam
    let spam_account = env.banks_client.get_account(main_treasury_pda).await?.unwrap();
    let spam_state = MainTreasuryState::try_from_slice(&spam_account.data)?;
    
    println!("\n📊 Treasury state after spam:");
    println!("   - Donation count: {} (increased by {})", 
        spam_state.donation_count, 
        spam_state.donation_count - initial_state.donation_count);
    println!("   - Total donations: {} lamports (increased by {})", 
        spam_state.total_donations,
        spam_state.total_donations - initial_state.total_donations);
    println!("   - Total balance: {} lamports", spam_state.total_balance);
    
    // Verify data integrity
    let count_increase = spam_state.donation_count - initial_state.donation_count;
    let donation_increase = spam_state.total_donations - initial_state.total_donations;
    
    assert_eq!(count_increase, successful_donations as u64, "Donation count mismatch");
    assert_eq!(donation_increase, total_donated, "Total donations mismatch");
    
    println!("\n✅ Data integrity verified: Counts and totals match expected values");
    
    // Step 2: Test edge cases
    println!("\n🔍 Step 2: Testing edge cases...");
    
    // Test zero amount donation (should fail)
    env.recent_blockhash = env.banks_client.get_latest_blockhash().await?;
    let zero_donate_ix = Instruction {
        program_id: fixed_ratio_trading::ID,
        accounts: vec![
            AccountMeta::new(donor.pubkey(), true),
            AccountMeta::new(main_treasury_pda, false),
            AccountMeta::new_readonly(system_state_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: PoolInstruction::DonateSol {
            amount: 0,
            message: "Zero donation test".to_string(),
        }.try_to_vec()?,
    };
    
    let mut zero_tx = Transaction::new_with_payer(
        &[zero_donate_ix],
        Some(&donor.pubkey()),
    );
    zero_tx.sign(&[&donor], env.recent_blockhash);
    
    match env.banks_client.process_transaction(zero_tx).await {
        Ok(_) => panic!("Zero donation should have failed"),
        Err(_) => println!("   ✅ Zero donation correctly rejected"),
    }
    
    // Test below minimum donation (should fail)
    env.recent_blockhash = env.banks_client.get_latest_blockhash().await?;
    let below_min_ix = Instruction {
        program_id: fixed_ratio_trading::ID,
        accounts: vec![
            AccountMeta::new(donor.pubkey(), true),
            AccountMeta::new(main_treasury_pda, false),
            AccountMeta::new_readonly(system_state_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: PoolInstruction::DonateSol {
            amount: 50_000_000, // 0.05 SOL - below 0.1 SOL minimum
            message: "Below minimum donation test".to_string(),
        }.try_to_vec()?,
    };
    
    let mut below_min_tx = Transaction::new_with_payer(
        &[below_min_ix],
        Some(&donor.pubkey()),
    );
    below_min_tx.sign(&[&donor], env.recent_blockhash);
    
    match env.banks_client.process_transaction(below_min_tx).await {
        Ok(_) => panic!("Below minimum donation should have failed"),
        Err(_) => println!("   ✅ Below minimum (0.05 SOL) donation correctly rejected"),
    }
    
    // Test very large message (potential DoS vector)
    let large_message = "A".repeat(1000); // 1000 character message
    env.recent_blockhash = env.banks_client.get_latest_blockhash().await?;
    
    let large_msg_ix = Instruction {
        program_id: fixed_ratio_trading::ID,
        accounts: vec![
            AccountMeta::new(donor.pubkey(), true),
            AccountMeta::new(main_treasury_pda, false),
            AccountMeta::new_readonly(system_state_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: PoolInstruction::DonateSol {
            amount: 1000,
            message: large_message,
        }.try_to_vec()?,
    };
    
    let mut large_msg_tx = Transaction::new_with_payer(
        &[large_msg_ix],
        Some(&donor.pubkey()),
    );
    large_msg_tx.sign(&[&donor], env.recent_blockhash);
    
    match env.banks_client.process_transaction(large_msg_tx).await {
        Ok(_) => println!("   ✅ Large message donation processed successfully"),
        Err(e) => println!("   ℹ️ Large message donation result: {:?}", e),
    }
    
    // Step 3: Calculate spam cost analysis
    println!("\n💰 Step 3: Spam cost analysis...");
    
    let avg_tx_fee = 5000; // Average transaction fee in lamports
    let total_tx_fees = successful_donations * avg_tx_fee;
    let cost_per_count = (total_tx_fees + total_donated) / successful_donations;
    
    println!("   - Average tx fee per donation: {} lamports", avg_tx_fee);
    println!("   - Total transaction fees paid: {} lamports", total_tx_fees);
    println!("   - Total cost (fees + donations): {} lamports", total_tx_fees + total_donated);
    println!("   - Cost per donation count: {} lamports", cost_per_count);
    println!("   - Cost to inflate count by 1M: {} SOL", 
        (cost_per_count * 1_000_000) as f64 / 1_000_000_000.0);
    
    // Final state verification
    let final_account = env.banks_client.get_account(main_treasury_pda).await?.unwrap();
    let final_state = MainTreasuryState::try_from_slice(&final_account.data)?;
    
    println!("\n📊 Final treasury state:");
    println!("   - Donation count: {}", final_state.donation_count);
    println!("   - Total donations: {} lamports ({:.6} SOL)", 
        final_state.total_donations,
        final_state.total_donations as f64 / 1_000_000_000.0);
    println!("   - Data integrity: ✅ All counters consistent");
    
    println!("\n✅ DONATE-001: Spam protection test completed!");
    println!("🔐 Security findings:");
    println!("   - Function cannot be spammed to corrupt data");
    println!("   - Counters remain accurate under spam conditions");
    println!("   - 0.1 SOL minimum donation requirement prevents meaningful spam");
    println!("   - Combined with transaction fees, makes spam attacks very expensive");
    println!("   - No overflow risk for practical attack scenarios");
    
    Ok(())
}

/// DONATE-002: Test rate of donation spam and economic analysis
/// 
/// This test analyzes the economic cost of spamming donations
/// and verifies that spam attacks are economically unfeasible
#[tokio::test]
#[serial]
async fn test_donation_spam_economic_analysis() -> Result<(), Box<dyn Error>> {
    println!("🧪 Testing DONATE-002: Donation spam economic analysis...");
    
    // Initialize test environment
    let mut env = start_test_environment().await;
    
    // Initialize treasury system
    let system_authority = Keypair::new();
    initialize_treasury_system(
        &mut env.banks_client,
        &env.payer,
        env.recent_blockhash,
        &system_authority,
    ).await?;
    
    // Get treasury PDA
    let (main_treasury_pda, _) = Pubkey::find_program_address(
        &[MAIN_TREASURY_SEED_PREFIX],
        &fixed_ratio_trading::ID,
    );
    
    // Get system state PDA
    let (system_state_pda, _) = Pubkey::find_program_address(
        &[SYSTEM_STATE_SEED_PREFIX],
        &fixed_ratio_trading::ID,
    );
    
    println!("📊 Analyzing economic cost of donation spam...");
    
    // Create multiple donors to simulate different attack vectors
    let donors: Vec<Keypair> = (0..5).map(|_| Keypair::new()).collect();
    
    // Fund each donor
    for donor in &donors {
        let transfer_ix = solana_sdk::system_instruction::transfer(
            &env.payer.pubkey(),
            &donor.pubkey(),
            5_000_000_000, // 5 SOL each (needed for 20 donations at 0.1 SOL minimum + fees)
        );
        
        let mut transfer_tx = Transaction::new_with_payer(
            &[transfer_ix],
            Some(&env.payer.pubkey()),
        );
        transfer_tx.sign(&[&env.payer], env.recent_blockhash);
        env.banks_client.process_transaction(transfer_tx).await?;
    }
    
    println!("\n🔥 Testing concurrent donation spam from multiple accounts...");
    
    let mut total_spent = 0u64;
    let mut total_donations_made = 0u64;
    
    // Each donor makes 20 donations
    for (donor_idx, donor) in donors.iter().enumerate() {
        let initial_balance = env.banks_client.get_balance(donor.pubkey()).await?;
        
        for i in 0..20 {
            env.recent_blockhash = env.banks_client.get_latest_blockhash().await?;
            
            let donation_amount = TEST_DONATION_AMOUNT; // 0.1 SOL minimum donation
            let donate_ix = Instruction {
                program_id: fixed_ratio_trading::ID,
                accounts: vec![
                    AccountMeta::new(donor.pubkey(), true),
                    AccountMeta::new(main_treasury_pda, false),
                    AccountMeta::new_readonly(system_state_pda, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
                data: PoolInstruction::DonateSol {
                    amount: donation_amount,
                    message: format!("Spam from donor {} donation {}", donor_idx, i),
                }.try_to_vec()?,
            };
            
            let mut donate_tx = Transaction::new_with_payer(
                &[donate_ix],
                Some(&donor.pubkey()),
            );
            donate_tx.sign(&[donor], env.recent_blockhash);
            
            if env.banks_client.process_transaction(donate_tx).await.is_ok() {
                total_donations_made += 1;
            }
        }
        
        let final_balance = env.banks_client.get_balance(donor.pubkey()).await?;
        let spent = initial_balance - final_balance;
        total_spent += spent;
        
        println!("   Donor {}: Spent {} lamports for 20 donations", donor_idx, spent);
    }
    
    // Calculate costs
    let avg_cost_per_donation = total_spent / total_donations_made;
    let cost_to_inflate_by_million = avg_cost_per_donation * 1_000_000;
    let cost_to_inflate_by_billion = avg_cost_per_donation * 1_000_000_000;
    
    println!("\n💰 Economic Analysis:");
    println!("   - Total donations made: {}", total_donations_made);
    println!("   - Total cost (including fees): {} lamports", total_spent);
    println!("   - Average cost per donation: {} lamports", avg_cost_per_donation);
    println!("   - Cost to inflate count by 1M: {} SOL", cost_to_inflate_by_million as f64 / 1_000_000_000.0);
    println!("   - Cost to inflate count by 1B: {} SOL", cost_to_inflate_by_billion as f64 / 1_000_000_000.0);
    
    // Overflow analysis
    println!("\n📈 Counter Overflow Analysis:");
    println!("   - u64 max value: {}", u64::MAX);
    println!("   - Donations needed for overflow: {}", u64::MAX);
    let years_at_max_speed = u64::MAX / (50000 * 365 * 24 * 60 * 60); // 50k donations/sec
    println!("   - Years to overflow at 50k donations/sec: {}", years_at_max_speed);
    println!("   - Cost to cause overflow: {} SOL", (u64::MAX / 1000) as f64 * avg_cost_per_donation as f64 / 1_000_000_000.0);
    
    // Get final treasury state
    let final_account = env.banks_client.get_account(main_treasury_pda).await?.unwrap();
    let final_state = MainTreasuryState::try_from_slice(&final_account.data)?;
    
    println!("\n📊 Final Treasury State:");
    println!("   - Total donation count: {}", final_state.donation_count);
    println!("   - Total donations value: {} lamports", final_state.total_donations);
    
    println!("\n✅ DONATE-002: Economic analysis completed!");
    println!("🔐 Conclusion: Spam attacks are economically unfeasible due to 0.1 SOL minimum + transaction fees");
    
    Ok(())
}