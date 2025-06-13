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

//! # Token Creation and Management Utilities
//! 
//! This module provides utilities for creating and managing SPL tokens
//! in integration tests, including mint creation, token account setup,
//! and token minting operations.

use solana_program_test::BanksClient;
use solana_sdk::{signature::Keypair, signer::Signer, program_pack::Pack};
use spl_token::{instruction as token_instruction, state::Account as TokenAccount};
use crate::common::{constants, TestResult};

/// Helper function to create a token mint
/// 
/// Creates a new SPL token mint with the specified authority and decimals.
/// This is the primary utility for creating test tokens.
/// 
/// # Arguments
/// * `banks` - Banks client for transaction processing
/// * `payer` - Account that pays for the mint creation
/// * `recent_blockhash` - Recent blockhash for transaction
/// * `mint` - Keypair for the new mint account
/// * `decimals` - Number of decimal places (defaults to 9 if None)
/// 
/// # Returns
/// Result indicating success or failure of mint creation
pub async fn create_mint(
    banks: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
    mint: &Keypair,
    decimals: Option<u8>,
) -> TestResult {
    let decimals = decimals.unwrap_or(constants::TOKEN_DECIMALS);
    let rent = banks.get_rent().await.unwrap();
    let lamports = rent.minimum_balance(spl_token::state::Mint::LEN);

    let create_account_ix = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        lamports,
        spl_token::state::Mint::LEN as u64,
        &spl_token::id(),
    );
    
    let initialize_mint_ix = token_instruction::initialize_mint(
        &spl_token::id(),
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        decimals,
    )
    .unwrap();

    let mut transaction = solana_sdk::transaction::Transaction::new_with_payer(
        &[create_account_ix, initialize_mint_ix], 
        Some(&payer.pubkey())
    );
    transaction.sign(&[payer, mint], recent_blockhash);
    banks.process_transaction(transaction).await
}

/// Create a token account for a specific mint and owner
/// 
/// # Arguments
/// * `banks` - Banks client for transaction processing
/// * `payer` - Account that pays for the token account creation
/// * `recent_blockhash` - Recent blockhash for transaction
/// * `token_account` - Keypair for the new token account
/// * `mint` - Mint that this token account will hold
/// * `owner` - Owner of the token account
pub async fn create_token_account(
    banks: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
    token_account: &Keypair,
    mint: &solana_program::pubkey::Pubkey,
    owner: &solana_program::pubkey::Pubkey,
) -> TestResult {
    let rent = banks.get_rent().await.unwrap();
    let lamports = rent.minimum_balance(TokenAccount::LEN);

    let create_account_ix = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &token_account.pubkey(),
        lamports,
        TokenAccount::LEN as u64,
        &spl_token::id(),
    );
    
    let initialize_account_ix = token_instruction::initialize_account(
        &spl_token::id(),
        &token_account.pubkey(),
        mint,
        owner,
    )
    .unwrap();

    let mut transaction = solana_sdk::transaction::Transaction::new_with_payer(
        &[create_account_ix, initialize_account_ix],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, token_account], recent_blockhash);
    banks.process_transaction(transaction).await
}

/// Mint tokens to a specified token account
/// 
/// # Arguments
/// * `banks` - Banks client for transaction processing
/// * `payer` - Account that pays for the transaction
/// * `recent_blockhash` - Recent blockhash for transaction
/// * `mint` - Mint to mint tokens from
/// * `destination` - Token account to mint tokens to
/// * `authority` - Mint authority
/// * `amount` - Amount of tokens to mint
pub async fn mint_tokens(
    banks: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
    mint: &solana_program::pubkey::Pubkey,
    destination: &solana_program::pubkey::Pubkey,
    authority: &Keypair,
    amount: u64,
) -> TestResult {
    let mint_to_ix = token_instruction::mint_to(
        &spl_token::id(),
        mint,
        destination,
        &authority.pubkey(),
        &[],
        amount,
    )
    .unwrap();

    let mut transaction = solana_sdk::transaction::Transaction::new_with_payer(
        &[mint_to_ix],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, authority], recent_blockhash);
    banks.process_transaction(transaction).await
}

