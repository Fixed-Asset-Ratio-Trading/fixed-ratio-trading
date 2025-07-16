//! # Phase 1 Fee Validation Tests
//! 
//! This module validates the Phase 1 fee validation framework improvements including:
//! - Fees first pattern implementation
//! - Atomic fee collection
//! - Pre-flight validation
//! - Post-transfer validation
//! - Error handling for insufficient funds

use solana_sdk::{
    account::Account,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
};
use fixed_ratio_trading::{
    error::PoolError,
    utils::fee_validation::{
        validate_fee_payment,
        validate_treasury_account,
    },
    constants::*,
};
use solana_program::{
    account_info::AccountInfo,
    program_error::ProgramError,
};

mod common;
use common::*;

/// Test pre-flight fee validation with sufficient funds
#[tokio::test]
async fn test_fee_validation_sufficient_funds() {
    println!("🔍 Testing fee validation with sufficient funds");
    
    // Create a mock account with sufficient balance
    let payer_keypair = Keypair::new();
    let mut payer_lamports = 10_000_000_000u64; // 10 SOL
    let mut payer_account = Account {
        lamports: payer_lamports,
        data: vec![],
        owner: system_program::id(),
        executable: false,
        rent_epoch: 0,
    };
    
    let payer_pubkey = payer_keypair.pubkey();
    let system_program_id = system_program::id();
    let payer_info = AccountInfo::new(
        &payer_pubkey,
        true,
        true,
        &mut payer_lamports,
        &mut payer_account.data,
        &system_program_id,
        false,
        0,
    );
    
    // Test various fee amounts
    let test_cases = vec![
        (REGISTRATION_FEE, VALIDATION_CONTEXT_POOL_CREATION),
        (DEPOSIT_WITHDRAWAL_FEE, VALIDATION_CONTEXT_LIQUIDITY),
        (SWAP_FEE, VALIDATION_CONTEXT_SWAP),
        (SWAP_FEE, VALIDATION_CONTEXT_SWAP),
    ];
    
    for (fee_amount, validation_context_code) in test_cases {
        let result = validate_fee_payment(&payer_info, fee_amount, validation_context_code);
        
        assert!(result.is_valid, "Fee validation should pass for context code {}", validation_context_code);
        assert_eq!(result.required_amount, fee_amount);
        assert_eq!(result.available_balance, 10_000_000_000u64);
        assert!(result.error_message.is_none());
        
        println!("✅ Context {} fee validation passed", validation_context_code);
    }
}

/// Test pre-flight fee validation with insufficient funds
#[tokio::test]
async fn test_fee_validation_insufficient_funds() {
    println!("🔍 Testing fee validation with insufficient funds");
    
    // Create a mock account with insufficient balance
    let payer_keypair = Keypair::new();
    let mut payer_lamports = 1000u64; // Very small amount
    let mut payer_account = Account {
        lamports: payer_lamports,
        data: vec![],
        owner: system_program::id(),
        executable: false,
        rent_epoch: 0,
    };
    
    let payer_pubkey = payer_keypair.pubkey();
    let system_program_id = system_program::id();
    let payer_info = AccountInfo::new(
        &payer_pubkey,
        true,
        true,
        &mut payer_lamports,
        &mut payer_account.data,
        &system_program_id,
        false,
        0,
    );
    
    // Test validation with insufficient funds
    let result = validate_fee_payment(&payer_info, REGISTRATION_FEE, VALIDATION_CONTEXT_POOL_CREATION);
    
    assert!(!result.is_valid, "Fee validation should fail with insufficient funds");
    assert_eq!(result.required_amount, REGISTRATION_FEE);
    assert_eq!(result.available_balance, 1000u64);
    assert!(result.error_message.is_some());
    
    let error_msg = result.error_message.unwrap();
    assert!(error_msg.contains("Insufficient balance"));
    assert!(error_msg.contains("1"));  // Context code 1 for pool creation
    
    println!("✅ Insufficient funds validation failed as expected");
}

