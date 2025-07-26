/**
 * Phase 2.1: Swap Page JavaScript
 * Implements swap interface with Phase 1.3 display rules and expandable Pool State display
 */

// Global variables
let poolAddress = null;
let poolData = null;
let connection = null;
let swapDirection = 'AtoB'; // 'AtoB' or 'BtoA'
// Phase 4.1: Enhanced swap settings
let slippageTolerance = 0.5; // Default 0.5%
let mockBalances = {
    tokenA: 150.543210,
    tokenB: 2500.876543
};

/**
 * Initialize the swap page
 */
async function initializeSwapPage() {
    console.log('🔄 Initializing Swap Page...');
    
    try {
        // Initialize connection
        await CONFIG.initialize();
        connection = new solanaWeb3.Connection(CONFIG.rpcUrl, 'confirmed');
        
        // Get pool address from URL params or sessionStorage
        const urlParams = new URLSearchParams(window.location.search);
        poolAddress = urlParams.get('pool') || sessionStorage.getItem('selectedPoolAddress');
        
        if (!poolAddress) {
            showStatus('error', 'No pool selected. Please select a pool from the dashboard.');
            return;
        }
        
        console.log('🎯 Loading pool for swap:', poolAddress);
        await loadPoolData();
        
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
        
        // Try to load from generated state file first
        let loadedFromState = false;
        try {
            const response = await fetch('state.json');
            if (response.ok) {
                const stateData = await response.json();
                const pool = stateData.pools?.find(p => p.address === poolAddress);
                if (pool) {
                    poolData = { ...pool, dataSource: 'JSON' };
                    loadedFromState = true;
                    console.log('✅ Pool loaded from state.json');
                }
            }
        } catch (stateError) {
            console.log('ℹ️ State file not available, trying other sources');
        }
        
        // Try sessionStorage if not found in state
        if (!loadedFromState) {
            try {
                const sessionPools = JSON.parse(sessionStorage.getItem('pools') || '[]');
                const sessionPool = sessionPools.find(p => p.address === poolAddress);
                if (sessionPool) {
                    poolData = { ...sessionPool, dataSource: 'sessionStorage' };
                    loadedFromState = true;
                    console.log('✅ Pool loaded from sessionStorage');
                }
            } catch (sessionError) {
                console.log('ℹ️ SessionStorage not available, trying RPC');
            }
        }
        
        // Fallback to RPC
        if (!loadedFromState) {
            poolData = await loadPoolFromRPC();
            if (poolData) {
                poolData.dataSource = 'RPC';
                console.log('✅ Pool loaded from RPC');
            }
        }
        
        if (poolData) {
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
 * Load pool data from RPC
 */
async function loadPoolFromRPC() {
    try {
        const poolPubkey = new solanaWeb3.PublicKey(poolAddress);
        const accountInfo = await connection.getAccountInfo(poolPubkey);
        
        if (!accountInfo) {
            throw new Error('Pool account not found');
        }
        
        // Parse the pool state (this would need to match your program's data structure)
        const poolState = parsePoolState(accountInfo.data);
        return {
            address: poolAddress,
            ...poolState
        };
    } catch (error) {
        console.error('❌ Error loading from RPC:', error);
        return null;
    }
}

/**
 * Parse pool state from account data
 */
function parsePoolState(data) {
    try {
        const dataArray = new Uint8Array(data);
        let offset = 0;
        
        // Helper functions to read bytes
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

        const readI64 = () => {
            const view = new DataView(dataArray.buffer, offset, 8);
            const value = view.getBigInt64(0, true); // little-endian
            offset += 8;
            return Number(value);
        };

        const readU8 = () => {
            const value = dataArray[offset];
            offset += 1;
            return value;
        };

        const readBool = () => {
            const value = dataArray[offset] !== 0;
            offset += 1;
            return value;
        };
        
        // Parse all PoolState fields according to the struct definition
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
        
        // Bump seeds
        const poolAuthorityBumpSeed = readU8();
        const tokenAVaultBumpSeed = readU8();
        const tokenBVaultBumpSeed = readU8();
        const lpTokenAMintBumpSeed = readU8();
        const lpTokenBMintBumpSeed = readU8();
        
        // Pool flags (bitwise operations)
        const flags = readU8();
        
        // Configurable contract fees
        const contractLiquidityFee = readU64();
        const swapContractFee = readU64();
        
        // Token fee tracking
        const collectedFeesTokenA = readU64();
        const collectedFeesTokenB = readU64();
        const totalFeesWithdrawnTokenA = readU64();
        const totalFeesWithdrawnTokenB = readU64();
        
        // SOL fee tracking
        const collectedLiquidityFees = readU64();
        const collectedSwapContractFees = readU64();
        const totalSolFeesCollected = readU64();
        
        // Consolidation management
        const lastConsolidationTimestamp = readI64();
        const totalConsolidations = readU64();
        const totalFeesConsolidated = readU64();
        
        // Decode flags
        const flagsDecoded = {
            one_to_many_ratio: (flags & 1) !== 0,
            liquidity_paused: (flags & 2) !== 0,
            swaps_paused: (flags & 4) !== 0,
            withdrawal_protection: (flags & 8) !== 0,
            single_lp_token_mode: (flags & 16) !== 0
        };
        
        return {
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
            tokenBLiquidity: totalTokenBLiquidity,
            
            // Bump seeds
            poolAuthorityBumpSeed,
            tokenAVaultBumpSeed,
            tokenBVaultBumpSeed,
            lpTokenAMintBumpSeed,
            lpTokenBMintBumpSeed,
            
            // Flags
            flags,
            flagsDecoded,
            
            // Fee configuration
            contractLiquidityFee,
            swapContractFee,
            
            // Token fee tracking
            collectedFeesTokenA,
            collectedFeesTokenB,
            totalFeesWithdrawnTokenA,
            totalFeesWithdrawnTokenB,
            
            // SOL fee tracking
            collectedLiquidityFees,
            collectedSwapContractFees,
            totalSolFeesCollected,
            
            // Consolidation data
            lastConsolidationTimestamp,
            totalConsolidations,
            totalFeesConsolidated
        };
    } catch (error) {
        console.error('❌ Error parsing pool state:', error);
        throw new Error(`Failed to parse pool state: ${error.message}`);
    }
}

/**
 * Enrich pool data with token symbols
 */
async function enrichPoolData() {
    if (!poolData) return;
    
    try {
        const symbols = await getTokenSymbols(poolData.tokenAMint, poolData.tokenBMint);
        poolData.tokenASymbol = symbols.tokenA;
        poolData.tokenBSymbol = symbols.tokenB;
    } catch (error) {
        console.warn('Warning: Could not load token symbols:', error);
        poolData.tokenASymbol = `TOKEN-${poolData.tokenAMint?.slice(0, 4) || 'A'}`;
        poolData.tokenBSymbol = `TOKEN-${poolData.tokenBMint?.slice(0, 4) || 'B'}`;
    }
}

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
 * Phase 2.1: Update pool display with Phase 1.3 enhancements
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
 * Phase 2.1: Initialize swap interface
 */
function initializeSwapInterface() {
    if (!poolData) return;
    
    const swapLoading = document.getElementById('swap-loading');
    const swapForm = document.getElementById('swap-form');
    
    // Hide loading, show form
    swapLoading.style.display = 'none';
    swapForm.style.display = 'grid';
    
    // Set initial token symbols and setup
    updateSwapInterfaceEnhanced();
    updateExchangeRate();
}

/**
 * Update swap interface based on current direction
 */
function updateSwapInterface() {
    if (!poolData) return;
    
    const display = window.TokenDisplayUtils.getDisplayTokenOrder(poolData);
    
    if (swapDirection === 'AtoB') {
        document.getElementById('from-token-symbol').textContent = poolData.tokenASymbol;
        document.getElementById('to-token-symbol').textContent = poolData.tokenBSymbol;
    } else {
        document.getElementById('from-token-symbol').textContent = poolData.tokenBSymbol;
        document.getElementById('to-token-symbol').textContent = poolData.tokenASymbol;
    }
    
    // Reset amounts
    document.getElementById('from-amount').value = '';
    document.getElementById('to-amount').value = '';
    
    // Update balances (placeholder - would need actual wallet integration)
    document.getElementById('from-token-balance').textContent = '0';
    document.getElementById('to-token-balance').textContent = '0';
}

/**
 * Toggle swap direction
 */
function toggleSwapDirection() {
    swapDirection = swapDirection === 'AtoB' ? 'BtoA' : 'AtoB';
    updateSwapInterfaceEnhanced();
    updateExchangeRate();
    calculateSwapOutputEnhanced();
}

/**
 * Update exchange rate display
 */
function updateExchangeRate() {
    if (!poolData) return;
    
    const display = window.TokenDisplayUtils.getDisplayTokenOrder(poolData);
    const exchangeRateText = document.getElementById('exchange-rate-text');
    
    if (swapDirection === 'AtoB') {
        const rate = poolData.ratioBDenominator / poolData.ratioANumerator;
        exchangeRateText.textContent = `1 ${poolData.tokenASymbol} = ${window.TokenDisplayUtils.formatExchangeRateStandard(rate)} ${poolData.tokenBSymbol}`;
    } else {
        const rate = poolData.ratioANumerator / poolData.ratioBDenominator;
        exchangeRateText.textContent = `1 ${poolData.tokenBSymbol} = ${window.TokenDisplayUtils.formatExchangeRateStandard(rate)} ${poolData.tokenASymbol}`;
    }
}

/**
 * Calculate swap output amount
 */
function calculateSwapOutput() {
    if (!poolData) return;
    
    const fromAmount = parseFloat(document.getElementById('from-amount').value) || 0;
    const toAmountInput = document.getElementById('to-amount');
    
    if (fromAmount <= 0) {
        toAmountInput.value = '';
        document.getElementById('swap-btn').disabled = true;
        return;
    }
    
    let outputAmount;
    if (swapDirection === 'AtoB') {
        // Convert from Token A to Token B
        outputAmount = (fromAmount * poolData.ratioBDenominator) / poolData.ratioANumerator;
    } else {
        // Convert from Token B to Token A
        outputAmount = (fromAmount * poolData.ratioANumerator) / poolData.ratioBDenominator;
    }
    
    toAmountInput.value = outputAmount.toFixed(6);
    document.getElementById('swap-btn').disabled = false;
}

/**
 * Execute swap transaction
 */
function executeSwap() {
    if (!poolData) return;
    
    const fromAmount = parseFloat(document.getElementById('from-amount').value);
    const toAmount = parseFloat(document.getElementById('to-amount').value);
    
    if (!fromAmount || !toAmount) {
        showStatus('error', 'Please enter valid amounts');
        return;
    }
    
    // For now, show a placeholder message
    showStatus('info', `Swap functionality not yet implemented. Would swap ${fromAmount} ${swapDirection === 'AtoB' ? poolData.tokenASymbol : poolData.tokenBSymbol} for ${toAmount} ${swapDirection === 'AtoB' ? poolData.tokenBSymbol : poolData.tokenASymbol}`);
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

// Copy the helper functions from liquidity.js for Phase 2.1 compliance
/**
 * Phase 2.1: Generate pool flags display section for swap page
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
 * Phase 2.1: Generate all PoolState struct fields display (same as liquidity page)
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
 * Phase 4.1: Enhanced token selection for "from" token
 */
function selectFromToken() {
    // In a real implementation, this would show a dropdown with available tokens
    // For now, just toggle between the two pool tokens
    toggleSwapDirection();
}

/**
 * Phase 4.1: Enhanced token selection for "to" token  
 */
function selectToToken() {
    // In a real implementation, this would show a dropdown with available tokens
    // For now, just toggle between the two pool tokens
    toggleSwapDirection();
}

/**
 * Phase 4.1: Set maximum amount from wallet balance
 */
function setMaxAmount() {
    if (!poolData) return;
    
    const maxBalance = swapDirection === 'AtoB' ? mockBalances.tokenA : mockBalances.tokenB;
    document.getElementById('from-amount').value = maxBalance.toFixed(6);
    calculateSwapOutputEnhanced();
}

/**
 * Phase 4.1: Enhanced swap output calculation with preview
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
        return;
    }
    
    // Calculate output amount
    let outputAmount;
    if (swapDirection === 'AtoB') {
        outputAmount = (fromAmount * poolData.ratioBDenominator) / poolData.ratioANumerator;
    } else {
        outputAmount = (fromAmount * poolData.ratioANumerator) / poolData.ratioBDenominator;
    }
    
    toAmountInput.value = outputAmount.toFixed(6);
    
    // Calculate minimum received with slippage
    const minimumReceived = outputAmount * (1 - slippageTolerance / 100);
    
    // Update transaction preview
    updateTransactionPreview(fromAmount, outputAmount, minimumReceived);
    
    // Show preview and enable button
    preview.style.display = 'block';
    swapBtn.disabled = false;
    
    // Update price impact (mock calculation)
    const priceImpact = Math.min(fromAmount * 0.01, 5); // Simple mock calculation
    document.getElementById('price-impact-value').textContent = priceImpact.toFixed(2) + '%';
    
    // Show warning for high slippage
    const warning = document.getElementById('preview-warning');
    if (slippageTolerance > 2.0 || priceImpact > 3.0) {
        warning.style.display = 'block';
    } else {
        warning.style.display = 'none';
    }
}

/**
 * Phase 4.1: Update transaction preview
 */
function updateTransactionPreview(fromAmount, toAmount, minimumReceived) {
    if (!poolData) return;
    
    const fromSymbol = swapDirection === 'AtoB' ? poolData.tokenASymbol : poolData.tokenBSymbol;
    const toSymbol = swapDirection === 'AtoB' ? poolData.tokenBSymbol : poolData.tokenASymbol;
    
    document.getElementById('preview-from-amount').textContent = `${fromAmount.toFixed(6)} ${fromSymbol}`;
    document.getElementById('preview-to-amount').textContent = `${toAmount.toFixed(6)} ${toSymbol}`;
    document.getElementById('preview-minimum').textContent = `${minimumReceived.toFixed(6)} ${toSymbol}`;
    
    // Exchange rate
    const rate = toAmount / fromAmount;
    document.getElementById('preview-rate').textContent = `1 ${fromSymbol} = ${rate.toFixed(6)} ${toSymbol}`;
}

/**
 * Phase 4.1: Set slippage tolerance
 */
function setSlippage(percentage) {
    slippageTolerance = percentage;
    
    // Update UI
    document.getElementById('current-slippage').textContent = percentage + '%';
    document.getElementById('custom-slippage').value = '';
    
    // Update active button
    document.querySelectorAll('.slippage-btn').forEach(btn => {
        btn.classList.remove('active');
    });
    
    // Find and activate the correct button
    document.querySelectorAll('.slippage-btn').forEach(btn => {
        if (btn.textContent === percentage + '%') {
            btn.classList.add('active');
        }
    });
    
    // Recalculate if we have amounts
    calculateSwapOutputEnhanced();
}

/**
 * Phase 4.1: Set custom slippage tolerance
 */
function setCustomSlippage() {
    const customValue = parseFloat(document.getElementById('custom-slippage').value);
    
    if (!isNaN(customValue) && customValue >= 0 && customValue <= 50) {
        slippageTolerance = customValue;
        document.getElementById('current-slippage').textContent = customValue + '%';
        
        // Remove active class from preset buttons
        document.querySelectorAll('.slippage-btn').forEach(btn => {
            btn.classList.remove('active');
        });
        
        // Recalculate
        calculateSwapOutputEnhanced();
    }
}

/**
 * Phase 4.1: Override updateSwapInterface with enhanced features
 */
function updateSwapInterfaceEnhanced() {
    if (!poolData) return;
    
    const display = window.TokenDisplayUtils.getDisplayTokenOrder(poolData);
    
    // Update token symbols and icons
    if (swapDirection === 'AtoB') {
        document.getElementById('from-token-symbol').textContent = poolData.tokenASymbol;
        document.getElementById('to-token-symbol').textContent = poolData.tokenBSymbol;
        document.getElementById('from-token-icon').textContent = poolData.tokenASymbol.charAt(0);
        document.getElementById('to-token-icon').textContent = poolData.tokenBSymbol.charAt(0);
        document.getElementById('from-token-balance').textContent = mockBalances.tokenA.toFixed(6);
        document.getElementById('to-token-balance').textContent = mockBalances.tokenB.toFixed(6);
    } else {
        document.getElementById('from-token-symbol').textContent = poolData.tokenBSymbol;
        document.getElementById('to-token-symbol').textContent = poolData.tokenASymbol;
        document.getElementById('from-token-icon').textContent = poolData.tokenBSymbol.charAt(0);
        document.getElementById('to-token-icon').textContent = poolData.tokenASymbol.charAt(0);
        document.getElementById('from-token-balance').textContent = mockBalances.tokenB.toFixed(6);
        document.getElementById('to-token-balance').textContent = mockBalances.tokenA.toFixed(6);
    }
    
    // Reset amounts
    document.getElementById('from-amount').value = '';
    document.getElementById('to-amount').value = '';
    
    // Hide preview
    document.getElementById('transaction-preview').style.display = 'none';
    document.getElementById('swap-btn').disabled = true;
}

// Export functions for global access
window.toggleSwapDirection = toggleSwapDirection;
window.calculateSwapOutput = calculateSwapOutput;
window.executeSwap = executeSwap;
// Phase 4.1: Export enhanced functions
window.selectFromToken = selectFromToken;
window.selectToToken = selectToToken;
window.setMaxAmount = setMaxAmount;
window.calculateSwapOutputEnhanced = calculateSwapOutputEnhanced;
window.setSlippage = setSlippage;
window.setCustomSlippage = setCustomSlippage;
window.updateSwapInterfaceEnhanced = updateSwapInterfaceEnhanced;
window.togglePoolStateDetails = togglePoolStateDetails;

// Initialize when page loads
document.addEventListener('DOMContentLoaded', initializeSwapPage);

console.log('🔄 Swap JavaScript loaded successfully'); 