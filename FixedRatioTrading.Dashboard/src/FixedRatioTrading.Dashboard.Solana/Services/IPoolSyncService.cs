using FixedRatioTrading.Dashboard.Core.Models;

namespace FixedRatioTrading.Dashboard.Solana.Services;

/// <summary>
/// Service interface for synchronizing pool data between blockchain and database
/// </summary>
public interface IPoolSyncService
{
    /// <summary>
    /// Synchronize a single pool's data from blockchain
    /// </summary>
    /// <param name="poolAddress">Pool account address</param>
    /// <returns>Synchronized pool model or null if not found</returns>
    Task<Pool?> SyncPoolAsync(string poolAddress);
    
    /// <summary>
    /// Synchronize multiple pools in parallel
    /// </summary>
    /// <param name="poolAddresses">List of pool addresses</param>
    /// <returns>List of synchronized pools</returns>
    Task<IEnumerable<Pool>> SyncPoolsAsync(IEnumerable<string> poolAddresses);
    
    /// <summary>
    /// Synchronize all pools in the database
    /// </summary>
    /// <param name="network">Network to sync (optional, defaults to current network)</param>
    /// <returns>Number of pools synchronized</returns>
    Task<int> SyncAllPoolsAsync(string? network = null);
    
    /// <summary>
    /// Synchronize system state from blockchain
    /// </summary>
    /// <param name="systemStateAddress">System state account address</param>
    /// <returns>Synchronized system state or null if not found</returns>
    Task<SystemState?> SyncSystemStateAsync(string systemStateAddress);
    
    /// <summary>
    /// Discover and sync new pools created on-chain
    /// </summary>
    /// <param name="fromSlot">Start scanning from this slot</param>
    /// <param name="toSlot">End scanning at this slot (optional)</param>
    /// <returns>Number of new pools discovered and synced</returns>
    Task<int> DiscoverNewPoolsAsync(ulong fromSlot, ulong? toSlot = null);
    
    /// <summary>
    /// Sync recent transactions for a pool
    /// </summary>
    /// <param name="poolAddress">Pool account address</param>
    /// <param name="limit">Maximum number of transactions to sync</param>
    /// <returns>Number of new transactions synced</returns>
    Task<int> SyncPoolTransactionsAsync(string poolAddress, int limit = 50);
    
    /// <summary>
    /// Sync recent transactions for all pools
    /// </summary>
    /// <param name="limit">Maximum number of transactions per pool</param>
    /// <returns>Number of new transactions synced across all pools</returns>
    Task<int> SyncAllPoolTransactionsAsync(int limit = 50);
    
    /// <summary>
    /// Get the last synchronized slot for tracking incremental updates
    /// </summary>
    /// <param name="network">Network to check</param>
    /// <returns>Last synced slot number</returns>
    Task<ulong> GetLastSyncedSlotAsync(string network);
    
    /// <summary>
    /// Update the last synchronized slot
    /// </summary>
    /// <param name="network">Network to update</param>
    /// <param name="slot">Slot number to record</param>
    Task UpdateLastSyncedSlotAsync(string network, ulong slot);
}

/// <summary>
/// Result of a synchronization operation
/// </summary>
public class SyncResult
{
    public bool Success { get; set; }
    public int ItemsProcessed { get; set; }
    public int ItemsUpdated { get; set; }
    public int ItemsCreated { get; set; }
    public int ItemsSkipped { get; set; }
    public string? ErrorMessage { get; set; }
    public TimeSpan Duration { get; set; }
    public ulong? LastProcessedSlot { get; set; }
}

/// <summary>
/// Configuration for sync operations
/// </summary>
public class SyncOptions
{
    /// <summary>
    /// Maximum number of items to process in a single batch
    /// </summary>
    public int BatchSize { get; set; } = 50;
    
    /// <summary>
    /// Maximum number of parallel operations
    /// </summary>
    public int MaxConcurrency { get; set; } = 10;
    
    /// <summary>
    /// Whether to sync transaction history
    /// </summary>
    public bool SyncTransactions { get; set; } = true;
    
    /// <summary>
    /// Whether to force refresh even if data seems up to date
    /// </summary>
    public bool ForceRefresh { get; set; } = false;
    
    /// <summary>
    /// Maximum age of data before forcing a refresh
    /// </summary>
    public TimeSpan MaxDataAge { get; set; } = TimeSpan.FromMinutes(5);
    
    /// <summary>
    /// Network to sync (if null, uses service default)
    /// </summary>
    public string? Network { get; set; }
} 