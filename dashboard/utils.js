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
    
    // BASIS POINTS REFACTOR: Convert basis points to display units using token decimals
    const tokenADisplayUnits = tokenARatio / Math.pow(10, tokenAPrecision);
    const tokenBDisplayUnits = tokenBRatio / Math.pow(10, tokenBPrecision);
    
    console.log('🔧 BASIS POINTS CONVERSION:', {
        tokenABasisPoints: tokenARatio,
        tokenADecimals: tokenAPrecision,
        tokenADisplay: tokenADisplayUnits,
        tokenBBasisPoints: tokenBRatio,
        tokenBDecimals: tokenBPrecision,
        tokenBDisplay: tokenBDisplayUnits
    });
    
    // CORRECT CALCULATION: Show what the pool actually represents
    // Based on swap formula: amount_out_B = amount_in_A * ratio_B_denominator / ratio_A_numerator
    // So: 1 TokenA gets you (ratio_B_denominator / ratio_A_numerator) TokenB
    // But now using display units instead of basis points
    
    const actualExchangeRate = tokenBDisplayUnits / tokenADisplayUnits;
    
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
        const inverseRate = tokenADisplayUnits / tokenBDisplayUnits;
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

// ========================================
// BASIS POINTS REFACTOR: CONVERSION UTILITIES
// ========================================

/**
 * **BASIS POINTS REFACTOR: Convert display units to basis points**
 * 
 * Converts user-friendly display amounts (like 1.0 SOL) to basis points
 * (smallest token units) that the smart contract expects. This is the core
 * conversion function that all pool creation and swap operations must use.
 * 
 * @param {number} displayAmount - Amount in display units (e.g., 1.5)
 * @param {number} decimals - Token decimal places (e.g., 9 for SOL)
 * @returns {number} Amount in basis points (e.g., 1500000000000000000 for 1.5 SOL)
 * 
 * @example
 * // Converting 1.5 USDC (6 decimals) to basis points
 * const basisPoints = displayToBasisPoints(1.5, 6); // Returns 1,500,000
 * 
 * // Converting 0.001 BTC (8 decimals) to basis points  
 * const basisPoints = displayToBasisPoints(0.001, 8); // Returns 100,000
 * 
 * // Converting 1.0 SOL (9 decimals) to basis points
 * const basisPoints = displayToBasisPoints(1.0, 9); // Returns 1,000,000,000
 */
function displayToBasisPoints(displayAmount, decimals) {
    if (typeof displayAmount !== 'number' || isNaN(displayAmount) || displayAmount < 0) {
        throw new Error(`Invalid display amount: ${displayAmount}. Must be a positive number.`);
    }
    
    if (typeof decimals !== 'number' || !Number.isInteger(decimals) || decimals < 0 || decimals > 9) {
        throw new Error(`Invalid decimals: ${decimals}. Must be an integer between 0 and 9.`);
    }
    
    const factor = Math.pow(10, decimals);
    const exactCalculation = displayAmount * factor;
    const basisPoints = Math.round(exactCalculation);
    
    // Debug log when rounding occurs (helps track precision fixes)
    if (Math.abs(exactCalculation - basisPoints) > 0.001) {
        console.log(`🔧 ROUNDING APPLIED: ${exactCalculation} → ${basisPoints} (diff: ${Math.abs(exactCalculation - basisPoints)})`);
    }
    
    console.log(`🔧 BASIS POINTS CONVERSION: ${displayAmount} (display) → ${basisPoints} (basis points) [${decimals} decimals]`);
    
    return basisPoints;
}

/**
 * **BASIS POINTS REFACTOR: Convert basis points to display units**
 * 
 * Converts basis points (smallest token units) from the smart contract back to
 * user-friendly display amounts. Used for showing swap results, pool liquidity,
 * and other user-facing amounts.
 * 
 * @param {number} basisPoints - Amount in basis points (e.g., 1500000000000000000)
 * @param {number} decimals - Token decimal places (e.g., 9 for SOL)
 * @returns {number} Amount in display units (e.g., 1.5)
 * 
 * @example
 * // Converting 1,500,000 basis points to USDC display units
 * const display = basisPointsToDisplay(1500000, 6); // Returns 1.5
 * 
 * // Converting 100,000 basis points to BTC display units
 * const display = basisPointsToDisplay(100000, 8); // Returns 0.001
 * 
 * // Converting 1,000,000,000 basis points to SOL display units
 * const display = basisPointsToDisplay(1000000000, 9); // Returns 1.0
 */
