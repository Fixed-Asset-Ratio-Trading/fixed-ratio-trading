# Solana Transaction Building Guide for .NET Developers

## Overview

This guide documents the critical requirements for building Solana transactions in .NET applications when interacting with the Fixed Ratio Trading smart contract. These guidelines prevent common serialization issues and ensure reliable blockchain communication.

## ⚠️ Critical Issue: Solnet Transaction Serialization

### Problem
The `Solnet.Rpc.Builders.TransactionBuilder` class has known issues that can produce malformed transaction bytes, resulting in errors like:
```
failed to deserialize solana_transaction::versioned::VersionedTransaction: io error: failed to fill whole buffer
```

### Solution: Raw RPC Transaction Building

For production applications requiring reliable transaction execution, use **raw RPC calls** instead of Solnet's transaction builders.

## 🔧 Implementation Example

### 1. Manual Transaction Construction

```csharp
public class RawTransactionBuilder
{
    private readonly HttpClient _httpClient;
    private readonly string _rpcUrl;
    
    public async Task<string> BuildGetVersionTransaction(string programId)
    {
        var programIdBytes = DecodeBase58(programId);
        var feePayerBytes = new byte[32]; // Generate or provide fee payer
        var recentBlockhash = await GetLatestBlockhashAsync();
        
        using var stream = new MemoryStream();
        using var writer = new BinaryWriter(stream);

        // 1. Number of signatures (1 byte)
        writer.Write((byte)1);

        // 2. Dummy signature (64 bytes) - ignored with sigVerify: false
        writer.Write(new byte[64]);

        // 3. Message header
        writer.Write((byte)1); // Number of required signatures
        writer.Write((byte)0); // Number of readonly signed accounts
        writer.Write((byte)1); // Number of readonly unsigned accounts

        // 4. Account addresses (compact array)
        writer.Write((byte)2); // 2 accounts: fee payer + program
        writer.Write(feePayerBytes); // Fee payer (32 bytes)
        writer.Write(programIdBytes); // Program ID (32 bytes)

        // 5. Recent blockhash (32 bytes)
        writer.Write(recentBlockhash);

        // 6. Instructions (compact array)
        writer.Write((byte)1); // 1 instruction

        // 7. GetVersion instruction
        writer.Write((byte)1); // Program ID index
        writer.Write((byte)0); // Accounts array length (GetVersion needs no accounts)
        writer.Write((byte)1); // Instruction data length
        writer.Write((byte)14); // GetVersion discriminator

        return Convert.ToBase64String(stream.ToArray());
    }
}
```

### 2. RPC Simulation Call

```csharp
public async Task<string> SimulateTransaction(string transactionBase64)
{
    var request = new
    {
        jsonrpc = "2.0",
        id = 1,
        method = "simulateTransaction",
        @params = new object[]
        {
            transactionBase64,
            new
            {
                sigVerify = false,
                replaceRecentBlockhash = true,
                encoding = "base64"
            }
        }
    };

    var jsonRequest = JsonSerializer.Serialize(request);
    var content = new StringContent(jsonRequest, Encoding.UTF8, "application/json");
    var response = await _httpClient.PostAsync(_rpcUrl, content);
    
    return await response.Content.ReadAsStringAsync();
}
```

## 📋 Instruction Reference

### GetVersion (Discriminator: 14)
- **Purpose**: Retrieve contract version information
- **Accounts Required**: None
- **Instruction Data**: `[14]` (single byte)
- **Expected Response**: Program logs containing version information

**Example Transaction Format:**
```
Transaction Structure:
├── Signatures (65 bytes): [1 signature count] + [64-byte dummy signature]
├── Message Header (3 bytes): [1, 0, 1]
├── Accounts (65 bytes): [2 account count] + [32-byte fee payer] + [32-byte program ID]
├── Recent Blockhash (32 bytes)
└── Instructions (4 bytes): [1 instruction count] + [1 program index] + [0 accounts] + [1 data length] + [14 discriminator]
```

### InitializePool (Discriminator: 1)

Build the InitializePool instruction exactly as specified in the API documentation. This section summarizes the precise wire format and account ordering, and provides .NET-focused examples.

**Instruction Data (exactly 17 bytes):**
- Byte 0: discriminator = `1`
- Bytes 1-8: `ratio_a_numerator` as u64 little-endian (basis points)
- Bytes 9-16: `ratio_b_denominator` as u64 little-endian (basis points)

