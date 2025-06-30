using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;

namespace FixedRatioTrading.Dashboard.Core.Models;

/// <summary>
/// Represents a fixed-ratio trading pool between two tokens
/// </summary>
public class Pool
{
    [Key]
    public Guid Id { get; set; } = Guid.NewGuid();
    
    /// <summary>
    /// The Solana program-derived address (PDA) of this pool
    /// </summary>
    [Required]
    [StringLength(44)]  // Base58 public key length
    public string PoolAddress { get; set; } = string.Empty;
    
    /// <summary>
    /// First token in the pool (lexicographically normalized)
    /// </summary>
    [Required]
    [StringLength(44)]
    public string TokenAMint { get; set; } = string.Empty;
    
    /// <summary>
    /// Second token in the pool (lexicographically normalized)
    /// </summary>
    [Required]
    [StringLength(44)]
    public string TokenBMint { get; set; } = string.Empty;
    
    /// <summary>
    /// Token A symbol (e.g., "BTC")
    /// </summary>
    [Required]
    [StringLength(10)]
    public string TokenASymbol { get; set; } = string.Empty;
    
    /// <summary>
    /// Token B symbol (e.g., "USDC")
    /// </summary>
    [Required]
    [StringLength(10)]
    public string TokenBSymbol { get; set; } = string.Empty;
    
    /// <summary>
    /// Token A display name (e.g., "Bitcoin")
    /// </summary>
    [StringLength(50)]
    public string TokenAName { get; set; } = string.Empty;
    
    /// <summary>
    /// Token B display name (e.g., "USD Coin")
    /// </summary>
    [StringLength(50)]
    public string TokenBName { get; set; } = string.Empty;
    
    /// <summary>
    /// The numerator of the ratio for TokenA (e.g., 10000 in 10000:1)
    /// IMPORTANT: This means "RatioANumerator of TokenA per RatioBDenominator of TokenB"
    /// </summary>
    [Required]
    public ulong RatioANumerator { get; set; }
    
    /// <summary>
    /// The denominator of the ratio for TokenB (e.g., 1 in 10000:1)
    /// IMPORTANT: This means "RatioANumerator of TokenA per RatioBDenominator of TokenB"
    /// </summary>
    [Required]
    public ulong RatioBDenominator { get; set; }
    
    /// <summary>
    /// Current liquidity amount of TokenA in the pool (in smallest units, e.g., satoshis)
    /// </summary>
    public ulong TokenALiquidity { get; set; } = 0;
    
    /// <summary>
    /// Current liquidity amount of TokenB in the pool (in smallest units, e.g., lamports)
    /// </summary>
    public ulong TokenBLiquidity { get; set; } = 0;
    
    /// <summary>
    /// Total supply of LP tokens for this pool
    /// </summary>
    public ulong LpTokenSupply { get; set; } = 0;
    
    /// <summary>
    /// Address of the LP token mint for this pool
    /// </summary>
    [StringLength(44)]
    public string LpTokenMint { get; set; } = string.Empty;
    
    /// <summary>
    /// Public key of the account that created this pool
    /// </summary>
    [Required]
    [StringLength(44)]
    public string CreatorAddress { get; set; } = string.Empty;
    
    /// <summary>
    /// When this pool was created
    /// </summary>
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
    
    /// <summary>
    /// Last time pool data was updated from blockchain
    /// </summary>
    public DateTime LastUpdated { get; set; } = DateTime.UtcNow;
    
    /// <summary>
    /// Whether this pool is currently active
    /// </summary>
    public bool IsActive { get; set; } = true;
    
    /// <summary>
    /// Network where this pool exists (mainnet-beta, testnet, devnet)
    /// </summary>
    [Required]
    [StringLength(20)]
    public string Network { get; set; } = "testnet";
    
    /// <summary>
    /// Block number when pool was created
    /// </summary>
    public ulong CreationBlockNumber { get; set; } = 0;
    
    /// <summary>
    /// Transaction signature of pool creation
    /// </summary>
    [StringLength(88)]  // Base58 signature length
    public string CreationTxSignature { get; set; } = string.Empty;
    
    /// <summary>
    /// Total volume traded in this pool (TokenA)
    /// </summary>
    public ulong TotalVolumeTokenA { get; set; } = 0;
    
    /// <summary>
    /// Total volume traded in this pool (TokenB)
    /// </summary>
    public ulong TotalVolumeTokenB { get; set; } = 0;
    
    /// <summary>
    /// Number of unique liquidity providers
    /// </summary>
    public int UniqueLiquidityProviders { get; set; } = 0;
    
    /// <summary>
    /// Navigation property for pool transactions
    /// </summary>
    public virtual ICollection<PoolTransaction> Transactions { get; set; } = new List<PoolTransaction>();
} 