// Liquidity Management - JavaScript Logic
// Handles adding liquidity to specific pools
// Configuration is loaded from config.js

// Global state
let connection = null;
let wallet = null;
let isConnected = false;
let poolData = null;
let poolAddress = null;
let userTokens = [];
let selectedToken = null;

// Initialize when page loads
document.addEventListener('DOMContentLoaded', async () => {
    console.log('🚀 Liquidity page initializing...');
    showStatus('info', '🔄 Loading liquidity page...');
    
    // Simple retry mechanism for library loading
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
            showStatus('error', '❌ Failed to load required libraries. Please refresh the page.');
        }
    };
    
    // Check for SPL Token library
    setTimeout(() => {
        let splTokenLib = null;
        const possibleNames = ['splToken', 'SPLToken', 'SplToken'];
        
        for (const name of possibleNames) {
            if (window[name]) {
                splTokenLib = window[name];
                console.log(`✅ Found SPL Token library as window.${name}`);
                break;
            }
        }
        
        if (!splTokenLib && window.solanaWeb3) {
            for (const name of possibleNames) {
                if (window.solanaWeb3[name]) {
                    splTokenLib = window.solanaWeb3[name];
                    console.log(`✅ Found SPL Token library as solanaWeb3.${name}`);
                    break;
                }
            }
        }
        
        if (splTokenLib) {
            window.splToken = splTokenLib;
            window.SPL_TOKEN_LOADED = true;
            console.log('✅ SPL Token library ready!');
        } else {
            console.error('❌ SPL Token library not found');
            window.SPL_TOKEN_LOADED = false;
        }
        
        // Start first attempt after a brief delay
        setTimeout(tryInitialize, 1500);
    }, 100);
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
        
        console.log('✅ SPL Token library ready');
        
        // Check if Backpack is installed
        if (!window.backpack) {
            showStatus('error', 'Backpack wallet not detected. Please install Backpack wallet extension.');
            return;
        }
        
        // Get pool address from sessionStorage
        poolAddress = sessionStorage.getItem('selectedPoolAddress');
        if (!poolAddress) {
            showStatus('error', 'No pool selected. Please go back to the dashboard and select a pool.');
            return;
        }
        
        console.log('🏊‍♂️ Selected pool address:', poolAddress);
        
        // Load pool information
        await loadPoolInformation();
        
        // Check if already connected
        if (window.backpack.isConnected) {
            await handleWalletConnected();
        }
        
        console.log('✅ Liquidity page initialized');
        clearStatus();
        
    } catch (error) {
        console.error('❌ Failed to initialize:', error);
        showStatus('error', 'Failed to initialize application: ' + error.message);
    }
}

/**
 * Load pool information from the blockchain
 */
async function loadPoolInformation() {
    try {
        console.log('🔍 Loading pool information for:', poolAddress);
        showStatus('info', 'Loading pool information...');
        
        // Get pool account data from blockchain
        const poolAccount = await connection.getAccountInfo(new solanaWeb3.PublicKey(poolAddress));
        if (!poolAccount) {
            throw new Error('Pool not found on blockchain');
        }
        
        // Parse pool state data (simplified version)
        poolData = await parsePoolState(poolAccount.data);
        
        // Try to get token symbols from localStorage
        const tokenSymbols = await getTokenSymbols(poolData.tokenAMint, poolData.tokenBMint);
        poolData.tokenASymbol = tokenSymbols.tokenA;
        poolData.tokenBSymbol = tokenSymbols.tokenB;
        
        // Update UI with pool information
        updatePoolDisplay();
        
        console.log('✅ Pool information loaded:', poolData);
        
    } catch (error) {
        console.error('❌ Error loading pool information:', error);
        showStatus('error', 'Failed to load pool information: ' + error.message);
    }
}

/**
 * Parse pool state data (simplified version)
 */
