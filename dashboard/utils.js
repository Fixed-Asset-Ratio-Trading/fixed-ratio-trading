/**
 * Token Pair Display Utilities
 * User-friendly display patterns for Fixed Ratio Trading Dashboard
 */

/**
 * Get user-friendly display order for token pairs
 * Always shows the "base token" (ratio = 1) first, regardless of lexicographic order
 * 
 * @param {Object} pool - Pool data with ratioANumerator, ratioBDenominator, tokenASymbol, tokenBSymbol, etc.
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
            rateText: '1 Token A = 1.00 Token B',
            isReversed: false
        };
    }

    const ratioANumerator = pool.ratioANumerator || 1;
    const ratioBDenominator = pool.ratioBDenominator || 1;
    
    // CRITICAL FIX: Stored ratio means "ratioANumerator of TokenA per ratioBDenominator of TokenB"
    // So ratioANumerator:ratioBDenominator = 10000:1 means "10000 TokenA per 1 TokenB"
    const tokensA_per_tokenB = ratioANumerator / ratioBDenominator;  // How many A per B
    const tokensB_per_tokenA = ratioBDenominator / ratioANumerator;  // How many B per A
    
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
            rateText: `1 ${pool.tokenBSymbol} = ${formatExchangeRate(tokensA_per_tokenB)} ${pool.tokenASymbol}`,
            isReversed: true
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
            rateText: `1 ${pool.tokenASymbol} = ${formatExchangeRate(tokensB_per_tokenA)} ${pool.tokenBSymbol}`,
            isReversed: false
        };
    }
}

/**
 * Format exchange rate with appropriate decimal places and notation
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
        getSimpleDisplayOrder,
        formatLargeNumber,
        createPoolTitle,
        createExchangeRateDisplay
    };
}

// Export for Node.js environments (if needed)
if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        getDisplayTokenOrder,
        formatExchangeRate,
        getSimpleDisplayOrder,
        formatLargeNumber,
        createPoolTitle,
        createExchangeRateDisplay
    };
} 