```csharp
// Build 17-byte instruction data for InitializePool in C#
// Input ratios MUST already be in basis points (amount * 10^decimals)
Span<byte> data = stackalloc byte[17];
data[0] = 1; // InitializePool discriminator
System.Buffers.Binary.BinaryPrimitives.WriteUInt64LittleEndian(data.Slice(1, 8), ratioANumeratorBasisPoints);
System.Buffers.Binary.BinaryPrimitives.WriteUInt64LittleEndian(data.Slice(9, 8), ratioBDenominatorBasisPoints);

byte[] instructionData = data.ToArray();
```

**Basis Points Conversion:**
```csharp
// Convert display amount to basis points using token decimals
static ulong ToBasisPoints(double displayAmount, int decimals)
{
    // Use decimal for precision if needed; floor toward zero
    var scale = Math.Pow(10, decimals);
    var value = Math.Floor(displayAmount * scale);
    return (ulong)value;
}
```

**Token Ordering (lexicographic byte comparison):**
The program normalizes token order using byte-wise lexicographic comparison identical to Rust `Pubkey::cmp`. Do NOT compare Base58 strings.

```csharp
static int CompareLex(ReadOnlySpan<byte> a, ReadOnlySpan<byte> b)
{
    for (int i = 0; i < 32; i++)
    {
        if (a[i] < b[i]) return -1;
        if (a[i] > b[i]) return 1;
    }
    return 0;
}

// Example usage:
byte[] tokenABytes = DecodeBase58(tokenAMintBase58);
byte[] tokenBBytes = DecodeBase58(tokenBMintBase58);
bool aLess = CompareLex(tokenABytes, tokenBBytes) < 0;

// If not in lex order, swap tokens and swap ratio sides
if (!aLess)
{
    (tokenAMintBase58, tokenBMintBase58) = (tokenBMintBase58, tokenAMintBase58);
    (ratioANumeratorBasisPoints, ratioBDenominatorBasisPoints) = (ratioBDenominatorBasisPoints, ratioANumeratorBasisPoints);
}
```

Equivalent JavaScript example:
```javascript
const a = tokenAMint.toBytes();
const b = tokenBMint.toBytes();
let aLessThanB = false;
for (let i = 0; i < 32; i++) {
  if (a[i] < b[i]) { aLessThanB = true; break; }
  if (a[i] > b[i]) { aLessThanB = false; break; }
}
if (!aLessThanB) {
  // swap token mints and ratio sides
}
```

**Required Account Structure (exactly 13 accounts in this order):**
| Index | Account | Type | Notes |
|------:|---------|------|-------|
| 0 | User Authority | Signer, Writable | Pool creator |
| 1 | System Program | Readable | `11111111111111111111111111111111` |
| 2 | System State PDA | Readable | Global state for pause validation |
| 3 | Pool State PDA | Writable | Will be created |
| 4 | SPL Token Program | Readable | Token program ID |
| 5 | Main Treasury PDA | Writable | For 1.15 SOL fee collection |
| 6 | Rent Sysvar | Readable | Rent calculations |
| 7 | Token A Mint | Readable | Lexicographically smaller mint |
| 8 | Token B Mint | Readable | Lexicographically larger mint |
| 9 | Token A Vault PDA | Writable | Will be created |
| 10 | Token B Vault PDA | Writable | Will be created |
| 11 | LP Token A Mint PDA | Writable | Will be created |
| 12 | LP Token B Mint PDA | Writable | Will be created |

Note: All 6 PDAs must match expected derived addresses. If any PDA is wrong, the transaction fails.

**JavaScript instruction data (reference):**
```javascript
const discriminator = new Uint8Array([1]);
const ratioABytes = new Uint8Array(new BigUint64Array([BigInt(ratioANumeratorBasisPoints)]).buffer);
const ratioBBytes = new Uint8Array(new BigUint64Array([BigInt(ratioBDenominatorBasisPoints)]).buffer);
const instructionData = new Uint8Array([...discriminator, ...ratioABytes, ...ratioBBytes]); // 17 bytes
```

**Submission Strategy (avoid Solnet builder issues):**
- Prefer raw RPC or manual transaction composition
- Ensure a fresh recent blockhash
- Sign with the fee payer; submit using `sendRawTransaction`

