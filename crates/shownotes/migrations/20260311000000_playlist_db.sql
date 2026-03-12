-- Workspaces: each unique working directory
CREATE TABLE workspaces (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT UNIQUE NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Playlist items: ordered files in a workspace's playlist
CREATE TABLE playlist_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id INTEGER NOT NULL,
    file_path_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    FOREIGN KEY (file_path_id) REFERENCES file_paths(id) ON DELETE CASCADE,
    UNIQUE(workspace_id, file_path_id)
);

CREATE INDEX idx_playlist_items_workspace ON playlist_items(workspace_id, position);

-- File metadata: duration, mime_type, deletion status, timestamps
CREATE TABLE file_metadata (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path_id INTEGER NOT NULL UNIQUE,
    duration_seconds REAL,
    mime_type TEXT,
    deleted INTEGER NOT NULL DEFAULT 0,
    time_added TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (file_path_id) REFERENCES file_paths(id) ON DELETE CASCADE
);

CREATE INDEX idx_file_metadata_file_path ON file_metadata(file_path_id);

-- Virtual files: URLs and non-filesystem entries
CREATE TABLE virtual_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path_id INTEGER NOT NULL UNIQUE,
    FOREIGN KEY (file_path_id) REFERENCES file_paths(id) ON DELETE CASCADE
);

CREATE INDEX idx_virtual_files_file_path ON virtual_files(file_path_id);

-- Aliases: display names keyed by workspace
CREATE TABLE aliases (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path_id INTEGER NOT NULL,
    workspace_id INTEGER NOT NULL,
    alias TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (file_path_id) REFERENCES file_paths(id) ON DELETE CASCADE,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    UNIQUE(file_path_id, workspace_id)
);

CREATE INDEX idx_aliases_file_path ON aliases(file_path_id);
CREATE INDEX idx_aliases_workspace ON aliases(workspace_id);
