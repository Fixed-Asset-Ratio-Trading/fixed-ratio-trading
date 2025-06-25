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
    await initializeApp();
});

/**
 * Initialize the application
 */
async function initializeApp() {
    try {
        // Initialize Solana connection
        connection = new solanaWeb3.Connection(CONFIG.rpcUrl, CONFIG.commitment);
        
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
    const requiredInputs = form.querySelectorAll('input[required]');
    
    let allValid = isConnected;
    
    requiredInputs.forEach(input => {
        if (!input.value.trim()) {
            allValid = false;
        }
    });
    
    createBtn.disabled = !allValid;
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
        
        showStatus('success', `🎉 Token "${formData.name}" created successfully! Mint: ${tokenInfo.mint}`);
        
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
        
        // Generate a new keypair for the token mint
        const mint = solanaWeb3.Keypair.generate();
        
        console.log('🔑 Generated mint keypair:', mint.publicKey.toString());
        
        // Calculate the minimum balance for a mint account
        const mintRent = await connection.getMinimumBalanceForRentExemption(
            splToken.MintLayout.span
        );
        
        console.log('💰 Mint rent:', mintRent, 'lamports');
        
        // Create mint account
        const createMintAccountInstruction = solanaWeb3.SystemProgram.createAccount({
            fromPubkey: wallet.publicKey,
            newAccountPubkey: mint.publicKey,
            lamports: mintRent,
            space: splToken.MintLayout.span,
            programId: splToken.TOKEN_PROGRAM_ID,
        });
        
        // Initialize mint
        const initializeMintInstruction = splToken.createInitializeMintInstruction(
            mint.publicKey,
            tokenData.decimals,
            wallet.publicKey, // mint authority
            wallet.publicKey  // freeze authority (optional, can be null)
        );
        
        // Create associated token account for the wallet
        const associatedTokenAccount = await splToken.getAssociatedTokenAddress(
            mint.publicKey,
            wallet.publicKey
        );
        
        const createAssociatedTokenAccountInstruction = 
            splToken.createAssociatedTokenAccountInstruction(
                wallet.publicKey,     // payer
                associatedTokenAccount, // associated token account
                wallet.publicKey,     // owner
                mint.publicKey        // mint
            );
        
        // Mint tokens to the associated token account
        const mintToInstruction = splToken.createMintToInstruction(
            mint.publicKey,
            associatedTokenAccount,
            wallet.publicKey, // mint authority
            tokenData.supply * Math.pow(10, tokenData.decimals) // amount (adjusted for decimals)
        );
        
        // Create transaction
        const transaction = new solanaWeb3.Transaction()
            .add(createMintAccountInstruction)
            .add(initializeMintInstruction)
            .add(createAssociatedTokenAccountInstruction)
            .add(mintToInstruction);
        
        // Set recent blockhash and fee payer
        const { blockhash } = await connection.getLatestBlockhash();
        transaction.recentBlockhash = blockhash;
        transaction.feePayer = wallet.publicKey;
        
        // Sign transaction (both wallet and mint keypair need to sign)
        transaction.partialSign(mint);
        
        console.log('📝 Transaction created, requesting wallet signature...');
        
        // Sign and send transaction
        const signedTransaction = await wallet.signTransaction(transaction);
        const signature = await connection.sendRawTransaction(signedTransaction.serialize());
        
        console.log('📡 Transaction sent:', signature);
        showStatus('info', `Transaction submitted: ${signature}`);
        
        // Confirm transaction
        const confirmation = await connection.confirmTransaction(signature, CONFIG.commitment);
        
        if (confirmation.value.err) {
            throw new Error('Transaction failed: ' + JSON.stringify(confirmation.value.err));
        }
        
        console.log('✅ Transaction confirmed');
        
        // Return token info
        const tokenInfo = {
            mint: mint.publicKey.toString(),
            name: tokenData.name,
            symbol: tokenData.symbol,
            decimals: tokenData.decimals,
            supply: tokenData.supply,
            description: tokenData.description,
            owner: wallet.publicKey.toString(),
            associatedTokenAccount: associatedTokenAccount.toString(),
            signature: signature,
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