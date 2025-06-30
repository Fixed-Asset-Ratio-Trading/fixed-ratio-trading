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

The Fixed Ratio Trading dashboard includes **7 comprehensive MVP features** that provide a complete user experience for pool management and trading operations:

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
    private readonly ISolanaService _solanaService;

    [HttpGet("create")]
    public async Task<IActionResult> Create()
    {
        // Server-side validation and network check
        var isTestnet = await _solanaService.IsTestnetAsync();
        if (!isTestnet)
            return RedirectToAction("NotAvailable");

        var viewModel = new CreateTokenViewModel
        {
            IsTestnet = isTestnet,
            RecommendedDecimals = 6,
            MaxSupply = 1_000_000_000,
            NetworkInfo = await _solanaService.GetNetworkInfoAsync()
        };

        return View(viewModel);
    }

    [HttpPost("create")]
    public async Task<IActionResult> Create(CreateTokenRequest request)
    {
        // Server-side validation
        if (!await _solanaService.IsTestnetAsync())
            return BadRequest("Token creation only available on testnet");

        var validationResult = await _tokenService.ValidateTokenCreationAsync(request);
        if (!validationResult.IsValid)
        {
            foreach (var error in validationResult.Errors)
                ModelState.AddModelError("", error);
            return View(request);
        }

        try
        {
            var result = await _tokenService.CreateTokenAsync(request);
            
            // Server-side success data preparation
            var successViewModel = new TokenCreationSuccessViewModel
            {
                TokenAddress = result.TokenAddress,
                TokenSymbol = request.Symbol,
                TokenName = request.Name,
                InitialSupply = request.InitialSupply,
                TransactionSignature = result.TransactionSignature,
                ExplorerUrl = _solanaService.GetExplorerUrl(result.TransactionSignature)
            };

            return View("CreateSuccess", successViewModel);
        }
        catch (Exception ex)
        {
            ModelState.AddModelError("", $"Token creation failed: {ex.Message}");
            return View(request);
        }
    }
}
```

**Razor View Example:**
```html
@model CreateTokenViewModel

<div class="token-creation-container">
    <h1>Create Test Token</h1>
    
    <!-- Server-rendered network status -->
    <div class="network-status @(Model.IsTestnet ? "testnet" : "mainnet")">
        <span>Network: @(Model.IsTestnet ? "Testnet" : "Mainnet")</span>
        @if (!Model.IsTestnet)
        {
            <p class="error">Token creation is only available on testnet</p>
        }
    </div>

    @if (Model.IsTestnet)
    {
        <form method="post" class="token-form">
            <div class="form-group">
                <label for="Name">Token Name</label>
                <input type="text" name="Name" placeholder="Test Solana" maxlength="50" required />
            </div>
            
            <div class="form-group">
                <label for="Symbol">Token Symbol</label>
                <input type="text" name="Symbol" placeholder="TS" maxlength="10" required />
            </div>
            
            <div class="form-group">
                <label for="Decimals">Decimals</label>
                <select name="Decimals">
                    <option value="6" selected>6 (Recommended)</option>
                    <option value="9">9 (SOL-like)</option>
                    <option value="0">0 (Whole numbers)</option>
                </select>
            </div>
            
            <div class="form-group">
                <label for="InitialSupply">Initial Supply</label>
                <input type="number" name="InitialSupply" value="1000000" max="@Model.MaxSupply" required />
            </div>
            
            <button type="submit" class="btn btn-primary">Create Token</button>
        </form>
    }
</div>

<!-- Minimal JavaScript for form validation only -->
<script>
    // Simple client-side validation enhancement
    document.querySelector('.token-form').addEventListener('submit', function(e) {
        const symbol = document.querySelector('input[name="Symbol"]').value;
        if (symbol.length < 2) {
            alert('Token symbol must be at least 2 characters');
            e.preventDefault();
        }
    });
</script>
```

#### **MVP Feature 2: 🏊 Pool Creation**

**Purpose**: Enable users to create new fixed-ratio trading pools between any two SPL tokens.

**ASP.NET Implementation:**
```csharp
[Route("pools")]
public class PoolController : Controller
{
    private readonly IPoolService _poolService;
    private readonly ITokenService _tokenService;
    private readonly ISolanaService _solanaService;

