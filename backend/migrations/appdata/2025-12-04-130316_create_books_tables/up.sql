CREATE TABLE books (
    id INTEGER NOT NULL,
    uid VARCHAR NOT NULL,
    document_type VARCHAR NOT NULL,
    title VARCHAR,
    author VARCHAR,
    language VARCHAR,
    file_path VARCHAR,
    metadata_json VARCHAR,
    enable_embedded_css BOOLEAN NOT NULL DEFAULT 1,
    created_at DATETIME DEFAULT (CURRENT_TIMESTAMP),
    updated_at DATETIME,
    PRIMARY KEY (id),
    UNIQUE (uid)
);

CREATE TABLE book_spine_items (
    id INTEGER NOT NULL,
    book_id INTEGER NOT NULL,
    book_uid VARCHAR NOT NULL,
    spine_item_uid VARCHAR NOT NULL,
    spine_index INTEGER NOT NULL,
    title VARCHAR,
    language VARCHAR,
    content_html VARCHAR,
    content_plain VARCHAR,
    created_at DATETIME DEFAULT (CURRENT_TIMESTAMP),
    updated_at DATETIME,
    PRIMARY KEY (id),
    UNIQUE (spine_item_uid),
    FOREIGN KEY(book_id) REFERENCES books (id) ON DELETE CASCADE
);

CREATE TABLE book_resources (
    id INTEGER NOT NULL,
    book_id INTEGER NOT NULL,
    book_uid VARCHAR NOT NULL,
    resource_path VARCHAR NOT NULL,
    mime_type VARCHAR,
    content_data BLOB,
    created_at DATETIME DEFAULT (CURRENT_TIMESTAMP),
    updated_at DATETIME,
    PRIMARY KEY (id),
    FOREIGN KEY(book_id) REFERENCES books (id) ON DELETE CASCADE
);

-- B-tree indexes for efficient queries and deletions:

-- Index on books.uid for fast uid lookups
CREATE INDEX IF NOT EXISTS idx_books_uid ON books(uid);

-- Index on books.document_type for filtering by type
CREATE INDEX IF NOT EXISTS idx_books_document_type ON books(document_type);

-- Index on books.language for filtering by language
CREATE INDEX IF NOT EXISTS idx_books_language ON books(language);

-- Indexes on foreign key columns in child tables for fast CASCADE deletes
CREATE INDEX IF NOT EXISTS idx_book_spine_items_book_id ON book_spine_items(book_id);
CREATE INDEX IF NOT EXISTS idx_book_resources_book_id ON book_resources(book_id);

-- Index on book_spine_items.book_uid for fast book-based queries
CREATE INDEX IF NOT EXISTS idx_book_spine_items_book_uid ON book_spine_items(book_uid);

-- Index on book_spine_items.spine_item_uid for fast spine item lookups
CREATE INDEX IF NOT EXISTS idx_book_spine_items_spine_item_uid ON book_spine_items(spine_item_uid);

-- Index on book_spine_items.language for filtering by language
CREATE INDEX IF NOT EXISTS idx_book_spine_items_language ON book_spine_items(language);

-- Composite index for book_uid + spine_index ordering
CREATE INDEX IF NOT EXISTS idx_book_spine_items_book_uid_spine_index ON book_spine_items(book_uid, spine_index);

-- Index on book_resources.book_uid for fast book-based resource queries
CREATE INDEX IF NOT EXISTS idx_book_resources_book_uid ON book_resources(book_uid);

-- Composite index for book_uid + resource_path lookups (common pattern)
CREATE INDEX IF NOT EXISTS idx_book_resources_book_uid_resource_path ON book_resources(book_uid, resource_path);

-- FTS5 trigram indexes will be added with sql script.
