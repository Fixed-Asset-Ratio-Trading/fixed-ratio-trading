using FixedRatioTrading.Dashboard.Core.Models;

namespace FixedRatioTrading.Dashboard.Data.Repositories;

/// <summary>
/// Repository interface for PoolTransaction entities with domain-specific operations
/// </summary>
public interface IPoolTransactionRepository : IRepository<PoolTransaction>
{
    // Transaction-specific queries
    Task<PoolTransaction?> GetByTransactionSignatureAsync(string signature);
    Task<IEnumerable<PoolTransaction>> GetByPoolIdAsync(Guid poolId, int? limit = null);
    Task<IEnumerable<PoolTransaction>> GetByUserAddressAsync(string userAddress, int? limit = null);
    Task<IEnumerable<PoolTransaction>> GetByTypeAsync(TransactionType type, string? network = null);
    Task<IEnumerable<PoolTransaction>> GetByPoolAndTypeAsync(Guid poolId, TransactionType type);
    
    // Time-based queries
    Task<IEnumerable<PoolTransaction>> GetRecentTransactionsAsync(int count = 50, string? network = null);
    Task<IEnumerable<PoolTransaction>> GetTransactionsByDateRangeAsync(
        DateTime startDate, 
        DateTime endDate, 
        Guid? poolId = null);
    Task<IEnumerable<PoolTransaction>> GetTransactionsInLastPeriodAsync(
        TimeSpan period, 
        Guid? poolId = null,
        TransactionType? type = null);
    
    // Transaction statistics
    Task<int> GetTransactionCountByTypeAsync(TransactionType type, string? network = null);
    Task<Dictionary<TransactionType, int>> GetTransactionCountsByTypeAsync(string? network = null);
    Task<(ulong TokenAVolume, ulong TokenBVolume)> GetSwapVolumeAsync(
        Guid? poolId = null, 
        TimeSpan? period = null);
    Task<decimal> GetAverageGasFeeAsync(TransactionType? type = null, TimeSpan? period = null);
    
    // User activity
    Task<IEnumerable<string>> GetMostActiveUsersAsync(int count = 10, TimeSpan? period = null);
    Task<int> GetUniqueUsersCountAsync(Guid? poolId = null, TimeSpan? period = null);
    Task<IEnumerable<PoolTransaction>> GetUserTransactionHistoryAsync(
        string userAddress, 
        int page = 1, 
        int pageSize = 20);
    
    // Delegate operations
    Task<IEnumerable<PoolTransaction>> GetDelegateTransactionsAsync(string delegateAddress);
    Task<(ulong TokenAFees, ulong TokenBFees)> GetFeesWithdrawnByDelegateAsync(
        string delegateAddress, 
        Guid? poolId = null);
    
    // Failed transactions
    Task<IEnumerable<PoolTransaction>> GetFailedTransactionsAsync(
        string? network = null, 
        TimeSpan? period = null);
    Task<decimal> GetFailureRateAsync(TransactionType? type = null, TimeSpan? period = null);
    
    // Bulk operations
    Task<IEnumerable<PoolTransaction>> AddTransactionRangeAsync(IEnumerable<PoolTransaction> transactions);
    Task UpdateTransactionStatusAsync(string signature, bool isSuccessful, string? errorMessage = null);
} 