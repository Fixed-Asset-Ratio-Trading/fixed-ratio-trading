# Fixed Ratio Trading - Owner CLI Application
**Secure Command Line Interface for Owner-Only Operations**

## 🎯 **Overview**

### **Purpose**
The Owner CLI Application provides secure, keypair-based access to all owner-only operations for the Fixed Ratio Trading system. This separation ensures that the web dashboard cannot perform sensitive operations, maintaining security through keypair isolation.

### **Security Architecture**
- 🔑 **Keypair Required**: All operations require the `fixed_ratio_trading-keypair.json` file
- 🔐 **Local Execution**: Runs locally with direct blockchain access
- 🚨 **No Web Exposure**: Never exposed to web interfaces or remote access
- 🛡️ **Single Purpose**: Exclusively for owner operations

## 🏗️ **Application Architecture**

### **Technology Stack**
```xml
<!-- .NET 8 Console Application -->
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
    <AssemblyName>frt-owner-cli</AssemblyName>
  </PropertyGroup>

  <ItemGroup>
    <!-- Solana Integration -->
    <PackageReference Include="Solnet.Wallet" Version="6.0.*" />
    <PackageReference Include="Solnet.Rpc" Version="6.0.*" />
    <PackageReference Include="Solnet.Programs" Version="6.0.*" />
    
    <!-- CLI Framework -->
    <PackageReference Include="System.CommandLine" Version="2.0.*" />
    
    <!-- Configuration -->
    <PackageReference Include="Microsoft.Extensions.Configuration" Version="8.0.*" />
    <PackageReference Include="Microsoft.Extensions.Configuration.Json" Version="8.0.*" />
    
    <!-- Logging -->
    <PackageReference Include="Microsoft.Extensions.Logging" Version="8.0.*" />
    <PackageReference Include="Microsoft.Extensions.Logging.Console" Version="8.0.*" />
  </ItemGroup>
</Project>
```

### **Project Structure**
```
FixedRatioTrading.OwnerCli/
├── Commands/
│   ├── FeeCommands.cs             # Fee withdrawal operations
│   ├── SystemCommands.cs          # System pause/unpause
│   ├── PoolCommands.cs            # Pool owner operations
│   └── BaseCommand.cs             # Common functionality
├── Services/
│   ├── SolanaService.cs           # Blockchain interactions
│   ├── KeypairService.cs          # Keypair management
│   ├── ValidatorService.cs        # Operation validation
│   └── LoggingService.cs          # Audit logging
├── Models/
│   ├── CommandResults.cs          # Operation results
│   ├── FeeModels.cs               # Fee operation data structures
│   └── SystemModels.cs            # System operation models
├── Utils/
│   ├── ConfigurationHelper.cs     # Configuration management
│   └── SecurityHelper.cs          # Security utilities
├── Program.cs                     # CLI entry point
├── appsettings.json              # Configuration
└── README.md                     # Usage instructions
```

## 📋 **Command Categories**

### **1. Fee Management Commands**

#### **Withdraw Pool Fees (Owner)**
```bash
frt-owner-cli fees withdraw \
  --pool-address "5xyz...abc" \
  --amount 1000 \
  --token SOL \
  --keypair "./fixed_ratio_trading-keypair.json" \
  --network testnet
```

**Features:**
- Direct fee withdrawal (immediate, owner-only)
- Support for SOL and SPL token fees
- Automatic balance validation
- Maintains pool rent exemption

#### **View Available Fees**
```bash
frt-owner-cli fees view \
  --pool-address "5xyz...abc" \
  --keypair "./fixed_ratio_trading-keypair.json" \
  --network testnet
```

**Features:**
- Display available fees by token type
- Show fee accumulation history
- Calculate potential yields
- Export fee reports

#### **Set Fee Rates**
```bash
frt-owner-cli fees set-rate \
  --pool-address "5xyz...abc" \
  --swap-fee-percentage 0.25 \
  --keypair "./fixed_ratio_trading-keypair.json" \
  --network testnet
```

**Features:**
- Set swap fee rates (0% to 0.5%)
- Immediate effect (no wait time)
- Validation of fee ranges
- Historical fee rate tracking

### **2. System Management Commands**

#### **Emergency System Pause**
```bash
frt-owner-cli system pause \
  --reason "Security incident detected" \
  --keypair "./fixed_ratio_trading-keypair.json" \
  --network testnet
```

**Features:**
- Immediate system-wide pause
- Required reason for audit purposes
- All operations blocked until unpause
- Emergency contact notifications

