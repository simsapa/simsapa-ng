-- Drop indexes
DROP INDEX IF EXISTS idx_chanting_recordings_type;
DROP INDEX IF EXISTS idx_chanting_sections_sort_index;
DROP INDEX IF EXISTS idx_chanting_chants_sort_index;
DROP INDEX IF EXISTS idx_chanting_collections_sort_index;
DROP INDEX IF EXISTS idx_chanting_recordings_section_uid;
DROP INDEX IF EXISTS idx_chanting_sections_chant_uid;
DROP INDEX IF EXISTS idx_chanting_chants_collection_uid;
DROP INDEX IF EXISTS idx_chanting_recordings_uid;
DROP INDEX IF EXISTS idx_chanting_sections_uid;
DROP INDEX IF EXISTS idx_chanting_chants_uid;
DROP INDEX IF EXISTS idx_chanting_collections_uid;

-- Drop tables in reverse order (child tables first, then parent)
DROP TABLE IF EXISTS chanting_recordings;
DROP TABLE IF EXISTS chanting_sections;
DROP TABLE IF EXISTS chanting_chants;
DROP TABLE IF EXISTS chanting_collections;

-- Vacuum to reclaim space
VACUUM;