function basisPointsToDisplay(basisPoints, decimals) {
    if (typeof basisPoints !== 'number' || isNaN(basisPoints) || basisPoints < 0) {
        throw new Error(`Invalid basis points: ${basisPoints}. Must be a positive number.`);
    }
    
    if (typeof decimals !== 'number' || !Number.isInteger(decimals) || decimals < 0 || decimals > 9) {
        throw new Error(`Invalid decimals: ${decimals}. Must be an integer between 0 and 9.`);
    }
    
    const factor = Math.pow(10, decimals);
    
    // Use more precise calculation for better floating-point handling
    let displayAmount;
    if (decimals === 0) {
        // No conversion needed for 0 decimals
        displayAmount = basisPoints;
    } else {
        // Use BigInt for precise division, then convert to number
        displayAmount = Number(BigInt(basisPoints)) / factor;
        
        // Round to avoid floating-point precision issues (keep reasonable precision)
        const precision = Math.min(decimals, 6); // Max 6 decimal places for display
        displayAmount = Math.round(displayAmount * Math.pow(10, precision)) / Math.pow(10, precision);
    }
    
    console.log(`🔧 BASIS POINTS CONVERSION: ${basisPoints} (basis points) → ${displayAmount} (display) [${decimals} decimals]`);
    
    return displayAmount;
}

/**
 * **BASIS POINTS REFACTOR: Validate one-to-many ratio pattern**
 * 
 * Validates whether a ratio qualifies for the one-to-many flag by checking if:
 * 1. Both ratios represent whole numbers in display units
 * 2. One side equals exactly 1.0 in display units  
 * 3. Both sides are positive
 * 
 * This mirrors the smart contract's validation logic and should be used in the
 * dashboard to provide user feedback about flag setting.
 * 
 * @param {number} ratioADisplay - Token A amount in display units
 * @param {number} ratioBDisplay - Token B amount in display units  
 * @param {number} decimalsA - Token A decimal places
 * @param {number} decimalsB - Token B decimal places
 * @returns {boolean} True if the ratio qualifies for one-to-many flag
 * 
 * @example
 * // Valid one-to-many: 1 SOL = 160 USDT
 * const isOneToMany = validateOneToManyRatio(1.0, 160.0, 9, 6); // Returns true
 * 
 * // Invalid: 1.5 SOL = 240 USDT (first side not 1.0)
 * const isOneToMany = validateOneToManyRatio(1.5, 240.0, 9, 6); // Returns false
 * 
 * // Invalid: 1 SOL = 160.5 USDT (not whole number)  
 * const isOneToMany = validateOneToManyRatio(1.0, 160.5, 9, 6); // Returns false
 */
function validateOneToManyRatio(ratioADisplay, ratioBDisplay, decimalsA, decimalsB) {
    try {
        // Convert to basis points for validation
        const basisPointsA = displayToBasisPoints(ratioADisplay, decimalsA);
        const basisPointsB = displayToBasisPoints(ratioBDisplay, decimalsB);
        
        const factorA = Math.pow(10, decimalsA);
        const factorB = Math.pow(10, decimalsB);
        
        // Check if both ratios represent whole numbers in display units
        const aIsWhole = (basisPointsA % factorA) === 0;
        const bIsWhole = (basisPointsB % factorB) === 0;
        
        // Check if both are positive and one equals exactly 1.0
        const bothPositive = ratioADisplay > 0 && ratioBDisplay > 0;
        const oneEqualsOne = ratioADisplay === 1.0 || ratioBDisplay === 1.0;
        
        const result = aIsWhole && bIsWhole && bothPositive && oneEqualsOne;
        
        console.log(`🔍 ONE-TO-MANY VALIDATION:`, {
            ratioADisplay, ratioBDisplay, decimalsA, decimalsB,
            aIsWhole, bIsWhole, bothPositive, oneEqualsOne, result
        });
        
        return result;
        
    } catch (error) {
        console.error('❌ Error validating one-to-many ratio:', error);
        return false;
    }
}

