using Microsoft.EntityFrameworkCore;
using FixedRatioTrading.Dashboard.Core.Models;
using System.Linq.Expressions;

namespace FixedRatioTrading.Dashboard.Data.Repositories;

public class SystemStateRepository : ISystemStateRepository
{
    private readonly DashboardDbContext _context;

    public SystemStateRepository(DashboardDbContext context)
    {
        _context = context;
    }

    // Base repository methods
    public async Task<SystemState?> GetByIdAsync(Guid id) => await _context.SystemStates.FindAsync(id);
    public async Task<IEnumerable<SystemState>> GetAllAsync() => await _context.SystemStates.ToListAsync();
    public async Task<SystemState> AddAsync(SystemState entity) { _context.SystemStates.Add(entity); await _context.SaveChangesAsync(); return entity; }
    public async Task<SystemState> UpdateAsync(SystemState entity) { _context.SystemStates.Update(entity); await _context.SaveChangesAsync(); return entity; }
    public async Task DeleteAsync(Guid id) { var entity = await GetByIdAsync(id); if (entity != null) { _context.SystemStates.Remove(entity); await _context.SaveChangesAsync(); } }
    public async Task<bool> ExistsAsync(Guid id) => await _context.SystemStates.AnyAsync(s => s.Id == id);
    public async Task<int> CountAsync() => await _context.SystemStates.CountAsync();

    // Missing IRepository<T> methods
    public async Task<IEnumerable<SystemState>> FindAsync(Expression<Func<SystemState, bool>> predicate)
    {
        return await _context.SystemStates.Where(predicate).ToListAsync();
    }

    public async Task<SystemState?> FirstOrDefaultAsync(Expression<Func<SystemState, bool>> predicate)
    {
        return await _context.SystemStates.FirstOrDefaultAsync(predicate);
    }

    public async Task<int> CountAsync(Expression<Func<SystemState, bool>> predicate)
    {
        return await _context.SystemStates.CountAsync(predicate);
    }

    public async Task<bool> ExistsAsync(Expression<Func<SystemState, bool>> predicate)
    {
        return await _context.SystemStates.AnyAsync(predicate);
    }

    public async Task<(IEnumerable<SystemState> Items, int TotalCount)> GetPagedAsync(
        int pageNumber,
        int pageSize,
        Expression<Func<SystemState, bool>>? filter = null,
        Func<IQueryable<SystemState>, IOrderedQueryable<SystemState>>? orderBy = null)
    {
        var query = _context.SystemStates.AsQueryable();

        if (filter != null)
            query = query.Where(filter);

        var totalCount = await query.CountAsync();

        if (orderBy != null)
            query = orderBy(query);

        var items = await query
            .Skip((pageNumber - 1) * pageSize)
            .Take(pageSize)
            .ToListAsync();

        return (items, totalCount);
    }

    public async Task<IEnumerable<SystemState>> AddRangeAsync(IEnumerable<SystemState> entities)
    {
        _context.SystemStates.AddRange(entities);
        await _context.SaveChangesAsync();
        return entities;
    }

    public void Update(SystemState entity)
    {
        _context.SystemStates.Update(entity);
    }

    public void UpdateRange(IEnumerable<SystemState> entities)
    {
        _context.SystemStates.UpdateRange(entities);
    }

    public void Remove(SystemState entity)
    {
        _context.SystemStates.Remove(entity);
    }

    public void RemoveRange(IEnumerable<SystemState> entities)
    {
        _context.SystemStates.RemoveRange(entities);
    }

    public async Task RemoveByIdAsync(Guid id)
    {
        var entity = await GetByIdAsync(id);
        if (entity != null)
        {
            _context.SystemStates.Remove(entity);
            await _context.SaveChangesAsync();
        }
    }

    public async Task<int> SaveChangesAsync()
    {
        return await _context.SaveChangesAsync();
    }

    // SystemState-specific methods
    public async Task<SystemState?> GetByNetworkAsync(string network)
    {
        return await _context.SystemStates
            .Where(s => s.Network == network)
            .OrderByDescending(s => s.UpdatedAt)
            .FirstOrDefaultAsync();
    }

