using FixedRatioTrading.Dashboard.Core.Models;

namespace FixedRatioTrading.Dashboard.Web.Services;

/// <summary>
/// Pool operational status combining all pause/active states
/// </summary>
public enum PoolStatus
{
    /// <summary>
    /// Pool is fully operational - all operations allowed
    /// </summary>
    Operational = 1,
    
    /// <summary>
    /// Pool is inactive in database (deprecated/failed)
    /// </summary>
    Inactive = 2,
    
    /// <summary>
    /// Entire system is paused - no operations allowed anywhere
    /// </summary>
    SystemPaused = 3,
    
    /// <summary>
    /// This specific pool is paused by owner - no operations allowed
    /// </summary>
    PoolPaused = 4,
    
    /// <summary>
    /// Only swaps are paused - liquidity operations still allowed
    /// </summary>
    SwapsPaused = 5
}

/// <summary>
/// Business service interface for pool operations
/// Provides high-level business logic for pool management
/// </summary>
public interface IPoolService
{
    /// <summary>
    /// Get all pools with optional filtering
    /// Results are sorted by creation date (newest to oldest)
    /// </summary>
    /// <param name="network">Network filter (optional)</param>
    /// <param name="isActive">Active status filter (optional)</param>
    /// <param name="page">Page number for pagination</param>
    /// <param name="pageSize">Page size for pagination</param>
    /// <returns>Paginated pool results sorted newest to oldest</returns>
    Task<PoolListResult> GetPoolsAsync(string? network = null, bool? isActive = null, int page = 1, int pageSize = 20);
    
    /// <summary>
    /// Get a specific pool by ID
    /// </summary>
    /// <param name="id">Pool ID</param>
    /// <returns>Pool details or null if not found</returns>
    Task<PoolDetailsResult?> GetPoolAsync(Guid id);
    
    /// <summary>
    /// Get a specific pool by address
    /// </summary>
    /// <param name="poolAddress">Pool blockchain address</param>
    /// <returns>Pool details or null if not found</returns>
    Task<PoolDetailsResult?> GetPoolByAddressAsync(string poolAddress);
    
    /// <summary>
    /// Search pools by token symbols or names
    /// Results are sorted by creation date (newest to oldest)
    /// </summary>
    /// <param name="searchTerm">Search term</param>
    /// <param name="network">Network filter (optional)</param>
    /// <param name="page">Page number</param>
    /// <param name="pageSize">Page size</param>
    /// <returns>Search results sorted newest to oldest</returns>
    Task<PoolListResult> SearchPoolsAsync(string searchTerm, string? network = null, int page = 1, int pageSize = 20);
    
    /// <summary>
    /// Get pool statistics
    /// </summary>
    /// <param name="network">Network filter (optional)</param>
    /// <returns>Pool statistics</returns>
    Task<PoolStatistics> GetPoolStatisticsAsync(string? network = null);
    
    /// <summary>
    /// Get recent transactions for a pool
    /// </summary>
    /// <param name="poolId">Pool ID</param>
    /// <param name="limit">Maximum number of transactions</param>
    /// <returns>Recent transactions</returns>
    Task<IEnumerable<PoolTransactionResult>> GetPoolTransactionsAsync(Guid poolId, int limit = 50);
    
    /// <summary>
    /// Sync a pool from blockchain (manual refresh)
    /// </summary>
    /// <param name="poolAddress">Pool address to sync</param>
    /// <returns>Sync result</returns>
    Task<SyncResult> SyncPoolAsync(string poolAddress);
    
    /// <summary>
    /// Get top pools by volume or liquidity
    /// </summary>
    /// <param name="sortBy">Sort criteria (volume, liquidity, etc.)</param>
    /// <param name="count">Number of pools to return</param>
    /// <param name="network">Network filter (optional)</param>
    /// <returns>Top pools</returns>
    Task<IEnumerable<PoolSummaryResult>> GetTopPoolsAsync(string sortBy = "volume", int count = 10, string? network = null);
}

