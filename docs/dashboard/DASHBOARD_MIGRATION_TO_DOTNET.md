# Dashboard Migration to ASP.NET Core C#
**Fixed Ratio Trading - Dashboard Migration Plan**

## 🎯 **Migration Objectives**

### **Problems Solved**
- ✅ **Type Safety**: C# eliminates runtime type errors
- ✅ **Debugging**: Step-through debugging with breakpoints
- ✅ **Server-Side Rendering**: AI can inspect HTML output via curl
- ✅ **Data Persistence**: Proper database with Supabase
- ✅ **Testing**: Unit tests with familiar tooling
- ✅ **Maintainability**: Strongly-typed interfaces and models

### **Architecture Principles**
1. **Server-Side First**: All data processing and business logic on server
2. **Minimal JavaScript**: Only for UI interactions (form validation, modals, etc.)
3. **RESTful API**: Clean separation between frontend and backend
4. **Database-Driven**: Supabase as single source of truth
5. **Blockchain Polling**: Background service to sync Solana data
6. **🚨 NO OWNER OPERATIONS**: Dashboard is read-only for owner functions - all owner operations handled by separate CLI app

## ⚠️ **IMPORTANT: Security Architecture**

### **Dashboard Scope (User Functions Only)**
The dashboard will **ONLY** support user-level operations:
- ✅ **Pool Viewing**: Browse and search existing pools
- ✅ **Token Creation**: Create test tokens (testnet only)
- ✅ **Pool Creation**: Create new trading pools
- ✅ **Liquidity Management**: Add/remove liquidity as regular user
- ✅ **Token Swapping**: Execute trades between tokens

### **CLI App Scope (Owner Operations)**
**ALL owner-only operations require a separate command line application**:
- 🔑 **Fee Management**: Change fee rates and withdraw collected fees  
- 🔑 **System Pause/Unpause**: Emergency system controls
- 🔑 **Pool Management**: Pause/unpause individual pools
- 🔑 **Security Operations**: All operations requiring owner keypair

**The dashboard will NEVER have access to owner keypairs or perform owner-only operations.**

## 📋 **Migration Phases**

### **Phase 1: Infrastructure Setup (Week 1)**

#### **1.1 Project Structure**
```
FixedRatioTrading.Dashboard/
├── src/
│   ├── FixedRatioTrading.Dashboard.Web/          # ASP.NET Core Web App
│   │   ├── Controllers/                          # MVC Controllers
│   │   ├── Views/                               # Razor Views
│   │   ├── wwwroot/                             # Static files (CSS, minimal JS)
│   │   ├── Models/                              # View Models
│   │   └── Program.cs                           # Startup
│   ├── FixedRatioTrading.Dashboard.Core/         # Business Logic
│   │   ├── Services/                            # Application Services
│   │   ├── Models/                              # Domain Models
│   │   ├── Interfaces/                          # Service Contracts
│   │   └── Extensions/                          # Helper Extensions
│   ├── FixedRatioTrading.Dashboard.Data/         # Data Access
│   │   ├── Repositories/                        # Data Repositories
│   │   ├── Entities/                            # Database Entities
│   │   ├── Migrations/                          # EF Migrations
│   │   └── Context/                             # DbContext
│   └── FixedRatioTrading.Dashboard.Solana/       # Blockchain Integration
│       ├── Services/                            # Solana Web3 Services
│       ├── Models/                              # Blockchain Models
│       └── Clients/                             # RPC Clients
├── tests/
│   ├── FixedRatioTrading.Dashboard.Tests.Unit/   # Unit Tests
│   └── FixedRatioTrading.Dashboard.Tests.Integration/ # Integration Tests
└── docs/
    └── api/                                     # API Documentation
```

#### **1.2 Technology Stack**
```xml
<!-- Core Framework -->
<PackageReference Include="Microsoft.AspNetCore.App" Version="8.0.*" />
<PackageReference Include="Microsoft.EntityFrameworkCore" Version="8.0.*" />

<!-- Database -->
<PackageReference Include="Npgsql.EntityFrameworkCore.PostgreSQL" Version="8.0.*" />
<PackageReference Include="Supabase" Version="0.15.*" />

<!-- Solana Integration -->
<PackageReference Include="Solnet.Wallet" Version="6.0.*" />
<PackageReference Include="Solnet.Rpc" Version="6.0.*" />
<PackageReference Include="Solnet.Programs" Version="6.0.*" />

<!-- Background Services -->
<PackageReference Include="Microsoft.Extensions.Hosting" Version="8.0.*" />

<!-- Testing -->
<PackageReference Include="Microsoft.AspNetCore.Mvc.Testing" Version="8.0.*" />
<PackageReference Include="xunit" Version="2.4.*" />
<PackageReference Include="Moq" Version="4.20.*" />
```

