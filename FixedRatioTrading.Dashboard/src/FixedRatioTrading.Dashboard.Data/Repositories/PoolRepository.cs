using Microsoft.EntityFrameworkCore;
using FixedRatioTrading.Dashboard.Core.Models;
using System.Linq.Expressions;

namespace FixedRatioTrading.Dashboard.Data.Repositories;

/// <summary>
/// Entity Framework implementation of IPoolRepository
/// </summary>
public class PoolRepository : IPoolRepository
{
    private readonly DashboardDbContext _context;

    public PoolRepository(DashboardDbContext context)
    {
        _context = context;
    }

    // Base repository methods
    public async Task<Pool?> GetByIdAsync(Guid id)
    {
        return await _context.Pools.FindAsync(id);
    }

    public async Task<IEnumerable<Pool>> GetAllAsync()
    {
        return await _context.Pools.ToListAsync();
    }

    public async Task<Pool> AddAsync(Pool entity)
    {
        _context.Pools.Add(entity);
        await _context.SaveChangesAsync();
        return entity;
    }

    public async Task<Pool> UpdateAsync(Pool entity)
    {
        _context.Pools.Update(entity);
        await _context.SaveChangesAsync();
        return entity;
    }

    public async Task DeleteAsync(Guid id)
    {
        var pool = await GetByIdAsync(id);
        if (pool != null)
        {
            _context.Pools.Remove(pool);
            await _context.SaveChangesAsync();
        }
    }

    public async Task<bool> ExistsAsync(Guid id)
    {
        return await _context.Pools.AnyAsync(p => p.Id == id);
    }

    public async Task<int> CountAsync()
    {
        return await _context.Pools.CountAsync();
    }

    // Missing IRepository<T> methods
    public async Task<IEnumerable<Pool>> FindAsync(Expression<Func<Pool, bool>> predicate)
    {
        return await _context.Pools.Where(predicate).ToListAsync();
    }

    public async Task<Pool?> FirstOrDefaultAsync(Expression<Func<Pool, bool>> predicate)
    {
        return await _context.Pools.FirstOrDefaultAsync(predicate);
    }

    public async Task<int> CountAsync(Expression<Func<Pool, bool>> predicate)
    {
        return await _context.Pools.CountAsync(predicate);
    }

    public async Task<bool> ExistsAsync(Expression<Func<Pool, bool>> predicate)
    {
        return await _context.Pools.AnyAsync(predicate);
    }

