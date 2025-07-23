#!/usr/bin/env node

/**
 * Fixed Ratio Trading Program State Query Script
 * 
 * This script queries all program accounts and state data from the Solana blockchain
 * and exports them to a JSON file for dashboard consumption.
 * 
 * Features:
 * - Queries all pool states using Program Account Query
 * - Retrieves MainTreasuryState and SystemState
 * - Serializes all data to JSON format
 * - Handles Solana environment resets gracefully
 * - Integrates with deployment pipeline
 */

const { Connection, PublicKey } = require('@solana/web3.js');
const fs = require('fs');
const path = require('path');

// Load shared configuration
function loadSharedConfig() {
    try {
        const configPath = path.join(__dirname, '../shared-config.json');
        const configData = fs.readFileSync(configPath, 'utf8');
        const sharedConfig = JSON.parse(configData);
        
        console.log('✅ Loaded shared configuration from:', configPath);
        
        return {
            // Environment variables can override shared config
            rpcUrl: process.env.SOLANA_RPC_URL || sharedConfig.solana.rpcUrl,
            programId: process.env.PROGRAM_ID || sharedConfig.program.programId,
            commitment: sharedConfig.solana.commitment,
            outputFile: path.join(__dirname, '..', sharedConfig.dashboard.stateFile)
        };
        
    } catch (error) {
        console.warn('⚠️ Failed to load shared config, using fallback:', error.message);
        
        // Fallback configuration
        return {
            rpcUrl: process.env.SOLANA_RPC_URL || 'http://192.168.2.88:8899',
            programId: process.env.PROGRAM_ID || '4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn',
            outputFile: path.join(__dirname, '../dashboard/state.json'),
            commitment: 'confirmed'
        };
    }
}

// Configuration
const CONFIG = loadSharedConfig();

// PDA seed constants (must match smart contract)
const SEEDS = {
    POOL_STATE: 'pool_state',
    MAIN_TREASURY: 'main_treasury',
    SYSTEM_STATE: 'system_state'
};

/**
 * Parse PoolState account data
 */
function parsePoolState(data, address) {
    try {
        const dataArray = new Uint8Array(data);
        let offset = 0;
        
        const readPubkey = () => {
            const pubkey = new PublicKey(dataArray.slice(offset, offset + 32));
            offset += 32;
            return pubkey.toString();
        };

        const readU64 = () => {
            const value = dataArray.slice(offset, offset + 8);
            offset += 8;
            let result = 0n;
            for (let i = 7; i >= 0; i--) {
                result = (result << 8n) + BigInt(value[i]);
            }
            return Number(result);
        };

        const readI64 = () => {
            const value = dataArray.slice(offset, offset + 8);
            offset += 8;
            const view = new DataView(value.buffer, value.byteOffset, value.byteLength);
            return Number(view.getBigInt64(0, true)); // little endian
        };

        const readU8 = () => {
            const value = dataArray[offset];
            offset += 1;
            return value;
        };

        // Parse all PoolState fields according to the struct definition
        const poolState = {
            // Pool address (derived)
            address: address.toString(),
            
            // Basic pool information
            owner: readPubkey(),
            token_a_mint: readPubkey(),
            token_b_mint: readPubkey(),
            token_a_vault: readPubkey(),
            token_b_vault: readPubkey(),
            lp_token_a_mint: readPubkey(),
            lp_token_b_mint: readPubkey(),
            
            // Ratio configuration
            ratio_a_numerator: readU64(),
            ratio_b_denominator: readU64(),
            
            // Liquidity information
            total_token_a_liquidity: readU64(),
            total_token_b_liquidity: readU64(),
            
            // Bump seeds
            pool_authority_bump_seed: readU8(),
            token_a_vault_bump_seed: readU8(),
            token_b_vault_bump_seed: readU8(),
            lp_token_a_mint_bump_seed: readU8(),
            lp_token_b_mint_bump_seed: readU8(),
            
            // Pool flags (bitwise operations)
            flags: readU8(),
            
            // Configurable contract fees
            contract_liquidity_fee: readU64(),
            swap_contract_fee: readU64(),
            
            // Token fee tracking
            collected_fees_token_a: readU64(),
            collected_fees_token_b: readU64(),
            total_fees_withdrawn_token_a: readU64(),
            total_fees_withdrawn_token_b: readU64(),
            
            // SOL fee tracking
            collected_liquidity_fees: readU64(),
            collected_swap_contract_fees: readU64(),
            total_sol_fees_collected: readU64(),
            
            // Consolidation management
            last_consolidation_timestamp: readI64(),
            total_consolidations: readU64(),
            total_fees_consolidated: readU64(),
            
            // Derived fields for UI
            flags_decoded: {
                one_to_many_ratio: (readU8() & 1) !== 0,
                liquidity_paused: (dataArray[offset - 1] & 2) !== 0,
                swaps_paused: (dataArray[offset - 1] & 4) !== 0,
                withdrawal_protection: (dataArray[offset - 1] & 8) !== 0,
                single_lp_token_mode: (dataArray[offset - 1] & 16) !== 0
            }
        };
        
        // Reset flags reading offset
        offset -= 1;
        const flags = readU8();
        poolState.flags_decoded = {
            one_to_many_ratio: (flags & 1) !== 0,
            liquidity_paused: (flags & 2) !== 0,
            swaps_paused: (flags & 4) !== 0,
            withdrawal_protection: (flags & 8) !== 0,
            single_lp_token_mode: (flags & 16) !== 0
        };
        
        return poolState;
    } catch (error) {
        console.error(`Error parsing PoolState for ${address}:`, error);
        return null;
    }
}

