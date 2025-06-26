// Token Creation Dashboard - JavaScript Logic
// Handles Phantom wallet connection and SPL token creation

// Configuration
const CONFIG = {
    rpcUrl: 'http://localhost:8899',
    expectedWallet: '5GGZiMwU56rYL1L52q7Jz7ELkSN4iYyQqdv418hxPh6t',
    commitment: 'confirmed'
};

// Global state
let connection = null;
let wallet = null;
let isConnected = false;
let createdTokens = [];

// Initialize when page loads
document.addEventListener('DOMContentLoaded', async () => {
    console.log('🚀 Token Creation Dashboard initializing...');
    showStatus('info', '🔄 Loading libraries and initializing...');
    
    // Simple retry mechanism with clearer feedback
    let attempts = 0;
    const maxAttempts = 8;
    
    const tryInitialize = async () => {
        attempts++;
        console.log(`📋 Initialization attempt ${attempts}/${maxAttempts}`);
        
        // Check if libraries are loaded
        if (window.solanaWeb3 && window.SPL_TOKEN_LOADED === true) {
            console.log('✅ All libraries loaded successfully!');
            await initializeApp();
            return;
        }
        
        // If libraries aren't loaded yet, try again
        if (attempts < maxAttempts) {
            console.log(`⏳ Libraries still loading... retrying in 1 second`);
            setTimeout(tryInitialize, 1000);
        } else {
            console.error('❌ Failed to load libraries after', maxAttempts, 'attempts');
            showStatus('error', '❌ Failed to load required libraries. Please refresh the page and check your internet connection.');
        }
    };
    
    // Start first attempt after a brief delay
    setTimeout(tryInitialize, 1500);
});

/**
 * Initialize the application
 */
async function initializeApp() {
    try {
        // Initialize Solana connection
        connection = new solanaWeb3.Connection(CONFIG.rpcUrl, CONFIG.commitment);
        
        // Check if SPL Token library is available
        if (!window.splToken || !window.SPL_TOKEN_LOADED) {
            console.error('❌ SPL Token library not loaded properly');
            showStatus('error', 'SPL Token library not loaded. Please refresh the page.');
            return;
        }
        
        console.log('✅ SPL Token library ready:', Object.keys(window.splToken).slice(0, 10) + '...');
        
        // Check if Backpack is installed
        if (!window.backpack) {
            showStatus('error', 'Backpack wallet not detected. Please install Backpack wallet extension.');
            return;
        }
        
        // Check if already connected
        if (window.backpack.isConnected) {
            await handleWalletConnected();
        }
        
        // Setup form event listeners
        setupFormListeners();
        
        console.log('✅ Token Creation Dashboard initialized');
    } catch (error) {
        console.error('❌ Failed to initialize:', error);
        showStatus('error', 'Failed to initialize application: ' + error.message);
    }
}

/**
 * Setup form event listeners
 */
function setupFormListeners() {
    const form = document.getElementById('token-form');
    const inputs = form.querySelectorAll('input[required]');
    
    // Form submission
    form.addEventListener('submit', handleTokenCreation);
    
    // Real-time validation
    inputs.forEach(input => {
        input.addEventListener('input', updateCreateButtonState);
    });
    
    // Initial button state
    updateCreateButtonState();
}

/**
 * Connect to Backpack wallet
 */
async function connectWallet() {
    try {
        if (!window.backpack) {
            showStatus('error', 'Backpack wallet not installed. Please install the Backpack browser extension.');
            return;
        }
        
        showStatus('info', 'Connecting to Backpack wallet...');
        
        const response = await window.backpack.connect();
        await handleWalletConnected();
        
        console.log('✅ Wallet connected:', response.publicKey.toString());
    } catch (error) {
        console.error('❌ Failed to connect wallet:', error);
        showStatus('error', 'Failed to connect wallet: ' + error.message);
    }
}

/**
 * Handle successful wallet connection
 */
