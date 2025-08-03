mod common;

#[cfg(test)]
mod test_token_account_security {
    use {
        solana_program::{
            program_error::ProgramError,
            pubkey::Pubkey,
            system_instruction,
        },
        solana_program_test::*,
        solana_sdk::{
            account::Account,
            signature::{Keypair, Signer},
            transaction::Transaction,
        },
        spl_token::{
            instruction as token_instruction,
            state::Account as TokenAccount,
        },
        fixed_ratio_trading::{
            state::PoolState,
            constants::*,
            instruction::FixedRatioTradingInstruction,
        },
        borsh::BorshSerialize,
    };

    use crate::common::{
        setup::*,
        tokens::*,
        pool_helpers::*,
        utils_test_utils::*,
    };

    /// Test that frozen token accounts are rejected
    #[tokio::test]
    async fn test_frozen_account_rejection() {
        let mut test_env = setup_test_environment().await;
        
        // Create pool with standard config
        let pool_config = PoolConfig::new_1_to_16_config(
            test_env.token_a_mint.pubkey(),
            test_env.token_b_mint.pubkey(),
        );
        
        // Initialize pool
        let pool_result = create_pool_standard(
            &mut test_env,
            &pool_config,
            100_000_000_000,  // 100 token A
            1_600_000_000_000, // 1600 token B
        ).await;
        
        assert!(pool_result.is_ok(), "Pool creation failed: {:?}", pool_result);

        // Create user and fund account
        let user = Keypair::new();
        test_env.fund_account(&user.pubkey(), 10_000_000_000).await;
        
        // Create and fund user token accounts
        let user_token_a = create_and_fund_token_account(
            &mut test_env,
            &test_env.token_a_mint.pubkey(),
            &user.pubkey(),
            1_000_000_000, // 1 token A
        ).await;
        
        let user_token_b = create_token_account(
            &mut test_env,
            &test_env.token_b_mint.pubkey(),
            &user.pubkey(),
        ).await;

        // Freeze token A account
        let freeze_ix = token_instruction::freeze_account(
            &spl_token::id(),
            &user_token_a,
            &test_env.token_a_mint.pubkey(),
            &test_env.mint_authority.pubkey(),
            &[],
        ).unwrap();

        let mut transaction = Transaction::new_with_payer(
            &[freeze_ix],
            Some(&test_env.context.payer.pubkey()),
        );
        transaction.sign(&[&test_env.context.payer, &test_env.mint_authority], test_env.context.last_blockhash);
        test_env.context.banks_client.process_transaction(transaction).await.unwrap();

        // Build swap instruction
        let swap_ix_data = FixedRatioTradingInstruction::Swap {
            amount: 500_000_000,
            is_a_to_b: true,
        };
        let mut swap_data = Vec::new();
        swap_ix_data.serialize(&mut swap_data).unwrap();

        let swap_accounts = vec![
            AccountMeta::new_readonly(pool_config.pool_state_pda, false),
            AccountMeta::new_readonly(user.pubkey(), true),
            AccountMeta::new(user_token_a, false),
            AccountMeta::new(user_token_b, false),
            AccountMeta::new(pool_config.token_a_vault_pda, false),
            AccountMeta::new(pool_config.token_b_vault_pda, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ];

        let swap_ix = Instruction {
            program_id: test_env.program_id,
            accounts: swap_accounts,
            data: swap_data,
        };

        // Try to execute swap with frozen account
        let mut transaction = Transaction::new_with_payer(
            &[swap_ix],
            Some(&test_env.context.payer.pubkey()),
        );
        transaction.sign(&[&test_env.context.payer, &user], test_env.context.last_blockhash);
        
        let result = test_env.context.banks_client.process_transaction(transaction).await;
        
        // Should fail due to frozen account
        assert!(result.is_err(), "Swap should fail with frozen account");
    }

    /// Test that delegated accounts are rejected
    #[tokio::test]
    async fn test_delegated_account_rejection() {
        let mut test_env = setup_test_environment().await;
        
        // Create pool
        let pool_config = PoolConfig::new_1_to_16_config(
            test_env.token_a_mint.pubkey(),
            test_env.token_b_mint.pubkey(),
        );
        
        let pool_result = create_pool_standard(
            &mut test_env,
            &pool_config,
            100_000_000_000,
            1_600_000_000_000,
        ).await;
        
        assert!(pool_result.is_ok(), "Pool creation failed");

        // Create user and delegate
        let user = Keypair::new();
        let delegate = Keypair::new();
        test_env.fund_account(&user.pubkey(), 10_000_000_000).await;
        
        // Create and fund user token account
        let user_token_a = create_and_fund_token_account(
            &mut test_env,
            &test_env.token_a_mint.pubkey(),
            &user.pubkey(),
            10_000_000_000,
        ).await;

        // Approve delegate
        let approve_ix = token_instruction::approve(
            &spl_token::id(),
            &user_token_a,
            &delegate.pubkey(),
            &user.pubkey(),
            &[],
            5_000_000_000,
        ).unwrap();

        let mut transaction = Transaction::new_with_payer(
            &[approve_ix],
            Some(&test_env.context.payer.pubkey()),
        );
        transaction.sign(&[&test_env.context.payer, &user], test_env.context.last_blockhash);
        test_env.context.banks_client.process_transaction(transaction).await.unwrap();

        // Derive LP token mint
        let (lp_token_a_mint_pda, _) = Pubkey::find_program_address(
            &[LP_TOKEN_A_MINT_SEED_PREFIX, pool_config.pool_state_pda.as_ref()],
            &test_env.program_id,
        );

        // Create LP token account
        let user_lp_account = create_token_account(
            &mut test_env,
            &lp_token_a_mint_pda,
            &user.pubkey(),
        ).await;

        // Build deposit instruction
        let deposit_ix_data = FixedRatioTradingInstruction::Deposit {
            amount: 3_000_000_000,
            deposit_token_mint_key: test_env.token_a_mint.pubkey(),
        };
        let mut deposit_data = Vec::new();
        deposit_ix_data.serialize(&mut deposit_data).unwrap();

        let deposit_accounts = vec![
            AccountMeta::new_readonly(pool_config.pool_state_pda, false),
            AccountMeta::new_readonly(user.pubkey(), true),
            AccountMeta::new(user_token_a, false),
            AccountMeta::new(user_lp_account, false),
            AccountMeta::new(pool_config.token_a_vault_pda, false),
            AccountMeta::new(pool_config.token_b_vault_pda, false),
            AccountMeta::new(lp_token_a_mint_pda, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ];

        let deposit_ix = Instruction {
            program_id: test_env.program_id,
            accounts: deposit_accounts,
            data: deposit_data,
        };

        // Try to execute deposit with delegated account
        let mut transaction = Transaction::new_with_payer(
            &[deposit_ix],
            Some(&test_env.context.payer.pubkey()),
        );
        transaction.sign(&[&test_env.context.payer, &user], test_env.context.last_blockhash);
        
        let result = test_env.context.banks_client.process_transaction(transaction).await;
        
        // Should fail due to delegated account
        assert!(result.is_err(), "Deposit should fail with delegated account");
    }

    /// Test mint correspondence validation
    #[tokio::test]
    async fn test_mint_correspondence() {
        let mut test_env = setup_test_environment().await;
        
        // Create pool
        let pool_config = PoolConfig::new_1_to_16_config(
            test_env.token_a_mint.pubkey(),
            test_env.token_b_mint.pubkey(),
        );
        
        let pool_result = create_pool_standard(
            &mut test_env,
            &pool_config,
            100_000_000_000,
            1_600_000_000_000,
        ).await;
        
        assert!(pool_result.is_ok(), "Pool creation failed");

        // Create user
        let user = Keypair::new();
        test_env.fund_account(&user.pubkey(), 10_000_000_000).await;
        
        // Create wrong mint
        let wrong_mint = Keypair::new();
        create_mint(&mut test_env, &wrong_mint, 9).await;
        
        // Create accounts with wrong mint
        let user_wrong_mint_account = create_and_fund_token_account(
            &mut test_env,
            &wrong_mint.pubkey(),
            &user.pubkey(),
            1_000_000_000,
        ).await;
        
        let user_token_b = create_token_account(
            &mut test_env,
            &test_env.token_b_mint.pubkey(),
            &user.pubkey(),
        ).await;

        // Build swap with wrong mint
        let swap_ix_data = FixedRatioTradingInstruction::Swap {
            amount: 500_000_000,
            is_a_to_b: true,
        };
        let mut swap_data = Vec::new();
        swap_ix_data.serialize(&mut swap_data).unwrap();

        let swap_accounts = vec![
            AccountMeta::new_readonly(pool_config.pool_state_pda, false),
            AccountMeta::new_readonly(user.pubkey(), true),
            AccountMeta::new(user_wrong_mint_account, false), // Wrong mint!
            AccountMeta::new(user_token_b, false),
            AccountMeta::new(pool_config.token_a_vault_pda, false),
            AccountMeta::new(pool_config.token_b_vault_pda, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ];

        let swap_ix = Instruction {
            program_id: test_env.program_id,
            accounts: swap_accounts,
            data: swap_data,
        };

        // Try to execute swap
        let mut transaction = Transaction::new_with_payer(
            &[swap_ix],
            Some(&test_env.context.payer.pubkey()),
        );
        transaction.sign(&[&test_env.context.payer, &user], test_env.context.last_blockhash);
        
        let result = test_env.context.banks_client.process_transaction(transaction).await;
        
        // Should fail due to wrong mint
        assert!(result.is_err(), "Swap should fail with wrong mint");
    }
}