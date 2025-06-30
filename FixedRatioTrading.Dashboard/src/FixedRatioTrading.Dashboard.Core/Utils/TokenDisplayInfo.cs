using FixedRatioTrading.Dashboard.Core.Models;

namespace FixedRatioTrading.Dashboard.Core.Utils;

/// <summary>
/// Information about how to display a token pair based on the UX design pattern
/// </summary>
public class TokenDisplayInfo
{
    /// <summary>
    /// The base token symbol (always has ratio = 1)
    /// </summary>
    public string BaseToken { get; set; } = string.Empty;
    
    /// <summary>
    /// The quote token symbol (calculated ratio)
    /// </summary>
    public string QuoteToken { get; set; } = string.Empty;
    
    /// <summary>
    /// Base token name for display
    /// </summary>
    public string BaseTokenName { get; set; } = string.Empty;
    
    /// <summary>
    /// Quote token name for display
    /// </summary>
    public string QuoteTokenName { get; set; } = string.Empty;
    
    /// <summary>
    /// Liquidity amount for the base token
    /// </summary>
    public ulong BaseLiquidity { get; set; }
    
    /// <summary>
    /// Liquidity amount for the quote token
    /// </summary>
    public ulong QuoteLiquidity { get; set; }
    
    /// <summary>
    /// Exchange rate (how many quote tokens per 1 base token)
    /// </summary>
    public decimal ExchangeRate { get; set; }
    
    /// <summary>
    /// Display pair format (e.g., "BTC/USDC")
    /// </summary>
    public string DisplayPair { get; set; } = string.Empty;
    
    /// <summary>
    /// Human-readable rate text (e.g., "1 BTC = 50,000.00 USDC")
    /// </summary>
    public string RateText { get; set; } = string.Empty;
    
    /// <summary>
    /// Gets display information for a pool based on the UX design pattern.
    /// 
    /// CRITICAL UNDERSTANDING: The stored ratio RatioANumerator:RatioBDenominator means:
    /// "RatioANumerator of TokenA per RatioBDenominator of TokenB"
    /// 
    /// The logic determines which token should be the base token (ratio = 1) for user-friendly display.
    /// </summary>
    /// <param name="pool">The pool to get display information for</param>
    /// <returns>TokenDisplayInfo with proper base/quote ordering and exchange rates</returns>
    public static TokenDisplayInfo GetDisplayInfo(Pool pool)
    {
        // Calculate what the stored ratio means
        var tokensA_per_tokenB = (decimal)pool.RatioANumerator / pool.RatioBDenominator;
        var tokensB_per_tokenA = (decimal)pool.RatioBDenominator / pool.RatioANumerator;
        
        if (tokensA_per_tokenB >= 1.0m)
        {
            // Many TokenA per 1 TokenB → TokenB is more valuable → TokenB is base
            return new TokenDisplayInfo
            {
                BaseToken = pool.TokenBSymbol,
                QuoteToken = pool.TokenASymbol,
                BaseTokenName = pool.TokenBName,
                QuoteTokenName = pool.TokenAName,
                BaseLiquidity = pool.TokenBLiquidity,
                QuoteLiquidity = pool.TokenALiquidity,
                ExchangeRate = tokensA_per_tokenB,
                DisplayPair = $"{pool.TokenBSymbol}/{pool.TokenASymbol}",
                RateText = FormatRateText(pool.TokenBSymbol, tokensA_per_tokenB, pool.TokenASymbol)
            };
        }
        else
        {
            // Few TokenA per 1 TokenB → TokenA is more valuable → TokenA is base
            return new TokenDisplayInfo
            {
                BaseToken = pool.TokenASymbol,
                QuoteToken = pool.TokenBSymbol,
                BaseTokenName = pool.TokenAName,
                QuoteTokenName = pool.TokenBName,
                BaseLiquidity = pool.TokenALiquidity,
                QuoteLiquidity = pool.TokenBLiquidity,
                ExchangeRate = tokensB_per_tokenA,
                DisplayPair = $"{pool.TokenASymbol}/{pool.TokenBSymbol}",
                RateText = FormatRateText(pool.TokenASymbol, tokensB_per_tokenA, pool.TokenBSymbol)
            };
        }
    }
    
    /// <summary>
    /// Formats the exchange rate text with appropriate decimal places
    /// </summary>
    /// <param name="baseToken">The base token symbol</param>
    /// <param name="rate">The exchange rate</param>
    /// <param name="quoteToken">The quote token symbol</param>
    /// <returns>Formatted rate text like "1 BTC = 50,000.00 USDC"</returns>
    private static string FormatRateText(string baseToken, decimal rate, string quoteToken)
    {
        string formattedRate;
        
        if (rate >= 1000000)
        {
            // Use scientific notation for very large numbers
            formattedRate = rate.ToString("0.##e+0");
        }
        else if (rate >= 1000)
        {
            // Use comma separators for large numbers
            formattedRate = rate.ToString("#,##0.00");
        }
        else if (rate >= 1)
        {
            // Standard decimal format
            formattedRate = rate.ToString("0.00");
        }
        else if (rate >= 0.001m)
        {
            // More decimal places for small numbers
            formattedRate = rate.ToString("0.000000");
        }
        else
        {
            // Scientific notation for very small numbers
            formattedRate = rate.ToString("0.##e-0");
        }
        
        return $"1 {baseToken} = {formattedRate} {quoteToken}";
    }
    
    /// <summary>
    /// Gets the original pool ratios (as stored) for a given display arrangement
    /// This is useful when we need to work backwards from display to storage
    /// </summary>
    /// <param name="pool">The pool</param>
    /// <returns>Tuple of (ratioANumerator, ratioBDenominator) as stored</returns>
    public static (ulong ratioANumerator, ulong ratioBDenominator) GetStoredRatios(Pool pool)
    {
        return (pool.RatioANumerator, pool.RatioBDenominator);
    }
    
    /// <summary>
    /// Validates that the display logic is working correctly for a pool
    /// This can be used in tests or debugging
    /// </summary>
    /// <param name="pool">The pool to validate</param>
    /// <returns>True if the display logic produces consistent results</returns>
    public static bool ValidateDisplayLogic(Pool pool)
    {
        try
        {
            var displayInfo = GetDisplayInfo(pool);
            
            // Basic validation checks
            if (string.IsNullOrEmpty(displayInfo.BaseToken) || 
                string.IsNullOrEmpty(displayInfo.QuoteToken))
                return false;
                
            if (displayInfo.ExchangeRate <= 0)
                return false;
                
            if (string.IsNullOrEmpty(displayInfo.DisplayPair) || 
                string.IsNullOrEmpty(displayInfo.RateText))
                return false;
                
            // The display pair should contain both tokens
            if (!displayInfo.DisplayPair.Contains(displayInfo.BaseToken) ||
                !displayInfo.DisplayPair.Contains(displayInfo.QuoteToken))
                return false;
                
            return true;
        }
        catch
        {
            return false;
        }
    }
} 