async function handleWalletConnected() {
    try {
        wallet = window.backpack;
        isConnected = true;
        
        const publicKey = wallet.publicKey.toString();
        
        // Update UI
        document.getElementById('wallet-info').style.display = 'flex';
        document.getElementById('wallet-disconnected').style.display = 'none';
        document.getElementById('wallet-address').textContent = publicKey;
        document.getElementById('connect-wallet-btn').textContent = 'Disconnect';
        document.getElementById('connect-wallet-btn').onclick = disconnectWallet;
        
        // Check if this is the expected wallet
        if (publicKey === CONFIG.expectedWallet) {
            showStatus('success', `✅ Connected with Backpack deployment wallet: ${publicKey.slice(0, 20)}...`);
            document.getElementById('wallet-avatar').textContent = '🎯';
        } else {
            showStatus('info', `ℹ️ Connected with Backpack wallet: ${publicKey.slice(0, 20)}... (Note: This is not the deployment wallet)`);
        }
        
        // Check balance
        await checkWalletBalance();
        
        // Update form state
        updateCreateButtonState();
        
        // Load existing tokens
        await loadCreatedTokens();
        
    } catch (error) {
        console.error('❌ Error handling wallet connection:', error);
        showStatus('error', 'Error handling wallet connection: ' + error.message);
    }
}

/**
 * Disconnect wallet
 */
async function disconnectWallet() {
    try {
        if (window.backpack) {
            await window.backpack.disconnect();
        }
        
        // Reset state
        wallet = null;
        isConnected = false;
        
        // Update UI
        document.getElementById('wallet-info').style.display = 'none';
        document.getElementById('wallet-disconnected').style.display = 'flex';
        document.getElementById('connect-wallet-btn').textContent = 'Connect Backpack Wallet';
        document.getElementById('connect-wallet-btn').onclick = connectWallet;
        
        // Update form state
        updateCreateButtonState();
        
        showStatus('info', 'Wallet disconnected');
        
    } catch (error) {
        console.error('❌ Error disconnecting wallet:', error);
    }
}

/**
 * Check wallet balance
 */
async function checkWalletBalance() {
    try {
        const balance = await connection.getBalance(wallet.publicKey);
        const solBalance = balance / solanaWeb3.LAMPORTS_PER_SOL;
        
        if (solBalance < 0.1) {
            showStatus('error', `⚠️ Low SOL balance: ${solBalance.toFixed(4)} SOL. You may need more SOL for transactions.`);
        } else {
            console.log(`💰 Wallet balance: ${solBalance.toFixed(4)} SOL`);
        }
    } catch (error) {
        console.error('❌ Error checking balance:', error);
    }
}

/**
 * Update create button state based on form validation and wallet connection
 */
function updateCreateButtonState() {
    const form = document.getElementById('token-form');
    const createBtn = document.getElementById('create-btn');
    const sampleBtn = document.getElementById('create-sample-btn');
    const requiredInputs = form.querySelectorAll('input[required]');
    
    let allValid = isConnected;
    
    requiredInputs.forEach(input => {
        if (!input.value.trim()) {
            allValid = false;
        }
    });
    
    createBtn.disabled = !allValid;
    
    // Sample button only needs wallet connection
    if (sampleBtn) {
        sampleBtn.disabled = !isConnected;
    }
}

/**
 * Create sample token for quick testing
 */
async function createSampleToken() {
    if (!isConnected || !wallet) {
        showStatus('error', 'Please connect your wallet first');
        return;
    }
    
    const sampleBtn = document.getElementById('create-sample-btn');
    const originalText = sampleBtn.textContent;
    
    try {
        sampleBtn.disabled = true;
        sampleBtn.textContent = '🔄 Creating Sample Token...';
        
        // Sample token data
        const sampleData = {
            name: 'Token Sample',
            symbol: 'TS',
            decimals: 4,
            supply: 10000,
            description: 'Sample token for testing purposes'
        };
        
        showStatus('info', `Creating sample token "${sampleData.name}" (${sampleData.symbol})...`);
        
        // Create token
        const tokenInfo = await createSPLToken(sampleData);
        
        // Store created token
        createdTokens.push(tokenInfo);
        localStorage.setItem('createdTokens', JSON.stringify(createdTokens));
        
        // Update UI
        updateTokensList();
        
        showStatus('success', `🎉 Sample token "${sampleData.name}" created successfully! 
        💰 ${sampleData.supply.toLocaleString()} ${sampleData.symbol} tokens minted to your wallet
        🔑 Mint Address: ${tokenInfo.mint}`);
        
    } catch (error) {
        console.error('❌ Error creating sample token:', error);
        showStatus('error', 'Failed to create sample token: ' + error.message);
    } finally {
        sampleBtn.disabled = false;
        sampleBtn.textContent = originalText;
    }
}

/**
 * Handle token creation form submission
 */
