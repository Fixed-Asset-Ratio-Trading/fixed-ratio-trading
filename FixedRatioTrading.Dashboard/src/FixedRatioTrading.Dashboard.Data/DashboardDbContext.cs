using Microsoft.EntityFrameworkCore;
using FixedRatioTrading.Dashboard.Core.Models;

namespace FixedRatioTrading.Dashboard.Data;

/// <summary>
/// Entity Framework DbContext for the Fixed Ratio Trading Dashboard
/// Configured for PostgreSQL (Supabase)
/// </summary>
public class DashboardDbContext : DbContext
{
    public DashboardDbContext(DbContextOptions<DashboardDbContext> options) : base(options)
    {
    }

    // DbSets for all domain models
    public DbSet<Pool> Pools { get; set; } = null!;
    public DbSet<PoolTransaction> PoolTransactions { get; set; } = null!;
    public DbSet<Token> Tokens { get; set; } = null!;
    public DbSet<SystemState> SystemStates { get; set; } = null!;

    protected override void OnModelCreating(ModelBuilder modelBuilder)
    {
        base.OnModelCreating(modelBuilder);

        // Configure Pool entity
        modelBuilder.Entity<Pool>(entity =>
        {
            entity.HasKey(e => e.Id);
            entity.HasIndex(e => e.PoolAddress).IsUnique();
            entity.HasIndex(e => new { e.TokenAMint, e.TokenBMint }).IsUnique();
            entity.HasIndex(e => e.Network);
            entity.HasIndex(e => e.IsActive);
            entity.HasIndex(e => e.CreatedAt);
            
            // Configure relationships
            entity.HasMany(e => e.Transactions)
                  .WithOne(e => e.Pool)
                  .HasForeignKey(e => e.PoolId)
                  .OnDelete(DeleteBehavior.Cascade);
        });

        // Configure PoolTransaction entity
        modelBuilder.Entity<PoolTransaction>(entity =>
        {
            entity.HasKey(e => e.Id);
            entity.HasIndex(e => e.TransactionSignature).IsUnique();
            entity.HasIndex(e => e.UserAddress);
            entity.HasIndex(e => e.Type);
            entity.HasIndex(e => e.ProcessedAt);
            entity.HasIndex(e => e.Network);
            entity.HasIndex(e => e.IsSuccessful);
            entity.HasIndex(e => new { e.PoolId, e.ProcessedAt });
            
            // Configure enum conversion
            entity.Property(e => e.Type)
                  .HasConversion<int>();
        });

        // Configure Token entity
        modelBuilder.Entity<Token>(entity =>
        {
            entity.HasKey(e => e.Id);
            entity.HasIndex(e => e.MintAddress).IsUnique();
            entity.HasIndex(e => e.Symbol);
            entity.HasIndex(e => e.Network);
            entity.HasIndex(e => e.IsActive);
            entity.HasIndex(e => e.IsTestnetCreated);
            entity.HasIndex(e => e.IsVerified);
        });

        // Configure SystemState entity
        modelBuilder.Entity<SystemState>(entity =>
        {
            entity.HasKey(e => e.Id);
            entity.HasIndex(e => e.Network).IsUnique();
            entity.HasIndex(e => e.IsPaused);
            entity.HasIndex(e => e.IsEmergencyStop);
            entity.HasIndex(e => e.UpdatedAt);
            
            // Configure enum conversion
            entity.Property(e => e.LastOperationType)
                  .HasConversion<int>();
        });

        // Configure table names (PostgreSQL naming convention)
        modelBuilder.Entity<Pool>().ToTable("pools");
        modelBuilder.Entity<PoolTransaction>().ToTable("pool_transactions");
        modelBuilder.Entity<Token>().ToTable("tokens");
        modelBuilder.Entity<SystemState>().ToTable("system_states");
    }

    /// <summary>
    /// Override SaveChanges to automatically update LastUpdated timestamps
    /// </summary>
    public override int SaveChanges()
    {
        UpdateTimestamps();
        return base.SaveChanges();
    }

    /// <summary>
    /// Override SaveChangesAsync to automatically update LastUpdated timestamps
    /// </summary>
    public override async Task<int> SaveChangesAsync(CancellationToken cancellationToken = default)
    {
        UpdateTimestamps();
        return await base.SaveChangesAsync(cancellationToken);
    }

    /// <summary>
    /// Updates LastUpdated timestamps for entities that support it
    /// </summary>
    private void UpdateTimestamps()
    {
        var entries = ChangeTracker.Entries()
            .Where(e => e.State == EntityState.Added || e.State == EntityState.Modified);

        foreach (var entry in entries)
        {
            if (entry.Entity is Pool pool)
            {
                if (entry.State == EntityState.Added)
                    pool.CreatedAt = DateTime.UtcNow;
                pool.LastUpdated = DateTime.UtcNow;
            }
            else if (entry.Entity is Token token)
            {
                if (entry.State == EntityState.Added)
                    token.CreatedAt = DateTime.UtcNow;
                token.LastUpdated = DateTime.UtcNow;
            }
            else if (entry.Entity is SystemState systemState)
            {
                systemState.UpdatedAt = DateTime.UtcNow;
            }
        }
    }
} 