/**
 * Parse MainTreasuryState account data
 */
function parseMainTreasuryState(data) {
    try {
        const dataArray = new Uint8Array(data);
        let offset = 0;
        
        const readU64 = () => {
            const value = dataArray.slice(offset, offset + 8);
            offset += 8;
            let result = 0n;
            for (let i = 7; i >= 0; i--) {
                result = (result << 8n) + BigInt(value[i]);
            }
            return Number(result);
        };

        const readI64 = () => {
            const value = dataArray.slice(offset, offset + 8);
            offset += 8;
            const view = new DataView(value.buffer, value.byteOffset, value.byteLength);
            return Number(view.getBigInt64(0, true));
        };

        return {
            // Balance information
            total_balance: readU64(),
            rent_exempt_minimum: readU64(),
            total_withdrawn: readU64(),
            
            // Operation counters
            pool_creation_count: readU64(),
            liquidity_operation_count: readU64(),
            regular_swap_count: readU64(),
            treasury_withdrawal_count: readU64(),
            failed_operation_count: readU64(),
            
            // Fee totals
            total_pool_creation_fees: readU64(),
            total_liquidity_fees: readU64(),
            total_regular_swap_fees: readU64(),
            total_swap_contract_fees: readU64(),
            
            // Timestamps and consolidation
            last_update_timestamp: readI64(),
            total_consolidations_performed: readU64(),
            last_consolidation_timestamp: readI64()
        };
    } catch (error) {
        console.error('Error parsing MainTreasuryState:', error);
        return null;
    }
}

/**
 * Parse SystemState account data
 */
function parseSystemState(data) {
    try {
        const dataArray = new Uint8Array(data);
        let offset = 0;
        
        const readBool = () => {
            const value = dataArray[offset] !== 0;
            offset += 1;
            return value;
        };

        const readI64 = () => {
            const value = dataArray.slice(offset, offset + 8);
            offset += 8;
            const view = new DataView(value.buffer, value.byteOffset, value.byteLength);
            return Number(view.getBigInt64(0, true));
        };

        const readU8 = () => {
            const value = dataArray[offset];
            offset += 1;
            return value;
        };

        const pauseReasonCode = readU8();
        
        return {
            is_paused: readBool(),
            pause_timestamp: readI64(),
            pause_reason_code: pauseReasonCode,
            pause_reason_decoded: getPauseReasonDescription(pauseReasonCode)
        };
    } catch (error) {
        console.error('Error parsing SystemState:', error);
        return null;
    }
}

/**
 * Decode pause reason code to human-readable description
 */
function getPauseReasonDescription(code) {
    const reasons = {
        0: 'No pause / Normal operation',
        1: 'Emergency pause - Security incident',
        2: 'Maintenance pause - System upgrade',
        3: 'Administrative pause - Manual intervention',
        4: 'Circuit breaker - Automated safety trigger'
    };
    return reasons[code] || `Unknown reason code: ${code}`;
}

/**
 * Derive PDA addresses
 */
async function derivePDAAddresses(programId) {
    const programPubkey = new PublicKey(programId);
    
    // Main Treasury PDA
    const [mainTreasuryPda] = await PublicKey.findProgramAddress(
        [Buffer.from(SEEDS.MAIN_TREASURY)],
        programPubkey
    );
    
    // System State PDA
    const [systemStatePda] = await PublicKey.findProgramAddress(
        [Buffer.from(SEEDS.SYSTEM_STATE)],
        programPubkey
    );
    
    return {
        mainTreasuryPda,
        systemStatePda
    };
}

/**
 * Query all program accounts for pools
 */
async function queryAllPools(connection, programId) {
    console.log('🔍 Querying all pool accounts...');
    
    try {
        const programPubkey = new PublicKey(programId);
        
        // Get all accounts owned by the program
        const accounts = await connection.getProgramAccounts(programPubkey, {
            commitment: CONFIG.commitment,
            encoding: 'base64'
        });
        
        console.log(`📊 Found ${accounts.length} program accounts`);
        
        // Filter and parse pool state accounts
        const pools = [];
        
        for (const account of accounts) {
            // Skip if account data is too small for PoolState
            if (account.account.data.length < 400) { // PoolState is larger than this
                continue;
            }
            
            // Try to parse as PoolState
            const poolState = parsePoolState(account.account.data, account.pubkey);
            if (poolState) {
                pools.push(poolState);
                console.log(`✅ Parsed pool: ${account.pubkey.toString()}`);
            }
        }
        
        console.log(`🎯 Successfully parsed ${pools.length} pools`);
        return pools;
        
    } catch (error) {
        console.error('❌ Error querying pools:', error);
        return [];
    }
}

