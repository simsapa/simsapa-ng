-- Add is_user_added marker to tables that hold a mix of bootstrap-seeded rows
-- and user-created rows, so the export/import flow can filter to user rows.
-- Default is 1 so that runtime inserts (without an explicit value) are treated
-- as user-added. Bootstrap inserts must set is_user_added = 0 explicitly.

ALTER TABLE books ADD COLUMN is_user_added BOOLEAN NOT NULL DEFAULT 1;
ALTER TABLE bookmark_folders ADD COLUMN is_user_added BOOLEAN NOT NULL DEFAULT 1;
ALTER TABLE bookmark_items ADD COLUMN is_user_added BOOLEAN NOT NULL DEFAULT 1;
