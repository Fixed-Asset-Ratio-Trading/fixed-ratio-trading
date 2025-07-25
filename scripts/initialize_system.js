#!/usr/bin/env node

/**
 * Fixed Ratio Trading - System Initialization Script
 * 
 * This script initializes the Fixed Ratio Trading system by calling the InitializeProgram
 * instruction with the program upgrade authority. This creates essential system PDAs:
 * - SystemState PDA (global pause controls and authority management)
 * - MainTreasury PDA (pool creation fee collection)
 * 
 * Usage:
 *   node scripts/initialize_system.js <PROGRAM_ID> [RPC_URL] [KEYPAIR_PATH]
 * 
 * Arguments:
 *   PROGRAM_ID    - The deployed program ID (required)
 *   RPC_URL       - Solana RPC endpoint (default: http://192.168.2.88:8899)
 *   KEYPAIR_PATH  - Path to authority keypair (default: ~/.config/solana/id.json)
 * 
 * Examples:
 *   node scripts/initialize_system.js 2v1semv83194Uxq2ZmWnHP23LjKns9JTyhWWjaqKfNMx
 *   node scripts/initialize_system.js 2v1semv83194Uxq2ZmWnHP23LjKns9JTyhWWjaqKfNMx http://localhost:8899
 *   node scripts/initialize_system.js 2v1semv83194Uxq2ZmWnHP23LjKns9JTyhWWjaqKfNMx http://localhost:8899 ./keypair.json
 */

const { PublicKey, Connection, Transaction, TransactionInstruction, Keypair } = require('@solana/web3.js');
const fs = require('fs');
const path = require('path');
const os = require('os');

// Parse command line arguments
const args = process.argv.slice(2);

if (args.length < 1) {
    console.error('❌ Error: Program ID is required');
    console.error('Usage: node scripts/initialize_system.js <PROGRAM_ID> [RPC_URL] [KEYPAIR_PATH]');
    process.exit(1);
}

const PROGRAM_ID = args[0];
const RPC_URL = args[1] || 'http://192.168.2.88:8899';
const KEYPAIR_PATH = args[2] || path.join(os.homedir(), '.config', 'solana', 'id.json');