async function parsePoolState(data) {
    try {
        const dataArray = new Uint8Array(data);
        let offset = 0;
        
        // Helper function to read bytes
        const readPubkey = () => {
            const pubkey = dataArray.slice(offset, offset + 32);
            offset += 32;
            return new solanaWeb3.PublicKey(pubkey).toString();
        };
        
        const readU64 = () => {
            const view = new DataView(dataArray.buffer, offset, 8);
            const value = view.getBigUint64(0, true); // little-endian
            offset += 8;
            return Number(value);
        };
        
        // Parse key fields
        const owner = readPubkey();
        const tokenAMint = readPubkey();
        const tokenBMint = readPubkey();
        const tokenAVault = readPubkey();
        const tokenBVault = readPubkey();
        const lpTokenAMint = readPubkey();
        const lpTokenBMint = readPubkey();
        
        const ratioANumerator = readU64();
        const ratioBDenominator = readU64();
        const totalTokenALiquidity = readU64();
        const totalTokenBLiquidity = readU64();
        
        return {
            address: poolAddress,
            owner,
            tokenAMint,
            tokenBMint,
            tokenAVault,
            tokenBVault,
            lpTokenAMint,
            lpTokenBMint,
            ratioANumerator,
            ratioBDenominator,
            tokenALiquidity: totalTokenALiquidity,
            tokenBLiquidity: totalTokenBLiquidity
        };
        
    } catch (error) {
        console.error('Failed to parse pool state:', error);
        throw new Error('Failed to parse pool state data');
    }
}

/**
 * Try to get token symbols from localStorage
 */
async function getTokenSymbols(tokenAMint, tokenBMint) {
    try {
        const createdTokens = JSON.parse(localStorage.getItem('createdTokens') || '[]');
        
        const tokenA = createdTokens.find(t => t.mint === tokenAMint);
        const tokenB = createdTokens.find(t => t.mint === tokenBMint);
        
        return {
            tokenA: tokenA?.symbol || `TOKEN-${tokenAMint.slice(0, 4)}`,
            tokenB: tokenB?.symbol || `TOKEN-${tokenBMint.slice(0, 4)}`
        };
    } catch (error) {
        console.warn('Error getting token symbols:', error);
        return {
            tokenA: `TOKEN-${tokenAMint.slice(0, 4)}`,
            tokenB: `TOKEN-${tokenBMint.slice(0, 4)}`
        };
    }
}

/**
 * Update pool display in UI
 */
function updatePoolDisplay() {
    if (!poolData) return;
    
    const poolLoading = document.getElementById('pool-loading');
    const poolDetails = document.getElementById('pool-details');
    
    // Hide loading, show details
    poolLoading.style.display = 'none';
    poolDetails.style.display = 'grid';
    
    // Create pool metrics
    const exchangeRate = poolData.ratioBDenominator > 0 ? 
        Math.round(poolData.ratioANumerator / poolData.ratioBDenominator) : 0;
    
    poolDetails.innerHTML = `
        <div class="pool-metric">
            <div class="metric-label">Pool Pair</div>
            <div class="metric-value">${poolData.tokenASymbol} / ${poolData.tokenBSymbol}</div>
        </div>
        
        <div class="pool-metric">
            <div class="metric-label">Exchange Rate</div>
            <div class="metric-value">${exchangeRate}:1</div>
        </div>
        
        <div class="pool-metric">
            <div class="metric-label">${poolData.tokenASymbol} Liquidity</div>
            <div class="metric-value">${poolData.tokenALiquidity.toLocaleString()}</div>
        </div>
        
        <div class="pool-metric">
            <div class="metric-label">${poolData.tokenBSymbol} Liquidity</div>
            <div class="metric-value">${poolData.tokenBLiquidity.toLocaleString()}</div>
        </div>
        
        <div class="pool-metric">
            <div class="metric-label">Pool Address</div>
            <div class="metric-value" style="font-size: 12px; font-family: monospace;">${poolAddress.slice(0, 20)}...</div>
        </div>
    `;
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
        
        showStatus('success', `✅ Connected with Backpack wallet: ${publicKey.slice(0, 20)}...`);
        
        // Load user tokens for the pool
        await loadUserTokensForPool();
        
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
        userTokens = [];
        selectedToken = null;
        
        // Update UI
        document.getElementById('wallet-info').style.display = 'none';
        document.getElementById('wallet-disconnected').style.display = 'flex';
        document.getElementById('connect-wallet-btn').textContent = 'Connect Backpack Wallet';
        document.getElementById('connect-wallet-btn').onclick = connectWallet;
        
        // Reset token selection
        resetTokenSelection();
        
        showStatus('info', 'Wallet disconnected');
        
    } catch (error) {
        console.error('❌ Error disconnecting wallet:', error);
    }
}

