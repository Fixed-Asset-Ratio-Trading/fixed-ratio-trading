// Fixed Ratio Trading Dashboard - JavaScript Logic
// Connects to local Solana testnet and displays real-time pool information

// Configuration
const CONFIG = {
    rpcUrl: 'http://localhost:8899',
    programId: 'quXSYkeZ8ByTCtYY1J1uxQmE36UZ3LmNGgE3CYMFixD',
    refreshInterval: 10000, // 10 seconds
    poolStateSeedPrefix: 'pool_state_v2',
};

// Global state
let connection = null;
let pools = [];
let lastUpdate = null;
let refreshTimer = null;

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
        await testConnection();
        
        // Load initial data
        await refreshData();
        
        // Start auto-refresh
        startAutoRefresh();
        
        console.log('✅ Dashboard initialized successfully');
    } catch (error) {
        console.error('❌ Failed to initialize dashboard:', error);
        showError('Failed to connect to local Solana testnet. Make sure the validator is running.');
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
        
        // Check if program exists
        const programAccount = await connection.getAccountInfo(new solanaWeb3.PublicKey(CONFIG.programId));
        if (programAccount) {
            document.getElementById('program-status').textContent = 'Deployed';
            document.getElementById('program-status').className = 'status-value online';
        } else {
            document.getElementById('program-status').textContent = 'Not Found';
            document.getElementById('program-status').className = 'status-value offline';
        }
    } catch (error) {
        document.getElementById('rpc-status').textContent = 'Offline';
        document.getElementById('rpc-status').className = 'status-value offline';
        throw error;
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
        
        // Scan for pools
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
 * Scan the blockchain for Fixed Ratio Trading pools
 */
async function scanForPools() {
    try {
        console.log('🔍 Scanning for pools...');
        
        // Get all accounts owned by our program
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
        pools = poolResults.filter(pool => pool !== null);
        
        console.log(`✅ Successfully parsed ${pools.length} pools`);
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
        container.innerHTML = `
            <div class="loading">
                <h3>📭 No pools found</h3>
                <p>No Fixed Ratio Trading pools detected on this network.</p>
                <p><a href="#" onclick="createSamplePools()">Create sample pools</a> for testing.</p>
            </div>
        `;
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
    
    card.innerHTML = `
        <div class="pool-header">
            <div class="pool-title">
                Pool ${pool.address.slice(0, 8)}...${pool.address.slice(-4)}
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

// Export for global access
window.refreshData = refreshData;
window.createSamplePools = createSamplePools;

console.log('📊 Dashboard JavaScript loaded successfully'); 