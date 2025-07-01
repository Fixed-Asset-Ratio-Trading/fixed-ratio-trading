using Microsoft.Extensions.Logging;
using FixedRatioTrading.Dashboard.Core.Models;
using FixedRatioTrading.Dashboard.Data.Repositories;

namespace FixedRatioTrading.Dashboard.Solana.Services;

/// <summary>
/// Implementation of pool synchronization service
/// </summary>
public class PoolSyncService : IPoolSyncService
{
    private readonly ISolanaRpcService _solanaRpc;
    private readonly IPoolRepository _poolRepository;
    private readonly IPoolTransactionRepository _transactionRepository;
    private readonly ISystemStateRepository _systemStateRepository;
    private readonly ILogger<PoolSyncService> _logger;

    public PoolSyncService(
        ISolanaRpcService solanaRpc,
        IPoolRepository poolRepository,
        IPoolTransactionRepository transactionRepository,
        ISystemStateRepository systemStateRepository,
        ILogger<PoolSyncService> logger)
    {
        _solanaRpc = solanaRpc;
        _poolRepository = poolRepository;
        _transactionRepository = transactionRepository;
        _systemStateRepository = systemStateRepository;
        _logger = logger;
    }

    public async Task<Pool?> SyncPoolAsync(string poolAddress)
    {
        try
        {
            _logger.LogInformation("Syncing pool: {PoolAddress}", poolAddress);

            // Get pool state from blockchain
            var poolStateData = await _solanaRpc.GetPoolStateAsync(poolAddress);
            if (poolStateData == null)
            {
                _logger.LogWarning("Pool not found on blockchain: {PoolAddress}", poolAddress);
                return null;
            }

            // Get existing pool from database or create new one
            var existingPool = await _poolRepository.GetByPoolAddressAsync(poolAddress);
            
            Pool pool;
            if (existingPool != null)
            {
                // Update existing pool
                pool = UpdatePoolFromBlockchainData(existingPool, poolStateData);
                _poolRepository.Update(pool);
                await _poolRepository.SaveChangesAsync();
                _logger.LogInformation("Updated existing pool: {PoolAddress}", poolAddress);
            }
            else
            {
                // Create new pool
                pool = CreatePoolFromBlockchainData(poolStateData, poolAddress);
                await _poolRepository.AddAsync(pool);
                _logger.LogInformation("Created new pool: {PoolAddress}", poolAddress);
            }

            return pool;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to sync pool: {PoolAddress}", poolAddress);
            return null;
        }
    }

    public async Task<IEnumerable<Pool>> SyncPoolsAsync(IEnumerable<string> poolAddresses)
    {
        var syncTasks = poolAddresses.Select(SyncPoolAsync);
        var results = await Task.WhenAll(syncTasks);
        return results.Where(pool => pool != null)!;
    }

    public async Task<int> SyncAllPoolsAsync(string? network = null)
    {
        try
        {
            network ??= _solanaRpc.Network;
            _logger.LogInformation("Syncing all pools for network: {Network}", network);

            var pools = await _poolRepository.GetByNetworkAsync(network);
            var poolAddresses = pools.Select(p => p.PoolAddress);

            var syncedPools = await SyncPoolsAsync(poolAddresses);
            var count = syncedPools.Count();

            _logger.LogInformation("Synced {Count} pools for network: {Network}", count, network);
            return count;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to sync all pools for network: {Network}", network);
            return 0;
        }
    }

