// Centralized Configuration for Fixed Ratio Trading Dashboard
// This file contains all shared configuration values used across the dashboard

// Global configuration object
window.TRADING_CONFIG = {
    // Solana RPC endpoint - change this to switch between validators
    rpcUrl: 'https://vmdevbox1.dcs1.cc',
    
    // Fixed Ratio Trading program ID
    programId: '4aeVqtWhrUh6wpX8acNj2hpWXKEQwxjA3PYb2sHhNyCn',
    
    // Connection settings
    commitment: 'confirmed',
    
    // Dashboard settings
    refreshInterval: 10000, // 10 seconds
    
    // Pool state configuration
    poolStateSeedPrefix: 'pool_state_v2',
    
    // Expected Backpack wallet for testing (optional)
    expectedWallet: '5GGZiMwU56rYL1L52q7Jz7ELkSN4iYyQqdv418hxPh6t',
    
    // Version info
    version: '1.0.0',
    lastUpdated: '2024-01-15'
};

// Legacy alias for backward compatibility
window.CONFIG = window.TRADING_CONFIG;

console.log('✅ Trading configuration loaded:', window.TRADING_CONFIG.rpcUrl); 