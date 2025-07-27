/**
 * Token Pair Display Utilities
 * User-friendly display patterns for Fixed Ratio Trading Dashboard
 */

/**
 * Get user-friendly display order for token pairs
 * Phase 1.3: Implements special handling for One-to-many ratio pools (bit 0 flag)
 * 
 * @param {Object} pool - Pool data with ratioANumerator, ratioBDenominator, tokenASymbol, tokenBSymbol, flags, etc.
 * @returns {Object} Display configuration with base/quote tokens and exchange rates
 */
function getDisplayTokenOrder(pool) {
    // Handle missing or invalid data
    if (!pool || !pool.tokenASymbol || !pool.tokenBSymbol) {
        return {
            baseToken: 'Token A',
            quoteToken: 'Token B',
            baseLiquidity: 0,
            quoteLiquidity: 0,
            exchangeRate: 1,
            displayPair: 'Token A/Token B',
            rateText: '1 Token A = 1.000 Token B',
            isReversed: false,
            isOneToManyRatio: false
        };
    }

    const ratioANumerator = pool.ratioANumerator || 1;
    const ratioBDenominator = pool.ratioBDenominator || 1;
    
    // Check if this is a One-to-many ratio pool (bit 0 flag)
    const isOneToManyRatio = checkOneToManyRatioFlag(pool);
    
    // CRITICAL FIX: Stored ratio means "ratioANumerator of TokenA per ratioBDenominator of TokenB"
    // So ratioANumerator:ratioBDenominator = 10000:1 means "10000 TokenA per 1 TokenB"
    const tokensA_per_tokenB = ratioANumerator / ratioBDenominator;  // How many A per B
    const tokensB_per_tokenA = ratioBDenominator / ratioANumerator;  // How many B per A
    
    if (isOneToManyRatio) {
        // **Phase 1.3: One-to-many ratio special handling**
        // Place token with value 1 (excluding decimals) first
        // Ignore normalization for these pools
        
        if (ratioBDenominator === 1) {
            // TokenB has ratio of 1, display as TokenB/TokenA
            // Example: USDT/SOL 1002:1 → Display as SOL/USDT 1:1002
            return {
                baseToken: pool.tokenBSymbol,
                quoteToken: pool.tokenASymbol,
                baseLiquidity: pool.tokenBLiquidity || 0,
                quoteLiquidity: pool.tokenALiquidity || 0,
                exchangeRate: tokensA_per_tokenB,
                displayPair: `${pool.tokenBSymbol}/${pool.tokenASymbol}`,
                rateText: `1 ${pool.tokenBSymbol} = ${formatNumberWithCommas(ratioANumerator)} ${pool.tokenASymbol}`,
                isReversed: true,
                isOneToManyRatio: true
            };
        } else if (ratioANumerator === 1) {
            // TokenA has ratio of 1, display as TokenA/TokenB
            return {
                baseToken: pool.tokenASymbol,
                quoteToken: pool.tokenBSymbol,
                baseLiquidity: pool.tokenALiquidity || 0,
                quoteLiquidity: pool.tokenBLiquidity || 0,
                exchangeRate: tokensB_per_tokenA,
                displayPair: `${pool.tokenASymbol}/${pool.tokenBSymbol}`,
                rateText: `1 ${pool.tokenASymbol} = ${formatNumberWithCommas(ratioBDenominator)} ${pool.tokenBSymbol}`,
                isReversed: false,
                isOneToManyRatio: true
            };
        }
    }
    
    // **Standard pools: Keep normalized display with fractions to 3 decimal places**
    // Determine which token should be the "base" (ratio = 1) for display
    if (tokensA_per_tokenB >= 1.0) {
        // Many TokenA per 1 TokenB means TokenB is more valuable
        // Display as: TokenB/TokenA (e.g., "1 TS = 10000 MST")
        return {
            baseToken: pool.tokenBSymbol,
            quoteToken: pool.tokenASymbol,
            baseLiquidity: pool.tokenBLiquidity || 0,
            quoteLiquidity: pool.tokenALiquidity || 0,
            exchangeRate: tokensA_per_tokenB,
            displayPair: `${pool.tokenBSymbol}/${pool.tokenASymbol}`,
            rateText: `1 ${pool.tokenBSymbol} = ${formatExchangeRateStandard(tokensA_per_tokenB)} ${pool.tokenASymbol}`,
            isReversed: true,
            isOneToManyRatio: false
        };
    } else {
        // Many TokenB per 1 TokenA means TokenA is more valuable  
        // Display as: TokenA/TokenB (e.g., "1 BTC = 50000 USDC")
        return {
            baseToken: pool.tokenASymbol,
            quoteToken: pool.tokenBSymbol,
            baseLiquidity: pool.tokenALiquidity || 0,
            quoteLiquidity: pool.tokenBLiquidity || 0,
            exchangeRate: tokensB_per_tokenA,
            displayPair: `${pool.tokenASymbol}/${pool.tokenBSymbol}`,
            rateText: `1 ${pool.tokenASymbol} = ${formatExchangeRateStandard(tokensB_per_tokenA)} ${pool.tokenBSymbol}`,
            isReversed: false,
            isOneToManyRatio: false
        };
    }
}

