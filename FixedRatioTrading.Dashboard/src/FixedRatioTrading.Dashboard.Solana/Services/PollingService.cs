using Microsoft.Extensions.Logging;

namespace FixedRatioTrading.Dashboard.Solana.Services;

/// <summary>
/// Implementation of polling service for periodic blockchain synchronization
/// </summary>
public class PollingService : IPollingService
{
    private readonly IPoolSyncService _poolSyncService;
    private readonly ILogger<PollingService> _logger;
    private readonly PollingStatistics _statistics;
    
    private PollingConfiguration _configuration = new();
    private CancellationTokenSource? _cancellationTokenSource;
    private Task? _pollingTask;

    public bool IsRunning => _pollingTask != null && !_pollingTask.IsCompleted;
    public PollingConfiguration Configuration => _configuration;

    public event EventHandler<PollCompletedEventArgs>? PollCompleted;
    public event EventHandler<PollErrorEventArgs>? PollError;

    public PollingService(
        IPoolSyncService poolSyncService,
        ILogger<PollingService> logger)
    {
        _poolSyncService = poolSyncService;
        _logger = logger;
        _statistics = new PollingStatistics
        {
            ServiceStartTime = DateTime.UtcNow
        };
    }

    public async Task StartAsync(PollingConfiguration configuration, CancellationToken cancellationToken = default)
    {
        if (IsRunning)
        {
            _logger.LogWarning("Polling service is already running");
            return;
        }

        _configuration = configuration;
        _cancellationTokenSource = new CancellationTokenSource();
        
        _logger.LogInformation("Starting polling service with interval: {Interval}", configuration.PollInterval);
        
        _pollingTask = Task.Run(() => PollLoop(_cancellationTokenSource.Token), cancellationToken);
    }

    public async Task StopAsync(CancellationToken cancellationToken = default)
    {
        if (!IsRunning)
        {
            _logger.LogWarning("Polling service is not running");
            return;
        }

        _logger.LogInformation("Stopping polling service");
        
        _cancellationTokenSource?.Cancel();
        
        if (_pollingTask != null)
        {
            await _pollingTask;
        }
        
        _cancellationTokenSource?.Dispose();
        _cancellationTokenSource = null;
        _pollingTask = null;
        
        _logger.LogInformation("Polling service stopped");
    }

    public async Task TriggerPollAsync(CancellationToken cancellationToken = default)
    {
        if (!IsRunning)
        {
            _logger.LogWarning("Cannot trigger poll - service is not running");
            return;
        }

        _logger.LogInformation("Triggering immediate poll");
        await ExecutePollCycle(cancellationToken);
    }

    public async Task<PollingStatistics> GetStatisticsAsync()
    {
        _statistics.TotalRuntime = DateTime.UtcNow - _statistics.ServiceStartTime;
        _statistics.AverageCycleTime = _statistics.TotalPollCycles > 0 
            ? TimeSpan.FromMilliseconds(_statistics.TotalRuntime.TotalMilliseconds / _statistics.TotalPollCycles)
            : TimeSpan.Zero;
            
        return await Task.FromResult(_statistics);
    }

    private async Task PollLoop(CancellationToken cancellationToken)
    {
        _logger.LogInformation("Poll loop started");
        
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                await ExecutePollCycle(cancellationToken);
                await Task.Delay(_configuration.PollInterval, cancellationToken);
            }
            catch (OperationCanceledException)
            {
                _logger.LogInformation("Poll loop cancelled");
                break;
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Unexpected error in poll loop");
                _statistics.FailedCycles++;
                _statistics.ConsecutiveFailures++;
                _statistics.LastError = ex.Message;
                _statistics.LastFailedPoll = DateTime.UtcNow;
                
                OnPollError(new PollErrorEventArgs
                {
                    Timestamp = DateTime.UtcNow,
                    ErrorMessage = ex.Message,
                    Exception = ex,
                    Operation = "PollLoop",
                    RetryAttempt = 0,
                    WillRetry = !cancellationToken.IsCancellationRequested
                });
                
                if (!cancellationToken.IsCancellationRequested)
                {
                    await Task.Delay(_configuration.RetryDelay, cancellationToken);
                }
            }
        }
        
        _logger.LogInformation("Poll loop ended");
    }

    private async Task ExecutePollCycle(CancellationToken cancellationToken)
    {
        var startTime = DateTime.UtcNow;
        var eventArgs = new PollCompletedEventArgs
        {
            StartTime = startTime,
            Result = new SyncResult { Success = true }
        };

        try
        {
            _logger.LogDebug("Starting poll cycle");
            _statistics.TotalPollCycles++;

            // Sync all pools
            if (_configuration.SyncSystemState)
            {
                var poolCount = await _poolSyncService.SyncAllPoolsAsync(_configuration.Network);
                eventArgs.PoolsSynced = poolCount;
                _statistics.PoolsSynced += poolCount;
                
                _logger.LogDebug("Synced {PoolCount} pools", poolCount);
            }

            // TODO: Add system state sync when system state address is configured
            // TODO: Add transaction sync if enabled
            // TODO: Add pool discovery if enabled

            eventArgs.EndTime = DateTime.UtcNow;
            eventArgs.Duration = eventArgs.EndTime - eventArgs.StartTime;
            
            _statistics.SuccessfulCycles++;
            _statistics.ConsecutiveFailures = 0;
            _statistics.LastSuccessfulPoll = DateTime.UtcNow;
            
            _logger.LogDebug("Poll cycle completed in {Duration}", eventArgs.Duration);
            
            OnPollCompleted(eventArgs);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error during poll cycle");
            
            eventArgs.Result.Success = false;
            eventArgs.Result.ErrorMessage = ex.Message;
            eventArgs.EndTime = DateTime.UtcNow;
            eventArgs.Duration = eventArgs.EndTime - eventArgs.StartTime;
            
            _statistics.FailedCycles++;
            _statistics.ConsecutiveFailures++;
            _statistics.LastError = ex.Message;
            _statistics.LastFailedPoll = DateTime.UtcNow;
            
            OnPollError(new PollErrorEventArgs
            {
                Timestamp = DateTime.UtcNow,
                ErrorMessage = ex.Message,
                Exception = ex,
                Operation = "PollCycle",
                RetryAttempt = 0,
                WillRetry = false
            });
            
            OnPollCompleted(eventArgs);
        }
    }

    private void OnPollCompleted(PollCompletedEventArgs e)
    {
        PollCompleted?.Invoke(this, e);
    }

    private void OnPollError(PollErrorEventArgs e)
    {
        PollError?.Invoke(this, e);
    }
} 