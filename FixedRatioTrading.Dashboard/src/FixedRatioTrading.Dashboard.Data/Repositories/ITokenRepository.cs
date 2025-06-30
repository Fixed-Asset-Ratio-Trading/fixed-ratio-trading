using FixedRatioTrading.Dashboard.Core.Models;

namespace FixedRatioTrading.Dashboard.Data.Repositories;

/// <summary>
/// Repository interface for Token entities with domain-specific operations
/// </summary>
public interface ITokenRepository : IRepository<Token>
{
    // Token-specific queries
    Task<Token?> GetByMintAddressAsync(string mintAddress);
    Task<Token?> GetBySymbolAsync(string symbol, string? network = null);
    Task<IEnumerable<Token>> GetByNetworkAsync(string network);
    Task<IEnumerable<Token>> GetActiveTokensAsync(string? network = null);
    Task<IEnumerable<Token>> GetVerifiedTokensAsync(string? network = null);
    
    // Testnet token creation
    Task<IEnumerable<Token>> GetTestnetCreatedTokensAsync();
    Task<IEnumerable<Token>> GetTokensByCreatorAsync(string creatorAddress);
    Task<bool> HasCreatedTokensAsync(string creatorAddress);
    Task<int> GetCreatedTokenCountAsync(string creatorAddress);
    
    // Token search and discovery
    Task<IEnumerable<Token>> SearchTokensAsync(string searchTerm, string? network = null);
    Task<IEnumerable<Token>> GetTokensByTagAsync(string tag);
    Task<IEnumerable<Token>> GetRecentTokensAsync(int count = 10, string? network = null);
    Task<IEnumerable<Token>> GetPopularTokensAsync(int count = 10, string? network = null);
    
    // Token metadata and pricing
    Task UpdateTokenPriceAsync(string mintAddress, decimal priceUsd);
    Task<IEnumerable<Token>> GetTokensWithoutPricesAsync();
    Task<IEnumerable<Token>> GetTokensWithStalePrice(TimeSpan maxAge);
    Task BulkUpdatePricesAsync(Dictionary<string, decimal> priceUpdates);
    
    // Token validation and verification
    Task<bool> IsValidTokenAsync(string mintAddress);
    Task MarkTokenAsVerifiedAsync(string mintAddress);
    Task UnverifyTokenAsync(string mintAddress);
    Task<IEnumerable<Token>> GetUnverifiedTokensAsync(string? network = null);
    
    // Token usage statistics
    Task<int> GetTokenUsageCountAsync(string mintAddress); // Count of pools using this token
    Task<IEnumerable<Token>> GetMostUsedTokensAsync(int count = 10, string? network = null);
    Task<IEnumerable<Token>> GetUnusedTokensAsync(string? network = null);
    
    // Token creation management
    Task<Token> CreateTestnetTokenAsync(
        string mintAddress,
        string symbol,
        string name,
        byte decimals,
        string creatorAddress,
        string transactionSignature,
        ulong? totalSupply = null,
        string? logoUrl = null,
        string? description = null);
    
    // Bulk operations
    Task<IEnumerable<Token>> AddTokenRangeAsync(IEnumerable<Token> tokens);
    Task UpdateTokenMetadataAsync(string mintAddress, string? name = null, string? logoUrl = null, string? description = null);
    Task BulkUpdateLastUpdatedAsync(IEnumerable<string> mintAddresses);
    
    // Token relationships
    Task<IEnumerable<Token>> GetTokensInPoolsAsync(); // Tokens that are used in at least one pool
    Task<Dictionary<string, int>> GetTokenPoolCountsAsync(); // mintAddress -> count of pools
} 