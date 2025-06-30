using FixedRatioTrading.Dashboard.Core.Models;
using FixedRatioTrading.Dashboard.Data.Repositories;
using FixedRatioTrading.Dashboard.Solana.Services;

namespace FixedRatioTrading.Dashboard.Web.Services;

/// <summary>
/// Implementation of pool business service
/// </summary>
public class PoolService : IPoolService
{
    private readonly IPoolRepository _poolRepository;
    private readonly IPoolTransactionRepository _transactionRepository;
    private readonly IPoolSyncService _poolSyncService;
    private readonly ILogger<PoolService> _logger;

    public PoolService(
        IPoolRepository poolRepository,
        IPoolTransactionRepository transactionRepository,
        IPoolSyncService poolSyncService,
        ILogger<PoolService> logger)
    {
        _poolRepository = poolRepository;
        _transactionRepository = transactionRepository;
        _poolSyncService = poolSyncService;
        _logger = logger;
    }

    public async Task<PoolListResult> GetPoolsAsync(string? network = null, bool? isActive = null, int page = 1, int pageSize = 20)
    {
        try
        {
            _logger.LogDebug("Getting pools - Network: {Network}, Active: {IsActive}, Page: {Page}", 
                network, isActive, page);

            IEnumerable<Pool> pools;

            if (!string.IsNullOrEmpty(network))
            {
                pools = await _poolRepository.GetByNetworkAsync(network);
            }
            else
            {
                pools = await _poolRepository.GetAllAsync();
            }

            if (isActive.HasValue)
            {
                pools = pools.Where(p => p.IsActive == isActive.Value);
            }

            var totalCount = pools.Count();
            var paginatedPools = pools
                .OrderByDescending(p => p.CreatedAt)
                .Skip((page - 1) * pageSize)
                .Take(pageSize)
                .Select(MapToSummary);

            return new PoolListResult
            {
                Pools = paginatedPools,
                TotalCount = totalCount,
                Page = page,
                PageSize = pageSize
            };
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error getting pools");
            return new PoolListResult();
        }
    }

    public async Task<PoolDetailsResult?> GetPoolAsync(Guid id)
    {
        try
        {
            var pool = await _poolRepository.GetPoolWithTransactionsAsync(id, 10);
            return pool != null ? await MapToDetailsAsync(pool) : null;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error getting pool {PoolId}", id);
            return null;
        }
    }

    public async Task<PoolDetailsResult?> GetPoolByAddressAsync(string poolAddress)
    {
        try
        {
            var pool = await _poolRepository.GetByPoolAddressAsync(poolAddress);
            if (pool == null) return null;

            return await MapToDetailsAsync(pool);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error getting pool by address {PoolAddress}", poolAddress);
            return null;
        }
    }

    public async Task<PoolListResult> SearchPoolsAsync(string searchTerm, string? network = null, int page = 1, int pageSize = 20)
    {
        try
        {
            var pools = await _poolRepository.SearchPoolsAsync(searchTerm, network);
            var totalCount = pools.Count();
            
            var paginatedPools = pools
                .Skip((page - 1) * pageSize)
                .Take(pageSize)
                .Select(MapToSummary);

            return new PoolListResult
            {
                Pools = paginatedPools,
                TotalCount = totalCount,
                Page = page,
                PageSize = pageSize
            };
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error searching pools with term {SearchTerm}", searchTerm);
            return new PoolListResult();
        }
    }

