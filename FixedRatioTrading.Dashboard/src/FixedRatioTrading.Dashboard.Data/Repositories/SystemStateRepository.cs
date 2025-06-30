using Microsoft.EntityFrameworkCore;
using FixedRatioTrading.Dashboard.Core.Models;

namespace FixedRatioTrading.Dashboard.Data.Repositories;

public class SystemStateRepository : ISystemStateRepository
{
    private readonly DashboardDbContext _context;

    public SystemStateRepository(DashboardDbContext context)
    {
        _context = context;
    }

    public async Task<SystemState?> GetByIdAsync(Guid id) => await _context.SystemStates.FindAsync(id);
    public async Task<IEnumerable<SystemState>> GetAllAsync() => await _context.SystemStates.ToListAsync();
    public async Task<SystemState> AddAsync(SystemState entity) { _context.SystemStates.Add(entity); await _context.SaveChangesAsync(); return entity; }
    public async Task<SystemState> UpdateAsync(SystemState entity) { _context.SystemStates.Update(entity); await _context.SaveChangesAsync(); return entity; }
    public async Task DeleteAsync(Guid id) { var entity = await GetByIdAsync(id); if (entity != null) { _context.SystemStates.Remove(entity); await _context.SaveChangesAsync(); } }
    public async Task<bool> ExistsAsync(Guid id) => await _context.SystemStates.AnyAsync(s => s.Id == id);
    public async Task<int> CountAsync() => await _context.SystemStates.CountAsync();

    public async Task<SystemState?> GetByNetworkAsync(string network) => await _context.SystemStates.FirstOrDefaultAsync(s => s.Network == network);
    public async Task<IEnumerable<SystemState>> GetPausedSystemsAsync() => await _context.SystemStates.Where(s => s.IsPaused).ToListAsync();
    public async Task<SystemState?> GetMostRecentlyUpdatedAsync(string? network = null)
    {
        var query = _context.SystemStates.AsQueryable();
        if (!string.IsNullOrEmpty(network)) query = query.Where(s => s.Network == network);
        return await query.OrderByDescending(s => s.UpdatedAt).FirstOrDefaultAsync();
    }
    public async Task<bool> IsSystemPausedAsync(string network)
    {
        var systemState = await GetByNetworkAsync(network);
        return systemState?.IsPaused ?? false;
    }
    public async Task<IEnumerable<SystemState>> GetByAuthorityAsync(string authority) => await _context.SystemStates.Where(s => s.Authority == authority).ToListAsync();
    public async Task<IEnumerable<SystemState>> GetByDateRangeAsync(DateTime startDate, DateTime endDate) => await _context.SystemStates.Where(s => s.UpdatedAt >= startDate && s.UpdatedAt <= endDate).ToListAsync();
    public async Task<IEnumerable<SystemState>> GetRecentUpdatesAsync(int count = 10, string? network = null)
    {
        var query = _context.SystemStates.AsQueryable();
        if (!string.IsNullOrEmpty(network)) query = query.Where(s => s.Network == network);
        return await query.OrderByDescending(s => s.UpdatedAt).Take(count).ToListAsync();
    }
    public async Task<Dictionary<string, bool>> GetPauseStatusByNetworkAsync() => await _context.SystemStates.ToDictionaryAsync(s => s.Network, s => s.IsPaused);
    public async Task<SystemState?> GetOldestUnprocessedAsync(string? network = null) => await _context.SystemStates.OrderBy(s => s.UpdatedAt).FirstOrDefaultAsync();
    public async Task<IEnumerable<SystemState>> GetActiveSystemsAsync() => await _context.SystemStates.Where(s => !s.IsPaused).ToListAsync();
    public async Task UpdatePauseStatusAsync(string network, bool isPaused, string? reason = null)
    {
        var systemState = await GetByNetworkAsync(network);
        if (systemState != null)
        {
            systemState.IsPaused = isPaused;
            systemState.PauseReason = reason ?? string.Empty;
            systemState.PauseTimestamp = isPaused ? DateTimeOffset.UtcNow.ToUnixTimeSeconds() : 0;
            systemState.UpdatedAt = DateTime.UtcNow;
            await _context.SaveChangesAsync();
        }
    }
    public async Task BulkUpdateLastSyncAsync(IEnumerable<string> networks)
    {
        var systemStates = await _context.SystemStates.Where(s => networks.Contains(s.Network)).ToListAsync();
        foreach (var systemState in systemStates)
        {
            systemState.LastSyncAt = DateTime.UtcNow;
            systemState.UpdatedAt = DateTime.UtcNow;
        }
        await _context.SaveChangesAsync();
    }
    public async Task<TimeSpan?> GetAverageUpdateIntervalAsync(string? network = null)
    {
        // Simplified implementation - return null for now
        return null;
    }
} 