#### **1.3 Supabase Database Schema**
```sql
-- Pools table (MVP)
CREATE TABLE pools (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pool_address VARCHAR(44) NOT NULL UNIQUE,
    token_a_address VARCHAR(44) NOT NULL,
    token_b_address VARCHAR(44) NOT NULL,
    token_a_symbol VARCHAR(20) NOT NULL,
    token_b_symbol VARCHAR(20) NOT NULL,
    token_a_name VARCHAR(100),
    token_b_name VARCHAR(100),
    ratio_a_numerator BIGINT NOT NULL,
    ratio_b_denominator BIGINT NOT NULL,
    token_a_liquidity BIGINT DEFAULT 0,
    token_b_liquidity BIGINT DEFAULT 0,
    creator_address VARCHAR(44) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    is_active BOOLEAN DEFAULT true,
    network VARCHAR(20) DEFAULT 'testnet' -- 'testnet' or 'mainnet'
);

-- Indexes for performance
CREATE INDEX idx_pools_created_at ON pools(created_at DESC);
CREATE INDEX idx_pools_symbols ON pools(token_a_symbol, token_b_symbol);
CREATE INDEX idx_pools_network ON pools(network);
CREATE INDEX idx_pools_active ON pools(is_active);
```

### **Phase 2: Core Models & Data Layer (Week 1-2)**

#### **2.1 Domain Models**
```csharp
// FixedRatioTrading.Dashboard.Core/Models/Pool.cs
public class Pool
{
    public Guid Id { get; set; }
    public string PoolAddress { get; set; } = string.Empty;
    public string TokenAAddress { get; set; } = string.Empty;
    public string TokenBAddress { get; set; } = string.Empty;
    public string TokenASymbol { get; set; } = string.Empty;
    public string TokenBSymbol { get; set; } = string.Empty;
    public string? TokenAName { get; set; }
    public string? TokenBName { get; set; }
    public long RatioANumerator { get; set; }
    public long RatioBDenominator { get; set; }
    public long TokenALiquidity { get; set; }
    public long TokenBLiquidity { get; set; }
    public string CreatorAddress { get; set; } = string.Empty;
    public DateTime CreatedAt { get; set; }
    public DateTime UpdatedAt { get; set; }
    public bool IsActive { get; set; } = true;
    public string Network { get; set; } = "testnet";

    // Calculated Properties (following UX_DESIGN_TOKEN_PAIR_DISPLAY.md)
    public TokenDisplayInfo GetDisplayInfo()
    {
        var tokensAPerTokenB = (decimal)RatioANumerator / RatioBDenominator;
        
        if (tokensAPerTokenB >= 1.0m)
        {
            return new TokenDisplayInfo
            {
                BaseToken = TokenBSymbol,
                QuoteToken = TokenASymbol,
                BaseLiquidity = TokenBLiquidity,
                QuoteLiquidity = TokenALiquidity,
                ExchangeRate = tokensAPerTokenB,
                DisplayPair = $"{TokenBSymbol}/{TokenASymbol}",
                RateText = $"1 {TokenBSymbol} = {tokensAPerTokenB:N2} {TokenASymbol}"
            };
        }
        else
        {
            var tokensBPerTokenA = (decimal)RatioBDenominator / RatioANumerator;
            return new TokenDisplayInfo
            {
                BaseToken = TokenASymbol,
                QuoteToken = TokenBSymbol,
                BaseLiquidity = TokenALiquidity,
                QuoteLiquidity = TokenBLiquidity,
                ExchangeRate = tokensBPerTokenA,
                DisplayPair = $"{TokenASymbol}/{TokenBSymbol}",
                RateText = $"1 {TokenASymbol} = {tokensBPerTokenA:N2} {TokenBSymbol}"
            };
        }
    }
}

public class TokenDisplayInfo
{
    public string BaseToken { get; set; } = string.Empty;
    public string QuoteToken { get; set; } = string.Empty;
    public long BaseLiquidity { get; set; }
    public long QuoteLiquidity { get; set; }
    public decimal ExchangeRate { get; set; }
    public string DisplayPair { get; set; } = string.Empty;
    public string RateText { get; set; } = string.Empty;
}
```

