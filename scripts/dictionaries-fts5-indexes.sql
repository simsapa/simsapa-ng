-- Standalone script to add FTS5 index and performance indexes to existing dictionaries database
-- Run with: sqlite3 <path_to_dictionaries.sqlite3> < scripts/add-dict-words-fts5.sql
--
-- NOTE: Modifying this script (e.g. adding/removing indexed columns on dict_words_fts)
-- requires a manual re-bootstrap of the dictionaries DB. There is no Diesel migration —
-- the FTS table and triggers are recreated by re-running this script.

-- Drop existing FTS table and triggers if they exist
-- This ensures we start fresh with the new schema
DROP TRIGGER IF EXISTS dict_words_fts_insert;
DROP TRIGGER IF EXISTS dict_words_fts_update;
DROP TRIGGER IF EXISTS dict_words_fts_delete;
DROP TABLE IF EXISTS dict_words_fts;

-- Create the FTS5 virtual table with trigram tokenizer for dictionary headwords + definitions.
-- The trigram tokenizer enables efficient substring matching like LIKE '%query%'
-- Both `word` and `definition_plain` are indexed with the trigram tokenizer can serve
-- substring searches against either column.
-- detail='none' reduces index size by not storing term positions
--
-- PERFORMANCE: the source row id (dict_words.id) is stored as the FTS5 `rowid`,
-- NOT as a separate UNINDEXED column. FTS5 has no secondary indexes, so a
-- `WHERE dict_word_id = ?` lookup against an UNINDEXED column is a FULL TABLE
-- SCAN. The per-row delete/update triggers below run such a lookup once per
-- affected `dict_words` row, so a cascade delete of an N-row dictionary became
-- N full scans of the whole FTS table (e.g. ~3 min for 2000 rows against a
-- 198k-row FTS). Using the rowid makes those lookups O(log n). Queries join
-- `f.rowid = dict_words.id` instead of the old `f.dict_word_id`.
CREATE VIRTUAL TABLE dict_words_fts USING fts5(
    language UNINDEXED,      -- Store language for filtering, but don't index for search
    dict_label UNINDEXED,    -- Store dict_label for filtering, but don't index for search
    word,                    -- Headword: indexed for substring search
    definition_plain,        -- Definition body: indexed for substring search
    tokenize='trigram',      -- Use trigram tokenizer for substring search
    detail='none'            -- Reduce index size
);

-- Populate the FTS table with existing data.
-- Include any row where at least one of `word` / `definition_plain` is non-null.
-- `rowid` is set to dict_words.id so deletes/updates can use the fast rowid path.
INSERT INTO dict_words_fts (rowid, language, dict_label, word, definition_plain)
SELECT id, language, dict_label, word, definition_plain
FROM dict_words
WHERE word IS NOT NULL OR definition_plain IS NOT NULL;

-- Create triggers to keep FTS table in sync with main table

-- Trigger for INSERT operations
CREATE TRIGGER dict_words_fts_insert
AFTER INSERT ON dict_words
WHEN NEW.word IS NOT NULL OR NEW.definition_plain IS NOT NULL
BEGIN
    INSERT INTO dict_words_fts (rowid, language, dict_label, word, definition_plain)
    VALUES (NEW.id, NEW.language, NEW.dict_label, NEW.word, NEW.definition_plain);
END;

-- Trigger for UPDATE operations
CREATE TRIGGER dict_words_fts_update
AFTER UPDATE ON dict_words
BEGIN
    -- Delete old entry if it exists (fast rowid lookup)
    DELETE FROM dict_words_fts WHERE rowid = OLD.id;

    -- Insert new entry if at least one indexed column is non-null
    INSERT INTO dict_words_fts (rowid, language, dict_label, word, definition_plain)
    SELECT NEW.id, NEW.language, NEW.dict_label, NEW.word, NEW.definition_plain
    WHERE NEW.word IS NOT NULL OR NEW.definition_plain IS NOT NULL;
END;

-- Trigger for DELETE operations (fast rowid lookup)
CREATE TRIGGER dict_words_fts_delete
AFTER DELETE ON dict_words
BEGIN
    DELETE FROM dict_words_fts WHERE rowid = OLD.id;
END;

-- Optimize the FTS index
INSERT INTO dict_words_fts(dict_words_fts) VALUES('optimize');

-- Vacuum to reclaim space and optimize database
VACUUM;
