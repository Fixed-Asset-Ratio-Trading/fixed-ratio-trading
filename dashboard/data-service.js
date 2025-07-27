// Fixed Ratio Trading - Centralized Data Service
// This service provides a unified interface for loading pool and system state data
// Supports multiple data sources: state.json, RPC, and future API servers

class TradingDataService {
    constructor() {
        this.connection = null;
        this.config = null;
        this.cache = new Map();
        this.cacheTimeout = 30000; // 30 seconds
    }

    /**
     * Initialize the data service with configuration
     */
    async initialize(config, connection = null) {
        this.config = config;
        this.connection = connection;
        console.log('📊 TradingDataService initialized');
    }

    /**
     * Load all data from the configured source
     * @param {string} source - 'state.json', 'rpc', or 'api' (future)
     * @returns {Object} Complete state data
     */
    async loadAllData(source = 'auto') {
        try {
            console.log(`📥 Loading data from source: ${source}`);
            
            switch (source) {
                case 'state.json':
                    return await this.loadFromStateFile();
                case 'rpc':
                    return await this.loadFromRPC();
                case 'auto':
                default:
                    // Try state.json first, fallback to RPC
                    try {
                        const stateData = await this.loadFromStateFile();
                        if (stateData.pools.length > 0 || stateData.mainTreasuryState || stateData.systemState) {
                            return stateData;
                        }
                    } catch (error) {
                        console.warn('⚠️ State file not available, trying RPC...');
                    }
                    return await this.loadFromRPC();
            }
        } catch (error) {
            console.error('❌ Error loading data:', error);
            throw error;
        }
    }

