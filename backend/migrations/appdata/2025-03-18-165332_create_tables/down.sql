DROP TABLE app_settings;
DROP TABLE suttas;
DROP TABLE sutta_variants;
DROP TABLE sutta_comments;
DROP TABLE sutta_glosses;

-- Remove FTS5 fulltext search

-- Drop triggers first
DROP TRIGGER IF EXISTS suttas_fts_insert;
DROP TRIGGER IF EXISTS suttas_fts_update;
DROP TRIGGER IF EXISTS suttas_fts_delete;

-- Drop the FTS5 virtual table
DROP TABLE IF EXISTS suttas_fts;

-- Vacuum to reclaim space
VACUUM;