async function initializeSystem() {
    try {
        console.log('🔧 Fixed Ratio Trading - System Initialization');
        console.log('================================================');
        console.log(`📋 Program ID: ${PROGRAM_ID}`);
        console.log(`🌐 RPC URL: ${RPC_URL}`);
        console.log(`🔑 Keypair: ${KEYPAIR_PATH}`);
        console.log('');

        // Validate program ID format
        let programId;
        try {
            programId = new PublicKey(PROGRAM_ID);
        } catch (error) {
            console.error('❌ Invalid program ID format:', PROGRAM_ID);
            process.exit(1);
        }

        // Load authority keypair
        if (!fs.existsSync(KEYPAIR_PATH)) {
            console.error(`❌ Keypair file not found: ${KEYPAIR_PATH}`);
            process.exit(1);
        }

        const authorityKeypair = JSON.parse(fs.readFileSync(KEYPAIR_PATH, 'utf8'));
        const authority = Keypair.fromSecretKey(new Uint8Array(authorityKeypair));

        console.log(`✅ Program Authority: ${authority.publicKey.toString()}`);

        // Connect to Solana
        const connection = new Connection(RPC_URL, 'confirmed');
        
        // Test connection
        try {
            const version = await connection.getVersion();
            console.log(`✅ Connected to Solana RPC (version: ${version['solana-core']})`);
        } catch (error) {
            console.error(`❌ Failed to connect to RPC at ${RPC_URL}`);
            console.error(`   Error: ${error.message}`);
            process.exit(1);
        }

        // Check if system is already initialized
        const [systemStatePDA] = await PublicKey.findProgramAddress(
            [Buffer.from('system_state')],
            programId
        );

        const existingSystemState = await connection.getAccountInfo(systemStatePDA);
        if (existingSystemState && existingSystemState.data.length > 0) {
            console.log('⚠️  System is already initialized!');
            console.log(`   SystemState PDA: ${systemStatePDA.toString()}`);
            console.log(`   Data length: ${existingSystemState.data.length} bytes`);
            console.log('✅ System initialization verification complete');
            process.exit(0);
        }

        // Derive required PDAs
        const [mainTreasuryPDA] = await PublicKey.findProgramAddress(
            [Buffer.from('main_treasury')],
            programId
        );

        // Get program data account (contains upgrade authority)
        const BPF_LOADER_UPGRADEABLE = new PublicKey('BPFLoaderUpgradeab1e11111111111111111111111');
        const [programDataPDA] = await PublicKey.findProgramAddress(
            [programId.toBuffer()],
            BPF_LOADER_UPGRADEABLE
        );

        console.log('📍 Derived PDAs:');
        console.log(`   SystemState: ${systemStatePDA.toString()}`);
        console.log(`   MainTreasury: ${mainTreasuryPDA.toString()}`);
        console.log(`   Program Data: ${programDataPDA.toString()}`);
        console.log('');

        // Check authority balance
        const authorityBalance = await connection.getBalance(authority.publicKey);
        console.log(`💰 Authority Balance: ${(authorityBalance / 1e9).toFixed(4)} SOL`);
        
        if (authorityBalance < 1e8) { // Less than 0.1 SOL
            console.error('❌ Insufficient SOL balance for system initialization');
            console.error('   Need at least 0.1 SOL for account creation fees');
            process.exit(1);
        }

        // Create InitializeProgram instruction
        console.log('🚀 Creating system initialization transaction...');
        
        const instructionData = new Uint8Array([0]); // InitializeProgram discriminator (single byte)
        
        const accounts = [
            { pubkey: authority.publicKey, isSigner: true, isWritable: true },        // 0: Program Authority
            { pubkey: require('@solana/web3.js').SystemProgram.programId, isSigner: false, isWritable: false }, // 1: System Program
            { pubkey: require('@solana/web3.js').SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },      // 2: Rent Sysvar
            { pubkey: systemStatePDA, isSigner: false, isWritable: true },           // 3: System State PDA
            { pubkey: mainTreasuryPDA, isSigner: false, isWritable: true },          // 4: Main Treasury PDA
            { pubkey: programDataPDA, isSigner: false, isWritable: false },          // 5: Program Data Account
        ];

        const instruction = new TransactionInstruction({
            keys: accounts,
            programId,
            data: instructionData
        });

        const transaction = new Transaction().add(instruction);

        // Get recent blockhash
        const { blockhash } = await connection.getLatestBlockhash();
        transaction.recentBlockhash = blockhash;
        transaction.feePayer = authority.publicKey;

        // Sign transaction
        transaction.sign(authority);

        console.log('📡 Sending system initialization transaction...');
        
        // Send transaction
        const signature = await connection.sendRawTransaction(transaction.serialize());
        console.log(`   Transaction: ${signature}`);

        // Wait for confirmation
        console.log('⏳ Waiting for confirmation...');
        await connection.confirmTransaction(signature, 'confirmed');

        console.log('✅ System initialization transaction confirmed!');

        // Verify initialization
        console.log('🔍 Verifying system initialization...');
        
        const systemStateAccount = await connection.getAccountInfo(systemStatePDA);
        if (systemStateAccount && systemStateAccount.data.length > 0) {
            console.log('✅ SystemState PDA created successfully!');
            console.log(`   Address: ${systemStatePDA.toString()}`);
            console.log(`   Data length: ${systemStateAccount.data.length} bytes`);
            console.log(`   Owner: ${systemStateAccount.owner.toString()}`);
        } else {
            console.log('❌ SystemState PDA verification failed');
            process.exit(1);
        }

        const mainTreasuryAccount = await connection.getAccountInfo(mainTreasuryPDA);
        if (mainTreasuryAccount && mainTreasuryAccount.data.length > 0) {
            console.log('✅ MainTreasury PDA created successfully!');
            console.log(`   Address: ${mainTreasuryPDA.toString()}`);
            console.log(`   Data length: ${mainTreasuryAccount.data.length} bytes`);
        } else {
            console.log('❌ MainTreasury PDA verification failed');
            process.exit(1);
        }

        console.log('');
        console.log('🎉 Fixed Ratio Trading system initialization complete!');
        console.log('   The system is now ready for users to create pools.');
        
        process.exit(0);

    } catch (error) {
        console.error('❌ System initialization failed:', error.message);
        
        if (error.message.includes('Transaction simulation failed')) {
            console.error('');
            console.error('💡 Common causes:');
            console.error('   • Program authority mismatch (check if you deployed the program)');
            console.error('   • Insufficient SOL balance for account creation');
            console.error('   • Program not properly deployed or accessible');
            console.error('   • Network connectivity issues');
        }
        
        process.exit(1);
    }
}

// Run the initialization
initializeSystem(); 