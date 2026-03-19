-- Add workspace_id to virtual_files for workspace scoping
-- URLs are now associated with specific workspaces

CREATE TABLE virtual_files_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path_id INTEGER NOT NULL,
    workspace_id INTEGER NOT NULL,
    FOREIGN KEY (file_path_id) REFERENCES file_paths(id) ON DELETE CASCADE,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    UNIQUE(file_path_id, workspace_id)
);

-- Migrate existing virtual files using playlist associations
-- URLs in playlists get associated with their workspace
-- URLs not in any playlist are orphaned and dropped
INSERT INTO virtual_files_new (file_path_id, workspace_id)
SELECT DISTINCT vf.file_path_id, pi.workspace_id
FROM virtual_files vf
JOIN playlist_items pi ON vf.file_path_id = pi.file_path_id;

DROP TABLE virtual_files;
ALTER TABLE virtual_files_new RENAME TO virtual_files;

CREATE INDEX idx_virtual_files_file_path ON virtual_files(file_path_id);
CREATE INDEX idx_virtual_files_workspace ON virtual_files(workspace_id);
