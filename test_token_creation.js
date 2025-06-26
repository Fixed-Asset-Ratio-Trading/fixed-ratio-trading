#!/usr/bin/env node

// Test Token Creation Script
// Run this to test token creation without using the web interface
// Usage: node test_token_creation.js

const { Connection, Keypair, SystemProgram, LAMPORTS_PER_SOL } = require('@solana/web3.js');
const { createMint, createAssociatedTokenAccount, mintTo, getAssociatedTokenAddress } = require('@solana/spl-token');

// Configuration
const RPC_URL = 'http://localhost:8899';
const WALLET_KEYPAIR_PATH = process.env.HOME + '/.config/solana/id.json'; // Default Solana keypair

async function testTokenCreation() {
    console.log('🧪 Testing Token Creation');
    console.log('========================');
    
    try {
        // Connect to local Solana cluster
        const connection = new Connection(RPC_URL, 'confirmed');
        console.log('✅ Connected to Solana RPC:', RPC_URL);
        
        // Load wallet keypair (you might need to adjust this path)
        let payer;
        try {
            const fs = require('fs');
            const keypairData = JSON.parse(fs.readFileSync(WALLET_KEYPAIR_PATH, 'utf8'));
            payer = Keypair.fromSecretKey(new Uint8Array(keypairData));
            console.log('✅ Loaded wallet:', payer.publicKey.toString());
        } catch (error) {
            console.log('⚠️  Could not load default wallet, generating new one for testing');
            payer = Keypair.generate();
            console.log('🔑 Generated test wallet:', payer.publicKey.toString());
            
            // Airdrop some SOL for testing
            const airdropSignature = await connection.requestAirdrop(payer.publicKey, 2 * LAMPORTS_PER_SOL);
            await connection.confirmTransaction(airdropSignature);
            console.log('💰 Airdropped 2 SOL for testing');
        }
        
        // Check balance
        const balance = await connection.getBalance(payer.publicKey);
        console.log(`💰 Wallet balance: ${balance / LAMPORTS_PER_SOL} SOL`);
        
        // Test token data
        const tokenData = {
            name: 'Test Token',
            symbol: 'TEST',
            decimals: 9,
            supply: 1000000
        };
        
        console.log('\n🪙 Creating token with data:', tokenData);
        
        // Create mint
        console.log('📝 Creating mint...');
        const mintKeypair = Keypair.generate();
        
        const mint = await createMint(
            connection,
            payer,           // payer
            payer.publicKey, // mint authority
            payer.publicKey, // freeze authority
            tokenData.decimals,
            mintKeypair
        );
        
        console.log('✅ Mint created:', mint.toString());
        
        // Create associated token account
        console.log('📝 Creating associated token account...');
        const associatedTokenAccount = await createAssociatedTokenAccount(
            connection,
            payer,           // payer
            mint,            // mint
            payer.publicKey  // owner
        );
        
        console.log('✅ Associated token account created:', associatedTokenAccount.toString());
        
        // Mint tokens
        console.log(`📝 Minting ${tokenData.supply} tokens...`);
        const mintAmount = tokenData.supply * Math.pow(10, tokenData.decimals);
        
        await mintTo(
            connection,
            payer,                    // payer
            mint,                     // mint
            associatedTokenAccount,   // destination
            payer,                    // authority
            mintAmount               // amount
        );
        
        console.log('✅ Tokens minted successfully!');
        
        // Summary
        console.log('\n🎉 TOKEN CREATION SUCCESS!');
        console.log('==========================');
        console.log(`Token Name: ${tokenData.name} (${tokenData.symbol})`);
        console.log(`Mint Address: ${mint.toString()}`);
        console.log(`Decimals: ${tokenData.decimals}`);
        console.log(`Total Supply: ${tokenData.supply.toLocaleString()} tokens`);
        console.log(`Token Account: ${associatedTokenAccount.toString()}`);
        console.log(`Owner: ${payer.publicKey.toString()}`);
        
        return {
            success: true,
            mint: mint.toString(),
            tokenAccount: associatedTokenAccount.toString(),
            owner: payer.publicKey.toString()
        };
        
    } catch (error) {
        console.error('❌ Token creation failed:', error.message);
        console.error('Stack trace:', error.stack);
        return { success: false, error: error.message };
    }
}

// Run the test if called directly
if (require.main === module) {
    testTokenCreation()
        .then(result => {
            if (result.success) {
                console.log('\n✨ Test completed successfully!');
                process.exit(0);
            } else {
                console.log('\n💥 Test failed!');
                process.exit(1);
            }
        })
        .catch(error => {
            console.error('💥 Unexpected error:', error);
            process.exit(1);
        });
}

module.exports = { testTokenCreation }; 