-- Drop composite indexes
DROP INDEX IF EXISTS idx_suttas_title_ascii_language;
DROP INDEX IF EXISTS idx_suttas_nikaya_language;
DROP INDEX IF EXISTS idx_suttas_source_uid_language;

-- Drop single column indexes
DROP INDEX IF EXISTS idx_suttas_nikaya;
DROP INDEX IF EXISTS idx_suttas_sutta_ref;
DROP INDEX IF EXISTS idx_suttas_source_uid;

-- Drop original indexes
DROP INDEX IF EXISTS idx_suttas_language_uid;
DROP INDEX IF EXISTS idx_sutta_glosses_sutta_id;
DROP INDEX IF EXISTS idx_sutta_comments_sutta_id;
DROP INDEX IF EXISTS idx_sutta_variants_sutta_id;
DROP INDEX IF EXISTS idx_suttas_language;

DROP TABLE app_settings;
DROP TABLE suttas;
DROP TABLE sutta_variants;
DROP TABLE sutta_comments;
DROP TABLE sutta_glosses;

-- Vacuum to reclaim space
VACUUM;
