CREATE TABLE app_settings (
    id INTEGER NOT NULL,
    "key" VARCHAR NOT NULL,
    value VARCHAR,
    created_at DATETIME DEFAULT (CURRENT_TIMESTAMP),
    updated_at DATETIME,
    PRIMARY KEY (id),
    UNIQUE ("key")
);

CREATE TABLE suttas (
    id INTEGER NOT NULL,
    uid VARCHAR NOT NULL,
    sutta_ref VARCHAR NOT NULL,
    nikaya VARCHAR NOT NULL,
    language VARCHAR NOT NULL,
    group_path VARCHAR,
    group_index INTEGER,
    order_index INTEGER,
    sutta_range_group VARCHAR,
    sutta_range_start INTEGER,
    sutta_range_end INTEGER,
    title VARCHAR,
    title_ascii VARCHAR,
    title_pali VARCHAR,
    title_trans VARCHAR,
    description VARCHAR,
    content_plain VARCHAR,
    content_html VARCHAR,
    content_json VARCHAR,
    content_json_tmpl VARCHAR,
    source_uid VARCHAR,
    source_info VARCHAR,
    source_language VARCHAR,
    message VARCHAR,
    copyright VARCHAR,
    license VARCHAR,
    created_at DATETIME DEFAULT (CURRENT_TIMESTAMP),
    updated_at DATETIME,
    indexed_at DATETIME,
    PRIMARY KEY (id),
    UNIQUE (uid)
);

CREATE TABLE sutta_variants (
    id INTEGER NOT NULL,
    sutta_id INTEGER NOT NULL,
    sutta_uid VARCHAR NOT NULL,
    language VARCHAR,
    source_uid VARCHAR,
    content_json VARCHAR,
    created_at DATETIME DEFAULT (CURRENT_TIMESTAMP),
    updated_at DATETIME,
    PRIMARY KEY (id),
    FOREIGN KEY(sutta_id) REFERENCES suttas (id) ON DELETE CASCADE
);

CREATE TABLE sutta_comments (
    id INTEGER NOT NULL,
    sutta_id INTEGER NOT NULL,
    sutta_uid VARCHAR NOT NULL,
    language VARCHAR,
    source_uid VARCHAR,
    content_json VARCHAR,
    created_at DATETIME DEFAULT (CURRENT_TIMESTAMP),
    updated_at DATETIME,
    PRIMARY KEY (id),
    FOREIGN KEY(sutta_id) REFERENCES suttas (id) ON DELETE CASCADE
);

CREATE TABLE sutta_glosses (
    id INTEGER NOT NULL,
    sutta_id INTEGER NOT NULL,
    sutta_uid VARCHAR NOT NULL,
    language VARCHAR,
    source_uid VARCHAR,
    content_json VARCHAR,
    created_at DATETIME DEFAULT (CURRENT_TIMESTAMP),
    updated_at DATETIME,
    PRIMARY KEY (id),
    FOREIGN KEY(sutta_id) REFERENCES suttas (id) ON DELETE CASCADE
);

-- B-tree indexes for efficient queries and deletions:

-- Index on suttas.language for fast filtering by language (used in removal operations)
CREATE INDEX IF NOT EXISTS idx_suttas_language ON suttas(language);

-- Indexes on foreign key columns in child tables for fast CASCADE deletes
CREATE INDEX IF NOT EXISTS idx_sutta_variants_sutta_id ON sutta_variants(sutta_id);
CREATE INDEX IF NOT EXISTS idx_sutta_comments_sutta_id ON sutta_comments(sutta_id);
CREATE INDEX IF NOT EXISTS idx_sutta_glosses_sutta_id ON sutta_glosses(sutta_id);

-- Composite index for common query patterns (language + uid lookups)
CREATE INDEX IF NOT EXISTS idx_suttas_language_uid ON suttas(language, uid);

-- Index on suttas.source_uid for fast source filtering
CREATE INDEX IF NOT EXISTS idx_suttas_source_uid ON suttas(source_uid);

-- Index on suttas.sutta_ref for fast reference lookup
CREATE INDEX IF NOT EXISTS idx_suttas_sutta_ref ON suttas(sutta_ref);

-- Index on suttas.nikaya for fast nikaya filtering
CREATE INDEX IF NOT EXISTS idx_suttas_nikaya ON suttas(nikaya);

-- Composite indexes for common filter combinations

-- Composite index for source_uid + language filtering
CREATE INDEX IF NOT EXISTS idx_suttas_source_uid_language ON suttas(source_uid, language);

-- Composite index for nikaya + language filtering
CREATE INDEX IF NOT EXISTS idx_suttas_nikaya_language ON suttas(nikaya, language);

-- Composite index for title searches (ASCII for case-insensitive search)
CREATE INDEX IF NOT EXISTS idx_suttas_title_ascii_language ON suttas(title_ascii, language);

-- FTS5 trigram indexes will be added with sql script.
