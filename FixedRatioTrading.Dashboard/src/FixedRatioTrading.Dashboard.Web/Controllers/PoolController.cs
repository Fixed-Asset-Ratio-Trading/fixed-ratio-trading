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
    /// Results are sorted by creation date (newest to oldest)
    /// </summary>
    /// <param name="network">Network filter (testnet, mainnet, devnet)</param>
    /// <param name="isActive">Active status filter</param>
    /// <param name="page">Page number (default: 1)</param>
    /// <param name="pageSize">Page size (default: 20, max: 100)</param>
    /// <returns>Paginated list of pools sorted newest to oldest</returns>
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
                // Indicates whether the API request was processed successfully
                success = true,
                
                // Array of pool summary objects containing essential pool information
                // Each pool object contains:
                // - id: Unique identifier for the pool in the database
                // - poolAddress: The Solana program-derived address (PDA) of this pool on the blockchain
                // - tokenASymbol: Symbol of the first token in the trading pair (e.g., "BTC", "SOL")
                // - tokenBSymbol: Symbol of the second token in the trading pair (e.g., "USDC", "ETH")
                // - tokenAName: Full display name of the first token (e.g., "Bitcoin")
                // - tokenBName: Full display name of the second token (e.g., "USD Coin")
                // - ratio: Human-readable ratio string combining numerator and denominator (e.g., "10000:1")
                // - ratio: Trading ratio representing how many units of TokenA per 1 unit of TokenB
                // - totalTokenALiquidity: Current total liquidity amount of TokenA in the pool (in smallest token units, e.g., satoshis for BTC)
                // - totalTokenBLiquidity: Current total liquidity amount of TokenB in the pool (in smallest token units, e.g., micro-USDC for USDC)
                // - totalVolumeTokenA: Total trading volume of TokenA that has passed through this pool since creation
                // - totalVolumeTokenB: Total trading volume of TokenB that has passed through this pool since creation
                // - status: Simplified operational status (Operational, Inactive, SystemPaused, PoolPaused, SwapsPaused)
                // - statusDescription: Human-readable description of the current pool status
                // - createdAt: UTC timestamp when this pool was created on the blockchain
                // - lastUpdated: UTC timestamp when pool data was last synchronized from the blockchain
                // - network: Blockchain network where this pool exists (mainnet-beta, testnet, or devnet)
                data = result.Pools,
                
                // Pagination information for navigating through multiple pages of results
                pagination = new
                {
                    // Current page number being returned (1-based indexing)
                    currentPage = result.Page,
                    
                    // Number of pools returned per page (maximum 100, default 20)
                    pageSize = result.PageSize,
                    
                    // Total number of pools matching the search criteria across all pages
                    totalCount = result.TotalCount,
                    
                    // Total number of pages available based on totalCount and pageSize
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

            return Ok(new { 
                // Indicates whether the API request was processed successfully
                success = true, 
                
                // Detailed pool information object containing:
                // - id: Unique identifier for the pool in the database
                // - poolAddress: The Solana program-derived address (PDA) of this pool on the blockchain
                // - owner: Pool owner (creator) public key - READ-ONLY field for display purposes
                // - tokenAMint: First token mint address (TokenA) on the Solana blockchain
                // - tokenBMint: Second token mint address (TokenB) on the Solana blockchain
                // - tokenAVault: TokenA vault PDA address where TokenA liquidity is stored
                // - tokenBVault: TokenB vault PDA address where TokenB liquidity is stored
                // - lpTokenAMint: LP Token A mint address for liquidity providers in TokenA
                // - lpTokenBMint: LP Token B mint address for liquidity providers in TokenB
                // - tokenASymbol: Symbol of the first token (e.g., "BTC", "SOL")
                // - tokenBSymbol: Symbol of the second token (e.g., "USDC", "ETH")
                // - tokenAName: Full display name of the first token
                // - tokenBName: Full display name of the second token
                // - ratioDisplay: Human-readable ratio string (e.g., "10000:1")
                // - ratio: Trading ratio representing how many units of TokenA per 1 unit of TokenB
                // - totalTokenALiquidity: Current total liquidity amount of TokenA in the pool (smallest units)
                // - totalTokenBLiquidity: Current total liquidity amount of TokenB in the pool (smallest units)
                // - totalVolumeTokenA: Total trading volume of TokenA since pool creation
                // - totalVolumeTokenB: Total trading volume of TokenB since pool creation
                // - status: Simplified operational status (Operational, Inactive, SystemPaused, PoolPaused, SwapsPaused)
                // - statusDescription: Human-readable description of the current pool status
                // - collectedFeesTokenA: Collected fees in TokenA awaiting withdrawal by owner (READ-ONLY, in smallest units)
                // - collectedFeesTokenB: Collected fees in TokenB awaiting withdrawal by owner (READ-ONLY, in smallest units)
                // - swapFeeBasisPoints: Current swap fee rate in basis points (e.g., 30 = 0.3%) (READ-ONLY)
                // - collectedSolFees: Collected SOL fees in lamports (READ-ONLY)
                // - uniqueLiquidityProviders: Number of unique addresses that have provided liquidity to this pool
                // - createdAt: UTC timestamp when this pool was created
                // - lastUpdated: UTC timestamp when pool data was last synchronized from blockchain
                // - network: Blockchain network where this pool exists
                // - recentTransactions: Array of recent transactions for this pool (up to 10 most recent)
                //   Each transaction contains:
                //   - id: Unique identifier for the transaction in the database
                //   - type: Type of transaction (1=Swap, 2=AddLiquidity, 3=RemoveLiquidity, 7=PoolCreation)
                //   - typeDisplay: Human-readable transaction type (e.g., "Swap", "AddLiquidity")
                //   - transactionSignature: Solana transaction signature for this transaction
                //   - userAddress: Public key of the user who initiated this transaction
                //   - tokenAAmount: Amount of TokenA involved in this transaction (0 if not applicable, in smallest units)
                //   - tokenBAmount: Amount of TokenB involved in this transaction (0 if not applicable, in smallest units)
                //   - lpTokenAmount: Amount of LP tokens involved (for liquidity operations, in smallest units)
                //   - processedAt: UTC timestamp when this transaction was processed on the blockchain
                //   - isSuccessful: Whether the transaction completed successfully
                //   - errorMessage: Error message if transaction failed (null if successful)
                //   - gasFee: Gas fees paid for this transaction (in lamports)
                //   - swapPrice: Exchange rate at time of swap (TokenA per TokenB, null for non-swap transactions)
                //   - description: Human-readable description of what this transaction accomplished
                data = pool 
            });
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

            return Ok(new { 
                // Indicates whether the API request was processed successfully
                success = true, 
                
                // Detailed pool information object (same structure as Get Pool by ID response)
                // Contains all pool details including owner, token mints, vaults, LP tokens,
                // liquidity amounts, trading volumes, status information, fees, and recent transactions
                data = pool 
            });
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error getting pool by address {PoolAddress}", address);
            return StatusCode(500, new { success = false, error = "Internal server error" });
        }
    }

    /// <summary>
    /// Search pools by token symbols, names, or token pairs
    /// Supports both individual token search (e.g., "BTC") and token pair search (e.g., "BTC/USDC")
    /// Results are sorted by creation date (newest to oldest)
    /// </summary>
    /// <param name="q">Search query - supports individual tokens ("BTC") or token pairs ("BTC/USDC", "USDC/BTC")</param>
    /// <param name="network">Network filter (optional)</param>
    /// <param name="page">Page number (default: 1)</param>
    /// <param name="pageSize">Page size (default: 20)</param>
    /// <returns>Search results sorted newest to oldest</returns>
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
                // Indicates whether the search was processed successfully
                success = true,
                
                // Array of pool summary objects matching the search criteria
                // Search supports:
                // - Individual token symbols: "BTC", "USDC", "SOL"
                // - Individual token names: "Bitcoin", "USD Coin"
                // - Token pairs: "BTC/USDC", "USDC/BTC" (order independent)
                // - Pool addresses: partial or full blockchain addresses
                // Each pool has the same structure as the Get All Pools response
                data = result.Pools,
                
                // Pagination information for navigating through search results
                pagination = new
                {
                    // Current page number being returned (1-based indexing)
                    currentPage = result.Page,
                    
                    // Number of pools returned per page (maximum 100, default 20)
                    pageSize = result.PageSize,
                    
                    // Total number of pools matching the search criteria across all pages
                    totalCount = result.TotalCount,
                    
                    // Total number of pages available based on totalCount and pageSize
                    totalPages = result.TotalPages
                },
                
                // The search query that was executed against token symbols, names, and token pairs
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
            return Ok(new { 
                // Indicates whether statistics were retrieved successfully
                success = true, 
                
                // Aggregated statistics object containing:
                // - totalPools: Total number of pools across all networks (or filtered network)
                // - activePools: Number of pools currently accepting transactions
                // - pausedPools: Number of pools that are paused (either globally or swaps-only)
                // - totalValueLocked: Total value locked across all pools (sum of all token liquidity in smallest units)
                // - volume24h: Total trading volume in the last 24 hours (in smallest units)
                // - uniqueUsers24h: Number of unique user addresses that traded in the last 24 hours
                // - totalTransactions: Total number of transactions across all pools since inception
                // - lastUpdated: UTC timestamp when these statistics were last calculated
                data = stats 
            });
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
                // Indicates whether transactions were retrieved successfully
                success = true, 
                
                // Array of transaction objects for this pool, each containing:
                // - id: Unique identifier for the transaction in the database
                // - type: Type of transaction as enum value (1=Swap, 2=AddLiquidity, 3=RemoveLiquidity, 7=PoolCreation)
                // - typeDisplay: Human-readable transaction type string
                // - transactionSignature: Solana transaction signature for verification on blockchain
                // - userAddress: Public key of the user who initiated this transaction
                // - tokenAAmount: Amount of TokenA involved (0 if not applicable, in smallest token units)
                // - tokenBAmount: Amount of TokenB involved (0 if not applicable, in smallest token units)
                // - lpTokenAmount: Amount of LP tokens minted/burned (for liquidity operations, in smallest units)
                // - processedAt: UTC timestamp when transaction was processed on blockchain
                // - isSuccessful: Whether the transaction completed successfully on blockchain
                // - errorMessage: Error message if transaction failed (null if successful)
                // - gasFee: Gas fees paid for this transaction (in lamports)
                // - swapPrice: Exchange rate at time of swap (TokenA per TokenB, null for non-swap transactions)
                // - description: Human-readable description of what this transaction accomplished
                data = transactions,
                
                // The pool ID these transactions belong to
                poolId = id,
                
                // The limit parameter that was applied to this query
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
                return Ok(new { 
                    // Indicates whether the sync operation was initiated successfully
                    success = true, 
                    
                    // Sync operation result object containing:
                    // - success: Whether the blockchain sync operation completed successfully
                    // - errorMessage: Error message if sync failed (null if successful)
                    // - syncedAt: UTC timestamp when the sync operation was performed
                    // - pool: The updated pool data after sync (same structure as Get Pool by ID response)
                    data = result, 
                    
                    // Human-readable message about the sync operation result
                    message = "Pool synchronized successfully" 
                });
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
                // Indicates whether top pools were retrieved successfully
                success = true, 
                
                // Array of top pool summary objects (same structure as individual pools in Get All Pools)
                // Ordered by the specified criteria (volume, liquidity, or recent creation)
                data = pools,
                
                // The sorting criteria that was applied ("volume", "liquidity", or "recent")
                sortBy = sortBy,
                
                // The number of pools returned
                count = count,
                
                // The network filter applied (null if no filter)
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