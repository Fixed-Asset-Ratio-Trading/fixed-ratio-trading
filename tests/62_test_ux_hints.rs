mod common;

use common::*;
use solana_program_test::BanksClientError;

#[tokio::test]
async fn test_optimized_pool_creation_with_ux_hints() -> TestResult {
    println!("🧪 Testing optimized pool creation with UX hints...");
    
    // Setup test environment
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create ordered token mints
    let keypair1 = Keypair::new();
    let keypair2 = Keypair::new();
    
    let (primary_mint, base_mint) = if keypair1.pubkey() < keypair2.pubkey() {
        (keypair1, keypair2)
    } else {
        (keypair2, keypair1)
    };
    
    // Initialize treasury system (required first)
    init_treasury_for_test(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
    ).await?;
    
    // Create token mints
    create_test_mints(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &[&primary_mint, &base_mint],
    ).await?;
    
    // Test pool creation with optimized UX hints
    let ratio_a_numerator = 1;
    let ratio_b_denominator = 2;
    
    println!("🔨 Creating pool with ratio {}:{}", ratio_a_numerator, ratio_b_denominator);
    
    let config = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &primary_mint,
        &base_mint,
        Some(ratio_a_numerator),
    ).await?;
    
    println!("✅ Pool created successfully!");
    println!("   Pool State: {}", config.pool_state_pda);
    println!("   Token A: {}", primary_mint.pubkey());
    println!("   Token B: {}", base_mint.pubkey());
    println!("   Ratio: {} : {}", ratio_a_numerator, ratio_b_denominator);
    
    // Verify pool state was created correctly
    verify_pool_state(
        &mut ctx.env.banks_client,
        &config,
        &ctx.env.payer.pubkey(),
        &ctx.lp_token_a_mint.pubkey(),
        &ctx.lp_token_b_mint.pubkey(),
    ).await.map_err(|e| BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    
    println!("✅ Pool state verification passed!");
    Ok(())
}

#[tokio::test]
async fn test_pool_creation_ux_messages() -> TestResult {
    println!("🧪 Testing pool creation UX messages...");
    
    // Setup test environment
    let mut ctx = setup_pool_test_context(false).await;
    
    // Create ordered token mints
    let keypair1 = Keypair::new();
    let keypair2 = Keypair::new();
    
    let (primary_mint, base_mint) = if keypair1.pubkey() < keypair2.pubkey() {
        (keypair1, keypair2)
    } else {
        (keypair2, keypair1)
    };
    
    // Initialize treasury system (required first)
    init_treasury_for_test(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
    ).await?;
    
    // Create token mints
    create_test_mints(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &[&primary_mint, &base_mint],
    ).await?;
    
    // Test pool creation with UX messages
    let config = create_pool_new_pattern(
        &mut ctx.env.banks_client,
        &ctx.env.payer,
        ctx.env.recent_blockhash,
        &primary_mint,
        &base_mint,
        Some(1),
    ).await?;
    
    println!("✅ Pool creation with UX messages completed!");
    println!("   Pool: {}", config.pool_state_pda);
    
    // Verify the pool exists
    verify_pool_state(
        &mut ctx.env.banks_client,
        &config,
        &ctx.env.payer.pubkey(),
        &ctx.lp_token_a_mint.pubkey(),
        &ctx.lp_token_b_mint.pubkey(),
    ).await.map_err(|e| BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    
    Ok(())
}

/// Helper function to convert treasury system initialization errors to BanksClientError
async fn init_treasury_for_test(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: solana_sdk::hash::Hash,
) -> Result<(), BanksClientError> {
    // ✅ PHASE 11 SECURITY: Use test program authority for treasury initialization
    use crate::common::setup::{create_test_program_authority_keypair, verify_test_program_authority_consistency};
    
    // Create keypair that matches the test program authority
    let system_authority = create_test_program_authority_keypair()
        .map_err(|e| BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, 
            format!("Failed to create program authority keypair: {}", e))))?;
    
    // Verify the loaded keypair matches the expected authority
    verify_test_program_authority_consistency(&system_authority)
        .map_err(|e| BanksClientError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData, e)))?;
    
    println!("🔐 Using test program authority for testing: {}", system_authority.pubkey());
    
    initialize_treasury_system(banks_client, payer, recent_blockhash, &system_authority)
        .await
        .map_err(|e| {
            let error_msg = format!("Treasury system initialization error: {:?}", e);
            println!("{}", error_msg);
            BanksClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, error_msg))
        })
} 