    [HttpGet("create")]
    public async Task<IActionResult> Create()
    {
        // Server-side data preparation
        var availableTokens = await _tokenService.GetAvailableTokensAsync();
        var networkInfo = await _solanaService.GetNetworkInfoAsync();
        
        var viewModel = new CreatePoolViewModel
        {
            AvailableTokens = availableTokens,
            NetworkInfo = networkInfo,
            RecommendedRatios = GetRecommendedRatios(),
            PoolCreationFee = 1.15m, // 1.15 SOL
            EstimatedTransactionFee = 0.001m
        };

        return View(viewModel);
    }

    [HttpPost("create")]
    public async Task<IActionResult> Create(CreatePoolRequest request)
    {
        // Server-side validation
        var validationResult = await _poolService.ValidatePoolCreationAsync(request);
        if (!validationResult.IsValid)
        {
            foreach (var error in validationResult.Errors)
                ModelState.AddModelError("", error);
            
            // Re-populate form data
            var viewModel = await PrepareCreateViewModelAsync();
            return View(viewModel);
        }

        try
        {
            var poolResult = await _poolService.CreatePoolAsync(request);
            
            // Server-side success data with display info
            var pool = await _poolService.GetPoolByAddressAsync(poolResult.PoolAddress);
            var displayInfo = pool.GetDisplayInfo();
            
            var successViewModel = new PoolCreationSuccessViewModel
            {
                PoolAddress = poolResult.PoolAddress,
                DisplayPair = displayInfo.DisplayPair,
                ExchangeRate = displayInfo.RateText,
                TransactionSignature = poolResult.TransactionSignature,
                CreationFeesPaid = 1.15m,
                ExplorerUrl = _solanaService.GetExplorerUrl(poolResult.TransactionSignature)
            };

            return View("CreateSuccess", successViewModel);
        }
        catch (Exception ex)
        {
            ModelState.AddModelError("", $"Pool creation failed: {ex.Message}");
            var viewModel = await PrepareCreateViewModelAsync();
            return View(viewModel);
        }
    }
}
```

#### **MVP Feature 3: 💧 Liquidity Management (Add & Remove)**

**Purpose**: Enable users to provide liquidity to pools and earn LP tokens representing their share.

**ASP.NET Implementation:**
```csharp
[Route("liquidity")]
public class LiquidityController : Controller
{
    private readonly IPoolService _poolService;
    private readonly ILiquidityService _liquidityService;

    [HttpGet("manage/{poolId:guid}")]
    public async Task<IActionResult> Manage(Guid poolId)
    {
        var pool = await _poolService.GetPoolByIdAsync(poolId);
        if (pool == null) return NotFound();

        var userAddress = User.GetSolanaAddress(); // Extension method
        var userLpBalance = await _liquidityService.GetUserLpBalanceAsync(poolId, userAddress);
        var poolStats = await _liquidityService.GetPoolStatsAsync(poolId);
        var displayInfo = pool.GetDisplayInfo();

        var viewModel = new LiquidityManagementViewModel
        {
            Pool = pool,
            DisplayInfo = displayInfo,
            UserLpBalance = userLpBalance,
            UserPoolPercentage = poolStats.TotalLpSupply > 0 
                ? (decimal)userLpBalance / poolStats.TotalLpSupply * 100 
                : 0,
            PoolStats = poolStats,
            TransactionFee = 0.0013m, // 0.0013 SOL
            MinimumDeposit = 1000 // Minimum tokens for deposit
        };

        return View(viewModel);
    }

    [HttpPost("add-liquidity")]
    public async Task<IActionResult> AddLiquidity(AddLiquidityRequest request)
    {
        var validationResult = await _liquidityService.ValidateAddLiquidityAsync(request);
        if (!validationResult.IsValid)
        {
            return Json(new { success = false, errors = validationResult.Errors });
        }

        try
        {
            var result = await _liquidityService.AddLiquidityAsync(request);
            
            return Json(new { 
                success = true, 
                lpTokensReceived = result.LpTokensReceived,
                transactionSignature = result.TransactionSignature,
                newPoolPercentage = result.NewPoolPercentage
            });
        }
        catch (Exception ex)
        {
            return Json(new { success = false, error = ex.Message });
        }
    }

