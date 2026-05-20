CREATE TABLE dict_resources (
    id INTEGER NOT NULL,
    dictionary_id INTEGER NOT NULL,
    resource_path VARCHAR NOT NULL,
    mime_type VARCHAR,
    content_data BLOB,
    created_at DATETIME DEFAULT (CURRENT_TIMESTAMP),
    updated_at DATETIME,
    PRIMARY KEY (id),
    FOREIGN KEY(dictionary_id) REFERENCES dictionaries (id) ON DELETE CASCADE
);

-- Lookup by dictionary_id + resource_path (the serving/render path).
CREATE INDEX dict_resources_dict_id_path_idx ON dict_resources(dictionary_id, resource_path);
