-- Migration: Add TokenAIsTheMultiple field to Pools table
-- Date: $(date)
-- Description: Adds the TokenAIsTheMultiple boolean field to store which token is the multiple in the ratio calculation

-- Add the new column with default value false
ALTER TABLE Pools ADD COLUMN TokenAIsTheMultiple BOOLEAN NOT NULL DEFAULT FALSE;

-- Update comment
COMMENT ON COLUMN Pools.TokenAIsTheMultiple IS 'Whether TokenA is the multiple token in the ratio calculation. Used to determine how to display the ratio string.';

-- No need to update existing rows since default false is appropriate
-- (existing pools will need to be re-synced from blockchain to get correct values) 