    public async Task<SystemState?> SyncSystemStateAsync(string systemStateAddress)
    {
        try
        {
            _logger.LogInformation("Syncing system state: {SystemStateAddress}", systemStateAddress);

            var systemStateData = await _solanaRpc.GetSystemStateAsync(systemStateAddress);
            if (systemStateData == null)
            {
                _logger.LogWarning("System state not found on blockchain: {SystemStateAddress}", systemStateAddress);
                return null;
            }

            // Get existing system state or create new one
            var existing = await _systemStateRepository.GetByNetworkAsync(_solanaRpc.Network);
            
            SystemState systemState;
            if (existing != null)
            {
                // Update existing
                systemState = UpdateSystemStateFromBlockchainData(existing, systemStateData);
                _systemStateRepository.Update(systemState);
                await _systemStateRepository.SaveChangesAsync();
            }
            else
            {
                // Create new
                systemState = CreateSystemStateFromBlockchainData(systemStateData);
                await _systemStateRepository.AddAsync(systemState);
            }

            _logger.LogInformation("Synced system state: {SystemStateAddress}", systemStateAddress);
            return systemState;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to sync system state: {SystemStateAddress}", systemStateAddress);
            return null;
        }
    }

    public async Task<int> DiscoverNewPoolsAsync(ulong fromSlot, ulong? toSlot = null)
    {
        // TODO: Implement pool discovery by scanning blockchain logs
        _logger.LogInformation("Pool discovery not yet implemented");
        return 0;
    }

    public async Task<int> SyncPoolTransactionsAsync(string poolAddress, int limit = 50)
    {
        // TODO: Implement transaction synchronization
        _logger.LogInformation("Transaction sync not yet implemented for pool: {PoolAddress}", poolAddress);
        return 0;
    }

    public async Task<int> SyncAllPoolTransactionsAsync(int limit = 50)
    {
        // TODO: Implement bulk transaction synchronization
        _logger.LogInformation("Bulk transaction sync not yet implemented");
        return 0;
    }

    public async Task<ulong> GetLastSyncedSlotAsync(string network)
    {
        var systemState = await _systemStateRepository.GetByNetworkAsync(network);
        return systemState?.LastSyncSlot ?? 0;
    }

    public async Task UpdateLastSyncedSlotAsync(string network, ulong slot)
    {
        var systemState = await _systemStateRepository.GetByNetworkAsync(network);
        if (systemState != null)
        {
            systemState.LastSyncSlot = slot;
            systemState.LastSyncAt = DateTime.UtcNow;
            _systemStateRepository.Update(systemState);
            await _systemStateRepository.SaveChangesAsync();
        }
    }

    private Pool UpdatePoolFromBlockchainData(Pool existingPool, PoolStateData poolStateData)
    {
        // Update pool with blockchain data
        existingPool.Owner = poolStateData.Owner;
        existingPool.TokenAVault = poolStateData.TokenAVault;
        existingPool.TokenBVault = poolStateData.TokenBVault;
        existingPool.LpTokenAMint = poolStateData.LpTokenAMint;
        existingPool.LpTokenBMint = poolStateData.LpTokenBMint;
                    existingPool.Ratio = poolStateData.Ratio;
        existingPool.TokenAIsTheMultiple = poolStateData.TokenAIsTheMultiple;
        existingPool.TotalTokenALiquidity = poolStateData.TotalTokenALiquidity;
        existingPool.TotalTokenBLiquidity = poolStateData.TotalTokenBLiquidity;
        existingPool.PoolAuthorityBumpSeed = poolStateData.PoolAuthorityBumpSeed;
        existingPool.TokenAVaultBumpSeed = poolStateData.TokenAVaultBumpSeed;
        existingPool.TokenBVaultBumpSeed = poolStateData.TokenBVaultBumpSeed;
        existingPool.IsInitialized = poolStateData.IsInitialized;
        existingPool.IsPaused = poolStateData.IsPaused;
        existingPool.SwapsPaused = poolStateData.SwapsPaused;
        existingPool.SwapsPauseInitiatedBy = poolStateData.SwapsPauseInitiatedBy;
        existingPool.SwapsPauseInitiatedTimestamp = poolStateData.SwapsPauseInitiatedTimestamp;
        existingPool.WithdrawalProtectionActive = poolStateData.WithdrawalProtectionActive;
        existingPool.CollectedFeesTokenA = poolStateData.CollectedFeesTokenA;
        existingPool.CollectedFeesTokenB = poolStateData.CollectedFeesTokenB;
        existingPool.TotalFeesWithdrawnTokenA = poolStateData.TotalFeesWithdrawnTokenA;
        existingPool.TotalFeesWithdrawnTokenB = poolStateData.TotalFeesWithdrawnTokenB;
        existingPool.SwapFeeBasisPoints = poolStateData.SwapFeeBasisPoints;
        existingPool.CollectedSolFees = poolStateData.CollectedSolFees;
        existingPool.TotalSolFeesWithdrawn = poolStateData.TotalSolFeesWithdrawn;
        existingPool.LastUpdated = DateTime.UtcNow;

        return existingPool;
    }

