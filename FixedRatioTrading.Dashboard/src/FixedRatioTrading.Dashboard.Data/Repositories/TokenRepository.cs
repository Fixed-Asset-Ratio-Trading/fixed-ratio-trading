using Microsoft.EntityFrameworkCore;
using FixedRatioTrading.Dashboard.Core.Models;
using System.Linq.Expressions;

namespace FixedRatioTrading.Dashboard.Data.Repositories;

public class TokenRepository : ITokenRepository
{
    private readonly DashboardDbContext _context;

    public TokenRepository(DashboardDbContext context)
    {
        _context = context;
    }

    // Base repository methods
    public async Task<Token?> GetByIdAsync(Guid id) => await _context.Tokens.FindAsync(id);
    public async Task<IEnumerable<Token>> GetAllAsync() => await _context.Tokens.ToListAsync();
    public async Task<Token> AddAsync(Token entity) { _context.Tokens.Add(entity); await _context.SaveChangesAsync(); return entity; }
    public async Task<Token> UpdateAsync(Token entity) { _context.Tokens.Update(entity); await _context.SaveChangesAsync(); return entity; }
    public async Task DeleteAsync(Guid id) { var entity = await GetByIdAsync(id); if (entity != null) { _context.Tokens.Remove(entity); await _context.SaveChangesAsync(); } }
    public async Task<bool> ExistsAsync(Guid id) => await _context.Tokens.AnyAsync(t => t.Id == id);
    public async Task<int> CountAsync() => await _context.Tokens.CountAsync();

    // Missing IRepository<T> methods
    public async Task<IEnumerable<Token>> FindAsync(Expression<Func<Token, bool>> predicate)
    {
        return await _context.Tokens.Where(predicate).ToListAsync();
    }

    public async Task<Token?> FirstOrDefaultAsync(Expression<Func<Token, bool>> predicate)
    {
        return await _context.Tokens.FirstOrDefaultAsync(predicate);
    }

    public async Task<int> CountAsync(Expression<Func<Token, bool>> predicate)
    {
        return await _context.Tokens.CountAsync(predicate);
    }

    public async Task<bool> ExistsAsync(Expression<Func<Token, bool>> predicate)
    {
        return await _context.Tokens.AnyAsync(predicate);
    }

