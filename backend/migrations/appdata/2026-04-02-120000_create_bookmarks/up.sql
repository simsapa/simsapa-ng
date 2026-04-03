CREATE TABLE bookmark_folders (
    id INTEGER NOT NULL,
    name VARCHAR NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    is_last_session BOOLEAN NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT (CURRENT_TIMESTAMP),
    updated_at DATETIME,
    PRIMARY KEY (id)
);

CREATE TABLE bookmark_items (
    id INTEGER NOT NULL,
    folder_id INTEGER NOT NULL,
    item_uid VARCHAR NOT NULL,
    table_name VARCHAR NOT NULL,
    title VARCHAR,
    tab_group VARCHAR NOT NULL,
    scroll_position REAL NOT NULL DEFAULT 0.0,
    find_query VARCHAR NOT NULL DEFAULT '',
    find_match_index INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT (CURRENT_TIMESTAMP),
    updated_at DATETIME,
    PRIMARY KEY (id),
    FOREIGN KEY(folder_id) REFERENCES bookmark_folders (id) ON DELETE CASCADE
);

-- B-tree indexes for efficient queries and deletions:

-- Index on sort_order for ordering queries
CREATE INDEX IF NOT EXISTS idx_bookmark_folders_sort_order ON bookmark_folders(sort_order);

-- Index on is_last_session for filtering last session folders
CREATE INDEX IF NOT EXISTS idx_bookmark_folders_is_last_session ON bookmark_folders(is_last_session);

-- Index on folder_id foreign key for fast CASCADE deletes and queries
CREATE INDEX IF NOT EXISTS idx_bookmark_items_folder_id ON bookmark_items(folder_id);

-- Composite index on folder_id + sort_order for ordered item listing within a folder
CREATE INDEX IF NOT EXISTS idx_bookmark_items_folder_sort ON bookmark_items(folder_id, sort_order);