    [HttpPost("remove-liquidity")]
    public async Task<IActionResult> RemoveLiquidity(RemoveLiquidityRequest request)
    {
        try
        {
            var result = await _liquidityService.RemoveLiquidityAsync(request);
            
            return Json(new { 
                success = true, 
                tokensReturned = result.TokensReturned,
                transactionSignature = result.TransactionSignature
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

**Purpose**: Enable users to exchange tokens at the pool's fixed exchange rate.

**ASP.NET Implementation:**
```csharp
[Route("swap")]
public class SwapController : Controller
{
    private readonly IPoolService _poolService;
    private readonly ISwapService _swapService;

    [HttpGet("{poolId:guid?}")]
    public async Task<IActionResult> Index(Guid? poolId)
    {
        var pools = await _poolService.GetAllActivePoolsAsync();
        var selectedPool = poolId.HasValue 
            ? pools.FirstOrDefault(p => p.Id == poolId.Value)
            : pools.FirstOrDefault();

        var viewModel = new SwapViewModel
        {
            Pools = pools.Select(p => new PoolSelectItem
            {
                Id = p.Id,
                DisplayPair = p.GetDisplayInfo().DisplayPair,
                ExchangeRate = p.GetDisplayInfo().RateText
            }).ToList(),
            SelectedPool = selectedPool,
            ContractFee = 0.0000125m, // 0.0000125 SOL
            MaxSlippage = 0.5m // 0.5%
        };

        return View(viewModel);
    }

    [HttpPost("calculate")]
    public async Task<IActionResult> CalculateSwap(SwapCalculationRequest request)
    {
        try
        {
            var calculation = await _swapService.CalculateSwapAsync(request);
            
            return Json(new {
                success = true,
                outputAmount = calculation.OutputAmount,
                exchangeRate = calculation.ExchangeRate,
                tradingFee = calculation.TradingFee,
                contractFee = calculation.ContractFee,
                finalAmount = calculation.FinalAmount,
                priceImpact = 0 // Always 0 for fixed ratio
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
        var validationResult = await _swapService.ValidateSwapAsync(request);
        if (!validationResult.IsValid)
        {
            return Json(new { success = false, errors = validationResult.Errors });
        }

        try
        {
            var result = await _swapService.ExecuteSwapAsync(request);
            
            return Json(new {
                success = true,
                outputAmount = result.OutputAmount,
                transactionSignature = result.TransactionSignature,
                explorerUrl = _solanaService.GetExplorerUrl(result.TransactionSignature)
            });
        }
        catch (Exception ex)
        {
            return Json(new { success = false, error = ex.Message });
        }
    }
}
```

#### **MVP Feature 5: 👥 Delegate Management (Add & Remove)**

**Purpose**: Enable pool owners to authorize trusted delegates for fee management and pool governance.

**ASP.NET Implementation:**
```csharp
[Route("delegates")]
public class DelegateController : Controller
{
    private readonly IDelegateService _delegateService;
    private readonly IPoolService _poolService;

    [HttpGet("manage/{poolId:guid}")]
    public async Task<IActionResult> Manage(Guid poolId)
    {
        var pool = await _poolService.GetPoolByIdAsync(poolId);
        if (pool == null) return NotFound();

        var currentUser = User.GetSolanaAddress();
        if (pool.CreatorAddress != currentUser)
            return Forbid("Only pool owner can manage delegates");

        var delegates = await _delegateService.GetPoolDelegatesAsync(poolId);
        var pendingActions = await _delegateService.GetPendingActionsAsync(poolId);

        var viewModel = new DelegateManagementViewModel
        {
            Pool = pool,
            Delegates = delegates,
            PendingActions = pendingActions,
            MaxDelegates = 3,
            CanAddMore = delegates.Count < 3,
            DefaultTimeLimits = new DelegateTimeLimits
            {
                FeeChangeWaitTime = 3600,  // 1 hour
                WithdrawWaitTime = 86400,  // 24 hours
                PauseWaitTime = 300        // 5 minutes
            }
        };

        return View(viewModel);
    }

    [HttpPost("add")]
    public async Task<IActionResult> AddDelegate(AddDelegateRequest request)
    {
        try
        {
            var result = await _delegateService.AddDelegateAsync(request);
            
            return Json(new {
                success = true,
                delegateAddress = result.DelegateAddress,
                transactionSignature = result.TransactionSignature,
                message = "Delegate added successfully"
            });
        }
        catch (Exception ex)
        {
            return Json(new { success = false, error = ex.Message });
        }
    }

    [HttpPost("remove")]
    public async Task<IActionResult> RemoveDelegate(RemoveDelegateRequest request)
    {
        try
        {
            var result = await _delegateService.RemoveDelegateAsync(request);
            
            return Json(new {
                success = true,
                cancelledActions = result.CancelledActionCount,
                transactionSignature = result.TransactionSignature,
                message = $"Delegate removed. {result.CancelledActionCount} pending actions cancelled."
            });
        }
        catch (Exception ex)
        {
            return Json(new { success = false, error = ex.Message });
        }
    }

    [HttpPost("configure-limits")]
    public async Task<IActionResult> ConfigureTimeLimits(ConfigureTimeLimitsRequest request)
    {
        try
        {
            var result = await _delegateService.ConfigureTimeLimitsAsync(request);
            
            return Json(new {
                success = true,
                transactionSignature = result.TransactionSignature,
                message = "Time limits updated successfully"
            });
        }
        catch (Exception ex)
        {
            return Json(new { success = false, error = ex.Message });
        }
    }
}
```

#### **MVP Feature 6: 💰 Fee Withdrawal (Delegates Only)**

**Purpose**: Enable authorized delegates to withdraw collected trading fees from pools.

**ASP.NET Implementation:**
```csharp
[Route("fees")]
public class FeeController : Controller
{
    private readonly IFeeService _feeService;
    private readonly IDelegateService _delegateService;

    [HttpGet("withdraw/{poolId:guid}")]
    public async Task<IActionResult> Withdraw(Guid poolId)
    {
        var currentUser = User.GetSolanaAddress();
        var isDelegate = await _delegateService.IsAuthorizedDelegateAsync(poolId, currentUser);
        
        if (!isDelegate)
            return Forbid("Only authorized delegates can access fee withdrawal");

        var availableFees = await _feeService.GetAvailableFeesAsync(poolId);
        var withdrawalHistory = await _feeService.GetWithdrawalHistoryAsync(poolId);
        var pendingWithdrawals = await _feeService.GetPendingWithdrawalsAsync(poolId, currentUser);

        var viewModel = new FeeWithdrawalViewModel
        {
            PoolId = poolId,
            AvailableFees = availableFees,
            WithdrawalHistory = withdrawalHistory,
            PendingWithdrawals = pendingWithdrawals,
            DelegateAddress = currentUser,
            MinimumWithdrawal = 1000 // Minimum tokens for withdrawal
        };

        return View(viewModel);
    }

    [HttpPost("request-withdrawal")]
    public async Task<IActionResult> RequestWithdrawal(RequestWithdrawalRequest request)
    {
        try
        {
            var result = await _feeService.RequestWithdrawalAsync(request);
            
            return Json(new {
                success = true,
                actionId = result.ActionId,
                waitTime = result.WaitTimeSeconds,
                executeAfter = result.ExecuteAfter,
                message = $"Withdrawal requested. Can execute after {result.ExecuteAfter:yyyy-MM-dd HH:mm:ss}"
            });
        }
        catch (Exception ex)
        {
            return Json(new { success = false, error = ex.Message });
        }
    }

    [HttpPost("execute-withdrawal")]
    public async Task<IActionResult> ExecuteWithdrawal(ExecuteWithdrawalRequest request)
    {
        try
        {
            var result = await _feeService.ExecuteWithdrawalAsync(request);
            
            return Json(new {
                success = true,
                withdrawnAmount = result.WithdrawnAmount,
                tokenSymbol = result.TokenSymbol,
                transactionSignature = result.TransactionSignature,
                message = $"Successfully withdrew {result.WithdrawnAmount} {result.TokenSymbol}"
            });
        }
        catch (Exception ex)
        {
            return Json(new { success = false, error = ex.Message });
        }
    }

    [HttpGet("history/{poolId:guid}")]
    public async Task<IActionResult> GetWithdrawalHistory(Guid poolId)
    {
        try
        {
            var history = await _feeService.GetWithdrawalHistoryAsync(poolId);
            
            return Json(new {
                success = true,
                history = history.Select(h => new {
                    date = h.WithdrawalDate,
                    delegate = h.DelegateAddress,
                    amount = h.Amount,
                    token = h.TokenSymbol,
                    transactionSignature = h.TransactionSignature
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

#### **MVP Feature 7: ⏸️ System Pause → Upgrade → Unpause Process**

**Purpose**: Enable system-wide emergency pause, contract upgrades, and controlled system resumption.

**ASP.NET Implementation:**
```csharp
[Route("system")]
[Authorize(Policy = "SystemAuthority")] // Custom authorization policy
public class SystemController : Controller
{
    private readonly ISystemService _systemService;
    private readonly ISolanaService _solanaService;

    [HttpGet("status")]
    public async Task<IActionResult> Status()
    {
        var systemStatus = await _systemService.GetSystemStatusAsync();
        var networkInfo = await _solanaService.GetNetworkInfoAsync();

        var viewModel = new SystemStatusViewModel
        {
            IsSystemPaused = systemStatus.IsPaused,
            PauseReason = systemStatus.PauseReason,
            PausedAt = systemStatus.PausedAt,
            PausedBy = systemStatus.PausedBy,
            NetworkInfo = networkInfo,
            ActivePools = systemStatus.ActivePoolCount,
            TotalTransactions = systemStatus.TotalTransactions,
            SystemVersion = systemStatus.ContractVersion
        };

        return View(viewModel);
    }

    [HttpPost("pause")]
    public async Task<IActionResult> PauseSystem(PauseSystemRequest request)
    {
        try
        {
            var result = await _systemService.PauseSystemAsync(request);
            
            return Json(new {
                success = true,
                pauseReason = request.Reason,
                transactionSignature = result.TransactionSignature,
                pausedAt = DateTime.UtcNow,
                message = "System successfully paused. All operations are now blocked."
            });
        }
        catch (Exception ex)
        {
            return Json(new { success = false, error = ex.Message });
        }
    }

    [HttpPost("unpause")]
    public async Task<IActionResult> UnpauseSystem()
    {
        try
        {
            var result = await _systemService.UnpauseSystemAsync();
            
            return Json(new {
                success = true,
                transactionSignature = result.TransactionSignature,
                unpausedAt = DateTime.UtcNow,
                message = "System successfully unpaused. All operations are now available."
            });
        }
        catch (Exception ex)
        {
            return Json(new { success = false, error = ex.Message });
        }
    }

    [HttpGet("upgrade-status")]
    public async Task<IActionResult> GetUpgradeStatus()
    {
        try
        {
            var upgradeStatus = await _systemService.GetUpgradeStatusAsync();
            
            return Json(new {
                success = true,
                isUpgradeInProgress = upgradeStatus.IsUpgradeInProgress,
                currentVersion = upgradeStatus.CurrentVersion,
                targetVersion = upgradeStatus.TargetVersion,
                upgradeStarted = upgradeStatus.UpgradeStarted,
                estimatedCompletion = upgradeStatus.EstimatedCompletion
            });
        }
        catch (Exception ex)
        {
            return Json(new { success = false, error = ex.Message });
        }
    }
}
```

**System Status Razor View:**
```html
@model SystemStatusViewModel

<div class="system-status-container">
    <h1>System Management</h1>
    
    <!-- Server-rendered system status -->
    <div class="status-card @(Model.IsSystemPaused ? "paused" : "active")">
        <h2>System Status: @(Model.IsSystemPaused ? "PAUSED" : "ACTIVE")</h2>
        
        @if (Model.IsSystemPaused)
        {
            <div class="pause-info">
                <p><strong>Reason:</strong> @Model.PauseReason</p>
                <p><strong>Paused At:</strong> @Model.PausedAt?.ToString("yyyy-MM-dd HH:mm:ss UTC")</p>
                <p><strong>Paused By:</strong> @Model.PausedBy</p>
            </div>
            
            <button id="unpause-btn" class="btn btn-success">UNPAUSE SYSTEM</button>
        }
        else
        {
            <div class="active-info">
                <p><strong>Active Pools:</strong> @Model.ActivePools</p>
                <p><strong>Network:</strong> @Model.NetworkInfo.Network</p>
                <p><strong>Version:</strong> @Model.SystemVersion</p>
            </div>
            
            <div class="emergency-controls">
                <input type="text" id="pause-reason" placeholder="Emergency pause reason..." maxlength="200" />
                <button id="pause-btn" class="btn btn-danger">EMERGENCY PAUSE</button>
            </div>
        }
    </div>
</div>

<!-- Minimal JavaScript for emergency controls -->
<script>
    // Server-side data available as constants
    const SYSTEM_STATUS = {
        isPaused: @Model.IsSystemPaused.ToString().ToLower(),
        pauseReason: '@Model.PauseReason',
        activeControls: true
    };

    // Emergency pause handler
    document.getElementById('pause-btn')?.addEventListener('click', async function() {
        const reason = document.getElementById('pause-reason').value.trim();
        if (!reason) {
            alert('Please provide a reason for the emergency pause');
            return;
        }
        
        if (!confirm(`EMERGENCY PAUSE: ${reason}\n\nThis will immediately halt ALL system operations. Continue?`)) {
            return;
        }
        
        try {
            const response = await fetch('/system/pause', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ reason: reason })
            });
            
            const result = await response.json();
            if (result.success) {
                alert('System paused successfully');
                window.location.reload();
            } else {
                alert('Failed to pause system: ' + result.error);
            }
        } catch (error) {
            alert('Error: ' + error.message);
        }
    });

    // Unpause handler
    document.getElementById('unpause-btn')?.addEventListener('click', async function() {
        if (!confirm('UNPAUSE SYSTEM\n\nThis will restore all system operations. Continue?')) {
            return;
        }
        
        try {
            const response = await fetch('/system/unpause', { method: 'POST' });
            const result = await response.json();
            
            if (result.success) {
                alert('System unpaused successfully');
                window.location.reload();
            } else {
                alert('Failed to unpause system: ' + result.error);
            }
        } catch (error) {
            alert('Error: ' + error.message);
        }
    });
</script>
```

### **MVP Integration Benefits**

These 7 comprehensive features provide a complete ecosystem for decentralized trading with server-side reliability:

**For Users:**
- 🎯 **Complete Control**: Create tokens, pools, and manage liquidity with full transparency
- 🔒 **Server-Side Security**: All validation and business logic processed securely on server
- 💰 **Revenue Generation**: Earn fees through liquidity provision with real-time calculations
- 🛡️ **Risk Management**: Fixed ratios eliminate price volatility with predictable outcomes

**For Pool Owners:**
- 👥 **Delegation Management**: Authorize trusted operators with configurable permissions
- 🏛️ **Complete Governance**: Control fee rates, delegate permissions, and pool operations
- 💸 **Fee Collection**: Transparent revenue generation through trading fees
- ⏸️ **Emergency Control**: Pause individual pools during disputes with audit trails

**For System Operators:**
- 🚨 **Emergency Response**: System-wide pause capability with immediate effect
- 🔄 **Safe Upgrades**: Protected upgrade process with pause protection
- 📊 **Complete Visibility**: Comprehensive monitoring, audit trails, and real-time status
- 🛡️ **Security First**: Multiple validation layers, time delays, and server-side processing

**Technical Benefits:**
- 🔍 **AI Debuggable**: Server-side rendering allows AI inspection via curl commands
- 🛠️ **Type Safety**: C# eliminates runtime errors with compile-time validation
- 🔬 **Step-through Debugging**: Full Visual Studio debugging capabilities
- 📊 **Database-Driven**: Persistent data storage with proper indexing and queries
- 🚀 **Performance**: Fast server-side processing with minimal client-side JavaScript

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
- [ ] Delegate management
- [ ] System pause/unpause

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
- [ ] All 7 MVP features working
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