#### **2.2 Repository Pattern**
```csharp
// FixedRatioTrading.Dashboard.Core/Interfaces/IPoolRepository.cs
public interface IPoolRepository
{
    Task<IEnumerable<Pool>> GetAllPoolsAsync(string network = "testnet");
    Task<IEnumerable<Pool>> SearchPoolsAsync(string searchTerm, string network = "testnet");
    Task<Pool?> GetPoolByAddressAsync(string poolAddress);
    Task<Pool> CreatePoolAsync(Pool pool);
    Task<Pool> UpdatePoolAsync(Pool pool);
    Task<bool> DeletePoolAsync(Guid id);
    Task<int> ClearInactivePoolsAsync(string network = "testnet");
}
```

#### **2.3 EF Core DbContext**
```csharp
// FixedRatioTrading.Dashboard.Data/Context/DashboardDbContext.cs
public class DashboardDbContext : DbContext
{
    public DashboardDbContext(DbContextOptions<DashboardDbContext> options) : base(options) { }

    public DbSet<PoolEntity> Pools { get; set; }

    protected override void OnModelCreating(ModelBuilder modelBuilder)
    {
        modelBuilder.Entity<PoolEntity>(entity =>
        {
            entity.ToTable("pools");
            entity.HasKey(e => e.Id);
            entity.HasIndex(e => e.PoolAddress).IsUnique();
            entity.HasIndex(e => e.CreatedAt);
            entity.HasIndex(e => new { e.TokenASymbol, e.TokenBSymbol });
            
            entity.Property(e => e.Id).HasColumnName("id");
            entity.Property(e => e.PoolAddress).HasColumnName("pool_address").HasMaxLength(44);
            entity.Property(e => e.CreatedAt).HasColumnName("created_at");
            // ... other property mappings
        });
    }
}
```

### **Phase 3: Blockchain Integration (Week 2)**

#### **3.1 Solana Polling Service**
```csharp
// FixedRatioTrading.Dashboard.Solana/Services/BlockchainPollingService.cs
public class BlockchainPollingService : BackgroundService
{
    private readonly IServiceProvider _serviceProvider;
    private readonly ILogger<BlockchainPollingService> _logger;
    private readonly IConfiguration _configuration;

    public BlockchainPollingService(
        IServiceProvider serviceProvider,
        ILogger<BlockchainPollingService> logger,
        IConfiguration configuration)
    {
        _serviceProvider = serviceProvider;
        _logger = logger;
        _configuration = configuration;
    }

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        var pollInterval = _configuration.GetValue<int>("Solana:PollIntervalSeconds", 30);
        
        while (!stoppingToken.IsCancellationRequested)
        {
            try
            {
                using var scope = _serviceProvider.CreateScope();
                var poolSyncService = scope.ServiceProvider.GetRequiredService<IPoolSyncService>();
                
                await poolSyncService.SyncPoolsFromBlockchainAsync();
                
                await Task.Delay(TimeSpan.FromSeconds(pollInterval), stoppingToken);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error during blockchain polling");
                await Task.Delay(TimeSpan.FromSeconds(5), stoppingToken); // Short delay on error
            }
        }
    }
}
```

#### **3.2 Pool Sync Service**
```csharp
// FixedRatioTrading.Dashboard.Solana/Services/PoolSyncService.cs
public interface IPoolSyncService
{
    Task SyncPoolsFromBlockchainAsync();
    Task<bool> IsTestnetWithNoPoolsAsync();
}

public class PoolSyncService : IPoolSyncService
{
    private readonly IRpcClient _rpcClient;
    private readonly IPoolRepository _poolRepository;
    private readonly IConfiguration _configuration;
    private readonly ILogger<PoolSyncService> _logger;

    public async Task SyncPoolsFromBlockchainAsync()
    {
        var network = _configuration["Solana:Network"] ?? "testnet";
        var programId = _configuration["Solana:ProgramId"];
        
        // Check if testnet with no pools - clear DB if so
        if (network == "testnet" && await IsTestnetWithNoPoolsAsync())
        {
            await _poolRepository.ClearInactivePoolsAsync(network);
            _logger.LogInformation("Cleared testnet database - no active pools found");
            return;
        }

        // Fetch pools from blockchain
        var accounts = await _rpcClient.GetProgramAccountsAsync(programId);
        var pools = new List<Pool>();

        foreach (var account in accounts.Result)
        {
            try
            {
                var poolData = ParsePoolAccount(account.Account.Data);
                pools.Add(poolData);
            }
            catch (Exception ex)
            {
                _logger.LogWarning(ex, "Failed to parse pool account {Address}", account.PublicKey);
            }
        }

        // Update database
        await UpdatePoolsInDatabase(pools, network);
    }
}
```

