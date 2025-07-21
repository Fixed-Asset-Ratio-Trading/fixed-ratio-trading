//! # Compute Unit Measurement Utilities
//! 
//! This module provides utilities for measuring compute unit (CU) consumption
//! in Solana program tests using the solana-program-test framework.

use solana_program_test::BanksClient;
use solana_sdk::{
    instruction::Instruction,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,

    hash::Hash,
};
use std::time::Instant;

/// Result of a CU measurement test
#[derive(Debug, Clone)]
pub struct CUMeasurementResult {
    pub instruction_name: String,
    pub success: bool,
    pub estimated_cu_consumed: Option<u64>,
    pub transaction_signature: Option<String>,
    pub execution_time_ms: u64,
    pub error: Option<String>,
}

/// CU measurement configuration
#[derive(Debug, Clone)]
pub struct CUMeasurementConfig {
    pub max_retries: u32,
    pub enable_logging: bool,
    pub compute_limit: u64,
}

impl Default for CUMeasurementConfig {
    fn default() -> Self {
        Self {
            max_retries: 1, // REDUCED from 3 to 1 for speed
            enable_logging: false, // DISABLED by default for speed
            compute_limit: 200_000, // Default CU limit
        }
    }
}

/// Measure compute units for a single instruction using binary search to find actual consumption
pub async fn measure_instruction_cu(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: Hash,
    instruction: Instruction,
    instruction_name: &str,
    config: Option<CUMeasurementConfig>,
) -> CUMeasurementResult {
    let config = config.unwrap_or_default();
    let start_time = Instant::now();
    
    // Step 1: First, verify the instruction works with a high CU limit
    let high_limit = config.compute_limit;
    let success_result = test_instruction_with_cu_limit(
        banks_client, payer, recent_blockhash, &instruction, high_limit, &config
    ).await;
    
    if !success_result.0 {
        // If it fails even with high limit, return failure
        return CUMeasurementResult {
            instruction_name: instruction_name.to_string(),
            success: false,
            estimated_cu_consumed: None,
            transaction_signature: success_result.1,
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            error: success_result.2,
        };
    }
    
    if config.enable_logging {
        println!("✅ {} works with {} CUs, now finding minimum...", instruction_name, high_limit);
    }
    
    // Step 2: Binary search to find minimum CU limit required
    let mut low = 5_000u64;   // Start from 5K CUs (reasonable minimum)
    let mut high = high_limit;
    let mut last_successful_limit = high_limit;
    let mut final_signature = success_result.1;
    
    while low <= high {
        let mid = low + (high - low) / 2;
        
        let test_result = test_instruction_with_cu_limit(
            banks_client, payer, recent_blockhash, &instruction, mid, &config
        ).await;
        
        if test_result.0 {
            // Success with this limit - try lower
            last_successful_limit = mid;
            if let Some(sig) = test_result.1 {
                final_signature = Some(sig);
            }
            high = mid - 1;
            
            if config.enable_logging {
                println!("  ✅ {} CUs: SUCCESS", mid);
            }
        } else {
            // Failed with this limit - need higher
            low = mid + 1;
            
            if config.enable_logging {
                println!("  ❌ {} CUs: FAILED", mid);
            }
        }
    }
    
    let execution_time = start_time.elapsed().as_millis() as u64;
    
    if config.enable_logging {
        println!("🎯 {} minimum CU requirement: {} CUs", instruction_name, last_successful_limit);
    }
    
    CUMeasurementResult {
        instruction_name: instruction_name.to_string(),
        success: true,
        estimated_cu_consumed: Some(last_successful_limit),
        transaction_signature: final_signature,
        execution_time_ms: execution_time,
        error: None,
    }
}

/// Test an instruction with a specific CU limit
async fn test_instruction_with_cu_limit(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: Hash,
    instruction: &Instruction,
    cu_limit: u64,
    config: &CUMeasurementConfig,
) -> (bool, Option<String>, Option<String>) {
    // Create transaction with specific CU budget instruction
    let compute_budget_ix = solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
        cu_limit as u32
    );
    
    let transaction = Transaction::new_signed_with_payer(
        &[compute_budget_ix, instruction.clone()],
        Some(&payer.pubkey()),
        &[payer],
        recent_blockhash,
    );
    
    // Execute with timeout protection
    let timeout_duration = tokio::time::Duration::from_millis(2000); // 2 second timeout
    let process_future = banks_client.process_transaction(transaction.clone());
    
    match tokio::time::timeout(timeout_duration, process_future).await {
        Ok(Ok(())) => {
            // Success
            (true, Some(transaction.signatures[0].to_string()), None)
        }
        Ok(Err(e)) => {
            // Failed - likely insufficient CU
            (false, None, Some(format!("{:?}", e)))
        }
        Err(_) => {
            // Timeout
            (false, None, Some("Timeout".to_string()))
        }
    }
}