/// Test treasury account validation with correct PDA
#[tokio::test]
async fn test_treasury_account_validation_success() {
    println!("🔍 Testing treasury account validation with correct PDA");
    
    // Create a mock treasury account
    let treasury_keypair = Keypair::new();
    let mut treasury_lamports = 1000000u64;
    let mut treasury_account = Account {
        lamports: treasury_lamports,
        data: vec![],
        owner: PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };
    
    let treasury_pubkey = treasury_keypair.pubkey();
    let treasury_info = AccountInfo::new(
        &treasury_pubkey,
        false,
        true, // writable
        &mut treasury_lamports,
        &mut treasury_account.data,
        &PROGRAM_ID,
        false,
        0,
    );
    
    // Test validation with matching PDA
    let result = validate_treasury_account(
        &treasury_info,
        &treasury_pubkey,
        TREASURY_TYPE_MAIN,
    );
    
    assert!(result.is_ok(), "Treasury validation should pass with matching PDA");
    
    println!("✅ Treasury account validation passed");
}

/// Test treasury account validation with incorrect PDA
#[tokio::test]
async fn test_treasury_account_validation_failure() {
    println!("🔍 Testing treasury account validation with incorrect PDA");
    
    // Create a mock treasury account
    let treasury_keypair = Keypair::new();
    let different_keypair = Keypair::new();
    let mut treasury_lamports = 1000000u64;
    let mut treasury_account = Account {
        lamports: treasury_lamports,
        data: vec![],
        owner: PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };
    
    let treasury_pubkey = treasury_keypair.pubkey();
    let treasury_info = AccountInfo::new(
        &treasury_pubkey,
        false,
        true, // writable
        &mut treasury_lamports,
        &mut treasury_account.data,
        &PROGRAM_ID,
        false,
        0,
    );
    
    // Test validation with non-matching PDA
    let different_pubkey = different_keypair.pubkey();
    let result = validate_treasury_account(
        &treasury_info,
        &different_pubkey, // Different pubkey
        TREASURY_TYPE_MAIN,
    );
    
    assert!(result.is_err(), "Treasury validation should fail with wrong PDA");
    
    match result.unwrap_err() {
        ProgramError::Custom(code) => {
            assert_eq!(code, 1033); // TreasuryValidationFailed error code
        }
        _ => panic!("Expected TreasuryValidationFailed error"),
    }
    
    println!("✅ Treasury account validation failed as expected");
}

/// Test fee collection workflow simulation
#[tokio::test]
async fn test_fee_collection_workflow() {
    println!("🔍 Testing complete fee collection workflow");
    
    // Test data structure to track results
    struct FeeTestCase {
        name: &'static str,
        fee_amount: u64,
        initial_balance: u64,
        should_succeed: bool,
        validation_context_code: u8,
    }
    
    let test_cases = vec![
        FeeTestCase {
            name: "Pool Creation (sufficient funds)",
            fee_amount: REGISTRATION_FEE,
            initial_balance: 10_000_000_000u64, // 10 SOL
            should_succeed: true,
            validation_context_code: VALIDATION_CONTEXT_POOL_CREATION,
        },
        FeeTestCase {
            name: "Liquidity Operation (sufficient funds)",
            fee_amount: DEPOSIT_WITHDRAWAL_FEE,
            initial_balance: 10_000_000u64, // 0.01 SOL
            should_succeed: true,
            validation_context_code: VALIDATION_CONTEXT_LIQUIDITY,
        },
        FeeTestCase {
            name: "Regular Swap (sufficient funds)",
            fee_amount: SWAP_FEE,
            initial_balance: 1_000_000u64, // 0.001 SOL
            should_succeed: true,
            validation_context_code: VALIDATION_CONTEXT_SWAP,
        },
        FeeTestCase {
                    name: "Swap (sufficient funds)",
        fee_amount: SWAP_FEE,
            initial_balance: 100_000u64, // 0.0001 SOL
            should_succeed: true,
            validation_context_code: VALIDATION_CONTEXT_SWAP,
        },
        FeeTestCase {
            name: "Pool Creation (insufficient funds)",
            fee_amount: REGISTRATION_FEE,
            initial_balance: 1000u64, // Very small amount
            should_succeed: false,
            validation_context_code: VALIDATION_CONTEXT_POOL_CREATION,
        },
    ];
    
    for test_case in test_cases {
        println!("Testing: {}", test_case.name);
        
        // Create payer account
        let payer_keypair = Keypair::new();
        let mut payer_lamports = test_case.initial_balance;
        let mut payer_account = Account {
            lamports: payer_lamports,
            data: vec![],
            owner: system_program::id(),
            executable: false,
            rent_epoch: 0,
        };
        
        let payer_pubkey = payer_keypair.pubkey();
        let system_program_id = system_program::id();
        let payer_info = AccountInfo::new(
            &payer_pubkey,
            true,
            true,
            &mut payer_lamports,
            &mut payer_account.data,
            &system_program_id,
            false,
            0,
        );
        
        // Validate fee payment
        let validation_result = validate_fee_payment(&payer_info, test_case.fee_amount, test_case.validation_context_code);
        
        if test_case.should_succeed {
            assert!(validation_result.is_valid, "Fee validation should pass for {}", test_case.name);
            assert_eq!(validation_result.required_amount, test_case.fee_amount);
            assert_eq!(validation_result.available_balance, test_case.initial_balance);
            println!("  ✅ Fee validation passed");
        } else {
            assert!(!validation_result.is_valid, "Fee validation should fail for {}", test_case.name);
            assert!(validation_result.error_message.is_some());
            println!("  ✅ Fee validation failed as expected");
        }
    }
}