/// Get the balance of a token account
/// 
/// # Arguments
/// * `banks` - Banks client for account fetching
/// * `token_account` - Token account to check balance of
/// 
/// # Returns
/// Token balance or 0 if account doesn't exist
pub async fn get_token_balance(
    banks: &mut BanksClient,
    token_account: &solana_program::pubkey::Pubkey,
) -> u64 {
    match banks.get_account(*token_account).await {
        Ok(Some(account)) => {
            match TokenAccount::unpack(&account.data) {
                Ok(token_account_data) => token_account_data.amount,
                Err(_) => 0,
            }
        },
        _ => 0,
    }
}

/// Convenience function to create multiple test mints at once
/// 
/// # Arguments
/// * `banks` - Banks client for transaction processing
/// * `payer` - Account that pays for mint creation
/// * `recent_blockhash` - Recent blockhash for transaction
/// * `mint_keypairs` - Slice of keypairs for the mints to create
/// 
/// # Returns
/// Result indicating success or failure
pub async fn create_test_mints(
    banks: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
    mint_keypairs: &[&Keypair],
) -> TestResult {
    for mint_kp in mint_keypairs {
        create_mint(banks, payer, recent_blockhash, mint_kp, None).await?;
    }
    Ok(())
}

/// Create a pair of user token accounts for primary and base tokens
/// 
/// Returns (primary_token_account, base_token_account) keypairs
/// 
/// # Arguments
/// * `banks` - Banks client for transaction processing
/// * `payer` - Account that pays for token account creation
/// * `recent_blockhash` - Recent blockhash for transaction
/// * `primary_mint` - Primary token mint
/// * `base_mint` - Base token mint
/// * `user` - User who will own the token accounts
pub async fn create_user_token_accounts(
    banks: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
    primary_mint: &solana_program::pubkey::Pubkey,
    base_mint: &solana_program::pubkey::Pubkey,
    user: &solana_program::pubkey::Pubkey,
) -> Result<(Keypair, Keypair), solana_program_test::BanksClientError> {
    let primary_token_account = Keypair::new();
    let base_token_account = Keypair::new();

    create_token_account(
        banks, 
        payer, 
        recent_blockhash, 
        &primary_token_account, 
        primary_mint, 
        user
    ).await?;
    
    create_token_account(
        banks, 
        payer, 
        recent_blockhash, 
        &base_token_account, 
        base_mint, 
        user
    ).await?;

    Ok((primary_token_account, base_token_account))
}

/// Setup a test user with SOL and token accounts
/// 
/// Creates a user account, airdrops SOL, and creates token accounts for specified mints
/// 
/// # Arguments
/// * `banks` - Banks client for transaction processing
/// * `payer` - Account that pays for setup
/// * `recent_blockhash` - Recent blockhash for transaction
/// * `primary_mint` - Primary token mint
/// * `base_mint` - Base token mint
/// * `sol_amount` - Amount of SOL to airdrop (uses default if None)
/// 
/// # Returns
/// (user_keypair, primary_token_account, base_token_account)
pub async fn setup_test_user(
    banks: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
    primary_mint: &solana_program::pubkey::Pubkey,
    base_mint: &solana_program::pubkey::Pubkey,
    sol_amount: Option<u64>,
) -> Result<(Keypair, Keypair, Keypair), solana_program_test::BanksClientError> {
    let user = Keypair::new();
    let sol_amount = sol_amount.unwrap_or(constants::DEFAULT_SOL_AIRDROP);

    // Airdrop SOL to user
    let airdrop_ix = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &user.pubkey(),
        sol_amount,
    );
    let mut airdrop_tx = solana_sdk::transaction::Transaction::new_with_payer(
        &[airdrop_ix], 
        Some(&payer.pubkey())
    );
    airdrop_tx.sign(&[payer], recent_blockhash);
    banks.process_transaction(airdrop_tx).await?;

    // Create user token accounts
    let (primary_token_account, base_token_account) = create_user_token_accounts(
        banks,
        payer,
        recent_blockhash,
        primary_mint,
        base_mint,
        &user.pubkey(),
    ).await?;

    Ok((user, primary_token_account, base_token_account))
} 