/**
 * **BASIS POINTS REFACTOR: Calculate swap output in basis points**
 * 
 * Performs the core swap calculation using basis points arithmetic, exactly
 * matching the smart contract's logic. This ensures precision and accuracy.
 * 
 * @param {number} inputDisplay - Input amount in display units
 * @param {number} inputDecimals - Input token decimals
 * @param {number} outputDecimals - Output token decimals  
 * @param {number} numeratorBasisPoints - Pool ratio numerator (in basis points)
 * @param {number} denominatorBasisPoints - Pool ratio denominator (in basis points)
 * @returns {number} Output amount in display units
 * 
 * @example
 * // Pool: 1 SOL = 160 USDT (1×10^9 : 160×10^6 basis points)
 * // Swap: 0.5 SOL → ? USDT
 * const output = calculateSwapOutput(0.5, 9, 6, 160000000, 1000000000); // Returns 80.0
 */
function calculateSwapOutput(inputDisplay, inputDecimals, outputDecimals, numeratorBasisPoints, denominatorBasisPoints) {
    try {
        // Convert input to basis points
        const inputBasisPoints = displayToBasisPoints(inputDisplay, inputDecimals);
        
        // Perform calculation in basis points (matches smart contract)
        const outputBasisPoints = Math.floor((inputBasisPoints * numeratorBasisPoints) / denominatorBasisPoints);
        
        // Convert result back to display units
        const outputDisplay = basisPointsToDisplay(outputBasisPoints, outputDecimals);
        
        console.log(`🔄 SWAP CALCULATION:`, {
            input: `${inputDisplay} (${inputBasisPoints} basis points)`,
            output: `${outputDisplay} (${outputBasisPoints} basis points)`,
            ratio: `${numeratorBasisPoints} : ${denominatorBasisPoints}`
        });
        
        return outputDisplay;
        
    } catch (error) {
        console.error('❌ Error calculating swap output:', error);
        throw error;
    }
}

/**
 * CENTRALIZED POOL DISPLAY FUNCTION - Handles decimals and display logic
 * Use this function across all dashboard pages for consistent pool display
 */