### **Phase 4: Web Controllers & Views (Week 2-3)**

#### **4.1 Home Controller (Pool Dashboard)**
```csharp
// FixedRatioTrading.Dashboard.Web/Controllers/HomeController.cs
public class HomeController : Controller
{
    private readonly IPoolService _poolService;
    
    public async Task<IActionResult> Index(string search = "", int page = 1)
    {
        var pools = string.IsNullOrEmpty(search) 
            ? await _poolService.GetAllPoolsAsync()
            : await _poolService.SearchPoolsAsync(search);

        var viewModel = new DashboardViewModel
        {
            Pools = pools.Select(p => new PoolViewModel
            {
                Id = p.Id,
                PoolAddress = p.PoolAddress,
                DisplayInfo = p.GetDisplayInfo(),
                CreatedAt = p.CreatedAt,
                IsActive = p.IsActive
            }).ToList(),
            SearchTerm = search,
            TotalPools = pools.Count()
        };

        return View(viewModel);
    }
}
```

#### **4.2 Razor View with Server-Side Data**
```html
<!-- FixedRatioTrading.Dashboard.Web/Views/Home/Index.cshtml -->
@model DashboardViewModel

<div class="dashboard-container">
    <h1>Fixed Ratio Trading Dashboard</h1>
    
    <!-- Search Form -->
    <form method="get" class="search-form">
        <input type="text" name="search" value="@Model.SearchTerm" placeholder="Search pools by token symbol..." />
        <button type="submit">Search</button>
    </form>

    <!-- Pool Data as JavaScript Constants -->
    <script>
        window.POOL_DATA = @Html.Raw(Json.Serialize(Model.Pools));
        window.SEARCH_TERM = "@Model.SearchTerm";
    </script>

    <!-- Server-Rendered Pool Cards -->
    <div class="pools-grid">
        @foreach (var pool in Model.Pools)
        {
            <div class="pool-card" data-pool-id="@pool.Id">
                <div class="pool-header">
                    <h3>@pool.DisplayInfo.DisplayPair Pool</h3>
                    <span class="pool-status @(pool.IsActive ? "active" : "inactive")">
                        @(pool.IsActive ? "Active" : "Inactive")
                    </span>
                </div>
                
                <div class="pool-info">
                    <p class="exchange-rate">@pool.DisplayInfo.RateText</p>
                    <p class="liquidity">
                        Base: @pool.DisplayInfo.BaseLiquidity.ToString("N0") @pool.DisplayInfo.BaseToken<br>
                        Quote: @pool.DisplayInfo.QuoteLiquidity.ToString("N0") @pool.DisplayInfo.QuoteToken
                    </p>
                    <p class="created">Created: @pool.CreatedAt.ToString("MMM dd, yyyy HH:mm")</p>
                </div>
                
                <div class="pool-actions">
                    <a href="/Pool/Details/@pool.Id" class="btn btn-primary">View Details</a>
                    <a href="/Liquidity/Manage/@pool.Id" class="btn btn-secondary">Manage Liquidity</a>
                    <a href="/Swap/@pool.Id" class="btn btn-accent">Swap Tokens</a>
                </div>
            </div>
        }
    </div>
</div>

<!-- Minimal JavaScript for Interactions Only -->
<script>
    // Simple search enhancement (optional)
    document.querySelector('.search-form input').addEventListener('input', debounce(function(e) {
        // Could add live search here if needed
    }, 300));
    
    function debounce(func, wait) {
        let timeout;
        return function executedFunction(...args) {
            const later = () => {
                clearTimeout(timeout);
                func(...args);
            };
            clearTimeout(timeout);
            timeout = setTimeout(later, wait);
        };
    }
</script>
```

### **Phase 5: API Controllers (Week 3)**