#### **System Unpause**
```bash
frt-owner-cli system unpause \
  --keypair "./fixed_ratio_trading-keypair.json" \
  --network testnet
```

**Features:**
- Restore all system operations
- Validation of system state before unpause
- Automatic notification of unpause
- Resume operation logging

#### **System Status**
```bash
frt-owner-cli system status \
  --keypair "./fixed_ratio_trading-keypair.json" \
  --network testnet
```

**Features:**
- Current system pause status
- Active pool count and statistics
- Recent system events
- Network health information

### **3. Pool Owner Operations**

#### **Pause Individual Pool**
```bash
frt-owner-cli pool pause \
  --pool-address "5xyz...abc" \
  --reason "Pool maintenance" \
  --keypair "./fixed_ratio_trading-keypair.json" \
  --network testnet
```

#### **Unpause Individual Pool**
```bash
frt-owner-cli pool unpause \
  --pool-address "5xyz...abc" \
  --keypair "./fixed_ratio_trading-keypair.json" \
  --network testnet
```

#### **Pool Information**
```bash
frt-owner-cli pool info \
  --pool-address "5xyz...abc" \
  --keypair "./fixed_ratio_trading-keypair.json" \
  --network testnet
```

**Features:**
- Detailed pool statistics
- Owner information and settings
- Recent pool activities
- Financial summaries

## 🔧 **Implementation Details**

### **Program.cs - CLI Entry Point**
```csharp
using System.CommandLine;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using FixedRatioTrading.OwnerCli.Commands;
using FixedRatioTrading.OwnerCli.Services;

var rootCommand = new RootCommand("Fixed Ratio Trading - Owner CLI Application");

// Build dependency injection container
var services = new ServiceCollection()
    .AddLogging(builder => builder.AddConsole())
    .AddSingleton<ISolanaService, SolanaService>()
    .AddSingleton<IKeypairService, KeypairService>()
    .AddSingleton<IValidatorService, ValidatorService>()
    .AddSingleton<ILoggingService, LoggingService>()
    .BuildServiceProvider();

// Register command categories
var feeCommands = new FeeCommands(services);
var systemCommands = new SystemCommands(services);
var poolCommands = new PoolCommands(services);

// Add all commands to root
rootCommand.AddCommand(feeCommands.CreateCommands());
rootCommand.AddCommand(systemCommands.CreateCommands());
rootCommand.AddCommand(poolCommands.CreateCommands());

// Execute CLI
return await rootCommand.InvokeAsync(args);
```

### **Base Command Structure**
```csharp
public abstract class BaseCommand
{
    protected readonly ISolanaService _solanaService;
    protected readonly IKeypairService _keypairService;
    protected readonly IValidatorService _validatorService;
    protected readonly ILoggingService _loggingService;

    protected BaseCommand(IServiceProvider services)
    {
        _solanaService = services.GetRequiredService<ISolanaService>();
        _keypairService = services.GetRequiredService<IKeypairService>();
        _validatorService = services.GetRequiredService<IValidatorService>();
        _loggingService = services.GetRequiredService<ILoggingService>();
    }

    protected async Task<bool> ValidateOwnershipAsync(string poolAddress, string keypairPath, string network)
    {
        var keypair = await _keypairService.LoadKeypairAsync(keypairPath);
        var poolInfo = await _solanaService.GetPoolInfoAsync(poolAddress, network);
        
        if (poolInfo.Owner != keypair.PublicKey.Key)
        {
            Console.WriteLine($"❌ Error: You are not the owner of pool {poolAddress}");
            Console.WriteLine($"   Pool Owner: {poolInfo.Owner}");
            Console.WriteLine($"   Your Address: {keypair.PublicKey.Key}");
            return false;
        }

        return true;
    }

    protected async Task LogOperationAsync(string operation, string details, string transactionSignature = null)
    {
        await _loggingService.LogOwnerOperationAsync(new OwnerOperationLog
        {
            Operation = operation,
            Details = details,
            TransactionSignature = transactionSignature,
            Timestamp = DateTime.UtcNow,
            OperatorAddress = _keypairService.CurrentKeypair?.PublicKey.Key
        });
    }
}
```

