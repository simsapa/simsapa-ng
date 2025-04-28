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
