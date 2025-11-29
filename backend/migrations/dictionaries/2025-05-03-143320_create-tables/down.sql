-- Drop additional indexes
DROP INDEX IF EXISTS dict_words_dict_label_idx;
DROP INDEX IF EXISTS dict_words_idx;

-- Drop main tables
DROP TABLE IF EXISTS dict_words;
DROP TABLE IF EXISTS dictionaries;