/**
 * Phase 1.3: Check if pool has One-to-many ratio flag (bit 0) set
 * 
 * @param {Object} pool - Pool data with flags or flagsDecoded
 * @returns {boolean} True if One-to-many ratio flag is set
 */
function checkOneToManyRatioFlag(pool) {
    // Check flagsDecoded first (from JSON state)
    if (pool.flagsDecoded && typeof pool.flagsDecoded.one_to_many_ratio === 'boolean') {
        return pool.flagsDecoded.one_to_many_ratio;
    }
    
    // Check raw flags field (bitwise check for bit 0)
    if (typeof pool.flags === 'number') {
        return (pool.flags & 1) !== 0; // Bit 0 (value 1)
    }
    
    return false;
}

/**
 * Phase 1.3: Pool State Flags Interpretation
 * 
 * @param {Object} pool - Pool data with flags
 * @returns {Object} Decoded flag information
 */
function interpretPoolFlags(pool) {
    const flags = pool.flags || 0;
    
    return {
        oneToManyRatio: (flags & 1) !== 0,        // Bit 0 (1): One-to-many ratio configuration
        liquidityPaused: (flags & 2) !== 0,       // Bit 1 (2): Liquidity operations paused
        swapsPaused: (flags & 4) !== 0,           // Bit 2 (4): Swap operations paused
        withdrawalProtection: (flags & 8) !== 0,   // Bit 3 (8): Withdrawal protection active
        singleLpTokenMode: (flags & 16) !== 0      // Bit 4 (16): Single LP token mode (future feature)
    };
}

/**
 * Phase 1.3: Format exchange rate for standard pools with 3 decimal places
 * 
 * @param {number} rate - Exchange rate to format
 * @returns {string} Formatted rate string with 3 decimal places
 */
function formatExchangeRateStandard(rate) {
    if (rate >= 1000000) {
        // Use scientific notation for very large numbers
        return rate.toExponential(2);
    } else if (rate >= 100) {
        // 3 decimal places for standard pools as per Phase 1.3 requirements
        return rate.toLocaleString('en-US', { 
            minimumFractionDigits: 3,
            maximumFractionDigits: 3
        });
    } else if (rate >= 1) {
        // 3 decimal places for normal numbers
        return rate.toLocaleString('en-US', { 
            minimumFractionDigits: 3,
            maximumFractionDigits: 3
        });
    } else if (rate >= 0.001) {
        // More decimal places for small numbers but minimum 3
        return rate.toLocaleString('en-US', { 
            minimumFractionDigits: 3,
            maximumFractionDigits: 6
        });
    } else {
        // Scientific notation for very small numbers
        return rate.toExponential(3);
    }
}

/**
 * Legacy format exchange rate function (maintained for compatibility)
 * 
 * @param {number} rate - Exchange rate to format
 * @returns {string} Formatted rate string
 */
