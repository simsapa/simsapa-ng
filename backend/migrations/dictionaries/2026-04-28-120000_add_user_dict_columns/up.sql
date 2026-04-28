ALTER TABLE dictionaries ADD COLUMN is_user_imported BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE dictionaries ADD COLUMN language TEXT NULL;
ALTER TABLE dictionaries ADD COLUMN indexed_at TIMESTAMP NULL;
