CREATE TABLE file_paths (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT UNIQUE NOT NULL
);

CREATE TABLE notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path_id INTEGER UNIQUE NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (file_path_id) REFERENCES file_paths(id) ON DELETE CASCADE
);

CREATE INDEX idx_notes_file_path_id ON notes(file_path_id);
