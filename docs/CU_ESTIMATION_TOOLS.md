# Compute Unit (CU) Estimation Tools

## Overview

This document provides a comprehensive guide to all available tools for estimating compute units (CUs) in the Fixed Ratio Trading system. There are several approaches available, ranging from simple testing tools to production-ready measurement systems.

## 📊 Current CU Estimates (Already Documented)

Your codebase already contains documented CU estimates for major operations:

### Pool Operations
- **Pool Creation**: 45,000 - 50,000 CUs
- **Liquidity Deposit**: 35,000 - 40,000 CUs  
- **Liquidity Withdrawal**: 30,000 - 35,000 CUs
- **Regular Swap**: 18,000 - 23,000 CUs
- **HFT Optimized Swap**: 13,000 - 16,000 CUs (35-40% savings)

### Treasury Operations
- **Treasury Info**: Low CU (view operation)
- **Treasury Withdrawal**: 420-840 CUs savings from optimization

## 🛠️ Available CU Estimation Tools

### 1. **Custom CU Measurement Framework (NEW)**

Location: `tests/common/cu_measurement.rs`

#### Features:
- **Instruction Measurement**: Measure individual instructions
- **Comparison Testing**: Compare CUs between different instruction versions
- **Benchmarking**: Run multiple iterations for statistical analysis
- **Configurable Limits**: Set custom compute budgets
- **Report Generation**: Generate detailed markdown reports

#### Usage Example:
```rust
use crate::common::*;

#[tokio::test]
async fn test_my_instruction_cu() {
    let env = start_test_environment().await;
    
    // Create your instruction
    let instruction = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![/* your accounts */],
        data: your_instruction.try_to_vec().unwrap(),
    };
    
    // Measure CUs
    let result = measure_instruction_cu(
        &mut env.banks_client.clone(),
        &env.payer,
        env.recent_blockhash,
        instruction,
        "my_instruction",
        Some(CUMeasurementConfig {
            compute_limit: 200_000,
            enable_logging: true,
            max_retries: 3,
        }),
    ).await;
    
    println!("Execution time: {}ms", result.execution_time_ms);
}
```

### 2. **solana-program-test Framework**

#### Built-in CU Tracking:
```rust
use solana_program_test::*;

#[tokio::test]
async fn test_with_cu_tracking() {
    let mut program_test = ProgramTest::new(
        "fixed-ratio-trading",
        fixed_ratio_trading::id(),
        processor!(process_instruction),
    );
    
    // Set compute budget
    program_test.set_compute_max_units(200_000);
    
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Your test logic here
}
```

### 3. **Solana CLI Tools**

#### For Devnet/Mainnet Testing:
```bash
# Test with compute budget
solana program test --compute-budget 200000

# Monitor transaction logs for CU consumption
solana logs --url devnet
```

### 4. **Production CU Monitoring**

#### Transaction Log Analysis:
```rust
// When deployed to devnet/mainnet, you can analyze transaction logs
let signature = /* your transaction signature */;
let transaction = rpc_client.get_transaction(&signature, UiTransactionEncoding::Json).await?;

// Check logs for "consumed X compute units"
for log in transaction.meta.log_messages {
    if log.contains("consumed") && log.contains("compute units") {
        println!("CU consumption: {}", log);
    }
}
```

## 📋 CU Measurement Test Examples

### Example 1: Compare Regular vs HFT Swap
```bash
# Run the comparison test
cargo test test_cu_measurement_swap_comparison

# Expected output:
# regular_swap: ~23ms execution
# hft_swap: ~16ms execution (faster due to optimizations)
```

### Example 2: Benchmark Pool Creation
```bash
# Run benchmark test
cargo test test_cu_measurement_pool_creation

# Expected output:
# Pool creation measurement with detailed timing
```

### Example 3: Generate Comprehensive Report
```bash
# Generate full CU report
cargo test test_cu_measurement_comprehensive_report

# Creates: cu_measurement_report.md
```

## 🔧 Advanced CU Optimization Techniques

### 1. **Compute Budget Instructions**
```rust
use solana_sdk::compute_budget::ComputeBudgetInstruction;

// Set compute unit limit
let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(50_000);

// Set compute unit price (for priority fees)
let compute_price_ix = ComputeBudgetInstruction::set_compute_unit_price(1000); // microlamports

let transaction = Transaction::new_signed_with_payer(
    &[compute_budget_ix, compute_price_ix, your_instruction],
    Some(&payer.pubkey()),
    &[&payer],
    recent_blockhash,
);
```

### 2. **CU Profiling in Code**
```rust
// Add to your processor functions for development
#[cfg(feature = "debug-cu")]
{
    let start_cu = /* get current CU count */;
    
    // Your logic here
    
    let end_cu = /* get current CU count */;
    msg!("Operation consumed {} CUs", start_cu - end_cu);
}
```

