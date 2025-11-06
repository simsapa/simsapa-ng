-- Create nikaya_structures table with unique nikaya field
CREATE TABLE IF NOT EXISTS nikaya_structures (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    nikaya TEXT NOT NULL UNIQUE,
    levels TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create xml_fragments table with cst_file and nikaya foreign key
CREATE TABLE IF NOT EXISTS xml_fragments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cst_file TEXT NOT NULL,
    frag_idx INTEGER NOT NULL,
    frag_type TEXT NOT NULL,
    frag_review TEXT,
    nikaya TEXT NOT NULL,
    cst_code TEXT,
    sc_code TEXT,
    content TEXT NOT NULL,
    cst_vagga TEXT,
    cst_sutta TEXT,
    cst_paranum TEXT,
    sc_sutta TEXT,
    start_line INTEGER NOT NULL,
    start_char INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    end_char INTEGER NOT NULL,
    group_levels TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (nikaya) REFERENCES nikaya_structures(nikaya)
);
