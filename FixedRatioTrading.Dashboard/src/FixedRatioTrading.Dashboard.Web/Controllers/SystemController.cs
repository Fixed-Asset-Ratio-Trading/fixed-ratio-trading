using Microsoft.AspNetCore.Mvc;
using FixedRatioTrading.Dashboard.Data.Repositories;
using FixedRatioTrading.Dashboard.Solana.Services;

namespace FixedRatioTrading.Dashboard.Web.Controllers;

/// <summary>
/// API Controller for system status and monitoring
/// </summary>
[ApiController]
[Route("api/[controller]")]
public class SystemController : ControllerBase
{
    private readonly ISystemStateRepository _systemStateRepository;
    private readonly IPollingService _pollingService;
    private readonly ISolanaRpcService _solanaRpcService;
    private readonly ILogger<SystemController> _logger;

    public SystemController(
        ISystemStateRepository systemStateRepository,
        IPollingService pollingService,
        ISolanaRpcService solanaRpcService,
        ILogger<SystemController> logger)
    {
        _systemStateRepository = systemStateRepository;
        _pollingService = pollingService;
        _solanaRpcService = solanaRpcService;
        _logger = logger;
    }

    /// <summary>
    /// Get system state for a specific network
    /// </summary>
    /// <param name="network">Network name (testnet, mainnet, devnet)</param>
    /// <returns>System state information</returns>
    [HttpGet("state/{network}")]
    public async Task<IActionResult> GetSystemState(string network)
    {
        try
        {
            var systemState = await _systemStateRepository.GetByNetworkAsync(network);
            
            if (systemState == null)
            {
                return NotFound(new { success = false, error = $"System state not found for network: {network}" });
            }

            return Ok(new { 
                // Indicates whether system state was retrieved successfully
                success = true, 
                
                // System state information object containing:
                // - id: Unique identifier for this system state record in the database
                // - authority: Public key of the authority that can pause/unpause the system (READ-ONLY)
                // - isPaused: Whether the entire system is currently paused (READ-ONLY)
                // - pauseTimestamp: Unix timestamp when system was paused (0 if not paused, READ-ONLY)
                // - pauseReason: Human-readable reason for system pause (empty if not paused, READ-ONLY)
                // - network: Blockchain network this system state applies to
                // - updatedAt: UTC timestamp when this state record was last updated in dashboard
                // - lastSyncAt: UTC timestamp of last synchronization with blockchain
                // - lastSyncSlot: Last blockchain slot number that was synchronized
                // - lastOperationTxSignature: Transaction signature of the last system operation (null if none)
                // - lastOperationType: Type of last system operation (null if none, values: 1=Pause, 2=Unpause, 3=Upgrade, 4=EmergencyStop, 5=Configuration)
                data = systemState 
            });
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error getting system state for network {Network}", network);
            return StatusCode(500, new { success = false, error = "Internal server error" });
        }
    }

    /// <summary>
    /// Get system health status
    /// </summary>
    /// <returns>Health check results</returns>
    [HttpGet("health")]
    public async Task<IActionResult> GetHealth()
    {
        try
        {
            var health = new
            {
                // Overall system health status ("healthy", "degraded", or "unhealthy")
                status = "healthy",
                
                // UTC timestamp when health check was performed
                timestamp = DateTime.UtcNow,
                
                // Individual service health status
                services = new
                {
                    // Database connectivity health
                    database = await CheckDatabaseHealth(),
                    
                    // Solana RPC connection health
                    solanaRpc = await CheckSolanaRpcHealth(),
                    
                    // Polling service health
                    polling = CheckPollingServiceHealth()
                }
            };

            // Check each service health individually
            var dbHealth = (dynamic)health.services.database;
            var rpcHealth = (dynamic)health.services.solanaRpc;
            var pollingHealth = (dynamic)health.services.polling;
            
            var allHealthy = (bool)dbHealth.healthy && 
                           (bool)rpcHealth.healthy && 
                           (bool)pollingHealth.healthy;

            if (!allHealthy)
            {
                return StatusCode(503, new { success = false, data = health });
            }

            return Ok(new { 
                // Indicates whether health check completed (may be false if critical failure)
                success = true, 
                
                // Health status information object containing:
                // - status: Overall system health status ("healthy", "degraded", or "unhealthy")
                // - timestamp: UTC timestamp when health check was performed
                // - services: Individual service health status object containing:
                //   - database: Database connectivity health with fields:
                //     - healthy: Whether database connection is working
                //     - message: Human-readable status message
                //   - solanaRpc: Solana RPC connection health with fields:
                //     - healthy: Whether Solana RPC connection is working
                //     - message: Human-readable status message
                //     - network: Which network the RPC is connected to
                //   - polling: Polling service health with fields:
                //     - healthy: Whether polling service is running properly
                //     - message: Human-readable status message
                //     - isRunning: Whether the polling service is currently active
                data = health 
            });
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error checking system health");
            return StatusCode(500, new { 
                success = false, 
                error = "Health check failed",
                timestamp = DateTime.UtcNow
            });
        }
    }

