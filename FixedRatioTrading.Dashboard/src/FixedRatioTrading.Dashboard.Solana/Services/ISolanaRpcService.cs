using Solnet.Rpc.Models;
using FixedRatioTrading.Dashboard.Core.Models;

namespace FixedRatioTrading.Dashboard.Solana.Services;

/// <summary>
/// Service interface for communicating with Solana blockchain
/// Provides methods to read pool state, system state, and transaction data
/// </summary>
public interface ISolanaRpcService
{
    /// <summary>
    /// Get the current network configuration
    /// </summary>
    string Network { get; }
    
    /// <summary>
    /// Test connection to Solana RPC endpoint
    /// </summary>
    /// <returns>True if connection is successful</returns>
    Task<bool> TestConnectionAsync();
    
    /// <summary>
    /// Get current slot (block) number
    /// </summary>
    /// <returns>Current slot number</returns>
    Task<ulong> GetCurrentSlotAsync();
    
    /// <summary>
    /// Read pool state from blockchain
    /// </summary>
    /// <param name="poolAddress">Pool account address</param>
    /// <returns>Pool state data or null if not found</returns>
    Task<PoolStateData?> GetPoolStateAsync(string poolAddress);
    
    /// <summary>
    /// Read system state from blockchain
    /// </summary>
    /// <param name="systemStateAddress">System state account address</param>
    /// <returns>System state data or null if not found</returns>
    Task<SystemStateData?> GetSystemStateAsync(string systemStateAddress);
    
    /// <summary>
    /// Get recent transactions for a pool
    /// </summary>
    /// <param name="poolAddress">Pool account address</param>
    /// <param name="limit">Maximum number of transactions to fetch</param>
    /// <returns>List of transaction signatures</returns>
    Task<IEnumerable<string>> GetRecentPoolTransactionsAsync(string poolAddress, int limit = 50);
    
    /// <summary>
    /// Get transaction details by signature
    /// </summary>
    /// <param name="signature">Transaction signature</param>
    /// <returns>Transaction details or null if not found</returns>
    Task<TransactionData?> GetTransactionDetailsAsync(string signature);
    
    /// <summary>
    /// Get account balance for a token account
    /// </summary>
    /// <param name="accountAddress">Token account address</param>
    /// <returns>Balance in smallest token units</returns>
    Task<ulong> GetTokenAccountBalanceAsync(string accountAddress);
    
    /// <summary>
    /// Get multiple account data in a single RPC call
    /// </summary>
    /// <param name="accountAddresses">List of account addresses</param>
    /// <returns>Dictionary of address to account data</returns>
    Task<Dictionary<string, AccountInfo?>> GetMultipleAccountsAsync(IEnumerable<string> accountAddresses);
}

/// <summary>
/// Raw pool state data from blockchain
/// </summary>
public class PoolStateData
{
    public string Owner { get; set; } = string.Empty;
    public string TokenAMint { get; set; } = string.Empty;
    public string TokenBMint { get; set; } = string.Empty;
    public string TokenAVault { get; set; } = string.Empty;
    public string TokenBVault { get; set; } = string.Empty;
    public string LpTokenAMint { get; set; } = string.Empty;
    public string LpTokenBMint { get; set; } = string.Empty;
    public ulong Ratio { get; set; }
    public ulong TotalTokenALiquidity { get; set; }
    public ulong TotalTokenBLiquidity { get; set; }
    public byte PoolAuthorityBumpSeed { get; set; }
    public byte TokenAVaultBumpSeed { get; set; }
    public byte TokenBVaultBumpSeed { get; set; }
    public bool IsInitialized { get; set; }
    public bool IsPaused { get; set; }
    public bool SwapsPaused { get; set; }
    public string? SwapsPauseInitiatedBy { get; set; }
    public long SwapsPauseInitiatedTimestamp { get; set; }
    public bool WithdrawalProtectionActive { get; set; }
    public ulong CollectedFeesTokenA { get; set; }
    public ulong CollectedFeesTokenB { get; set; }
    public ulong TotalFeesWithdrawnTokenA { get; set; }
    public ulong TotalFeesWithdrawnTokenB { get; set; }
    public ulong SwapFeeBasisPoints { get; set; }
    public ulong CollectedSolFees { get; set; }
    public ulong TotalSolFeesWithdrawn { get; set; }
}

/// <summary>
/// Raw system state data from blockchain
/// </summary>
public class SystemStateData
{
    public string Authority { get; set; } = string.Empty;
    public bool IsPaused { get; set; }
    public long PauseTimestamp { get; set; }
    public string PauseReason { get; set; } = string.Empty;
}

/// <summary>
/// Raw transaction data from blockchain
/// </summary>
public class TransactionData
{
    public string Signature { get; set; } = string.Empty;
    public ulong Slot { get; set; }
    public DateTime BlockTime { get; set; }
    public bool IsSuccessful { get; set; }
    public string? ErrorMessage { get; set; }
    public ulong Fee { get; set; }
    public TransactionType Type { get; set; }
    public string UserAddress { get; set; } = string.Empty;
    public string PoolAddress { get; set; } = string.Empty;
    public ulong TokenAAmount { get; set; }
    public ulong TokenBAmount { get; set; }
    public ulong LpTokenAmount { get; set; }
} 