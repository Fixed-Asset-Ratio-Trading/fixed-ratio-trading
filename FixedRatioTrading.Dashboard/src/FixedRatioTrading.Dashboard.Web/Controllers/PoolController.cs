using Microsoft.AspNetCore.Mvc;
using FixedRatioTrading.Dashboard.Web.Services;

namespace FixedRatioTrading.Dashboard.Web.Controllers;

/// <summary>
/// API Controller for pool operations
/// Provides endpoints for viewing, searching, and managing pools
/// </summary>
[ApiController]
[Route("api/[controller]")]
public class PoolController : ControllerBase
{
    private readonly IPoolService _poolService;
    private readonly ILogger<PoolController> _logger;

    public PoolController(IPoolService poolService, ILogger<PoolController> logger)
    {
        _poolService = poolService;
        _logger = logger;
    }

    /// <summary>
    /// Get all pools with optional filtering and pagination
    /// </summary>
    /// <param name="network">Network filter (testnet, mainnet, devnet)</param>
    /// <param name="isActive">Active status filter</param>
    /// <param name="page">Page number (default: 1)</param>
    /// <param name="pageSize">Page size (default: 20, max: 100)</param>
    /// <returns>Paginated list of pools</returns>
    [HttpGet]
    public async Task<IActionResult> GetPools(
        [FromQuery] string? network = null,
        [FromQuery] bool? isActive = null,
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 20)
    {
        try
        {
            // Validate parameters
            if (page < 1) page = 1;
            if (pageSize < 1 || pageSize > 100) pageSize = 20;

            var result = await _poolService.GetPoolsAsync(network, isActive, page, pageSize);
            
            return Ok(new
            {
                success = true,
                data = result.Pools,
                pagination = new
                {
                    currentPage = result.Page,
                    pageSize = result.PageSize,
                    totalCount = result.TotalCount,
                    totalPages = result.TotalPages
                }
            });
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error getting pools");
            return StatusCode(500, new { success = false, error = "Internal server error" });
        }
    }

    /// <summary>
    /// Get a specific pool by ID
    /// </summary>
    /// <param name="id">Pool ID</param>
    /// <returns>Pool details</returns>
    [HttpGet("{id:guid}")]
    public async Task<IActionResult> GetPool(Guid id)
    {
        try
        {
            var pool = await _poolService.GetPoolAsync(id);
            
            if (pool == null)
            {
                return NotFound(new { success = false, error = "Pool not found" });
            }

            return Ok(new { success = true, data = pool });
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error getting pool {PoolId}", id);
            return StatusCode(500, new { success = false, error = "Internal server error" });
        }
    }

    /// <summary>
    /// Get a specific pool by blockchain address
    /// </summary>
    /// <param name="address">Pool blockchain address</param>
    /// <returns>Pool details</returns>
    [HttpGet("address/{address}")]
    public async Task<IActionResult> GetPoolByAddress(string address)
    {
        try
        {
            var pool = await _poolService.GetPoolByAddressAsync(address);
            
            if (pool == null)
            {
                return NotFound(new { success = false, error = "Pool not found" });
            }

            return Ok(new { success = true, data = pool });
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error getting pool by address {PoolAddress}", address);
            return StatusCode(500, new { success = false, error = "Internal server error" });
        }
    }

    /// <summary>
    /// Search pools by token symbols or names
    /// </summary>
    /// <param name="q">Search query</param>
    /// <param name="network">Network filter (optional)</param>
    /// <param name="page">Page number (default: 1)</param>
    /// <param name="pageSize">Page size (default: 20)</param>
    /// <returns>Search results</returns>
    [HttpGet("search")]
    public async Task<IActionResult> SearchPools(
        [FromQuery] string q,
        [FromQuery] string? network = null,
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 20)
    {
        try
        {
            if (string.IsNullOrWhiteSpace(q))
            {
                return BadRequest(new { success = false, error = "Search query is required" });
            }

            // Validate parameters
            if (page < 1) page = 1;
            if (pageSize < 1 || pageSize > 100) pageSize = 20;

            var result = await _poolService.SearchPoolsAsync(q, network, page, pageSize);
            
            return Ok(new
            {
                success = true,
                data = result.Pools,
                pagination = new
                {
                    currentPage = result.Page,
                    pageSize = result.PageSize,
                    totalCount = result.TotalCount,
                    totalPages = result.TotalPages
                },
                query = q
            });
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error searching pools with query {Query}", q);
            return StatusCode(500, new { success = false, error = "Internal server error" });
        }
    }

