// Pool Creation Dashboard - JavaScript Logic
// Handles Backpack wallet connection, token fetching, and pool creation
// Configuration is loaded from config.js

// Global state
let connection = null;
let wallet = null;
let isConnected = false;
let userTokens = [];
let selectedTokenA = null;
let selectedTokenB = null;
let currentRatio = 1;
let errorCountdownTimer = null;

/**
 * Browser-compatible helper to concatenate Uint8Arrays (replaces Buffer.concat)
 */
function concatUint8Arrays(arrays) {
    const totalLength = arrays.reduce((sum, arr) => sum + arr.length, 0);
    const result = new Uint8Array(totalLength);
    let offset = 0;
    for (const arr of arrays) {
        result.set(arr, offset);
        offset += arr.length;
    }
    return result;
}

// Initialize when page loads
document.addEventListener('DOMContentLoaded', async () => {
    console.log('🚀 Pool Creation Dashboard initializing...');
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
            showStatus('error', '❌ Failed to load required libraries. Please refresh the page.');
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
        // Initialize Solana connection with WebSocket configuration
        console.log('🔌 Connecting to Solana RPC...');
        const connectionConfig = {
            commitment: CONFIG.commitment,
            disableRetryOnRateLimit: CONFIG.disableRetryOnRateLimit || true
        };
        
        if (CONFIG.wsUrl) {
            console.log('📡 Using WebSocket endpoint:', CONFIG.wsUrl);
            connection = new solanaWeb3.Connection(CONFIG.rpcUrl, connectionConfig, CONFIG.wsUrl);
        } else {
            console.log('📡 Using HTTP polling (WebSocket disabled)');
            connectionConfig.wsEndpoint = false; // Explicitly disable WebSocket
            connection = new solanaWeb3.Connection(CONFIG.rpcUrl, connectionConfig);
        }
        
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
        
        // Check if already connected
        if (window.backpack.isConnected) {
            await handleWalletConnected();
        }
        
        console.log('✅ Pool Creation Dashboard initialized');
        clearStatus();
    } catch (error) {
        console.error('❌ Failed to initialize:', error);
        showStatus('error', 'Failed to initialize application: ' + error.message);
    }
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
        
        // Check balance
        await checkWalletBalance();
        
        // Load user tokens
        await loadUserTokens();
        
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
        selectedTokenA = null;
        selectedTokenB = null;
        
        // Update UI
        document.getElementById('wallet-info').style.display = 'none';
        document.getElementById('wallet-disconnected').style.display = 'flex';
        document.getElementById('connect-wallet-btn').textContent = 'Connect Backpack Wallet';
        document.getElementById('connect-wallet-btn').onclick = connectWallet;
        
        // Reset tokens section
        document.getElementById('tokens-loading').style.display = 'block';
        document.getElementById('tokens-container').style.display = 'none';
        document.getElementById('tokens-loading').innerHTML = `
            <h3>🔍 Loading your tokens...</h3>
            <p>Please connect your wallet to see your token balances</p>
        `;
        
        // Reset pool creation section
        resetPoolCreation();
        
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
 * Load user's SPL tokens
 */
async function loadUserTokens() {
    try {
        showStatus('info', '🔍 Loading your tokens...');
        
        // Get all token accounts for the user
        const tokenAccounts = await connection.getParsedTokenAccountsByOwner(
            wallet.publicKey,
            { programId: window.splToken.TOKEN_PROGRAM_ID }
        );
        
        console.log(`Found ${tokenAccounts.value.length} token accounts`);
        
        userTokens = [];
        
        for (const tokenAccount of tokenAccounts.value) {
            const accountInfo = tokenAccount.account.data.parsed.info;
            
            // Skip accounts with zero balance
            if (parseFloat(accountInfo.tokenAmount.uiAmount) <= 0) {
                continue;
            }
            
            try {
                // Try to get token metadata (name, symbol)
                const mintAddress = accountInfo.mint;
                let tokenInfo = {
                    mint: mintAddress,
                    balance: parseFloat(accountInfo.tokenAmount.uiAmount),
                    decimals: accountInfo.tokenAmount.decimals,
                    symbol: `TOKEN-${mintAddress.slice(0, 4)}`, // Default symbol
                    name: `Token ${mintAddress.slice(0, 8)}...`, // Default name
                    tokenAccount: tokenAccount.pubkey.toString()
                };
                
                // Try to fetch metadata from common sources
                await tryFetchTokenMetadata(tokenInfo);
                
                userTokens.push(tokenInfo);
            } catch (error) {
                console.warn(`Failed to process token ${accountInfo.mint}:`, error);
            }
        }
        
        console.log(`✅ Loaded ${userTokens.length} tokens with balances`);
        
        // Update UI
        updateTokensDisplay();
        
        if (userTokens.length === 0) {
            showStatus('info', '📭 No tokens found in your wallet. Create some tokens first!');
        } else {
            clearStatus();
        }
        
    } catch (error) {
        console.error('❌ Error loading tokens:', error);
        showStatus('error', 'Failed to load tokens: ' + error.message);
    }
}

/**
 * Try to fetch token metadata from various sources
 */
async function tryFetchTokenMetadata(tokenInfo) {
    try {
        // Check if this is a token we created (stored in sessionStorage)
        const createdTokens = JSON.parse(sessionStorage.getItem('createdTokens') || '[]');
        const createdToken = createdTokens.find(t => t.mint === tokenInfo.mint);
        
        if (createdToken) {
            tokenInfo.symbol = createdToken.symbol;
            tokenInfo.name = createdToken.name;
            return;
        }
        
        // For now, use default values. In a real app, you'd query token registries or metadata programs
        console.log(`Using default metadata for token ${tokenInfo.mint}`);
        
    } catch (error) {
        console.warn('Error fetching token metadata:', error);
    }
}

/**
 * Update tokens display
 */
function updateTokensDisplay() {
    const tokensContainer = document.getElementById('tokens-container');
    const tokensLoading = document.getElementById('tokens-loading');
    
    if (userTokens.length === 0) {
        tokensLoading.style.display = 'block';
        tokensContainer.style.display = 'none';
        tokensLoading.innerHTML = `
            <h3>📭 No tokens found</h3>
            <p>You don't have any SPL tokens in your wallet.</p>
            <p><a href="token-creation.html">Create some tokens</a> first to start creating pools!</p>
        `;
        return;
    }
    
    tokensLoading.style.display = 'none';
    tokensContainer.style.display = 'grid';
    tokensContainer.innerHTML = '';
    
    userTokens.forEach((token, index) => {
        const tokenCard = document.createElement('div');
        tokenCard.className = 'token-card';
        tokenCard.onclick = () => selectToken(token);
        
        // Check if token is selected
        const isSelectedA = selectedTokenA && selectedTokenA.mint === token.mint;
        const isSelectedB = selectedTokenB && selectedTokenB.mint === token.mint;
        
        if (isSelectedA || isSelectedB) {
            tokenCard.classList.add('selected');
        }
        
        tokenCard.innerHTML = `
            <div class="token-header">
                <div class="token-symbol">${token.symbol}</div>
                <div class="token-balance">${token.balance.toLocaleString()}</div>
            </div>
            <div class="token-name">${token.name}</div>
            <div class="token-mint">${token.mint.slice(0, 20)}...</div>
            ${(isSelectedA || isSelectedB) ? `<div class="selected-badge">${isSelectedA ? 'A' : 'B'}</div>` : ''}
        `;
        
        tokensContainer.appendChild(tokenCard);
    });
}

/**
 * Select a token for the pool
 */
function selectToken(token) {
    // If no tokens selected, this becomes Token A
    if (!selectedTokenA && !selectedTokenB) {
        selectedTokenA = token;
        showStatus('success', `Selected ${token.symbol} as Token A`);
    }
    // If only Token A is selected, this becomes Token B (unless it's the same token)
    else if (selectedTokenA && !selectedTokenB) {
        if (selectedTokenA.mint === token.mint) {
            showStatus('error', 'Cannot select the same token for both positions');
            return;
        }
        selectedTokenB = token;
        showStatus('success', `Selected ${token.symbol} as Token B`);
    }
    // If both are selected, replace the most recently clicked one
    else {
        // For simplicity, let's replace Token B
        if (selectedTokenA.mint === token.mint) {
            showStatus('info', `${token.symbol} is already selected as Token A`);
            return;
        }
        selectedTokenB = token;
        showStatus('success', `Replaced Token B with ${token.symbol}`);
    }
    
    updateTokensDisplay();
    updatePoolCreationDisplay();
}

/**
 * Update pool creation display based on selected tokens
 */
function updatePoolCreationDisplay() {
    const tokenASelection = document.getElementById('token-a-selection');
    const tokenBSelection = document.getElementById('token-b-selection');
    const swapButton = document.getElementById('swap-tokens-btn');
    const ratioSection = document.getElementById('ratio-section');
    
    // Update Token A display
    if (selectedTokenA) {
        tokenASelection.className = 'token-selection active';
        tokenASelection.innerHTML = `
            <div class="selected-token-symbol">${selectedTokenA.symbol}</div>
            <div class="selected-token-name">${selectedTokenA.name}</div>
            <div class="selected-token-balance">Balance: ${selectedTokenA.balance.toLocaleString()}</div>
        `;
    } else {
        tokenASelection.className = 'token-selection empty';
        tokenASelection.innerHTML = '<div class="empty-selection">Select Token A</div>';
    }
    
    // Update Token B display
    if (selectedTokenB) {
        tokenBSelection.className = 'token-selection active';
        tokenBSelection.innerHTML = `
            <div class="selected-token-symbol">${selectedTokenB.symbol}</div>
            <div class="selected-token-name">${selectedTokenB.name}</div>
            <div class="selected-token-balance">Balance: ${selectedTokenB.balance.toLocaleString()}</div>
        `;
    } else {
        tokenBSelection.className = 'token-selection empty';
        tokenBSelection.innerHTML = '<div class="empty-selection">Select Token B</div>';
    }
    
    // Enable swap button if both tokens are selected
    swapButton.disabled = !selectedTokenA || !selectedTokenB;
    
    // Show ratio section if both tokens are selected
    if (selectedTokenA && selectedTokenB) {
        ratioSection.style.display = 'block';
        updateRatioDisplay();
        updatePoolSummary();
        updateCreateButtonState();
    } else {
        ratioSection.style.display = 'none';
        document.getElementById('pool-summary-section').style.display = 'none';
    }
}

/**
 * Swap Token A and Token B
 */
function swapTokens() {
    if (!selectedTokenA || !selectedTokenB) return;
    
    const temp = selectedTokenA;
    selectedTokenA = selectedTokenB;
    selectedTokenB = temp;
    
    showStatus('info', `Swapped tokens: ${selectedTokenA.symbol} ⇄ ${selectedTokenB.symbol}`);
    
    updateTokensDisplay();
    updatePoolCreationDisplay();
}

/**
 * Update ratio display
 */
function updateRatioDisplay() {
    if (!selectedTokenA || !selectedTokenB) return;
    
    const ratioInput = document.getElementById('ratio-input');
    currentRatio = parseFloat(ratioInput.value) || 1;
    
    // Use display utilities to show user-friendly ordering in ratio display
    const display = window.TokenDisplayUtils.getSimpleDisplayOrder(
        selectedTokenA.symbol, 
        selectedTokenB.symbol, 
        Math.floor(currentRatio), 
        1
    );
    
    // Update display elements with user-friendly ordering
    document.getElementById('ratio-token-a').textContent = display.baseToken;
    document.getElementById('ratio-token-b').textContent = display.quoteToken;
    document.getElementById('ratio-value').textContent = window.TokenDisplayUtils.formatExchangeRate(display.exchangeRate);
    document.getElementById('ratio-input-label').textContent = display.quoteToken;
    
    // Update pool summary
    updatePoolSummary();
}

/**
 * Update pool summary display
 */
function updatePoolSummary() {
    if (!selectedTokenA || !selectedTokenB) return;
    
    const summarySection = document.getElementById('pool-summary-section');
    const summaryPair = document.getElementById('summary-pair');
    const summaryRate = document.getElementById('summary-rate');
    
    // Use display utilities to show user-friendly ordering
    const display = window.TokenDisplayUtils.getSimpleDisplayOrder(
        selectedTokenA.symbol, 
        selectedTokenB.symbol, 
        Math.floor(currentRatio), 
        1
    );
    
    summaryPair.textContent = display.displayPair;
    summaryRate.textContent = display.rateText;
    
    summarySection.style.display = 'block';
}

/**
 * Update create pool button state
 */
function updateCreateButtonState() {
    const createBtn = document.getElementById('create-pool-btn');
    
    const canCreate = isConnected && 
                     selectedTokenA && 
                     selectedTokenB &&
                     currentRatio > 0;
    
    createBtn.disabled = !canCreate;
}

/**
 * Show token selection help
 */
function showTokenHelp(position) {
    if (!isConnected) {
        showStatus('info', 'Please connect your wallet first to see your tokens');
        return;
    }
    
    if (userTokens.length === 0) {
        showStatus('info', 'No tokens found in your wallet. Create some tokens first!');
        return;
    }
    
    showStatus('info', `Click on a token card above to select it as Token ${position}`);
}

/**
 * Reset pool creation state
 */
function resetPoolCreation() {
    selectedTokenA = null;
    selectedTokenB = null;
    currentRatio = 1;
    
    document.getElementById('ratio-input').value = '1';
    
    updatePoolCreationDisplay();
}

/**
 * Create the pool
 */
async function createPool() {
    // Hide any existing errors
    hidePoolError();
    
    if (!isConnected || !selectedTokenA || !selectedTokenB) {
        showPoolError('Please connect wallet and select two tokens');
        return;
    }
    
    if (currentRatio <= 0) {
        showPoolError('Please enter a valid exchange ratio');
        return;
    }
    
    // Check for duplicate pools
    if (await checkDuplicatePool(selectedTokenA, selectedTokenB, currentRatio)) {
        showPoolError(`Pool already exists: ${selectedTokenA.symbol}/${selectedTokenB.symbol} with ratio 1:${currentRatio}. Each token pair with the same ratio can only have one pool.`);
        return;
    }
    
    const createBtn = document.getElementById('create-pool-btn');
    const originalText = createBtn.textContent;
    
    try {
        createBtn.disabled = true;
        createBtn.textContent = '🔄 Creating Pool...';
        
        showStatus('info', `Creating pool: ${selectedTokenA.symbol}/${selectedTokenB.symbol} with ratio 1:${currentRatio}...`);
        
        // Call the smart contract to create the pool
        const poolData = await createPoolTransaction(selectedTokenA, selectedTokenB, currentRatio);
        
        // Redirect to pool success page with pool details
        const params = new URLSearchParams({
            poolAddress: poolData.poolId,
            tokenASymbol: selectedTokenA.symbol,
            tokenBSymbol: selectedTokenB.symbol,
            tokenAName: selectedTokenA.name,
            tokenBName: selectedTokenB.name,
            ratio: currentRatio,
            creator: wallet.publicKey.toString(),
            createdAt: new Date().toISOString()
        });
        
        window.location.href = `pool-success.html?${params.toString()}`;
        
    } catch (error) {
        console.error('❌ Error creating pool:', error);
        
        // Show detailed error with specific messaging
        let errorMessage = error.message;
        
        if (error.message.includes('User rejected')) {
            errorMessage = 'Transaction was rejected by user. Please try again and approve the transaction in your Backpack wallet.';
        } else if (error.message.includes('Insufficient funds')) {
            errorMessage = 'Insufficient SOL balance to pay for pool creation fee and transaction costs. Please add more SOL to your wallet.';
        } else if (error.message.includes('Network error')) {
            errorMessage = 'Network connection error. Please check your internet connection and try again.';
        } else if (error.message.includes('Program not deployed')) {
            errorMessage = 'Fixed Ratio Trading program is not deployed on this network. Please deploy the program first or switch to a network where it is deployed.';
        }
        
        showPoolError(errorMessage);
    } finally {
        createBtn.disabled = false;
        createBtn.textContent = originalText;
        updateCreateButtonState();
    }
}

/**
 * Confirm transaction with extended timeout and progress updates
 */
async function confirmTransactionWithProgress(signature, commitment = 'confirmed') {
    const maxRetries = 60; // 60 attempts = up to 2 minutes
    const retryDelay = 2000; // 2 seconds between checks
    let attempts = 0;
    
    while (attempts < maxRetries) {
        try {
            const confirmation = await connection.confirmTransaction(signature, commitment);
            
            if (confirmation.value) {
                console.log('✅ Transaction confirmed after', attempts + 1, 'attempts');
                showStatus('success', `Transaction confirmed! Processing completed.`);
                return confirmation;
            }
        } catch (error) {
            // If it's a timeout error, continue retrying
            if (error.message.includes('was not confirmed') || error.message.includes('timeout')) {
                attempts++;
                const timeElapsed = (attempts * retryDelay) / 1000;
                console.log(`⏳ Still waiting for confirmation... (${timeElapsed}s elapsed)`);
                showStatus('info', `⏳ Transaction processing... ${timeElapsed}s elapsed (will wait up to 2 minutes)`);
                
                // Wait before next attempt
                await new Promise(resolve => setTimeout(resolve, retryDelay));
                continue;
            } else {
                // For other errors, throw immediately
                throw error;
            }
        }
        
        attempts++;
        const timeElapsed = (attempts * retryDelay) / 1000;
        
        // Check transaction status manually
        try {
            const status = await connection.getSignatureStatus(signature);
            if (status && status.value) {
                if (status.value.err) {
                    throw new Error('Transaction failed: ' + JSON.stringify(status.value.err));
                }
                if (status.value.confirmationStatus === commitment || 
                    status.value.confirmationStatus === 'finalized') {
                    console.log('✅ Transaction confirmed via status check');
                    showStatus('success', `Transaction confirmed! Processing completed.`);
                    return { value: status.value };
                }
            }
        } catch (statusError) {
            console.log('ℹ️ Could not check transaction status:', statusError.message);
        }
        
        console.log(`⏳ Still waiting for confirmation... (${timeElapsed}s elapsed)`);
        showStatus('info', `⏳ Transaction processing... ${timeElapsed}s elapsed (will wait up to 2 minutes)`);
        
        // Wait before next attempt
        await new Promise(resolve => setTimeout(resolve, retryDelay));
    }
    
    // If we get here, we've exhausted all retries
    throw new Error(`Transaction was not confirmed after ${(maxRetries * retryDelay) / 1000} seconds. Check signature ${signature} manually using Solana Explorer.`);
}

/**
 * Create pool transaction
 */
async function createPoolTransaction(tokenA, tokenB, ratio) {
    try {
        console.log('🏊‍♂️ Creating real pool transaction...');
        
        // Check if program is deployed
        const programId = new solanaWeb3.PublicKey(CONFIG.programId);
        const programAccount = await connection.getAccountInfo(programId);
        
        if (!programAccount) {
            throw new Error('Program not deployed: Fixed Ratio Trading program not found on this network. Please deploy the program first.');
        }
        
        // Use original token order - let smart contract do normalization internally
        const primaryTokenMint = new solanaWeb3.PublicKey(tokenA.mint); // User-selected Token A
        const baseTokenMint = new solanaWeb3.PublicKey(tokenB.mint);     // User-selected Token B
        const ratioPrimaryPerBase = ratio; // User-defined ratio
        
        console.log('Pool configuration (original order):', {
            primaryToken: primaryTokenMint.toString(),
            baseToken: baseTokenMint.toString(),
            ratio: ratioPrimaryPerBase
        });
        
        // Determine normalized token order for PDA derivation (same logic as smart contract)
        // Use string comparison for lexicographic ordering (same as Rust toString() comparison)
        const tokenAMint = primaryTokenMint.toString() < baseTokenMint.toString() 
            ? primaryTokenMint : baseTokenMint;
        const tokenBMint = primaryTokenMint.toString() < baseTokenMint.toString() 
            ? baseTokenMint : primaryTokenMint;
        
        console.log('Normalized token order (for PDA derivation):', {
            tokenA: tokenAMint.toString(),
            tokenB: tokenBMint.toString()
        });
        
        // Create pool state PDA - same derivation logic as smart contract
        const poolStatePDA = await solanaWeb3.PublicKey.findProgramAddress(
            [
                new TextEncoder().encode('pool_state'),
                tokenAMint.toBuffer(),
                tokenBMint.toBuffer(),
                new Uint8Array(new BigUint64Array([BigInt(ratioPrimaryPerBase)]).buffer),
                new Uint8Array(new BigUint64Array([BigInt(1)]).buffer) // ratio_b_denominator = 1
            ],
            programId
        );
        
        console.log('Pool state PDA:', poolStatePDA[0].toString());
        
        // ✅ SECURITY FIX: Derive LP token mint PDAs (controlled by smart contract)
        // This prevents users from creating fake LP tokens to drain pools
        const lpTokenAMintPDA = await solanaWeb3.PublicKey.findProgramAddress(
            [
                new TextEncoder().encode('lp_token_a_mint'),
                poolStatePDA[0].toBuffer()
            ],
            programId
        );
        
        const lpTokenBMintPDA = await solanaWeb3.PublicKey.findProgramAddress(
            [
                new TextEncoder().encode('lp_token_b_mint'),
                poolStatePDA[0].toBuffer()
            ],
            programId
        );
        
        console.log('🔒 SECURE LP token mints (PDAs controlled by smart contract):', {
            lpTokenA: lpTokenAMintPDA[0].toString(),
            lpTokenB: lpTokenBMintPDA[0].toString()
        });
        
        // Create token vault PDAs
        const tokenAVaultPDA = await solanaWeb3.PublicKey.findProgramAddress(
            [
                new TextEncoder().encode('token_a_vault'),
                poolStatePDA[0].toBuffer()
            ],
            programId
        );
        
        const tokenBVaultPDA = await solanaWeb3.PublicKey.findProgramAddress(
            [
                new TextEncoder().encode('token_b_vault'),
                poolStatePDA[0].toBuffer()
            ],
            programId
        );
        
        console.log('Token vault PDAs:', {
            tokenAVault: tokenAVaultPDA[0].toString(),
            tokenBVault: tokenBVaultPDA[0].toString()
        });
        
        // Get main treasury PDA
        const mainTreasuryPDA = await solanaWeb3.PublicKey.findProgramAddress(
            [new TextEncoder().encode('main_treasury')],
            programId
        );
        
        console.log('Main treasury PDA:', mainTreasuryPDA[0].toString());
        
        // Create instruction data for InitializePool
        const instructionData = concatUint8Arrays([
            new Uint8Array([0]), // InitializePool instruction discriminator
            new Uint8Array(new BigUint64Array([BigInt(ratioPrimaryPerBase)]).buffer), // ratio_a_numerator  
            new Uint8Array(new BigUint64Array([BigInt(1)]).buffer) // ratio_b_denominator
        ]);
        
        // Check if accounts already exist
        const existingPoolAccount = await connection.getAccountInfo(poolStatePDA[0]);
        const existingTokenAVault = await connection.getAccountInfo(tokenAVaultPDA[0]);
        const existingTokenBVault = await connection.getAccountInfo(tokenBVaultPDA[0]);
        
        if (existingPoolAccount) {
            console.log('WARNING: Pool state account already exists:', poolStatePDA[0].toString());
        }
        if (existingTokenAVault) {
            console.log('WARNING: Token A vault already exists:', tokenAVaultPDA[0].toString());
        }
        if (existingTokenBVault) {
            console.log('WARNING: Token B vault already exists:', tokenBVaultPDA[0].toString());
        }
        
        // ✅ SECURITY FIX: Updated account structure to match smart contract's Phase 11 ultra-secure pattern
        // 12 accounts total, LP token mints are now PDAs, not user-controlled keypairs
        const createPoolInstruction = new solanaWeb3.TransactionInstruction({
            keys: [
                { pubkey: wallet.publicKey, isSigner: true, isWritable: true },           // 0: Authority/User Signer
                { pubkey: solanaWeb3.SystemProgram.programId, isSigner: false, isWritable: false }, // 1: System Program
                { pubkey: solanaWeb3.SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },      // 2: Rent Sysvar
                { pubkey: poolStatePDA[0], isSigner: false, isWritable: true },           // 3: Pool State PDA
                { pubkey: primaryTokenMint, isSigner: false, isWritable: false },         // 4: First Token Mint
                { pubkey: baseTokenMint, isSigner: false, isWritable: false },            // 5: Second Token Mint
                { pubkey: tokenAVaultPDA[0], isSigner: false, isWritable: true },         // 6: Token A Vault PDA
                { pubkey: tokenBVaultPDA[0], isSigner: false, isWritable: true },         // 7: Token B Vault PDA
                { pubkey: window.splToken.TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },   // 8: SPL Token Program
                { pubkey: mainTreasuryPDA[0], isSigner: false, isWritable: true },        // 9: Main Treasury PDA
                { pubkey: lpTokenAMintPDA[0], isSigner: false, isWritable: true },        // 10: LP Token A Mint PDA
                { pubkey: lpTokenBMintPDA[0], isSigner: false, isWritable: true }         // 11: LP Token B Mint PDA
            ],
            programId: programId,
            data: instructionData
        });
        
        // Create compute budget instruction for pool creation (500K CUs)
        const computeBudgetInstruction = solanaWeb3.ComputeBudgetProgram.setComputeUnitLimit({
            units: 500_000
        });
        
        // Create transaction with compute budget and pool creation instruction
        const transaction = new solanaWeb3.Transaction()
            .add(computeBudgetInstruction)
            .add(createPoolInstruction);
        
        // Set recent blockhash and fee payer
        transaction.recentBlockhash = (await connection.getRecentBlockhash()).blockhash;
        transaction.feePayer = wallet.publicKey;
        
        // ✅ SECURITY FIX: No longer need to sign with LP token mint keypairs
        // LP token mints are now PDAs controlled by the smart contract
        console.log('🔒 SECURITY: LP token mints are now PDAs controlled by the smart contract');
        console.log('   This prevents users from creating fake LP tokens to drain pools');
        
        // Sign and send transaction
        const signature = await wallet.signAndSendTransaction(transaction);
        console.log('✅ Pool creation transaction sent:', signature);
        
        // Wait for confirmation
        const confirmation = await connection.confirmTransaction(signature, 'confirmed');
        
        if (confirmation.value.err) {
            throw new Error(`Pool creation failed: ${JSON.stringify(confirmation.value.err)}`);
        }
        
        console.log('✅ Pool created successfully!');
        console.log('Pool details:', {
            poolId: poolStatePDA[0].toString(),
            tokenA: tokenAMint.toString(),
            tokenB: tokenBMint.toString(),
            ratio: ratioPrimaryPerBase,
            lpTokenAMint: lpTokenAMintPDA[0].toString(),
            lpTokenBMint: lpTokenBMintPDA[0].toString()
        });
        
        return {
            success: true,
            poolId: poolStatePDA[0].toString(),
            signature: signature,
            lpTokenAMint: lpTokenAMintPDA[0].toString(),
            lpTokenBMint: lpTokenBMintPDA[0].toString(),
            tokenAVault: tokenAVaultPDA[0].toString(),
            tokenBVault: tokenBVaultPDA[0].toString()
        };
        
    } catch (error) {
        console.error('❌ Pool creation error:', error);
        throw error;
    }
}

/**
 * Generate a mock pool address based on token mints and ratio
 */
function generateMockPoolAddress(tokenAMint, tokenBMint, ratio) {
    // Simple hash-like generation for demo purposes
    const combined = tokenAMint + tokenBMint + ratio.toString();
    const chars = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
    let result = '';
    for (let i = 0; i < 44; i++) {
        const index = (combined.charCodeAt(i % combined.length) + i) % chars.length;
        result += chars.charAt(index);
    }
    return result;
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

/**
 * Show error with countdown and copy functionality
 */
function showPoolError(errorMessage) {
    // Clear any existing countdown
    if (errorCountdownTimer) {
        clearInterval(errorCountdownTimer);
    }
    
    const errorDisplay = document.getElementById('error-display');
    const errorMessageText = document.getElementById('error-message-text');
    const errorCountdown = document.getElementById('error-countdown');
    
    // Set error message
    errorMessageText.textContent = errorMessage;
    
    // Show error display
    errorDisplay.style.display = 'block';
    
    // Start countdown
    let countdown = 30;
    errorCountdown.textContent = countdown;
    
    errorCountdownTimer = setInterval(() => {
        countdown--;
        errorCountdown.textContent = countdown;
        
        if (countdown <= 0) {
            clearInterval(errorCountdownTimer);
            errorDisplay.style.display = 'none';
            errorCountdownTimer = null;
        }
    }, 1000);
    
    console.error('🚨 Pool creation error:', errorMessage);
}

/**
 * Hide error display
 */
function hidePoolError() {
    if (errorCountdownTimer) {
        clearInterval(errorCountdownTimer);
        errorCountdownTimer = null;
    }
    
    const errorDisplay = document.getElementById('error-display');
    errorDisplay.style.display = 'none';
}

/**
 * Copy error message to clipboard
 */
function copyErrorMessage() {
    const errorMessageText = document.getElementById('error-message-text');
    const message = errorMessageText.textContent;
    
    navigator.clipboard.writeText(message).then(() => {
        const copyBtn = document.getElementById('copy-error-btn');
        const originalText = copyBtn.textContent;
        copyBtn.textContent = '✅ Copied';
        
        setTimeout(() => {
            copyBtn.textContent = originalText;
        }, 2000);
    }).catch(err => {
        console.error('Failed to copy error message:', err);
    });
}

// Make function available globally for HTML onclick
window.copyErrorMessage = copyErrorMessage;

/**
 * Check if pool already exists (check RPC data by calculating expected pool address)
 */
async function checkDuplicatePool(tokenA, tokenB, ratio) {
    try {
        // Calculate what the pool address would be for this token pair and ratio
        const programId = new solanaWeb3.PublicKey(CONFIG.programId);
        
        // Determine normalized token order (same logic as smart contract)
        const tokenAMint = tokenA.mint < tokenB.mint ? 
            new solanaWeb3.PublicKey(tokenA.mint) : new solanaWeb3.PublicKey(tokenB.mint);
        const tokenBMint = tokenA.mint < tokenB.mint ? 
            new solanaWeb3.PublicKey(tokenB.mint) : new solanaWeb3.PublicKey(tokenA.mint);
        
        const ratioANumerator = Math.floor(ratio);
        const ratioBDenominator = 1;
        
        // Convert strings to bytes using TextEncoder
        const encoder = new TextEncoder();
        
        // Convert ratio numbers to little-endian bytes
        const ratioABytes = new Uint8Array(8);
        const ratioBBytes = new Uint8Array(8);
        new DataView(ratioABytes.buffer).setBigUint64(0, BigInt(ratioANumerator), true);
        new DataView(ratioBBytes.buffer).setBigUint64(0, BigInt(ratioBDenominator), true);
        
        const [poolStatePDA] = await solanaWeb3.PublicKey.findProgramAddress(
            [
                encoder.encode('pool_state'),
                tokenAMint.toBytes(), 
                tokenBMint.toBytes(),
                ratioABytes,
                ratioBBytes
            ],
            programId
        );
        
        // Check if this pool address already exists on-chain
        const poolAccount = await connection.getAccountInfo(poolStatePDA);
        
        if (poolAccount) {
            console.log('🚫 Pool already exists on-chain:', poolStatePDA.toString());
            return true;
        }
        
        console.log('✅ Pool does not exist, safe to create:', poolStatePDA.toString());
        return false;
        
    } catch (error) {
        console.warn('⚠️ Could not check for duplicate pools:', error);
        // Fallback to sessionStorage check if RPC fails
        const existingPools = JSON.parse(sessionStorage.getItem('createdPools') || '[]');
        
        return existingPools.some(pool => {
            const sameAB = pool.tokenAMint === tokenA.mint && 
                          pool.tokenBMint === tokenB.mint && 
                          pool.ratio === ratio;
            
            const sameBA = pool.tokenAMint === tokenB.mint && 
                          pool.tokenBMint === tokenA.mint && 
                          pool.ratio === (1 / ratio);
            
            return sameAB || sameBA;
        });
    }
} 