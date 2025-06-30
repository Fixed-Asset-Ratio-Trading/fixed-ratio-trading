using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace FixedRatioTrading.Dashboard.Solana.Services;

/// <summary>
/// Hosted service wrapper for the polling service
/// Manages the polling service lifecycle within the ASP.NET Core application
/// </summary>
public class PollingHostedService : BackgroundService
{
    private readonly IPollingService _pollingService;
    private readonly PollingConfiguration _configuration;
    private readonly ILogger<PollingHostedService> _logger;

    public PollingHostedService(
        IPollingService pollingService,
        PollingConfiguration configuration,
        ILogger<PollingHostedService> logger)
    {
        _pollingService = pollingService;
        _configuration = configuration;
        _logger = logger;
        
        // Subscribe to polling events for logging
        _pollingService.PollCompleted += OnPollCompleted;
        _pollingService.PollError += OnPollError;
    }

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        _logger.LogInformation("Starting Solana polling hosted service");

        try
        {
            await _pollingService.StartAsync(_configuration, stoppingToken);
            
            // Keep the service running until cancellation is requested
            while (!stoppingToken.IsCancellationRequested)
            {
                await Task.Delay(TimeSpan.FromMinutes(1), stoppingToken);
            }
        }
        catch (OperationCanceledException)
        {
            _logger.LogInformation("Solana polling hosted service cancelled");
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Unexpected error in Solana polling hosted service");
        }
        finally
        {
            _logger.LogInformation("Stopping Solana polling hosted service");
            await _pollingService.StopAsync(stoppingToken);
        }
    }

    public override async Task StopAsync(CancellationToken cancellationToken)
    {
        _logger.LogInformation("Solana polling hosted service stop requested");
        
        // Unsubscribe from events
        _pollingService.PollCompleted -= OnPollCompleted;
        _pollingService.PollError -= OnPollError;
        
        await base.StopAsync(cancellationToken);
    }

    private void OnPollCompleted(object? sender, PollCompletedEventArgs e)
    {
        if (e.Result.Success)
        {
            _logger.LogInformation(
                "Poll completed successfully - Pools: {PoolsSynced}, Transactions: {TransactionsSynced}, Duration: {Duration}",
                e.PoolsSynced,
                e.TransactionsSynced,
                e.Duration);
        }
        else
        {
            _logger.LogWarning(
                "Poll completed with errors - Duration: {Duration}, Error: {Error}",
                e.Duration,
                e.Result.ErrorMessage);
        }
    }

    private void OnPollError(object? sender, PollErrorEventArgs e)
    {
        _logger.LogError(
            e.Exception,
            "Polling error in operation {Operation} - {ErrorMessage} (Retry: {WillRetry})",
            e.Operation,
            e.ErrorMessage,
            e.WillRetry);
    }
} 