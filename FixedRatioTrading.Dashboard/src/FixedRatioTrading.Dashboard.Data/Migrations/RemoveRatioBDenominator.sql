-- Migration: Remove redundant RatioBDenominator field
-- Description: RatioBDenominator is always 1 due to business logic, so it's redundant
-- Date: Generated automatically
-- 
-- This migration:
-- 1. Adds a new Ratio column 
-- 2. Copies RatioANumerator values to Ratio
-- 3. Drops the redundant RatioBDenominator column
-- 4. Drops the old RatioANumerator column

BEGIN TRANSACTION;

-- Step 1: Add new Ratio column
ALTER TABLE Pools ADD COLUMN Ratio INTEGER NOT NULL DEFAULT 0;

-- Step 2: Copy existing RatioANumerator values to Ratio column
UPDATE Pools SET Ratio = RatioANumerator;

-- Step 3: Drop the redundant RatioBDenominator column
ALTER TABLE Pools DROP COLUMN RatioBDenominator;

-- Step 4: Drop the old RatioANumerator column
ALTER TABLE Pools DROP COLUMN RatioANumerator;

COMMIT TRANSACTION; 