async function enrichPoolWithCorrectDisplay(poolData, connection) {
    try {
        console.log('🔧 CENTRALIZING POOL DISPLAY: Starting enrichment...');
        
        // Skip if already enriched or missing required data
        if (!poolData || poolData._displayEnriched) {
            console.log('📋 Pool already enriched or invalid data, skipping...');
            return poolData;
        }
        
        // Fetch token decimals if not already available
        let tokenDecimals = null;
        if (poolData.ratioADecimal !== undefined && poolData.ratioBDecimal !== undefined) {
            // Use decimals from state.json if available
            tokenDecimals = {
                tokenADecimals: poolData.ratioADecimal,
                tokenBDecimals: poolData.ratioBDecimal
            };
            console.log(`✅ Using decimals from pool data: TS=${tokenDecimals.tokenADecimals}, MST=${tokenDecimals.tokenBDecimals}`);
        } else if (connection && (poolData.tokenAMint || poolData.token_a_mint) && (poolData.tokenBMint || poolData.token_b_mint)) {
            // Fetch from blockchain if not in pool data
            console.log('🔍 Fetching token decimals from blockchain...');
            const [tokenADecimals, tokenBDecimals] = await Promise.all([
                getTokenDecimals(poolData.tokenAMint || poolData.token_a_mint, connection),
                getTokenDecimals(poolData.tokenBMint || poolData.token_b_mint, connection)
            ]);
            
            tokenDecimals = { tokenADecimals, tokenBDecimals };
            console.log(`✅ Fetched decimals: ${poolData.tokenASymbol || 'TokenA'}=${tokenADecimals}, ${poolData.tokenBSymbol || 'TokenB'}=${tokenBDecimals}`);
        } else {
            console.warn('⚠️ Unable to fetch token decimals - using defaults');
            tokenDecimals = { tokenADecimals: 6, tokenBDecimals: 6 };
        }
        
        // Create corrected display using proper decimals
        const correctedDisplay = getCorrectTokenDisplay(
            poolData.tokenASymbol || 'Token A',
            poolData.tokenBSymbol || 'Token B',
            poolData.ratioANumerator || poolData.ratio_a_numerator || 1,
            poolData.ratioBDenominator || poolData.ratio_b_denominator || 1,
            tokenDecimals.tokenADecimals,
            tokenDecimals.tokenBDecimals
        );
        
        // Enrich pool data with display information
        const enrichedPool = {
            ...poolData,
            tokenDecimals: tokenDecimals,
            correctedDisplay: correctedDisplay,
            _displayEnriched: true  // Mark as enriched to avoid re-processing
        };
        
        console.log('🎯 CENTRALIZED ENRICHMENT COMPLETE:', {
            poolAddress: enrichedPool.address?.slice(0, 8) + '...',
            tokenPair: correctedDisplay.displayPair,
            exchangeRate: correctedDisplay.rateText,
            decimals: `${tokenDecimals.tokenADecimals}:${tokenDecimals.tokenBDecimals}`
        });
        
        return enrichedPool;
        
    } catch (error) {
        console.error('❌ Error enriching pool display:', error);
        // Return original pool data if enrichment fails
        return poolData;
    }
}

/**
 * 🎯 CENTRALIZED: Get consistent pair name across all pages
 * Always returns "TokenA/TokenB" format regardless of ratio
 * 
 * @param {Object} pool - Pool data
 * @returns {string} Consistent pair name (e.g., "TS/MST")
 */
function getCentralizedPairName(pool) {
    const tokenASymbol = pool.tokenASymbol || 'Token A';
    const tokenBSymbol = pool.tokenBSymbol || 'Token B';
    return `${tokenASymbol}/${tokenBSymbol}`;
}

/**
 * 🎯 CENTRALIZED: Get consistent ratio display across all pages
 * Always shows "1 TokenA = X TokenB" format
 * 
 * @param {Object} pool - Pool data
 * @returns {string} Consistent ratio text (e.g., "1 TS = 10,000 MST")
 */
function getCentralizedRatioText(pool) {
    // Get basis points values
    const ratioABasisPoints = pool.ratioANumerator || pool.ratio_a_numerator || 1;
    const ratioBBasisPoints = pool.ratioBDenominator || pool.ratio_b_denominator || 1;
    
    // Get token decimals
    const tokenADecimals = pool.ratioADecimal !== undefined ? pool.ratioADecimal : 6;
    const tokenBDecimals = pool.ratioBDecimal !== undefined ? pool.ratioBDecimal : 6;
    
    // Convert to display units
    const ratioADisplay = ratioABasisPoints / Math.pow(10, tokenADecimals);
    const ratioBDisplay = ratioBBasisPoints / Math.pow(10, tokenBDecimals);
    
    // Calculate exchange rate: 1 TokenA = X TokenB
    // For "1 TS = 1000 MST", we need: exchangeRate = 1000
    // So: exchangeRate = ratioBDisplay / ratioADisplay
    const exchangeRate = ratioBDisplay / ratioADisplay;
    
    const tokenASymbol = pool.tokenASymbol || 'Token A';
    const tokenBSymbol = pool.tokenBSymbol || 'Token B';
    
    return `1 ${tokenASymbol} = ${formatNumberWithCommas(exchangeRate)} ${tokenBSymbol}`;
}

/**
 * 🎯 CENTRALIZED: Get consistent ratio display for dashboard cards
 * Shows simplified ratio format (e.g., "1:10,000")
 * 
 * @param {Object} pool - Pool data
 * @returns {string} Simplified ratio text
 */
