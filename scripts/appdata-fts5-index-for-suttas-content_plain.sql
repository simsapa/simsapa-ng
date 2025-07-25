-- Migration: Add FTS5 fulltext search for suttas table

-- Create the FTS5 virtual table with trigram tokenizer
-- The trigram tokenizer enables efficient substring matching like LIKE '%query%'
-- detail='none' reduces index size by not storing term positions
CREATE VIRTUAL TABLE IF NOT EXISTS suttas_fts USING fts5(
    sutta_id UNINDEXED,  -- Store the reference to original table, but don't index it
    content_plain,       -- The field we want to search
    tokenize='trigram', -- Use trigram tokenizer for substring search
    detail='none'       -- Reduce index size
);

-- Populate the FTS table with existing data
-- Only insert rows where content_plain is not NULL
INSERT INTO suttas_fts (sutta_id, content_plain)
SELECT id, content_plain
FROM suttas
WHERE content_plain IS NOT NULL;

-- Create triggers to keep FTS table in sync with main table

-- Trigger for INSERT operations
CREATE TRIGGER IF NOT EXISTS suttas_fts_insert
AFTER INSERT ON suttas
WHEN NEW.content_plain IS NOT NULL
BEGIN
    INSERT INTO suttas_fts (sutta_id, content_plain)
    VALUES (NEW.id, NEW.content_plain);
END;

-- Trigger for UPDATE operations
CREATE TRIGGER IF NOT EXISTS suttas_fts_update
AFTER UPDATE ON suttas
BEGIN
    -- Delete old entry if it exists
    DELETE FROM suttas_fts WHERE sutta_id = OLD.id;

    -- Insert new entry if content_plain is not NULL
    INSERT INTO suttas_fts (sutta_id, content_plain)
    SELECT NEW.id, NEW.content_plain
    WHERE NEW.content_plain IS NOT NULL;
END;

-- Trigger for DELETE operations
CREATE TRIGGER IF NOT EXISTS suttas_fts_delete
AFTER DELETE ON suttas
BEGIN
    DELETE FROM suttas_fts WHERE sutta_id = OLD.id;
END;

-- Optimize the FTS index
INSERT INTO suttas_fts(suttas_fts) VALUES('optimize');

-- Vacuum to reclaim space and optimize database
VACUUM;
