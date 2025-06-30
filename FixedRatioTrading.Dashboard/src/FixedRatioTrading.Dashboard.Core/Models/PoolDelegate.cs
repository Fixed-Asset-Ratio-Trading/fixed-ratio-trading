using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;

namespace FixedRatioTrading.Dashboard.Core.Models;

/// <summary>
/// Represents an authorized delegate for a pool who can withdraw fees
/// </summary>
public class PoolDelegate
{
    [Key]
    public Guid Id { get; set; } = Guid.NewGuid();
    
    /// <summary>
    /// The pool this delegate is authorized for
    /// </summary>
    [Required]
    public Guid PoolId { get; set; }
    
    /// <summary>
    /// Navigation property to the pool
    /// </summary>
    [ForeignKey(nameof(PoolId))]
    public virtual Pool Pool { get; set; } = null!;
    
    /// <summary>
    /// Public key of the delegate account
    /// </summary>
    [Required]
    [StringLength(44)]
    public string DelegateAddress { get; set; } = string.Empty;
    
    /// <summary>
    /// Optional display name for the delegate
    /// </summary>
    [StringLength(100)]
    public string? DisplayName { get; set; }
    
    /// <summary>
    /// Optional email contact for the delegate
    /// </summary>
    [StringLength(255)]
    public string? ContactEmail { get; set; }
    
    /// <summary>
    /// When this delegate was added
    /// </summary>
    public DateTime AddedAt { get; set; } = DateTime.UtcNow;
    
    /// <summary>
    /// Public key of the account that added this delegate (pool creator or existing delegate)
    /// </summary>
    [Required]
    [StringLength(44)]
    public string AddedByAddress { get; set; } = string.Empty;
    
    /// <summary>
    /// Whether this delegate authorization is currently active
    /// </summary>
    public bool IsActive { get; set; } = true;
    
    /// <summary>
    /// When this delegate was removed (if applicable)
    /// </summary>
    public DateTime? RemovedAt { get; set; }
    
    /// <summary>
    /// Public key of the account that removed this delegate (if applicable)
    /// </summary>
    [StringLength(44)]
    public string? RemovedByAddress { get; set; }
    
    /// <summary>
    /// Transaction signature when delegate was added
    /// </summary>
    [Required]
    [StringLength(88)]
    public string AddTransactionSignature { get; set; } = string.Empty;
    
    /// <summary>
    /// Transaction signature when delegate was removed (if applicable)
    /// </summary>
    [StringLength(88)]
    public string? RemoveTransactionSignature { get; set; }
    
    /// <summary>
    /// Total fees withdrawn by this delegate (TokenA)
    /// </summary>
    public ulong TotalFeesWithdrawnTokenA { get; set; } = 0;
    
    /// <summary>
    /// Total fees withdrawn by this delegate (TokenB)
    /// </summary>
    public ulong TotalFeesWithdrawnTokenB { get; set; } = 0;
    
    /// <summary>
    /// Number of fee withdrawal transactions by this delegate
    /// </summary>
    public int WithdrawalCount { get; set; } = 0;
    
    /// <summary>
    /// Last time this delegate withdrew fees
    /// </summary>
    public DateTime? LastWithdrawalAt { get; set; }
    
    /// <summary>
    /// Network where this delegate operates
    /// </summary>
    [Required]
    [StringLength(20)]
    public string Network { get; set; } = "testnet";
    
    /// <summary>
    /// Optional notes about this delegate
    /// </summary>
    [StringLength(500)]
    public string? Notes { get; set; }
} 