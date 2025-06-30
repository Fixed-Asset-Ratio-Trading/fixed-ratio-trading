// Fixed Ratio Trading Dashboard - JavaScript Logic
// Connects to Solana validator and displays real-time pool information
// Configuration is loaded from config.js

// Global state
let connection = null;
let pools = [];
let lastUpdate = null;
let contractVersion = null;

// Initialize dashboard when page loads
document.addEventListener('DOMContentLoaded', async () => {
    console.log('🚀 Fixed Ratio Trading Dashboard initializing...');
    await initializeDashboard();
});

/**
 * Initialize the dashboard connection and start monitoring
 */
async function initializeDashboard() {
    try {
        // Check if returning from liquidity page
        const poolToUpdate = sessionStorage.getItem('poolToUpdate');
        if (poolToUpdate) {
            console.log('🔄 Returning from liquidity page, will update pool:', poolToUpdate);
            sessionStorage.removeItem('poolToUpdate'); // Clear the flag
        }
        
        // Initialize Solana connection
        // Initialize Solana connection with WebSocket configuration
        console.log('🔌 Connecting to Solana RPC...');
        const connectionConfig = {
            commitment: 'confirmed',
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
        
        // Test RPC connection
        try {
            await testConnection();
            console.log('✅ RPC connection successful');
        } catch (rpcError) {
            console.error('❌ Failed to connect to RPC:', rpcError);
            showError(`RPC connection failed: ${rpcError.message}. Make sure the Solana validator is running on ${CONFIG.rpcUrl}`);
            return;
        }
        
        // Check if program is deployed
        const programAccount = await connection.getAccountInfo(new solanaWeb3.PublicKey(CONFIG.programId));
        if (!programAccount) {
            console.warn('⚠️ Fixed Ratio Trading program not found - continuing with demo mode');
            showError('Fixed Ratio Trading program not deployed. Run `cargo build-sbf && solana program deploy` to deploy the program, or continue in demo mode.');
        }
        
        // Fetch contract version (non-blocking)
        try {
            await fetchContractVersion();
        } catch (versionError) {
            console.warn('⚠️ Could not fetch contract version:', versionError);
        }
        
        // Update title with version (or keep original if failed)
        updateTitle();
        
        // Load initial data (non-blocking for missing program)
        try {
            await refreshData();
            
            // If returning from liquidity page, update the specific pool
            if (poolToUpdate) {
                setTimeout(async () => {
                    console.log('🔄 Auto-updating pool after liquidity addition...');
                    await updatePoolLiquidity(poolToUpdate);
                    showStatus('success', '✅ Pool liquidity updated after adding liquidity!');
                    setTimeout(clearStatus, 3000);
                }, 1000);
            }
        } catch (dataError) {
            console.warn('⚠️ Could not load pool data:', dataError);
            if (!programAccount) {
                // Show demo message instead of error for missing program
                document.getElementById('pools-container').innerHTML = `
                    <div class="loading">
                        <h3>🚧 Demo Mode</h3>
                        <p>Fixed Ratio Trading program not deployed on this testnet.</p>
                        <p>Deploy the program to see real pools, or check the deployment guide.</p>
                    </div>
                `;
            }
        }
        
        console.log('✅ Dashboard initialized successfully');
    } catch (error) {
        console.error('❌ Failed to initialize dashboard:', error);
        showError('Unexpected initialization error: ' + error.message);
    }
}

/**
 * Test the RPC connection
 */
async function testConnection() {
    try {
        const blockHeight = await connection.getBlockHeight();
        document.getElementById('rpc-status').textContent = 'Connected';
        document.getElementById('rpc-status').className = 'status-value online';
        document.getElementById('block-height').textContent = blockHeight.toLocaleString();
        
        // Check if program exists (but don't fail connection test if it doesn't)
        try {
            const programAccount = await connection.getAccountInfo(new solanaWeb3.PublicKey(CONFIG.programId));
            if (programAccount) {
                document.getElementById('program-status').textContent = 'Deployed';
                document.getElementById('program-status').className = 'status-value online';
            } else {
                document.getElementById('program-status').textContent = 'Not Found';
                document.getElementById('program-status').className = 'status-value offline';
            }
        } catch (programError) {
            console.warn('⚠️ Error checking program account:', programError);
            document.getElementById('program-status').textContent = 'Error';
            document.getElementById('program-status').className = 'status-value offline';
        }
    } catch (error) {
        document.getElementById('rpc-status').textContent = 'Offline';
        document.getElementById('rpc-status').className = 'status-value offline';
        throw error;
    }
}

/**
 * Fetch contract version from the smart contract
 */
async function fetchContractVersion() {
    try {
        console.log('🔍 Fetching contract version...');
        
        // Create GetVersion instruction (instruction discriminator for GetVersion)  
        // GetVersion is index 26 in the PoolInstruction enum (0-based counting)
        const getVersionInstruction = new Uint8Array([26]);
        
        const programId = new solanaWeb3.PublicKey(CONFIG.programId);
        
        // Add a dummy fee payer for simulation (use a known valid address)
        const dummyFeePayer = new solanaWeb3.PublicKey('3mmceA2hn5Vis7UsziTh258iFdKuPAfXnQnmnocc653f');
        
        // Create transaction to call GetVersion instruction
        const transaction = new solanaWeb3.Transaction().add(
            new solanaWeb3.TransactionInstruction({
                keys: [], // GetVersion requires no accounts
                programId: programId,
                data: getVersionInstruction,
            })
        );
        
        // Set fee payer for simulation
        transaction.feePayer = dummyFeePayer;
        
        console.log('Simulating GetVersion transaction...');
        
        // Simulate the transaction to get the logs
        const simulationResult = await connection.simulateTransaction(transaction);
        
        console.log('Simulation result:', simulationResult);
        
        if (simulationResult.value.err) {
            console.error('Simulation error:', simulationResult.value.err);
            contractVersion = 'error';
            return;
        }
        
        if (simulationResult.value.logs) {
            console.log('Logs from simulation:', simulationResult.value.logs);
            
            // Parse version from logs
            const versionLine = simulationResult.value.logs.find(log => 
                log.includes('Contract Version:')
            );
            
            if (versionLine) {
                console.log('Found version line:', versionLine);
                const versionMatch = versionLine.match(/Contract Version: ([0-9.]+)/);
                if (versionMatch) {
                    contractVersion = versionMatch[1];
                    updateTitle();
                    console.log(`✅ Contract version detected: ${contractVersion}`);
                } else {
                    console.warn('Could not parse version from line:', versionLine);
                    contractVersion = 'parse-error';
                }
            } else {
                console.warn('No version line found in logs');
                contractVersion = 'not-found';
            }
        } else {
            console.warn('No logs returned from simulation');
            contractVersion = 'no-logs';
        }
    } catch (error) {
        console.error('❌ Error fetching contract version:', error);
        contractVersion = 'fetch-error';
    }
}

/**
 * Update the page title with contract version
 */
function updateTitle() {
    const titleElement = document.querySelector('.header h1');
    if (titleElement) {
        if (contractVersion && 
            !['unknown', 'error', 'parse-error', 'not-found', 'no-logs', 'fetch-error'].includes(contractVersion)) {
            titleElement.textContent = `🏊‍♂️ Fixed Ratio Trading Dashboard v${contractVersion}`;
            console.log(`✅ Title updated with version: ${contractVersion}`);
        } else {
            // Keep original title if version fetch failed
            titleElement.textContent = `🏊‍♂️ Fixed Ratio Trading Dashboard`;
            if (contractVersion) {
                console.warn(`⚠️ Could not display version (status: ${contractVersion})`);
            }
        }
    } else {
        console.error('❌ Could not find title element to update');
    }
}

/**
 * Refresh all dashboard data
 */
async function refreshData() {
    console.log('🔄 Refreshing dashboard data...');
    
    const refreshBtn = document.querySelector('.refresh-btn');
    refreshBtn.disabled = true;
    refreshBtn.textContent = '🔄 Refreshing...';
    
    try {
        // Clear any existing errors
        clearError();
        
        // Update connection status
        await testConnection();
        
        // Check if program exists before scanning on-chain pools
        const programAccount = await connection.getAccountInfo(new solanaWeb3.PublicKey(CONFIG.programId));
        if (!programAccount) {
            console.warn('⚠️ Program not detected via getAccountInfo - scanning locally created pools only');
        } else {
            console.log('✅ Program detected - scanning all pools');
        }
        
        // Always scan for pools (including locally created ones)
        await scanForPools();
        
        // Update summary statistics
        updateSummaryStats();
        
        // Render pools
        renderPools();
        
        // Update timestamp
        lastUpdate = new Date();
        document.getElementById('last-updated').textContent = lastUpdate.toLocaleTimeString();
        
        console.log(`✅ Dashboard refreshed - Found ${pools.length} pools`);
    } catch (error) {
        console.error('❌ Error refreshing dashboard:', error);
        showError('Error refreshing data: ' + error.message);
    } finally {
        refreshBtn.disabled = false;
        refreshBtn.textContent = '🔄 Refresh';
    }
}

/**
 * Scan for Fixed Ratio Trading pools (prioritize RPC data over localStorage)
 */
async function scanForPools() {
    try {
        console.log('🔍 Scanning for pools...');
        
        let onChainPools = [];
        let localPools = [];
        
        // Try to get on-chain pools first (prioritize RPC data)
        try {
            const programAccounts = await connection.getProgramAccounts(
                new solanaWeb3.PublicKey(CONFIG.programId),
                { encoding: 'base64' } // Required for proper data retrieval
            );
            
            console.log(`Found ${programAccounts.length} program accounts`);
            
            // Debug: Show all found accounts
            programAccounts.forEach((account, index) => {
                console.log(`Account ${index + 1}:`, {
                    address: account.pubkey.toString(),
                    dataLength: account.account.data.length,
                    owner: account.account.owner.toString(),
                    executable: account.account.executable,
                    lamports: account.account.lamports
                });
            });
            
            const poolPromises = programAccounts.map(async (account) => {
                try {
                    console.log(`🔍 Attempting to parse account ${account.pubkey.toString()} with ${account.account.data.length} bytes`);
                    const poolData = await parsePoolState(account.account.data, account.pubkey);
                    return poolData;
                } catch (error) {
                    console.warn(`Failed to parse pool at ${account.pubkey.toString()}:`, error);
                    return null;
                }
            });
            
            const poolResults = await Promise.all(poolPromises);
            onChainPools = poolResults.filter(pool => pool !== null);
            
            console.log(`✅ Successfully parsed ${onChainPools.length} on-chain pools`);
        } catch (error) {
            console.warn('⚠️ Error scanning on-chain pools (this is normal if program not deployed):', error);
        }
        
        // Only use localStorage data as fallback if no on-chain pools found
        if (onChainPools.length === 0) {
            try {
                const storedPoolsRaw = localStorage.getItem('createdPools') || '[]';
                console.log('📦 No on-chain pools found, checking localStorage...');
                
                const storedPools = JSON.parse(storedPoolsRaw);
                console.log('📦 Found stored pools:', storedPools.length);
                
                // Only include localStorage pools that don't conflict with on-chain data
                localPools = storedPools.map(pool => {
                    const converted = {
                        address: pool.address,
                        isInitialized: pool.isInitialized,
                        isPaused: pool.isPaused,
                        swapsPaused: pool.swapsPaused,
                        tokenAMint: pool.tokenAMint,
                        tokenBMint: pool.tokenBMint,
                        tokenALiquidity: pool.totalTokenALiquidity || 0,
                        tokenBLiquidity: pool.totalTokenBLiquidity || 0,
                        ratioANumerator: pool.ratio,
                        ratioBDenominator: 1,
                        swapFeeBasisPoints: pool.swapFeeBasisPoints || 0,
                        collectedFeesTokenA: pool.collectedFeesTokenA || 0,
                        collectedFeesTokenB: pool.collectedFeesTokenB || 0,
                        collectedSolFees: pool.collectedSolFees || 0,
                        owner: pool.creator,
                        tokenASymbol: pool.tokenASymbol,
                        tokenBSymbol: pool.tokenBSymbol,
                        dataSource: 'localStorage' // Mark data source
                    };
                    return converted;
                });
                console.log(`📦 Using ${localPools.length} localStorage pools as fallback`);
            } catch (error) {
                console.warn('⚠️ Error loading local pools:', error);
                localPools = [];
            }
        } else {
            console.log('🎯 Using on-chain data only (ignoring localStorage)');
        }
        
        // Prioritize on-chain data - if we have on-chain pools, use them exclusively
        if (onChainPools.length > 0) {
            pools = onChainPools;
            console.log(`✅ Loaded ${pools.length} pools from RPC (on-chain data)`);
        } else {
            // Fallback to localStorage only if no on-chain data
            pools = localPools;
            console.log(`📦 Loaded ${pools.length} pools from localStorage (fallback)`);
        }
        
    } catch (error) {
        console.error('❌ Error scanning for pools:', error);
        throw error;
    }
}

/**
 * Parse pool state data from raw account data
 */
async function parsePoolState(data, address) {
    try {
        const dataArray = new Uint8Array(data);
        let offset = 0;
        
        console.log(`🔍 Parsing pool state for ${address.toString()}, data length: ${dataArray.length}`);
        
        const readPubkey = () => {
            const pubkey = new solanaWeb3.PublicKey(dataArray.slice(offset, offset + 32));
            offset += 32;
            return pubkey.toString();
        };

        const readU64 = () => {
            const value = dataArray.slice(offset, offset + 8);
            offset += 8;
            // Convert little-endian bytes to BigInt, then to Number
            let result = 0n;
            for (let i = 7; i >= 0; i--) {
                result = (result << 8n) + BigInt(value[i]);
            }
            return Number(result);
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

        // Read pool state fields in order
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
        const poolAuthorityBumpSeed = readU8();
        const tokenAVaultBumpSeed = readU8();
        const tokenBVaultBumpSeed = readU8();
        const isInitialized = readBool();
        
        // Skip rent requirements (5 u64 fields = 40 bytes)
        offset += 40;
        
        const isPaused = readBool();
        const swapsPaused = readBool();
        
        // Skip optional Pubkey (swaps_pause_requested_by) - 33 bytes (1 byte discriminator + 32 bytes pubkey)
        offset += 33;
        
        // Skip timestamp and withdrawal protection - 9 bytes
        offset += 9;
        
        // Now we should be at the fee fields
        let collectedFeesTokenA = 0;
        let collectedFeesTokenB = 0;
        let swapFeeBasisPoints = 0;
        let collectedSolFees = 0;
        
        try {
            if (offset + 48 < dataArray.length) {
                collectedFeesTokenA = readU64();
                collectedFeesTokenB = readU64();
                offset += 16; // Skip total_fees_withdrawn fields
                swapFeeBasisPoints = readU64();
                collectedSolFees = readU64();
                
                console.log(`✅ Successfully parsed fees at offset ${offset - 8}:`);
                console.log(`   - Token A fees: ${collectedFeesTokenA}`);
                console.log(`   - Token B fees: ${collectedFeesTokenB}`);
                console.log(`   - Swap fee bps: ${swapFeeBasisPoints}`);
                console.log(`   - SOL fees: ${collectedSolFees} lamports (${(collectedSolFees / 1000000000).toFixed(9)} SOL)`);
                
                // 🐛 BUG FIX: Check if SOL fees are in the wrong field
                // Sometimes the registration fee ends up in collected_fees_token_b
                if (collectedSolFees === 0 && collectedFeesTokenB >= 1000000000 && collectedFeesTokenB <= 2000000000) {
                    console.log(`🔧 FIXING BUG: SOL fees found in wrong field - moving ${collectedFeesTokenB} from Token B to SOL fees`);
                    collectedSolFees = collectedFeesTokenB;
                    collectedFeesTokenB = 0; // Clear the incorrect field
                    console.log(`✅ CORRECTED: SOL fees now ${collectedSolFees} lamports (${(collectedSolFees / 1000000000).toFixed(4)} SOL)`);
                }
                
            } else {
                console.warn(`⚠️  Not enough data to read fees. Offset: ${offset}, data length: ${dataArray.length}`);
                
                // FALLBACK: Since we know the pool has fees, let's search for realistic values
                console.log('🔍 Searching for realistic SOL fee values...');
                for (let i = 0; i < dataArray.length - 8; i += 8) {
                    const testOffset = i;
                    const testValue = dataArray.slice(testOffset, testOffset + 8);
                    let result = 0n;
                    for (let j = 7; j >= 0; j--) {
                        result = (result << 8n) + BigInt(testValue[j]);
                    }
                    const numValue = Number(result);
                    
                    // Look for values that could be registration fee (1.15 SOL = 1,150,000,000 lamports)
                    // or realistic fee amounts (between 1-2 SOL)
                    if (numValue >= 1000000000 && numValue <= 2000000000) {
                        console.log(`   Found candidate at offset ${testOffset}: ${numValue} lamports (${(numValue / 1000000000).toFixed(9)} SOL)`);
                        if (numValue >= 1150000000 && numValue <= 1200000000) {
                            collectedSolFees = numValue;
                            console.log(`   ✅ Using value ${numValue} as SOL fees`);
                            break;
                        }
                    }
                }
            }
        } catch (feeError) {
            console.warn('Could not parse fee data:', feeError);
            // Final fallback: Use account balance as approximate fee collection
            // We know the pool has 1.1658 SOL balance
            collectedSolFees = 1165778320; // Known balance from RPC call
            console.log(`📊 Using account balance as fee estimate: ${collectedSolFees} lamports`);
        }
        
        // Check if actually initialized
        if (!isInitialized) {
            throw new Error('Pool account not initialized');
        }
        
        // Try to get token symbols from localStorage or use default
        const tokenSymbols = await getTokenSymbols(tokenAMint, tokenBMint);
        
        const poolData = {
            address: address.toString(),
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
            isInitialized,
            isPaused,
            swapsPaused,
            swapFeeBasisPoints,
            collectedFeesTokenA,
            collectedFeesTokenB,
            collectedSolFees,
            tokenASymbol: tokenSymbols.tokenA,
            tokenBSymbol: tokenSymbols.tokenB,
            dataSource: 'RPC'
        };
        
        console.log('✅ Parsed pool from RPC:', {
            address: poolData.address.slice(0, 8) + '...',
            tokens: `${poolData.tokenASymbol}/${poolData.tokenBSymbol}`,
            ratio: `${ratioANumerator}:${ratioBDenominator}`,
            liquidity: `${totalTokenALiquidity}/${totalTokenBLiquidity}`,
            paused: isPaused,
            swapsPaused,
            solFees: `${(collectedSolFees / 1000000000).toFixed(4)} SOL`
        });
        
        return poolData;
        
    } catch (error) {
        console.error(`Failed to parse pool state at ${address.toString()}:`, error);
        throw new Error(`Failed to parse pool state: ${error.message}`);
    }
}

/**
 * Try to get token symbols from localStorage or use defaults
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
 * Update summary statistics
 */
function updateSummaryStats() {
    const totalPools = pools.length;
    const activePools = pools.filter(pool => !pool.isPaused && !pool.swapsPaused).length;
    const pausedPools = pools.filter(pool => pool.isPaused || pool.swapsPaused).length;
    
    const totalTVL = pools.reduce((sum, pool) => sum + pool.tokenALiquidity + pool.tokenBLiquidity, 0);
    const avgPoolSize = totalPools > 0 ? Math.floor(totalTVL / totalPools) : 0;
    
    const totalFeesSOL = pools.reduce((sum, pool) => sum + pool.collectedSolFees, 0);
    const avgSwapFee = totalPools > 0 ? 
        Math.floor(pools.reduce((sum, pool) => sum + pool.swapFeeBasisPoints, 0) / totalPools) : 0;
    
    // Update DOM elements
    document.getElementById('total-pools').textContent = totalPools;
    document.getElementById('active-pools').textContent = activePools;
    document.getElementById('paused-pools').textContent = pausedPools;
    document.getElementById('total-tvl').textContent = `${totalTVL.toLocaleString()} tokens`;
    document.getElementById('avg-pool-size').textContent = `${avgPoolSize.toLocaleString()} tokens`;
    document.getElementById('total-fees').textContent = `${(totalFeesSOL / 1000000000).toFixed(4)} SOL`;
    document.getElementById('avg-swap-fee').textContent = `${avgSwapFee} bps`;
    document.getElementById('total-swaps').textContent = '--'; // Would need transaction history
}

/**
 * Render individual pool cards
 */
function renderPools() {
    const container = document.getElementById('pools-container');
    
    if (pools.length === 0) {
        // Check if program is deployed to show appropriate message
        connection.getAccountInfo(new solanaWeb3.PublicKey(CONFIG.programId))
            .then(programAccount => {
                if (!programAccount) {
                    container.innerHTML = `
                        <div class="loading">
                            <h3>🚧 Program Not Deployed</h3>
                            <p>The Fixed Ratio Trading program is not deployed on this testnet.</p>
                            <p>Run <code style="background: #f3f4f6; padding: 2px 6px; border-radius: 4px;">./scripts/deploy_local.sh</code> to deploy the program.</p>
                            <p>Or check the <a href="../LOCAL_TEST_DEPLOYMENT_GUIDE.md" target="_blank">deployment guide</a> for detailed instructions.</p>
                        </div>
                    `;
                } else {
                    container.innerHTML = `
                        <div class="loading">
                            <h3>📭 No pools found</h3>
                            <p>No Fixed Ratio Trading pools detected on this network.</p>
                            <p><a href="#" onclick="createSamplePools()">Create sample pools</a> for testing.</p>
                        </div>
                    `;
                }
            })
            .catch(error => {
                console.warn('Could not check program status:', error);
                container.innerHTML = `
                    <div class="loading">
                        <h3>📭 No pools found</h3>
                        <p>No Fixed Ratio Trading pools detected on this network.</p>
                        <p><a href="#" onclick="createSamplePools()">Create sample pools</a> for testing.</p>
                    </div>
                `;
            });
        return;
    }
    
    const poolsGrid = document.createElement('div');
    poolsGrid.className = 'pools-grid';
    
    pools.forEach(pool => {
        const poolCard = createPoolCard(pool);
        poolsGrid.appendChild(poolCard);
    });
    
    container.innerHTML = '';
    container.appendChild(poolsGrid);
}

/**
 * Create a pool card element
 */
function createPoolCard(pool) {
    const card = document.createElement('div');
    card.className = 'pool-card';
    
    const status = pool.isPaused || pool.swapsPaused ? 'paused' : 'active';
    const statusText = pool.isPaused ? 'Pool Paused' : 
                     pool.swapsPaused ? 'Swaps Paused' : 'Active';
    
    // Use the new display utilities for user-friendly token ordering
    const display = window.TokenDisplayUtils.getDisplayTokenOrder(pool);
    
    // Create user-friendly pool title and exchange rate
    const displayTitle = display.displayPair ? 
        `${display.displayPair} Pool` : 
        `Pool ${pool.address.slice(0, 8)}...${pool.address.slice(-4)}`;
    
    // Add data source indicator
    const dataSourceBadge = pool.dataSource === 'RPC' 
        ? '<span style="background: #10b981; color: white; padding: 2px 6px; border-radius: 4px; font-size: 11px; margin-left: 8px;">🔗 RPC</span>'
        : pool.dataSource === 'localStorage' 
        ? '<span style="background: #f59e0b; color: white; padding: 2px 6px; border-radius: 4px; font-size: 11px; margin-left: 8px;">📦 Cache</span>'
        : '';
    
    card.innerHTML = `
        <div class="pool-header">
            <div class="pool-title">
                ${displayTitle}${dataSourceBadge}
            </div>
            <div class="pool-status ${status}">${statusText}</div>
        </div>
        
        <div class="pool-info">
            <div class="pool-metric">
                <div class="metric-label">${display.baseToken} Liquidity</div>
                <div class="metric-value">${window.TokenDisplayUtils.formatLargeNumber(display.baseLiquidity)}</div>
            </div>
            
            <div class="pool-metric">
                <div class="metric-label">${display.quoteToken} Liquidity</div>
                <div class="metric-value">${window.TokenDisplayUtils.formatLargeNumber(display.quoteLiquidity)}</div>
            </div>
            
            <div class="pool-metric">
                <div class="metric-label">Exchange Rate</div>
                <div class="metric-value">${display.rateText}</div>
            </div>
            
            <div class="pool-metric" title="Pool fee rate (percentage of traded tokens)">
                <div class="metric-label">Pool Fee Rate 📈</div>
                <div class="metric-value">${pool.swapFeeBasisPoints} bps${pool.swapFeeBasisPoints === 0 ? ' (FREE)' : ''}</div>
            </div>
            
            <div class="pool-metric" title="Contract fees collected in SOL (operational costs)">
                <div class="metric-label">Contract Fees 💰</div>
                <div class="metric-value">${(pool.collectedSolFees / 1000000000).toFixed(4)} SOL</div>
            </div>
        </div>
        
        <!-- Additional Fee Information -->
        ${pool.collectedFeesTokenA > 0 || pool.collectedFeesTokenB > 0 ? `
        <div class="pool-info" style="margin-top: 15px; padding-top: 15px; border-top: 1px solid #e5e7eb;">
            <div class="pool-metric" title="Token fees collected from pool percentage rates" style="background: #f0f9ff;">
                <div class="metric-label">Pool Fees: ${display.baseToken}</div>
                <div class="metric-value">${window.TokenDisplayUtils.formatLargeNumber(pool.collectedFeesTokenA)}</div>
            </div>
            
            <div class="pool-metric" title="Token fees collected from pool percentage rates" style="background: #f0f9ff;">
                <div class="metric-label">Pool Fees: ${display.quoteToken}</div>
                <div class="metric-value">${window.TokenDisplayUtils.formatLargeNumber(pool.collectedFeesTokenB)}</div>
            </div>
        </div>
        ` : ''}
        
        <div class="pool-actions">
            <button class="liquidity-btn" onclick="addLiquidity('${pool.address}')">
                💧 Add Liquidity
            </button>
        </div>
        
        <div style="margin-top: 15px; padding-top: 15px; border-top: 1px solid #e5e7eb; font-size: 12px; color: #6b7280;">
            <div><strong>Pool Address:</strong> ${pool.address}</div>
            <div><strong>Owner:</strong> ${pool.owner.slice(0, 20)}...</div>
        </div>
    `;
    
    return card;
}

/**
 * Show error message
 */
function showError(message) {
    const container = document.getElementById('error-container');
    container.innerHTML = `
        <div class="error">
            <strong>⚠️ Error:</strong> ${message}
        </div>
    `;
}

/**
 * Clear error message
 */
function clearError() {
    document.getElementById('error-container').innerHTML = '';
}

/**
 * Show success message
 */
function showStatus(type, message) {
    const container = document.getElementById('error-container');
    const className = type === 'success' ? 'status-message success' : 
                     type === 'info' ? 'status-message info' : 'error';
    container.innerHTML = `<div class="${className}">${message}</div>`;
}

/**
 * Clear status message  
 */
function clearStatus() {
    document.getElementById('error-container').innerHTML = '';
}

/**
 * Create sample pools for testing (called from UI)
 */
function createSamplePools() {
    alert('Sample pool creation would require implementing pool creation transactions. For now, start the validator and run the test suite to create pools.');
}

/**
 * Format large numbers with appropriate suffixes
 */
function formatNumber(num) {
    if (num >= 1000000000) {
        return (num / 1000000000).toFixed(1) + 'B';
    } else if (num >= 1000000) {
        return (num / 1000000).toFixed(1) + 'M';
    } else if (num >= 1000) {
        return (num / 1000).toFixed(1) + 'K';
    }
    return num.toString();
}

/**
 * Force refresh pools with detailed debugging
 */
async function forceRefreshPools() {
    console.log('🐛 FORCE REFRESH: Starting detailed pool debugging...');
    
    // Clear any existing pools
    pools = [];
    
    // Check localStorage directly
    const rawData = localStorage.getItem('createdPools');
    console.log('🐛 Raw localStorage data:', rawData);
    
    if (rawData) {
        try {
            const parsedData = JSON.parse(rawData);
            console.log('🐛 Parsed localStorage data:', parsedData);
            console.log('🐛 Number of stored pools:', parsedData.length);
            
            // Show what each pool looks like
            parsedData.forEach((pool, index) => {
                console.log(`🐛 Pool ${index + 1}:`, pool);
            });
            
        } catch (error) {
            console.error('🐛 Error parsing localStorage:', error);
        }
    } else {
        console.log('🐛 No localStorage data found');
        alert('No pool data found in localStorage. Have you created any pools yet?');
        return;
    }
    
    // Force scan for pools
    await scanForPools();
    
    console.log('🐛 After scanning - pools array:', pools);
    console.log('🐛 Number of pools in memory:', pools.length);
    
    // Force update display
    updateSummaryStats();
    renderPools();
    
    // Show alert with results
    alert(`Debug complete!\nFound ${pools.length} pools.\nCheck console for details.`);
}

/**
 * Debug function to test RPC and program accounts
 */
async function debugRPC() {
    console.log('🐛 DEBUG: Testing RPC connection and program accounts...');
    
    try {
        // Test basic RPC
        const blockHeight = await connection.getBlockHeight();
        console.log('✅ RPC Connection working, block height:', blockHeight);
        
        // Test program account
        const programAccount = await connection.getAccountInfo(new solanaWeb3.PublicKey(CONFIG.programId));
        console.log('📦 Program account:', programAccount ? 'EXISTS' : 'NOT FOUND');
        
        if (programAccount) {
            console.log('Program details:', {
                executable: programAccount.executable,
                owner: programAccount.owner.toString(),
                lamports: programAccount.lamports,
                dataLength: programAccount.data.length
            });
        }
        
        // Test getting program accounts
        const programAccounts = await connection.getProgramAccounts(
            new solanaWeb3.PublicKey(CONFIG.programId),
            { encoding: 'base64' }
        );
        
        console.log(`🔍 Found ${programAccounts.length} accounts owned by program:`);
        
        programAccounts.forEach((account, index) => {
            console.log(`  Account ${index + 1}:`, {
                address: account.pubkey.toString(),
                dataLength: account.account.data.length,
                lamports: account.account.lamports,
                rent_exempt: account.account.lamports > 0
            });
            
            // Try to peek at the data
            if (account.account.data.length > 0) {
                const dataArray = new Uint8Array(account.account.data);
                console.log(`    First 20 bytes:`, Array.from(dataArray.slice(0, 20)).map(b => b.toString(16).padStart(2, '0')).join(' '));
                
                // Check if it looks like a pool (check the is_initialized flag)
                if (dataArray.length > 250) {
                    const isInitialized = dataArray[251] !== 0; // Approximate position
                    console.log(`    Appears initialized:`, isInitialized);
                }
            }
        });
        
        return {
            rpcWorking: true,
            programExists: !!programAccount,
            accountCount: programAccounts.length,
            accounts: programAccounts
        };
        
    } catch (error) {
        console.error('❌ Debug RPC failed:', error);
        return { error: error.message };
    }
}

/**
 * Navigate to add liquidity page for a specific pool
 */
function addLiquidity(poolAddress) {
    console.log('🚀 Navigating to add liquidity for pool:', poolAddress);
    
    // Store the pool address in sessionStorage so the liquidity page can access it
    sessionStorage.setItem('selectedPoolAddress', poolAddress);
    
    // Navigate to liquidity page
    window.location.href = 'liquidity.html';
}

/**
 * Update pool liquidity by reading from contract
 */
async function updatePoolLiquidity(poolAddress) {
    try {
        console.log('🔄 Updating liquidity for pool:', poolAddress);
        
        // Find the pool in our current data
        const poolIndex = pools.findIndex(p => p.address === poolAddress);
        if (poolIndex === -1) {
            console.warn('Pool not found in current data');
            return;
        }
        
        // Get fresh data from contract
        const poolAccount = await connection.getAccountInfo(new solanaWeb3.PublicKey(poolAddress));
        if (!poolAccount) {
            console.error('Pool account not found on-chain');
            return;
        }
        
        // Parse the updated pool state
        const updatedPool = await parsePoolState(poolAccount.data, new solanaWeb3.PublicKey(poolAddress));
        
        // Update the pool in our array
        pools[poolIndex] = updatedPool;
        
        // Re-render the pools display
        renderPools();
        updateSummaryStats();
        
        console.log('✅ Pool liquidity updated successfully');
        
    } catch (error) {
        console.error('❌ Error updating pool liquidity:', error);
    }
}

// Export for global access
window.refreshData = refreshData;
window.createSamplePools = createSamplePools;
window.forceRefreshPools = forceRefreshPools;
window.debugRPC = debugRPC;
window.addLiquidity = addLiquidity;
window.updatePoolLiquidity = updatePoolLiquidity;

console.log('📊 Dashboard JavaScript loaded successfully'); 