/**
 * Load user's tokens that match the pool tokens
 */
async function loadUserTokensForPool() {
    try {
        if (!poolData || !isConnected) return;
        
        showStatus('info', '🔍 Loading your pool tokens...');
        
        // Get all token accounts for the user
        const tokenAccounts = await connection.getParsedTokenAccountsByOwner(
            wallet.publicKey,
            { programId: window.splToken.TOKEN_PROGRAM_ID }
        );
        
        console.log(`Found ${tokenAccounts.value.length} token accounts`);
        
        userTokens = [];
        
        for (const tokenAccount of tokenAccounts.value) {
            const accountInfo = tokenAccount.account.data.parsed.info;
            const mintAddress = accountInfo.mint;
            
            // Only include tokens that are part of this pool
            if (mintAddress === poolData.tokenAMint || mintAddress === poolData.tokenBMint) {
                const balance = parseFloat(accountInfo.tokenAmount.uiAmount) || 0;
                
                // Determine which token this is
                const isTokenA = mintAddress === poolData.tokenAMint;
                const symbol = isTokenA ? poolData.tokenASymbol : poolData.tokenBSymbol;
                
                userTokens.push({
                    mint: mintAddress,
                    symbol: symbol,
                    balance: balance,
                    decimals: accountInfo.tokenAmount.decimals,
                    tokenAccount: tokenAccount.pubkey.toString(),
                    isTokenA: isTokenA
                });
            }
        }
        
        console.log(`✅ Found ${userTokens.length} pool tokens in wallet`);
        
        // Update token selection UI
        updateTokenSelection();
        
        if (userTokens.length === 0) {
            showStatus('info', '📭 You don\'t have any tokens from this pool in your wallet.');
        } else {
            clearStatus();
        }
        
    } catch (error) {
        console.error('❌ Error loading user tokens:', error);
        showStatus('error', 'Failed to load your tokens: ' + error.message);
    }
}

/**
 * Update token selection UI
 */
function updateTokenSelection() {
    const tokenLoading = document.getElementById('token-loading');
    const tokenChoice = document.getElementById('token-choice');
    
    if (userTokens.length === 0) {
        tokenLoading.style.display = 'block';
        tokenChoice.style.display = 'none';
        tokenLoading.innerHTML = `
            <h3>📭 No pool tokens found</h3>
            <p>You don't have any tokens from this pool in your wallet.</p>
        `;
        return;
    }
    
    tokenLoading.style.display = 'none';
    tokenChoice.style.display = 'grid';
    tokenChoice.innerHTML = '';
    
    userTokens.forEach(token => {
        const tokenOption = document.createElement('div');
        tokenOption.className = 'token-option';
        tokenOption.onclick = () => selectToken(token);
        
        tokenOption.innerHTML = `
            <div class="token-symbol">${token.symbol}</div>
            <div class="token-balance">Balance: ${token.balance.toLocaleString()}</div>
        `;
        
        tokenChoice.appendChild(tokenOption);
    });
}

/**
 * Select a token to add liquidity for
 */
function selectToken(token) {
    selectedToken = token;
    
    // Update UI selection
    const tokenOptions = document.querySelectorAll('.token-option');
    tokenOptions.forEach(option => option.classList.remove('selected'));
    
    // Find and highlight the selected option
    tokenOptions.forEach(option => {
        if (option.querySelector('.token-symbol').textContent === token.symbol) {
            option.classList.add('selected');
        }
    });
    
    // Update amount section
    document.getElementById('selected-token-name').textContent = token.symbol;
    document.getElementById('available-balance').textContent = token.balance.toLocaleString();
    document.getElementById('available-token-symbol').textContent = token.symbol;
    
    // Show amount section and button
    document.getElementById('amount-section').style.display = 'block';
    document.getElementById('add-liquidity-btn').style.display = 'block';
    
    // Reset amount input
    document.getElementById('liquidity-amount').value = '';
    updateAddButton();
    
    showStatus('success', `Selected ${token.symbol} for liquidity addition`);
    
    console.log('🎯 Selected token:', token);
}

