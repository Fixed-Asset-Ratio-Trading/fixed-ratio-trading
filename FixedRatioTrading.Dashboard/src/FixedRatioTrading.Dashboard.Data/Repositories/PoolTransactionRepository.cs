using Microsoft.EntityFrameworkCore;
using FixedRatioTrading.Dashboard.Core.Models;
using System.Linq.Expressions;

namespace FixedRatioTrading.Dashboard.Data.Repositories;

public class PoolTransactionRepository : IPoolTransactionRepository
{
    private readonly DashboardDbContext _context;

    public PoolTransactionRepository(DashboardDbContext context)
    {
        _context = context;
    }

    // Base repository methods
    public async Task<PoolTransaction?> GetByIdAsync(Guid id) => await _context.PoolTransactions.FindAsync(id);
    public async Task<IEnumerable<PoolTransaction>> GetAllAsync() => await _context.PoolTransactions.ToListAsync();
    public async Task<PoolTransaction> AddAsync(PoolTransaction entity) { _context.PoolTransactions.Add(entity); await _context.SaveChangesAsync(); return entity; }
    public async Task<PoolTransaction> UpdateAsync(PoolTransaction entity) { _context.PoolTransactions.Update(entity); await _context.SaveChangesAsync(); return entity; }
    public async Task DeleteAsync(Guid id) { var entity = await GetByIdAsync(id); if (entity != null) { _context.PoolTransactions.Remove(entity); await _context.SaveChangesAsync(); } }
    public async Task<bool> ExistsAsync(Guid id) => await _context.PoolTransactions.AnyAsync(t => t.Id == id);
    public async Task<int> CountAsync() => await _context.PoolTransactions.CountAsync();

    // Missing IRepository<T> methods
    public async Task<IEnumerable<PoolTransaction>> FindAsync(Expression<Func<PoolTransaction, bool>> predicate)
    {
        return await _context.PoolTransactions.Where(predicate).ToListAsync();
    }

    public async Task<PoolTransaction?> FirstOrDefaultAsync(Expression<Func<PoolTransaction, bool>> predicate)
    {
        return await _context.PoolTransactions.FirstOrDefaultAsync(predicate);
    }

    public async Task<int> CountAsync(Expression<Func<PoolTransaction, bool>> predicate)
    {
        return await _context.PoolTransactions.CountAsync(predicate);
    }

    public async Task<bool> ExistsAsync(Expression<Func<PoolTransaction, bool>> predicate)
    {
        return await _context.PoolTransactions.AnyAsync(predicate);
    }