### 3. **Static Analysis Tools**

#### Anchor Framework (if using):
```bash
# Generate CU estimates
anchor build --verifiable
anchor test --skip-deploy
```

#### Custom Analysis:
```rust
// Count instruction complexity
fn estimate_cu_static(instruction: &Instruction) -> u64 {
    let base_cost = 2_000; // Base instruction cost
    let account_cost = instruction.accounts.len() as u64 * 100; // Per account
    let data_cost = instruction.data.len() as u64 * 10; // Per byte
    
    base_cost + account_cost + data_cost
}
```

## 🎯 Best Practices for CU Measurement

### 1. **Test Environment Setup**
```rust
// Use consistent test environment
let env = start_test_environment().await;

// Set appropriate compute limits
let config = CUMeasurementConfig {
    compute_limit: 200_000, // Conservative limit
    enable_logging: true,
    max_retries: 3,
};
```

### 2. **Measurement Accuracy**
- **Multiple Runs**: Always run multiple iterations
- **Statistical Analysis**: Calculate averages and standard deviations
- **Environment Consistency**: Use same test environment
- **Account State**: Ensure accounts are in expected state

### 3. **Production Validation**
```rust
// Add CU assertions in tests
assert!(result.execution_time_ms < 100, "Function too slow");

// Add CU budget checks
let cu_budget = 50_000;
assert!(estimated_cu < cu_budget, "CU budget exceeded");
```

## 📊 CU Benchmarking Framework

### Running All CU Tests:
```bash
# Run all CU measurement tests
cargo test test_cu_measurement --features hft-debug-logs

# Run with detailed output
RUST_LOG=debug cargo test test_cu_measurement

# Run specific measurement
cargo test test_cu_measurement_pool_creation
```

### Automated CU Regression Testing:
```bash
# Create script for CI/CD
#!/bin/bash
echo "Running CU regression tests..."
cargo test test_cu_measurement_comprehensive_report
echo "Checking CU budgets..."
# Parse generated report and check against limits
```

## 🚀 Performance Optimization Workflow

### 1. **Measure Current State**
```bash
cargo test test_cu_measurement_comprehensive_report
```

### 2. **Identify Bottlenecks**
- Check generated report for high CU operations
- Focus on frequently called functions
- Look for optimization opportunities

### 3. **Implement Optimizations**
- Apply HFT optimization patterns
- Reduce unnecessary computations
- Optimize account access patterns

### 4. **Validate Improvements**
```bash
# Re-run measurements
cargo test test_cu_measurement_swap_comparison

# Compare before/after results
```

## 📈 CU Monitoring in Production

### 1. **Transaction Logging**
```rust
// Add to your client SDK
pub async fn monitor_transaction_cu(
    &self,
    signature: &Signature,
) -> Result<u64, ClientError> {
    let transaction = self.rpc_client
        .get_transaction(signature, UiTransactionEncoding::Json)
        .await?;
    
    // Parse CU consumption from logs
    for log in transaction.meta.log_messages {
        if log.contains("consumed") && log.contains("compute units") {
            // Extract CU number from log
            return Ok(/* parsed CU count */);
        }
    }
    
    Ok(0)
}
```

### 2. **Performance Metrics**
```rust
// Track CU usage patterns
struct CUMetrics {
    operation_type: String,
    average_cu: u64,
    max_cu: u64,
    success_rate: f64,
    timestamp: u64,
}

// Store metrics for analysis
impl CUMetrics {
    pub fn record_operation(&mut self, cu_consumed: u64, success: bool) {
        // Update metrics
    }
}
```

## 🔍 Troubleshooting CU Issues

### Common Problems:
1. **CU Limit Exceeded**: Increase compute budget or optimize code
2. **Inconsistent Measurements**: Check test environment consistency
3. **Account State Issues**: Ensure proper account initialization

### Debug Commands:
```bash
# Check current CU limits
solana program show <program_id>

# Monitor live transactions
solana logs --url devnet | grep "consumed"

# Test with different CU limits
RUST_LOG=debug cargo test test_cu_measurement_config
```

## 📚 Additional Resources

### Documentation:
- [HFT CU Optimization Guide](./HFT_CU_OPTIMIZATION_GUIDE.md)
- [Solana Compute Budget Documentation](https://docs.solana.com/developing/programming-model/runtime#compute-budget)
- [Performance Best Practices](https://docs.solana.com/developing/programming-model/runtime#compute-budget)

### Example Reports:
- Generated reports in `cu_measurement_report.md`
- Historical CU data in test logs
- Performance regression tracking

---

**Last Updated**: Current  
**Framework Version**: Custom CU Measurement v1.0  
**Compatibility**: Solana 1.18.26+, solana-program-test framework 