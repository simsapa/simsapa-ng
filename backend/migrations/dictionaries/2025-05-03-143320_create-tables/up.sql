CREATE TABLE dictionaries (
    id INTEGER NOT NULL,
    label VARCHAR NOT NULL,
    title VARCHAR NOT NULL,
    dict_type VARCHAR NOT NULL,
    creator VARCHAR,
    description VARCHAR,
    feedback_email VARCHAR,
    feedback_url VARCHAR,
    version VARCHAR,
    created_at DATETIME DEFAULT (CURRENT_TIMESTAMP),
    updated_at DATETIME,
    PRIMARY KEY (id),
    UNIQUE (label)
);

CREATE TABLE dict_words (
    id INTEGER NOT NULL,
    dictionary_id INTEGER NOT NULL,
    dict_label VARCHAR NOT NULL,
    uid VARCHAR NOT NULL,
    word VARCHAR NOT NULL,
    word_ascii VARCHAR NOT NULL,
    language VARCHAR,
    source_uid VARCHAR,
    word_nom_sg VARCHAR,
    inflections VARCHAR,
    phonetic VARCHAR,
    transliteration VARCHAR,
    meaning_order INTEGER,
    definition_plain VARCHAR,
    definition_html VARCHAR,
    summary VARCHAR,
    synonyms VARCHAR,
    antonyms VARCHAR,
    homonyms VARCHAR,
    also_written_as VARCHAR,
    see_also VARCHAR,
    created_at DATETIME DEFAULT (CURRENT_TIMESTAMP),
    updated_at DATETIME,
    indexed_at DATETIME,
    PRIMARY KEY (id),
    FOREIGN KEY(dictionary_id) REFERENCES dictionaries (id) ON DELETE CASCADE,
    UNIQUE (uid)
);

CREATE INDEX dict_words_idx ON dict_words(dict_label, word);