async function handleTokenCreation(event) {
    event.preventDefault();
    
    if (!isConnected || !wallet) {
        showStatus('error', 'Please connect your wallet first');
        return;
    }
    
    const createBtn = document.getElementById('create-btn');
    const originalText = createBtn.textContent;
    
    try {
        createBtn.disabled = true;
        createBtn.textContent = '🔄 Creating Token...';
        
        // Get form data
        const formData = getFormData();
        
        showStatus('info', `Creating token "${formData.name}" (${formData.symbol})...`);
        
        // Create token
        const tokenInfo = await createSPLToken(formData);
        
        // Store created token
        createdTokens.push(tokenInfo);
        localStorage.setItem('createdTokens', JSON.stringify(createdTokens));
        
        // Update UI
        updateTokensList();
        clearForm();
        
        showStatus('success', `🎉 Token "${formData.name}" created successfully! 
        💰 ${formData.supply.toLocaleString()} ${formData.symbol} tokens minted to your wallet
        🔑 Mint Address: ${tokenInfo.mint}`);
        
    } catch (error) {
        console.error('❌ Error creating token:', error);
        showStatus('error', 'Failed to create token: ' + error.message);
    } finally {
        createBtn.disabled = false;
        createBtn.textContent = originalText;
        updateCreateButtonState();
    }
}

/**
 * Get form data
 */
function getFormData() {
    return {
        name: document.getElementById('token-name').value.trim(),
        symbol: document.getElementById('token-symbol').value.trim().toUpperCase(),
        decimals: parseInt(document.getElementById('token-decimals').value),
        supply: parseInt(document.getElementById('token-supply').value),
        description: document.getElementById('token-description').value.trim()
    };
}

/**
 * Create SPL Token
 */
async function createSPLToken(tokenData) {
    try {
        console.log('🎨 Creating SPL token with data:', tokenData);
        
        // Debug: Check if SPL Token library is available
        if (!window.splToken) {
            throw new Error('SPL Token library not available. Please refresh the page.');
        }
        
        console.log('🔍 SPL Token library ready for token creation');
        
        console.log('🚀 Creating SPL token...');
        
        let mint, associatedTokenAccount;
        
        // Generate mint keypair
        const mintKeypair = solanaWeb3.Keypair.generate();
        console.log('🔑 Generated mint keypair:', mintKeypair.publicKey.toString());
        
        // Get rent exemption for mint account
        const mintRent = await connection.getMinimumBalanceForRentExemption(window.splToken.MintLayout.span);
        
        // Build instructions array
        const instructions = [];
        
        // 1. Create mint account
        instructions.push(
            solanaWeb3.SystemProgram.createAccount({
                fromPubkey: wallet.publicKey,
                newAccountPubkey: mintKeypair.publicKey,
                lamports: mintRent,
                space: window.splToken.MintLayout.span,
                programId: window.splToken.TOKEN_PROGRAM_ID
            })
        );
        
        // 2. Initialize mint instruction
        instructions.push(
            window.splToken.Token.createInitMintInstruction(
                window.splToken.TOKEN_PROGRAM_ID,
                mintKeypair.publicKey,
                tokenData.decimals,
                wallet.publicKey,     // mint authority (you control minting)
                wallet.publicKey      // freeze authority (you control freezing)
            )
        );
        
        // 3. Get associated token address for your wallet
        associatedTokenAccount = await window.splToken.Token.getAssociatedTokenAddress(
            window.splToken.ASSOCIATED_TOKEN_PROGRAM_ID,
            window.splToken.TOKEN_PROGRAM_ID,
            mintKeypair.publicKey,
            wallet.publicKey
        );
        console.log('📍 Token account address:', associatedTokenAccount.toString());
        
        // 4. Create associated token account instruction
        instructions.push(
            window.splToken.Token.createAssociatedTokenAccountInstruction(
                window.splToken.ASSOCIATED_TOKEN_PROGRAM_ID,
                window.splToken.TOKEN_PROGRAM_ID,
                mintKeypair.publicKey,
                associatedTokenAccount,
                wallet.publicKey,     // owner (YOU own this account)
                wallet.publicKey      // payer (you pay for creation)
            )
        );
        
        // 5. Mint all tokens to your wallet
        const totalSupplyWithDecimals = tokenData.supply * Math.pow(10, tokenData.decimals);
        console.log(`💰 Minting ${tokenData.supply} ${tokenData.symbol} tokens to your wallet...`);
        
        instructions.push(
            window.splToken.Token.createMintToInstruction(
                window.splToken.TOKEN_PROGRAM_ID,
                mintKeypair.publicKey,
                associatedTokenAccount,   // destination (YOUR token account)
                wallet.publicKey,         // mint authority (you control minting)
                [],                       // multi signers
                totalSupplyWithDecimals  // amount (ALL the supply goes to you)
            )
        );
        
        // Create and send transaction
        const transaction = new solanaWeb3.Transaction().add(...instructions);
        
        // Set recent blockhash and fee payer
        const { blockhash } = await connection.getLatestBlockhash();
        transaction.recentBlockhash = blockhash;
        transaction.feePayer = wallet.publicKey;
        
        // Sign with mint keypair (partial sign)
        transaction.partialSign(mintKeypair);
        
        console.log('📝 Requesting wallet signature...');
        
        // Sign with wallet and send
        const signedTransaction = await wallet.signTransaction(transaction);
        const signature = await connection.sendRawTransaction(signedTransaction.serialize());
        
        console.log('📡 Transaction sent:', signature);
        showStatus('info', `Transaction submitted: ${signature}`);
        
        // Confirm transaction
        const confirmation = await connection.confirmTransaction(signature, CONFIG.commitment);
        
        if (confirmation.value.err) {
            throw new Error('Transaction failed: ' + JSON.stringify(confirmation.value.err));
        }
        
        console.log('✅ Token created successfully!');
        
        // Set mint for return value
        mint = { publicKey: mintKeypair.publicKey };
        
        console.log('✅ Tokens minted successfully to your wallet!');
        
        // Return token info
        const tokenInfo = {
            mint: mint.publicKey.toString(),  // mint is a Token instance, need .publicKey
            name: tokenData.name,
            symbol: tokenData.symbol,
            decimals: tokenData.decimals,
            supply: tokenData.supply,
            description: tokenData.description,
            owner: wallet.publicKey.toString(),
            associatedTokenAccount: associatedTokenAccount.toString(),
            createdAt: new Date().toISOString()
        };
        
        console.log('🎉 Token created successfully:', tokenInfo);
        return tokenInfo;
        
    } catch (error) {
        console.error('❌ Error in createSPLToken:', error);
        throw error;
    }
}

