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

-- Create the FTS5 virtual table with trigram tokenizer
-- The trigram tokenizer enables efficient substring matching like LIKE '%query%'
-- detail='none' reduces index size by not storing term positions
CREATE VIRTUAL TABLE IF NOT EXISTS suttas_fts USING fts5(
    sutta_id UNINDEXED,
    content_plain,
    tokenize='trigram',
    detail='none'
);

-- Trigger for INSERT operations
CREATE TRIGGER IF NOT EXISTS suttas_fts_insert
AFTER INSERT ON suttas
WHEN NEW.content_plain IS NOT NULL
BEGIN
    INSERT INTO suttas_fts (sutta_id, content_plain)
    VALUES (NEW.id, NEW.content_plain);
END;

-- Trigger for UPDATE operations
CREATE TRIGGER IF NOT EXISTS suttas_fts_update
AFTER UPDATE ON suttas
BEGIN
    -- Delete old entry if it exists
    DELETE FROM suttas_fts WHERE sutta_id = OLD.id;

    -- Insert new entry if content_plain is not NULL
    INSERT INTO suttas_fts (sutta_id, content_plain)
    SELECT NEW.id, NEW.content_plain
    WHERE NEW.content_plain IS NOT NULL;
END;

-- Trigger for DELETE operations
CREATE TRIGGER IF NOT EXISTS suttas_fts_delete
AFTER DELETE ON suttas
BEGIN
    DELETE FROM suttas_fts WHERE sutta_id = OLD.id;
END;