    /// <summary>
    /// Get polling service statistics
    /// </summary>
    /// <returns>Polling statistics</returns>
    [HttpGet("polling/statistics")]
    public async Task<IActionResult> GetPollingStatistics()
    {
        try
        {
            var stats = await _pollingService.GetStatisticsAsync();
            
            return Ok(new { 
                // Indicates whether polling statistics were retrieved successfully
                success = true, 
                
                // Polling service statistics object containing:
                // - isRunning: Whether the polling service is currently running
                // - configuration: Polling service configuration with fields:
                //   - pollInterval: How often the polling service runs (TimeSpan format)
                //   - batchSize: Number of pools processed in each batch
                //   - concurrentRequests: Number of concurrent requests allowed
                // - statistics: Detailed runtime statistics with fields:
                //   - totalPollCycles: Total number of polling cycles completed since service start
                //   - successfulCycles: Number of polling cycles that completed successfully
                //   - failedCycles: Number of polling cycles that failed
                //   - averageCycleTime: Average time each polling cycle takes (TimeSpan format)
                //   - lastSuccessfulPoll: UTC timestamp of last successful polling cycle
                //   - consecutiveFailures: Number of consecutive failed cycles (0 indicates healthy)
                //   - poolsSynced: Total number of pools synchronized since service start
                //   - transactionsSynced: Total number of transactions synchronized since service start
                //   - newPoolsDiscovered: Number of new pools discovered and added since service start
                //   - totalRuntime: Total runtime since polling service started (TimeSpan format)
                data = new
                {
                    isRunning = _pollingService.IsRunning,
                    configuration = _pollingService.Configuration,
                    statistics = stats
                }
            });
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error getting polling statistics");
            return StatusCode(500, new { success = false, error = "Internal server error" });
        }
    }

    /// <summary>
    /// Trigger manual polling cycle
    /// </summary>
    /// <returns>Trigger result</returns>
    [HttpPost("polling/trigger")]
    public async Task<IActionResult> TriggerPolling()
    {
        try
        {
            if (!_pollingService.IsRunning)
            {
                return BadRequest(new { success = false, error = "Polling service is not running" });
            }

            await _pollingService.TriggerPollAsync();
            
            return Ok(new { 
                // Indicates whether the polling trigger was successful
                success = true, 
                
                // Human-readable message about the operation
                message = "Polling cycle triggered successfully",
                
                // UTC timestamp when the trigger was initiated
                timestamp = DateTime.UtcNow
            });
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error triggering polling");
            return StatusCode(500, new { success = false, error = "Internal server error" });
        }
    }

    /// <summary>
    /// Get system configuration
    /// </summary>
    /// <returns>System configuration (safe values only)</returns>
    [HttpGet("config")]
    public IActionResult GetConfiguration()
    {
        try
        {
            var config = new
            {
                // Solana blockchain configuration
                solana = new
                {
                    // Which Solana network the system is connected to
                    network = _solanaRpcService.Network,
                    // Don't expose sensitive configuration like RPC URLs
                },
                
                // Polling service configuration
                polling = new
                {
                    // Whether polling service is currently running
                    isRunning = _pollingService.IsRunning,
                    
                    // Polling configuration details
                    configuration = _pollingService.Configuration
                },
                
                // Version and environment information
                version = new
                {
                    // API version string
                    api = "1.0.0",
                    
                    // Runtime environment (Development, Staging, Production)
                    environment = Environment.GetEnvironmentVariable("ASPNETCORE_ENVIRONMENT") ?? "Unknown"
                }
            };

            return Ok(new { 
                // Indicates whether configuration was retrieved successfully
                success = true, 
                
                // System configuration object (sensitive values excluded) containing:
                // - solana: Solana blockchain configuration with fields:
                //   - network: Which Solana network the system is connected to
                // - polling: Polling service configuration with fields:
                //   - isRunning: Whether polling service is currently running
                //   - configuration: Polling configuration details with pollInterval and batchSize
                // - version: Version and environment information with fields:
                //   - api: API version string
                //   - environment: Runtime environment (Development, Staging, Production)
                data = config 
            });
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error getting system configuration");
            return StatusCode(500, new { success = false, error = "Internal server error" });
        }
    }