    public async Task<(IEnumerable<Pool> Items, int TotalCount)> GetPagedAsync(
        int pageNumber,
        int pageSize,
        Expression<Func<Pool, bool>>? filter = null,
        Func<IQueryable<Pool>, IOrderedQueryable<Pool>>? orderBy = null)
    {
        var query = _context.Pools.AsQueryable();

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

    public async Task<IEnumerable<Pool>> AddRangeAsync(IEnumerable<Pool> entities)
    {
        _context.Pools.AddRange(entities);
        await _context.SaveChangesAsync();
        return entities;
    }

    public void Update(Pool entity)
    {
        _context.Pools.Update(entity);
    }

    public void UpdateRange(IEnumerable<Pool> entities)
    {
        _context.Pools.UpdateRange(entities);
    }

    public void Remove(Pool entity)
    {
        _context.Pools.Remove(entity);
    }

    public void RemoveRange(IEnumerable<Pool> entities)
    {
        _context.Pools.RemoveRange(entities);
    }

    public async Task RemoveByIdAsync(Guid id)
    {
        var entity = await GetByIdAsync(id);
        if (entity != null)
        {
            _context.Pools.Remove(entity);
            await _context.SaveChangesAsync();
        }
    }

    public async Task<int> SaveChangesAsync()
    {
        return await _context.SaveChangesAsync();
    }

    // Pool-specific methods
    public async Task<Pool?> GetByPoolAddressAsync(string poolAddress)
    {
        return await _context.Pools.FirstOrDefaultAsync(p => p.PoolAddress == poolAddress);
    }

    public async Task<Pool?> GetByTokenPairAsync(string tokenAMint, string tokenBMint)
    {
        return await _context.Pools.FirstOrDefaultAsync(p => 
            p.TokenAMint == tokenAMint && p.TokenBMint == tokenBMint);
    }

    public async Task<IEnumerable<Pool>> GetByNetworkAsync(string network)
    {
        return await _context.Pools.Where(p => p.Network == network).ToListAsync();
    }

    public async Task<IEnumerable<Pool>> GetActivePoolsAsync(string? network = null)
    {
        var query = _context.Pools.Where(p => p.IsActive);
        
        if (!string.IsNullOrEmpty(network))
        {
            query = query.Where(p => p.Network == network);
        }
        
        return await query.ToListAsync();
    }

    public async Task<IEnumerable<Pool>> GetPoolsByCreatorAsync(string creatorAddress)
    {
        return await _context.Pools.Where(p => p.Owner == creatorAddress).ToListAsync();
    }

    public async Task<IEnumerable<Pool>> GetPoolsWithTokenAsync(string tokenMint)
    {
        return await _context.Pools.Where(p => p.TokenAMint == tokenMint || p.TokenBMint == tokenMint).ToListAsync();
    }

    public async Task<int> GetActivePoolCountAsync(string? network = null)
    {
        var query = _context.Pools.Where(p => p.IsActive);
        
        if (!string.IsNullOrEmpty(network))
        {
            query = query.Where(p => p.Network == network);
        }
        
        return await query.CountAsync();
    }

    public async Task<decimal?> GetTotalValueLockedAsync(string? network = null)
    {
        var query = _context.Pools.Where(p => p.IsActive);
        
        if (!string.IsNullOrEmpty(network))
        {
            query = query.Where(p => p.Network == network);
        }
        
        var totalLiquidity = await query.SumAsync(p => (decimal)(p.TotalTokenALiquidity + p.TotalTokenBLiquidity));
        return totalLiquidity;
    }

    public async Task<(ulong TokenAVolume, ulong TokenBVolume)> GetPoolVolumeAsync(Guid poolId, TimeSpan? period = null)
    {
        var pool = await GetByIdAsync(poolId);
        if (pool == null) return (0, 0);
        
        // For now, return the total volume. In a full implementation, you'd filter by period
        return (pool.TotalVolumeTokenA, pool.TotalVolumeTokenB);
    }

    public async Task<IEnumerable<Pool>> SearchPoolsAsync(string searchTerm, string? network = null)
    {
        var query = _context.Pools.Where(p => 
            p.TokenASymbol.Contains(searchTerm) || 
            p.TokenBSymbol.Contains(searchTerm) ||
            p.TokenAName.Contains(searchTerm) ||
            p.TokenBName.Contains(searchTerm) ||
            p.PoolAddress.Contains(searchTerm));
        
        if (!string.IsNullOrEmpty(network))
        {
            query = query.Where(p => p.Network == network);
        }
        
        return await query.ToListAsync();
    }

    public async Task<IEnumerable<Pool>> GetTopPoolsByVolumeAsync(int count = 10, string? network = null)
    {
        var query = _context.Pools.Where(p => p.IsActive);
        
        if (!string.IsNullOrEmpty(network))
        {
            query = query.Where(p => p.Network == network);
        }
        
        return await query
            .OrderByDescending(p => p.TotalVolumeTokenA + p.TotalVolumeTokenB)
            .Take(count)
            .ToListAsync();
    }

    public async Task<IEnumerable<Pool>> GetRecentPoolsAsync(int count = 10, string? network = null)
    {
        var query = _context.Pools.Where(p => p.IsActive);
        
        if (!string.IsNullOrEmpty(network))
        {
            query = query.Where(p => p.Network == network);
        }
        
        return await query
            .OrderByDescending(p => p.CreatedAt)
            .Take(count)
            .ToListAsync();
    }

    public async Task<Pool?> GetPoolWithTransactionsAsync(Guid poolId, int? transactionLimit = null)
    {
        var query = _context.Pools
            .Include(p => p.Transactions)
            .Where(p => p.Id == poolId);
        
        var pool = await query.FirstOrDefaultAsync();
        
        if (pool != null && transactionLimit.HasValue)
        {
            pool.Transactions = pool.Transactions
                .OrderByDescending(t => t.ProcessedAt)
                .Take(transactionLimit.Value)
                .ToList();
        }
        
        return pool;
    }

    public async Task<Pool?> GetPoolWithFullDataAsync(Guid poolId)
    {
        return await _context.Pools
            .Include(p => p.Transactions)
            .FirstOrDefaultAsync(p => p.Id == poolId);
    }

    public async Task UpdatePoolLiquidityAsync(string poolAddress, ulong tokenALiquidity, ulong tokenBLiquidity)
    {
        var pool = await GetByPoolAddressAsync(poolAddress);
        if (pool != null)
        {
            pool.TotalTokenALiquidity = tokenALiquidity;
            pool.TotalTokenBLiquidity = tokenBLiquidity;
            pool.LastUpdated = DateTime.UtcNow;
            await _context.SaveChangesAsync();
        }
    }

    public async Task UpdatePoolStatsAsync(string poolAddress, ulong volumeA, ulong volumeB, int uniqueProviders)
    {
        var pool = await GetByPoolAddressAsync(poolAddress);
        if (pool != null)
        {
            pool.TotalVolumeTokenA = volumeA;
            pool.TotalVolumeTokenB = volumeB;
            pool.UniqueLiquidityProviders = uniqueProviders;
            pool.LastUpdated = DateTime.UtcNow;
            await _context.SaveChangesAsync();
        }
    }

    public async Task BulkUpdateLastUpdatedAsync(IEnumerable<string> poolAddresses)
    {
        var pools = await _context.Pools
            .Where(p => poolAddresses.Contains(p.PoolAddress))
            .ToListAsync();
        
        foreach (var pool in pools)
        {
            pool.LastUpdated = DateTime.UtcNow;
        }
        
        await _context.SaveChangesAsync();
    }
} 