    public async Task<PoolStatistics> GetPoolStatisticsAsync(string? network = null)
    {
        try
        {
            var totalPools = await _poolRepository.GetActivePoolCountAsync(network);
            var allPools = string.IsNullOrEmpty(network) 
                ? await _poolRepository.GetAllAsync()
                : await _poolRepository.GetByNetworkAsync(network);
            
            var activePools = allPools.Count(p => p.IsActive);
            var pausedPools = allPools.Count(p => p.IsPaused || p.SwapsPaused);

            // Calculate total value locked (sum of all pool liquidity)
            var totalValueLocked = allPools.Sum(p => p.TotalTokenALiquidity + p.TotalTokenBLiquidity);

            // Get 24h statistics
            var yesterday = DateTime.UtcNow.AddDays(-1);
            var transactions24h = await _transactionRepository.GetTransactionsInLastPeriodAsync(TimeSpan.FromDays(1));
            var volume24h = transactions24h
                .Where(t => t.Type == TransactionType.Swap)
                .Sum(t => t.TokenAAmount + t.TokenBAmount);
            
            var uniqueUsers24h = transactions24h
                .Select(t => t.UserAddress)
                .Distinct()
                .Count();

            var totalTransactions = await _transactionRepository.GetAllAsync();

            return new PoolStatistics
            {
                TotalPools = allPools.Count(),
                ActivePools = activePools,
                PausedPools = pausedPools,
                TotalValueLocked = totalValueLocked,
                Volume24h = volume24h,
                UniqueUsers24h = uniqueUsers24h,
                TotalTransactions = totalTransactions.Count(),
                LastUpdated = DateTime.UtcNow
            };
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error getting pool statistics");
            return new PoolStatistics { LastUpdated = DateTime.UtcNow };
        }
    }

    public async Task<IEnumerable<PoolTransactionResult>> GetPoolTransactionsAsync(Guid poolId, int limit = 50)
    {
        try
        {
            var transactions = await _transactionRepository.GetByPoolIdAsync(poolId, limit);
            return transactions.Select(MapTransactionToResult);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error getting transactions for pool {PoolId}", poolId);
            return Enumerable.Empty<PoolTransactionResult>();
        }
    }

