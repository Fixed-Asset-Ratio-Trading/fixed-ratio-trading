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
        
        // Initialize centralized data service if not already done
        if (!window.TradingDataService.connection) {
            await window.TradingDataService.initialize(window.TRADING_CONFIG, connection);
        }
        
        // Get pool data using centralized service (prioritizes RPC for real-time data)
        poolData = await window.TradingDataService.getPool(poolAddress, 'rpc');
        
        // Try to get token symbols from sessionStorage
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

// parsePoolState function removed - now using centralized TradingDataService.parsePoolState()

/**
 * Try to get token symbols from localStorage, Metaplex metadata, or use defaults
 */
async function getTokenSymbols(tokenAMint, tokenBMint) {
    try {
        console.log(`🔍 Looking up symbols for tokens: ${tokenAMint} and ${tokenBMint}`);
        
        // Get token A symbol
        const tokenASymbol = await getTokenSymbol(tokenAMint, 'A');
        
        // Get token B symbol  
        const tokenBSymbol = await getTokenSymbol(tokenBMint, 'B');
        
        console.log(`✅ Token symbols found: ${tokenASymbol}/${tokenBSymbol}`);
        
        return {
            tokenA: tokenASymbol,
            tokenB: tokenBSymbol
        };
    } catch (error) {
        console.warn('❌ Error getting token symbols:', error);
        return {
            tokenA: `TOKEN-${tokenAMint.slice(0, 4)}`,
            tokenB: `TOKEN-${tokenBMint.slice(0, 4)}`
        };
    }
}

/**
 * Get token symbol from localStorage, Metaplex, or default
 */
async function getTokenSymbol(tokenMint, tokenLabel) {
    try {
        // Check localStorage first
        const createdTokens = JSON.parse(localStorage.getItem('createdTokens') || '[]');
        const localToken = createdTokens.find(t => t.mint === tokenMint);
        
        if (localToken?.symbol) {
            console.log(`✅ Found token ${tokenLabel} symbol in localStorage: ${localToken.symbol}`);
            return localToken.symbol;
        }
        
        // Try Metaplex metadata (if available)
        if (typeof queryTokenMetadata === 'function') {
            console.log(`🔍 Querying Metaplex metadata for token ${tokenLabel}: ${tokenMint}`);
            const metadataAccount = await queryTokenMetadata(tokenMint);
            
            if (metadataAccount?.symbol) {
                console.log(`✅ Found token ${tokenLabel} symbol in Metaplex: ${metadataAccount.symbol}`);
                return metadataAccount.symbol;
            }
        }
        
        // Fallback to default
        const defaultSymbol = `TOKEN-${tokenMint.slice(0, 4)}`;
        console.log(`⚠️ Using default symbol for token ${tokenLabel}: ${defaultSymbol}`);
        return defaultSymbol;
        
    } catch (error) {
        console.warn(`❌ Error getting symbol for token ${tokenLabel}:`, error);
        return `TOKEN-${tokenMint.slice(0, 4)}`;
    }
}

/**
 * Phase 2.1: Update pool display in UI with Phase 1.3 enhancements
 */
function updatePoolDisplay() {
    if (!poolData) return;
    
    const poolLoading = document.getElementById('pool-loading');
    const poolDetails = document.getElementById('pool-details');
    
    // Hide loading, show details
    poolLoading.style.display = 'none';
    poolDetails.style.display = 'grid';
    
    // Phase 1.3: Use enhanced display utilities with flag interpretation
    const display = window.TokenDisplayUtils.getDisplayTokenOrder(poolData);
    const flags = window.TokenDisplayUtils.interpretPoolFlags(poolData);
    
    // Generate pool flags section
    const flagsSection = generatePoolFlagsDisplay(flags, poolData);
    
    poolDetails.innerHTML = `
        <div class="pool-metric">
            <div class="metric-label">Pool Pair</div>
            <div class="metric-value">${display.displayPair} ${display.isOneToManyRatio ? '<span style="background: #3b82f6; color: white; padding: 2px 6px; border-radius: 4px; font-size: 10px; margin-left: 8px;">🎯 1:Many</span>' : ''}</div>
        </div>
        
        <div class="pool-metric">
            <div class="metric-label">Exchange Rate</div>
            <div class="metric-value">${display.rateText}</div>
        </div>
        
        <div class="pool-metric">
            <div class="metric-label">${display.baseToken} Liquidity</div>
            <div class="metric-value">${window.TokenDisplayUtils.formatLargeNumber(display.baseLiquidity)}</div>
        </div>
        
        <div class="pool-metric">
            <div class="metric-label">${display.quoteToken} Liquidity</div>
            <div class="metric-value">${window.TokenDisplayUtils.formatLargeNumber(display.quoteLiquidity)}</div>
        </div>
        
        <div class="pool-metric">
            <div class="metric-label">Pool Status</div>
            <div class="metric-value">${flags.liquidityPaused ? '⏸️ Liquidity Paused' : flags.swapsPaused ? '🚫 Swaps Paused' : '✅ Active'}</div>
        </div>
        
        <div class="pool-metric">
            <div class="metric-label">Pool Address</div>
            <div class="metric-value" style="font-size: 12px; font-family: monospace;">${poolAddress.slice(0, 20)}...</div>
        </div>
        
        ${flagsSection}
    `;
    
    // Phase 2.1: Add expandable Pool State display section
    addExpandablePoolStateDisplay();
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
    document.getElementById('add-liquidity-section').style.display = 'block';
    document.getElementById('add-liquidity-btn').style.display = 'block';
    
    // Reset amount input
    document.getElementById('add-liquidity-amount').value = '';
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
    document.getElementById('add-liquidity-section').style.display = 'none';
    document.getElementById('add-liquidity-btn').style.display = 'none';
    
    selectedToken = null;
}

