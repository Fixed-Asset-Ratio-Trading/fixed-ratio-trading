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

//! Admin Change Success Case Test
//! 
//! This test covers a happy-path flow for `process_admin_change`:
//! - Initialize the system with a known admin authority
//! - Propose a new admin authority (starts 72h timelock)
//! - Warp time forward beyond the timelock
//! - Call `process_admin_change` again to complete the change
//! - Verify the new admin can execute an admin-only instruction (`PauseSystem`)

#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

mod common;

use common::*;
use common::setup::{
    create_program_test,
    initialize_treasury_system,
    get_test_program_data_address,
};

use borsh::{BorshDeserialize, BorshSerialize};
use fixed_ratio_trading::{
    types::instructions::PoolInstruction,
    state::SystemState,
    constants::SYSTEM_STATE_SEED_PREFIX,
};
use solana_program_test::{ProgramTestContext};
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::{Keypair, Signer},
    transaction::Transaction,
    clock::Clock,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn get_system_state_pda(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[SYSTEM_STATE_SEED_PREFIX], program_id).0
}

#[tokio::test]
async fn test_process_admin_change_success_and_new_admin_can_pause() -> TestResult {
    // Build program test context (we need warp_to_slot for timelock)
    let program_test = create_program_test();
    let mut context: ProgramTestContext = program_test.start_with_context().await;

    // Initialize treasury/system using existing helper so SystemState exists
    let current_admin = Keypair::new(); // will be the admin authority at init
    initialize_treasury_system(
        &mut context.banks_client,
        &context.payer,
        context.last_blockhash,
        &current_admin,
    ).await?;

    let program_id = fixed_ratio_trading::id();
    let system_state_pda = get_system_state_pda(&program_id);
    let program_data_address = get_test_program_data_address(&program_id);

    // Verify initial admin in SystemState
    if let Some(account) = context.banks_client.get_account(system_state_pda).await? {
        let state = SystemState::from_account_data_unchecked(&account.data)?;
        assert_eq!(state.admin_authority, current_admin.pubkey(), "Initial admin should match initializer");
        assert!(!state.is_paused, "System should start unpaused");
    } else {
        panic!("SystemState account must exist after initialization");
    }

    // 1) Propose a new admin (starts timelock)
    let new_admin = Keypair::new();
    let initiate_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(current_admin.pubkey(), true),
            AccountMeta::new(system_state_pda, false),
            AccountMeta::new_readonly(program_data_address, false),
        ],
        data: PoolInstruction::ProcessAdminChange { new_admin: new_admin.pubkey() }.try_to_vec()?,
    };

    let mut tx = Transaction::new_with_payer(&[initiate_ix], Some(&context.payer.pubkey()));
    tx.sign(&[&context.payer, &current_admin], context.last_blockhash);
    context.banks_client.process_transaction(tx).await?;

    // 2) Verify pending admin set
    if let Some(account) = context.banks_client.get_account(system_state_pda).await? {
        let state = SystemState::from_account_data_unchecked(&account.data)?;
        assert_eq!(state.pending_admin_authority, Some(new_admin.pubkey()), "Pending admin should be set to proposed new admin");
    } else {
        panic!("SystemState account must exist after initiation");
    }

    // 3) Advance on-chain clock beyond timelock using test sysvar override
    let account = context
        .banks_client
        .get_account(system_state_pda)
        .await?
        .expect("SystemState should exist");
    let state = SystemState::from_account_data_unchecked(&account.data)?;
    let mut clock = context.banks_client.get_sysvar::<Clock>().await?;
    
    // Add extra buffer time to ensure timelock is definitely satisfied
    let buffer_time = 3600; // 1 hour buffer to prevent race conditions
    clock.unix_timestamp = state.admin_change_timestamp + SystemState::ADMIN_CHANGE_TIMELOCK + buffer_time;
    context.set_sysvar(&clock);
    
    // Wait a moment for sysvar propagation to ensure consistency
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // 4) Complete the admin change
    let bh_complete = context.banks_client.get_latest_blockhash().await?;
    let complete_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(current_admin.pubkey(), true),
            AccountMeta::new(system_state_pda, false),
            AccountMeta::new_readonly(program_data_address, false),
        ],
        data: PoolInstruction::ProcessAdminChange { new_admin: new_admin.pubkey() }.try_to_vec()?,
    };
    let mut complete_tx = Transaction::new_with_payer(&[complete_ix], Some(&context.payer.pubkey()));
    complete_tx.sign(&[&context.payer, &current_admin], bh_complete);
    
    // Process transaction and verify it succeeded
    let tx_result = context.banks_client.process_transaction(complete_tx).await;
    if let Err(e) = &tx_result {
        // If transaction failed, get current state for debugging
        if let Some(debug_account) = context.banks_client.get_account(system_state_pda).await? {
            let debug_state = SystemState::from_account_data_unchecked(&debug_account.data)?;
            eprintln!("❌ Admin change transaction failed: {:?}", e);
            eprintln!("🔍 Current SystemState:");
            eprintln!("   - Current admin: {}", debug_state.admin_authority);
            eprintln!("   - Pending admin: {:?}", debug_state.pending_admin_authority);
            eprintln!("   - Admin change timestamp: {}", debug_state.admin_change_timestamp);
            eprintln!("   - Expected new admin: {}", new_admin.pubkey());
        }
    }
    tx_result?;

    // Verify admin changed
    if let Some(account) = context.banks_client.get_account(system_state_pda).await? {
        let state = SystemState::from_account_data_unchecked(&account.data)?;
        
        // Detailed debugging for assertion failure
        if state.admin_authority != new_admin.pubkey() {
            eprintln!("❌ ADMIN CHANGE VERIFICATION FAILED:");
            eprintln!("   - Expected new admin: {}", new_admin.pubkey());
            eprintln!("   - Actual current admin: {}", state.admin_authority);
            eprintln!("   - Pending admin: {:?}", state.pending_admin_authority);
            eprintln!("   - Admin change timestamp: {}", state.admin_change_timestamp);
            eprintln!("   - System paused: {}", state.is_paused);
            
            // Get current clock for additional debugging
            let current_clock = context.banks_client.get_sysvar::<Clock>().await?;
            eprintln!("   - Current timestamp: {}", current_clock.unix_timestamp);
            eprintln!("   - Timelock duration: {} seconds", SystemState::ADMIN_CHANGE_TIMELOCK);
            
            if let Some(pending) = state.pending_admin_authority {
                let time_elapsed = current_clock.unix_timestamp - state.admin_change_timestamp;
                eprintln!("   - Time elapsed since initiation: {} seconds", time_elapsed);
                eprintln!("   - Timelock satisfied: {}", time_elapsed >= SystemState::ADMIN_CHANGE_TIMELOCK);
            }
        }
        
        assert_eq!(state.admin_authority, new_admin.pubkey(), "Admin should be transferred to new_admin after timelock completion");
        assert!(state.pending_admin_authority.is_none(), "Pending admin should be cleared after completion");
    } else {
        panic!("SystemState account must exist after admin change");
    }

    // 5) New admin executes PauseSystem successfully (admin-only)
    let pause_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(new_admin.pubkey(), true), // new admin signer
            AccountMeta::new(system_state_pda, false),   // writable
            AccountMeta::new_readonly(program_data_address, false),
        ],
        data: PoolInstruction::PauseSystem { reason_code: 1 }.try_to_vec()?,
    };

    let recent_blockhash2 = context.banks_client.get_latest_blockhash().await?;
    let mut pause_tx = Transaction::new_with_payer(&[pause_ix], Some(&context.payer.pubkey()));
    pause_tx.sign(&[&context.payer, &new_admin], recent_blockhash2);
    context.banks_client.process_transaction(pause_tx).await?;

    // Verify paused
    if let Some(account) = context.banks_client.get_account(system_state_pda).await? {
        let state = SystemState::from_account_data_unchecked(&account.data)?;
        assert!(state.is_paused, "System should be paused by new admin");
        assert_eq!(state.pause_reason_code, 1, "Pause reason code should match");
    } else {
        panic!("SystemState account must exist after pause");
    }

    Ok(())
}


