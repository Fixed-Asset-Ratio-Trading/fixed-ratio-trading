using FixedRatioTrading.Dashboard.Core.Models;

namespace FixedRatioTrading.Dashboard.Data.Repositories;

/// <summary>
/// Repository interface for PoolDelegate entities with domain-specific operations
/// </summary>
public interface IPoolDelegateRepository : IRepository<PoolDelegate>
{
    // Delegate-specific queries
    Task<PoolDelegate?> GetByPoolAndDelegateAsync(Guid poolId, string delegateAddress);
    Task<IEnumerable<PoolDelegate>> GetByPoolIdAsync(Guid poolId);
    Task<IEnumerable<PoolDelegate>> GetByDelegateAddressAsync(string delegateAddress);
    Task<IEnumerable<PoolDelegate>> GetActiveDelegatesAsync(Guid? poolId = null);
    Task<IEnumerable<PoolDelegate>> GetByNetworkAsync(string network);
    
    // Authorization checks
    Task<bool> IsAuthorizedDelegateAsync(Guid poolId, string delegateAddress);
    Task<bool> HasActiveDelegatesAsync(Guid poolId);
    Task<int> GetActiveDelegateCountAsync(Guid poolId);
    
    // Delegate management
    Task<PoolDelegate> AddDelegateAsync(
        Guid poolId, 
        string delegateAddress, 
        string addedByAddress, 
        string transactionSignature,
        string? displayName = null,
        string? contactEmail = null);
    Task RemoveDelegateAsync(
        Guid poolId, 
        string delegateAddress, 
        string removedByAddress, 
        string transactionSignature);
    
    // Fee withdrawal tracking
    Task UpdateFeeWithdrawalStatsAsync(
        Guid poolId, 
        string delegateAddress, 
        ulong tokenAFees, 
        ulong tokenBFees);
    Task<(ulong TokenAFees, ulong TokenBFees)> GetTotalFeesWithdrawnAsync(
        string delegateAddress, 
        Guid? poolId = null);
    Task<IEnumerable<PoolDelegate>> GetTopDelegatesByFeesAsync(int count = 10);
    
    // Delegate activity
    Task<DateTime?> GetLastWithdrawalDateAsync(string delegateAddress, Guid? poolId = null);
    Task<IEnumerable<PoolDelegate>> GetMostActiveDelegatesAsync(int count = 10, TimeSpan? period = null);
    Task<IEnumerable<PoolDelegate>> GetInactiveDelegatesAsync(TimeSpan inactivePeriod);
    
    // Historical tracking
    Task<IEnumerable<PoolDelegate>> GetDelegateHistoryAsync(Guid poolId);
    Task<IEnumerable<PoolDelegate>> GetRemovedDelegatesAsync(Guid? poolId = null);
    Task<TimeSpan> GetAverageDelegateAuthorizedDurationAsync(Guid? poolId = null);
    
    // Bulk operations
    Task BulkUpdateFeeStatsAsync(Dictionary<(Guid PoolId, string DelegateAddress), (ulong TokenAFees, ulong TokenBFees)> updates);
    Task<int> GetTotalUniqueDelegatesAsync(string? network = null);
} 