/**
 * Query treasury and system state
 */
async function querySystemStates(connection, pdaAddresses) {
    console.log('🔍 Querying system states...');
    
    const results = {
        mainTreasuryState: null,
        systemState: null
    };
    
    try {
        // Query Main Treasury State
        const treasuryAccount = await connection.getAccountInfo(
            pdaAddresses.mainTreasuryPda,
            CONFIG.commitment
        );
        
        if (treasuryAccount) {
            results.mainTreasuryState = parseMainTreasuryState(treasuryAccount.data);
            console.log('✅ Parsed MainTreasuryState');
        } else {
            console.log('⚠️ MainTreasuryState account not found');
        }
        
        // Query System State
        const systemAccount = await connection.getAccountInfo(
            pdaAddresses.systemStatePda,
            CONFIG.commitment
        );
        
        if (systemAccount) {
            results.systemState = parseSystemState(systemAccount.data);
            console.log('✅ Parsed SystemState');
        } else {
            console.log('⚠️ SystemState account not found');
        }
        
    } catch (error) {
        console.error('❌ Error querying system states:', error);
    }
    
    return results;
}

/**
 * Generate metadata for the JSON export
 */
function generateMetadata() {
    return {
        generated_at: new Date().toISOString(),
        program_id: CONFIG.programId,
        rpc_url: CONFIG.rpcUrl,
        script_version: '1.0.0',
        solana_environment: process.env.SOLANA_ENVIRONMENT || 'localnet'
    };
}

/**
 * Save state data to JSON file
 */
function saveStateToFile(stateData, outputPath) {
    try {
        // Ensure output directory exists
        const outputDir = path.dirname(outputPath);
        if (!fs.existsSync(outputDir)) {
            fs.mkdirSync(outputDir, { recursive: true });
        }
        
        // Write JSON file with pretty formatting
        fs.writeFileSync(
            outputPath,
            JSON.stringify(stateData, null, 2),
            'utf8'
        );
        
        console.log(`💾 State data saved to: ${outputPath}`);
        console.log(`📊 File size: ${fs.statSync(outputPath).size} bytes`);
        
    } catch (error) {
        console.error('❌ Error saving state file:', error);
        throw error;
    }
}

/**
 * Main execution function
 */
async function main() {
    console.log('🚀 Fixed Ratio Trading Program State Query');
    console.log('==========================================');
    console.log(`📡 RPC URL: ${CONFIG.rpcUrl}`);
    console.log(`🆔 Program ID: ${CONFIG.programId}`);
    console.log(`📁 Output File: ${CONFIG.outputFile}`);
    console.log('');
    
    try {
        // Initialize Solana connection
        const connection = new Connection(CONFIG.rpcUrl, CONFIG.commitment);
        
        // Test connection
        const version = await connection.getVersion();
        console.log(`✅ Connected to Solana cluster version: ${version['solana-core']}`);
        
        // Derive PDA addresses
        const pdaAddresses = await derivePDAAddresses(CONFIG.programId);
        console.log(`🔑 Main Treasury PDA: ${pdaAddresses.mainTreasuryPda.toString()}`);
        console.log(`🔑 System State PDA: ${pdaAddresses.systemStatePda.toString()}`);
        console.log('');
        
        // Query all data
        const [pools, systemStates] = await Promise.all([
            queryAllPools(connection, CONFIG.programId),
            querySystemStates(connection, pdaAddresses)
        ]);
        
        // Compile final state data
        const stateData = {
            metadata: generateMetadata(),
            pools: pools,
            main_treasury_state: systemStates.mainTreasuryState,
            system_state: systemStates.systemState,
            pda_addresses: {
                main_treasury: pdaAddresses.mainTreasuryPda.toString(),
                system_state: pdaAddresses.systemStatePda.toString()
            }
        };
        
        // Save to file
        saveStateToFile(stateData, CONFIG.outputFile);
        
        // Summary
        console.log('');
        console.log('📊 QUERY SUMMARY');
        console.log('================');
        console.log(`🏊 Pools found: ${pools.length}`);
        console.log(`💰 Treasury state: ${systemStates.mainTreasuryState ? '✅' : '❌'}`);
        console.log(`⚙️ System state: ${systemStates.systemState ? '✅' : '❌'}`);
        console.log(`📁 Output file: ${CONFIG.outputFile}`);
        console.log('');
        console.log('🎉 Program state query completed successfully!');
        
    } catch (error) {
        console.error('💥 Fatal error during state query:', error);
        process.exit(1);
    }
}

// Error handling for uncaught exceptions
process.on('uncaughtException', (error) => {
    console.error('💥 Uncaught Exception:', error);
    process.exit(1);
});

process.on('unhandledRejection', (reason, promise) => {
    console.error('💥 Unhandled Rejection at:', promise, 'reason:', reason);
    process.exit(1);
});

// Execute main function if script is run directly
if (require.main === module) {
    main();
}

module.exports = {
    main,
    parsePoolState,
    parseMainTreasuryState,
    parseSystemState,
    CONFIG
}; 