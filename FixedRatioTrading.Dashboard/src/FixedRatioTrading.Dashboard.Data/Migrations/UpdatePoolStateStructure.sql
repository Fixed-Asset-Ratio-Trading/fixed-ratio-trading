-- Migration: Update Pool State Structure to Match Smart Contract
-- Date: $(date)
-- Description: Updates the pools table to match the current smart contract PoolState structure
-- Removes delegate system references and adds new fields for fee tracking, swap pause controls, etc.

BEGIN;

-- Add new required fields from smart contract
ALTER TABLE pools ADD COLUMN IF NOT EXISTS owner VARCHAR(44) NOT NULL DEFAULT '';
ALTER TABLE pools ADD COLUMN IF NOT EXISTS token_a_vault VARCHAR(44) NOT NULL DEFAULT '';
ALTER TABLE pools ADD COLUMN IF NOT EXISTS token_b_vault VARCHAR(44) NOT NULL DEFAULT '';
ALTER TABLE pools ADD COLUMN IF NOT EXISTS lp_token_a_mint VARCHAR(44) NOT NULL DEFAULT '';
ALTER TABLE pools ADD COLUMN IF NOT EXISTS lp_token_b_mint VARCHAR(44) NOT NULL DEFAULT '';

-- Add bump seeds for PDA derivation
ALTER TABLE pools ADD COLUMN IF NOT EXISTS pool_authority_bump_seed SMALLINT NOT NULL DEFAULT 0;
ALTER TABLE pools ADD COLUMN IF NOT EXISTS token_a_vault_bump_seed SMALLINT NOT NULL DEFAULT 0;
ALTER TABLE pools ADD COLUMN IF NOT EXISTS token_b_vault_bump_seed SMALLINT NOT NULL DEFAULT 0;

-- Add initialization and pause state fields
ALTER TABLE pools ADD COLUMN IF NOT EXISTS is_initialized BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE pools ADD COLUMN IF NOT EXISTS is_paused BOOLEAN NOT NULL DEFAULT false;

-- Add pool-specific swap pause controls (separate from system pause)
ALTER TABLE pools ADD COLUMN IF NOT EXISTS swaps_paused BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE pools ADD COLUMN IF NOT EXISTS swaps_pause_initiated_by VARCHAR(44);
ALTER TABLE pools ADD COLUMN IF NOT EXISTS swaps_pause_initiated_timestamp BIGINT NOT NULL DEFAULT 0;

-- Add automatic withdrawal protection
ALTER TABLE pools ADD COLUMN IF NOT EXISTS withdrawal_protection_active BOOLEAN NOT NULL DEFAULT false;

-- Add fee collection and withdrawal tracking
ALTER TABLE pools ADD COLUMN IF NOT EXISTS collected_fees_token_a BIGINT NOT NULL DEFAULT 0;
ALTER TABLE pools ADD COLUMN IF NOT EXISTS collected_fees_token_b BIGINT NOT NULL DEFAULT 0;
ALTER TABLE pools ADD COLUMN IF NOT EXISTS total_fees_withdrawn_token_a BIGINT NOT NULL DEFAULT 0;
ALTER TABLE pools ADD COLUMN IF NOT EXISTS total_fees_withdrawn_token_b BIGINT NOT NULL DEFAULT 0;
ALTER TABLE pools ADD COLUMN IF NOT EXISTS swap_fee_basis_points BIGINT NOT NULL DEFAULT 0;
ALTER TABLE pools ADD COLUMN IF NOT EXISTS collected_sol_fees BIGINT NOT NULL DEFAULT 0;
ALTER TABLE pools ADD COLUMN IF NOT EXISTS total_sol_fees_withdrawn BIGINT NOT NULL DEFAULT 0;

-- Rename liquidity fields to match smart contract naming
ALTER TABLE pools RENAME COLUMN token_a_liquidity TO total_token_a_liquidity;
ALTER TABLE pools RENAME COLUMN token_b_liquidity TO total_token_b_liquidity;

-- Update creator_address to owner (if not already done)
UPDATE pools SET owner = creator_address WHERE owner = '' AND creator_address IS NOT NULL AND creator_address != '';

-- Create indexes for new fields to improve query performance
CREATE INDEX IF NOT EXISTS idx_pools_owner ON pools(owner);
CREATE INDEX IF NOT EXISTS idx_pools_swaps_paused ON pools(swaps_paused);
CREATE INDEX IF NOT EXISTS idx_pools_is_paused ON pools(is_paused);
CREATE INDEX IF NOT EXISTS idx_pools_fee_tracking ON pools(collected_fees_token_a, collected_fees_token_b);

-- Drop any delegate-related tables or columns if they exist
-- (These would be custom additions not in the standard migration, so use IF EXISTS)
DROP TABLE IF EXISTS pool_delegates CASCADE;
DROP TABLE IF EXISTS delegate_permissions CASCADE;
DROP TABLE IF EXISTS delegate_time_limits CASCADE;

-- Remove delegate-related columns from pools table if they exist
ALTER TABLE pools DROP COLUMN IF EXISTS delegate_count;
ALTER TABLE pools DROP COLUMN IF EXISTS max_delegates;
ALTER TABLE pools DROP COLUMN IF EXISTS delegate_permissions;

-- Update system_state table to match smart contract structure
-- Create system_state table if it doesn't exist
CREATE TABLE IF NOT EXISTS system_state (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    authority VARCHAR(44) NOT NULL,
    is_paused BOOLEAN NOT NULL DEFAULT false,
    pause_timestamp BIGINT NOT NULL DEFAULT 0,
    pause_reason VARCHAR(200) NOT NULL DEFAULT '',
    network VARCHAR(20) NOT NULL DEFAULT 'testnet',
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_sync_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_operation_tx_signature VARCHAR(88),
    last_operation_type INTEGER
);

-- Create unique index to ensure one system state per network
CREATE UNIQUE INDEX IF NOT EXISTS idx_system_state_network ON system_state(network);

-- Insert default system state if none exists
INSERT INTO system_state (authority, network, pause_reason)
SELECT '', 'testnet', 'System initialized'
WHERE NOT EXISTS (SELECT 1 FROM system_state WHERE network = 'testnet');

-- Update pool_transactions table to remove delegate-related transaction types
-- Update transaction types enum to remove delegate operations
-- Note: This might require application-level updates depending on how enums are handled

-- Add comment about the migration
COMMENT ON TABLE pools IS 'Updated to match smart contract PoolState structure - removed delegate system, added fee tracking and swap pause controls';
COMMENT ON TABLE system_state IS 'Matches smart contract SystemState structure for global pause functionality';

COMMIT;

-- Verification queries (run these after migration to verify success)
/*
-- Verify new columns exist
SELECT column_name, data_type, is_nullable 
FROM information_schema.columns 
WHERE table_name = 'pools' 
AND column_name IN ('owner', 'token_a_vault', 'token_b_vault', 'swaps_paused', 'collected_fees_token_a');

-- Verify system_state table structure
SELECT column_name, data_type, is_nullable 
FROM information_schema.columns 
WHERE table_name = 'system_state';

-- Check that pools have been updated with owner information
SELECT COUNT(*) as pools_with_owner FROM pools WHERE owner IS NOT NULL AND owner != '';

-- Check indexes were created
SELECT indexname FROM pg_indexes WHERE tablename = 'pools' AND indexname LIKE 'idx_pools_%';
*/ 