    /// <summary>
    /// Get system metrics and statistics
    /// </summary>
    /// <returns>System metrics</returns>
    [HttpGet("metrics")]
    public async Task<IActionResult> GetMetrics()
    {
        try
        {
            var pollingStats = await _pollingService.GetStatisticsAsync();
            
            var metrics = new
            {
                // Total time the system has been running (TimeSpan format)
                uptime = pollingStats.TotalRuntime,
                
                // Polling service performance metrics
                polling = new
                {
                    // Total polling cycles completed
                    totalCycles = pollingStats.TotalPollCycles,
                    
                    // Number of successful polling cycles
                    successfulCycles = pollingStats.SuccessfulCycles,
                    
                    // Number of failed polling cycles
                    failedCycles = pollingStats.FailedCycles,
                    
                    // Average time per polling cycle (TimeSpan format)
                    averageCycleTime = pollingStats.AverageCycleTime,
                    
                    // UTC timestamp of last successful poll
                    lastSuccessfulPoll = pollingStats.LastSuccessfulPoll,
                    
                    // Current consecutive failure count (0 = healthy)
                    consecutiveFailures = pollingStats.ConsecutiveFailures
                },
                
                // Data synchronization metrics
                synchronization = new
                {
                    // Total pools synchronized from blockchain
                    poolsSynced = pollingStats.PoolsSynced,
                    
                    // Total transactions synchronized from blockchain
                    transactionsSynced = pollingStats.TransactionsSynced,
                    
                    // New pools discovered and added to database
                    newPoolsDiscovered = pollingStats.NewPoolsDiscovered
                },
                
                // UTC timestamp when these metrics were calculated
                lastUpdated = DateTime.UtcNow
            };

            return Ok(new { 
                // Indicates whether metrics were retrieved successfully
                success = true, 
                
                // System performance metrics object containing:
                // - uptime: Total time the system has been running (TimeSpan format)
                // - polling: Polling service performance metrics with fields:
                //   - totalCycles: Total polling cycles completed
                //   - successfulCycles: Number of successful polling cycles
                //   - failedCycles: Number of failed polling cycles
                //   - averageCycleTime: Average time per polling cycle (TimeSpan format)
                //   - lastSuccessfulPoll: UTC timestamp of last successful poll
                //   - consecutiveFailures: Current consecutive failure count (0 = healthy)
                // - synchronization: Data synchronization metrics with fields:
                //   - poolsSynced: Total pools synchronized from blockchain
                //   - transactionsSynced: Total transactions synchronized from blockchain
                //   - newPoolsDiscovered: New pools discovered and added to database
                // - lastUpdated: UTC timestamp when these metrics were calculated
                data = metrics 
            });
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error getting system metrics");
            return StatusCode(500, new { success = false, error = "Internal server error" });
        }
    }

    private async Task<object> CheckDatabaseHealth()
    {
        try
        {
            // Simple database connectivity check
            await _systemStateRepository.GetAllAsync();
            return new { 
                // Whether database connection is working
                healthy = true, 
                
                // Human-readable status message
                message = "Database connection successful" 
            };
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Database health check failed");
            return new { 
                // Whether database connection is working
                healthy = false, 
                
                // Human-readable status message
                message = "Database connection failed", 
                
                // Error details for debugging
                error = ex.Message 
            };
        }
    }

    private async Task<object> CheckSolanaRpcHealth()
    {
        try
        {
            var isHealthy = await _solanaRpcService.TestConnectionAsync();
            return new { 
                // Whether Solana RPC connection is working
                healthy = isHealthy, 
                
                // Human-readable status message
                message = isHealthy ? "Solana RPC connection successful" : "Solana RPC connection failed",
                
                // Which network the RPC is connected to
                network = _solanaRpcService.Network
            };
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Solana RPC health check failed");
            return new { 
                // Whether Solana RPC connection is working
                healthy = false, 
                
                // Human-readable status message
                message = "Solana RPC health check failed", 
                
                // Error details for debugging
                error = ex.Message 
            };
        }
    }

    private object CheckPollingServiceHealth()
    {
        try
        {
            var isRunning = _pollingService.IsRunning;
            return new { 
                // Whether polling service is running properly
                healthy = isRunning, 
                
                // Human-readable status message
                message = isRunning ? "Polling service is running" : "Polling service is not running",
                
                // Whether the polling service is currently active
                isRunning = isRunning
            };
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Polling service health check failed");
            return new { 
                // Whether polling service is running properly
                healthy = false, 
                
                // Human-readable status message
                message = "Polling service health check failed", 
                
                // Error details for debugging
                error = ex.Message 
            };
        }
    }
} 