#### **5.1 Pool API Controller**
```csharp
// FixedRatioTrading.Dashboard.Web/Controllers/Api/PoolsController.cs
[ApiController]
[Route("api/[controller]")]
public class PoolsController : ControllerBase
{
    private readonly IPoolService _poolService;

    [HttpGet]
    public async Task<ActionResult<IEnumerable<PoolDto>>> GetPools([FromQuery] string search = "")
    {
        var pools = string.IsNullOrEmpty(search) 
            ? await _poolService.GetAllPoolsAsync()
            : await _poolService.SearchPoolsAsync(search);

        var poolDtos = pools.Select(p => new PoolDto
        {
            Id = p.Id,
            PoolAddress = p.PoolAddress,
            DisplayPair = p.GetDisplayInfo().DisplayPair,
            ExchangeRate = p.GetDisplayInfo().RateText,
            CreatedAt = p.CreatedAt,
            IsActive = p.IsActive
        });

        return Ok(poolDtos);
    }

    [HttpGet("{id:guid}")]
    public async Task<ActionResult<PoolDetailDto>> GetPool(Guid id)
    {
        var pool = await _poolService.GetPoolByIdAsync(id);
        if (pool == null)
            return NotFound();

        var displayInfo = pool.GetDisplayInfo();
        return Ok(new PoolDetailDto
        {
            Id = pool.Id,
            PoolAddress = pool.PoolAddress,
            TokenAAddress = pool.TokenAAddress,
            TokenBAddress = pool.TokenBAddress,
            DisplayInfo = displayInfo,
            CreatedAt = pool.CreatedAt,
            IsActive = pool.IsActive
        });
    }
}
```

### **Phase 6: MVP Feature Implementation (Week 3-4)**

The Fixed Ratio Trading dashboard includes **4 user-focused MVP features** that provide a complete user experience for pool discovery, creation, and trading operations:

#### **MVP Feature 1: 🪙 Token Creation (Testnet Only)**

**Purpose**: Enable users to create new SPL tokens for testing and experimentation on Solana testnet.

**Features:**
- **Testnet Restriction**: Token creation is only available on testnet to prevent mainnet spam
- **Custom Token Properties**: Users can specify token name, symbol, decimals, and initial supply
- **Automatic Metadata**: Token metadata is automatically configured for proper display
- **Minting Authority**: User retains minting authority for test token flexibility

**ASP.NET Implementation:**
```csharp
[Route("tokens")]
public class TokenController : Controller
{
    private readonly ITokenService _tokenService;
    private readonly IConfiguration _configuration;

    [HttpGet("create")]
    public IActionResult Create()
    {
        // Only allow on testnet
        var network = _configuration["Solana:Network"];
        if (network != "testnet")
        {
            return NotFound("Token creation only available on testnet");
        }

        return View(new TokenCreationViewModel());
    }

    [HttpPost("create")]
    public async Task<IActionResult> Create(CreateTokenRequest request)
    {
        var network = _configuration["Solana:Network"];
        if (network != "testnet")
        {
            return BadRequest("Token creation only available on testnet");
        }

        try
        {
            var result = await _tokenService.CreateTokenAsync(request);
            
            return Json(new {
                success = true,
                tokenAddress = result.TokenAddress,
                transactionSignature = result.TransactionSignature,
                explorerUrl = $"https://explorer.solana.com/address/{result.TokenAddress}?cluster=testnet"
            });
        }
        catch (Exception ex)
        {
            return Json(new { success = false, error = ex.Message });
        }
    }
}
```

#### **MVP Feature 2: 🏊 Pool Creation**

**Purpose**: Enable users to create new fixed-ratio trading pools with custom token pairs.

**Features:**
- **Token Pair Selection**: Choose any two SPL tokens for the pool
- **Fixed Ratio Configuration**: Set exact exchange ratios between tokens
- **Initial Liquidity**: Optionally provide initial liquidity during creation
- **Pool Verification**: Automatic validation of token addresses and ratios

**ASP.NET Implementation:**
```csharp
[Route("pools")]
public class PoolController : Controller
{
    private readonly IPoolService _poolService;
    private readonly ITokenService _tokenService;

    [HttpGet("create")]
    public async Task<IActionResult> Create()
    {
        var tokens = await _tokenService.GetAvailableTokensAsync();
        
        var viewModel = new PoolCreationViewModel
        {
            AvailableTokens = tokens,
            SuggestedRatios = new[]
            {
                new RatioSuggestion { AToB = 1, BToA = 1, Description = "1:1 Equal Exchange" },
                new RatioSuggestion { AToB = 10, BToA = 1, Description = "10:1 High Value A" },
                new RatioSuggestion { AToB = 100, BToA = 1, Description = "100:1 Very High Value A" },
                new RatioSuggestion { AToB = 1000, BToA = 1, Description = "1000:1 Extremely High Value A" }
            }
        };

        return View(viewModel);
    }

    [HttpPost("create")]
    public async Task<IActionResult> Create(CreatePoolRequest request)
    {
        try
        {
            var result = await _poolService.CreatePoolAsync(request);
            
            return Json(new {
                success = true,
                poolAddress = result.PoolAddress,
                transactionSignature = result.TransactionSignature,
                poolId = result.PoolId,
                explorerUrl = $"https://explorer.solana.com/address/{result.PoolAddress}?cluster={request.Network}"
            });
        }
        catch (Exception ex)
        {
            return Json(new { success = false, error = ex.Message });
        }
    }

    [HttpGet("details/{id:guid}")]
    public async Task<IActionResult> Details(Guid id)
    {
        var pool = await _poolService.GetPoolByIdAsync(id);
        if (pool == null) return NotFound();

        var displayInfo = pool.GetDisplayInfo();
        var liquidityHistory = await _poolService.GetLiquidityHistoryAsync(id);
        var tradeHistory = await _poolService.GetTradeHistoryAsync(id);

        var viewModel = new PoolDetailsViewModel
        {
            Pool = pool,
            DisplayInfo = displayInfo,
            LiquidityHistory = liquidityHistory,
            TradeHistory = tradeHistory,
            CurrentUserAddress = User.GetSolanaAddress()
        };

        return View(viewModel);
    }
}
```

