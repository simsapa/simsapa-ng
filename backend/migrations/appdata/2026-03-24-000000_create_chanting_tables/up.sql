CREATE TABLE chanting_collections (
    id INTEGER NOT NULL,
    uid VARCHAR NOT NULL,
    title VARCHAR NOT NULL,
    description VARCHAR,
    language VARCHAR NOT NULL DEFAULT 'pali',
    sort_index INTEGER NOT NULL DEFAULT 0,
    is_user_added BOOLEAN NOT NULL DEFAULT 0,
    metadata_json VARCHAR,
    created_at DATETIME DEFAULT (CURRENT_TIMESTAMP),
    updated_at DATETIME,
    PRIMARY KEY (id),
    UNIQUE (uid)
);

CREATE TABLE chanting_chants (
    id INTEGER NOT NULL,
    uid VARCHAR NOT NULL,
    collection_uid VARCHAR NOT NULL,
    title VARCHAR NOT NULL,
    description VARCHAR,
    sort_index INTEGER NOT NULL DEFAULT 0,
    is_user_added BOOLEAN NOT NULL DEFAULT 0,
    metadata_json VARCHAR,
    created_at DATETIME DEFAULT (CURRENT_TIMESTAMP),
    updated_at DATETIME,
    PRIMARY KEY (id),
    UNIQUE (uid),
    FOREIGN KEY(collection_uid) REFERENCES chanting_collections (uid) ON DELETE CASCADE
);

CREATE TABLE chanting_sections (
    id INTEGER NOT NULL,
    uid VARCHAR NOT NULL,
    chant_uid VARCHAR NOT NULL,
    title VARCHAR NOT NULL,
    content_pali VARCHAR NOT NULL,
    sort_index INTEGER NOT NULL DEFAULT 0,
    is_user_added BOOLEAN NOT NULL DEFAULT 0,
    metadata_json VARCHAR,
    created_at DATETIME DEFAULT (CURRENT_TIMESTAMP),
    updated_at DATETIME,
    PRIMARY KEY (id),
    UNIQUE (uid),
    FOREIGN KEY(chant_uid) REFERENCES chanting_chants (uid) ON DELETE CASCADE
);

CREATE TABLE chanting_recordings (
    id INTEGER NOT NULL,
    uid VARCHAR NOT NULL,
    section_uid VARCHAR NOT NULL,
    file_name VARCHAR NOT NULL,
    recording_type VARCHAR NOT NULL,
    label VARCHAR,
    duration_ms INTEGER NOT NULL DEFAULT 0,
    markers_json VARCHAR,
    created_at DATETIME DEFAULT (CURRENT_TIMESTAMP),
    updated_at DATETIME,
    PRIMARY KEY (id),
    UNIQUE (uid),
    FOREIGN KEY(section_uid) REFERENCES chanting_sections (uid) ON DELETE CASCADE
);

-- B-tree indexes for efficient queries and deletions:

-- Index on uid columns for fast lookups
CREATE INDEX IF NOT EXISTS idx_chanting_collections_uid ON chanting_collections(uid);
CREATE INDEX IF NOT EXISTS idx_chanting_chants_uid ON chanting_chants(uid);
CREATE INDEX IF NOT EXISTS idx_chanting_sections_uid ON chanting_sections(uid);
CREATE INDEX IF NOT EXISTS idx_chanting_recordings_uid ON chanting_recordings(uid);

-- Indexes on foreign key columns for fast CASCADE deletes and queries
CREATE INDEX IF NOT EXISTS idx_chanting_chants_collection_uid ON chanting_chants(collection_uid);
CREATE INDEX IF NOT EXISTS idx_chanting_sections_chant_uid ON chanting_sections(chant_uid);
CREATE INDEX IF NOT EXISTS idx_chanting_recordings_section_uid ON chanting_recordings(section_uid);

-- Index on sort_index for ordering queries
CREATE INDEX IF NOT EXISTS idx_chanting_collections_sort_index ON chanting_collections(sort_index);
CREATE INDEX IF NOT EXISTS idx_chanting_chants_sort_index ON chanting_chants(collection_uid, sort_index);
CREATE INDEX IF NOT EXISTS idx_chanting_sections_sort_index ON chanting_sections(chant_uid, sort_index);

-- Index on recording_type for filtering
CREATE INDEX IF NOT EXISTS idx_chanting_recordings_type ON chanting_recordings(recording_type);
