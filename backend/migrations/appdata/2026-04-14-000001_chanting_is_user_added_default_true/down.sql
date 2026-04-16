-- Revert DEFAULT on is_user_added back to 0 for chanting tables.

CREATE TABLE chanting_collections_new (
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
INSERT INTO chanting_collections_new
    (id, uid, title, description, language, sort_index, is_user_added, metadata_json, created_at, updated_at)
SELECT id, uid, title, description, language, sort_index, is_user_added, metadata_json, created_at, updated_at
FROM chanting_collections;
DROP TABLE chanting_collections;
ALTER TABLE chanting_collections_new RENAME TO chanting_collections;

CREATE TABLE chanting_chants_new (
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
INSERT INTO chanting_chants_new
    (id, uid, collection_uid, title, description, sort_index, is_user_added, metadata_json, created_at, updated_at)
SELECT id, uid, collection_uid, title, description, sort_index, is_user_added, metadata_json, created_at, updated_at
FROM chanting_chants;
DROP TABLE chanting_chants;
ALTER TABLE chanting_chants_new RENAME TO chanting_chants;

CREATE TABLE chanting_sections_new (
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
INSERT INTO chanting_sections_new
    (id, uid, chant_uid, title, content_pali, sort_index, is_user_added, metadata_json, created_at, updated_at)
SELECT id, uid, chant_uid, title, content_pali, sort_index, is_user_added, metadata_json, created_at, updated_at
FROM chanting_sections;
DROP TABLE chanting_sections;
ALTER TABLE chanting_sections_new RENAME TO chanting_sections;

CREATE INDEX IF NOT EXISTS idx_chanting_collections_uid ON chanting_collections(uid);
CREATE INDEX IF NOT EXISTS idx_chanting_chants_uid ON chanting_chants(uid);
CREATE INDEX IF NOT EXISTS idx_chanting_sections_uid ON chanting_sections(uid);
CREATE INDEX IF NOT EXISTS idx_chanting_chants_collection_uid ON chanting_chants(collection_uid);
CREATE INDEX IF NOT EXISTS idx_chanting_sections_chant_uid ON chanting_sections(chant_uid);
CREATE INDEX IF NOT EXISTS idx_chanting_collections_sort_index ON chanting_collections(sort_index);
CREATE INDEX IF NOT EXISTS idx_chanting_chants_sort_index ON chanting_chants(collection_uid, sort_index);
CREATE INDEX IF NOT EXISTS idx_chanting_sections_sort_index ON chanting_sections(chant_uid, sort_index);
