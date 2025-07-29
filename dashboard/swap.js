/**
 * Enhanced Swap Page JavaScript
 * Implements complete swap functionality with wallet integration and real transactions
 */

// Global variables
let poolAddress = null;
let poolData = null;
let connection = null;
let wallet = null;
let isConnected = false;
let userTokens = [];
let swapDirection = 'AtoB'; // 'AtoB' or 'BtoA'
// No slippage tolerance needed for fixed ratio trading

/**
 * Initialize the swap page with library loading retry mechanism
 */
async function initializeSwapPage() {
    console.log('🔄 Initializing Enhanced Swap Page...');
    
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
}

/**
 * Initialize the application after libraries are loaded
 */
async function initializeApp() {
    try {
        // Wait for configuration to be loaded
        let configAttempts = 0;
        while (!window.TRADING_CONFIG && configAttempts < 30) {
            await new Promise(resolve => setTimeout(resolve, 100));
            configAttempts++;
        }
        
        if (!window.TRADING_CONFIG) {
            throw new Error('Configuration failed to load after 3 seconds');
        }
        
        // Set up CONFIG alias for backward compatibility
        window.CONFIG = window.TRADING_CONFIG;
        
        // Initialize Solana connection
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
            connectionConfig.wsEndpoint = false;
            connection = new solanaWeb3.Connection(CONFIG.rpcUrl, connectionConfig);
        }
        
        console.log('✅ SPL Token library ready');
        
        // Check if Backpack is installed
        if (!window.backpack) {
            showStatus('error', 'Backpack wallet not detected. Please install Backpack wallet extension.');
            return;
        }
        
        // Get pool address from URL params first, then sessionStorage as fallback
        const urlParams = new URLSearchParams(window.location.search);
        poolAddress = urlParams.get('pool') || sessionStorage.getItem('selectedPoolAddress');
        
        if (!poolAddress) {
            showStatus('error', 'No pool selected. Please select a pool from the dashboard or provide a pool ID in the URL (?pool=POOL_ID).');
            return;
        }
        
        console.log('🎯 Loading pool for swap:', poolAddress);
        
        // Store pool address in sessionStorage for potential navigation
        sessionStorage.setItem('selectedPoolAddress', poolAddress);
        
        await loadPoolData();
        
        // Check if wallet is already connected
        if (window.backpack.isConnected) {
            await handleWalletConnected();
        } else {
            showWalletConnection();
        }
        
    } catch (error) {
        console.error('❌ Error initializing swap page:', error);
        showStatus('error', `Failed to initialize: ${error.message}`);
    }
}

/**
 * Load pool data from various sources
 */
async function loadPoolData() {
    try {
        showStatus('info', 'Loading pool information...');
        
        // Initialize centralized data service if not already done
        if (!window.TradingDataService.connection) {
            await window.TradingDataService.initialize(window.TRADING_CONFIG, connection);
        }
        
        // Get pool data using centralized service (tries state.json first, then RPC)
        poolData = await window.TradingDataService.getPool(poolAddress, 'auto');
        
        // 🔧 MERGE: Add decimal information from state.json if missing
        if (poolData && (!poolData.ratioADecimal || !poolData.ratioBDecimal)) {
            try {
                const stateData = await window.TradingDataService.loadFromStateFile();
                const statePool = stateData.pools.find(p => p.address === poolAddress);
                
                if (statePool) {
                    // Merge decimal information from state.json
                    poolData = {
                        ...poolData,
                        ratioADecimal: statePool.ratioADecimal,
                        ratioAActual: statePool.ratioAActual,
                        ratioBDecimal: statePool.ratioBDecimal,
                        ratioBActual: statePool.ratioBActual,
                        dataSource: poolData.dataSource + '+state'
                    };
                    console.log('✅ Merged decimal information from state.json');
                }
            } catch (error) {
                console.warn('⚠️ Could not merge decimal information from state.json:', error);
            }
        }
        
        if (poolData) {
            console.log(`✅ Pool loaded via TradingDataService (source: ${poolData.source || poolData.dataSource || 'unknown'})`);
            
            // 🔍 DEVELOPER DEBUGGING: Log complete pool data to console
            console.group('🔍 POOL DATA FOR DEVELOPERS');
            console.log('📊 Complete Pool State:', poolData);
            console.log('🏊‍♂️ Pool Address:', poolAddress);
            console.log('🪙 Token A Mint:', poolData.tokenAMint || poolData.token_a_mint);
            console.log('🪙 Token B Mint:', poolData.tokenBMint || poolData.token_b_mint);
            console.log('⚖️ Ratio A Numerator:', poolData.ratioANumerator || poolData.ratio_a_numerator);
            console.log('⚖️ Ratio B Denominator:', poolData.ratioBDenominator || poolData.ratio_b_denominator);
            console.log('💧 Token A Liquidity:', poolData.tokenALiquidity || poolData.total_token_a_liquidity);
            console.log('💧 Token B Liquidity:', poolData.tokenBLiquidity || poolData.total_token_b_liquidity);
            console.log('🚩 Pool Flags:', poolData.flags);
            console.log('🔒 Pool Owner:', poolData.owner);
            console.groupEnd();
            
            await enrichPoolData();
            updatePoolDisplay();
            initializeSwapInterface();
            clearStatus();
        } else {
            showStatus('error', 'Pool not found. Please check the pool address.');
        }
        
    } catch (error) {
        console.error('❌ Error loading pool data:', error);
        showStatus('error', `Failed to load pool: ${error.message}`);
    }
}

