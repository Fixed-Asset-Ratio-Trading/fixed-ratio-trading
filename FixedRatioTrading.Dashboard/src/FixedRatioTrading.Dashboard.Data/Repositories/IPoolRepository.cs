using FixedRatioTrading.Dashboard.Core.Models;

namespace FixedRatioTrading.Dashboard.Data.Repositories;

/// <summary>
/// Repository interface for Pool entities with domain-specific operations
/// </summary>
public interface IPoolRepository : IRepository<Pool>
{
    // Pool-specific queries
    Task<Pool?> GetByPoolAddressAsync(string poolAddress);
    Task<Pool?> GetByTokenPairAsync(string tokenAMint, string tokenBMint);
    Task<IEnumerable<Pool>> GetByNetworkAsync(string network);
    Task<IEnumerable<Pool>> GetActivePoolsAsync(string? network = null);
    Task<IEnumerable<Pool>> GetPoolsByCreatorAsync(string creatorAddress);
    Task<IEnumerable<Pool>> GetPoolsWithTokenAsync(string tokenMint);
    
    // Pool statistics
    Task<int> GetActivePoolCountAsync(string? network = null);
    Task<decimal?> GetTotalValueLockedAsync(string? network = null);
    Task<(ulong TokenAVolume, ulong TokenBVolume)> GetPoolVolumeAsync(Guid poolId, TimeSpan? period = null);
    
    // Pool search and filtering
    Task<IEnumerable<Pool>> SearchPoolsAsync(string searchTerm, string? network = null);
    Task<IEnumerable<Pool>> GetTopPoolsByVolumeAsync(int count = 10, string? network = null);
    Task<IEnumerable<Pool>> GetRecentPoolsAsync(int count = 10, string? network = null);
    
    // Pool with related data
    Task<Pool?> GetPoolWithTransactionsAsync(Guid poolId, int? transactionLimit = null);
    Task<Pool?> GetPoolWithFullDataAsync(Guid poolId);
    
    // Bulk operations
    Task UpdatePoolLiquidityAsync(string poolAddress, ulong tokenALiquidity, ulong tokenBLiquidity);
    Task UpdatePoolStatsAsync(string poolAddress, ulong volumeA, ulong volumeB, int uniqueProviders);
    Task BulkUpdateLastUpdatedAsync(IEnumerable<string> poolAddresses);
} 