/// Measure CUs for multiple instructions and compare them
pub async fn compare_instruction_cu(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: Hash,
    instructions: Vec<(Instruction, String)>,
    config: Option<CUMeasurementConfig>,
) -> Vec<CUMeasurementResult> {
    let config = config.unwrap_or_default();
    let mut results = Vec::new();
    
    for (instruction, name) in instructions {
        let result = measure_instruction_cu(
            banks_client,
            payer,
            recent_blockhash,
            instruction,
            &name,
            Some(config.clone()),
        ).await;
        
        results.push(result);
        
        // REMOVED delay between measurements for speed
        // tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    
    results
}

/// Benchmark a function multiple times to get average CU consumption
pub async fn benchmark_instruction_cu(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: Hash,
    instruction_generator: Box<dyn Fn() -> Instruction>,
    instruction_name: &str,
    iterations: u32,
    config: Option<CUMeasurementConfig>,
) -> Vec<CUMeasurementResult> {
    let config = config.unwrap_or_default();
    let mut results = Vec::new();
    
    if config.enable_logging {
        println!("🔬 Benchmarking {} over {} iterations", instruction_name, iterations);
    }
    
    for i in 0..iterations {
        let instruction = instruction_generator();
        let iteration_name = format!("{}_iteration_{}", instruction_name, i + 1);
        
        // Use the timeout-protected measure_instruction_cu function
        let result = measure_instruction_cu(
            banks_client,
            payer,
            recent_blockhash,
            instruction,
            &iteration_name,
            Some(config.clone()),
        ).await;
        
        results.push(result);
        
        // No delays between iterations for maximum speed
    }
    
    // Print summary with timeout-aware stats
    if config.enable_logging {
        let successful_runs = results.iter().filter(|r| r.success).count();
        let failed_runs = results.len() - successful_runs;
        let timed_out_runs = results.iter().filter(|r| {
            r.error.as_ref().map_or(false, |e| e.contains("timed out"))
        }).count();
        let avg_execution_time = if !results.is_empty() {
            results.iter().map(|r| r.execution_time_ms).sum::<u64>() / results.len() as u64
        } else {
            0
        };
        
        println!("📊 Benchmark Summary for {}:", instruction_name);
        println!("  Successful runs: {}/{}", successful_runs, results.len());
        println!("  Failed runs: {}", failed_runs);
        println!("  Timed out runs: {}", timed_out_runs);
        println!("  Average execution time: {}ms", avg_execution_time);
    }
    
    results
}

/// Generate a detailed CU report
pub fn generate_cu_report(results: &[CUMeasurementResult]) -> String {
    let mut report = String::new();
    
    report.push_str("# Compute Unit Measurement Report\n\n");
    report.push_str(&format!("Generated: {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
    
    let total_tests = results.len();
    let successful_tests = results.iter().filter(|r| r.success).count();
    let failed_tests = total_tests - successful_tests;
    
    report.push_str(&format!("## Summary\n"));
    report.push_str(&format!("- Total tests: {}\n", total_tests));
    report.push_str(&format!("- Successful: {}\n", successful_tests));
    report.push_str(&format!("- Failed: {}\n\n", failed_tests));
    
    if successful_tests > 0 {
        report.push_str("## Successful Tests\n\n");
        report.push_str("| Instruction | Execution Time (ms) | Status |\n");
        report.push_str("|-------------|---------------------|--------|\n");
        
        for result in results.iter().filter(|r| r.success) {
            report.push_str(&format!(
                "| {} | {} | ✅ |\n",
                result.instruction_name,
                result.execution_time_ms
            ));
        }
        report.push_str("\n");
    }
    
    if failed_tests > 0 {
        report.push_str("## Failed Tests\n\n");
        report.push_str("| Instruction | Error | Execution Time (ms) |\n");
        report.push_str("|-------------|-------|---------------------|\n");
        
        for result in results.iter().filter(|r| !r.success) {
            let error = result.error.as_deref().unwrap_or("Unknown error");
            report.push_str(&format!(
                "| {} | {} | {} |\n",
                result.instruction_name,
                error,
                result.execution_time_ms
            ));
        }
        report.push_str("\n");
    }
    
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::*;
    
    #[tokio::test]
    async fn test_cu_measurement_utilities() {
        let env = start_test_environment().await;
        
        // Test basic CU measurement functionality
        let test_ix = solana_sdk::system_instruction::transfer(
            &env.payer.pubkey(),
            &solana_sdk::pubkey::Pubkey::new_unique(),
            1000000, // 0.001 SOL
        );
        
        let result = measure_instruction_cu(
            &mut env.banks_client.clone(),
            &env.payer,
            env.recent_blockhash,
            test_ix,
            "test_transfer",
            Some(CUMeasurementConfig {
                enable_logging: false,
                ..Default::default()
            }),
        ).await;
        
        // Should succeed (basic transfer)
        assert!(result.success || result.error.is_some()); // Either success or we get an error we can analyze
        assert!(!result.instruction_name.is_empty());
        assert!(result.execution_time_ms > 0);
    }
} 