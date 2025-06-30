using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Solnet.Rpc;
using FixedRatioTrading.Dashboard.Solana.Services;

namespace FixedRatioTrading.Dashboard.Solana;

/// <summary>
/// Extension methods for configuring Solana blockchain integration services
/// </summary>
public static class DependencyInjection
{
    /// <summary>
    /// Add Solana blockchain integration services to the DI container
    /// </summary>
    /// <param name="services">Service collection</param>
    /// <param name="configuration">Application configuration</param>
    /// <returns>Service collection for chaining</returns>
    public static IServiceCollection AddSolanaIntegration(
        this IServiceCollection services,
        IConfiguration configuration)
    {
        // Configure Solana settings
        services.Configure<SolanaConfiguration>(
            configuration.GetSection("Solana"));

        // Register RPC client
        services.AddSingleton<IRpcClient>(provider =>
        {
            var config = configuration.GetSection("Solana").Get<SolanaConfiguration>()
                ?? new SolanaConfiguration();
            
            return ClientFactory.GetClient(config.RpcUrl);
        });

        // Register Solana services
        services.AddScoped<ISolanaRpcService, SolanaRpcService>();
        services.AddScoped<IPoolSyncService, PoolSyncService>();
        services.AddSingleton<IPollingService, PollingService>();

        return services;
    }

    /// <summary>
    /// Add Solana blockchain integration services with custom configuration
    /// </summary>
    /// <param name="services">Service collection</param>
    /// <param name="configureOptions">Configuration action</param>
    /// <returns>Service collection for chaining</returns>
    public static IServiceCollection AddSolanaIntegration(
        this IServiceCollection services,
        Action<SolanaConfiguration> configureOptions)
    {
        var config = new SolanaConfiguration();
        configureOptions(config);

        // Configure Solana settings
        services.Configure<SolanaConfiguration>(opts =>
        {
            opts.RpcUrl = config.RpcUrl;
            opts.Network = config.Network;
            opts.ProgramId = config.ProgramId;
            opts.SystemStateAddress = config.SystemStateAddress;
            opts.RequestTimeoutSeconds = config.RequestTimeoutSeconds;
            opts.MaxRetryAttempts = config.MaxRetryAttempts;
            opts.EnableLogging = config.EnableLogging;
        });

        // Register RPC client
        services.AddSingleton<IRpcClient>(provider =>
            ClientFactory.GetClient(config.RpcUrl));

        // Register Solana services
        services.AddScoped<ISolanaRpcService, SolanaRpcService>();
        services.AddScoped<IPoolSyncService, PoolSyncService>();
        services.AddSingleton<IPollingService, PollingService>();

        return services;
    }

    /// <summary>
    /// Add Solana hosted polling service for automatic background synchronization
    /// </summary>
    /// <param name="services">Service collection</param>
    /// <param name="configuration">Polling configuration</param>
    /// <returns>Service collection for chaining</returns>
    public static IServiceCollection AddSolanaPollingHostedService(
        this IServiceCollection services,
        PollingConfiguration? configuration = null)
    {
        configuration ??= new PollingConfiguration();

        services.AddSingleton(configuration);
        services.AddHostedService<PollingHostedService>();

        return services;
    }
} 