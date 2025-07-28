/**
 * Token Pair Display Utilities
 * User-friendly display patterns for Fixed Ratio Trading Dashboard
 */

/**
 * SIMPLE TOKEN DISPLAY CORRECTOR
 * If a token has precision value of 1 in the ratio, it comes first!
 * 
 * @param {string} tokenAName - Token A symbol/name
 * @param {string} tokenBName - Token B symbol/name  
 * @param {number} tokenARatio - Token A ratio value (numerator)
 * @param {number} tokenBRatio - Token B ratio value (denominator)
 * @param {number} tokenAPrecision - Token A decimal precision (optional)
 * @param {number} tokenBPrecision - Token B decimal precision (optional)
 * @returns {Object} Simple display configuration
 */
function getCorrectTokenDisplay(tokenAName, tokenBName, tokenARatio, tokenBRatio, tokenAPrecision = 6, tokenBPrecision = 6) {
    console.log('🔧 SHOWING ACTUAL POOL RATIO:', {
        tokenAName, tokenBName, tokenARatio, tokenBRatio, tokenAPrecision, tokenBPrecision
    });
    
    // CORRECT CALCULATION: Show what the pool actually represents
    // Based on swap formula: amount_out_B = amount_in_A * ratio_B_denominator / ratio_A_numerator
    // So: 1 TokenA gets you (ratio_B_denominator / ratio_A_numerator) TokenB
    
    const actualExchangeRate = tokenBRatio / tokenARatio;
    
    if (actualExchangeRate >= 1) {
        // TokenA is more valuable, show as "1 TokenA = X TokenB"
        return {
            baseToken: tokenAName,
            quoteToken: tokenBName,
            displayPair: `${tokenAName}/${tokenBName}`,
            rateText: `1 ${tokenAName} = ${formatNumberWithCommas(actualExchangeRate)} ${tokenBName}`,
            exchangeRate: actualExchangeRate,
            isReversed: false
        };
    } else {
        // TokenB is more valuable, show as "1 TokenB = X TokenA"
        const inverseRate = tokenARatio / tokenBRatio;
        return {
            baseToken: tokenBName,
            quoteToken: tokenAName,
            displayPair: `${tokenBName}/${tokenAName}`,
            rateText: `1 ${tokenBName} = ${formatNumberWithCommas(inverseRate)} ${tokenAName}`,
            exchangeRate: inverseRate,
            isReversed: true
        };
    }
}

/**
 * OVERRIDE FUNCTION: Use simple logic instead of complex getDisplayTokenOrder
 * 
 * @param {Object} pool - Pool data
 * @param {Object} tokenDecimals - Optional decimal info
 * @returns {Object} Corrected display configuration
 */
function getDisplayTokenOrderCorrected(pool, tokenDecimals = null) {
    // Extract data with fallbacks for different naming conventions
    const tokenAName = pool.tokenASymbol || 'Token A';
    const tokenBName = pool.tokenBSymbol || 'Token B';
    const tokenARatio = pool.ratioANumerator || pool.ratio_a_numerator || 1;
    const tokenBRatio = pool.ratioBDenominator || pool.ratio_b_denominator || 1;
    const tokenAPrecision = tokenDecimals?.tokenADecimals || 6;
    const tokenBPrecision = tokenDecimals?.tokenBDecimals || 6;
    
    console.log('🔧 USING CORRECTED DISPLAY LOGIC');
    
    const result = getCorrectTokenDisplay(tokenAName, tokenBName, tokenARatio, tokenBRatio, tokenAPrecision, tokenBPrecision);
    
    // Add additional fields that the UI expects
    const getFormattedLiquidity = (rawAmount, isTokenA) => {
        if (tokenDecimals) {
            const decimals = isTokenA ? tokenDecimals.tokenADecimals : tokenDecimals.tokenBDecimals;
            return formatLiquidityAmount(rawAmount, decimals);
        }
        return formatLargeNumber(rawAmount);
    };
    
    const flags = interpretPoolFlags(pool);
    
    return {
        baseToken: result.baseToken,
        quoteToken: result.quoteToken,
        displayPair: result.displayPair,
        rateText: result.rateText,
        exchangeRate: result.exchangeRate,
        baseLiquidity: result.isReversed 
            ? getFormattedLiquidity(pool.tokenBLiquidity || pool.total_token_b_liquidity || 0, false)
            : getFormattedLiquidity(pool.tokenALiquidity || pool.total_token_a_liquidity || 0, true),
        quoteLiquidity: result.isReversed
            ? getFormattedLiquidity(pool.tokenALiquidity || pool.total_token_a_liquidity || 0, true) 
            : getFormattedLiquidity(pool.tokenBLiquidity || pool.total_token_b_liquidity || 0, false),
        isReversed: result.isReversed,
        isOneToManyRatio: flags.oneToManyRatio
    };
}