/// <summary>
/// Result model for pool list operations
/// </summary>
public class PoolListResult
{
    public IEnumerable<PoolSummaryResult> Pools { get; set; } = Enumerable.Empty<PoolSummaryResult>();
    public int TotalCount { get; set; }
    public int Page { get; set; }
    public int PageSize { get; set; }
    public int TotalPages => (int)Math.Ceiling((double)TotalCount / PageSize);
}

/// <summary>
/// Summary model for pool list items
/// </summary>
public class PoolSummaryResult
{
    public Guid Id { get; set; }
    public string PoolAddress { get; set; } = string.Empty;
    public string TokenASymbol { get; set; } = string.Empty;
    public string TokenBSymbol { get; set; } = string.Empty;
    public string TokenAName { get; set; } = string.Empty;
    public string TokenBName { get; set; } = string.Empty;
    public string Ratio => $"{RatioANumerator}:{RatioBDenominator}";
    public ulong RatioANumerator { get; set; }
    public ulong RatioBDenominator { get; set; }
    public ulong TotalTokenALiquidity { get; set; }
    public ulong TotalTokenBLiquidity { get; set; }
    public ulong TotalVolumeTokenA { get; set; }
    public ulong TotalVolumeTokenB { get; set; }
    
    // Simplified status that combines all pause/active states
    public PoolStatus Status { get; set; }
    public string StatusDescription { get; set; } = string.Empty;
    
    public DateTime CreatedAt { get; set; }
    public DateTime LastUpdated { get; set; }
    public string Network { get; set; } = string.Empty;
}

/// <summary>
/// Detailed model for individual pool view
/// </summary>
public class PoolDetailsResult : PoolSummaryResult
{
    public string Owner { get; set; } = string.Empty;
    public string TokenAMint { get; set; } = string.Empty;
    public string TokenBMint { get; set; } = string.Empty;
    public string TokenAVault { get; set; } = string.Empty;
    public string TokenBVault { get; set; } = string.Empty;
    public string LpTokenAMint { get; set; } = string.Empty;
    public string LpTokenBMint { get; set; } = string.Empty;
    public bool IsInitialized { get; set; }
    public bool WithdrawalProtectionActive { get; set; }
    public ulong CollectedFeesTokenA { get; set; }
    public ulong CollectedFeesTokenB { get; set; }
    public ulong SwapFeeBasisPoints { get; set; }
    public ulong CollectedSolFees { get; set; }
    public int UniqueLiquidityProviders { get; set; }
    public ulong CreationBlockNumber { get; set; }
    public string CreationTxSignature { get; set; } = string.Empty;
    public IEnumerable<PoolTransactionResult> RecentTransactions { get; set; } = Enumerable.Empty<PoolTransactionResult>();
}

/// <summary>
/// Model for pool transaction results
/// </summary>
public class PoolTransactionResult
{
    public Guid Id { get; set; }
    public TransactionType Type { get; set; }
    public string TypeDisplay => Type.ToString();
    public string TransactionSignature { get; set; } = string.Empty;
    public string UserAddress { get; set; } = string.Empty;
    public ulong TokenAAmount { get; set; }
    public ulong TokenBAmount { get; set; }
    public ulong LpTokenAmount { get; set; }
    public DateTime ProcessedAt { get; set; }
    public bool IsSuccessful { get; set; }
    public string? ErrorMessage { get; set; }
    public ulong GasFee { get; set; }
    public decimal? SwapPrice { get; set; }
    public string Description { get; set; } = string.Empty;
}

/// <summary>
/// Pool statistics model
/// </summary>
public class PoolStatistics
{
    public int TotalPools { get; set; }
    public int ActivePools { get; set; }
    public int PausedPools { get; set; }
    public ulong TotalValueLocked { get; set; }
    public ulong Volume24h { get; set; }
    public int UniqueUsers24h { get; set; }
    public int TotalTransactions { get; set; }
    public DateTime LastUpdated { get; set; }
}

/// <summary>
/// Sync operation result
/// </summary>
public class SyncResult
{
    public bool Success { get; set; }
    public string? ErrorMessage { get; set; }
    public DateTime SyncedAt { get; set; }
    public PoolDetailsResult? Pool { get; set; }
} 