    public async Task<(IEnumerable<Token> Items, int TotalCount)> GetPagedAsync(
        int pageNumber,
        int pageSize,
        Expression<Func<Token, bool>>? filter = null,
        Func<IQueryable<Token>, IOrderedQueryable<Token>>? orderBy = null)
    {
        var query = _context.Tokens.AsQueryable();

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

    public async Task<IEnumerable<Token>> AddRangeAsync(IEnumerable<Token> entities)
    {
        _context.Tokens.AddRange(entities);
        await _context.SaveChangesAsync();
        return entities;
    }

    public void Update(Token entity)
    {
        _context.Tokens.Update(entity);
    }

    public void UpdateRange(IEnumerable<Token> entities)
    {
        _context.Tokens.UpdateRange(entities);
    }

    public void Remove(Token entity)
    {
        _context.Tokens.Remove(entity);
    }

    public void RemoveRange(IEnumerable<Token> entities)
    {
        _context.Tokens.RemoveRange(entities);
    }

    public async Task RemoveByIdAsync(Guid id)
    {
        var entity = await GetByIdAsync(id);
        if (entity != null)
        {
            _context.Tokens.Remove(entity);
            await _context.SaveChangesAsync();
        }
    }

    public async Task<int> SaveChangesAsync()
    {
        return await _context.SaveChangesAsync();
    }

    // Token-specific methods
    public async Task<Token?> GetByMintAddressAsync(string mintAddress) => await _context.Tokens.FirstOrDefaultAsync(t => t.MintAddress == mintAddress);
    public async Task<IEnumerable<Token>> GetByNetworkAsync(string network) => await _context.Tokens.Where(t => t.Network == network).ToListAsync();
    public async Task<IEnumerable<Token>> GetTestnetCreatedTokensAsync() => await _context.Tokens.Where(t => t.Network == "testnet").ToListAsync();
    public async Task<bool> HasCreatedTokensAsync(string creatorAddress) => await _context.Tokens.AnyAsync(t => t.CreatorAddress == creatorAddress);
    public async Task<int> GetCreatedTokenCountAsync(string creatorAddress) => await _context.Tokens.CountAsync(t => t.CreatorAddress == creatorAddress);
    public async Task<IEnumerable<Token>> GetTokensByTagAsync(string tag) => await _context.Tokens.Where(t => t.Tags != null && t.Tags.Contains(tag)).ToListAsync();
    public async Task<IEnumerable<Token>> GetPopularTokensAsync(int count, string? network = null)
    {
        var query = _context.Tokens.AsQueryable();
        if (!string.IsNullOrEmpty(network))
            query = query.Where(t => t.Network == network);
        
        return await query.OrderByDescending(t => t.TotalSupply).Take(count).ToListAsync();
    }
    public async Task UpdateTokenPriceAsync(string mintAddress, decimal price)
    {
        var token = await GetByMintAddressAsync(mintAddress);
        if (token != null)
        {
            token.LastKnownPriceUsd = price;
            token.PriceLastUpdated = DateTime.UtcNow;
            await _context.SaveChangesAsync();
        }
    }
    public async Task<IEnumerable<Token>> GetTokensWithoutPricesAsync() => await _context.Tokens.Where(t => t.LastKnownPriceUsd == null || t.LastKnownPriceUsd == 0).ToListAsync();
    public async Task<IEnumerable<Token>> GetTokensWithStalePrice(TimeSpan maxAge)
    {
        var cutoff = DateTime.UtcNow - maxAge;
        return await _context.Tokens.Where(t => t.PriceLastUpdated < cutoff).ToListAsync();
    }
    public async Task BulkUpdatePricesAsync(Dictionary<string, decimal> priceUpdates)
    {
        foreach (var update in priceUpdates)
        {
            var token = await GetByMintAddressAsync(update.Key);
            if (token != null)
            {
                token.LastKnownPriceUsd = update.Value;
                token.PriceLastUpdated = DateTime.UtcNow;
            }
        }
        await _context.SaveChangesAsync();
    }
    public async Task<bool> IsValidTokenAsync(string mintAddress) => await _context.Tokens.AnyAsync(t => t.MintAddress == mintAddress);
    public async Task MarkTokenAsVerifiedAsync(string mintAddress)
    {
        var token = await GetByMintAddressAsync(mintAddress);
        if (token != null)
        {
            token.IsVerified = true;
            await _context.SaveChangesAsync();
        }
    }
    public async Task UnverifyTokenAsync(string mintAddress)
    {
        var token = await GetByMintAddressAsync(mintAddress);
        if (token != null)
        {
            token.IsVerified = false;
            await _context.SaveChangesAsync();
        }
    }
    public async Task<IEnumerable<Token>> GetUnverifiedTokensAsync(string? network = null)
    {
        var query = _context.Tokens.Where(t => !t.IsVerified);
        if (!string.IsNullOrEmpty(network))
            query = query.Where(t => t.Network == network);
        
        return await query.ToListAsync();
    }

    public async Task<Token?> GetBySymbolAsync(string symbol, string? network = null) => await _context.Tokens.FirstOrDefaultAsync(t => t.Symbol == symbol);
    public async Task<IEnumerable<Token>> GetActiveTokensAsync(string? network = null) => await _context.Tokens.Where(t => t.IsActive).ToListAsync();
    public async Task<IEnumerable<Token>> GetVerifiedTokensAsync(string? network = null) => await _context.Tokens.Where(t => t.IsVerified).ToListAsync();
    public async Task<IEnumerable<Token>> GetTestnetCreatedTokensAsync(string? network = null) => await _context.Tokens.Where(t => t.IsTestnetCreated).ToListAsync();
    public async Task<IEnumerable<Token>> SearchTokensAsync(string searchTerm, string? network = null) => 
        await _context.Tokens.Where(t => t.Symbol.Contains(searchTerm) || t.Name.Contains(searchTerm)).ToListAsync();
    public async Task<IEnumerable<Token>> GetTokensByCreatorAsync(string creatorAddress) => await _context.Tokens.Where(t => t.CreatorAddress == creatorAddress).ToListAsync();
    public async Task<IEnumerable<Token>> GetTokensByDateRangeAsync(DateTime startDate, DateTime endDate) => 
        await _context.Tokens.Where(t => t.CreatedAt >= startDate && t.CreatedAt <= endDate).ToListAsync();
    public async Task<IEnumerable<Token>> GetRecentTokensAsync(int count = 10, string? network = null) => 
        await _context.Tokens.OrderByDescending(t => t.CreatedAt).Take(count).ToListAsync();
    public async Task<int> GetTokenCountByNetworkAsync(string network) => await _context.Tokens.CountAsync(t => t.Network == network);
    public async Task<int> GetActiveTokenCountAsync(string? network = null) => await _context.Tokens.CountAsync(t => t.IsActive);
    public async Task<int> GetVerifiedTokenCountAsync(string? network = null) => await _context.Tokens.CountAsync(t => t.IsVerified);
    public async Task<decimal> GetAverageSupplyAsync(string? network = null) => 
        await _context.Tokens.Where(t => t.TotalSupply.HasValue).AverageAsync(t => (decimal)t.TotalSupply!.Value);
    public async Task<IEnumerable<Token>> GetTokensWithoutMetadataAsync() => await _context.Tokens.Where(t => string.IsNullOrEmpty(t.Name) || string.IsNullOrEmpty(t.LogoUrl)).ToListAsync();
    public async Task<IEnumerable<Token>> GetTopTokensBySupplyAsync(int count = 10, string? network = null) => 
        await _context.Tokens.Where(t => t.TotalSupply.HasValue).OrderByDescending(t => t.TotalSupply).Take(count).ToListAsync();
    public async Task<int> GetTokenUsageCountAsync(string mintAddress) => 0; // Would need to query pools
    public async Task<IEnumerable<Token>> GetMostUsedTokensAsync(int count = 10, string? network = null) => await _context.Tokens.Take(count).ToListAsync();
    public async Task<IEnumerable<Token>> GetUnusedTokensAsync(string? network = null) => await _context.Tokens.Take(0).ToListAsync();
    public async Task<Token> CreateTestnetTokenAsync(string mintAddress, string symbol, string name, byte decimals, string creatorAddress, string transactionSignature, ulong? totalSupply = null, string? logoUrl = null, string? description = null)
    {
        var token = new Token
        {
            MintAddress = mintAddress,
            Symbol = symbol,
            Name = name,
            Decimals = decimals,
            CreatorAddress = creatorAddress,
            CreationTxSignature = transactionSignature,
            TotalSupply = totalSupply,
            LogoUrl = logoUrl,
            Description = description,
            IsTestnetCreated = true,
            Network = "testnet",
            IsActive = true,
            CreatedAt = DateTime.UtcNow
        };
        return await AddAsync(token);
    }
    public async Task<IEnumerable<Token>> AddTokenRangeAsync(IEnumerable<Token> tokens) { _context.Tokens.AddRange(tokens); await _context.SaveChangesAsync(); return tokens; }
    public async Task UpdateTokenMetadataAsync(string mintAddress, string? name = null, string? logoUrl = null, string? description = null)
    {
        var token = await GetByMintAddressAsync(mintAddress);
        if (token != null)
        {
            if (name != null) token.Name = name;
            if (logoUrl != null) token.LogoUrl = logoUrl;
            if (description != null) token.Description = description;
            await _context.SaveChangesAsync();
        }
    }
    public async Task BulkUpdateLastUpdatedAsync(IEnumerable<string> mintAddresses)
    {
        var tokens = await _context.Tokens.Where(t => mintAddresses.Contains(t.MintAddress)).ToListAsync();
        foreach (var token in tokens) token.LastUpdated = DateTime.UtcNow;
        await _context.SaveChangesAsync();
    }
    public async Task<IEnumerable<Token>> GetTokensInPoolsAsync() => await _context.Tokens.ToListAsync(); // Simplified
    public async Task<Dictionary<string, int>> GetTokenPoolCountsAsync() => await _context.Tokens.ToDictionaryAsync(t => t.MintAddress, t => 0); // Simplified
} 