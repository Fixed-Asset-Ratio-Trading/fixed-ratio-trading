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
/// Updated to match the current smart contract SystemState structure
/// 
/// IMPORTANT: The dashboard can only VIEW system state information.
/// All system operations (pause/unpause) are OWNER-ONLY and handled by separate CLI application.
/// This model is READ-ONLY in the dashboard context.
/// </summary>
public class SystemState
{
    [Key]
    public Guid Id { get; set; } = Guid.NewGuid();
    
    /// <summary>
    /// Authority that can pause/unpause the entire system and perform contract operations
    /// Maps to the smart contract's SystemState.authority field
    /// READ-ONLY: Dashboard displays authority information but cannot modify
    /// </summary>
    [Required]
    [StringLength(44)]
    public string Authority { get; set; } = string.Empty;
    
    /// <summary>
    /// Global pause state - when true, all operations are blocked except unpause
    /// Maps to the smart contract's SystemState.is_paused field
    /// READ-ONLY: Dashboard displays pause status but cannot modify (authority operation via CLI)
    /// </summary>
    public bool IsPaused { get; set; } = false;
    
    /// <summary>
    /// Unix timestamp when the system was paused
    /// Maps to the smart contract's SystemState.pause_timestamp field
    /// READ-ONLY: Dashboard displays pause timing but cannot modify
    /// </summary>
    public long PauseTimestamp { get; set; } = 0;
    
    /// <summary>
    /// Human-readable reason for the system pause
    /// Maps to the smart contract's SystemState.pause_reason field
    /// READ-ONLY: Dashboard displays pause reason but cannot modify
    /// </summary>
    [StringLength(200)]  // Match the smart contract's 200 byte limit
    public string PauseReason { get; set; } = string.Empty;
    
    /// <summary>
    /// Network this system state applies to
    /// </summary>
    [Required]
    [StringLength(20)]
    public string Network { get; set; } = "testnet";
    
    /// <summary>
    /// When this state record was created/updated in the dashboard
    /// </summary>
    public DateTime UpdatedAt { get; set; } = DateTime.UtcNow;
    
    /// <summary>
    /// Last sync with the blockchain
    /// </summary>
    public DateTime LastSyncAt { get; set; } = DateTime.UtcNow;
    
    /// <summary>
    /// Last synchronized slot number from blockchain
    /// </summary>
    public ulong LastSyncSlot { get; set; } = 0;
    
    /// <summary>
    /// Transaction signature of the last system operation
    /// </summary>
    [StringLength(88)]
    public string? LastOperationTxSignature { get; set; }
    
    /// <summary>
    /// Type of the last system operation
    /// </summary>
    public SystemOperationType? LastOperationType { get; set; }
    
    // CONVENIENCE METHODS
    
    /// <summary>
    /// Validates that the provided pubkey has authority to modify system state
    /// </summary>
    public bool ValidateAuthority(string authority)
    {
        return Authority.Equals(authority, StringComparison.OrdinalIgnoreCase);
    }
    
    /// <summary>
    /// Gets a user-friendly description of the current pause status
    /// </summary>
    public string GetPauseStatusDescription()
    {
        if (!IsPaused)
            return "System is operational";
            
        var pauseTime = DateTimeOffset.FromUnixTimeSeconds(PauseTimestamp);
        var duration = DateTime.UtcNow - pauseTime.DateTime;
        
        return $"System paused for {duration.TotalMinutes:F0} minutes. Reason: {PauseReason}";
    }
    
    /// <summary>
    /// Gets the pause duration in a human-readable format
    /// </summary>
    public string GetPauseDuration()
    {
        if (!IsPaused || PauseTimestamp == 0)
            return "Not paused";
            
        var pauseTime = DateTimeOffset.FromUnixTimeSeconds(PauseTimestamp);
        var duration = DateTime.UtcNow - pauseTime.DateTime;
        
        if (duration.TotalDays >= 1)
            return $"{duration.TotalDays:F0} days";
        else if (duration.TotalHours >= 1)
            return $"{duration.TotalHours:F0} hours";
        else
            return $"{duration.TotalMinutes:F0} minutes";
    }
    
    // DEPRECATED FIELDS: Kept for backward compatibility but not used in current smart contract
    
    [Obsolete("IsEmergencyStop is deprecated. Use IsPaused instead.")]
    public bool IsEmergencyStop 
    { 
        get => IsPaused; 
        set => IsPaused = value; 
    }
    
    [Obsolete("Version is not tracked in the smart contract.")]
    [StringLength(20)]
    public string Version { get; set; } = "1.0.0";
    
    [Obsolete("LastPausedAt is deprecated. Use PauseTimestamp instead.")]
    public DateTime? LastPausedAt 
    { 
        get => PauseTimestamp > 0 ? DateTimeOffset.FromUnixTimeSeconds(PauseTimestamp).DateTime : null;
        set => PauseTimestamp = value.HasValue ? ((DateTimeOffset)value.Value).ToUnixTimeSeconds() : 0; 
    }
    
    [Obsolete("LastPausedBy is deprecated. Use Authority instead.")]
    [StringLength(44)]
    public string? LastPausedBy 
    { 
        get => IsPaused ? Authority : null; 
        set { /* Ignore - use Authority field */ } 
    }
    
    [Obsolete("LastUnpausedAt is not tracked in the smart contract.")]
    public DateTime? LastUnpausedAt { get; set; }
    
    [Obsolete("LastUnpausedBy is not tracked in the smart contract.")]
    [StringLength(44)]
    public string? LastUnpausedBy { get; set; }
    
    [Obsolete("LastUpgradeAt is not tracked in the smart contract.")]
    public DateTime? LastUpgradeAt { get; set; }
    
    [Obsolete("LastUpgradeBy is not tracked in the smart contract.")]
    [StringLength(44)]
    public string? LastUpgradeBy { get; set; }
    
    [Obsolete("TotalPools is not tracked in system state. Query pool repository instead.")]
    public int TotalPools { get; set; } = 0;
    
    [Obsolete("ActivePools is not tracked in system state. Query pool repository instead.")]
    public int ActivePools { get; set; } = 0;
    
    [Obsolete("TotalValueLockedUsd is not tracked in system state. Calculate from pool data instead.")]
    public decimal? TotalValueLockedUsd { get; set; }
    
    [Obsolete("Volume24hUsd is not tracked in system state. Calculate from transaction data instead.")]
    public decimal? Volume24hUsd { get; set; }
    
    [Obsolete("UniqueUsers is not tracked in system state. Calculate from transaction data instead.")]
    public int UniqueUsers { get; set; } = 0;
    
    [Obsolete("Notes is not tracked in the smart contract.")]
    [StringLength(1000)]
    public string? Notes { get; set; }
    
    [Obsolete("IsUnderMaintenance is not tracked in the smart contract. Use IsPaused instead.")]
    public bool IsUnderMaintenance 
    { 
        get => IsPaused; 
        set => IsPaused = value; 
    }
    
    [Obsolete("MaintenanceEndTime is not tracked in the smart contract.")]
    public DateTime? MaintenanceEndTime { get; set; }
} 