    public async Task<SystemState> GetCurrentStateAsync(string network = "testnet")
    {
        var state = await _context.SystemStates
            .Where(s => s.Network == network)
            .OrderByDescending(s => s.UpdatedAt)
            .FirstOrDefaultAsync();
        
        return state ?? new SystemState 
        { 
            Network = network, 
            Authority = string.Empty, 
            IsPaused = false,
            PauseTimestamp = 0,
            PauseReason = string.Empty,
            UpdatedAt = DateTime.UtcNow,
            LastSyncAt = DateTime.UtcNow
        };
    }

    public async Task<bool> IsSystemPausedAsync(string network = "testnet")
    {
        var state = await GetByNetworkAsync(network);
        return state?.IsPaused ?? false;
    }

    public async Task<bool> IsSystemInEmergencyStopAsync(string network = "testnet")
    {
        var state = await GetByNetworkAsync(network);
        return state?.IsPaused ?? false;
    }

    public async Task<bool> IsSystemUnderMaintenanceAsync(string network = "testnet")
    {
        var state = await GetByNetworkAsync(network);
        return state?.IsPaused ?? false;
    }

    // NOTE: These methods are READ-ONLY in dashboard. They return data as if operations were performed
    // but do NOT actually modify the blockchain. Actual operations must be done via CLI.
    public async Task PauseSystemAsync(string network, string pausedBy, string reason, string transactionSignature)
    {
        // Dashboard cannot perform this operation - it's authority-only via CLI
        await Task.CompletedTask;
    }

    public async Task UnpauseSystemAsync(string network, string unpausedBy, string transactionSignature)
    {
        // Dashboard cannot perform this operation - it's authority-only via CLI
        await Task.CompletedTask;
    }

    public async Task SetEmergencyStopAsync(string network, string triggeredBy, string reason, string transactionSignature)
    {
        // Dashboard cannot perform this operation - it's authority-only via CLI
        await Task.CompletedTask;
    }

    public async Task ClearEmergencyStopAsync(string network, string clearedBy, string transactionSignature)
    {
        // Dashboard cannot perform this operation - it's authority-only via CLI
        await Task.CompletedTask;
    }

    public async Task SetMaintenanceModeAsync(string network, DateTime? endTime, string notes)
    {
        // Dashboard cannot perform this operation - it's authority-only via CLI
        await Task.CompletedTask;
    }

    public async Task ClearMaintenanceModeAsync(string network)
    {
        // Dashboard cannot perform this operation - it's authority-only via CLI
        await Task.CompletedTask;
    }

    public async Task UpdateSystemStatsAsync(string network, int totalPools, int activePools, decimal? totalValueLockedUsd = null, decimal? volume24hUsd = null, int? uniqueUsers = null)
    {
        var state = await GetByNetworkAsync(network);
        if (state != null)
        {
            state.LastSyncAt = DateTime.UtcNow;
            await _context.SaveChangesAsync();
        }
    }

    public async Task<SystemState?> GetLatestStatsAsync(string network = "testnet")
    {
        return await GetByNetworkAsync(network);
    }

    public async Task<IEnumerable<SystemState>> GetStatsHistoryAsync(string network, int days = 30)
    {
        var cutoff = DateTime.UtcNow.AddDays(-days);
        return await _context.SystemStates
            .Where(s => s.Network == network && s.UpdatedAt >= cutoff)
            .OrderByDescending(s => s.UpdatedAt)
            .ToListAsync();
    }

    public async Task UpdateSystemVersionAsync(string network, string version, string upgradedBy, string transactionSignature)
    {
        // Dashboard cannot perform this operation - it's authority-only via CLI
        await Task.CompletedTask;
    }

    public async Task<string> GetCurrentVersionAsync(string network = "testnet")
    {
        var state = await GetByNetworkAsync(network);
        return state?.Version ?? "1.0.0";
    }

