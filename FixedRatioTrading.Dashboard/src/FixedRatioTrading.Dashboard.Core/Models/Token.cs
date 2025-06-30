using System.ComponentModel.DataAnnotations;

namespace FixedRatioTrading.Dashboard.Core.Models;

/// <summary>
/// Represents an SPL token in the system
/// </summary>
public class Token
{
    [Key]
    public Guid Id { get; set; } = Guid.NewGuid();
    
    /// <summary>
    /// The mint address of this SPL token
    /// </summary>
    [Required]
    [StringLength(44)]
    public string MintAddress { get; set; } = string.Empty;
    
    /// <summary>
    /// Token symbol (e.g., "BTC", "USDC", "TS")
    /// </summary>
    [Required]
    [StringLength(10)]
    public string Symbol { get; set; } = string.Empty;
    
    /// <summary>
    /// Full name of the token (e.g., "Bitcoin", "USD Coin", "Test Solana")
    /// </summary>
    [Required]
    [StringLength(100)]
    public string Name { get; set; } = string.Empty;
    
    /// <summary>
    /// Number of decimal places for this token
    /// </summary>
    [Required]
    public byte Decimals { get; set; } = 9;
    
    /// <summary>
    /// URL to token logo/icon
    /// </summary>
    [StringLength(500)]
    public string? LogoUrl { get; set; }
    
    /// <summary>
    /// Optional description of the token
    /// </summary>
    [StringLength(1000)]
    public string? Description { get; set; }
    
    /// <summary>
    /// Public key of the account that created this token (if created via our testnet feature)
    /// </summary>
    [StringLength(44)]
    public string? CreatorAddress { get; set; }
    
    /// <summary>
    /// When this token was created
    /// </summary>
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
    
    /// <summary>
    /// Network where this token exists
    /// </summary>
    [Required]
    [StringLength(20)]
    public string Network { get; set; } = "testnet";
    
    /// <summary>
    /// Whether this token was created via our testnet token creation feature
    /// </summary>
    public bool IsTestnetCreated { get; set; } = false;
    
    /// <summary>
    /// Transaction signature of token creation (if created via our feature)
    /// </summary>
    [StringLength(88)]
    public string? CreationTxSignature { get; set; }
    
    /// <summary>
    /// Total supply of this token
    /// </summary>
    public ulong? TotalSupply { get; set; }
    
    /// <summary>
    /// Whether this token is currently active/tradeable
    /// </summary>
    public bool IsActive { get; set; } = true;
    
    /// <summary>
    /// Tags for categorizing tokens (JSON array, e.g., ["stablecoin", "defi"])
    /// </summary>
    [StringLength(500)]
    public string? Tags { get; set; }
    
    /// <summary>
    /// Official website URL
    /// </summary>
    [StringLength(500)]
    public string? WebsiteUrl { get; set; }
    
    /// <summary>
    /// CoinGecko ID for price tracking (if available)
    /// </summary>
    [StringLength(100)]
    public string? CoinGeckoId { get; set; }
    
    /// <summary>
    /// Last known price in USD (for reference, not real-time)
    /// </summary>
    public decimal? LastKnownPriceUsd { get; set; }
    
    /// <summary>
    /// When the price was last updated
    /// </summary>
    public DateTime? PriceLastUpdated { get; set; }
    
    /// <summary>
    /// Whether this token has been verified by our system
    /// </summary>
    public bool IsVerified { get; set; } = false;
    
    /// <summary>
    /// When token metadata was last updated
    /// </summary>
    public DateTime LastUpdated { get; set; } = DateTime.UtcNow;
} 