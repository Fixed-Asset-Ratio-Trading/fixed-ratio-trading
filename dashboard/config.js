// Centralized Configuration for Fixed Ratio Trading Dashboard
// This file loads configuration from the centralized shared-config.json file

// Load configuration from centralized config file
async function loadSharedConfig() {
    try {
        const response = await fetch('../shared-config.json');
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        const sharedConfig = await response.json();
        
        // Map shared config to dashboard format for backward compatibility
        window.TRADING_CONFIG = {
            // Solana connection settings
            rpcUrl: sharedConfig.solana.rpcUrl,
            wsUrl: sharedConfig.solana.wsUrl,
            commitment: sharedConfig.solana.commitment,
            disableRetryOnRateLimit: sharedConfig.solana.disableRetryOnRateLimit,
            
            // Program settings
            programId: sharedConfig.program.programId,
            poolStateSeedPrefix: sharedConfig.program.poolStateSeedPrefix,
            
            // Dashboard settings
            refreshInterval: sharedConfig.dashboard.refreshInterval,
            stateFile: sharedConfig.dashboard.stateFile,
            
            // Wallet settings
            expectedWallet: sharedConfig.wallets.expectedBackpackWallet,
            
            // Version info
            version: sharedConfig.version,
            lastUpdated: sharedConfig.lastUpdated
        };
        
        console.log('✅ Shared configuration loaded:', sharedConfig.solana.rpcUrl);
        return true;
    } catch (error) {
        console.error('❌ Failed to load shared configuration:', error);
        
        // Fallback to hardcoded values
        window.TRADING_CONFIG = {
            rpcUrl: 'http://192.168.2.88:8899',
            wsUrl: null,
            programId: '4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn',
            commitment: 'confirmed',
            disableRetryOnRateLimit: true,
            refreshInterval: 10000,
            poolStateSeedPrefix: 'pool_state',
            expectedWallet: '5GGZiMwU56rYL1L52q7Jz7ELkSN4iYyQqdv418hxPh6t',
            version: '1.0.0',
            lastUpdated: '2024-01-15'
        };
        
        console.warn('⚠️ Using fallback configuration');
        return false;
    }
}

// Initialize configuration immediately
loadSharedConfig();

// Legacy alias for backward compatibility
window.CONFIG = window.TRADING_CONFIG;

console.log('✅ Trading configuration loaded:', window.TRADING_CONFIG.rpcUrl); 