function getCentralizedRatioDisplay(pool) {
    // Get basis points values
    const ratioABasisPoints = pool.ratioANumerator || pool.ratio_a_numerator || 1;
    const ratioBBasisPoints = pool.ratioBDenominator || pool.ratio_b_denominator || 1;
    
    // Get token decimals
    const tokenADecimals = pool.ratioADecimal !== undefined ? pool.ratioADecimal : 6;
    const tokenBDecimals = pool.ratioBDecimal !== undefined ? pool.ratioBDecimal : 6;
    
    // Convert to display units
    const ratioADisplay = ratioABasisPoints / Math.pow(10, tokenADecimals);
    const ratioBDisplay = ratioBBasisPoints / Math.pow(10, tokenBDecimals);
    
    // Calculate exchange rate: 1 TokenA = X TokenB
    // For "1 TS = 1000 MST", we need: exchangeRate = 1000
    // So: exchangeRate = ratioBDisplay / ratioADisplay
    const exchangeRate = ratioBDisplay / ratioADisplay;
    
    return `1:${formatNumberWithCommas(exchangeRate)}`;
}

/**
 * 🎯 CENTRALIZED: Get complete display info for any page
 * Returns consistent object with all display information
 * 
 * @param {Object} pool - Pool data
 * @returns {Object} Complete display information
 */
function getCentralizedDisplayInfo(pool) {
    const pairName = getCentralizedPairName(pool);
    const ratioText = getCentralizedRatioText(pool);
    const ratioDisplay = getCentralizedRatioDisplay(pool);
    
    // Get basis points values for calculations
    const ratioABasisPoints = pool.ratioANumerator || pool.ratio_a_numerator || 1;
    const ratioBBasisPoints = pool.ratioBDenominator || pool.ratio_b_denominator || 1;
    const tokenADecimals = pool.ratioADecimal !== undefined ? pool.ratioADecimal : 6;
    const tokenBDecimals = pool.ratioBDecimal !== undefined ? pool.ratioBDecimal : 6;
    
    // Convert to display units
    const ratioADisplay = ratioABasisPoints / Math.pow(10, tokenADecimals);
    const ratioBDisplay = ratioBBasisPoints / Math.pow(10, tokenBDecimals);
    // Calculate exchange rate: 1 TokenA = X TokenB
    // For "1 TS = 1000 MST", we need: exchangeRate = 1000
    // So: exchangeRate = ratioBDisplay / ratioADisplay
    const exchangeRate = ratioBDisplay / ratioADisplay;
    
    return {
        pairName: pairName,
        ratioText: ratioText,
        ratioDisplay: ratioDisplay,
        exchangeRate: exchangeRate,
        tokenASymbol: pool.tokenASymbol || 'Token A',
        tokenBSymbol: pool.tokenBSymbol || 'Token B',
        ratioABasisPoints: ratioABasisPoints,
        ratioBBasisPoints: ratioBBasisPoints,
        tokenADecimals: tokenADecimals,
        tokenBDecimals: tokenBDecimals,
        ratioADisplay: ratioADisplay,
        ratioBDisplay: ratioBDisplay
    };
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
        interpretPoolFlags,
        // BASIS POINTS REFACTOR: New conversion functions
        displayToBasisPoints,
        basisPointsToDisplay,
        validateOneToManyRatio,
        calculateSwapOutput,
        // CENTRALIZED DISPLAY: New unified function
        enrichPoolWithCorrectDisplay,
        // CENTRALIZED: New centralized display functions
        getCentralizedPairName,
        getCentralizedRatioText,
        getCentralizedRatioDisplay,
        getCentralizedDisplayInfo
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
        interpretPoolFlags,
        // BASIS POINTS REFACTOR: New conversion functions
        displayToBasisPoints,
        basisPointsToDisplay,
        validateOneToManyRatio,
        calculateSwapOutput,
        // CENTRALIZED DISPLAY: New unified function
        enrichPoolWithCorrectDisplay,
        // CENTRALIZED: New centralized display functions
        getCentralizedPairName,
        getCentralizedRatioText,
        getCentralizedRatioDisplay,
        getCentralizedDisplayInfo
    };
} 