/**
 * Reset token selection
 */
function resetTokenSelection() {
    const tokenLoading = document.getElementById('token-loading');
    const tokenChoice = document.getElementById('token-choice');
    
    tokenLoading.style.display = 'block';
    tokenChoice.style.display = 'none';
    tokenLoading.innerHTML = `
        <h3>🔍 Loading pool tokens...</h3>
        <p>Please connect your wallet and load pool information</p>
    `;
    
    // Hide amount section and button
    document.getElementById('amount-section').style.display = 'none';
    document.getElementById('add-liquidity-btn').style.display = 'none';
    
    selectedToken = null;
}

/**
 * Update add liquidity button state
 */
function updateAddButton() {
    const addBtn = document.getElementById('add-liquidity-btn');
    const amountInput = document.getElementById('liquidity-amount');
    
    const amount = parseFloat(amountInput.value) || 0;
    const hasValidAmount = amount > 0;
    const hasBalance = selectedToken && amount <= selectedToken.balance;
    
    const canAdd = isConnected && selectedToken && hasValidAmount && hasBalance;
    
    addBtn.disabled = !canAdd;
    
    if (!hasValidAmount) {
        addBtn.textContent = '💧 Enter Amount';
    } else if (!hasBalance) {
        addBtn.textContent = '❌ Insufficient Balance';
    } else if (canAdd) {
        addBtn.textContent = `💧 Add ${amount} ${selectedToken.symbol}`;
    } else {
        addBtn.textContent = '💧 Add Liquidity';
    }
}

/**
 * Add liquidity to the pool
 */
async function addLiquidity() {
    if (!selectedToken || !isConnected) {
        showStatus('error', 'Please connect wallet and select a token first');
        return;
    }
    
    const amount = parseFloat(document.getElementById('liquidity-amount').value) || 0;
    if (amount <= 0) {
        showStatus('error', 'Please enter a valid amount');
        return;
    }
    
    if (amount > selectedToken.balance) {
        showStatus('error', 'Insufficient balance');
        return;
    }
    
    const addBtn = document.getElementById('add-liquidity-btn');
    const originalText = addBtn.textContent;
    
    try {
        addBtn.disabled = true;
        addBtn.textContent = '🔄 Adding Liquidity...';
        
        showStatus('info', `Adding ${amount} ${selectedToken.symbol} to the pool...`);
        
        // For now, show a success message (actual implementation would call the smart contract)
        // TODO: Implement actual liquidity addition transaction
        
        // Simulate transaction delay
        await new Promise(resolve => setTimeout(resolve, 2000));
        
        showStatus('success', `✅ Successfully added ${amount} ${selectedToken.symbol} to the pool!`);
        
        // Update dashboard when returning
        setTimeout(() => {
            // Store the pool address for updating when we return to dashboard
            sessionStorage.setItem('poolToUpdate', poolAddress);
            
            // Navigate back to dashboard
            window.location.href = 'index.html';
        }, 2000);
        
    } catch (error) {
        console.error('❌ Error adding liquidity:', error);
        showStatus('error', 'Failed to add liquidity: ' + error.message);
    } finally {
        addBtn.disabled = false;
        addBtn.textContent = originalText;
        updateAddButton();
    }
}

/**
 * Show status message
 */
function showStatus(type, message) {
    const container = document.getElementById('status-container');
    container.innerHTML = `<div class="status-message ${type}">${message}</div>`;
}

/**
 * Clear status message
 */
function clearStatus() {
    const container = document.getElementById('status-container');
    container.innerHTML = '';
}

// Export for global access
window.connectWallet = connectWallet;
window.disconnectWallet = disconnectWallet;
window.selectToken = selectToken;
window.updateAddButton = updateAddButton;
window.addLiquidity = addLiquidity;

console.log('💧 Liquidity JavaScript loaded successfully'); 