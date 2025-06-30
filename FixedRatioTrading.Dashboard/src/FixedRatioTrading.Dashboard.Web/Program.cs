using Microsoft.EntityFrameworkCore;
using FixedRatioTrading.Dashboard.Data;
using FixedRatioTrading.Dashboard.Data.Repositories;
using FixedRatioTrading.Dashboard.Solana;
using FixedRatioTrading.Dashboard.Solana.Services;
using FixedRatioTrading.Dashboard.Web.Services;

var builder = WebApplication.CreateBuilder(args);

// Add services to the container.
builder.Services.AddControllersWithViews()
    .AddNewtonsoftJson(options =>
    {
        options.SerializerSettings.ReferenceLoopHandling = Newtonsoft.Json.ReferenceLoopHandling.Ignore;
    });

// Add API Explorer and Swagger
builder.Services.AddEndpointsApiExplorer();
builder.Services.AddSwaggerGen(c =>
{
    c.SwaggerDoc("v1", new() { 
        Title = "Fixed Ratio Trading Dashboard API", 
        Version = "v1",
        Description = "API for managing and monitoring fixed ratio trading pools on Solana"
    });
});

// Configure Entity Framework with PostgreSQL
var connectionString = builder.Configuration.GetConnectionString("DefaultConnection") 
    ?? "Host=localhost;Database=fixed_ratio_trading_dashboard;Username=postgres;Password=password";

builder.Services.AddDbContext<DashboardDbContext>(options =>
    options.UseNpgsql(connectionString));

// Register repository interfaces (we'll need to create implementations)
builder.Services.AddScoped<IPoolRepository, PoolRepository>();
builder.Services.AddScoped<IPoolTransactionRepository, PoolTransactionRepository>();
builder.Services.AddScoped<ITokenRepository, TokenRepository>();
builder.Services.AddScoped<ISystemStateRepository, SystemStateRepository>();

// Add Solana blockchain integration
builder.Services.AddSolanaIntegration(builder.Configuration);

// Register business services
builder.Services.AddScoped<IPoolService, PoolService>();

// Add background polling service (optional - can be disabled in configuration)
var enableBackgroundPolling = builder.Configuration.GetValue<bool>("BackgroundPolling:Enabled", true);
if (enableBackgroundPolling)
{
    var pollingConfig = new PollingConfiguration
    {
        PollInterval = TimeSpan.FromMinutes(builder.Configuration.GetValue<int>("BackgroundPolling:IntervalMinutes", 2)),
        PoolDiscoveryInterval = TimeSpan.FromMinutes(builder.Configuration.GetValue<int>("BackgroundPolling:DiscoveryIntervalMinutes", 10)),
        SystemStateInterval = TimeSpan.FromMinutes(builder.Configuration.GetValue<int>("BackgroundPolling:SystemStateIntervalMinutes", 5)),
        MaxTransactionsPerPool = builder.Configuration.GetValue<int>("BackgroundPolling:MaxTransactionsPerPool", 50),
        MaxConcurrentPools = builder.Configuration.GetValue<int>("BackgroundPolling:MaxConcurrentPools", 10),
        EnablePoolDiscovery = builder.Configuration.GetValue<bool>("BackgroundPolling:EnablePoolDiscovery", false),
        SyncTransactions = builder.Configuration.GetValue<bool>("BackgroundPolling:SyncTransactions", false),
        SyncSystemState = builder.Configuration.GetValue<bool>("BackgroundPolling:SyncSystemState", true),
        Network = builder.Configuration.GetValue<string>("Solana:Network", "testnet")
    };

    builder.Services.AddSolanaPollingHostedService(pollingConfig);
}

// Add CORS for development
builder.Services.AddCors(options =>
{
    options.AddPolicy("AllowAll", builder =>
    {
        builder
            .AllowAnyOrigin()
            .AllowAnyMethod()
            .AllowAnyHeader();
    });
});

// Add health checks
builder.Services.AddHealthChecks()
    .AddDbContext<DashboardDbContext>();

var app = builder.Build();

// Configure the HTTP request pipeline.
if (app.Environment.IsDevelopment())
{
    app.UseSwagger();
    app.UseSwaggerUI(c =>
    {
        c.SwaggerEndpoint("/swagger/v1/swagger.json", "Fixed Ratio Trading Dashboard API v1");
        c.RoutePrefix = "api/docs"; // Swagger UI at /api/docs
    });
    app.UseCors("AllowAll");
}
else
{
    app.UseExceptionHandler("/Home/Error");
    app.UseHsts();
}

app.UseHttpsRedirection();
app.UseRouting();
app.UseAuthorization();

// Map static assets
app.MapStaticAssets();

// Map API routes
app.MapControllers();

// Map default MVC route for web pages
app.MapControllerRoute(
    name: "default",
    pattern: "{controller=Home}/{action=Index}/{id?}")
    .WithStaticAssets();

// Map health check endpoint
app.MapHealthChecks("/health");

// Auto-migrate database on startup (development only)
if (app.Environment.IsDevelopment())
{
    using var scope = app.Services.CreateScope();
    var context = scope.ServiceProvider.GetRequiredService<DashboardDbContext>();
    try
    {
        await context.Database.MigrateAsync();
        app.Logger.LogInformation("Database migrations applied successfully");
    }
    catch (Exception ex)
    {
        app.Logger.LogError(ex, "Error applying database migrations");
    }
}

app.Logger.LogInformation("Fixed Ratio Trading Dashboard starting...");
app.Logger.LogInformation("Swagger UI available at: /api/docs");
app.Logger.LogInformation("Health check available at: /health");

app.Run();
