mod common;

use common::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Test the two-step withdrawal validation process
#[tokio::test]
async fn test_withdrawal_delegate_process() -> TestResult {
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create token mints and pool
    create_test_mints(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &[&ctx.primary_mint, &ctx.base_mint],
    ).await?;

    let config = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &ctx.primary_mint,
        &ctx.base_mint,
        &ctx.lp_token_a_mint,
        &ctx.lp_token_b_mint,
        None,
    ).await?;

    // Create and add a delegate
    let delegate = create_funded_user(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        None,
    ).await?;

    let _add_result = add_delegate(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &config.pool_state_pda,
        &delegate.pubkey(),
    ).await?;

    // Step 1: Delegate requests withdrawal
    let request_amount = 1_000_000u64;
    let token_mint = config.token_a_mint;

    let request_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(delegate.pubkey(), true),
            AccountMeta::new(config.pool_state_pda, false),
            AccountMeta::new_readonly(solana_program::sysvar::clock::id(), false),
        ],
        data: PoolInstruction::RequestDelegateAction {
            action_type: DelegateActionType::Withdrawal,
            params: DelegateActionParams::Withdrawal {
                token_mint,
                amount: request_amount,
            },
        }.try_to_vec().unwrap(),
    };

    let mut request_tx = Transaction::new_with_payer(&[request_ix], Some(&delegate.pubkey()));
    request_tx.sign(&[&delegate], ctx.env.recent_blockhash);
    
    let request_result = ctx.env.banks_client.process_transaction(request_tx).await;
    assert!(request_result.is_ok(), "Delegate withdrawal request should succeed");
    println!("✅ Step 1: Delegate withdrawal request successful");

    // Step 2: Try to execute withdrawal before wait time (should fail)
    let execute_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(config.pool_state_pda, false),
            AccountMeta::new(delegate.pubkey(), true),
            AccountMeta::new_readonly(solana_program::sysvar::clock::id(), false),
        ],
        data: PoolInstruction::ExecuteDelegateAction {
            action_id: 1, // First action ID
        }.try_to_vec().unwrap(),
    };

    let mut execute_tx = Transaction::new_with_payer(&[execute_ix], Some(&delegate.pubkey()));
    execute_tx.sign(&[&delegate], ctx.env.recent_blockhash);
    
    let early_execute_result = ctx.env.banks_client.process_transaction(execute_tx).await;
    assert!(early_execute_result.is_err(), "Early execution should fail");
    println!("✅ Step 2: Early execution correctly prevented");

    // Step 3: Owner revokes the withdrawal request
    let revoke_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(ctx.env.payer.pubkey(), true), // Pool owner
            AccountMeta::new(config.pool_state_pda, false),
        ],
        data: PoolInstruction::RevokeAction {
            action_id: 1, // First action ID
        }.try_to_vec().unwrap(),
    };

    let mut revoke_tx = Transaction::new_with_payer(&[revoke_ix], Some(&ctx.env.payer.pubkey()));
    revoke_tx.sign(&[&ctx.env.payer], ctx.env.recent_blockhash);
    
    let revoke_result = ctx.env.banks_client.process_transaction(revoke_tx).await;
    assert!(revoke_result.is_ok(), "Owner should be able to revoke withdrawal request");
    println!("✅ Step 3: Owner successfully revoked withdrawal request");

    // Step 4: Try to execute revoked withdrawal (should fail)
    let execute_revoked_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(config.pool_state_pda, false),
            AccountMeta::new(delegate.pubkey(), true),
            AccountMeta::new_readonly(solana_program::sysvar::clock::id(), false),
        ],
        data: PoolInstruction::ExecuteDelegateAction {
            action_id: 1, // First action ID
        }.try_to_vec().unwrap(),
    };

    let mut execute_revoked_tx = Transaction::new_with_payer(&[execute_revoked_ix], Some(&delegate.pubkey()));
    execute_revoked_tx.sign(&[&delegate], ctx.env.recent_blockhash);
    
    let revoked_execute_result = ctx.env.banks_client.process_transaction(execute_revoked_tx).await;
    assert!(revoked_execute_result.is_err(), "Execution of revoked withdrawal should fail");
    println!("✅ Step 4: Execution of revoked withdrawal correctly prevented");

    println!("✅ Two-step withdrawal validation process successfully tested");
    Ok(())
} 