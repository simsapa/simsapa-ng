-- Drop composite indexes
DROP INDEX IF EXISTS idx_book_resources_book_uid_resource_path;
DROP INDEX IF EXISTS idx_book_spine_items_book_uid_spine_index;

-- Drop single column indexes
DROP INDEX IF EXISTS idx_book_resources_book_uid;
DROP INDEX IF EXISTS idx_book_spine_items_spine_item_uid;
DROP INDEX IF EXISTS idx_book_spine_items_book_uid;
DROP INDEX IF EXISTS idx_book_resources_book_id;
DROP INDEX IF EXISTS idx_book_spine_items_book_id;
DROP INDEX IF EXISTS idx_books_document_type;
DROP INDEX IF EXISTS idx_books_uid;

-- Drop tables in reverse order (child tables first, then parent)
DROP TABLE IF EXISTS book_resources;
DROP TABLE IF EXISTS book_spine_items;
DROP TABLE IF EXISTS books;

-- Vacuum to reclaim space
VACUUM;
