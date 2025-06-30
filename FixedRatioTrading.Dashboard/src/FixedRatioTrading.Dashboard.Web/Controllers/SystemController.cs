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

            return Ok(new { success = true, data = systemState });
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
                status = "healthy",
                timestamp = DateTime.UtcNow,
                services = new
                {
                    database = await CheckDatabaseHealth(),
                    solanaRpc = await CheckSolanaRpcHealth(),
                    polling = CheckPollingServiceHealth()
                }
            };

            var allHealthy = health.services.database.healthy && 
                           health.services.solanaRpc.healthy && 
                           health.services.polling.healthy;

            if (!allHealthy)
            {
                return StatusCode(503, new { success = false, data = health });
            }

            return Ok(new { success = true, data = health });
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
                success = true, 
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
                success = true, 
                message = "Polling cycle triggered successfully",
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
                solana = new
                {
                    network = _solanaRpcService.Network,
                    // Don't expose sensitive configuration like RPC URLs
                },
                polling = new
                {
                    isRunning = _pollingService.IsRunning,
                    configuration = _pollingService.Configuration
                },
                version = new
                {
                    api = "1.0.0",
                    environment = Environment.GetEnvironmentVariable("ASPNETCORE_ENVIRONMENT") ?? "Unknown"
                }
            };

            return Ok(new { success = true, data = config });
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
                uptime = pollingStats.TotalRuntime,
                polling = new
                {
                    totalCycles = pollingStats.TotalPollCycles,
                    successfulCycles = pollingStats.SuccessfulCycles,
                    failedCycles = pollingStats.FailedCycles,
                    averageCycleTime = pollingStats.AverageCycleTime,
                    lastSuccessfulPoll = pollingStats.LastSuccessfulPoll,
                    consecutiveFailures = pollingStats.ConsecutiveFailures
                },
                synchronization = new
                {
                    poolsSynced = pollingStats.PoolsSynced,
                    transactionsSynced = pollingStats.TransactionsSynced,
                    newPoolsDiscovered = pollingStats.NewPoolsDiscovered
                },
                lastUpdated = DateTime.UtcNow
            };

            return Ok(new { success = true, data = metrics });
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
            return new { healthy = true, message = "Database connection successful" };
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Database health check failed");
            return new { healthy = false, message = "Database connection failed", error = ex.Message };
        }
    }

    private async Task<object> CheckSolanaRpcHealth()
    {
        try
        {
            var isHealthy = await _solanaRpcService.TestConnectionAsync();
            return new { 
                healthy = isHealthy, 
                message = isHealthy ? "Solana RPC connection successful" : "Solana RPC connection failed",
                network = _solanaRpcService.Network
            };
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Solana RPC health check failed");
            return new { healthy = false, message = "Solana RPC health check failed", error = ex.Message };
        }
    }

    private object CheckPollingServiceHealth()
    {
        try
        {
            var isRunning = _pollingService.IsRunning;
            return new { 
                healthy = isRunning, 
                message = isRunning ? "Polling service is running" : "Polling service is not running",
                isRunning = isRunning
            };
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Polling service health check failed");
            return new { healthy = false, message = "Polling service health check failed", error = ex.Message };
        }
    }
} 