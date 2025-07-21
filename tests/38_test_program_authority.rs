//! Program Authority Utilities Tests
//! 
//! This module tests the program authority validation and derivation utilities
//! which are critical for security in the Fixed Ratio Trading protocol.

use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    bpf_loader_upgradeable,
};

mod common;
use common::*;

use fixed_ratio_trading::utils::program_authority::get_program_data_address;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Test the get_program_data_address function - comprehensive coverage
#[test]
fn test_get_program_data_address() -> TestResult {
    println!("🧪 Testing program data address derivation...");
    
    // Test with system program ID
    let system_program_id = solana_program::system_program::id();
    let program_data_address = get_program_data_address(&system_program_id);
    
    // Verify it's a valid PDA
    let (expected_pda, _bump) = Pubkey::find_program_address(
        &[system_program_id.as_ref()], 
        &bpf_loader_upgradeable::id()
    );
    
    assert_eq!(program_data_address, expected_pda, 
        "Program data address should match PDA derivation");
    
    println!("✅ Program data address: {}", program_data_address);
    println!("✅ Matches expected PDA: {}", expected_pda);
    
    // Test with multiple different program IDs - use distinct hardcoded IDs to avoid collisions
    let program_ids = vec![
        solana_program::system_program::id(),
        Pubkey::from([1u8; 32]),   // Distinct hardcoded ID
        Pubkey::from([2u8; 32]),   // Distinct hardcoded ID  
        Pubkey::from([3u8; 32]),   // Different from default
        Pubkey::from([255u8; 32]), // Max value
    ];
    
    let mut data_addresses = Vec::new();
    for program_id in &program_ids {
        let data_address = get_program_data_address(program_id);
        
        // Verify no duplicates
        assert!(!data_addresses.contains(&data_address),
            "Each program should have a unique data address");
        
        data_addresses.push(data_address);
        
        // Verify deterministic
        let data_address2 = get_program_data_address(program_id);
        assert_eq!(data_address, data_address2,
            "Address derivation should be deterministic");
    }
    
    println!("✅ Generated {} unique data addresses for {} programs", 
        data_addresses.len(), program_ids.len());
    
    // Test PDA derivation properties
    for (i, program_id) in program_ids.iter().enumerate() {
        let data_address = data_addresses[i];
        let (expected_pda, _bump) = Pubkey::find_program_address(
            &[program_id.as_ref()],
            &bpf_loader_upgradeable::id()
        );
        
        assert_eq!(data_address, expected_pda,
            "Derived address should match PDA calculation");
        
        // Verify PDA properties
        // Bump is always valid by design, so no need to check
        assert!(expected_pda.is_on_curve() == false, 
            "PDA should be off the curve");
    }
    
    println!("✅ All program authority scenarios working correctly");
    Ok(())
}

/// Test program authority consistency validation
#[test] 
fn test_program_authority_consistency() -> TestResult {
    println!("🧪 Testing program authority consistency validation...");
    
    // Test the hardcoded test authority
    let authority_keypair = create_test_program_authority_keypair()?;
    verify_test_program_authority_consistency(&authority_keypair)?;
    
    println!("✅ Test program authority consistency verified");
    
    // Test with invalid keypair should fail
    let invalid_keypair = Keypair::new();
    let consistency_result = verify_test_program_authority_consistency(&invalid_keypair);
    assert!(consistency_result.is_err(), 
        "Invalid keypair should fail consistency check");
    
    println!("✅ Invalid authority properly rejected");
    
    // Test multiple calls for consistency
    let authority_keypair2 = create_test_program_authority_keypair()?;
    assert_eq!(authority_keypair.pubkey(), authority_keypair2.pubkey(),
        "Multiple calls should return the same authority");
    
    println!("✅ Authority derivation is consistent across calls");
    Ok(())
}

/// Test authority creation and verification comprehensive scenarios
#[test]
fn test_program_authority_comprehensive() -> TestResult {
    println!("🧪 Testing comprehensive program authority scenarios...");
    
    // Test authority creation
    let authority1 = create_test_program_authority_keypair()?;
    let authority2 = create_test_program_authority_keypair()?;
    
    // Authorities should be identical (same hardcoded keypair)
    assert_eq!(authority1.pubkey(), authority2.pubkey(),
        "All created authorities should be identical");
    
    // Test with multiple different program IDs - use distinct hardcoded IDs to avoid collisions
    let program_ids = vec![
        solana_program::system_program::id(),
        Pubkey::from([1u8; 32]),   // Distinct hardcoded ID
        Pubkey::from([2u8; 32]),   // Distinct hardcoded ID  
        Pubkey::from([3u8; 32]),   // Different from default
        Pubkey::from([255u8; 32]), // Max value
    ];
    
    let mut data_addresses = Vec::new();
    for (index, program_id) in program_ids.iter().enumerate() {
        let data_address = get_program_data_address(program_id);
        
        println!("Program {}: {} -> {}", index, program_id, data_address);
        
        // Verify no duplicates
        if data_addresses.contains(&data_address) {
            println!("❌ Collision detected! Address {} already exists", data_address);
            for (i, existing_addr) in data_addresses.iter().enumerate() {
                println!("  Existing[{}]: {}", i, existing_addr);
            }
        }
        assert!(!data_addresses.contains(&data_address),
            "Each program should have a unique data address");
        
        data_addresses.push(data_address);
        
        // Verify deterministic
        let data_address2 = get_program_data_address(program_id);
        assert_eq!(data_address, data_address2,
            "Address derivation should be deterministic");
    }
    
    println!("✅ Generated {} unique data addresses for {} programs", 
        data_addresses.len(), program_ids.len());
    
    // Test PDA derivation properties
    for (i, program_id) in program_ids.iter().enumerate() {
        let data_address = data_addresses[i];
        let (expected_pda, _bump) = Pubkey::find_program_address(
            &[program_id.as_ref()],
            &bpf_loader_upgradeable::id()
        );
        
        assert_eq!(data_address, expected_pda,
            "Derived address should match PDA calculation");
        
        // Verify PDA properties
        // Bump is always valid by design, so no need to check
        assert!(expected_pda.is_on_curve() == false, 
            "PDA should be off the curve");
    }
    
    println!("✅ All program authority scenarios working correctly");
    Ok(())
} 