    public async Task<IEnumerable<SystemState>> GetVersionHistoryAsync(string network)
    {
        // Version history is not tracked in the current system state
        return Enumerable.Empty<SystemState>();
    }

    public async Task<DateTime?> GetLastOperationTimeAsync(string network, SystemOperationType operationType)
    {
        var state = await GetByNetworkAsync(network);
        if (state?.LastOperationType == operationType)
            return state.UpdatedAt;
        return null;
    }

    public async Task<IEnumerable<SystemState>> GetSystemEventsAsync(string network, TimeSpan period)
    {
        var cutoff = DateTime.UtcNow - period;
        return await _context.SystemStates
            .Where(s => s.Network == network && s.UpdatedAt >= cutoff)
            .OrderByDescending(s => s.UpdatedAt)
            .ToListAsync();
    }

    public async Task<TimeSpan?> GetSystemUptimeAsync(string network)
    {
        var state = await GetByNetworkAsync(network);
        if (state?.IsPaused == true && state.PauseTimestamp > 0)
        {
            var pauseTime = DateTimeOffset.FromUnixTimeSeconds(state.PauseTimestamp);
            return DateTime.UtcNow - pauseTime.DateTime;
        }
        return TimeSpan.FromDays(365); // Assume system has been up for a year if not paused
    }

    public async Task<int> GetMaintenanceCountAsync(string network, TimeSpan period)
    {
        // Maintenance events are not specifically tracked
        return 0;
    }

    public async Task<bool> IsSystemHealthyAsync(string network = "testnet")
    {
        var state = await GetByNetworkAsync(network);
        return state != null && !state.IsPaused;
    }

    public async Task<Dictionary<string, object>> GetSystemHealthMetricsAsync(string network = "testnet")
    {
        var state = await GetByNetworkAsync(network);
        
        return new Dictionary<string, object>
        {
            ["isHealthy"] = state != null && !state.IsPaused,
            ["isPaused"] = state?.IsPaused ?? false,
            ["lastSync"] = state?.LastSyncAt ?? DateTime.UtcNow,
            ["authority"] = state?.Authority ?? string.Empty
        };
    }

    public async Task RecordSystemHealthCheckAsync(string network, bool isHealthy, Dictionary<string, object>? metrics = null)
    {
        var state = await GetByNetworkAsync(network);
        if (state != null)
        {
            state.LastSyncAt = DateTime.UtcNow;
            await _context.SaveChangesAsync();
        }
    }

    public async Task<IEnumerable<SystemState>> GetAllNetworkStatesAsync()
    {
        return await _context.SystemStates
            .GroupBy(s => s.Network)
            .Select(g => g.OrderByDescending(s => s.UpdatedAt).First())
            .ToListAsync();
    }

    public async Task<Dictionary<string, bool>> GetNetworkStatusAsync()
    {
        var states = await GetAllNetworkStatesAsync();
        return states.ToDictionary(
            s => s.Network,
            s => s.IsPaused
        );
    }

    public async Task BulkUpdateNetworkStatsAsync(Dictionary<string, (int TotalPools, int ActivePools, decimal? TvlUsd)> updates)
    {
        foreach (var (network, stats) in updates)
        {
            var state = await GetByNetworkAsync(network);
            if (state != null)
            {
                state.LastSyncAt = DateTime.UtcNow;
            }
        }
        await _context.SaveChangesAsync();
    }

    public async Task<SystemState> InitializeNetworkAsync(string network, string version = "1.0.0")
    {
        var existing = await GetByNetworkAsync(network);
        if (existing == null)
        {
            var newState = new SystemState
            {
                Network = network,
                Authority = string.Empty,
                IsPaused = false,
                PauseTimestamp = 0,
                PauseReason = string.Empty,
                Version = version,
                UpdatedAt = DateTime.UtcNow,
                LastSyncAt = DateTime.UtcNow
            };
            return await AddAsync(newState);
        }
        return existing;
    }

    public async Task<bool> IsNetworkInitializedAsync(string network)
    {
        return await _context.SystemStates.AnyAsync(s => s.Network == network);
    }
} 