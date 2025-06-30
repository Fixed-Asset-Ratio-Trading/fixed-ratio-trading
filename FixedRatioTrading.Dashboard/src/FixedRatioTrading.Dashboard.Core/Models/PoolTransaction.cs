using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;

namespace FixedRatioTrading.Dashboard.Core.Models;

/// <summary>
/// Types of transactions that can occur in a pool
/// </summary>
public enum TransactionType
{
    Swap = 1,
    AddLiquidity = 2,
    RemoveLiquidity = 3,
    FeeWithdrawal = 6,
    PoolCreation = 7,
    SystemPause = 8,
    SystemUnpause = 9,
    FeeRateChange = 10,
    PoolPause = 11,
    PoolUnpause = 12
}

/// <summary>
/// Represents a transaction that occurred in a pool
/// </summary>
public class PoolTransaction
{
    [Key]
    public Guid Id { get; set; } = Guid.NewGuid();
    
    /// <summary>
    /// The pool this transaction belongs to
    /// </summary>
    [Required]
    public Guid PoolId { get; set; }
    
    /// <summary>
    /// Navigation property to the pool
    /// </summary>
    [ForeignKey(nameof(PoolId))]
    public virtual Pool Pool { get; set; } = null!;
    
    /// <summary>
    /// Type of transaction
    /// </summary>
    [Required]
    public TransactionType Type { get; set; }
    
    /// <summary>
    /// Solana transaction signature
    /// </summary>
    [Required]
    [StringLength(88)]  // Base58 signature length
    public string TransactionSignature { get; set; } = string.Empty;
    
    /// <summary>
    /// Public key of the user who initiated the transaction
    /// </summary>
    [Required]
    [StringLength(44)]
    public string UserAddress { get; set; } = string.Empty;
    
    /// <summary>
    /// Amount of TokenA involved in the transaction (0 if not applicable)
    /// </summary>
    public ulong TokenAAmount { get; set; } = 0;
    
    /// <summary>
    /// Amount of TokenB involved in the transaction (0 if not applicable)
    /// </summary>
    public ulong TokenBAmount { get; set; } = 0;
    
    /// <summary>
    /// Amount of LP tokens involved (for liquidity operations)
    /// </summary>
    public ulong LpTokenAmount { get; set; } = 0;
    
    /// <summary>
    /// Fee amount collected (for fee withdrawal transactions)
    /// </summary>
    public ulong FeeAmount { get; set; } = 0;
    
    /// <summary>
    /// Which token the fee was collected in (TokenA or TokenB)
    /// </summary>
    [StringLength(10)]
    public string FeeTokenSymbol { get; set; } = string.Empty;
    
    /// <summary>
    /// Block number when transaction was processed
    /// </summary>
    public ulong BlockNumber { get; set; } = 0;
    
    /// <summary>
    /// When the transaction was processed
    /// </summary>
    public DateTime ProcessedAt { get; set; } = DateTime.UtcNow;
    
    /// <summary>
    /// Network where transaction occurred
    /// </summary>
    [Required]
    [StringLength(20)]
    public string Network { get; set; } = "testnet";
    
    /// <summary>
    /// Whether the transaction was successful
    /// </summary>
    public bool IsSuccessful { get; set; } = true;
    
    /// <summary>
    /// Error message if transaction failed
    /// </summary>
    [StringLength(500)]
    public string? ErrorMessage { get; set; }
    
    /// <summary>
    /// Gas fees paid for this transaction (in lamports)
    /// </summary>
    public ulong GasFee { get; set; } = 0;
    
    /// <summary>
    /// Additional metadata as JSON (for complex transaction details)
    /// </summary>
    [Column(TypeName = "jsonb")]
    public string? Metadata { get; set; }
    
    /// <summary>
    /// For fee operations: the fee rate (in basis points) if this is a fee rate change
    /// </summary>
    public uint? FeeRateBasisPoints { get; set; }
    
    /// <summary>
    /// Human-readable description of the transaction
    /// </summary>
    [StringLength(200)]
    public string Description { get; set; } = string.Empty;
    
    /// <summary>
    /// Price at time of swap (TokenA per TokenB, for display purposes)
    /// </summary>
    [Column(TypeName = "decimal(20,10)")]
    public decimal? SwapPrice { get; set; }
    
    /// <summary>
    /// Pool liquidity after this transaction (TokenA)
    /// </summary>
    public ulong? PoolLiquidityTokenAAfter { get; set; }
    
    /// <summary>
    /// Pool liquidity after this transaction (TokenB)
    /// </summary>
    public ulong? PoolLiquidityTokenBAfter { get; set; }
} 