/**
 * Enrich pool data with token symbols
 */
async function enrichPoolData() {
    if (!poolData) return;
    
    try {
        const symbols = await getTokenSymbols(
            poolData.tokenAMint || poolData.token_a_mint, 
            poolData.tokenBMint || poolData.token_b_mint
        );
        poolData.tokenASymbol = symbols.tokenA;
        poolData.tokenBSymbol = symbols.tokenB;
        
        console.log(`✅ Token symbols resolved: ${poolData.tokenASymbol}/${poolData.tokenBSymbol}`);
    } catch (error) {
        console.warn('Warning: Could not load token symbols:', error);
        poolData.tokenASymbol = `TOKEN-${(poolData.tokenAMint || poolData.token_a_mint)?.slice(0, 4) || 'A'}`;
        poolData.tokenBSymbol = `TOKEN-${(poolData.tokenBMint || poolData.token_b_mint)?.slice(0, 4) || 'B'}`;
    }
    
    // ✅ CENTRALIZED: Pool data is ready for display - no additional enrichment needed
    console.log('✅ SWAP: Pool data ready for centralized display functions');
}

/**
 * Get token symbols from localStorage, Metaplex, or defaults
 */
async function getTokenSymbols(tokenAMint, tokenBMint) {
    try {
        console.log(`🔍 Looking up symbols for tokens: ${tokenAMint} and ${tokenBMint}`);
        
        const tokenASymbol = await getTokenSymbol(tokenAMint, 'A');
        const tokenBSymbol = await getTokenSymbol(tokenBMint, 'B');
        
        console.log(`✅ Token symbols found: ${tokenASymbol}/${tokenBSymbol}`);
        
        return {
            tokenA: tokenASymbol,
            tokenB: tokenBSymbol
        };
    } catch (error) {
        console.warn('❌ Error getting token symbols:', error);
        return {
            tokenA: `TOKEN-${tokenAMint?.slice(0, 4) || 'A'}`,
            tokenB: `TOKEN-${tokenBMint?.slice(0, 4) || 'B'}`
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
        const defaultSymbol = `TOKEN-${tokenMint?.slice(0, 4) || tokenLabel}`;
        console.log(`⚠️ Using default symbol for token ${tokenLabel}: ${defaultSymbol}`);
        return defaultSymbol;
        
    } catch (error) {
        console.warn(`❌ Error getting symbol for token ${tokenLabel}:`, error);
        return `TOKEN-${tokenMint?.slice(0, 4) || tokenLabel}`;
    }
}

/**
 * Show wallet connection UI
 */
function showWalletConnection() {
    const swapLoading = document.getElementById('swap-loading');
    swapLoading.style.display = 'block';
    swapLoading.innerHTML = `
        <div style="text-align: center; padding: 40px;">
            <h3>💼 Connect Your Wallet</h3>
            <p style="margin: 20px 0; color: #666;">Connect your Backpack wallet to start swapping tokens</p>
            <button id="connect-wallet-btn" class="swap-btn" onclick="connectWallet()" style="max-width: 300px; margin: 0 auto;">
                🔗 Connect Backpack Wallet
            </button>
        </div>
    `;
}

/**
 * Connect wallet
 */
async function connectWallet() {
    try {
        console.log('🔗 Connecting to Backpack wallet...');
        showStatus('info', 'Connecting to wallet...');
        
        await window.backpack.connect();
        await handleWalletConnected();
        
    } catch (error) {
        console.error('❌ Error connecting wallet:', error);
        showStatus('error', 'Failed to connect wallet: ' + error.message);
    }
}

/**
 * Handle wallet connected state
 */
async function handleWalletConnected() {
    try {
        wallet = window.backpack;
        isConnected = true;
        
        console.log('✅ Wallet connected:', wallet.publicKey.toString());
        
        // Check wallet balance
        await checkWalletBalance();
        
        // Load user tokens
        await loadUserTokensForPool();
        
        // Initialize swap interface
        initializeSwapInterface();
        
        showStatus('success', `Wallet connected: ${wallet.publicKey.toString().slice(0, 8)}...`);
        
    } catch (error) {
        console.error('❌ Error handling wallet connection:', error);
        showStatus('error', 'Failed to set up wallet: ' + error.message);
    }
}

/**
 * Check wallet balance
 */
async function checkWalletBalance() {
    try {
        const balance = await connection.getBalance(wallet.publicKey);
        const solBalance = balance / solanaWeb3.LAMPORTS_PER_SOL;
        
        console.log(`💰 Wallet SOL balance: ${solBalance.toFixed(4)} SOL`);
        
        if (solBalance < 0.01) {
            showStatus('error', `⚠️ Low SOL balance: ${solBalance.toFixed(4)} SOL. You need SOL for transaction fees.`);
        }
    } catch (error) {
        console.error('❌ Error checking balance:', error);
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
        const tokenAMint = poolData.tokenAMint || poolData.token_a_mint;
        const tokenBMint = poolData.tokenBMint || poolData.token_b_mint;
        
        for (const tokenAccount of tokenAccounts.value) {
            const accountInfo = tokenAccount.account.data.parsed.info;
            const mintAddress = accountInfo.mint;
            
            // Only include tokens that are part of this pool
            if (mintAddress === tokenAMint || mintAddress === tokenBMint) {
                const balance = parseInt(accountInfo.tokenAmount.amount) || 0;
                
                // Determine which token this is
                const isTokenA = mintAddress === tokenAMint;
                const symbol = isTokenA ? poolData.tokenASymbol : poolData.tokenBSymbol;
                
                // Validate that we have the decimals from the blockchain
                if (accountInfo.tokenAmount.decimals === undefined || accountInfo.tokenAmount.decimals === null) {
                    console.error(`❌ Token decimals not found for ${mintAddress}`);
                    showStatus('error', `Cannot determine decimals for token ${symbol}. This is required for safe transactions.`);
                    return;
                }
                
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
        
        console.log(`✅ Found ${userTokens.length} pool tokens in wallet:`, userTokens);
        
        // Update swap interface with real balances
        updateSwapInterfaceWithRealBalances();
        
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
 * Update pool display
 */
function updatePoolDisplay() {
    if (!poolData) return;
    
    const poolLoading = document.getElementById('pool-loading');
    const poolDetails = document.getElementById('pool-details');
    
    // Hide loading, show details
    poolLoading.style.display = 'none';
    poolDetails.style.display = 'grid';
    
    // ✅ CENTRALIZED: Use centralized display functions for consistency
    const displayInfo = window.TokenDisplayUtils?.getCentralizedDisplayInfo(poolData);
    
    if (!displayInfo) {
        throw new Error('Failed to get centralized display info');
    }
    
    // Build the full display object 
    const flags = window.TokenDisplayUtils.interpretPoolFlags(poolData);
    
    const display = {
        baseToken: displayInfo.tokenASymbol,
        quoteToken: displayInfo.tokenBSymbol,
        displayPair: displayInfo.pairName,
        rateText: displayInfo.ratioText,
        exchangeRate: displayInfo.exchangeRate,
        baseLiquidity: window.TokenDisplayUtils.formatLargeNumber(poolData.tokenALiquidity || poolData.total_token_a_liquidity || 0),
        quoteLiquidity: window.TokenDisplayUtils.formatLargeNumber(poolData.tokenBLiquidity || poolData.total_token_b_liquidity || 0),
        isReversed: false, // Always show TokenA/TokenB order
        isOneToManyRatio: flags.oneToManyRatio
    };
    
    console.log('🔧 SWAP CORRECTED:', display);
    
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
    
    // Add expandable Pool State display section
    addExpandablePoolStateDisplay();
}

/**
 * Initialize swap interface
 */
function initializeSwapInterface() {
    if (!poolData) return;
    
    const swapLoading = document.getElementById('swap-loading');
    const swapForm = document.getElementById('swap-form');
    
    if (!isConnected) {
        showWalletConnection();
        return;
    }
    
    // Hide loading, show form
    swapLoading.style.display = 'none';
    swapForm.style.display = 'grid';
    
    // Set initial token symbols and setup
    updateSwapInterfaceWithRealBalances();
    updateExchangeRate();
}

/**
 * Update swap interface with real user balances
 */
function updateSwapInterfaceWithRealBalances() {
    if (!poolData || !isConnected) return;
    
    // Update token symbols and icons
    if (swapDirection === 'AtoB') {
        document.getElementById('from-token-symbol').textContent = poolData.tokenASymbol;
        document.getElementById('to-token-symbol').textContent = poolData.tokenBSymbol;
        document.getElementById('from-token-icon').textContent = poolData.tokenASymbol.charAt(0);
        document.getElementById('to-token-icon').textContent = poolData.tokenBSymbol.charAt(0);
        
        // Set real balances - convert from basis points to display units
        const tokenA = userTokens.find(t => t.isTokenA);
        const tokenB = userTokens.find(t => !t.isTokenA);
        
        const tokenADisplayBalance = tokenA ? window.TokenDisplayUtils.basisPointsToDisplay(tokenA.balance, tokenA.decimals) : 0;
        const tokenBDisplayBalance = tokenB ? window.TokenDisplayUtils.basisPointsToDisplay(tokenB.balance, tokenB.decimals) : 0;
        
        document.getElementById('from-token-balance').textContent = tokenADisplayBalance.toFixed(tokenA?.decimals || 6);
        document.getElementById('to-token-balance').textContent = tokenBDisplayBalance.toFixed(tokenB?.decimals || 6);
    } else {
        document.getElementById('from-token-symbol').textContent = poolData.tokenBSymbol;
        document.getElementById('to-token-symbol').textContent = poolData.tokenASymbol;
        document.getElementById('from-token-icon').textContent = poolData.tokenBSymbol.charAt(0);
        document.getElementById('to-token-icon').textContent = poolData.tokenASymbol.charAt(0);
        
        // Set real balances - convert from basis points to display units
        const tokenA = userTokens.find(t => t.isTokenA);
        const tokenB = userTokens.find(t => !t.isTokenA);
        
        const tokenADisplayBalance = tokenA ? window.TokenDisplayUtils.basisPointsToDisplay(tokenA.balance, tokenA.decimals) : 0;
        const tokenBDisplayBalance = tokenB ? window.TokenDisplayUtils.basisPointsToDisplay(tokenB.balance, tokenB.decimals) : 0;
        
        document.getElementById('from-token-balance').textContent = tokenBDisplayBalance.toFixed(tokenB?.decimals || 6);
        document.getElementById('to-token-balance').textContent = tokenADisplayBalance.toFixed(tokenA?.decimals || 6);
    }
    
    // Reset amounts
    document.getElementById('from-amount').value = '';
    document.getElementById('to-amount').value = '';
    
    // Hide preview and disable button
    document.getElementById('transaction-preview').style.display = 'none';
    document.getElementById('swap-btn').disabled = true;
    document.getElementById('swap-btn').textContent = '🔄 Enter Amount to Swap';
}

/**
 * Toggle swap direction
 */
function toggleSwapDirection() {
    swapDirection = swapDirection === 'AtoB' ? 'BtoA' : 'AtoB';
    updateSwapInterfaceWithRealBalances();
    updateExchangeRate();
    calculateSwapOutputEnhanced();
}

/**
 * Update exchange rate display (removed from UI)
 */
function updateExchangeRate() {
    // Exchange rate display removed from UI - function kept for compatibility
    if (!poolData) return;
    
    // Use actual display values, not raw basis points
    const ratioAActual = poolData.ratioAActual || poolData.ratio_a_actual || 1;
    const ratioBActual = poolData.ratioBActual || poolData.ratio_b_actual || 1;
    
    if (swapDirection === 'AtoB') {
        // A→B: How many B tokens for 1 A token
        const rate = ratioBActual / ratioAActual;
        console.log(`Exchange rate: 1 ${poolData.tokenASymbol} = ${window.TokenDisplayUtils.formatExchangeRateStandard(rate)} ${poolData.tokenBSymbol}`);
    } else {
        // B→A: How many A tokens for 1 B token  
        const rate = ratioAActual / ratioBActual;
        console.log(`Exchange rate: 1 ${poolData.tokenBSymbol} = ${window.TokenDisplayUtils.formatExchangeRateStandard(rate)} ${poolData.tokenASymbol}`);
    }
}

/**
 * Set maximum amount from wallet balance
 */
function setMaxAmount() {
    if (!poolData || !isConnected) return;
    
    const fromToken = swapDirection === 'AtoB' 
        ? userTokens.find(t => t.isTokenA)
        : userTokens.find(t => !t.isTokenA);
    
    if (fromToken && fromToken.balance > 0) {
        // Convert balance from basis points to display units
        const displayBalance = window.TokenDisplayUtils.basisPointsToDisplay(fromToken.balance, fromToken.decimals);
        
        // Leave a small buffer for potential rounding issues (but much smaller for 0-decimal tokens)
        const bufferAmount = fromToken.decimals === 0 ? 0 : 0.000001;
        const maxAmount = Math.max(0, displayBalance - bufferAmount);
        
        document.getElementById('from-amount').value = maxAmount.toFixed(fromToken.decimals);
        calculateSwapOutputEnhanced();
    }
}

/**
 * ✅ BASIS POINTS REFACTOR: Calculate swap output with proper basis points arithmetic
 * 
 * This function now correctly handles the conversion between display units and basis points,
 * ensuring mathematical accuracy and matching the smart contract's calculation logic.
 */
function calculateSwapOutputEnhanced() {
    if (!poolData) return;
    
    const fromAmount = parseFloat(document.getElementById('from-amount').value) || 0;
    const toAmountInput = document.getElementById('to-amount');
    const preview = document.getElementById('transaction-preview');
    const swapBtn = document.getElementById('swap-btn');
    
    if (fromAmount <= 0) {
        toAmountInput.value = '';
        preview.style.display = 'none';
        swapBtn.disabled = true;
        swapBtn.textContent = '🔄 Enter Amount to Swap';
        return;
    }
    
    // Check if user has sufficient balance
    const fromToken = swapDirection === 'AtoB' 
        ? userTokens.find(t => t.isTokenA)
        : userTokens.find(t => !t.isTokenA);
    
    if (!fromToken) {
        swapBtn.disabled = true;
        swapBtn.textContent = '❌ Token Not Found';
        preview.style.display = 'none';
        return;
    }
    
    // Convert user input to basis points for comparison with stored balance
    const fromAmountBasisPoints = window.TokenDisplayUtils.displayToBasisPoints(fromAmount, fromToken.decimals);
    
    if (fromAmountBasisPoints > fromToken.balance) {
        swapBtn.disabled = true;
        swapBtn.textContent = '❌ Insufficient Balance';
        preview.style.display = 'none';
        return;
    }
    
    try {
        // ✅ BASIS POINTS REFACTOR: Get pool ratios in basis points (from smart contract)
        const ratioABasisPoints = poolData.ratioANumerator || poolData.ratio_a_numerator;
        const ratioBBasisPoints = poolData.ratioBDenominator || poolData.ratio_b_denominator;
        
        console.log('🔄 SWAP CALCULATION (BASIS POINTS):');
        console.log(`  Pool ratios: ${ratioABasisPoints} : ${ratioBBasisPoints} (basis points)`);
        console.log(`  Input: ${fromAmount} (display units)`);
        console.log(`  Direction: ${swapDirection}`);
        
        // ✅ BASIS POINTS REFACTOR: Get token decimals from enriched pool data
        let inputDecimals, outputDecimals, numerator, denominator;
        
        // 🚨 CRITICAL: Get token decimals - NEVER use fallbacks to prevent fund loss
        let tokenADecimals, tokenBDecimals;
        
        if (poolData.ratioADecimal !== undefined && poolData.ratioBDecimal !== undefined) {
            // Use decimals from pool data (preferred)
            tokenADecimals = poolData.ratioADecimal;
            tokenBDecimals = poolData.ratioBDecimal;
        } else if (poolData.tokenDecimals && 
                   poolData.tokenDecimals.tokenADecimals !== undefined && 
                   poolData.tokenDecimals.tokenBDecimals !== undefined) {
            // Use decimals from enriched data (backup)
            tokenADecimals = poolData.tokenDecimals.tokenADecimals;
            tokenBDecimals = poolData.tokenDecimals.tokenBDecimals;
        } else {
            // 🚨 CRITICAL ERROR: Missing decimal data - abort to prevent fund loss
            const error = 'CRITICAL ERROR: Token decimal information missing. Cannot calculate swaps safely. This could result in significant fund loss.';
            console.error('❌ SWAP CALCULATION ABORTED:', error);
            console.error('📊 Available pool data:', poolData);
            throw new Error(error);
        }
        
        if (swapDirection === 'AtoB') {
            // Swapping from Token A to Token B
            inputDecimals = tokenADecimals;   // TS = 4 decimals
            outputDecimals = tokenBDecimals;  // MST = 0 decimals
            numerator = ratioBBasisPoints;     // Token B amount in basis points
            denominator = ratioABasisPoints;   // Token A amount in basis points
        } else {
            // Swapping from Token B to Token A
            inputDecimals = tokenBDecimals;   // MST = 0 decimals
            outputDecimals = tokenADecimals;  // TS = 4 decimals
            numerator = ratioABasisPoints;     // Token A amount in basis points
            denominator = ratioBBasisPoints;   // Token B amount in basis points
        }
        
        console.log(`  Token decimals: input=${inputDecimals}, output=${outputDecimals}`);
        console.log(`  Calculation: (${fromAmount} * ${numerator}) / ${denominator}`);
        
        // ✅ BASIS POINTS REFACTOR: Use the new calculation function
        const outputAmount = calculateSwapOutput(
            fromAmount,         // Input in display units
            inputDecimals,      // Input token decimals
            outputDecimals,     // Output token decimals
            numerator,          // Ratio numerator (basis points)
            denominator         // Ratio denominator (basis points)
        );
        
        console.log(`  Output: ${outputAmount} (display units)`);
        
        toAmountInput.value = outputAmount.toFixed(6);
        
        // Update transaction preview
        updateTransactionPreview(fromAmount, outputAmount);
        
        // Show preview and enable button
        preview.style.display = 'block';
        swapBtn.disabled = false;
        swapBtn.textContent = '🔄 Execute Swap';
        
    } catch (error) {
        console.error('❌ Error calculating swap output:', error);
        swapBtn.disabled = true;
        swapBtn.textContent = '❌ Calculation Error';
        preview.style.display = 'none';
        showStatus('error', 'Error calculating swap: ' + error.message);
    }
}

/**
 * Update transaction preview
 */
function updateTransactionPreview(fromAmount, toAmount) {
    if (!poolData) return;
    
    const fromSymbol = swapDirection === 'AtoB' ? poolData.tokenASymbol : poolData.tokenBSymbol;
    const toSymbol = swapDirection === 'AtoB' ? poolData.tokenBSymbol : poolData.tokenASymbol;
    
    document.getElementById('preview-from-amount').textContent = `${fromAmount.toFixed(6)} ${fromSymbol}`;
    document.getElementById('preview-to-amount').textContent = `${toAmount.toFixed(6)} ${toSymbol}`;
            // No minimum received needed - fixed ratio guarantees exact amount
    
    // Exchange rate
    const rate = toAmount / fromAmount;
    document.getElementById('preview-rate').textContent = `1 ${fromSymbol} = ${rate.toFixed(6)} ${toSymbol}`;
}

/**
 * Execute swap transaction
 */
async function executeSwap() {
    if (!poolData || !isConnected) return;
    
    try {
    const fromAmount = parseFloat(document.getElementById('from-amount').value);
    const toAmount = parseFloat(document.getElementById('to-amount').value);
    
    if (!fromAmount || !toAmount) {
        showStatus('error', 'Please enter valid amounts');
        return;
    }
    
        // Disable swap button during transaction
        const swapBtn = document.getElementById('swap-btn');
        swapBtn.disabled = true;
        swapBtn.textContent = '⏳ Processing Swap...';
        
        console.log('🔄 Starting swap transaction...');
        console.log(`📊 Swapping ${fromAmount} ${swapDirection === 'AtoB' ? poolData.tokenASymbol : poolData.tokenBSymbol} for ${toAmount} ${swapDirection === 'AtoB' ? poolData.tokenBSymbol : poolData.tokenASymbol}`);
        
        showStatus('info', '🔄 Building swap transaction...');
        
        // Get user token accounts
        const fromToken = swapDirection === 'AtoB' 
            ? userTokens.find(t => t.isTokenA)
            : userTokens.find(t => !t.isTokenA);
        
        const toToken = swapDirection === 'AtoB' 
            ? userTokens.find(t => !t.isTokenA)
            : userTokens.find(t => t.isTokenA);
        
        if (!fromToken) {
            throw new Error('Source token account not found');
        }
        
        // Check if user has destination token account
        let toTokenAccountPubkey;
        if (toToken) {
            toTokenAccountPubkey = new solanaWeb3.PublicKey(toToken.tokenAccount);
        } else {
            // Create associated token account for destination token
            const toTokenMint = swapDirection === 'AtoB' 
                ? new solanaWeb3.PublicKey(poolData.tokenBMint || poolData.token_b_mint)
                : new solanaWeb3.PublicKey(poolData.tokenAMint || poolData.token_a_mint);
            
            toTokenAccountPubkey = await window.splToken.Token.getAssociatedTokenAddress(
                window.splToken.ASSOCIATED_TOKEN_PROGRAM_ID,
                window.splToken.TOKEN_PROGRAM_ID,
                toTokenMint,
                wallet.publicKey
            );
            
            console.log('📍 Creating associated token account for destination:', toTokenAccountPubkey.toString());
        }
        
        // Build swap transaction
        const transaction = await buildSwapTransaction(
            fromAmount,
            fromToken,
            toTokenAccountPubkey
        );
        
        showStatus('info', '📝 Requesting wallet signature...');
        
        // Sign and send transaction
        const signatureResult = await wallet.signAndSendTransaction(transaction);
        console.log('✅ Swap transaction sent:', signatureResult);
        
        // Extract signature string from result
        const signature = signatureResult.signature || signatureResult;
        
        showStatus('info', '⏳ Confirming transaction...');
        
        // Wait for confirmation
        const confirmation = await connection.confirmTransaction(signature, 'confirmed');
        
        if (confirmation.value.err) {
            throw new Error(`Swap failed: ${JSON.stringify(confirmation.value.err)}`);
        }
        
        console.log('✅ Swap completed successfully!');
        showStatus('success', `🎉 Swap completed! Transaction: ${signature.slice(0, 8)}...`);
        
        // Refresh user tokens after successful swap
        await loadUserTokensForPool();
        
        // Reset form
        document.getElementById('from-amount').value = '';
        document.getElementById('to-amount').value = '';
        document.getElementById('transaction-preview').style.display = 'none';
        
    } catch (error) {
        console.error('❌ Swap failed:', error);
        showStatus('error', `Swap failed: ${error.message}`);
    } finally {
        // Re-enable swap button
        const swapBtn = document.getElementById('swap-btn');
        swapBtn.disabled = false;
        swapBtn.textContent = '🔄 Execute Swap';
    }
}

/**
 * Build swap transaction
 */
async function buildSwapTransaction(fromAmount, fromToken, toTokenAccountPubkey) {
    console.log('🔧 Building swap transaction...');
    
    // Convert amount to basis points (using TokenDisplayUtils for consistency)
    const amountInBaseUnits = window.TokenDisplayUtils.displayToBasisPoints(fromAmount, fromToken.decimals);
    console.log(`💰 Amount in basis points: ${amountInBaseUnits} (${fromAmount} display units with ${fromToken.decimals} decimals)`);
    
    // Get program ID
    const programId = new solanaWeb3.PublicKey(CONFIG.programId);
    
    // Get system state PDA
    const systemStatePDA = await solanaWeb3.PublicKey.findProgramAddress(
        [new TextEncoder().encode('system_state')],
        programId
    );
    
    // Get pool state PDA (which is our poolAddress)
    const poolStatePDA = new solanaWeb3.PublicKey(poolAddress);
    
    // Get token vault PDAs
    const tokenAVaultPDA = await solanaWeb3.PublicKey.findProgramAddress(
        [new TextEncoder().encode('token_a_vault'), poolStatePDA.toBuffer()],
        programId
    );
    
    const tokenBVaultPDA = await solanaWeb3.PublicKey.findProgramAddress(
        [new TextEncoder().encode('token_b_vault'), poolStatePDA.toBuffer()],
        programId
    );
    
    console.log('🔍 Transaction accounts:');
    console.log('  System State PDA:', systemStatePDA[0].toString());
    console.log('  Pool State PDA:', poolStatePDA.toString());
    console.log('  Token A Vault PDA:', tokenAVaultPDA[0].toString());
    console.log('  Token B Vault PDA:', tokenBVaultPDA[0].toString());
    console.log('  User Input Token Account:', fromToken.tokenAccount);
    console.log('  User Output Token Account:', toTokenAccountPubkey.toString());
    
    // Create instruction data for Swap using Borsh serialization
    // Borsh enum discriminator: Swap = 4 (single byte, based on instruction ordering)
    const inputTokenMint = new solanaWeb3.PublicKey(fromToken.mint);
    
    const instructionData = new Uint8Array([
        4, // Swap discriminator (single byte, like other instructions)
        ...inputTokenMint.toBuffer(), // input_token_mint (32 bytes)
        ...new Uint8Array(new BigUint64Array([BigInt(amountInBaseUnits)]).buffer) // amount_in (u64 little-endian)
    ]);
    
    console.log('🔍 Swap instruction data:');
    console.log('  Discriminator: [4] (single byte)');
    console.log('  Input token mint:', inputTokenMint.toString());
    console.log('  Amount in base units:', amountInBaseUnits);
    console.log('  Total data length:', instructionData.length, 'bytes');
    
    // Build account keys array (11 accounts for decimal-aware swap)
    const outputTokenMint = swapDirection === 'AtoB'
        ? new solanaWeb3.PublicKey(poolData.tokenBMint || poolData.token_b_mint)
        : new solanaWeb3.PublicKey(poolData.tokenAMint || poolData.token_a_mint);
    
    const accountKeys = [
        { pubkey: wallet.publicKey, isSigner: true, isWritable: true },                    // 0: User Authority
        { pubkey: solanaWeb3.SystemProgram.programId, isSigner: false, isWritable: false }, // 1: System Program
        { pubkey: systemStatePDA[0], isSigner: false, isWritable: false },                 // 2: System State PDA
        { pubkey: poolStatePDA, isSigner: false, isWritable: true },                       // 3: Pool State PDA
        { pubkey: window.splToken.TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },  // 4: SPL Token Program
        { pubkey: tokenAVaultPDA[0], isSigner: false, isWritable: true },                  // 5: Token A Vault
        { pubkey: tokenBVaultPDA[0], isSigner: false, isWritable: true },                  // 6: Token B Vault
        { pubkey: new solanaWeb3.PublicKey(fromToken.tokenAccount), isSigner: false, isWritable: true }, // 7: User Input Token Account
        { pubkey: toTokenAccountPubkey, isSigner: false, isWritable: true },               // 8: User Output Token Account
        { pubkey: inputTokenMint, isSigner: false, isWritable: false },                    // 9: Input Token Mint (for decimals)
        { pubkey: outputTokenMint, isSigner: false, isWritable: false }                    // 10: Output Token Mint (for decimals)
    ];
    
    console.log('🔍 Account keys for swap:');
    accountKeys.forEach((account, index) => {
        console.log(`  ${index}: ${account.pubkey.toString()} (signer: ${account.isSigner}, writable: ${account.isWritable})`);
    });
    
    // Create instructions array
    const instructions = [];
    
    // Check if we need to create the destination token account
    const toTokenAccountInfo = await connection.getAccountInfo(toTokenAccountPubkey);
    if (!toTokenAccountInfo) {
        console.log('📍 Adding instruction to create destination token account');
        
        const createATAInstruction = window.splToken.Token.createAssociatedTokenAccountInstruction(
            window.splToken.ASSOCIATED_TOKEN_PROGRAM_ID,
            window.splToken.TOKEN_PROGRAM_ID,
            swapDirection === 'AtoB' 
                ? new solanaWeb3.PublicKey(poolData.tokenBMint || poolData.token_b_mint)
                : new solanaWeb3.PublicKey(poolData.tokenAMint || poolData.token_a_mint),
            toTokenAccountPubkey,
            wallet.publicKey,
            wallet.publicKey
        );
        
        instructions.push(createATAInstruction);
    }
    
    // Create compute budget instruction (200K CUs for swap + token transfers)
    const computeBudgetInstruction = solanaWeb3.ComputeBudgetProgram.setComputeUnitLimit({
        units: 200_000
    });
    
    instructions.push(computeBudgetInstruction);
    
    // Create swap instruction
    const swapInstruction = new solanaWeb3.TransactionInstruction({
        keys: accountKeys,
        programId: programId,
        data: instructionData
    });
    
    instructions.push(swapInstruction);
    
    // Create transaction
    const transaction = new solanaWeb3.Transaction().add(...instructions);
    
    // Get fresh blockhash
    const { blockhash } = await connection.getLatestBlockhash('finalized');
    transaction.recentBlockhash = blockhash;
    transaction.feePayer = wallet.publicKey;
    
    console.log('✅ Swap transaction built successfully');
    
    return transaction;
}

// Slippage functions removed - not needed for fixed ratio trading

/**
 * Enhanced token selection for "from" token
 */
function selectFromToken() {
    toggleSwapDirection();
}

/**
 * Enhanced token selection for "to" token  
 */
function selectToToken() {
    toggleSwapDirection();
}

/**
 * Calculate required input amount when user edits output amount
 */
function calculateSwapInputFromOutput() {
    if (!poolData || !isConnected) return;
    
    const toAmountInput = document.getElementById('to-amount');
    const fromAmountInput = document.getElementById('from-amount');
    const preview = document.getElementById('transaction-preview');
    const swapBtn = document.getElementById('swap-btn');
    
    const toAmount = parseFloat(toAmountInput.value) || 0;
    
    if (toAmount <= 0) {
        fromAmountInput.value = '';
        preview.style.display = 'none';
        swapBtn.disabled = true;
        swapBtn.textContent = '🔄 Enter Amount to Swap';
        return;
    }
    
    try {
        // Get pool ratios in basis points
        const ratioABasisPoints = poolData.ratioANumerator || poolData.ratio_a_numerator;
        const ratioBBasisPoints = poolData.ratioBDenominator || poolData.ratio_b_denominator;
        
        // Get token decimals
        let inputDecimals, outputDecimals, numerator, denominator;
        let tokenADecimals, tokenBDecimals;
        
        if (poolData.ratioADecimal !== undefined && poolData.ratioBDecimal !== undefined) {
            tokenADecimals = poolData.ratioADecimal;
            tokenBDecimals = poolData.ratioBDecimal;
        } else if (poolData.tokenDecimals && 
                   poolData.tokenDecimals.tokenADecimals !== undefined && 
                   poolData.tokenDecimals.tokenBDecimals !== undefined) {
            tokenADecimals = poolData.tokenDecimals.tokenADecimals;
            tokenBDecimals = poolData.tokenDecimals.tokenBDecimals;
        } else {
            throw new Error('Token decimal information missing');
        }
        
        if (swapDirection === 'AtoB') {
            // Reverse calculation: given output B, calculate required input A
            inputDecimals = tokenADecimals;
            outputDecimals = tokenBDecimals;
            // For reverse: input = (output * denominator) / numerator
            numerator = ratioBBasisPoints;
            denominator = ratioABasisPoints;
        } else {
            // Reverse calculation: given output A, calculate required input B
            inputDecimals = tokenBDecimals;
            outputDecimals = tokenADecimals;
            // For reverse: input = (output * denominator) / numerator
            numerator = ratioABasisPoints;
            denominator = ratioBBasisPoints;
        }
        
        // Calculate required input amount (reverse calculation)
        const requiredInput = calculateSwapInputReverse(
            toAmount,           // Desired output in display units
            inputDecimals,      // Input token decimals
            outputDecimals,     // Output token decimals
            numerator,          // Ratio numerator (basis points)
            denominator         // Ratio denominator (basis points)
        );
        
        fromAmountInput.value = requiredInput.toFixed(6);
        
        // Update transaction preview
        updateTransactionPreview(requiredInput, toAmount);
        
        // Check if user has sufficient balance
        const fromToken = swapDirection === 'AtoB' 
            ? userTokens.find(t => t.isTokenA)
            : userTokens.find(t => !t.isTokenA);
        
        if (fromToken) {
            const fromAmountBasisPoints = window.TokenDisplayUtils.displayToBasisPoints(requiredInput, fromToken.decimals);
            
            if (fromAmountBasisPoints > fromToken.balance) {
                swapBtn.disabled = true;
                swapBtn.textContent = '❌ Insufficient Balance';
                preview.style.display = 'none';
                return;
            }
        }
        
        // Enable swap button
        preview.style.display = 'block';
        swapBtn.disabled = false;
        swapBtn.textContent = '🔄 Execute Swap';
        
    } catch (error) {
        console.error('❌ Error calculating required input:', error);
        swapBtn.disabled = true;
        swapBtn.textContent = '❌ Calculation Error';
        preview.style.display = 'none';
        showStatus('error', 'Error calculating required input: ' + error.message);
    }
}

/**
 * Calculate required input amount for a desired output (reverse calculation)
 */
function calculateSwapInputReverse(outputDisplay, inputDecimals, outputDecimals, numeratorBasisPoints, denominatorBasisPoints) {
    try {
        // Validation
        if (typeof outputDisplay !== 'number' || outputDisplay < 0) {
            throw new Error(`Invalid output amount: ${outputDisplay}. Must be a positive number.`);
        }
        if (typeof inputDecimals !== 'number' || inputDecimals < 0 || inputDecimals > 9) {
            throw new Error(`Invalid input decimals: ${inputDecimals}. Must be between 0 and 9.`);
        }
        if (typeof outputDecimals !== 'number' || outputDecimals < 0 || outputDecimals > 9) {
            throw new Error(`Invalid output decimals: ${outputDecimals}. Must be between 0 and 9.`);
        }
        if (typeof numeratorBasisPoints !== 'number' || numeratorBasisPoints <= 0) {
            throw new Error(`Invalid numerator: ${numeratorBasisPoints}. Must be a positive number.`);
        }
        if (typeof denominatorBasisPoints !== 'number' || denominatorBasisPoints <= 0) {
            throw new Error(`Invalid denominator: ${denominatorBasisPoints}. Must be a positive number.`);
        }
        
        // Convert desired output to basis points
        const outputBasisPoints = window.TokenDisplayUtils.displayToBasisPoints(outputDisplay, outputDecimals);
        
        // Reverse calculation: input = (output * denominator) / numerator
        // Use ceiling to ensure we always have enough input
        const inputBasisPoints = Math.ceil((outputBasisPoints * denominatorBasisPoints) / numeratorBasisPoints);
        
        // Convert result back to display units
        const inputDisplay = window.TokenDisplayUtils.basisPointsToDisplay(inputBasisPoints, inputDecimals);
        
        console.log(`🔄 REVERSE SWAP CALCULATION:`, {
            desiredOutput: `${outputDisplay} (${outputBasisPoints} basis points)`,
            requiredInput: `${inputDisplay} (${inputBasisPoints} basis points)`,
            ratio: `${numeratorBasisPoints} : ${denominatorBasisPoints}`
        });
        
        return inputDisplay;
        
    } catch (error) {
        console.error('❌ Error calculating required input:', error);
        throw error;
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

// Helper functions from liquidity.js for pool display
/**
 * Generate pool flags display section
 */
function generatePoolFlagsDisplay(flags, pool) {
    const hasFlags = flags.oneToManyRatio || flags.liquidityPaused || flags.swapsPaused || 
                     flags.withdrawalProtection || flags.singleLpTokenMode;
    
    if (!hasFlags && (typeof pool.flags === 'undefined' || pool.flags === 0)) {
        return '';
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
 * Add expandable Pool State display section
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
                🔍 Pool State Details (Developer Debug Section)
                <span id="expand-indicator" style="margin-left: auto; font-size: 20px;">▼</span>
            </h3>
            <p style="margin: 5px 0 0 0; color: #666; font-size: 14px;">Click to view all PoolState struct fields for debugging</p>
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
 * Generate all PoolState struct fields display
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
        
        <!-- Additional debugging information -->
        <div class="pool-state-section">
            <h4 style="color: #dc2626; margin: 0 0 15px 0; border-bottom: 2px solid #fecaca; padding-bottom: 5px;">🚩 Pool Flags & Status</h4>
            <div class="state-field"><strong>flags (raw):</strong><br><code>${poolData.flags || 0}</code></div>
            <div class="state-field"><strong>Swaps Paused:</strong><br><code>${flags.swapsPaused ? 'Yes' : 'No'}</code></div>
            <div class="state-field"><strong>Liquidity Paused:</strong><br><code>${flags.liquidityPaused ? 'Yes' : 'No'}</code></div>
        </div>
    `;
}

/**
 * Toggle pool state details visibility
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

// Export functions for global access
window.toggleSwapDirection = toggleSwapDirection;
window.calculateSwapOutputEnhanced = calculateSwapOutputEnhanced;
window.executeSwap = executeSwap;
window.selectFromToken = selectFromToken;
window.selectToToken = selectToToken;
window.setMaxAmount = setMaxAmount;
// Slippage functions removed
window.togglePoolStateDetails = togglePoolStateDetails;
window.connectWallet = connectWallet;

// Initialize when page loads
document.addEventListener('DOMContentLoaded', initializeSwapPage);

console.log('🔄 Enhanced Swap JavaScript loaded successfully'); 