    /// <summary>
    /// Get pool statistics
    /// </summary>
    /// <param name="network">Network filter (optional)</param>
    /// <returns>Pool statistics</returns>
    [HttpGet("statistics")]
    public async Task<IActionResult> GetStatistics([FromQuery] string? network = null)
    {
        try
        {
            var stats = await _poolService.GetPoolStatisticsAsync(network);
            return Ok(new { success = true, data = stats });
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error getting pool statistics");
            return StatusCode(500, new { success = false, error = "Internal server error" });
        }
    }

    /// <summary>
    /// Get recent transactions for a pool
    /// </summary>
    /// <param name="id">Pool ID</param>
    /// <param name="limit">Maximum number of transactions (default: 50, max: 100)</param>
    /// <returns>Recent transactions</returns>
    [HttpGet("{id:guid}/transactions")]
    public async Task<IActionResult> GetPoolTransactions(Guid id, [FromQuery] int limit = 50)
    {
        try
        {
            if (limit < 1 || limit > 100) limit = 50;

            var transactions = await _poolService.GetPoolTransactionsAsync(id, limit);
            
            return Ok(new 
            { 
                success = true, 
                data = transactions,
                poolId = id,
                limit = limit
            });
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error getting transactions for pool {PoolId}", id);
            return StatusCode(500, new { success = false, error = "Internal server error" });
        }
    }

    /// <summary>
    /// Manually sync a pool from blockchain
    /// </summary>
    /// <param name="address">Pool blockchain address</param>
    /// <returns>Sync result</returns>
    [HttpPost("sync/{address}")]
    public async Task<IActionResult> SyncPool(string address)
    {
        try
        {
            _logger.LogInformation("Manual sync requested for pool {PoolAddress}", address);
            
            var result = await _poolService.SyncPoolAsync(address);
            
            if (result.Success)
            {
                return Ok(new { success = true, data = result, message = "Pool synchronized successfully" });
            }
            else
            {
                return BadRequest(new { success = false, error = result.ErrorMessage });
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error syncing pool {PoolAddress}", address);
            return StatusCode(500, new { success = false, error = "Internal server error" });
        }
    }

    /// <summary>
    /// Get top pools by various criteria
    /// </summary>
    /// <param name="sortBy">Sort criteria: volume, liquidity, recent (default: volume)</param>
    /// <param name="count">Number of pools to return (default: 10, max: 50)</param>
    /// <param name="network">Network filter (optional)</param>
    /// <returns>Top pools</returns>
    [HttpGet("top")]
    public async Task<IActionResult> GetTopPools(
        [FromQuery] string sortBy = "volume",
        [FromQuery] int count = 10,
        [FromQuery] string? network = null)
    {
        try
        {
            if (count < 1 || count > 50) count = 10;

            var validSortOptions = new[] { "volume", "liquidity", "recent" };
            if (!validSortOptions.Contains(sortBy.ToLowerInvariant()))
            {
                sortBy = "volume";
            }

            var pools = await _poolService.GetTopPoolsAsync(sortBy, count, network);
            
            return Ok(new 
            { 
                success = true, 
                data = pools,
                sortBy = sortBy,
                count = count,
                network = network
            });
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error getting top pools");
            return StatusCode(500, new { success = false, error = "Internal server error" });
        }
    }
} 