namespace FixedRatioTrading.Dashboard.Solana.Services;

/// <summary>
/// Service interface for managing periodic blockchain data polling
/// </summary>
public interface IPollingService
{
    /// <summary>
    /// Whether the polling service is currently running
    /// </summary>
    bool IsRunning { get; }
    
    /// <summary>
    /// Current polling configuration
    /// </summary>
    PollingConfiguration Configuration { get; }
    
    /// <summary>
    /// Start the polling service with the given configuration
    /// </summary>
    /// <param name="configuration">Polling configuration</param>
    /// <param name="cancellationToken">Cancellation token</param>
    Task StartAsync(PollingConfiguration configuration, CancellationToken cancellationToken = default);
    
    /// <summary>
    /// Stop the polling service
    /// </summary>
    /// <param name="cancellationToken">Cancellation token</param>
    Task StopAsync(CancellationToken cancellationToken = default);
    
    /// <summary>
    /// Force an immediate poll cycle
    /// </summary>
    /// <param name="cancellationToken">Cancellation token</param>
    Task TriggerPollAsync(CancellationToken cancellationToken = default);
    
    /// <summary>
    /// Get statistics about the polling service
    /// </summary>
    /// <returns>Polling statistics</returns>
    Task<PollingStatistics> GetStatisticsAsync();
    
    /// <summary>
    /// Event raised when a poll cycle completes
    /// </summary>
    event EventHandler<PollCompletedEventArgs>? PollCompleted;
    
    /// <summary>
    /// Event raised when an error occurs during polling
    /// </summary>
    event EventHandler<PollErrorEventArgs>? PollError;
}

/// <summary>
/// Configuration for the polling service
/// </summary>
public class PollingConfiguration
{
    /// <summary>
    /// How often to poll for updates
    /// </summary>
    public TimeSpan PollInterval { get; set; } = TimeSpan.FromMinutes(1);
    
    /// <summary>
    /// How often to poll for new pools
    /// </summary>
    public TimeSpan PoolDiscoveryInterval { get; set; } = TimeSpan.FromMinutes(5);
    
    /// <summary>
    /// How often to sync system state
    /// </summary>
    public TimeSpan SystemStateInterval { get; set; } = TimeSpan.FromMinutes(2);
    
    /// <summary>
    /// Maximum number of transactions to sync per pool per cycle
    /// </summary>
    public int MaxTransactionsPerPool { get; set; } = 50;
    
    /// <summary>
    /// Maximum number of pools to process in parallel
    /// </summary>
    public int MaxConcurrentPools { get; set; } = 10;
    
    /// <summary>
    /// Whether to enable pool discovery (looking for new pools)
    /// </summary>
    public bool EnablePoolDiscovery { get; set; } = true;
    
    /// <summary>
    /// Whether to sync transaction history
    /// </summary>
    public bool SyncTransactions { get; set; } = true;
    
    /// <summary>
    /// Whether to sync system state
    /// </summary>
    public bool SyncSystemState { get; set; } = true;
    
    /// <summary>
    /// Network to monitor
    /// </summary>
    public string Network { get; set; } = "testnet";
    
    /// <summary>
    /// Maximum time to wait for a poll operation before timing out
    /// </summary>
    public TimeSpan OperationTimeout { get; set; } = TimeSpan.FromMinutes(5);
    
    /// <summary>
    /// Whether to retry failed operations
    /// </summary>
    public bool RetryOnFailure { get; set; } = true;
    
    /// <summary>
    /// Maximum number of retry attempts
    /// </summary>
    public int MaxRetryAttempts { get; set; } = 3;
    
    /// <summary>
    /// Delay between retry attempts
    /// </summary>
    public TimeSpan RetryDelay { get; set; } = TimeSpan.FromSeconds(30);
}

/// <summary>
/// Statistics about polling operations
/// </summary>
public class PollingStatistics
{
    public DateTime ServiceStartTime { get; set; }
    public TimeSpan TotalRuntime { get; set; }
    public int TotalPollCycles { get; set; }
    public int SuccessfulCycles { get; set; }
    public int FailedCycles { get; set; }
    public DateTime? LastSuccessfulPoll { get; set; }
    public DateTime? LastFailedPoll { get; set; }
    public int PoolsSynced { get; set; }
    public int TransactionsSynced { get; set; }
    public int NewPoolsDiscovered { get; set; }
    public TimeSpan AverageCycleTime { get; set; }
    public string? LastError { get; set; }
    public int ConsecutiveFailures { get; set; }
}

/// <summary>
/// Event args for poll completed events
/// </summary>
public class PollCompletedEventArgs : EventArgs
{
    public DateTime StartTime { get; set; }
    public DateTime EndTime { get; set; }
    public TimeSpan Duration { get; set; }
    public int PoolsSynced { get; set; }
    public int TransactionsSynced { get; set; }
    public int NewPoolsDiscovered { get; set; }
    public bool SystemStateSynced { get; set; }
    public SyncResult Result { get; set; } = new();
}

/// <summary>
/// Event args for poll error events
/// </summary>
public class PollErrorEventArgs : EventArgs
{
    public DateTime Timestamp { get; set; }
    public string ErrorMessage { get; set; } = string.Empty;
    public Exception? Exception { get; set; }
    public string Operation { get; set; } = string.Empty;
    public int RetryAttempt { get; set; }
    public bool WillRetry { get; set; }
} 