-- Drop indexes
DROP INDEX IF EXISTS idx_bookmark_items_folder_sort;
DROP INDEX IF EXISTS idx_bookmark_items_folder_id;
DROP INDEX IF EXISTS idx_bookmark_folders_is_last_session;
DROP INDEX IF EXISTS idx_bookmark_folders_sort_order;

-- Drop tables in reverse order (child tables first, then parent)
DROP TABLE IF EXISTS bookmark_items;
DROP TABLE IF EXISTS bookmark_folders;

-- Vacuum to reclaim space
VACUUM;
