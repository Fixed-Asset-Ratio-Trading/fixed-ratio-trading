using FixedRatioTrading.Dashboard.Core.Models;

namespace FixedRatioTrading.Dashboard.Data.Repositories;

/// <summary>
/// Repository interface for SystemState entities with domain-specific operations
/// </summary>
public interface ISystemStateRepository : IRepository<SystemState>
{
    // System state queries
    Task<SystemState?> GetByNetworkAsync(string network);
    Task<SystemState> GetCurrentStateAsync(string network = "testnet");
    Task<bool> IsSystemPausedAsync(string network = "testnet");
    Task<bool> IsSystemInEmergencyStopAsync(string network = "testnet");
    Task<bool> IsSystemUnderMaintenanceAsync(string network = "testnet");
    
    // System operations
    Task PauseSystemAsync(string network, string pausedBy, string reason, string transactionSignature);
    Task UnpauseSystemAsync(string network, string unpausedBy, string transactionSignature);
    Task SetEmergencyStopAsync(string network, string triggeredBy, string reason, string transactionSignature);
    Task ClearEmergencyStopAsync(string network, string clearedBy, string transactionSignature);
    Task SetMaintenanceModeAsync(string network, DateTime? endTime, string notes);
    Task ClearMaintenanceModeAsync(string network);
    
    // System statistics
    Task UpdateSystemStatsAsync(
        string network,
        int totalPools,
        int activePools,
        decimal? totalValueLockedUsd = null,
        decimal? volume24hUsd = null,
        int? uniqueUsers = null);
    Task<SystemState?> GetLatestStatsAsync(string network = "testnet");
    Task<IEnumerable<SystemState>> GetStatsHistoryAsync(string network, int days = 30);
    
    // Version and upgrade tracking
    Task UpdateSystemVersionAsync(string network, string version, string upgradedBy, string transactionSignature);
    Task<string> GetCurrentVersionAsync(string network = "testnet");
    Task<IEnumerable<SystemState>> GetVersionHistoryAsync(string network);
    
    // System monitoring
    Task<DateTime?> GetLastOperationTimeAsync(string network, SystemOperationType operationType);
    Task<IEnumerable<SystemState>> GetSystemEventsAsync(string network, TimeSpan period);
    Task<TimeSpan?> GetSystemUptimeAsync(string network);
    Task<int> GetMaintenanceCountAsync(string network, TimeSpan period);
    
    // Health checks
    Task<bool> IsSystemHealthyAsync(string network = "testnet");
    Task<Dictionary<string, object>> GetSystemHealthMetricsAsync(string network = "testnet");
    Task RecordSystemHealthCheckAsync(string network, bool isHealthy, Dictionary<string, object>? metrics = null);
    
    // Multi-network operations
    Task<IEnumerable<SystemState>> GetAllNetworkStatesAsync();
    Task<Dictionary<string, bool>> GetNetworkStatusAsync(); // network -> isPaused
    Task BulkUpdateNetworkStatsAsync(Dictionary<string, (int TotalPools, int ActivePools, decimal? TvlUsd)> updates);
    
    // System initialization
    Task<SystemState> InitializeNetworkAsync(string network, string version = "1.0.0");
    Task<bool> IsNetworkInitializedAsync(string network);
} 