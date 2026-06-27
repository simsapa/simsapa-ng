-- Drop index
DROP INDEX IF EXISTS idx_gloss_prompts_history_type_updated;

-- Drop table
DROP TABLE IF EXISTS gloss_prompts_history;

-- Vacuum to reclaim space
VACUUM;