#### **MVP Feature 3: 💧 Liquidity Management**

**Purpose**: Enable users to provide liquidity to existing pools and earn a share of trading fees.

**Features:**
- **Add Liquidity**: Deposit tokens in correct ratios to earn LP tokens
- **Remove Liquidity**: Burn LP tokens to withdraw proportional shares
- **LP Token Tracking**: Monitor LP token balance and value
- **Fee Earnings**: Track earned fees from trading activity

**ASP.NET Implementation:**
```csharp
[Route("liquidity")]
public class LiquidityController : Controller
{
    private readonly ILiquidityService _liquidityService;
    private readonly IPoolService _poolService;

    [HttpGet("manage/{poolId:guid}")]
    public async Task<IActionResult> Manage(Guid poolId)
    {
        var pool = await _poolService.GetPoolByIdAsync(poolId);
        if (pool == null) return NotFound();

        var currentUser = User.GetSolanaAddress();
        var userLiquidity = await _liquidityService.GetUserLiquidityAsync(poolId, currentUser);
        var poolStats = await _liquidityService.GetPoolStatsAsync(poolId);

        var viewModel = new LiquidityManagementViewModel
        {
            Pool = pool,
            DisplayInfo = pool.GetDisplayInfo(),
            UserLiquidity = userLiquidity,
            PoolStats = poolStats,
            UserAddress = currentUser
        };

        return View(viewModel);
    }

    [HttpPost("add")]
    public async Task<IActionResult> AddLiquidity(AddLiquidityRequest request)
    {
        try
        {
            var result = await _liquidityService.AddLiquidityAsync(request);
            
            return Json(new {
                success = true,
                lpTokensReceived = result.LpTokensReceived,
                transactionSignature = result.TransactionSignature,
                newPoolTotalLiquidity = result.NewPoolTotalLiquidity,
                userSharePercentage = result.UserSharePercentage
            });
        }
        catch (Exception ex)
        {
            return Json(new { success = false, error = ex.Message });
        }
    }

    [HttpPost("remove")]
    public async Task<IActionResult> RemoveLiquidity(RemoveLiquidityRequest request)
    {
        try
        {
            var result = await _liquidityService.RemoveLiquidityAsync(request);
            
            return Json(new {
                success = true,
                tokensWithdrawn = result.TokensWithdrawn,
                transactionSignature = result.TransactionSignature,
                lpTokensBurned = result.LpTokensBurned,
                remainingLpTokens = result.RemainingLpTokens
            });
        }
        catch (Exception ex)
        {
            return Json(new { success = false, error = ex.Message });
        }
    }
}
```

#### **MVP Feature 4: 🔄 Token Swapping**

**Purpose**: Enable users to exchange tokens at fixed ratios with minimal slippage and predictable outcomes.

**Features:**
- **Fixed Ratio Trading**: Exchange tokens at predetermined ratios
- **Slippage Protection**: Minimal slippage due to fixed ratios
- **Swap Preview**: Calculate exact output amounts before transaction
- **Transaction History**: Track all swap activities