/**
 * Get user-friendly display order for token pairs
 * NOW USES THE CORRECTED LOGIC!
 * 
 * @param {Object} pool - Pool data with ratioANumerator, ratioBDenominator, tokenASymbol, tokenBSymbol, flags, etc.
 * @param {Object} tokenDecimals - Optional object with tokenADecimals and tokenBDecimals for proper liquidity formatting
 * @returns {Object} Display configuration with base/quote tokens and exchange rates
 */
function getDisplayTokenOrder(pool, tokenDecimals = null) {
    // Use the corrected display logic
    return getDisplayTokenOrderCorrected(pool, tokenDecimals);
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
 * Format liquidity amounts accounting for token decimal precision
 * 
 * @param {number} rawAmount - Raw amount from blockchain (in smallest units)
 * @param {number} decimals - Token decimal places (default: 6)
 * @returns {string} Formatted amount string with units
 */
function formatLiquidityAmount(rawAmount, decimals = 6) {
    if (typeof rawAmount !== 'number' || isNaN(rawAmount) || rawAmount < 0) {
        return '0';
    }
    
    // Convert from raw units to human-readable amount
    const adjustedAmount = rawAmount / Math.pow(10, decimals);
    
    // Use formatLargeNumber for consistent formatting
    return formatLargeNumber(adjustedAmount);
}

/**
 * Get token decimals from mint address using RPC
 * 
 * @param {string} mintAddress - Token mint address
 * @param {Object} connection - Solana connection object
 * @returns {Promise<number>} Token decimals (defaults to 6 if fetch fails)
 */
async function getTokenDecimals(mintAddress, connection) {
    if (!connection || !mintAddress) {
        throw new Error(`Invalid parameters for getTokenDecimals: connection=${!!connection}, mintAddress=${mintAddress}`);
    }
    
    try {
        const mintInfo = await connection.getParsedAccountInfo(
            new solanaWeb3.PublicKey(mintAddress)
        );
        
        if (!mintInfo.value) {
            throw new Error(`Token mint account not found: ${mintAddress}`);
        }
        
        if (!mintInfo.value.data.parsed) {
            throw new Error(`Token mint account data not parsed: ${mintAddress}`);
        }
        
        const decimals = mintInfo.value.data.parsed.info.decimals;
        
        if (decimals === undefined || decimals === null) {
            throw new Error(`Token decimals not found in mint info: ${mintAddress}`);
        }
        
        console.log(`✅ Fetched decimals for token ${mintAddress}: ${decimals}`);
        return decimals;
        
    } catch (error) {
        console.error(`❌ Failed to fetch decimals for token ${mintAddress}:`, error);
        throw new Error(`Cannot determine token decimals for ${mintAddress}. This is required for safe transaction processing. Error: ${error.message}`);
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
        getDisplayTokenOrderCorrected,  // NEW: The corrected logic
        getCorrectTokenDisplay,         // NEW: Simple corrector function
        formatExchangeRate,
        formatExchangeRateStandard,
        getSimpleDisplayOrder,
        formatLargeNumber,
        formatLiquidityAmount,
        getTokenDecimals,
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
        getDisplayTokenOrderCorrected,  // NEW: The corrected logic
        getCorrectTokenDisplay,         // NEW: Simple corrector function
        formatExchangeRate,
        formatExchangeRateStandard,
        getSimpleDisplayOrder,
        formatLargeNumber,
        formatLiquidityAmount,
        getTokenDecimals,
        formatNumberWithCommas,
        createPoolTitle,
        createExchangeRateDisplay,
        // Phase 1.3: New flag interpretation functions
        checkOneToManyRatioFlag,
        interpretPoolFlags
    };
} 