    public async Task<SyncResult> SyncPoolAsync(string poolAddress)
    {
        try
        {
            _logger.LogInformation("Syncing pool {PoolAddress}", poolAddress);
            
            var syncedPool = await _poolSyncService.SyncPoolAsync(poolAddress);
            
            if (syncedPool != null)
            {
                var details = await MapToDetailsAsync(syncedPool);
                return new SyncResult
                {
                    Success = true,
                    SyncedAt = DateTime.UtcNow,
                    Pool = details
                };
            }
            else
            {
                return new SyncResult
                {
                    Success = false,
                    ErrorMessage = "Pool not found on blockchain or sync failed",
                    SyncedAt = DateTime.UtcNow
                };
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error syncing pool {PoolAddress}", poolAddress);
            return new SyncResult
            {
                Success = false,
                ErrorMessage = ex.Message,
                SyncedAt = DateTime.UtcNow
            };
        }
    }

    public async Task<IEnumerable<PoolSummaryResult>> GetTopPoolsAsync(string sortBy = "volume", int count = 10, string? network = null)
    {
        try
        {
            IEnumerable<Pool> pools;

            switch (sortBy.ToLowerInvariant())
            {
                case "volume":
                    pools = await _poolRepository.GetTopPoolsByVolumeAsync(count, network);
                    break;
                case "liquidity":
                    pools = string.IsNullOrEmpty(network)
                        ? await _poolRepository.GetAllAsync()
                        : await _poolRepository.GetByNetworkAsync(network);
                    pools = pools
                        .OrderByDescending(p => p.TotalTokenALiquidity + p.TotalTokenBLiquidity)
                        .Take(count);
                    break;
                case "recent":
                    pools = await _poolRepository.GetRecentPoolsAsync(count, network);
                    break;
                default:
                    pools = await _poolRepository.GetTopPoolsByVolumeAsync(count, network);
                    break;
            }

            return pools.Select(MapToSummary);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error getting top pools sorted by {SortBy}", sortBy);
            return Enumerable.Empty<PoolSummaryResult>();
        }
    }

    private PoolSummaryResult MapToSummary(Pool pool)
    {
        return new PoolSummaryResult
        {
            Id = pool.Id,
            PoolAddress = pool.PoolAddress,
            TokenASymbol = pool.TokenASymbol,
            TokenBSymbol = pool.TokenBSymbol,
            TokenAName = pool.TokenAName,
            TokenBName = pool.TokenBName,
            RatioANumerator = pool.RatioANumerator,
            RatioBDenominator = pool.RatioBDenominator,
            TotalTokenALiquidity = pool.TotalTokenALiquidity,
            TotalTokenBLiquidity = pool.TotalTokenBLiquidity,
            TotalVolumeTokenA = pool.TotalVolumeTokenA,
            TotalVolumeTokenB = pool.TotalVolumeTokenB,
            IsActive = pool.IsActive,
            IsPaused = pool.IsPaused,
            SwapsPaused = pool.SwapsPaused,
            CreatedAt = pool.CreatedAt,
            LastUpdated = pool.LastUpdated,
            Network = pool.Network
        };
    }

    private async Task<PoolDetailsResult> MapToDetailsAsync(Pool pool)
    {
        // Get recent transactions for this pool
        var recentTransactions = await _transactionRepository.GetByPoolIdAsync(pool.Id, 10);
        
        return new PoolDetailsResult
        {
            Id = pool.Id,
            PoolAddress = pool.PoolAddress,
            Owner = pool.Owner,
            TokenASymbol = pool.TokenASymbol,
            TokenBSymbol = pool.TokenBSymbol,
            TokenAName = pool.TokenAName,
            TokenBName = pool.TokenBName,
            TokenAMint = pool.TokenAMint,
            TokenBMint = pool.TokenBMint,
            TokenAVault = pool.TokenAVault,
            TokenBVault = pool.TokenBVault,
            LpTokenAMint = pool.LpTokenAMint,
            LpTokenBMint = pool.LpTokenBMint,
            RatioANumerator = pool.RatioANumerator,
            RatioBDenominator = pool.RatioBDenominator,
            TotalTokenALiquidity = pool.TotalTokenALiquidity,
            TotalTokenBLiquidity = pool.TotalTokenBLiquidity,
            TotalVolumeTokenA = pool.TotalVolumeTokenA,
            TotalVolumeTokenB = pool.TotalVolumeTokenB,
            IsActive = pool.IsActive,
            IsInitialized = pool.IsInitialized,
            IsPaused = pool.IsPaused,
            SwapsPaused = pool.SwapsPaused,
            WithdrawalProtectionActive = pool.WithdrawalProtectionActive,
            CollectedFeesTokenA = pool.CollectedFeesTokenA,
            CollectedFeesTokenB = pool.CollectedFeesTokenB,
            SwapFeeBasisPoints = pool.SwapFeeBasisPoints,
            CollectedSolFees = pool.CollectedSolFees,
            UniqueLiquidityProviders = pool.UniqueLiquidityProviders,
            CreationBlockNumber = pool.CreationBlockNumber,
            CreationTxSignature = pool.CreationTxSignature,
            CreatedAt = pool.CreatedAt,
            LastUpdated = pool.LastUpdated,
            Network = pool.Network,
            RecentTransactions = recentTransactions.Select(MapTransactionToResult)
        };
    }

    private PoolTransactionResult MapTransactionToResult(PoolTransaction transaction)
    {
        return new PoolTransactionResult
        {
            Id = transaction.Id,
            Type = transaction.Type,
            TransactionSignature = transaction.TransactionSignature,
            UserAddress = transaction.UserAddress,
            TokenAAmount = transaction.TokenAAmount,
            TokenBAmount = transaction.TokenBAmount,
            LpTokenAmount = transaction.LpTokenAmount,
            ProcessedAt = transaction.ProcessedAt,
            IsSuccessful = transaction.IsSuccessful,
            ErrorMessage = transaction.ErrorMessage,
            GasFee = transaction.GasFee,
            SwapPrice = transaction.SwapPrice,
            Description = transaction.Description
        };
    }
} 