    /**
     * Load data from state.json file
     */
    async loadFromStateFile() {
        try {
            const stateFile = this.config?.stateFile || 'state.json';
            const cacheBuster = `?t=${Date.now()}`;
            
            const response = await fetch(`${stateFile}${cacheBuster}`, {
                cache: 'no-cache',
                headers: {
                    'Cache-Control': 'no-cache, no-store, must-revalidate',
                    'Pragma': 'no-cache',
                    'Expires': '0'
                }
            });
            
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }
            
            const stateData = await response.json();
            console.log(`✅ Loaded from state.json: ${stateData.pools?.length || 0} pools, treasury: ${!!stateData.main_treasury_state}, system: ${!!stateData.system_state}`);
            
            // Map snake_case field names from state.json to camelCase for JavaScript compatibility
            const mappedPools = (stateData.pools || []).map(pool => ({
                address: pool.address,
                owner: pool.owner,
                tokenAMint: pool.token_a_mint,
                tokenBMint: pool.token_b_mint,
                tokenAVault: pool.token_a_vault,
                tokenBVault: pool.token_b_vault,
                lpTokenAMint: pool.lp_token_a_mint,
                lpTokenBMint: pool.lp_token_b_mint,
                ratioANumerator: pool.ratio_a_numerator,
                ratioBDenominator: pool.ratio_b_denominator,
                tokenALiquidity: pool.total_token_a_liquidity,
                tokenBLiquidity: pool.total_token_b_liquidity,
                poolAuthorityBumpSeed: pool.pool_authority_bump_seed,
                tokenAVaultBumpSeed: pool.token_a_vault_bump_seed,
                tokenBVaultBumpSeed: pool.token_b_vault_bump_seed,
                lpTokenAMintBumpSeed: pool.lp_token_a_mint_bump_seed,
                lpTokenBMintBumpSeed: pool.lp_token_b_mint_bump_seed,
                flags: pool.flags,
                collectedFeesTokenA: pool.collected_fees_token_a,
                collectedFeesTokenB: pool.collected_fees_token_b,
                collectedSolFees: pool.collected_sol_fees,
                swapFeeBasisPoints: pool.swap_fee_basis_points,
                isInitialized: pool.is_initialized !== false, // Default to true if not specified
                isPaused: pool.is_paused || false,
                swapsPaused: pool.swaps_paused || false,
                dataSource: 'JSON'
            }));

            return {
                pools: mappedPools,
                mainTreasuryState: stateData.main_treasury_state,
                systemState: stateData.system_state,
                pdaAddresses: stateData.pda_addresses,
                metadata: stateData.metadata,
                source: 'state.json'
            };
        } catch (error) {
            console.error('❌ Error loading from state file:', error);
            throw error;
        }
    }

    /**
     * Load data from RPC
     */
    async loadFromRPC() {
        if (!this.connection) {
            throw new Error('RPC connection not initialized');
        }

        try {
            console.log('🔍 Loading data from RPC...');
            
            // Get all program accounts
            const programAccounts = await this.connection.getProgramAccounts(
                new solanaWeb3.PublicKey(this.config.programId),
                { encoding: 'base64' }
            );
            
            console.log(`📊 Found ${programAccounts.length} program accounts`);
            
            // Parse pools (filter by size to avoid parsing treasury/system state)
            const pools = [];
            for (const account of programAccounts) {
                if (account.account.data.length > 300) { // Pool states are larger
                    try {
                        const poolData = this.parsePoolState(account.account.data, account.pubkey.toString());
                        if (poolData) {
                            pools.push(poolData);
                        }
                    } catch (error) {
                        console.warn(`Failed to parse pool at ${account.pubkey.toString()}:`, error);
                    }
                }
            }
            
            // TODO: Parse treasury and system state from RPC
            // For now, return pools only from RPC
            
            return {
                pools,
                mainTreasuryState: null,
                systemState: null,
                pdaAddresses: null,
                metadata: {
                    generated_at: new Date().toISOString(),
                    source: 'rpc'
                },
                source: 'rpc'
            };
        } catch (error) {
            console.error('❌ Error loading from RPC:', error);
            throw error;
        }
    }

    /**
     * Get a specific pool by address
     * @param {string} poolAddress - Pool address
     * @param {string} source - Data source preference
     * @returns {Object} Pool data
     */
    async getPool(poolAddress, source = 'auto') {
        try {
            const cacheKey = `pool_${poolAddress}_${source}`;
            
            // Check cache first
            if (this.cache.has(cacheKey)) {
                const cached = this.cache.get(cacheKey);
                if (Date.now() - cached.timestamp < this.cacheTimeout) {
                    console.log(`📋 Using cached pool data for ${poolAddress}`);
                    return cached.data;
                }
            }
            
            let poolData = null;
            
            if (source === 'rpc' || source === 'auto') {
                // Load directly from RPC for real-time data
                poolData = await this.getPoolFromRPC(poolAddress);
            }
            
            if (!poolData && (source === 'state.json' || source === 'auto')) {
                // Fallback to state file
                const allData = await this.loadFromStateFile();
                poolData = allData.pools.find(p => p.address === poolAddress);
            }
            
            if (poolData) {
                // Cache the result
                this.cache.set(cacheKey, {
                    data: poolData,
                    timestamp: Date.now()
                });
            }
            
            return poolData;
        } catch (error) {
            console.error(`❌ Error getting pool ${poolAddress}:`, error);
            throw error;
        }
    }

    /**
     * Get pool data directly from RPC
     */
    async getPoolFromRPC(poolAddress) {
        if (!this.connection) {
            throw new Error('RPC connection not initialized');
        }

        try {
            console.log(`🔍 Loading pool ${poolAddress} from RPC...`);
            
            const poolAccount = await this.connection.getAccountInfo(
                new solanaWeb3.PublicKey(poolAddress)
            );
            
            if (!poolAccount) {
                throw new Error('Pool account not found');
            }
            
            return this.parsePoolState(poolAccount.data, poolAddress);
        } catch (error) {
            console.error(`❌ Error loading pool from RPC:`, error);
            throw error;
        }
    }

    /**
     * Centralized pool state parsing
     * This is the single source of truth for pool parsing logic
     */
    parsePoolState(data, address) {
        try {
            const dataArray = new Uint8Array(data);
            let offset = 0;
            
            // Helper functions to read bytes
            const readPubkey = () => {
                const pubkey = dataArray.slice(offset, offset + 32);
                offset += 32;
                return new solanaWeb3.PublicKey(pubkey).toString();
            };
            
            const readU64 = () => {
                const view = new DataView(dataArray.buffer, offset, 8);
                const value = view.getBigUint64(0, true); // little-endian
                offset += 8;
                return Number(value);
            };

            const readI64 = () => {
                const view = new DataView(dataArray.buffer, offset, 8);
                const value = view.getBigInt64(0, true); // little-endian
                offset += 8;
                return Number(value);
            };

            const readU8 = () => {
                const value = dataArray[offset];
                offset += 1;
                return value;
            };

            const readBool = () => {
                const value = dataArray[offset] !== 0;
                offset += 1;
                return value;
            };
            
            // Parse all PoolState fields according to the struct definition
            const owner = readPubkey();
            const tokenAMint = readPubkey();
            const tokenBMint = readPubkey();
            const tokenAVault = readPubkey();
            const tokenBVault = readPubkey();
            const lpTokenAMint = readPubkey();
            const lpTokenBMint = readPubkey();
            
            const ratioANumerator = readU64();
            const ratioBDenominator = readU64();
            const totalTokenALiquidity = readU64();
            const totalTokenBLiquidity = readU64();
            
            // Bump seeds
            const poolAuthorityBumpSeed = readU8();
            const tokenAVaultBumpSeed = readU8();
            const tokenBVaultBumpSeed = readU8();
            const lpTokenAMintBumpSeed = readU8();
            const lpTokenBMintBumpSeed = readU8();
            
            // Pool flags (bitwise operations)
            const flags = readU8();
            
            // Configurable contract fees
            const contractLiquidityFee = readU64();
            const swapContractFee = readU64();
            
            // Token fee tracking
            const collectedFeesTokenA = readU64();
            const collectedFeesTokenB = readU64();
            const totalFeesWithdrawnTokenA = readU64();
            const totalFeesWithdrawnTokenB = readU64();
            
            // SOL fee tracking
            const collectedLiquidityFees = readU64();
            const collectedSwapContractFees = readU64();
            const totalSolFeesCollected = readU64();
            
            // Consolidation management
            const lastConsolidationTimestamp = readI64();
            const totalConsolidations = readU64();
            const totalFeesConsolidated = readU64();
            
            // Decode flags
            const flagsDecoded = {
                one_to_many_ratio: (flags & 1) !== 0,
                liquidity_paused: (flags & 2) !== 0,
                swaps_paused: (flags & 4) !== 0,
                withdrawal_protection: (flags & 8) !== 0,
                single_lp_token_mode: (flags & 16) !== 0
            };
            
            return {
                address: address,
                owner,
                tokenAMint,
                tokenBMint,
                tokenAVault,
                tokenBVault,
                lpTokenAMint,
                lpTokenBMint,
                ratioANumerator,
                ratioBDenominator,
                tokenALiquidity: totalTokenALiquidity,
                tokenBLiquidity: totalTokenBLiquidity,
                
                // Bump seeds
                poolAuthorityBumpSeed,
                tokenAVaultBumpSeed,
                tokenBVaultBumpSeed,
                lpTokenAMintBumpSeed,
                lpTokenBMintBumpSeed,
                
                // Flags
                flags,
                flagsDecoded,
                
                // Fee configuration
                contractLiquidityFee,
                swapContractFee,
                
                // Token fee tracking
                collectedFeesTokenA,
                collectedFeesTokenB,
                totalFeesWithdrawnTokenA,
                totalFeesWithdrawnTokenB,
                
                // SOL fee tracking
                collectedLiquidityFees,
                collectedSwapContractFees,
                totalSolFeesCollected,
                
                // Consolidation data
                lastConsolidationTimestamp,
                totalConsolidations,
                totalFeesConsolidated,
                
                // Metadata
                dataSource: 'rpc',
                lastUpdated: Date.now()
            };
        } catch (error) {
            console.error(`❌ Error parsing pool state for ${address}:`, error);
            throw new Error(`Failed to parse pool state: ${error.message}`);
        }
    }

    /**
     * Clear cache
     */
    clearCache() {
        this.cache.clear();
        console.log('🧹 Data service cache cleared');
    }

    /**
     * Get cache stats
     */
    getCacheStats() {
        return {
            size: this.cache.size,
            keys: Array.from(this.cache.keys())
        };
    }
}

// Create a global instance
window.TradingDataService = new TradingDataService();

// Export for module systems
if (typeof module !== 'undefined' && module.exports) {
    module.exports = TradingDataService;
}

console.log('📊 TradingDataService loaded'); 