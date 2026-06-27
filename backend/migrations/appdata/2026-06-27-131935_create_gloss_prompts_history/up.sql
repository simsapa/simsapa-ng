CREATE TABLE gloss_prompts_history (
    id INTEGER NOT NULL,
    item_type VARCHAR NOT NULL,
    data_json TEXT NOT NULL,
    created_at DATETIME DEFAULT (CURRENT_TIMESTAMP),
    updated_at DATETIME,
    PRIMARY KEY (id)
);

-- Composite index for the primary query pattern:
-- WHERE item_type = ? ORDER BY updated_at DESC
CREATE INDEX IF NOT EXISTS idx_gloss_prompts_history_type_updated ON gloss_prompts_history(item_type, updated_at);