**ASP.NET Implementation:**
```csharp
[Route("swap")]
public class SwapController : Controller
{
    private readonly ISwapService _swapService;
    private readonly IPoolService _poolService;

    [HttpGet("{poolId:guid}")]
    public async Task<IActionResult> Index(Guid poolId)
    {
        var pool = await _poolService.GetPoolByIdAsync(poolId);
        if (pool == null) return NotFound();

        var displayInfo = pool.GetDisplayInfo();
        var currentUser = User.GetSolanaAddress();
        var userBalances = await _swapService.GetUserTokenBalancesAsync(currentUser, 
            pool.TokenAAddress, pool.TokenBAddress);

        var viewModel = new SwapViewModel
        {
            Pool = pool,
            DisplayInfo = displayInfo,
            UserBalances = userBalances,
            UserAddress = currentUser,
            MaxSlippage = 0.1m // 0.1% max slippage for fixed ratios
        };

        return View(viewModel);
    }

    [HttpPost("preview")]
    public async Task<IActionResult> PreviewSwap(PreviewSwapRequest request)
    {
        try
        {
            var preview = await _swapService.PreviewSwapAsync(request);
            
            return Json(new {
                success = true,
                inputAmount = preview.InputAmount,
                outputAmount = preview.OutputAmount,
                exchangeRate = preview.ExchangeRate,
                tradingFee = preview.TradingFee,
                minimumReceived = preview.MinimumReceived,
                priceImpact = preview.PriceImpact
            });
        }
        catch (Exception ex)
        {
            return Json(new { success = false, error = ex.Message });
        }
    }

    [HttpPost("execute")]
    public async Task<IActionResult> ExecuteSwap(ExecuteSwapRequest request)
    {
        try
        {
            var result = await _swapService.ExecuteSwapAsync(request);
            
            return Json(new {
                success = true,
                transactionSignature = result.TransactionSignature,
                inputAmount = result.InputAmount,
                outputAmount = result.OutputAmount,
                tradingFeesPaid = result.TradingFeesPaid,
                explorerUrl = $"https://explorer.solana.com/tx/{result.TransactionSignature}?cluster={request.Network}"
            });
        }
        catch (Exception ex)
        {
            return Json(new { success = false, error = ex.Message });
        }
    }

    [HttpGet("history/{poolId:guid}")]
    public async Task<IActionResult> GetSwapHistory(Guid poolId)
    {
        try
        {
            var currentUser = User.GetSolanaAddress();
            var history = await _swapService.GetUserSwapHistoryAsync(poolId, currentUser);
            
            return Json(new {
                success = true,
                swaps = history.Select(s => new {
                    date = s.SwapDate,
                    fromToken = s.FromTokenSymbol,
                    toToken = s.ToTokenSymbol,
                    fromAmount = s.FromAmount,
                    toAmount = s.ToAmount,
                    exchangeRate = s.ExchangeRate,
                    transactionSignature = s.TransactionSignature
                })
            });
        }
        catch (Exception ex)
        {
            return Json(new { success = false, error = ex.Message });
        }
    }
}
```

### **MVP Integration Benefits**

These 4 user-focused features provide a complete trading ecosystem for regular users:

**For Users:**
- 🎯 **Complete Control**: Create tokens, pools, and manage liquidity with full transparency
- 🔒 **Server-Side Security**: All validation and business logic processed securely on server
- 💰 **Revenue Generation**: Earn fees through liquidity provision with real-time calculations
- 🛡️ **Risk Management**: Fixed ratios eliminate price volatility with predictable outcomes

**Technical Benefits:**
- 🔍 **AI Debuggable**: Server-side rendering allows AI inspection via curl commands
- 🛠️ **Type Safety**: C# eliminates runtime errors with compile-time validation
- 🔬 **Step-through Debugging**: Full Visual Studio debugging capabilities
- 📊 **Database-Driven**: Persistent data storage with proper indexing and queries
- 🚀 **Performance**: Fast server-side processing with minimal client-side JavaScript

**Security Benefits:**
- 🔐 **No Owner Functions**: Dashboard cannot perform any owner-only operations
- 🔑 **Keypair Isolation**: No access to owner keypairs or sensitive operations
- 🛡️ **Read-Only for Owner Data**: Can display owner information but never modify it
- 🚨 **Separation of Concerns**: Clear boundary between user and owner operations

### **Phase 7: Configuration & Deployment (Week 4)**

#### **7.1 Configuration Setup**
```json
// appsettings.json
{
  "ConnectionStrings": {
    "DefaultConnection": "Host=db.supabase.co;Database=postgres;Username=your_user;Password=your_password"
  },
  "Supabase": {
    "Url": "https://your-project.supabase.co",
    "Key": "your-anon-key"
  },
  "Solana": {
    "Network": "testnet",
    "RpcUrl": "https://api.testnet.solana.com",
    "ProgramId": "YourProgramIdHere",
    "PollIntervalSeconds": 30
  },
  "Logging": {
    "LogLevel": {
      "Default": "Information",
      "Microsoft.AspNetCore": "Warning"
    }
  }
}
```