### **Fee Commands Implementation**
```csharp
public class FeeCommands : BaseCommand
{
    public FeeCommands(IServiceProvider services) : base(services) { }

    public Command CreateCommands()
    {
        var feeCommand = new Command("fees", "Manage pool fees");

        // Withdraw fees command
        var withdrawCommand = new Command("withdraw", "Withdraw accumulated fees from pool");
        withdrawCommand.AddOption(new Option<string>("--pool-address", "Pool address") { IsRequired = true });
        withdrawCommand.AddOption(new Option<ulong>("--amount", "Amount to withdraw") { IsRequired = true });
        withdrawCommand.AddOption(new Option<string>("--token", "Token type (SOL or token mint)") { IsRequired = true });
        withdrawCommand.AddOption(new Option<string>("--keypair", "Path to keypair file") { IsRequired = true });
        withdrawCommand.AddOption(new Option<string>("--network", "Network (testnet/mainnet)") { IsRequired = true });

        withdrawCommand.SetHandler(async (poolAddress, amount, token, keypairPath, network) =>
        {
            try
            {
                Console.WriteLine($"🔄 Withdrawing {amount} {token} fees from pool {poolAddress}...");

                // Validate ownership
                if (!await ValidateOwnershipAsync(poolAddress, keypairPath, network))
                    return;

                // Execute fee withdrawal transaction
                var result = await _solanaService.WithdrawFeesAsync(new WithdrawFeesRequest
                {
                    PoolAddress = poolAddress,
                    Amount = amount,
                    TokenMint = token,
                    KeypairPath = keypairPath,
                    Network = network
                });

                Console.WriteLine($"✅ Fees withdrawn successfully!");
                Console.WriteLine($"   Transaction: {result.TransactionSignature}");
                Console.WriteLine($"   Amount: {amount} {token}");
                Console.WriteLine($"   Explorer: https://explorer.solana.com/tx/{result.TransactionSignature}?cluster={network}");

                await LogOperationAsync("WITHDRAW_FEES", 
                    $"Withdrew {amount} {token} fees from pool {poolAddress}", 
                    result.TransactionSignature);
            }
            catch (Exception ex)
            {
                Console.WriteLine($"❌ Error withdrawing fees: {ex.Message}");
            }
        });

        feeCommand.AddCommand(withdrawCommand);

        // Add other fee commands (view, set-rate)...

        return feeCommand;
    }
}
```

### **Security Features**

#### **Keypair Validation**
```csharp
public class KeypairService : IKeypairService
{
    public async Task<Keypair> LoadKeypairAsync(string keypairPath)
    {
        if (!File.Exists(keypairPath))
            throw new FileNotFoundException($"Keypair file not found: {keypairPath}");

        var jsonContent = await File.ReadAllTextAsync(keypairPath);
        var keyData = JsonSerializer.Deserialize<byte[]>(jsonContent);
        
        if (keyData?.Length != 64)
            throw new InvalidOperationException("Invalid keypair format");

        return Keypair.RestoreKeypair(keyData);
    }

    public bool ValidateKeypairSecurity(string keypairPath)
    {
        var fileInfo = new FileInfo(keypairPath);
        
        // Check file permissions (Unix-like systems)
        if (Environment.OSVersion.Platform == PlatformID.Unix)
        {
            var permissions = File.GetUnixFileMode(keypairPath);
            if ((permissions & UnixFileMode.GroupRead) != 0 || 
                (permissions & UnixFileMode.OtherRead) != 0)
            {
                Console.WriteLine("⚠️  Warning: Keypair file has overly permissive permissions");
                return false;
            }
        }

        return true;
    }
}
```

#### **Operation Logging**
```csharp
public class LoggingService : ILoggingService
{
    private readonly string _logDirectory = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".frt-owner-cli", "logs");

    public async Task LogOwnerOperationAsync(OwnerOperationLog log)
    {
        Directory.CreateDirectory(_logDirectory);
        
        var logFile = Path.Combine(_logDirectory, $"owner-operations-{DateTime.UtcNow:yyyy-MM-dd}.log");
        var logEntry = $"[{log.Timestamp:yyyy-MM-dd HH:mm:ss UTC}] {log.Operation} - {log.Details}";
        
        if (!string.IsNullOrEmpty(log.TransactionSignature))
            logEntry += $" | TX: {log.TransactionSignature}";
        
        await File.AppendAllTextAsync(logFile, logEntry + Environment.NewLine);
        
        // Also log to console with colors
        Console.WriteLine($"📝 Logged: {log.Operation} at {log.Timestamp:HH:mm:ss}");
    }
}
```

## 🚀 **Usage Examples**

### **Complete Fee Management Workflow**
```bash
# 1. Check available fees across all pools
frt-owner-cli fees view-all --keypair "./keypair.json" --network testnet

# 2. View fees for specific pool
frt-owner-cli fees view --pool-address "5xyz...abc" --keypair "./keypair.json" --network testnet

# 3. Adjust fee rates if needed
frt-owner-cli fees set-rate \
  --pool-address "5xyz...abc" \
  --swap-fee-percentage 0.30 \
  --keypair "./keypair.json" \
  --network testnet

# 4. Withdraw accumulated fees
frt-owner-cli fees withdraw \
  --pool-address "5xyz...abc" \
  --amount 500 \
  --token SOL \
  --keypair "./keypair.json" \
  --network testnet
```