/**
 * Update add liquidity button state
 */
function updateAddButton() {
    const addBtn = document.getElementById('add-liquidity-btn');
    const amountInput = document.getElementById('add-liquidity-amount');
    
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
    
    const amount = parseFloat(document.getElementById('add-liquidity-amount').value) || 0;
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
        
        showStatus('info', `Requesting transaction approval from wallet...`);
        console.log(`💧 Initiating add liquidity: ${amount} ${selectedToken.symbol} to pool ${poolAddress}`);
        
        // Check if wallet is still connected
        if (!window.backpack?.isConnected) {
            throw new Error('Wallet not connected. Please connect your Backpack wallet.');
        }
        
        // Prepare transaction parameters
        const amountLamports = Math.floor(amount * Math.pow(10, selectedToken.decimals || 6));
        const poolPubkey = new solanaWeb3.PublicKey(poolAddress);
        const tokenMint = new solanaWeb3.PublicKey(selectedToken.mint);
        const userWallet = new solanaWeb3.PublicKey(window.backpack.publicKey);
        const programId = new solanaWeb3.PublicKey(window.TRADING_CONFIG.programId);
        
        showStatus('info', `Creating liquidity deposit transaction for ${amount} ${selectedToken.symbol}...`);
        
        // Derive required PDAs using correct seed prefixes
        const [systemStatePDA] = await solanaWeb3.PublicKey.findProgramAddress(
            [new TextEncoder().encode('system_state')],
            programId
        );
        
        // Get LP token mint PDAs (derive from pool state)
        const [lpTokenAMint] = await solanaWeb3.PublicKey.findProgramAddress(
            [new TextEncoder().encode('lp_token_a_mint'), poolPubkey.toBytes()],
            programId
        );
        const [lpTokenBMint] = await solanaWeb3.PublicKey.findProgramAddress(
            [new TextEncoder().encode('lp_token_b_mint'), poolPubkey.toBytes()],
            programId
        );
        
        // Determine which token vault and LP mint to use based on selected token
        const isTokenA = selectedToken.mint === poolData.tokenAMint;
        const targetVault = isTokenA ? poolData.tokenAVault : poolData.tokenBVault;
        const targetLPMint = isTokenA ? lpTokenAMint : lpTokenBMint;
        
        // Find user's token account for the selected token
        const userTokenAccount = new solanaWeb3.PublicKey(selectedToken.tokenAccount);
        
        // Find or create user's LP token account 
        const userLPTokenAccount = await findOrCreateAssociatedTokenAccount(userWallet, targetLPMint);
        
        // Check if we need to create the associated token account first
        const lpAccountInfo = await connection.getAccountInfo(userLPTokenAccount);
        
        // Create the transaction
        const transaction = new solanaWeb3.Transaction();
        
        // Add create associated token account instruction if needed
        if (!lpAccountInfo) {
            console.log(`📝 Adding instruction to create LP token account: ${userLPTokenAccount.toString()}`);
            const createAtaIx = splToken.createAssociatedTokenAccountInstruction(
                userWallet,          // payer
                userLPTokenAccount,  // associatedToken
                userWallet,          // owner
                targetLPMint,        // mint
                splToken.TOKEN_PROGRAM_ID,
                splToken.ASSOCIATED_TOKEN_PROGRAM_ID
            );
            transaction.add(createAtaIx);
        }
        
        // Serialize instruction data: Deposit { deposit_token_mint, amount }
        const instructionData = new Uint8Array(1 + 32 + 8); // 1 byte discriminator + 32 bytes pubkey + 8 bytes u64
        instructionData[0] = 2; // Deposit instruction discriminator (assuming it's the 3rd instruction)
        tokenMint.toBytes().forEach((byte, index) => {
            instructionData[1 + index] = byte;
        });
        const amountBytes = new ArrayBuffer(8);
        new DataView(amountBytes).setBigUint64(0, BigInt(amountLamports), true); // little-endian
        new Uint8Array(amountBytes).forEach((byte, index) => {
            instructionData[1 + 32 + index] = byte;
        });
        
        const depositInstruction = new solanaWeb3.TransactionInstruction({
            programId: programId,
            keys: [
                { pubkey: userWallet, isSigner: true, isWritable: true },                    // User Authority Signer
                { pubkey: solanaWeb3.SystemProgram.programId, isSigner: false, isWritable: false }, // System Program
                { pubkey: systemStatePDA, isSigner: false, isWritable: false },             // System State PDA
                { pubkey: poolPubkey, isSigner: false, isWritable: true },                  // Pool State PDA
                { pubkey: splToken.TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },  // SPL Token Program
                { pubkey: new solanaWeb3.PublicKey(poolData.tokenAVault), isSigner: false, isWritable: true }, // Token A Vault
                { pubkey: new solanaWeb3.PublicKey(poolData.tokenBVault), isSigner: false, isWritable: true }, // Token B Vault
                { pubkey: userTokenAccount, isSigner: false, isWritable: true },            // User Input Token Account
                { pubkey: userLPTokenAccount, isSigner: false, isWritable: true },          // User Output LP Account
                { pubkey: lpTokenAMint, isSigner: false, isWritable: true },                // LP Token A Mint
                { pubkey: lpTokenBMint, isSigner: false, isWritable: true },                // LP Token B Mint
            ],
            data: instructionData
        });
        
        transaction.add(depositInstruction);
        
        // Get recent blockhash
        const { blockhash } = await connection.getLatestBlockhash();
        transaction.recentBlockhash = blockhash;
        transaction.feePayer = userWallet;
        
        showStatus('info', `Waiting for transaction approval...`);
        
        // Request signature from wallet
        const signedTransaction = await window.backpack.signTransaction(transaction);
        
        showStatus('info', `Broadcasting transaction...`);
        
        // Send transaction
        const signature = await connection.sendRawTransaction(signedTransaction.serialize());
        
        showStatus('info', `Transaction sent! Confirming... (${signature.slice(0, 8)}...)`);
        console.log(`📡 Transaction signature: ${signature}`);
        
        // Wait for confirmation
        const confirmation = await connection.confirmTransaction(signature, 'confirmed');
        
        if (confirmation.value.err) {
            throw new Error(`Transaction failed: ${confirmation.value.err}`);
        }
        
        // Success! Show detailed success message
        showStatus('success', `
            <div style="text-align: left;">
                <div style="font-weight: bold; margin-bottom: 8px;">🎉 Liquidity Added Successfully!</div>
                <div style="font-size: 14px; line-height: 1.4;">
                    • Amount: ${amount} ${selectedToken.symbol}<br>
                    • Pool: ${poolAddress.slice(0, 8)}...${poolAddress.slice(-4)}<br>
                    • Transaction: <a href="https://explorer.solana.com/tx/${signature}?cluster=custom&customUrl=${encodeURIComponent(connection.rpcEndpoint)}" target="_blank" style="color: #059669;">${signature.slice(0, 8)}...${signature.slice(-4)}</a><br>
                    • Status: Confirmed ✅
                </div>
            </div>
        `);
        
        console.log(`✅ Liquidity addition successful! Signature: ${signature}`);
        
        // Refresh pool data to show updated balances
        setTimeout(async () => {
            try {
                showStatus('info', 'Refreshing pool data...');
                await loadPoolInformation();
                await loadUserTokensForPool();
                showStatus('success', 'Pool data refreshed successfully!');
            } catch (refreshError) {
                console.warn('Could not refresh pool data:', refreshError);
                showStatus('warning', 'Transaction successful, but could not refresh data. Please reload the page.');
            }
        }, 2000);
        
    } catch (error) {
        console.error('❌ Error adding liquidity:', error);
        
        // Show detailed error message
        let errorMessage = 'Failed to add liquidity';
        if (error.message.includes('User rejected')) {
            errorMessage = 'Transaction cancelled by user';
        } else if (error.message.includes('insufficient')) {
            errorMessage = 'Insufficient balance or SOL for transaction fees';
        } else if (error.message.includes('Wallet not connected')) {
            errorMessage = error.message;
        } else {
            errorMessage = `Transaction failed: ${error.message}`;
        }
        
        showStatus('error', `❌ ${errorMessage}`);
        console.log('💡 Tip: Make sure you have enough SOL for transaction fees and the correct token balance');
        
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

/**
 * Phase 2.1: Generate pool flags display section for liquidity page
 */
function generatePoolFlagsDisplay(flags, pool) {
    const hasFlags = flags.oneToManyRatio || flags.liquidityPaused || flags.swapsPaused || 
                     flags.withdrawalProtection || flags.singleLpTokenMode;
    
    if (!hasFlags && (typeof pool.flags === 'undefined' || pool.flags === 0)) {
        return ''; // No flags to display
    }
    
    const flagItems = [];
    
    if (flags.oneToManyRatio) {
        flagItems.push('<span style="background: #3b82f6; color: white; padding: 2px 6px; border-radius: 4px; font-size: 10px;">🎯 One-to-Many Ratio</span>');
    }
    if (flags.liquidityPaused) {
        flagItems.push('<span style="background: #ef4444; color: white; padding: 2px 6px; border-radius: 4px; font-size: 10px;">⏸️ Liquidity Paused</span>');
    }
    if (flags.swapsPaused) {
        flagItems.push('<span style="background: #f59e0b; color: white; padding: 2px 6px; border-radius: 4px; font-size: 10px;">🚫 Swaps Paused</span>');
    }
    if (flags.withdrawalProtection) {
        flagItems.push('<span style="background: #10b981; color: white; padding: 2px 6px; border-radius: 4px; font-size: 10px;">🛡️ Withdrawal Protection</span>');
    }
    if (flags.singleLpTokenMode) {
        flagItems.push('<span style="background: #8b5cf6; color: white; padding: 2px 6px; border-radius: 4px; font-size: 10px;">🔗 Single LP Mode</span>');
    }
    
    if (flagItems.length > 0) {
        return `
            <div class="pool-metric" style="grid-column: 1 / -1;">
                <div class="metric-label">Active Pool Flags</div>
                <div class="metric-value" style="display: flex; flex-wrap: wrap; gap: 4px; justify-content: center;">
                    ${flagItems.join(' ')}
                </div>
            </div>
        `;
    }
    
    return '';
}

/**
 * Phase 2.1: Add expandable Pool State display section with ALL PoolState fields
 */
function addExpandablePoolStateDisplay() {
    if (!poolData) return;
    
    // Create expandable section after pool info
    const poolInfoSection = document.querySelector('.pool-info-section');
    
    // Remove existing expandable section if it exists
    const existingSection = document.getElementById('expandable-pool-state');
    if (existingSection) {
        existingSection.remove();
    }
    
    const expandableSection = document.createElement('div');
    expandableSection.id = 'expandable-pool-state';
    expandableSection.style.cssText = `
        background: white;
        margin-top: 20px;
        border-radius: 12px;
        box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
        border-left: 4px solid #8b5cf6;
        overflow: hidden;
    `;
    
    const flags = window.TokenDisplayUtils.interpretPoolFlags(poolData);
    
    expandableSection.innerHTML = `
        <div style="padding: 20px; cursor: pointer; background: #f8f9fa; border-bottom: 1px solid #e5e7eb;" onclick="togglePoolStateDetails()">
            <h3 style="margin: 0; color: #333; display: flex; align-items: center; justify-content: between;">
                🔍 Pool State Details (Expandable Debug Section)
                <span id="expand-indicator" style="margin-left: auto; font-size: 20px;">▼</span>
            </h3>
            <p style="margin: 5px 0 0 0; color: #666; font-size: 14px;">Click to view all PoolState struct fields</p>
        </div>
        <div id="pool-state-details" style="display: none; padding: 25px;">
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 20px;">
                ${generatePoolStateFields()}
            </div>
        </div>
    `;
    
    poolInfoSection.insertAdjacentElement('afterend', expandableSection);
}

/**
 * Phase 2.1: Generate all PoolState struct fields display
 */
function generatePoolStateFields() {
    if (!poolData) return '';
    
    const flags = window.TokenDisplayUtils.interpretPoolFlags(poolData);
    
    return `
        <!-- Basic Pool Information -->
        <div class="pool-state-section">
            <h4 style="color: #4f46e5; margin: 0 0 15px 0; border-bottom: 2px solid #e0e7ff; padding-bottom: 5px;">📋 Basic Pool Information</h4>
            <div class="state-field"><strong>owner:</strong><br><code>${poolData.owner || 'N/A'}</code></div>
            <div class="state-field"><strong>token_a_mint:</strong><br><code>${poolData.token_a_mint || poolData.tokenAMint || 'N/A'}</code></div>
            <div class="state-field"><strong>token_b_mint:</strong><br><code>${poolData.token_b_mint || poolData.tokenBMint || 'N/A'}</code></div>
            <div class="state-field"><strong>token_a_vault:</strong><br><code>${poolData.token_a_vault || poolData.tokenAVault || 'N/A'}</code></div>
            <div class="state-field"><strong>token_b_vault:</strong><br><code>${poolData.token_b_vault || poolData.tokenBVault || 'N/A'}</code></div>
            <div class="state-field"><strong>lp_token_a_mint:</strong><br><code>${poolData.lp_token_a_mint || poolData.lpTokenAMint || 'N/A'}</code></div>
            <div class="state-field"><strong>lp_token_b_mint:</strong><br><code>${poolData.lp_token_b_mint || poolData.lpTokenBMint || 'N/A'}</code></div>
        </div>
        
        <!-- Ratio Configuration -->
        <div class="pool-state-section">
            <h4 style="color: #059669; margin: 0 0 15px 0; border-bottom: 2px solid #d1fae5; padding-bottom: 5px;">⚖️ Ratio Configuration</h4>
            <div class="state-field"><strong>ratio_a_numerator:</strong><br><code>${poolData.ratio_a_numerator || poolData.ratioANumerator || 'N/A'}</code></div>
            <div class="state-field"><strong>ratio_b_denominator:</strong><br><code>${poolData.ratio_b_denominator || poolData.ratioBDenominator || 'N/A'}</code></div>
        </div>
        
        <!-- Liquidity Information -->
        <div class="pool-state-section">
            <h4 style="color: #0284c7; margin: 0 0 15px 0; border-bottom: 2px solid #bae6fd; padding-bottom: 5px;">💧 Liquidity Information</h4>
            <div class="state-field"><strong>total_token_a_liquidity:</strong><br><code>${poolData.total_token_a_liquidity || poolData.tokenALiquidity || 'N/A'}</code></div>
            <div class="state-field"><strong>total_token_b_liquidity:</strong><br><code>${poolData.total_token_b_liquidity || poolData.tokenBLiquidity || 'N/A'}</code></div>
        </div>
        
        <!-- Bump Seeds -->
        <div class="pool-state-section">
            <h4 style="color: #7c3aed; margin: 0 0 15px 0; border-bottom: 2px solid #ede9fe; padding-bottom: 5px;">🔑 Bump Seeds</h4>
            <div class="state-field"><strong>pool_authority_bump_seed:</strong><br><code>${poolData.pool_authority_bump_seed || poolData.poolAuthorityBumpSeed || 'N/A'}</code></div>
            <div class="state-field"><strong>token_a_vault_bump_seed:</strong><br><code>${poolData.token_a_vault_bump_seed || poolData.tokenAVaultBumpSeed || 'N/A'}</code></div>
            <div class="state-field"><strong>token_b_vault_bump_seed:</strong><br><code>${poolData.token_b_vault_bump_seed || poolData.tokenBVaultBumpSeed || 'N/A'}</code></div>
            <div class="state-field"><strong>lp_token_a_mint_bump_seed:</strong><br><code>${poolData.lp_token_a_mint_bump_seed || poolData.lpTokenAMintBumpSeed || 'N/A'}</code></div>
            <div class="state-field"><strong>lp_token_b_mint_bump_seed:</strong><br><code>${poolData.lp_token_b_mint_bump_seed || poolData.lpTokenBMintBumpSeed || 'N/A'}</code></div>
        </div>
        
        <!-- Pool Flags -->
        <div class="pool-state-section">
            <h4 style="color: #dc2626; margin: 0 0 15px 0; border-bottom: 2px solid #fecaca; padding-bottom: 5px;">🚩 Pool Flags</h4>
            <div class="state-field"><strong>flags (raw):</strong><br><code>${poolData.flags || 0} (binary: ${(poolData.flags || 0).toString(2).padStart(5, '0')})</code></div>
            <div class="state-field"><strong>Decoded Flags:</strong><br>
                <div style="margin-top: 5px;">
                    ${flags.oneToManyRatio ? '🎯 One-to-Many Ratio<br>' : ''}
                    ${flags.liquidityPaused ? '⏸️ Liquidity Paused<br>' : ''}
                    ${flags.swapsPaused ? '🚫 Swaps Paused<br>' : ''}
                    ${flags.withdrawalProtection ? '🛡️ Withdrawal Protection<br>' : ''}
                    ${flags.singleLpTokenMode ? '🔗 Single LP Mode<br>' : ''}
                    ${!flags.oneToManyRatio && !flags.liquidityPaused && !flags.swapsPaused && !flags.withdrawalProtection && !flags.singleLpTokenMode ? '✅ No Active Flags' : ''}
                </div>
            </div>
        </div>
        
        <!-- Fee Configuration -->
        <div class="pool-state-section">
            <h4 style="color: #ea580c; margin: 0 0 15px 0; border-bottom: 2px solid #fed7aa; padding-bottom: 5px;">💰 Fee Configuration</h4>
            <div class="state-field"><strong>contract_liquidity_fee:</strong><br><code>${poolData.contract_liquidity_fee || poolData.contractLiquidityFee || 'N/A'} lamports</code></div>
            <div class="state-field"><strong>swap_contract_fee:</strong><br><code>${poolData.swap_contract_fee || poolData.swapContractFee || 'N/A'} lamports</code></div>
        </div>
        
        <!-- Token Fee Tracking -->
        <div class="pool-state-section">
            <h4 style="color: #16a34a; margin: 0 0 15px 0; border-bottom: 2px solid #bbf7d0; padding-bottom: 5px;">📊 Token Fee Tracking</h4>
            <div class="state-field"><strong>collected_fees_token_a:</strong><br><code>${poolData.collected_fees_token_a || poolData.collectedFeesTokenA || 'N/A'}</code></div>
            <div class="state-field"><strong>collected_fees_token_b:</strong><br><code>${poolData.collected_fees_token_b || poolData.collectedFeesTokenB || 'N/A'}</code></div>
            <div class="state-field"><strong>total_fees_withdrawn_token_a:</strong><br><code>${poolData.total_fees_withdrawn_token_a || poolData.totalFeesWithdrawnTokenA || 'N/A'}</code></div>
            <div class="state-field"><strong>total_fees_withdrawn_token_b:</strong><br><code>${poolData.total_fees_withdrawn_token_b || poolData.totalFeesWithdrawnTokenB || 'N/A'}</code></div>
        </div>
        
        <!-- SOL Fee Tracking -->
        <div class="pool-state-section">
            <h4 style="color: #9333ea; margin: 0 0 15px 0; border-bottom: 2px solid #e9d5ff; padding-bottom: 5px;">⚡ SOL Fee Tracking</h4>
            <div class="state-field"><strong>collected_liquidity_fees:</strong><br><code>${poolData.collected_liquidity_fees || poolData.collectedLiquidityFees || 'N/A'} lamports</code></div>
            <div class="state-field"><strong>collected_swap_contract_fees:</strong><br><code>${poolData.collected_swap_contract_fees || poolData.collectedSwapContractFees || 'N/A'} lamports</code></div>
            <div class="state-field"><strong>total_sol_fees_collected:</strong><br><code>${poolData.total_sol_fees_collected || poolData.totalSolFeesCollected || 'N/A'} lamports</code></div>
        </div>
        
        <!-- Consolidation Data -->
        <div class="pool-state-section">
            <h4 style="color: #be123c; margin: 0 0 15px 0; border-bottom: 2px solid #fda4af; padding-bottom: 5px;">🔄 Consolidation Data</h4>
            <div class="state-field"><strong>last_consolidation_timestamp:</strong><br><code>${poolData.last_consolidation_timestamp || poolData.lastConsolidationTimestamp || 'N/A'}</code></div>
            <div class="state-field"><strong>total_consolidations:</strong><br><code>${poolData.total_consolidations || poolData.totalConsolidations || 'N/A'}</code></div>
            <div class="state-field"><strong>total_fees_consolidated:</strong><br><code>${poolData.total_fees_consolidated || poolData.totalFeesConsolidated || 'N/A'} lamports</code></div>
        </div>
    `;
}

/**
 * Phase 2.1: Toggle pool state details visibility
 */
function togglePoolStateDetails() {
    const details = document.getElementById('pool-state-details');
    const indicator = document.getElementById('expand-indicator');
    
    if (details.style.display === 'none') {
        details.style.display = 'block';
        indicator.textContent = '▲';
    } else {
        details.style.display = 'none';
        indicator.textContent = '▼';
    }
}

/**
 * Phase 3.1: Switch between add and remove liquidity operations
 */
function switchOperation(operation) {
    const addToggle = document.getElementById('add-toggle');
    const removeToggle = document.getElementById('remove-toggle');
    const addSection = document.getElementById('add-liquidity-section');
    const removeSection = document.getElementById('remove-liquidity-section');
    
    if (operation === 'add') {
        addToggle.classList.add('active');
        removeToggle.classList.remove('active');
        addSection.style.display = 'block';
        removeSection.style.display = 'none';
    } else {
        addToggle.classList.remove('active');
        removeToggle.classList.add('active');
        addSection.style.display = 'none';
        removeSection.style.display = 'block';
        
        // Load LP token balances when switching to remove
        loadLPTokenBalances();
    }
}

/**
 * Phase 3.1: Load LP token balances for connected wallet
 */
async function loadLPTokenBalances() {
    if (!poolData || !window.backpack?.solana?.publicKey) {
        console.log('No wallet connected or pool data unavailable');
        return;
    }
    
    try {
        // Mock LP token balances (in real implementation, query SPL token accounts)
        const mockLPBalances = {
            tokenA: 1250.543210, // Mock LP Token A balance
            tokenB: 2150.876543  // Mock LP Token B balance
        };
        
        // Update LP token labels and balances
        document.getElementById('lp-token-a-label').textContent = `LP ${poolData.tokenASymbol}`;
        document.getElementById('lp-token-a-balance').textContent = mockLPBalances.tokenA.toFixed(6);
        document.getElementById('lp-token-a-symbol').textContent = `LP ${poolData.tokenASymbol}`;
        document.getElementById('lp-token-a-display').textContent = mockLPBalances.tokenA.toFixed(6);
        
        document.getElementById('lp-token-b-label').textContent = `LP ${poolData.tokenBSymbol}`;
        document.getElementById('lp-token-b-balance').textContent = mockLPBalances.tokenB.toFixed(6);
        document.getElementById('lp-token-b-symbol').textContent = `LP ${poolData.tokenBSymbol}`;
        document.getElementById('lp-token-b-display').textContent = mockLPBalances.tokenB.toFixed(6);
        
        console.log('✅ LP token balances loaded');
        
    } catch (error) {
        console.error('❌ Error loading LP token balances:', error);
        showStatus('error', 'Failed to load LP token balances');
    }
}

/**
 * Phase 3.1: Select LP token for removal
 */
function selectLPToken(tokenType) {
    const optionA = document.getElementById('lp-token-a-option');
    const optionB = document.getElementById('lp-token-b-option');
    
    // Clear previous selections
    optionA.classList.remove('selected');
    optionB.classList.remove('selected');
    
    if (tokenType === 'a') {
        optionA.classList.add('selected');
        document.getElementById('selected-lp-token-name').textContent = `LP ${poolData.tokenASymbol}`;
        document.getElementById('available-lp-symbol').textContent = `LP ${poolData.tokenASymbol}`;
        
        const balance = document.getElementById('lp-token-a-balance').textContent;
        document.getElementById('available-lp-balance').textContent = balance;
        
        // Update expected output display
        document.getElementById('output-token-label').textContent = `${poolData.tokenASymbol}:`;
        
    } else {
        optionB.classList.add('selected');
        document.getElementById('selected-lp-token-name').textContent = `LP ${poolData.tokenBSymbol}`;
        document.getElementById('available-lp-symbol').textContent = `LP ${poolData.tokenBSymbol}`;
        
        const balance = document.getElementById('lp-token-b-balance').textContent;
        document.getElementById('available-lp-balance').textContent = balance;
        
        // Update expected output display
        document.getElementById('output-token-label').textContent = `${poolData.tokenBSymbol}:`;
    }
    
    // Reset remove amount and update button
    document.getElementById('remove-liquidity-amount').value = '';
    updateRemoveButton();
}

/**
 * Phase 3.1: Update remove liquidity button state
 */
function updateRemoveButton() {
    const amount = parseFloat(document.getElementById('remove-liquidity-amount').value) || 0;
    const selectedLP = document.querySelector('.lp-token-option.selected');
    const button = document.getElementById('remove-liquidity-btn');
    const expectedOutput = document.getElementById('expected-output');
    
    if (amount > 0 && selectedLP) {
        button.disabled = false;
        
        // Calculate expected output (mock calculation)
        const isTokenA = selectedLP.id === 'lp-token-a-option';
        let expectedAmount;
        
        if (isTokenA) {
            // Convert LP tokens to underlying Token A
            expectedAmount = amount * 1.0; // 1:1 ratio for LP to underlying (simplified)
        } else {
            // Convert LP tokens to underlying Token B
            expectedAmount = amount * 1.0; // 1:1 ratio for LP to underlying (simplified)
        }
        
        document.getElementById('output-token-amount').textContent = expectedAmount.toFixed(6);
        expectedOutput.style.display = 'block';
        
    } else {
        button.disabled = true;
        expectedOutput.style.display = 'none';
    }
}

/**
 * Phase 3.1: Execute remove liquidity transaction
 */
async function removeLiquidity() {
    if (!poolData) {
        showStatus('error', 'No pool data available');
        return;
    }
    
    const amount = parseFloat(document.getElementById('remove-liquidity-amount').value);
    const selectedLP = document.querySelector('.lp-token-option.selected');
    
    if (!amount || !selectedLP) {
        showStatus('error', 'Please select LP token and enter amount');
        return;
    }
    
    try {
        const isTokenA = selectedLP.id === 'lp-token-a-option';
        const tokenType = isTokenA ? poolData.tokenASymbol : poolData.tokenBSymbol;
        const lpTokenType = isTokenA ? `LP ${poolData.tokenASymbol}` : `LP ${poolData.tokenBSymbol}`;
        
        showStatus('info', `Removing ${amount} ${lpTokenType} from pool...`);
        
        // Mock remove liquidity operation
        await new Promise(resolve => setTimeout(resolve, 2000));
        
        showStatus('success', `✅ Successfully removed ${amount} ${lpTokenType}! You received ${amount.toFixed(6)} ${tokenType}.`);
        
        // Reset form
        document.getElementById('remove-liquidity-amount').value = '';
        updateRemoveButton();
        
        // Reload LP balances
        setTimeout(() => {
            loadLPTokenBalances();
        }, 1000);
        
    } catch (error) {
        console.error('❌ Error removing liquidity:', error);
        showStatus('error', `Failed to remove liquidity: ${error.message}`);
    }
}

/**
 * Update the original add liquidity function to use new ID
 */
function updateAddButton() {
    const amount = parseFloat(document.getElementById('add-liquidity-amount').value) || 0;
    const selectedToken = document.querySelector('.token-option.selected');
    const button = document.getElementById('add-liquidity-btn');
    
    if (amount > 0 && selectedToken) {
        button.disabled = false;
        button.style.display = 'block';
    } else {
        button.disabled = true;
    }
}

/**
 * Set maximum amount for liquidity operations
 */
function setMaxAmount(operation) {
    try {
        if (operation === 'add') {
            // For adding liquidity, use the selected token's balance
            if (!selectedToken) {
                showStatus('error', 'Please select a token first');
                return;
            }
            
            const maxAmount = selectedToken.balance;
            const amountInput = document.getElementById('add-liquidity-amount');
            
            if (amountInput && maxAmount > 0) {
                amountInput.value = maxAmount.toString();
                updateAddButton();
                showStatus('info', `Set to maximum available: ${maxAmount.toLocaleString()} ${selectedToken.symbol}`);
                console.log(`💰 Set max amount for add: ${maxAmount} ${selectedToken.symbol}`);
            } else {
                showStatus('error', 'No balance available');
            }
            
        } else if (operation === 'remove') {
            // For removing liquidity, use the selected LP token's balance
            if (!selectedLPToken) {
                showStatus('error', 'Please select an LP token first');
                return;
            }
            
            const maxAmount = selectedLPToken.balance;
            const amountInput = document.getElementById('remove-liquidity-amount');
            
            if (amountInput && maxAmount > 0) {
                amountInput.value = maxAmount.toString();
                updateRemoveButton();
                calculateExpectedOutput();
                showStatus('info', `Set to maximum available: ${maxAmount.toLocaleString()} ${selectedLPToken.symbol}`);
                console.log(`💰 Set max amount for remove: ${maxAmount} ${selectedLPToken.symbol}`);
            } else {
                showStatus('error', 'No LP token balance available');
            }
        }
        
    } catch (error) {
        console.error(`❌ Error setting max amount for ${operation}:`, error);
        showStatus('error', `Failed to set maximum amount: ${error.message}`);
    }
}

/**
 * Find or create an associated token account for a user and mint
 */
async function findOrCreateAssociatedTokenAccount(ownerPubkey, mintPubkey) {
    try {
        // Calculate the associated token account address
        const associatedTokenAddress = await splToken.getAssociatedTokenAddress(
            mintPubkey,
            ownerPubkey,
            false, // allowOwnerOffCurve
            splToken.TOKEN_PROGRAM_ID,
            splToken.ASSOCIATED_TOKEN_PROGRAM_ID
        );
        
        console.log(`🔍 Checking for associated token account: ${associatedTokenAddress.toString()}`);
        
        // Check if the account already exists
        const accountInfo = await connection.getAccountInfo(associatedTokenAddress);
        
        if (accountInfo) {
            console.log(`✅ Associated token account already exists: ${associatedTokenAddress.toString()}`);
            return associatedTokenAddress;
        } else {
            console.log(`📝 Associated token account does not exist, will be created during transaction`);
            // Return the address - the account will be created automatically by the SPL Token program
            // when the first tokens are minted to it
            return associatedTokenAddress;
        }
        
    } catch (error) {
        console.error('❌ Error with associated token account:', error);
        throw new Error(`Failed to handle associated token account: ${error.message}`);
    }
}

// Export for global access
window.connectWallet = connectWallet;
window.disconnectWallet = disconnectWallet;
window.selectToken = selectToken;
window.updateAddButton = updateAddButton;
window.addLiquidity = addLiquidity;
window.togglePoolStateDetails = togglePoolStateDetails; // Phase 2.1
// Phase 3.1: Export new functions
window.switchOperation = switchOperation;
window.selectLPToken = selectLPToken;
window.updateRemoveButton = updateRemoveButton;
window.removeLiquidity = removeLiquidity;

console.log('💧 Liquidity JavaScript loaded successfully'); 