function formatExchangeRate(rate) {
    if (rate >= 1000000) {
        // Use scientific notation for very large numbers
        return rate.toExponential(2);
    } else if (rate >= 100) {
        // No decimal places for large whole numbers
        return rate.toLocaleString('en-US', { 
            minimumFractionDigits: 0,
            maximumFractionDigits: 0
        });
    } else if (rate >= 1) {
        // 2 decimal places for normal numbers
        return rate.toLocaleString('en-US', { 
            minimumFractionDigits: 2,
            maximumFractionDigits: 2
        });
    } else if (rate >= 0.01) {
        // More decimal places for small numbers
        return rate.toLocaleString('en-US', { 
            minimumFractionDigits: 4,
            maximumFractionDigits: 4
        });
    } else {
        // Scientific notation for very small numbers
        return rate.toExponential(2);
    }
}

/**
 * Get simplified display for pool creation/summary
 * Used during pool creation where we may not have full pool data
 * 
 * @param {string} tokenASymbol - Token A symbol
 * @param {string} tokenBSymbol - Token B symbol
 * @param {number} ratioANumerator - Ratio A numerator
 * @param {number} ratioBDenominator - Ratio B denominator
 * @returns {Object} Simplified display configuration
 */
function getSimpleDisplayOrder(tokenASymbol, tokenBSymbol, ratioANumerator, ratioBDenominator) {
    const mockPool = {
        tokenASymbol,
        tokenBSymbol,
        ratioANumerator,
        ratioBDenominator,
        tokenALiquidity: 0,
        tokenBLiquidity: 0
    };
    
    return getDisplayTokenOrder(mockPool);
}

/**
 * Format large numbers with appropriate units (K, M, B)
 * 
 * @param {number} num - Number to format
 * @returns {string} Formatted number string
 */
function formatLargeNumber(num) {
    if (num >= 1000000000) {
        return (num / 1000000000).toFixed(1) + 'B';
    } else if (num >= 1000000) {
        return (num / 1000000).toFixed(1) + 'M';
    } else if (num >= 1000) {
        return (num / 1000).toFixed(1) + 'K';
    } else {
        return num.toLocaleString('en-US', { 
            minimumFractionDigits: 0,
            maximumFractionDigits: 2
        });
    }
}

/**
 * Format numbers with commas (no abbreviations) - ideal for ratios and exact amounts
 * 
 * @param {number} num - Number to format
 * @returns {string} Formatted number string with commas
 */
function formatNumberWithCommas(num) {
    if (typeof num !== 'number' || isNaN(num)) {
        return '0';
    }
    
    return num.toLocaleString('en-US', { 
        minimumFractionDigits: 0,
        maximumFractionDigits: 0
    });
}

/**
 * Create user-friendly pool title
 * 
 * @param {Object} pool - Pool data
 * @returns {string} Formatted pool title
 */
function createPoolTitle(pool) {
    const display = getDisplayTokenOrder(pool);
    return `${display.baseToken}/${display.quoteToken} Pool`;
}

/**
 * Create user-friendly exchange rate display
 * 
 * @param {Object} pool - Pool data
 * @returns {string} Formatted exchange rate
 */
function createExchangeRateDisplay(pool) {
    const display = getDisplayTokenOrder(pool);
    return display.rateText;
}

// Make functions available globally for use in other dashboard files
if (typeof window !== 'undefined') {
    window.TokenDisplayUtils = {
        getDisplayTokenOrder,
        formatExchangeRate,
        formatExchangeRateStandard,
        getSimpleDisplayOrder,
        formatLargeNumber,
        formatNumberWithCommas,
        createPoolTitle,
        createExchangeRateDisplay,
        // Phase 1.3: New flag interpretation functions
        checkOneToManyRatioFlag,
        interpretPoolFlags
    };
}

// Export for Node.js environments (if needed)
if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        getDisplayTokenOrder,
        formatExchangeRate,
        formatExchangeRateStandard,
        getSimpleDisplayOrder,
        formatLargeNumber,
        createPoolTitle,
        createExchangeRateDisplay,
        // Phase 1.3: New flag interpretation functions
        checkOneToManyRatioFlag,
        interpretPoolFlags
    };
} 