    public async Task<(IEnumerable<PoolTransaction> Items, int TotalCount)> GetPagedAsync(
        int pageNumber,
        int pageSize,
        Expression<Func<PoolTransaction, bool>>? filter = null,
        Func<IQueryable<PoolTransaction>, IOrderedQueryable<PoolTransaction>>? orderBy = null)
    {
        var query = _context.PoolTransactions.AsQueryable();

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

    public async Task<IEnumerable<PoolTransaction>> AddRangeAsync(IEnumerable<PoolTransaction> entities)
    {
        _context.PoolTransactions.AddRange(entities);
        await _context.SaveChangesAsync();
        return entities;
    }

    public void Update(PoolTransaction entity)
    {
        _context.PoolTransactions.Update(entity);
    }

    public void UpdateRange(IEnumerable<PoolTransaction> entities)
    {
        _context.PoolTransactions.UpdateRange(entities);
    }

    public void Remove(PoolTransaction entity)
    {
        _context.PoolTransactions.Remove(entity);
    }

    public void RemoveRange(IEnumerable<PoolTransaction> entities)
    {
        _context.PoolTransactions.RemoveRange(entities);
    }

    public async Task RemoveByIdAsync(Guid id)
    {
        var entity = await GetByIdAsync(id);
        if (entity != null)
        {
            _context.PoolTransactions.Remove(entity);
            await _context.SaveChangesAsync();
        }
    }

    public async Task<int> SaveChangesAsync()
    {
        return await _context.SaveChangesAsync();
    }

    // Interface implementations - simplified for Phase 4
    public async Task<PoolTransaction?> GetByTransactionSignatureAsync(string signature) => await _context.PoolTransactions.FirstOrDefaultAsync(t => t.TransactionSignature == signature);
    public async Task<IEnumerable<PoolTransaction>> GetByPoolIdAsync(Guid poolId, int? limit = null) => await _context.PoolTransactions.Where(t => t.PoolId == poolId).Take(limit ?? 1000).ToListAsync();
    public async Task<IEnumerable<PoolTransaction>> GetByUserAddressAsync(string userAddress, int? limit = null) => await _context.PoolTransactions.Where(t => t.UserAddress == userAddress).Take(limit ?? 1000).ToListAsync();
    public async Task<IEnumerable<PoolTransaction>> GetByTypeAsync(TransactionType type, string? network = null) => await _context.PoolTransactions.Where(t => t.Type == type).ToListAsync();
    public async Task<IEnumerable<PoolTransaction>> GetByPoolAndTypeAsync(Guid poolId, TransactionType type) => await _context.PoolTransactions.Where(t => t.PoolId == poolId && t.Type == type).ToListAsync();
    public async Task<IEnumerable<PoolTransaction>> GetRecentTransactionsAsync(int count = 50, string? network = null) => await _context.PoolTransactions.OrderByDescending(t => t.ProcessedAt).Take(count).ToListAsync();
    public async Task<IEnumerable<PoolTransaction>> GetTransactionsByDateRangeAsync(DateTime startDate, DateTime endDate, Guid? poolId = null) => await _context.PoolTransactions.Where(t => t.ProcessedAt >= startDate && t.ProcessedAt <= endDate).ToListAsync();
    public async Task<IEnumerable<PoolTransaction>> GetTransactionsInLastPeriodAsync(TimeSpan period, Guid? poolId = null, TransactionType? type = null) => await _context.PoolTransactions.Where(t => t.ProcessedAt >= DateTime.UtcNow - period).ToListAsync();
    public async Task<int> GetTransactionCountByTypeAsync(TransactionType type, string? network = null) => await _context.PoolTransactions.CountAsync(t => t.Type == type);
    public async Task<Dictionary<TransactionType, int>> GetTransactionCountsByTypeAsync(string? network = null) => await _context.PoolTransactions.GroupBy(t => t.Type).ToDictionaryAsync(g => g.Key, g => g.Count());
    public async Task<(ulong TokenAVolume, ulong TokenBVolume)> GetSwapVolumeAsync(Guid? poolId = null, TimeSpan? period = null) { var txs = await _context.PoolTransactions.Where(t => t.Type == TransactionType.Swap).ToListAsync(); return ((ulong)txs.Sum(t => (decimal)t.TokenAAmount), (ulong)txs.Sum(t => (decimal)t.TokenBAmount)); }
    public async Task<decimal> GetAverageGasFeeAsync(TransactionType? type = null, TimeSpan? period = null) => await _context.PoolTransactions.AverageAsync(t => (decimal)t.GasFee);
    public async Task<IEnumerable<string>> GetMostActiveUsersAsync(int count = 10, TimeSpan? period = null) => await _context.PoolTransactions.GroupBy(t => t.UserAddress).OrderByDescending(g => g.Count()).Take(count).Select(g => g.Key).ToListAsync();
    public async Task<int> GetUniqueUsersCountAsync(Guid? poolId = null, TimeSpan? period = null) => await _context.PoolTransactions.Select(t => t.UserAddress).Distinct().CountAsync();
    public async Task<IEnumerable<PoolTransaction>> GetUserTransactionHistoryAsync(string userAddress, int page = 1, int pageSize = 20) => await _context.PoolTransactions.Where(t => t.UserAddress == userAddress).Skip((page - 1) * pageSize).Take(pageSize).ToListAsync();
    public async Task<IEnumerable<PoolTransaction>> GetFeeTransactionsAsync(Guid? poolId = null) => Enumerable.Empty<PoolTransaction>(); // Not tracked in user dashboard
    public async Task<(ulong TokenAFees, ulong TokenBFees)> GetTotalFeesWithdrawnAsync(Guid? poolId = null, TimeSpan? period = null) => (0, 0); // Not tracked in user dashboard
    public async Task<IEnumerable<PoolTransaction>> GetFailedTransactionsAsync(string? network = null, TimeSpan? period = null) => await _context.PoolTransactions.Where(t => !t.IsSuccessful).ToListAsync();
    public async Task<decimal> GetFailureRateAsync(TransactionType? type = null, TimeSpan? period = null) { var total = await _context.PoolTransactions.CountAsync(); return total == 0 ? 0 : (decimal)await _context.PoolTransactions.CountAsync(t => !t.IsSuccessful) / total * 100; }
    public async Task<IEnumerable<PoolTransaction>> AddTransactionRangeAsync(IEnumerable<PoolTransaction> transactions) { _context.PoolTransactions.AddRange(transactions); await _context.SaveChangesAsync(); return transactions; }
    public async Task UpdateTransactionStatusAsync(string signature, bool isSuccessful, string? errorMessage = null) { var tx = await GetByTransactionSignatureAsync(signature); if (tx != null) { tx.IsSuccessful = isSuccessful; tx.ErrorMessage = errorMessage; await _context.SaveChangesAsync(); } }
} 