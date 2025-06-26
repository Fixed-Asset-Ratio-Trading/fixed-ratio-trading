// Fixed Ratio Trading Dashboard - JavaScript Logic
// Connects to local Solana testnet and displays real-time pool information

// Configuration
const CONFIG = {
    rpcUrl: 'http://localhost:8899',
    programId: '4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn',
    refreshInterval: 10000, // 10 seconds
    poolStateSeedPrefix: 'pool_state_v2',
};

// Global state
let connection = null;
let pools = [];
let lastUpdate = null;
let refreshTimer = null;
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
        // Initialize Solana connection
        connection = new solanaWeb3.Connection(CONFIG.rpcUrl, 'confirmed');
        
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
        
        // Start auto-refresh
        startAutoRefresh();
        
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
        // GetVersion is index 25 in the PoolInstruction enum
        const getVersionInstruction = new Uint8Array([25]);
        
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
 * Scan for Fixed Ratio Trading pools (both on-chain and locally created)
 */
async function scanForPools() {
    try {
        console.log('🔍 Scanning for pools...');
        
        let onChainPools = [];
        let localPools = [];
        
        // Try to get on-chain pools
        try {
            const programAccounts = await connection.getProgramAccounts(
                new solanaWeb3.PublicKey(CONFIG.programId),
                {
                    filters: [
                        // Filter for pool state accounts (approximate size)
                        { dataSize: 1000 } // Adjust based on actual PoolState size
                    ]
                }
            );
            
            console.log(`Found ${programAccounts.length} program accounts`);
            
            const poolPromises = programAccounts.map(async (account) => {
                try {
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
        
        // Get locally created pools from localStorage
        try {
            const storedPoolsRaw = localStorage.getItem('createdPools') || '[]';
            console.log('📦 Raw localStorage data:', storedPoolsRaw);
            
            const storedPools = JSON.parse(storedPoolsRaw);
            console.log('📦 Parsed stored pools:', storedPools);
            
            localPools = storedPools.map(pool => {
                // Convert local pool format to dashboard format
                const converted = {
                    address: pool.address,
                    isInitialized: pool.isInitialized,
                    isPaused: pool.isPaused,
                    swapsPaused: pool.swapsPaused,
                    tokenAMint: pool.tokenAMint,
                    tokenBMint: pool.tokenBMint,
                    tokenALiquidity: pool.totalTokenALiquidity,
                    tokenBLiquidity: pool.totalTokenBLiquidity,
                    ratioANumerator: pool.ratio,
                    ratioBDenominator: 1,
                    swapFeeBasisPoints: pool.swapFeeBasisPoints,
                    collectedFeesTokenA: pool.collectedFeesTokenA,
                    collectedFeesTokenB: pool.collectedFeesTokenB,
                    collectedSolFees: pool.collectedSolFees,
                    delegateCount: pool.delegateCount,
                    owner: pool.creator,
                    // Add symbols for better display
                    tokenASymbol: pool.tokenASymbol,
                    tokenBSymbol: pool.tokenBSymbol
                };
                console.log('📦 Converted pool:', converted);
                return converted;
            });
            console.log(`✅ Loaded ${localPools.length} locally created pools`);
        } catch (error) {
            console.warn('⚠️ Error loading local pools:', error);
        }
        
        // Combine both sources (remove duplicates by address if any)
        const allPools = [...onChainPools, ...localPools];
        const uniquePools = allPools.filter((pool, index, self) => 
            index === self.findIndex(p => p.address === pool.address)
        );
        
        pools = uniquePools;
        console.log(`✅ Total unique pools loaded: ${pools.length} (${onChainPools.length} on-chain + ${localPools.length} local)`);
    } catch (error) {
        console.error('❌ Error scanning for pools:', error);
        throw error;
    }
}

/**
 * Parse pool state data from account data
 */
async function parsePoolState(data, address) {
    try {
        // Basic validation
        if (!data || data.length < 100) {
            throw new Error('Invalid account data size');
        }
        
        // Simple binary data parsing (adjust based on actual PoolState structure)
        const dataArray = new Uint8Array(data);
        
        // Check if account is initialized (first check for non-zero data)
        const isInitialized = dataArray.some(byte => byte !== 0);
        if (!isInitialized) {
            throw new Error('Account not initialized');
        }
        
        // For demonstration, create a mock pool structure
        // In a real implementation, you'd use proper Borsh deserialization
        const mockPool = {
            address: address.toString(),
            isInitialized: true,
            isPaused: Math.random() > 0.8, // Random pause status for demo
            swapsPaused: Math.random() > 0.9,
            tokenAMint: generateMockAddress(),
            tokenBMint: generateMockAddress(),
            tokenALiquidity: Math.floor(Math.random() * 1000000),
            tokenBLiquidity: Math.floor(Math.random() * 1000000),
            ratioANumerator: Math.floor(Math.random() * 10) + 1,
            ratioBDenominator: 1,
            swapFeeBasisPoints: Math.floor(Math.random() * 50),
            collectedFeesTokenA: Math.floor(Math.random() * 10000),
            collectedFeesTokenB: Math.floor(Math.random() * 10000),
            collectedSolFees: Math.floor(Math.random() * 5000000), // lamports
            delegateCount: Math.floor(Math.random() * 3),
            owner: generateMockAddress()
        };
        
        return mockPool;
    } catch (error) {
        throw new Error(`Failed to parse pool state: ${error.message}`);
    }
}

/**
 * Generate a mock address for demonstration
 */
function generateMockAddress() {
    const chars = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
    let result = '';
    for (let i = 0; i < 44; i++) {
        result += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return result;
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
    
    const totalDelegates = pools.reduce((sum, pool) => sum + pool.delegateCount, 0);
    
    // Update DOM elements
    document.getElementById('total-pools').textContent = totalPools;
    document.getElementById('active-pools').textContent = activePools;
    document.getElementById('paused-pools').textContent = pausedPools;
    document.getElementById('total-tvl').textContent = `${totalTVL.toLocaleString()} tokens`;
    document.getElementById('avg-pool-size').textContent = `${avgPoolSize.toLocaleString()} tokens`;
    document.getElementById('total-fees').textContent = `${(totalFeesSOL / 1000000000).toFixed(4)} SOL`;
    document.getElementById('avg-swap-fee').textContent = `${avgSwapFee} bps`;
    document.getElementById('total-delegates').textContent = totalDelegates;
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
    
    const exchangeRate = pool.ratioBDenominator > 0 ? 
        (pool.ratioANumerator / pool.ratioBDenominator).toFixed(2) : '0';
    
    // Use symbol information if available for better display
    const displayTitle = (pool.tokenASymbol && pool.tokenBSymbol) 
        ? `${pool.tokenASymbol} / ${pool.tokenBSymbol} Pool`
        : `Pool ${pool.address.slice(0, 8)}...${pool.address.slice(-4)}`;
    
    card.innerHTML = `
        <div class="pool-header">
            <div class="pool-title">
                ${displayTitle}
            </div>
            <div class="pool-status ${status}">${statusText}</div>
        </div>
        
        <div class="pool-info">
            <div class="pool-metric">
                <div class="metric-label">Token A Liquidity</div>
                <div class="metric-value">${pool.tokenALiquidity.toLocaleString()}</div>
            </div>
            
            <div class="pool-metric">
                <div class="metric-label">Token B Liquidity</div>
                <div class="metric-value">${pool.tokenBLiquidity.toLocaleString()}</div>
            </div>
            
            <div class="pool-metric">
                <div class="metric-label">Exchange Rate</div>
                <div class="metric-value">${exchangeRate}:1</div>
            </div>
            
            <div class="pool-metric">
                <div class="metric-label">Swap Fee</div>
                <div class="metric-value">${pool.swapFeeBasisPoints} bps</div>
            </div>
            
            <div class="pool-metric">
                <div class="metric-label">Collected Fees (SOL)</div>
                <div class="metric-value">${(pool.collectedSolFees / 1000000000).toFixed(4)}</div>
            </div>
            
            <div class="pool-metric">
                <div class="metric-label">Delegates</div>
                <div class="metric-value">${pool.delegateCount}/3</div>
            </div>
        </div>
        
        <div style="margin-top: 15px; padding-top: 15px; border-top: 1px solid #e5e7eb; font-size: 12px; color: #6b7280;">
            <div><strong>Pool Address:</strong> ${pool.address}</div>
            <div><strong>Owner:</strong> ${pool.owner.slice(0, 20)}...</div>
        </div>
    `;
    
    return card;
}

/**
 * Start auto-refresh timer
 */
function startAutoRefresh() {
    if (refreshTimer) {
        clearInterval(refreshTimer);
    }
    
    refreshTimer = setInterval(async () => {
        console.log('🔄 Auto-refreshing dashboard...');
        await refreshData();
    }, CONFIG.refreshInterval);
    
    console.log(`✅ Auto-refresh started (every ${CONFIG.refreshInterval / 1000} seconds)`);
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
 * Handle window visibility changes to pause/resume refreshing
 */
document.addEventListener('visibilitychange', () => {
    if (document.hidden) {
        console.log('📱 Page hidden - pausing auto-refresh');
        if (refreshTimer) {
            clearInterval(refreshTimer);
        }
    } else {
        console.log('📱 Page visible - resuming auto-refresh');
        startAutoRefresh();
        // Refresh immediately when page becomes visible
        refreshData();
    }
});

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

// Export for global access
window.refreshData = refreshData;
window.createSamplePools = createSamplePools;
window.forceRefreshPools = forceRefreshPools;

console.log('📊 Dashboard JavaScript loaded successfully'); 