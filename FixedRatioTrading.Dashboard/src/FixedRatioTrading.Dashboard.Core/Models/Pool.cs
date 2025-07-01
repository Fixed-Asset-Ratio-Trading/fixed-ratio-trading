using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;

namespace FixedRatioTrading.Dashboard.Core.Models;

/// <summary>
/// Represents a fixed-ratio trading pool between two tokens
/// Updated to match the current smart contract PoolState structure
/// 
/// IMPORTANT: This model contains both user-accessible and owner-only data.
/// The dashboard ONLY supports user operations. Owner fields are READ-ONLY for display purposes.
/// All owner operations (fee management, pause controls) are handled by separate CLI application.
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
    /// Pool owner (creator) public key
    /// READ-ONLY: Dashboard displays this information but cannot modify owner
    /// </summary>
    [Required]
    [StringLength(44)]
    public string Owner { get; set; } = string.Empty;
    
    /// <summary>
    /// First token mint address (TokenA)
    /// </summary>
    [Required]
    [StringLength(44)]
    public string TokenAMint { get; set; } = string.Empty;
    
    /// <summary>
    /// Second token mint address (TokenB)
    /// </summary>
    [Required]
    [StringLength(44)]
    public string TokenBMint { get; set; } = string.Empty;
    
    /// <summary>
    /// TokenA vault PDA address
    /// </summary>
    [Required]
    [StringLength(44)]
    public string TokenAVault { get; set; } = string.Empty;
    
    /// <summary>
    /// TokenB vault PDA address
    /// </summary>
    [Required]
    [StringLength(44)]
    public string TokenBVault { get; set; } = string.Empty;
    
    /// <summary>
    /// LP Token A mint address
    /// </summary>
    [Required]
    [StringLength(44)]
    public string LpTokenAMint { get; set; } = string.Empty;
    
    /// <summary>
    /// LP Token B mint address
    /// </summary>
    [Required]
    [StringLength(44)]
    public string LpTokenBMint { get; set; } = string.Empty;
    
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
    /// Trading ratio representing how many units of TokenA per 1 unit of TokenB.
    /// Example: If ratio is 10000, this means "10000 TokenA per 1 TokenB"
    /// High ratio → TokenB is more valuable → TokenB should be base token
    /// Low ratio → TokenA is more valuable → TokenA should be base token
    /// </summary>
    [Required]
    public ulong Ratio { get; set; }
    
    /// <summary>
    /// Current total liquidity amount of TokenA in the pool (in smallest units)
    /// </summary>
    public ulong TotalTokenALiquidity { get; set; } = 0;
    
    /// <summary>
    /// Current total liquidity amount of TokenB in the pool (in smallest units)
    /// </summary>
    public ulong TotalTokenBLiquidity { get; set; } = 0;
    
    /// <summary>
    /// Pool authority bump seed for PDA derivation
    /// </summary>
    public byte PoolAuthorityBumpSeed { get; set; } = 0;
    
    /// <summary>
    /// TokenA vault bump seed for PDA derivation
    /// </summary>
    public byte TokenAVaultBumpSeed { get; set; } = 0;
    
    /// <summary>
    /// TokenB vault bump seed for PDA derivation
    /// </summary>
    public byte TokenBVaultBumpSeed { get; set; } = 0;
    
    /// <summary>
    /// Whether this pool is initialized
    /// </summary>
    public bool IsInitialized { get; set; } = false;
    
    /// <summary>
    /// Whether the pool is paused by owner
    /// READ-ONLY: Dashboard displays pause status but cannot modify (owner operation via CLI)
    /// </summary>
    public bool IsPaused { get; set; } = false;
    
    /// <summary>
    /// Pool-specific swap pause controls (separate from system pause)
    /// READ-ONLY: Dashboard displays pause status but cannot modify (owner operation via CLI)
    /// </summary>
    public bool SwapsPaused { get; set; } = false;
    
    /// <summary>
    /// Who initiated the swap pause (if any)
    /// READ-ONLY: Dashboard displays this information but cannot modify
    /// </summary>
    [StringLength(44)]
    public string? SwapsPauseInitiatedBy { get; set; }
    
    /// <summary>
    /// Unix timestamp when swaps were paused
    /// READ-ONLY: Dashboard displays this information but cannot modify
    /// </summary>
    public long SwapsPauseInitiatedTimestamp { get; set; } = 0;
    
    /// <summary>
    /// Whether automatic withdrawal protection is active
    /// READ-ONLY: Dashboard displays this information but cannot modify
    /// </summary>
    public bool WithdrawalProtectionActive { get; set; } = false;
    
    /// <summary>
    /// Collected fees in TokenA (in smallest units)
    /// READ-ONLY: Dashboard displays fee information but cannot withdraw (owner operation via CLI)
    /// </summary>
    public ulong CollectedFeesTokenA { get; set; } = 0;
    
    /// <summary>
    /// Collected fees in TokenB (in smallest units)
    /// READ-ONLY: Dashboard displays fee information but cannot withdraw (owner operation via CLI)
    /// </summary>
    public ulong CollectedFeesTokenB { get; set; } = 0;
    
    /// <summary>
    /// Total fees withdrawn in TokenA (for tracking)
    /// READ-ONLY: Dashboard displays fee history but cannot perform withdrawals
    /// </summary>
    public ulong TotalFeesWithdrawnTokenA { get; set; } = 0;
    
    /// <summary>
    /// Total fees withdrawn in TokenB (for tracking)
    /// READ-ONLY: Dashboard displays fee history but cannot perform withdrawals
    /// </summary>
    public ulong TotalFeesWithdrawnTokenB { get; set; } = 0;
    
    /// <summary>
    /// Swap fee rate in basis points (e.g., 30 = 0.3%)
    /// READ-ONLY: Dashboard displays current fee rate but cannot modify (owner operation via CLI)
    /// </summary>
    public ulong SwapFeeBasisPoints { get; set; } = 0;
    
    /// <summary>
    /// Collected SOL fees (in lamports)
    /// READ-ONLY: Dashboard displays fee information but cannot withdraw (owner operation via CLI)
    /// </summary>
    public ulong CollectedSolFees { get; set; } = 0;
    
    /// <summary>
    /// Total SOL fees withdrawn (for tracking)
    /// READ-ONLY: Dashboard displays fee history but cannot perform withdrawals
    /// </summary>
    public ulong TotalSolFeesWithdrawn { get; set; } = 0;
    
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
    
    // DEPRECATED FIELD: Kept for backward compatibility but not used
    [Obsolete("CreatorAddress is deprecated. Use Owner instead.")]
    [StringLength(44)]
    public string CreatorAddress 
    { 
        get => Owner; 
        set => Owner = value; 
    }
    
    // DEPRECATED FIELD: Kept for backward compatibility but not used
    [Obsolete("TokenALiquidity is deprecated. Use TotalTokenALiquidity instead.")]
    public ulong TokenALiquidity 
    { 
        get => TotalTokenALiquidity; 
        set => TotalTokenALiquidity = value; 
    }
    
    // DEPRECATED FIELD: Kept for backward compatibility but not used
    [Obsolete("TokenBLiquidity is deprecated. Use TotalTokenBLiquidity instead.")]
    public ulong TokenBLiquidity 
    { 
        get => TotalTokenBLiquidity; 
        set => TotalTokenBLiquidity = value; 
    }
    
    // DEPRECATED FIELD: LP token supply is not stored in contract
    [Obsolete("LpTokenSupply is deprecated. LP tokens are managed separately for TokenA and TokenB.")]
    public ulong LpTokenSupply { get; set; } = 0;
    
    // DEPRECATED FIELD: Single LP token mint not used in current design
    [Obsolete("LpTokenMint is deprecated. Use LpTokenAMint and LpTokenBMint instead.")]
    [StringLength(44)]
    public string LpTokenMint { get; set; } = string.Empty;
} 