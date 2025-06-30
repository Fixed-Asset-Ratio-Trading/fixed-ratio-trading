using System.ComponentModel.DataAnnotations;

namespace FixedRatioTrading.Dashboard.Core.Models;

/// <summary>
/// Types of system operations
/// </summary>
public enum SystemOperationType
{
    Pause = 1,
    Unpause = 2,
    Upgrade = 3,
    EmergencyStop = 4,
    Configuration = 5
}

/// <summary>
/// Represents the current state of the trading system
/// </summary>
public class SystemState
{
    [Key]
    public Guid Id { get; set; } = Guid.NewGuid();
    
    /// <summary>
    /// Whether the system is currently paused
    /// </summary>
    public bool IsPaused { get; set; } = false;
    
    /// <summary>
    /// Whether the system is in emergency stop mode
    /// </summary>
    public bool IsEmergencyStop { get; set; } = false;
    
    /// <summary>
    /// Current system version
    /// </summary>
    [Required]
    [StringLength(20)]
    public string Version { get; set; } = "1.0.0";
    
    /// <summary>
    /// Network this system state applies to
    /// </summary>
    [Required]
    [StringLength(20)]
    public string Network { get; set; } = "testnet";
    
    /// <summary>
    /// When the system was last paused (if applicable)
    /// </summary>
    public DateTime? LastPausedAt { get; set; }
    
    /// <summary>
    /// Who paused the system (if applicable)
    /// </summary>
    [StringLength(44)]
    public string? LastPausedBy { get; set; }
    
    /// <summary>
    /// When the system was last unpaused (if applicable)
    /// </summary>
    public DateTime? LastUnpausedAt { get; set; }
    
    /// <summary>
    /// Who unpaused the system (if applicable)
    /// </summary>
    [StringLength(44)]
    public string? LastUnpausedBy { get; set; }
    
    /// <summary>
    /// When the last upgrade occurred
    /// </summary>
    public DateTime? LastUpgradeAt { get; set; }
    
    /// <summary>
    /// Who performed the last upgrade
    /// </summary>
    [StringLength(44)]
    public string? LastUpgradeBy { get; set; }
    
    /// <summary>
    /// Total number of pools in the system
    /// </summary>
    public int TotalPools { get; set; } = 0;
    
    /// <summary>
    /// Total number of active pools
    /// </summary>
    public int ActivePools { get; set; } = 0;
    
    /// <summary>
    /// Total value locked (TVL) in USD (approximate)
    /// </summary>
    public decimal? TotalValueLockedUsd { get; set; }
    
    /// <summary>
    /// Total trading volume in USD (24h)
    /// </summary>
    public decimal? Volume24hUsd { get; set; }
    
    /// <summary>
    /// Number of unique users who have interacted with the system
    /// </summary>
    public int UniqueUsers { get; set; } = 0;
    
    /// <summary>
    /// When this state record was created/updated
    /// </summary>
    public DateTime UpdatedAt { get; set; } = DateTime.UtcNow;
    
    /// <summary>
    /// Reason for last pause/emergency stop
    /// </summary>
    [StringLength(500)]
    public string? PauseReason { get; set; }
    
    /// <summary>
    /// Notes about the current system state
    /// </summary>
    [StringLength(1000)]
    public string? Notes { get; set; }
    
    /// <summary>
    /// Transaction signature of the last system operation
    /// </summary>
    [StringLength(88)]
    public string? LastOperationTxSignature { get; set; }
    
    /// <summary>
    /// Type of the last system operation
    /// </summary>
    public SystemOperationType? LastOperationType { get; set; }
    
    /// <summary>
    /// Whether the system is currently under maintenance
    /// </summary>
    public bool IsUnderMaintenance { get; set; } = false;
    
    /// <summary>
    /// Expected end time for maintenance (if applicable)
    /// </summary>
    public DateTime? MaintenanceEndTime { get; set; }
} 