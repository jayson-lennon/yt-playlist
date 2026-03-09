CREATE TABLE sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path_id INTEGER NOT NULL,
    source_url TEXT NOT NULL,
    label TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (file_path_id) REFERENCES file_paths(id) ON DELETE CASCADE
);

CREATE INDEX idx_sources_file_path_id ON sources(file_path_id);
