-- Migration: FTS5 fulltext search for dpd_headwords.lemma_1 for efficient "%{}%" LIKE queries
-- This enables fast substring matching for DPD headword searches

-- Drop existing FTS table and triggers if they exist
-- This ensures we start fresh with the new schema
DROP TRIGGER IF EXISTS dpd_headwords_fts_insert;
DROP TRIGGER IF EXISTS dpd_headwords_fts_update;
DROP TRIGGER IF EXISTS dpd_headwords_fts_delete;
DROP TABLE IF EXISTS dpd_headwords_fts;

-- Create the FTS5 virtual table with trigram tokenizer
-- The trigram tokenizer enables efficient substring matching like LIKE '%query%'
-- detail='none' reduces index size by not storing term positions
CREATE VIRTUAL TABLE dpd_headwords_fts USING fts5(
    headword_id UNINDEXED,  -- Store the reference to original table, but don't index it
    lemma_1,                -- The primary field we want to search for contains-match
    tokenize='trigram',     -- Use trigram tokenizer for substring search
    detail='none'           -- Reduce index size by not storing term positions
);

-- Populate the FTS table with existing data
-- Only insert rows where lemma_1 is not NULL (though it should always have a value)
INSERT INTO dpd_headwords_fts (headword_id, lemma_1)
SELECT id, lemma_1
FROM dpd_headwords
WHERE lemma_1 IS NOT NULL AND lemma_1 != '';

-- Create triggers to keep FTS table in sync with main table

-- Trigger for INSERT operations
CREATE TRIGGER dpd_headwords_fts_insert
AFTER INSERT ON dpd_headwords
WHEN NEW.lemma_1 IS NOT NULL AND NEW.lemma_1 != ''
BEGIN
    INSERT INTO dpd_headwords_fts (headword_id, lemma_1)
    VALUES (NEW.id, NEW.lemma_1);
END;

-- Trigger for UPDATE operations
CREATE TRIGGER dpd_headwords_fts_update
AFTER UPDATE ON dpd_headwords
BEGIN
    -- Delete old entry if it exists
    DELETE FROM dpd_headwords_fts WHERE headword_id = OLD.id;
    
    -- Insert new entry if lemma_1 is not NULL and not empty
    INSERT INTO dpd_headwords_fts (headword_id, lemma_1)
    SELECT NEW.id, NEW.lemma_1
    WHERE NEW.lemma_1 IS NOT NULL AND NEW.lemma_1 != '';
END;

-- Trigger for DELETE operations
CREATE TRIGGER dpd_headwords_fts_delete
AFTER DELETE ON dpd_headwords
BEGIN
    DELETE FROM dpd_headwords_fts WHERE headword_id = OLD.id;
END;

-- Optimize the FTS index
INSERT INTO dpd_headwords_fts(dpd_headwords_fts) VALUES('optimize');

-- Vacuum to reclaim space and optimize database
VACUUM;