### **Emergency Response Workflow**
```bash
# 1. Check system status
frt-owner-cli system status --keypair "./keypair.json" --network testnet

# 2. Emergency pause if needed
frt-owner-cli system pause \
  --reason "Security incident - investigating unusual activity" \
  --keypair "./keypair.json" \
  --network testnet

# 3. Investigate and resolve issue...

# 4. Unpause when safe
frt-owner-cli system unpause --keypair "./keypair.json" --network testnet
```

### **Pool Management Workflow**
```bash
# 1. Check pool information
frt-owner-cli pool info --pool-address "5xyz...abc" --keypair "./keypair.json" --network testnet

# 2. Pause pool if maintenance needed
frt-owner-cli pool pause \
  --pool-address "5xyz...abc" \
  --reason "Routine maintenance" \
  --keypair "./keypair.json" \
  --network testnet

# 3. Unpause pool when ready
frt-owner-cli pool unpause \
  --pool-address "5xyz...abc" \
  --keypair "./keypair.json" \
  --network testnet
```

## 🔒 **Security Best Practices**

### **Keypair Protection**
- Store keypair files with restricted permissions (600 on Unix systems)
- Use hardware wallets for mainnet operations when possible
- Never commit keypair files to version control
- Regular keypair rotation for enhanced security

### **Operational Security**
- Always verify pool ownership before operations
- Use descriptive reasons for pause operations
- Monitor operation logs regularly
- Test operations on testnet before mainnet

### **Network Security**
- Verify network parameter matches intended environment
- Use HTTPS RPC endpoints only
- Validate transaction signatures before confirmation
- Monitor for unusual network activity

## 📊 **Monitoring and Alerting**

### **Operation Logging**
All operations are logged with:
- Timestamp (UTC)
- Operation type
- Pool addresses involved
- Transaction signatures
- Success/failure status
- Error details (if applicable)

### **Log Locations**
- **Unix/Linux**: `~/.frt-owner-cli/logs/`
- **Windows**: `%USERPROFILE%\.frt-owner-cli\logs\`
- **Log Rotation**: Daily log files with automatic cleanup

### **Monitoring Integration**
The CLI can be integrated with monitoring systems:
- Exit codes for success/failure detection
- Structured JSON output option for parsing
- Webhook notifications for critical operations
- Integration with enterprise logging systems

## 🚦 **Installation and Setup**

### **Prerequisites**
- .NET 8.0 Runtime
- Solana CLI tools (for keypair generation)
- Network access to Solana RPC endpoints

### **Installation**
```bash
# Download latest release
wget https://github.com/your-org/frt-owner-cli/releases/latest/frt-owner-cli-linux.tar.gz

# Extract and install
tar -xzf frt-owner-cli-linux.tar.gz
sudo mv frt-owner-cli /usr/local/bin/

# Verify installation
frt-owner-cli --version
```

### **Configuration**
```bash
# Create configuration directory
mkdir -p ~/.frt-owner-cli

# Copy configuration template
cp appsettings.json ~/.frt-owner-cli/

# Edit configuration for your network
nano ~/.frt-owner-cli/appsettings.json
```

### **First Run**
```bash
# Verify keypair access
frt-owner-cli system status \
  --keypair "/path/to/your/keypair.json" \
  --network testnet

# If successful, you're ready to use the CLI!
```

## 📋 **Command Reference**

### **Global Options**
- `--keypair <path>`: Path to owner keypair JSON file (required)
- `--network <network>`: Network selection (testnet/mainnet)
- `--verbose`: Enable detailed output
- `--json`: Output results in JSON format
- `--help`: Show command help

### **Return Codes**
- `0`: Success
- `1`: General error
- `2`: Authentication/authorization error
- `3`: Network/connection error
- `4`: Invalid arguments
- `5`: Keypair/security error

---

**⚠️ IMPORTANT SECURITY NOTICE**

This CLI application handles sensitive cryptographic keys and performs irreversible blockchain operations. Always:
- Test on testnet first
- Verify all parameters before execution
- Keep keypair files secure and backed up
- Monitor all operations through logs
- Never share keypair files or expose them to web interfaces

**The separation between the dashboard (user functions) and CLI (owner functions) is a critical security feature that must be maintained.** 