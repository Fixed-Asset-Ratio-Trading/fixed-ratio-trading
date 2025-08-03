#[tokio::test]
#[serial]
async fn test_system_pause_different_reason_codes() -> TestResult {
    // ============================================================================
    // 🎯 TEST CONFIGURATION - MODIFY THESE VALUES TO CHANGE TEST BEHAVIOR
    // ============================================================================
    
    // Debug Configuration
    const ENABLE_DEBUG_LOGGING: bool = false; // Set to true for verbose Solana runtime logs
    
    // System State Configuration - Test multiple reason codes
    const REASON_CODES_TO_TEST: &[u8] = &[1, 2, 3, 4, 5, 255]; // Various reason codes
    const REASON_CODE_DESCRIPTIONS: &[&str] = &[
        "General halt",
        "Emergency",
        "Security incident", 
        "Maintenance",
        "Upgrade",
        "Custom code"
    ];
    
    // Treasury Configuration
    const USE_DONATE_SOL_FOR_SETUP: bool = true;   // Use donate_sol to add treasury liquidity
    const DONATION_AMOUNT_SOL: u64 = 3000;         // Smaller donation for multiple tests
    const DONATION_MESSAGE: &str = "Test treasury setup for reason code testing";
    
    // Verification Configuration
    const VERIFY_REASON_CODE_STORAGE: bool = true; // Verify reason codes stored correctly
    const VERIFY_UNPAUSE_BETWEEN_TESTS: bool = true; // Unpause between reason code tests
    
    // ============================================================================
    // 🧪 TEST SETUP AND EXECUTION
    // ============================================================================
    
    println!("🧪 TEST: System pause with different reason codes");
    println!("=================================================");
    println!("🎯 PURPOSE: Verify system pause works correctly with various reason codes");
    println!("🔍 SCENARIO: Test pause with codes 1, 2, 3, 4, 5, and 255");
    println!("✅ EXPECTED: All reason codes work correctly and are stored properly");
    
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
    
    let upgrade_authority = payer;
    let timeout_duration = Duration::from_secs(30);
    
    // Test each reason code
    for (i, &reason_code) in REASON_CODES_TO_TEST.iter().enumerate() {
        let description = REASON_CODE_DESCRIPTIONS[i];
        
        println!("\n🔧 Testing reason code {} ({})", reason_code, description);
        
        // Create pause instruction with this reason code
        let pause_instruction = create_pause_system_instruction(
            &program_id,
            upgrade_authority,
            &system_state_pda,
            &program_data_account,
            reason_code,
        )?;
        
        let mut transaction = Transaction::new_with_payer(
            &[pause_instruction],
            Some(&upgrade_authority.pubkey()),
        );
        transaction.sign(&[upgrade_authority], recent_blockhash);
        
        // Execute pause with timeout handling
        let transaction_future = banks_client.process_transaction(transaction);
        
        match tokio::time::timeout(timeout_duration, transaction_future).await {
            Ok(result) => result?,
            Err(_) => {
                return Err(format!("Pause transaction for reason code {} timed out after 30 seconds", reason_code).into());
            }
        };
        
        // Verify system is paused with correct reason code
        verify_system_paused(&mut banks_client, &system_state_pda, true, Some(reason_code)).await?;
        
        if VERIFY_REASON_CODE_STORAGE {
            let system_state_account = banks_client.get_account(system_state_pda).await?
                .ok_or("SystemState account not found")?;
            let system_state: SystemState = try_from_slice(&system_state_account.data)?;
            
            assert_eq!(
                system_state.pause_reason_code, reason_code,
                "Stored reason code should match requested code"
            );
            
            println!("✅ Reason code {} stored correctly: {}", reason_code, description);
            println!("   - Is paused: {}", system_state.is_paused);
            println!("   - Pause timestamp: {}", system_state.pause_timestamp);
        }
        
        // Unpause system before testing next reason code (if not the last one)
        if VERIFY_UNPAUSE_BETWEEN_TESTS && i < REASON_CODES_TO_TEST.len() - 1 {
            println!("🔓 Unpausing system for next test...");
            
            let unpause_instruction = create_unpause_system_instruction(
                &program_id,
                upgrade_authority,
                &system_state_pda,
                &main_treasury_pda,
                &program_data_account,
            )?;
            
            let mut transaction = Transaction::new_with_payer(
                &[unpause_instruction],
                Some(&upgrade_authority.pubkey()),
            );
            transaction.sign(&[upgrade_authority], recent_blockhash);
            
            // Execute unpause with timeout handling
            let transaction_future = banks_client.process_transaction(transaction);
            
            match tokio::time::timeout(timeout_duration, transaction_future).await {
                Ok(result) => result?,
                Err(_) => {
                    return Err(format!("Unpause transaction after reason code {} timed out after 30 seconds", reason_code).into());
                }
            };
            
            // Verify system is unpaused
            verify_system_paused(&mut banks_client, &system_state_pda, false, None).await?;
            println!("✅ System unpaused successfully");
        }
    }
    
    println!("\n✅ Test completed successfully");
    println!("   - All {} reason codes tested successfully", REASON_CODES_TO_TEST.len());
    println!("   - Reason codes stored correctly in SystemState");
    println!("   - Pause/unpause cycle works with all codes");
    
    Ok(())
}

/// Helper function to create unpause system instruction
fn create_unpause_system_instruction(
    program_id: &Pubkey,
    upgrade_authority: &Keypair,
    system_state_pda: &Pubkey,
    main_treasury_pda: &Pubkey,
    program_data_account: &Pubkey,
) -> Result<Instruction, Box<dyn Error>> {
    Ok(Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(upgrade_authority.pubkey(), true),    // Index 0: Program Upgrade Authority (signer, writable)
            AccountMeta::new(*system_state_pda, false),           // Index 1: System State PDA (writable)
            AccountMeta::new(*main_treasury_pda, false),          // Index 2: Main Treasury PDA (writable for penalty)
            AccountMeta::new_readonly(*program_data_account, false), // Index 3: Program Data Account (readable)
        ],
        data: PoolInstruction::UnpauseSystem.try_to_vec()?,
    })
}