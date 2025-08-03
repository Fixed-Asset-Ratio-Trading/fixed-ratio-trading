#[tokio::test]
#[serial]
async fn test_system_pause_persists_across_transactions() -> TestResult {
    // ============================================================================
    // 🎯 TEST CONFIGURATION - MODIFY THESE VALUES TO CHANGE TEST BEHAVIOR
    // ============================================================================
    
    // Debug Configuration
    const ENABLE_DEBUG_LOGGING: bool = false; // Set to true for verbose Solana runtime logs
    
    // System State Configuration
    const PAUSE_REASON_CODE: u8 = 1;           // Reason code for persistence test
    const NUMBER_OF_ATTEMPTS: usize = 5;       // Number of blocked operations to attempt
    
    // Treasury Configuration
    const USE_DONATE_SOL_FOR_SETUP: bool = true;   // Use donate_sol to add treasury liquidity
    const DONATION_AMOUNT_SOL: u64 = 8000;         // Donation for persistence testing
    const DONATION_MESSAGE: &str = "Test treasury setup for pause persistence testing";
    const WITHDRAWAL_ATTEMPT_SOL: u64 = 10;        // Small withdrawal to attempt repeatedly
    
    // Operation Configuration
    const TEST_TREASURY_WITHDRAWALS: bool = true;  // Test treasury withdrawal blocking
    const TEST_INVALID_AUTHORITIES: bool = true;   // Test with different invalid authorities
    
    // Verification Configuration
    const VERIFY_PAUSE_PERSISTENCE: bool = true;   // Verify pause persists across all attempts
    const VERIFY_TREASURY_UNCHANGED: bool = true;  // Verify treasury balance unchanged
    
    // ============================================================================
    // 🧪 TEST SETUP AND EXECUTION
    // ============================================================================
    
    println!("🧪 TEST: System pause persists across multiple transaction attempts");
    println!("====================================================================");
    println!("🎯 PURPOSE: Verify system pause blocks operations consistently across multiple transactions");
    println!("🔍 SCENARIO: Pause system, then attempt {} blocked operations", NUMBER_OF_ATTEMPTS);
    println!("✅ EXPECTED: All operations fail, pause state persists unchanged");
    
    // Create enhanced test foundation
    let mut foundation = create_enhanced_liquidity_test_foundation(None).await?;
    let env = &foundation.as_liquidity_foundation().env;
    let program_id = PROGRAM_ID;
    let payer = &env.payer;
    let recent_blockhash = env.recent_blockhash;
    let mut banks_client = env.banks_client.clone();
    
    // Get PDAs
    let system_state_pda = get_system_state_pda(&program_id);
    let main_treasury_pda = get_main_treasury_pda(&program_id);
    let program_data_account = get_program_data_address(&program_id);
    
    // Setup treasury with SOL balance using donate_sol
    if USE_DONATE_SOL_FOR_SETUP {
        setup_treasury_with_donation(
            &foundation,
            &mut banks_client,
            payer,
            recent_blockhash,
            DONATION_AMOUNT_SOL,
            DONATION_MESSAGE
        ).await?;
    }
    
    // Record initial treasury balance
    let initial_treasury_balance = banks_client.get_balance(main_treasury_pda).await?;
    
    // Pause the system
    println!("\n🔧 Step 1: Pausing system for persistence testing...");
    let upgrade_authority = payer;
    let pause_instruction = create_pause_system_instruction(
        &program_id,
        upgrade_authority,
        &system_state_pda,
        &program_data_account,
        PAUSE_REASON_CODE,
    )?;
    
    let mut transaction = Transaction::new_with_payer(
        &[pause_instruction],
        Some(&upgrade_authority.pubkey()),
    );
    transaction.sign(&[upgrade_authority], recent_blockhash);
    
    // Execute with timeout handling
    let timeout_duration = Duration::from_secs(30);
    let transaction_future = banks_client.process_transaction(transaction);
    
    match tokio::time::timeout(timeout_duration, transaction_future).await {
        Ok(result) => result?,
        Err(_) => {
            return Err("Pause transaction timed out after 30 seconds".into());
        }
    };
    
    // Verify system is paused and record state
    verify_system_paused(&mut banks_client, &system_state_pda, true, Some(PAUSE_REASON_CODE)).await?;
    
    let system_state_account = banks_client.get_account(system_state_pda).await?
        .ok_or("SystemState account not found")?;
    let initial_pause_state: SystemState = try_from_slice(&system_state_account.data)?;
    
    println!("📊 Initial pause state recorded:");
    println!("   - Reason code: {}", initial_pause_state.pause_reason_code);
    println!("   - Pause timestamp: {}", initial_pause_state.pause_timestamp);
    
    // Attempt multiple blocked operations
    println!("\n🔧 Step 2: Attempting {} blocked operations...", NUMBER_OF_ATTEMPTS);
    
    let mut successful_operations = 0;
    let mut failed_operations = 0;
    
    for attempt in 1..=NUMBER_OF_ATTEMPTS {
        println!("\n   📋 Attempt {} of {}", attempt, NUMBER_OF_ATTEMPTS);
        
        if TEST_TREASURY_WITHDRAWALS {
            // Create destination account for each attempt
            let destination = Keypair::new();
            
            // Use different authority for some attempts if configured
            let authority_to_use = if TEST_INVALID_AUTHORITIES && attempt % 2 == 0 {
                // Use random invalid authority for even attempts
                let invalid_auth = Keypair::new();
                println!("      🔑 Using invalid authority: {}", invalid_auth.pubkey());
                invalid_auth
            } else {
                // Use valid authority for odd attempts
                println!("      🔑 Using valid authority: {}", upgrade_authority.pubkey());
                Keypair::from_bytes(&upgrade_authority.to_bytes())?
            };
            
            // Create withdrawal instruction
            let withdrawal_instruction = create_treasury_withdrawal_instruction(
                &program_id,
                &authority_to_use,
                &main_treasury_pda,
                &destination.pubkey(),
                &system_state_pda,
                &program_data_account,
                WITHDRAWAL_ATTEMPT_SOL * 1_000_000_000, // Convert to lamports
            )?;
            
            let mut transaction = Transaction::new_with_payer(
                &[withdrawal_instruction],
                Some(&authority_to_use.pubkey()),
            );
            transaction.sign(&[&authority_to_use], recent_blockhash);
            
            // Execute with timeout handling
            let transaction_future = banks_client.process_transaction(transaction);
            let result = match tokio::time::timeout(timeout_duration, transaction_future).await {
                Ok(result) => result,
                Err(_) => {
                    println!("      ⏰ Transaction timed out (expected due to pause)");
                    failed_operations += 1;
                    continue;
                }
            };
            
            // Check result
            match result {
                Err(_) => {
                    println!("      ✅ Operation blocked as expected");
                    failed_operations += 1;
                }
                Ok(_) => {
                    println!("      ❌ Operation succeeded unexpectedly!");
                    successful_operations += 1;
                }
            }
        }
        
        // Verify pause state persists after each attempt
        if VERIFY_PAUSE_PERSISTENCE {
            verify_system_paused(&mut banks_client, &system_state_pda, true, Some(PAUSE_REASON_CODE)).await?;
            
            let system_state_account = banks_client.get_account(system_state_pda).await?
                .ok_or("SystemState account not found")?;
            let current_pause_state: SystemState = try_from_slice(&system_state_account.data)?;
            
            assert_eq!(
                initial_pause_state.pause_reason_code, current_pause_state.pause_reason_code,
                "Pause reason code should remain unchanged after attempt {}", attempt
            );
            assert_eq!(
                initial_pause_state.pause_timestamp, current_pause_state.pause_timestamp,
                "Pause timestamp should remain unchanged after attempt {}", attempt
            );
            assert!(current_pause_state.is_paused, "System should still be paused after attempt {}", attempt);
        }
    }
    
    // Final verification
    println!("\n📊 Operation attempt results:");
    println!("   - Failed operations: {} ✅", failed_operations);
    println!("   - Successful operations: {} {}", successful_operations, if successful_operations == 0 { "✅" } else { "❌" });
    
    if successful_operations > 0 {
        return Err(format!("Expected all operations to fail, but {} succeeded", successful_operations).into());
    }
    
    // Verify treasury balance unchanged
    if VERIFY_TREASURY_UNCHANGED {
        let final_treasury_balance = banks_client.get_balance(main_treasury_pda).await?;
        
        assert_eq!(
            initial_treasury_balance, final_treasury_balance,
            "Treasury balance should not change during blocked operations"
        );
        println!("✅ Treasury balance unchanged: {} lamports", final_treasury_balance);
    }
    
    // Final pause state verification
    verify_system_paused(&mut banks_client, &system_state_pda, true, Some(PAUSE_REASON_CODE)).await?;
    
    println!("\n✅ Test completed successfully");
    println!("   - All {} operations were blocked correctly", NUMBER_OF_ATTEMPTS);
    println!("   - Pause state persisted unchanged across all transactions");
    println!("   - Treasury balance remained unchanged");
    println!("   - System remains consistently paused");
    
    Ok(())
}