    private Pool CreatePoolFromBlockchainData(PoolStateData poolStateData, string poolAddress)
    {
        return new Pool
        {
            PoolAddress = poolAddress,
            Owner = poolStateData.Owner,
            TokenAMint = poolStateData.TokenAMint,
            TokenBMint = poolStateData.TokenBMint,
            TokenAVault = poolStateData.TokenAVault,
            TokenBVault = poolStateData.TokenBVault,
            LpTokenAMint = poolStateData.LpTokenAMint,
            LpTokenBMint = poolStateData.LpTokenBMint,
            Ratio = poolStateData.Ratio,
            TokenAIsTheMultiple = poolStateData.TokenAIsTheMultiple,
            TotalTokenALiquidity = poolStateData.TotalTokenALiquidity,
            TotalTokenBLiquidity = poolStateData.TotalTokenBLiquidity,
            PoolAuthorityBumpSeed = poolStateData.PoolAuthorityBumpSeed,
            TokenAVaultBumpSeed = poolStateData.TokenAVaultBumpSeed,
            TokenBVaultBumpSeed = poolStateData.TokenBVaultBumpSeed,
            IsInitialized = poolStateData.IsInitialized,
            IsPaused = poolStateData.IsPaused,
            SwapsPaused = poolStateData.SwapsPaused,
            SwapsPauseInitiatedBy = poolStateData.SwapsPauseInitiatedBy,
            SwapsPauseInitiatedTimestamp = poolStateData.SwapsPauseInitiatedTimestamp,
            WithdrawalProtectionActive = poolStateData.WithdrawalProtectionActive,
            CollectedFeesTokenA = poolStateData.CollectedFeesTokenA,
            CollectedFeesTokenB = poolStateData.CollectedFeesTokenB,
            TotalFeesWithdrawnTokenA = poolStateData.TotalFeesWithdrawnTokenA,
            TotalFeesWithdrawnTokenB = poolStateData.TotalFeesWithdrawnTokenB,
            SwapFeeBasisPoints = poolStateData.SwapFeeBasisPoints,
            CollectedSolFees = poolStateData.CollectedSolFees,
            TotalSolFeesWithdrawn = poolStateData.TotalSolFeesWithdrawn,
            Network = _solanaRpc.Network,
            CreatedAt = DateTime.UtcNow,
            LastUpdated = DateTime.UtcNow,
            IsActive = true,
            // TODO: Get token symbols and names from token metadata
            TokenASymbol = "UNKNOWN",
            TokenBSymbol = "UNKNOWN",
        };
    }

    private SystemState UpdateSystemStateFromBlockchainData(SystemState existing, SystemStateData data)
    {
        existing.Authority = data.Authority;
        existing.IsPaused = data.IsPaused;
        existing.PauseTimestamp = data.PauseTimestamp;
        existing.PauseReason = data.PauseReason;
        existing.LastSyncAt = DateTime.UtcNow;
        existing.UpdatedAt = DateTime.UtcNow;

        return existing;
    }

    private SystemState CreateSystemStateFromBlockchainData(SystemStateData data)
    {
        return new SystemState
        {
            Network = _solanaRpc.Network,
            Authority = data.Authority,
            IsPaused = data.IsPaused,
            PauseTimestamp = data.PauseTimestamp,
            PauseReason = data.PauseReason,
            LastSyncAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow,
        };
    }
} 