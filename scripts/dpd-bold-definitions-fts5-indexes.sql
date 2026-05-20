-- Migration: FTS5 trigram fulltext search for bold_definitions
--
-- Two virtual tables:
--   bold_definitions_fts      over commentary_plain  — Contains Match
--   bold_definitions_bold_fts over bold              — DPD Lookup / Headword Match substring
--
-- Trigram tokenizer accelerates LIKE '%query%' style substring matching
-- over the ~360k rows in bold_definitions without full scans.

-- =====================================================================
-- bold_definitions_fts (commentary_plain)
-- =====================================================================

DROP TRIGGER IF EXISTS bold_definitions_fts_insert;
DROP TRIGGER IF EXISTS bold_definitions_fts_update;
DROP TRIGGER IF EXISTS bold_definitions_fts_delete;
DROP TABLE IF EXISTS bold_definitions_fts;

-- PERFORMANCE: the source row id (bold_definitions.id) is stored as the FTS5
-- `rowid`, NOT as a separate UNINDEXED column. FTS5 has no secondary indexes,
-- so a `WHERE bold_definitions_id = ?` lookup against an UNINDEXED column is a
-- FULL TABLE SCAN; the per-row delete/update triggers would then scan the whole
-- FTS table once per affected row. Using the rowid makes those lookups
-- O(log n). Queries join `f.rowid = bold_definitions.id` instead of the old
-- `f.bold_definitions_id`.
CREATE VIRTUAL TABLE bold_definitions_fts USING fts5(
    commentary_plain,
    tokenize='trigram',
    detail='none'
);

INSERT INTO bold_definitions_fts (rowid, commentary_plain)
SELECT id, commentary_plain
FROM bold_definitions
WHERE commentary_plain IS NOT NULL AND commentary_plain != '';

CREATE TRIGGER bold_definitions_fts_insert
AFTER INSERT ON bold_definitions
WHEN NEW.commentary_plain IS NOT NULL AND NEW.commentary_plain != ''
BEGIN
    INSERT INTO bold_definitions_fts (rowid, commentary_plain)
    VALUES (NEW.id, NEW.commentary_plain);
END;

CREATE TRIGGER bold_definitions_fts_update
AFTER UPDATE ON bold_definitions
BEGIN
    DELETE FROM bold_definitions_fts WHERE rowid = OLD.id;
    INSERT INTO bold_definitions_fts (rowid, commentary_plain)
    SELECT NEW.id, NEW.commentary_plain
    WHERE NEW.commentary_plain IS NOT NULL AND NEW.commentary_plain != '';
END;

CREATE TRIGGER bold_definitions_fts_delete
AFTER DELETE ON bold_definitions
BEGIN
    DELETE FROM bold_definitions_fts WHERE rowid = OLD.id;
END;

INSERT INTO bold_definitions_fts(bold_definitions_fts) VALUES('optimize');

-- =====================================================================
-- bold_definitions_bold_fts (bold)
-- =====================================================================

DROP TRIGGER IF EXISTS bold_definitions_bold_fts_insert;
DROP TRIGGER IF EXISTS bold_definitions_bold_fts_update;
DROP TRIGGER IF EXISTS bold_definitions_bold_fts_delete;
DROP TABLE IF EXISTS bold_definitions_bold_fts;

-- PERFORMANCE: rowid carries bold_definitions.id (see the note on
-- bold_definitions_fts above); deletes/updates use the fast rowid path.
CREATE VIRTUAL TABLE bold_definitions_bold_fts USING fts5(
    bold,
    bold_ascii,
    tokenize='trigram',
    detail='none'
);

INSERT INTO bold_definitions_bold_fts (rowid, bold, bold_ascii)
SELECT id, bold, bold_ascii
FROM bold_definitions
WHERE bold IS NOT NULL AND bold != '';

CREATE TRIGGER bold_definitions_bold_fts_insert
AFTER INSERT ON bold_definitions
WHEN NEW.bold IS NOT NULL AND NEW.bold != ''
BEGIN
    INSERT INTO bold_definitions_bold_fts (rowid, bold, bold_ascii)
    VALUES (NEW.id, NEW.bold, NEW.bold_ascii);
END;

CREATE TRIGGER bold_definitions_bold_fts_update
AFTER UPDATE ON bold_definitions
BEGIN
    DELETE FROM bold_definitions_bold_fts WHERE rowid = OLD.id;
    INSERT INTO bold_definitions_bold_fts (rowid, bold, bold_ascii)
    SELECT NEW.id, NEW.bold, NEW.bold_ascii
    WHERE NEW.bold IS NOT NULL AND NEW.bold != '';
END;

CREATE TRIGGER bold_definitions_bold_fts_delete
AFTER DELETE ON bold_definitions
BEGIN
    DELETE FROM bold_definitions_bold_fts WHERE rowid = OLD.id;
END;

INSERT INTO bold_definitions_bold_fts(bold_definitions_bold_fts) VALUES('optimize');

-- Single VACUUM at the end of the script.
VACUUM;