/**
 * Clear the form
 */
function clearForm() {
    document.getElementById('token-form').reset();
    document.getElementById('token-decimals').value = '9'; // Reset default
}

/**
 * Load created tokens from localStorage
 */
async function loadCreatedTokens() {
    try {
        const stored = localStorage.getItem('createdTokens');
        if (stored) {
            createdTokens = JSON.parse(stored);
            updateTokensList();
        }
    } catch (error) {
        console.error('❌ Error loading tokens:', error);
    }
}

/**
 * Update the tokens list display
 */
function updateTokensList() {
    const container = document.getElementById('tokens-container');
    
    if (createdTokens.length === 0) {
        container.innerHTML = `
            <p style="color: #666; text-align: center; padding: 20px;">
                No tokens created yet. Create your first token above!
            </p>
        `;
        return;
    }
    
    const tokensHTML = createdTokens.map(token => `
        <div class="token-item">
            <div class="token-info">
                <h4>${token.name} (${token.symbol})</h4>
                <div class="token-mint">Mint: ${token.mint}</div>
                <div style="font-size: 12px; color: #888; margin-top: 5px;">
                    Created: ${new Date(token.createdAt).toLocaleString()}
                </div>
            </div>
            <div class="token-supply">
                ${token.supply.toLocaleString()} tokens
            </div>
        </div>
    `).join('');
    
    container.innerHTML = tokensHTML;
}

/**
 * Show status message
 */
function showStatus(type, message) {
    const container = document.getElementById('status-container');
    
    const statusDiv = document.createElement('div');
    statusDiv.className = `status-message ${type}`;
    statusDiv.textContent = message;
    
    // Clear existing status
    container.innerHTML = '';
    container.appendChild(statusDiv);
    
    // Auto-hide success/info messages after 10 seconds
    if (type === 'success' || type === 'info') {
        setTimeout(() => {
            if (container.contains(statusDiv)) {
                statusDiv.style.opacity = '0';
                setTimeout(() => {
                    if (container.contains(statusDiv)) {
                        container.removeChild(statusDiv);
                    }
                }, 300);
            }
        }, 10000);
    }
} 