/// Test fee amounts are correct
#[tokio::test]
async fn test_fee_amounts_consistency() {
    println!("🔍 Testing fee amounts consistency");
    
    // Verify fee amounts match constants
    assert_eq!(REGISTRATION_FEE, 1_150_000_000u64, "Pool creation fee should be 1.15 SOL");
    assert_eq!(DEPOSIT_WITHDRAWAL_FEE, 1_300_000u64, "Liquidity fee should be 0.0013 SOL");
    assert_eq!(SWAP_FEE, 27_150u64, "Regular swap fee should be 0.00002715 SOL");
        assert_eq!(SWAP_FEE, 27_150u64, "Swap fee should be 0.00002715 SOL");
    
    // Verify fee relationships
    assert!(REGISTRATION_FEE > DEPOSIT_WITHDRAWAL_FEE, "Pool creation fee should be higher than liquidity fee");
    assert!(DEPOSIT_WITHDRAWAL_FEE > SWAP_FEE, "Liquidity fee should be higher than swap fee");

    
    println!("✅ All fee amounts are consistent");
}

/// Test error code mappings
#[tokio::test]
async fn test_error_code_mappings() {
    println!("🔍 Testing error code mappings");
    
    // Test that new error types have correct codes
    let insufficient_fee_error = PoolError::InsufficientFeeBalance {
        required: 1000,
        available: 500,
        account: Pubkey::new_unique(),
    };
    assert_eq!(insufficient_fee_error.error_code(), 1030);
    
    let fee_collection_error = PoolError::FeeCollectionFailed {
        expected: 1000,
        collected: 500,
        fee_type: "Test".to_string(),
    };
    assert_eq!(fee_collection_error.error_code(), 1031);
    
    let fee_validation_error = PoolError::FeeValidationFailed {
        reason: "Test reason".to_string(),
    };
    assert_eq!(fee_validation_error.error_code(), 1032);
    
    let treasury_validation_error = PoolError::TreasuryValidationFailed {
        expected: Pubkey::new_unique(),
        provided: Pubkey::new_unique(),
        treasury_type: "Test".to_string(),
    };
    assert_eq!(treasury_validation_error.error_code(), 1033);
    
    println!("✅ All error codes are correctly mapped");
}



/// Helper function to create mock account info for testing
fn create_mock_account_info() -> AccountInfo<'static> {
    static mut LAMPORTS: u64 = 500; // Insufficient for most fees
    static mut DATA: Vec<u8> = Vec::new();
    static PUBKEY: Pubkey = Pubkey::new_from_array([0; 32]);
    static SYSTEM_PROGRAM_ID: Pubkey = system_program::ID;
    
    unsafe {
        AccountInfo::new(
            &PUBKEY,
            false,
            false,
            &mut LAMPORTS,
            &mut DATA,
            &SYSTEM_PROGRAM_ID,
            false,
            0,
        )
    }
} 