For full working examples, see the API doc’s “Working Pool Creation Implementation Guide”.

## 🔍 Validation Checklist

### Transaction Format Validation
- [ ] **Transaction serializes without errors**
- [ ] **Simulation returns `AccountNotFound` (not deserialization errors)**
- [ ] **Program ID is correctly encoded in Base58**
- [ ] **Recent blockhash is retrieved from RPC**
- [ ] **Instruction discriminator matches API specification**

### Expected Simulation Results

#### ✅ Success Indicators
```json
{
  "result": {
    "value": {
      "err": "AccountNotFound",  // Expected for unfunded fee payer
      "logs": [],               // May be empty for AccountNotFound
      "accounts": null
    }
  }
}
```

#### ❌ Failure Indicators
```json
{
  "error": {
    "message": "failed to deserialize solana_transaction::versioned::VersionedTransaction"
  }
}
```

## 🛠️ Development Tools

### Base58 Encoding/Decoding
```csharp
private static byte[] DecodeBase58(string base58)
{
    const string alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    
    var decoded = new List<byte>();
    var num = System.Numerics.BigInteger.Zero;
    
    foreach (var c in base58)
    {
        var index = alphabet.IndexOf(c);
        if (index == -1) throw new ArgumentException($"Invalid character: {c}");
        num = num * 58 + index;
    }
    
    while (num > 0)
    {
        decoded.Insert(0, (byte)(num % 256));
        num /= 256;
    }
    
    // Handle leading zeros
    var leadingZeros = 0;
    foreach (var c in base58)
    {
        if (c == '1') leadingZeros++;
        else break;
    }
    
    for (int i = 0; i < leadingZeros; i++)
    {
        decoded.Insert(0, 0);
    }
    
    return decoded.ToArray();
}
```

### RPC Helper Methods
```csharp
private async Task<byte[]> GetLatestBlockhashAsync()
{
    var request = new
    {
        jsonrpc = "2.0",
        id = 1,
        method = "getLatestBlockhash",
        @params = new object[] { new { commitment = "confirmed" } }
    };

    // Implementation details...
    // Returns 32-byte blockhash
}
```

## 🚨 Common Pitfalls

### 1. **Using Solnet TransactionBuilder**
```csharp
// ❌ AVOID - Can produce malformed transactions
var transaction = new TransactionBuilder()
    .SetRecentBlockHash(blockhash)
    .SetFeePayer(feePayer)
    .AddInstruction(instruction)
    .Build();
```

### 2. **Incorrect Account Ordering**
- Fee payer must be first account (index 0)
- Program ID must be correctly indexed in instruction
- Account indices must match the accounts array

### 3. **Missing Recent Blockhash**
- Always fetch a real blockhash from RPC
- Don't use dummy/zero blockhashes for production transactions

### 4. **Instruction Data Format**
- Use exact discriminator values from API documentation
- Ensure proper Borsh serialization for complex data structures

## 🔗 Related Resources

- [Fixed Ratio Trading API Documentation](../api/A_FIXED_RATIO_TRADING_API.md)
- [Solana Transaction Format Specification](https://docs.solana.com/developing/programming-model/transactions)
- [RPC API Reference](https://docs.solana.com/api/http)

## 📝 Testing Guidelines

### Unit Test Example
```csharp
[Fact]
public async Task BuildGetVersionTransaction_ShouldProduceValidFormat()
{
    // Arrange
    var builder = new RawTransactionBuilder(rpcUrl);
    
    // Act
    var transactionBase64 = await builder.BuildGetVersionTransaction(programId);
    var response = await builder.SimulateTransaction(transactionBase64);
    
    // Assert
    // Should NOT contain deserialization errors
    Assert.DoesNotContain("failed to deserialize", response);
    
    // Should indicate proper transaction format (AccountNotFound is OK)
    Assert.Contains("AccountNotFound", response);
}
```

## 🏷️ Version Compatibility

- **Solnet**: Avoid transaction builders in versions that exhibit serialization issues
- **Fixed Ratio Trading Contract**: 0.15.1053 (confirmed compatible)
- **Solana RPC**: Standard JSON-RPC API (v1.14+)

---

**Last Updated**: December 2024  
**Tested Against**: Fixed Ratio Trading Contract v0.15.1053 on Solana Localnet