#### **7.2 Program.cs Setup**
```csharp
// Program.cs
var builder = WebApplication.CreateBuilder(args);

// Add services
builder.Services.AddControllersWithViews();
builder.Services.AddDbContext<DashboardDbContext>(options =>
    options.UseNpgsql(builder.Configuration.GetConnectionString("DefaultConnection")));

// Register services
builder.Services.AddScoped<IPoolRepository, PoolRepository>();
builder.Services.AddScoped<IPoolService, PoolService>();
builder.Services.AddScoped<IPoolSyncService, PoolSyncService>();

// Background services
builder.Services.AddHostedService<BlockchainPollingService>();

// Solana services
builder.Services.AddSingleton<IRpcClient>(sp =>
    ClientFactory.GetClient(builder.Configuration["Solana:RpcUrl"]));

var app = builder.Build();

// Configure pipeline
if (!app.Environment.IsDevelopment())
{
    app.UseExceptionHandler("/Home/Error");
    app.UseHsts();
}

app.UseHttpsRedirection();
app.UseStaticFiles();
app.UseRouting();

app.MapControllerRoute(
    name: "default",
    pattern: "{controller=Home}/{action=Index}/{id?}");

app.Run();
```

## 🚀 **Migration Execution Plan**

### **Week 1: Foundation**
- [ ] Create ASP.NET Core project structure
- [ ] Setup Supabase database and tables
- [ ] Implement basic domain models
- [ ] Create repository pattern and EF Core context
- [ ] Basic unit tests for models

### **Week 2: Data Integration**
- [ ] Implement Solana blockchain polling service
- [ ] Create pool synchronization logic
- [ ] Database seeding and migration scripts
- [ ] Integration tests for data layer

### **Week 3: Web Interface**
- [ ] Home controller and dashboard view
- [ ] Pool detail pages
- [ ] API controllers for REST endpoints
- [ ] Basic CSS styling (clean, professional)

### **Week 4: MVP Features**
- [ ] Token creation (testnet)
- [ ] Pool creation interface
- [ ] Liquidity management
- [ ] Swap interface

## 🧪 **Testing Strategy**

### **Unit Tests**
```csharp
[Fact]
public void Pool_GetDisplayInfo_ShouldReturnCorrectBaseToken_WhenRatioIsHigh()
{
    // Arrange
    var pool = new Pool
    {
        TokenASymbol = "MST",
        TokenBSymbol = "TS", 
        RatioANumerator = 10000,
        RatioBDenominator = 1
    };

    // Act
    var displayInfo = pool.GetDisplayInfo();

    // Assert
    Assert.Equal("TS", displayInfo.BaseToken);
    Assert.Equal("1 TS = 10,000.00 MST", displayInfo.RateText);
}
```

### **Integration Tests**
```csharp
[Fact]
public async Task HomeController_Index_ShouldReturnPoolsOrderedByDate()
{
    // Arrange
    using var factory = new WebApplicationFactory<Program>();
    var client = factory.CreateClient();

    // Act
    var response = await client.GetAsync("/");

    // Assert
    response.EnsureSuccessStatusCode();
    var content = await response.Content.ReadAsStringAsync();
    Assert.Contains("Fixed Ratio Trading Dashboard", content);
}
```

## 🔧 **Development Tools**

### **Visual Studio Setup**
- Install .NET 8 SDK
- SQL Server Management Studio for database work
- Postman for API testing
- Browser dev tools for minimal JavaScript debugging

### **Debugging Benefits**
- Breakpoints in C# code
- Watch variables and call stack
- Step through Solana integration
- Database query profiling
- HTTP request/response inspection

## 📊 **Success Metrics**

### **Technical**
- [ ] Zero JavaScript runtime errors
- [ ] <200ms average page load time
- [ ] 100% type safety on data models
- [ ] Step-through debugging working
- [ ] AI can inspect rendered HTML via curl

### **Functional**
- [ ] All 4 MVP features working
- [ ] Pools display correctly per UX design document
- [ ] Real-time blockchain data synchronization
- [ ] Search functionality working
- [ ] Responsive design on mobile

## 🚦 **Risk Mitigation**

### **Potential Issues**
1. **Supabase Connection**: Test database connectivity early
2. **Solana Integration**: Validate RPC calls in testnet first
3. **EF Migrations**: Practice schema changes in dev environment
4. **Token Display Logic**: Unit test all ratio calculations

### **Rollback Plan**
- Keep current JavaScript dashboard running on different port
- Database backup before each migration phase
- Feature flags for new functionality
- Gradual user migration with A/B testing

---

**Next Steps**: Ready to begin Phase 1? I can help you create the initial project structure and set up the Supabase database schema. 