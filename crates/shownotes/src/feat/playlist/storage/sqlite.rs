use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use error_stack::{Report, ResultExt};
use marked_path::CanonicalPath;
use sqlx::SqlitePool;
use wherror::Error;

use super::super::{FileMetadata, IoError, PlaylistData, PlaylistStorage};
use crate::common::domain::ItemPath;

fn path_to_item_path(path: &str) -> Option<ItemPath> {
    if path.starts_with("http://") || path.starts_with("https://") {
        Some(ItemPath::Url(path.to_string()))
    } else {
        CanonicalPath::from_path(PathBuf::from(path))
            .ok()
            .map(ItemPath::File)
    }
}

fn item_path_to_path(item_path: &ItemPath) -> PathBuf {
    match item_path {
        ItemPath::File(canonical) => canonical.to_path_buf(),
        ItemPath::Url(url) => PathBuf::from(url),
    }
}

#[derive(Debug, Error)]
#[error("database operation failed")]
#[allow(dead_code)]
pub struct SqliteStorageError;

#[derive(Debug, Clone)]
pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn get_or_create_workspace(&self, path: &Path) -> Result<i64, Report<IoError>> {
        let path_str = path.to_string_lossy();

        if let Some(id) = sqlx::query_scalar::<_, i64>("SELECT id FROM workspaces WHERE path = ?")
            .bind(path_str.as_ref())
            .fetch_optional(&self.pool)
            .await
            .change_context(IoError)?
        {
            return Ok(id);
        }

        let id =
            sqlx::query_scalar::<_, i64>("INSERT INTO workspaces (path) VALUES (?) RETURNING id")
                .bind(path_str.as_ref())
                .fetch_one(&self.pool)
                .await
                .change_context(IoError)?;

        Ok(id)
    }

    async fn get_workspace(&self, path: &Path) -> Result<Option<i64>, Report<IoError>> {
        let path_str = path.to_string_lossy();

        let id = sqlx::query_scalar::<_, i64>("SELECT id FROM workspaces WHERE path = ?")
            .bind(path_str.as_ref())
            .fetch_optional(&self.pool)
            .await
            .change_context(IoError)?;

        Ok(id)
    }

    async fn get_or_create_file_path(&self, path: &Path) -> Result<i64, Report<IoError>> {
        let path_str = path.to_string_lossy();

        if let Some(id) = sqlx::query_scalar::<_, i64>("SELECT id FROM file_paths WHERE path = ?")
            .bind(path_str.as_ref())
            .fetch_optional(&self.pool)
            .await
            .change_context(IoError)?
        {
            return Ok(id);
        }

        let id =
            sqlx::query_scalar::<_, i64>("INSERT INTO file_paths (path) VALUES (?) RETURNING id")
                .bind(path_str.as_ref())
                .fetch_one(&self.pool)
                .await
                .change_context(IoError)?;

        Ok(id)
    }

    async fn get_file_path(&self, path: &Path) -> Result<Option<i64>, Report<IoError>> {
        let path_str = path.to_string_lossy();
        let id = sqlx::query_scalar::<_, i64>("SELECT id FROM file_paths WHERE path = ?")
            .bind(path_str.as_ref())
            .fetch_optional(&self.pool)
            .await
            .change_context(IoError)?;
        Ok(id)
    }

    /// Resolves the display alias for a file using the priority system.
    ///
    /// # Alias Resolution Priority
    ///
    /// 1. Workspace-specific alias (if exists for current workspace)
    /// 2. Most recently updated alias from any workspace (fallback)
    /// 3. `None` (caller should display filename instead)
    async fn resolve_alias_by_id(
        &self,
        file_path_id: i64,
        workspace_id: i64,
    ) -> Result<Option<String>, Report<IoError>> {
        // First, try to get workspace-specific alias
        if let Some(alias) = sqlx::query_scalar::<_, String>(
            "SELECT alias FROM aliases WHERE file_path_id = ? AND workspace_id = ?",
        )
        .bind(file_path_id)
        .bind(workspace_id)
        .fetch_optional(&self.pool)
        .await
        .change_context(IoError)?
        {
            return Ok(Some(alias));
        }

        // Fallback to most recently updated alias from any workspace
        let fallback = sqlx::query_scalar::<_, String>(
            "SELECT alias FROM aliases WHERE file_path_id = ? ORDER BY updated_at DESC LIMIT 1",
        )
        .bind(file_path_id)
        .fetch_optional(&self.pool)
        .await
        .change_context(IoError)?;

        Ok(fallback)
    }

    async fn upsert_file_metadata(
        &self,
        file_path_id: i64,
        metadata: &FileMetadata,
    ) -> Result<(), Report<IoError>> {
        let duration_seconds = metadata.duration.map(|d| d.as_secs_f64());
        let time_added = metadata
            .time_added
            .as_ref()
            .map_or_else(|| "datetime('now')".to_string(), ToString::to_string);
        let deleted = i32::from(metadata.deleted);

        sqlx::query(
            r#"
            INSERT INTO file_metadata (file_path_id, duration_seconds, mime_type, deleted, time_added)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(file_path_id) DO UPDATE SET
                duration_seconds = excluded.duration_seconds,
                mime_type = excluded.mime_type,
                deleted = excluded.deleted,
                time_added = excluded.time_added
            "#,
        )
        .bind(file_path_id)
        .bind(duration_seconds)
        .bind(&metadata.mime_type)
        .bind(deleted)
        .bind(&time_added)
        .execute(&self.pool)
        .await
        .change_context(IoError)?;

        Ok(())
    }

    async fn upsert_virtual_file(
        &self,
        file_path_id: i64,
        workspace_id: i64,
        is_virtual: bool,
    ) -> Result<(), Report<IoError>> {
        if is_virtual {
            sqlx::query("INSERT OR IGNORE INTO virtual_files (file_path_id, workspace_id) VALUES (?, ?)")
                .bind(file_path_id)
                .bind(workspace_id)
                .execute(&self.pool)
                .await
                .change_context(IoError)?;
        } else {
            sqlx::query("DELETE FROM virtual_files WHERE file_path_id = ? AND workspace_id = ?")
                .bind(file_path_id)
                .bind(workspace_id)
                .execute(&self.pool)
                .await
                .change_context(IoError)?;
        }

        Ok(())
    }

    /// Upserts an alias for a file in a specific workspace.
    ///
    /// The alias will be displayed in the TUI instead of the filename.
    async fn upsert_alias_by_id(
        &self,
        file_path_id: i64,
        workspace_id: i64,
        alias: &str,
    ) -> Result<(), Report<IoError>> {
        sqlx::query(
            r#"
            INSERT INTO aliases (file_path_id, workspace_id, alias, updated_at)
            VALUES (?, ?, ?, datetime('now'))
            ON CONFLICT(file_path_id, workspace_id) DO UPDATE SET
                alias = excluded.alias,
                updated_at = datetime('now')
            "#,
        )
        .bind(file_path_id)
        .bind(workspace_id)
        .bind(alias)
        .execute(&self.pool)
        .await
        .change_context(IoError)?;

        Ok(())
    }

    async fn is_virtual_file(&self, file_path_id: i64, workspace_id: i64) -> Result<bool, Report<IoError>> {
        let exists =
            sqlx::query_scalar::<_, i64>("SELECT 1 FROM virtual_files WHERE file_path_id = ? AND workspace_id = ?")
                .bind(file_path_id)
                .bind(workspace_id)
                .fetch_optional(&self.pool)
                .await
                .change_context(IoError)?;

        Ok(exists.is_some())
    }

    async fn get_file_metadata(
        &self,
        file_path_id: i64,
        workspace_id: i64,
    ) -> Result<FileMetadata, Report<IoError>> {
        let row = sqlx::query_as::<_, (Option<f64>, Option<String>, i32, Option<String>)>(
            "SELECT duration_seconds, mime_type, deleted, time_added FROM file_metadata WHERE file_path_id = ?",
        )
        .bind(file_path_id)
        .fetch_optional(&self.pool)
        .await
        .change_context(IoError)?;

        let is_virtual = self.is_virtual_file(file_path_id, workspace_id).await?;
        let alias = self.resolve_alias_by_id(file_path_id, workspace_id).await?;

        match row {
            Some((duration_seconds, mime_type, deleted, time_added)) => {
                let duration = duration_seconds
                    .filter(|&d| d.is_finite() && d > 0.0)
                    .map(Duration::from_secs_f64);
                let time_added = time_added.and_then(|s| s.parse().ok());
                Ok(FileMetadata {
                    duration,
                    is_virtual,
                    deleted: deleted != 0,
                    mime_type,
                    time_added,
                    alias,
                })
            }
            None => Ok(FileMetadata {
                duration: None,
                is_virtual,
                deleted: false,
                mime_type: None,
                time_added: None,
                alias,
            }),
        }
    }
}

#[async_trait]
impl PlaylistStorage for SqliteStorage {
    fn name(&self) -> &'static str {
        "sqlite"
    }

    async fn load(
        &self,
        working_directory: &CanonicalPath,
    ) -> Result<PlaylistData, Report<IoError>> {
        let Some(workspace_id) = self.get_workspace(working_directory.as_path()).await? else {
            return Ok(PlaylistData {
                working_directory: working_directory.clone(),
                playlist: Vec::new(),
                files: HashMap::new(),
            });
        };

        let items = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT fp.path, pi.file_path_id
            FROM playlist_items pi
            JOIN file_paths fp ON pi.file_path_id = fp.id
            WHERE pi.workspace_id = ?
            ORDER BY pi.position ASC
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await
        .change_context(IoError)?;

        let playlist: Vec<ItemPath> = items
            .iter()
            .filter_map(|(path, _)| path_to_item_path(path))
            .collect();
        let playlist_file_ids: HashSet<i64> = items.iter().map(|(_, id)| *id).collect();

        let mut files: HashMap<ItemPath, FileMetadata> = HashMap::new();

        for (path_str, file_path_id) in &items {
            if let Some(item_path) = path_to_item_path(path_str) {
                let metadata = self.get_file_metadata(*file_path_id, workspace_id).await?;
                files.insert(item_path, metadata);
            }
        }

        let working_dir_str = working_directory.as_path().to_string_lossy();
        let all_metadata_files = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT fp.path, fm.file_path_id
            FROM file_metadata fm
            JOIN file_paths fp ON fm.file_path_id = fp.id
            WHERE fp.path LIKE ?
            UNION
            SELECT fp.path, fm.file_path_id
            FROM file_metadata fm
            JOIN file_paths fp ON fm.file_path_id = fp.id
            JOIN virtual_files vf ON fm.file_path_id = vf.file_path_id
            WHERE vf.workspace_id = ?
            "#,
        )
        .bind(format!("{}%", working_dir_str.as_ref()))
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await
        .change_context(IoError)?;

        for (path_str, file_path_id) in all_metadata_files {
            if playlist_file_ids.contains(&file_path_id) {
                continue;
            }

            if let Some(item_path) = path_to_item_path(&path_str) {
                let metadata = self.get_file_metadata(file_path_id, workspace_id).await?;
                files.insert(item_path, metadata);
            }
        }

        Ok(PlaylistData {
            working_directory: working_directory.clone(),
            playlist,
            files,
        })
    }

    async fn save(&self, data: &PlaylistData) -> Result<(), Report<IoError>> {
        let workspace_id = self
            .get_or_create_workspace(data.working_directory.as_path())
            .await?;

        let mut file_path_ids: Vec<(PathBuf, i64)> = Vec::new();
        for item_path in &data.playlist {
            let path = item_path_to_path(item_path);
            let file_path_id = self.get_or_create_file_path(&path).await?;
            file_path_ids.push((path, file_path_id));
        }

        let mut tx = self.pool.begin().await.change_context(IoError)?;

        sqlx::query("DELETE FROM playlist_items WHERE workspace_id = ?")
            .bind(workspace_id)
            .execute(&mut *tx)
            .await
            .change_context(IoError)?;

        for (position, (_, file_path_id)) in file_path_ids.iter().enumerate() {
            #[allow(clippy::cast_possible_wrap)]
            let position = position as i64;
            sqlx::query(
                "INSERT INTO playlist_items (workspace_id, file_path_id, position) VALUES (?, ?, ?)",
            )
            .bind(workspace_id)
            .bind(*file_path_id)
            .bind(position)
            .execute(&mut *tx)
            .await
            .change_context(IoError)?;
        }

        tx.commit().await.change_context(IoError)?;

        for (item_path, metadata) in &data.files {
            let path = item_path_to_path(item_path);
            let file_path_id = self.get_or_create_file_path(&path).await?;
            self.upsert_file_metadata(file_path_id, metadata).await?;
            self.upsert_virtual_file(file_path_id, workspace_id, metadata.is_virtual)
                .await?;
        }

        Ok(())
    }

    async fn upsert_alias(
        &self,
        file_path: &CanonicalPath,
        workspace: &CanonicalPath,
        alias: &str,
    ) -> Result<(), Report<IoError>> {
        let file_path_id = self.get_or_create_file_path(file_path.as_path()).await?;
        let workspace_id = self.get_or_create_workspace(workspace.as_path()).await?;
        self.upsert_alias_by_id(file_path_id, workspace_id, alias)
            .await
    }

    async fn delete_alias(
        &self,
        file_path: &CanonicalPath,
        workspace: &CanonicalPath,
    ) -> Result<(), Report<IoError>> {
        let file_path_id = self.get_or_create_file_path(file_path.as_path()).await?;
        let workspace_id = self.get_or_create_workspace(workspace.as_path()).await?;
        sqlx::query("DELETE FROM aliases WHERE file_path_id = ? AND workspace_id = ?")
            .bind(file_path_id)
            .bind(workspace_id)
            .execute(&self.pool)
            .await
            .change_context(IoError)?;
        Ok(())
    }

    async fn resolve_alias(
        &self,
        file_path: &CanonicalPath,
        workspace: &CanonicalPath,
    ) -> Result<Option<String>, Report<IoError>> {
        let Some(file_path_id) = self.get_file_path(file_path.as_path()).await? else {
            return Ok(None);
        };

        // Try to get workspace-specific alias first
        if let Some(workspace_id) = self.get_workspace(workspace.as_path()).await? {
            if let Some(alias) = self.resolve_alias_by_id(file_path_id, workspace_id).await? {
                return Ok(Some(alias));
            }
        }

        // Fallback to most recently updated alias from any workspace
        let fallback = sqlx::query_scalar::<_, String>(
            "SELECT alias FROM aliases WHERE file_path_id = ? ORDER BY updated_at DESC LIMIT 1",
        )
        .bind(file_path_id)
        .fetch_optional(&self.pool)
        .await
        .change_context(IoError)?;

        Ok(fallback)
    }

    async fn get_path_counts(&self) -> Result<HashMap<i64, usize>, Report<IoError>> {
        let rows = sqlx::query_as::<_, (i64, i64)>(
            "SELECT file_path_id, COUNT(DISTINCT workspace_id) as count FROM playlist_items GROUP BY file_path_id",
        )
        .fetch_all(&self.pool)
        .await
        .change_context(IoError)?;

        Ok(rows
            .into_iter()
            .map(|(file_path_id, count)| (file_path_id, usize::try_from(count).unwrap_or(0)))
            .collect())
    }

    async fn resolve_file_path_id(&self, path: &ItemPath) -> Result<Option<i64>, Report<IoError>> {
        let path_buf = item_path_to_path(path);
        self.get_file_path(&path_buf).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feat::note_db::SqliteNoteDb;
    use jiff::Timestamp;
    use tempfile::{NamedTempFile, TempDir};

    fn create_temp_file() -> (NamedTempFile, CanonicalPath) {
        let temp = tempfile::Builder::new()
            .suffix(".mp4")
            .tempfile()
            .unwrap();
        let path = CanonicalPath::from_path(temp.path()).unwrap();
        (temp, path)
    }

    fn create_temp_file_in(dir: &Path, name: &str) -> (NamedTempFile, CanonicalPath) {
        let temp = tempfile::Builder::new()
            .prefix(name)
            .suffix(".mp4")
            .tempfile_in(dir)
            .unwrap();
        let path = CanonicalPath::from_path(temp.path()).unwrap();
        (temp, path)
    }

    async fn create_test_storage() -> SqliteStorage {
        let db = SqliteNoteDb::new("sqlite::memory:").await.unwrap();
        SqliteStorage::new(db.pool().clone())
    }

    #[tokio::test]
    async fn load_empty_workspace_returns_empty_data() {
        // Given an empty storage and workspace.
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();

        // When loading data.
        let result = storage.load(&working_dir).await;

        // Then the result is empty.
        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(data.playlist.is_empty());
        assert!(data.files.is_empty());
        assert_eq!(data.working_directory, working_dir);
    }

    #[tokio::test]
    async fn save_and_load_playlist() {
        // Given a storage with playlist data.
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();

        let (_temp1, file1_path) = create_temp_file_in(temp.path(), "file1");
        let (_temp2, file2_path) = create_temp_file_in(temp.path(), "file2");
        let (_temp3, non_playlist_file_path) = create_temp_file_in(temp.path(), "non_playlist");
        let file1 = ItemPath::File(file1_path.clone());
        let file2 = ItemPath::File(file2_path.clone());
        let non_playlist_file = ItemPath::File(non_playlist_file_path.clone());

        let data = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: vec![file1.clone(), file2.clone()],
            files: [
                (
                    file1.clone(),
                    FileMetadata {
                        duration: Some(Duration::from_secs(120)),
                        is_virtual: false,
                        deleted: false,
                        mime_type: Some("video/mp4".to_string()),
                        time_added: None,
                        alias: None,
                    },
                ),
                (
                    file2.clone(),
                    FileMetadata {
                        duration: Some(Duration::from_secs(240)),
                        is_virtual: false,
                        deleted: false,
                        mime_type: None,
                        time_added: None,
                        alias: None,
                    },
                ),
                (
                    non_playlist_file.clone(),
                    FileMetadata {
                        duration: Some(Duration::from_secs(300)),
                        is_virtual: false,
                        deleted: false,
                        mime_type: Some("video/mp4".to_string()),
                        time_added: None,
                        alias: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        };

        // When saving and loading the data.
        storage.save(&data).await.unwrap();

        let loaded = storage.load(&working_dir).await.unwrap();

        // Then the loaded data matches the saved data.
        assert_eq!(loaded.playlist.len(), 2);
        assert_eq!(loaded.playlist[0], file1);
        assert_eq!(loaded.playlist[1], file2);
        assert_eq!(loaded.files.len(), 3);

        let meta1 = loaded.files.get(&file1).unwrap();
        assert_eq!(meta1.duration, Some(Duration::from_secs(120)));
        assert_eq!(meta1.mime_type, Some("video/mp4".to_string()));
        assert!(!meta1.is_virtual);
        assert!(!meta1.deleted);

        let non_playlist_meta = loaded.files.get(&non_playlist_file).unwrap();
        assert_eq!(non_playlist_meta.duration, Some(Duration::from_secs(300)));
        assert_eq!(non_playlist_meta.mime_type, Some("video/mp4".to_string()));
    }

    #[tokio::test]
    async fn virtual_file_handling() {
        // Given a storage with virtual files.
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();

        let virtual_file = ItemPath::Url("https://example.com/video.mp4".to_string());
        let non_playlist_virtual = ItemPath::Url("https://example.com/other.mp4".to_string());

        let data = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: vec![virtual_file.clone()],
            files: [
                (
                    virtual_file.clone(),
                    FileMetadata {
                        duration: Some(Duration::from_secs(60)),
                        is_virtual: true,
                        deleted: false,
                        mime_type: Some("video/mp4".to_string()),
                        time_added: None,
                        alias: None,
                    },
                ),
                (
                    non_playlist_virtual.clone(),
                    FileMetadata {
                        duration: Some(Duration::from_secs(60)),
                        is_virtual: true,
                        deleted: false,
                        mime_type: Some("video/mp4".to_string()),
                        time_added: None,
                        alias: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        };

        // When saving and loading the data.
        storage.save(&data).await.unwrap();

        let loaded = storage.load(&working_dir).await.unwrap();

        // Then virtual files are preserved.
        assert_eq!(loaded.playlist.len(), 1);
        let meta = loaded.files.get(&virtual_file).unwrap();
        assert!(meta.is_virtual);

        let non_playlist_meta = loaded.files.get(&non_playlist_virtual).unwrap();
        assert!(non_playlist_meta.is_virtual);
    }

    #[tokio::test]
    async fn deleted_file_handling() {
        // Given a storage with deleted files.
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();

        let (_temp1, file_path) = create_temp_file_in(temp.path(), "file");
        let (_temp2, non_playlist_file_path) = create_temp_file_in(temp.path(), "non_playlist");
        let file = ItemPath::File(file_path.clone());
        let non_playlist_file = ItemPath::File(non_playlist_file_path.clone());

        let data = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: vec![file.clone()],
            files: [
                (
                    file.clone(),
                    FileMetadata {
                        duration: None,
                        is_virtual: false,
                        deleted: true,
                        mime_type: None,
                        time_added: None,
                        alias: None,
                    },
                ),
                (
                    non_playlist_file.clone(),
                    FileMetadata {
                        duration: None,
                        is_virtual: false,
                        deleted: true,
                        mime_type: None,
                        time_added: None,
                        alias: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        };

        // When saving and loading the data.
        storage.save(&data).await.unwrap();

        // Then deleted flags are preserved.
        let loaded = storage.load(&working_dir).await.unwrap();
        let meta = loaded.files.get(&file).unwrap();
        assert!(meta.deleted);

        let non_playlist_meta = loaded.files.get(&non_playlist_file).unwrap();
        assert!(non_playlist_meta.deleted);
    }

    #[tokio::test]
    async fn alias_resolution_priority() {
        // Given aliases for the same file in different workspaces.
        let storage = create_test_storage().await;

        let (_temp1, workspace1) = create_temp_file();
        let (_temp2, workspace2) = create_temp_file();
        let (_temp_file, file) = create_temp_file();

        storage
            .upsert_alias(&file, &workspace1, "Workspace1 Alias")
            .await
            .unwrap();
        storage
            .upsert_alias(&file, &workspace2, "Workspace2 Alias")
            .await
            .unwrap();

        // When resolving aliases for each workspace.
        let alias_ws1 = storage.resolve_alias(&file, &workspace1).await.unwrap();

        // Then each workspace gets its own alias.
        assert_eq!(alias_ws1, Some("Workspace1 Alias".to_string()));

        let alias_ws2 = storage.resolve_alias(&file, &workspace2).await.unwrap();
        assert_eq!(alias_ws2, Some("Workspace2 Alias".to_string()));
    }

    #[tokio::test]
    async fn alias_fallback_to_most_recent() {
        // Given aliases for a file in multiple workspaces.
        let storage = create_test_storage().await;

        let (_temp1, workspace1) = create_temp_file();
        let (_temp2, workspace2) = create_temp_file();
        let (_temp3, workspace3) = create_temp_file();
        let (_temp_file, file) = create_temp_file();

        storage
            .upsert_alias(&file, &workspace1, "First Alias")
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        storage
            .upsert_alias(&file, &workspace2, "Second Alias")
            .await
            .unwrap();

        // When resolving alias for a workspace without its own alias.
        let fallback = storage.resolve_alias(&file, &workspace3).await.unwrap();

        // Then a fallback alias is returned.
        assert!(fallback.is_some());
    }

    #[tokio::test]
    async fn alias_loaded_with_file_metadata() {
        // Given a file with an alias set.
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();

        let (_temp, file_path) = create_temp_file();
        let file = ItemPath::File(file_path.clone());

        let data = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: vec![file.clone()],
            files: [(
                file.clone(),
                FileMetadata {
                    duration: Some(Duration::from_secs(120)),
                    is_virtual: false,
                    deleted: false,
                    mime_type: Some("video/mp4".to_string()),
                    time_added: None,
                    alias: None,
                },
            )]
            .into_iter()
            .collect(),
        };
        storage.save(&data).await.unwrap();

        storage
            .upsert_alias(&file_path, &working_dir, "My Video")
            .await
            .unwrap();

        // When loading the playlist.
        let loaded = storage.load(&working_dir).await.unwrap();

        // Then the alias is included in the file metadata.
        let meta = loaded.files.get(&file).unwrap();
        assert_eq!(meta.alias, Some("My Video".to_string()));
    }

    #[tokio::test]
    async fn delete_alias_removes_existing_alias() {
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();
        let (_temp_file, file) = create_temp_file();

        // Given an alias exists
        storage
            .upsert_alias(&file, &working_dir, "My Video")
            .await
            .unwrap();
        let alias = storage.resolve_alias(&file, &working_dir).await.unwrap();
        assert_eq!(alias, Some("My Video".to_string()));

        // When deleting the alias
        storage.delete_alias(&file, &working_dir).await.unwrap();

        // Then the alias is removed
        let alias = storage.resolve_alias(&file, &working_dir).await.unwrap();
        assert!(alias.is_none());
    }

    #[tokio::test]
    async fn delete_alias_is_idempotent() {
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();
        let (_temp_file, file) = create_temp_file();

        // When deleting a non-existent alias
        let result = storage.delete_alias(&file, &working_dir).await;

        // Then it succeeds without error
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn workspace_alias_not_overridden_by_other_workspace() {
        // Given a shared file with different aliases in two workspaces.
        let storage = create_test_storage().await;

        let temp1 = TempDir::new().unwrap();
        let temp2 = TempDir::new().unwrap();
        let workspace1 = CanonicalPath::from_path(temp1.path()).unwrap();
        let workspace2 = CanonicalPath::from_path(temp2.path()).unwrap();

        let (_shared_temp, shared_file_canonical) = create_temp_file();
        let shared_file = ItemPath::File(shared_file_canonical.clone());

        let data1 = PlaylistData {
            working_directory: workspace1.clone(),
            playlist: vec![shared_file.clone()],
            files: [(
                shared_file.clone(),
                FileMetadata {
                    duration: Some(Duration::from_secs(100)),
                    is_virtual: false,
                    deleted: false,
                    mime_type: None,
                    time_added: None,
                    alias: None,
                },
            )]
            .into_iter()
            .collect(),
        };

        let data2 = PlaylistData {
            working_directory: workspace2.clone(),
            playlist: vec![shared_file.clone()],
            files: [(
                shared_file.clone(),
                FileMetadata {
                    duration: Some(Duration::from_secs(100)),
                    is_virtual: false,
                    deleted: false,
                    mime_type: None,
                    time_added: None,
                    alias: None,
                },
            )]
            .into_iter()
            .collect(),
        };

        storage.save(&data1).await.unwrap();
        storage.save(&data2).await.unwrap();

        storage
            .upsert_alias(&shared_file_canonical, &workspace1, "WS1 Alias")
            .await
            .unwrap();
        storage
            .upsert_alias(&shared_file_canonical, &workspace2, "WS2 Alias")
            .await
            .unwrap();

        // When loading both workspaces.
        let loaded1 = storage.load(&workspace1).await.unwrap();
        let loaded2 = storage.load(&workspace2).await.unwrap();

        // Then each workspace shows its own alias.
        let meta1 = loaded1.files.get(&shared_file).unwrap();
        let meta2 = loaded2.files.get(&shared_file).unwrap();

        assert_eq!(
            meta1.alias,
            Some("WS1 Alias".to_string()),
            "Workspace1 should show its own alias"
        );
        assert_eq!(
            meta2.alias,
            Some("WS2 Alias".to_string()),
            "Workspace2 should show its own alias, not WS1's alias"
        );
    }

    #[tokio::test]
    async fn workspace_alias_priority_over_fallback() {
        // Given a shared file with aliases in two workspaces at different times.
        let storage = create_test_storage().await;

        let temp1 = TempDir::new().unwrap();
        let temp2 = TempDir::new().unwrap();
        let workspace1 = CanonicalPath::from_path(temp1.path()).unwrap();
        let workspace2 = CanonicalPath::from_path(temp2.path()).unwrap();

        let (_shared_temp, shared_file_canonical) = create_temp_file();
        let shared_file = ItemPath::File(shared_file_canonical.clone());

        let data1 = PlaylistData {
            working_directory: workspace1.clone(),
            playlist: vec![shared_file.clone()],
            files: [(
                shared_file.clone(),
                FileMetadata {
                    duration: None,
                    is_virtual: false,
                    deleted: false,
                    mime_type: None,
                    time_added: None,
                    alias: None,
                },
            )]
            .into_iter()
            .collect(),
        };
        storage.save(&data1).await.unwrap();

        let data2 = PlaylistData {
            working_directory: workspace2.clone(),
            playlist: vec![shared_file.clone()],
            files: [(
                shared_file.clone(),
                FileMetadata {
                    duration: None,
                    is_virtual: false,
                    deleted: false,
                    mime_type: None,
                    time_added: None,
                    alias: None,
                },
            )]
            .into_iter()
            .collect(),
        };
        storage.save(&data2).await.unwrap();

        storage
            .upsert_alias(&shared_file_canonical, &workspace1, "Older Alias from WS1")
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        storage
            .upsert_alias(&shared_file_canonical, &workspace2, "Newer Alias from WS2")
            .await
            .unwrap();

        // When loading both workspaces and resolving aliases directly.
        let loaded1 = storage.load(&workspace1).await.unwrap();
        let meta1 = loaded1.files.get(&shared_file).unwrap();

        // Then workspace1 shows its own alias despite being older.
        assert_eq!(
            meta1.alias,
            Some("Older Alias from WS1".to_string()),
            "Workspace1 should show its own alias even though workspace2's alias is newer"
        );

        // And workspace2 shows its own alias.
        let loaded2 = storage.load(&workspace2).await.unwrap();
        let meta2 = loaded2.files.get(&shared_file).unwrap();
        assert_eq!(
            meta2.alias,
            Some("Newer Alias from WS2".to_string()),
            "Workspace2 should show its own alias"
        );

        // And direct resolution returns correct aliases.
        let direct1 = storage
            .resolve_alias(&shared_file_canonical, &workspace1)
            .await
            .unwrap();
        let direct2 = storage
            .resolve_alias(&shared_file_canonical, &workspace2)
            .await
            .unwrap();
        assert_eq!(direct1, Some("Older Alias from WS1".to_string()));
        assert_eq!(direct2, Some("Newer Alias from WS2".to_string()));
    }

    #[tokio::test]
    async fn multiple_workspaces_isolation() {
        // Given two workspaces with different files.
        let storage = create_test_storage().await;

        let temp1 = TempDir::new().unwrap();
        let temp2 = TempDir::new().unwrap();
        let workspace1 = CanonicalPath::from_path(temp1.path()).unwrap();
        let workspace2 = CanonicalPath::from_path(temp2.path()).unwrap();

        let (_temp1, file1_path) = create_temp_file();
        let (_temp2, file2_path) = create_temp_file();
        let file1 = ItemPath::File(file1_path.clone());
        let file2 = ItemPath::File(file2_path.clone());

        let data1 = PlaylistData {
            working_directory: workspace1.clone(),
            playlist: vec![file1.clone()],
            files: [(
                file1.clone(),
                FileMetadata {
                    duration: Some(Duration::from_secs(100)),
                    is_virtual: false,
                    deleted: false,
                    mime_type: None,
                    time_added: None,
                    alias: None,
                },
            )]
            .into_iter()
            .collect(),
        };

        let data2 = PlaylistData {
            working_directory: workspace2.clone(),
            playlist: vec![file2.clone()],
            files: [(
                file2.clone(),
                FileMetadata {
                    duration: Some(Duration::from_secs(200)),
                    is_virtual: false,
                    deleted: false,
                    mime_type: None,
                    time_added: None,
                    alias: None,
                },
            )]
            .into_iter()
            .collect(),
        };

        // When saving both workspaces.
        storage.save(&data1).await.unwrap();
        storage.save(&data2).await.unwrap();

        // Then each workspace loads its own files.
        let loaded1 = storage.load(&workspace1).await.unwrap();
        let loaded2 = storage.load(&workspace2).await.unwrap();

        assert_eq!(loaded1.playlist.len(), 1);
        assert_eq!(loaded1.playlist[0], file1);

        assert_eq!(loaded2.playlist.len(), 1);
        assert_eq!(loaded2.playlist[0], file2);
    }

    #[tokio::test]
    async fn save_overwrites_existing_playlist() {
        // Given a workspace with an existing playlist.
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();

        let (_temp1, file1_path) = create_temp_file();
        let (_temp2, file2_path) = create_temp_file();
        let (_temp3, file3_path) = create_temp_file();
        let file1 = ItemPath::File(file1_path.clone());
        let file2 = ItemPath::File(file2_path.clone());
        let file3 = ItemPath::File(file3_path.clone());

        let data1 = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: vec![file1.clone(), file2.clone()],
            files: [
                (
                    file1.clone(),
                    FileMetadata {
                        duration: None,
                        is_virtual: false,
                        deleted: false,
                        mime_type: None,
                        time_added: None,
                        alias: None,
                    },
                ),
                (
                    file2.clone(),
                    FileMetadata {
                        duration: None,
                        is_virtual: false,
                        deleted: false,
                        mime_type: None,
                        time_added: None,
                        alias: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        };

        storage.save(&data1).await.unwrap();

        // When saving a new playlist with different files.
        let data2 = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: vec![file3.clone()],
            files: [(
                file3.clone(),
                FileMetadata {
                    duration: None,
                    is_virtual: false,
                    deleted: false,
                    mime_type: None,
                    time_added: None,
                    alias: None,
                },
            )]
            .into_iter()
            .collect(),
        };

        storage.save(&data2).await.unwrap();

        // Then only the new playlist is present.
        let loaded = storage.load(&working_dir).await.unwrap();
        assert_eq!(loaded.playlist.len(), 1);
        assert_eq!(loaded.playlist[0], file3);
    }

    #[tokio::test]
    async fn playlist_order_preserved() {
        // Given a playlist with 10 files in specific order.
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();

        let temp_files: Vec<(NamedTempFile, CanonicalPath)> = (0..10)
            .map(|_| create_temp_file())
            .collect();
        let files: Vec<ItemPath> = temp_files
            .iter()
            .map(|(_, path)| ItemPath::File(path.clone()))
            .collect();

        let data = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: files.clone(),
            files: files
                .iter()
                .map(|path| {
                    (
                        path.clone(),
                        FileMetadata {
                            duration: None,
                            is_virtual: false,
                            deleted: false,
                            mime_type: None,
                            time_added: None,
                            alias: None,
                        },
                    )
                })
                .collect(),
        };

        // When saving and loading the playlist.
        storage.save(&data).await.unwrap();

        // Then the order is preserved.
        let loaded = storage.load(&working_dir).await.unwrap();
        assert_eq!(loaded.playlist.len(), 10);
        for (i, expected) in files.iter().enumerate() {
            assert_eq!(loaded.playlist[i], *expected);
        }
    }

    #[tokio::test]
    async fn time_added_preserved() {
        // Given a file with time_added metadata.
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();

        let timestamp = Timestamp::now();

        let (_temp1, file_path) = create_temp_file_in(temp.path(), "file");
        let (_temp2, non_playlist_file_path) = create_temp_file_in(temp.path(), "non_playlist");
        let file = ItemPath::File(file_path.clone());
        let non_playlist_file = ItemPath::File(non_playlist_file_path.clone());

        let data = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: vec![file.clone()],
            files: [
                (
                    file.clone(),
                    FileMetadata {
                        duration: None,
                        is_virtual: false,
                        deleted: false,
                        mime_type: None,
                        time_added: Some(timestamp),
                        alias: None,
                    },
                ),
                (
                    non_playlist_file.clone(),
                    FileMetadata {
                        duration: None,
                        is_virtual: false,
                        deleted: false,
                        mime_type: None,
                        time_added: Some(timestamp),
                        alias: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        };

        // When saving and loading the data.
        storage.save(&data).await.unwrap();

        // Then time_added is preserved for all files.
        let loaded = storage.load(&working_dir).await.unwrap();
        let meta = loaded.files.get(&file).unwrap();
        assert!(meta.time_added.is_some());

        let non_playlist_meta = loaded.files.get(&non_playlist_file).unwrap();
        assert!(non_playlist_meta.time_added.is_some());
    }

    #[tokio::test]
    async fn storage_name() {
        // Given a sqlite storage.
        let storage = create_test_storage().await;

        // When getting the storage name.
        // Then it returns "sqlite".
        assert_eq!(storage.name(), "sqlite");
    }

    #[tokio::test]
    async fn upsert_file_metadata_updates_existing() {
        // Given a file with initial metadata.
        let storage = create_test_storage().await;

        let file = PathBuf::from("/test/file.mp4");
        let file_id = storage.get_or_create_file_path(&file).await.unwrap();
        let workspace_id = storage
            .get_or_create_workspace(Path::new("/test"))
            .await
            .unwrap();

        let meta1 = FileMetadata {
            duration: Some(Duration::from_secs(100)),
            is_virtual: false,
            deleted: false,
            mime_type: Some("video/mp4".to_string()),
            time_added: None,
            alias: None,
        };

        storage.upsert_file_metadata(file_id, &meta1).await.unwrap();

        let loaded1 = storage
            .get_file_metadata(file_id, workspace_id)
            .await
            .unwrap();
        assert_eq!(loaded1.duration, Some(Duration::from_secs(100)));

        // When updating the metadata with new values.
        let meta2 = FileMetadata {
            duration: Some(Duration::from_secs(200)),
            is_virtual: false,
            deleted: true,
            mime_type: None,
            time_added: None,
            alias: None,
        };

        storage.upsert_file_metadata(file_id, &meta2).await.unwrap();

        // Then the metadata is updated.
        let loaded2 = storage
            .get_file_metadata(file_id, workspace_id)
            .await
            .unwrap();
        assert_eq!(loaded2.duration, Some(Duration::from_secs(200)));
        assert!(loaded2.deleted);
        assert!(loaded2.mime_type.is_none());
    }

    #[tokio::test]
    async fn virtual_file_flag_toggles() {
        // Given a file in storage.
        let storage = create_test_storage().await;

        let file = PathBuf::from("/test/file.mp4");
        let file_id = storage.get_or_create_file_path(&file).await.unwrap();
        let workspace_id = storage
            .get_or_create_workspace(Path::new("/test"))
            .await
            .unwrap();

        // When setting virtual to true.
        storage
            .upsert_virtual_file(file_id, workspace_id, true)
            .await
            .unwrap();

        // Then the file is marked as virtual.
        assert!(storage.is_virtual_file(file_id, workspace_id).await.unwrap());

        // When setting virtual to false.
        storage
            .upsert_virtual_file(file_id, workspace_id, false)
            .await
            .unwrap();

        // Then the file is no longer marked as virtual.
        assert!(!storage.is_virtual_file(file_id, workspace_id).await.unwrap());
    }

    #[tokio::test]
    async fn playlist_item_duration_is_persisted() {
        // Given a playlist item with duration metadata.
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();

        let (_temp, file_path) = create_temp_file();
        let file = ItemPath::File(file_path.clone());

        let data = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: vec![file.clone()],
            files: [(
                file.clone(),
                FileMetadata {
                    duration: Some(Duration::from_secs(120)),
                    is_virtual: false,
                    deleted: false,
                    mime_type: Some("video/mp4".to_string()),
                    time_added: None,
                    alias: None,
                },
            )]
            .into_iter()
            .collect(),
        };

        // When saving and loading the data.
        storage.save(&data).await.unwrap();

        let loaded = storage.load(&working_dir).await.unwrap();

        // Then the duration is persisted.
        assert_eq!(loaded.playlist.len(), 1);
        assert_eq!(loaded.playlist[0], file);

        let loaded_meta = loaded.files.get(&file).unwrap();
        assert_eq!(loaded_meta.duration, Some(Duration::from_secs(120)));
        assert_eq!(loaded_meta.mime_type, Some("video/mp4".to_string()));
        assert!(!loaded_meta.is_virtual);
        assert!(!loaded_meta.deleted);
    }

    #[tokio::test]
    async fn library_file_duration_is_persisted() {
        // Given a playlist with both playlist items and library files.
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();

        let (_temp1, playlist_file_path) = create_temp_file_in(temp.path(), "playlist");
        let (_temp2, library_file_path) = create_temp_file_in(temp.path(), "library");
        let playlist_file = ItemPath::File(playlist_file_path.clone());
        let library_file = ItemPath::File(library_file_path.clone());

        let data = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: vec![playlist_file.clone()],
            files: [
                (
                    playlist_file.clone(),
                    FileMetadata {
                        duration: Some(Duration::from_secs(60)),
                        is_virtual: false,
                        deleted: false,
                        mime_type: Some("video/mp4".to_string()),
                        time_added: None,
                        alias: None,
                    },
                ),
                (
                    library_file.clone(),
                    FileMetadata {
                        duration: Some(Duration::from_secs(180)),
                        is_virtual: false,
                        deleted: false,
                        mime_type: Some("video/mp4".to_string()),
                        time_added: None,
                        alias: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        };

        // When saving and loading the data.
        storage.save(&data).await.unwrap();

        let loaded = storage.load(&working_dir).await.unwrap();

        // Then library file duration is persisted.
        assert_eq!(loaded.playlist.len(), 1);
        assert!(loaded.files.contains_key(&library_file));

        let library_meta = loaded.files.get(&library_file).unwrap();
        assert_eq!(library_meta.duration, Some(Duration::from_secs(180)));
        assert_eq!(library_meta.mime_type, Some("video/mp4".to_string()));
    }

    #[tokio::test]
    async fn full_metadata_preserved_across_save_load_cycle() {
        // Given a playlist with playlist items, library files, and virtual files.
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();
        let timestamp = Timestamp::now();

        let (_temp1, playlist_file_path) = create_temp_file_in(temp.path(), "playlist");
        let (_temp2, library_file_path) = create_temp_file_in(temp.path(), "library");
        let playlist_file = ItemPath::File(playlist_file_path.clone());
        let library_file = ItemPath::File(library_file_path.clone());
        let virtual_file = ItemPath::Url("https://example.com/stream.mp4".to_string());

        let data = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: vec![playlist_file.clone(), virtual_file.clone()],
            files: [
                (
                    playlist_file.clone(),
                    FileMetadata {
                        duration: Some(Duration::from_secs(300)),
                        is_virtual: false,
                        deleted: false,
                        mime_type: Some("video/mp4".to_string()),
                        time_added: Some(timestamp),
                        alias: None,
                    },
                ),
                (
                    library_file.clone(),
                    FileMetadata {
                        duration: Some(Duration::from_secs(450)),
                        is_virtual: false,
                        deleted: true,
                        mime_type: Some("video/x-matroska".to_string()),
                        time_added: Some(timestamp),
                        alias: None,
                    },
                ),
                (
                    virtual_file.clone(),
                    FileMetadata {
                        duration: Some(Duration::from_secs(600)),
                        is_virtual: true,
                        deleted: false,
                        mime_type: Some("video/mp4".to_string()),
                        time_added: Some(timestamp),
                        alias: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        };

        // When saving and loading the data.
        storage.save(&data).await.unwrap();

        let loaded = storage.load(&working_dir).await.unwrap();

        // Then all metadata is preserved for all file types.
        assert_eq!(loaded.playlist.len(), 2);
        assert_eq!(loaded.playlist[0], playlist_file);
        assert_eq!(loaded.playlist[1], virtual_file);
        assert_eq!(loaded.files.len(), 3);

        let playlist_meta = loaded.files.get(&playlist_file).unwrap();
        assert_eq!(playlist_meta.duration, Some(Duration::from_secs(300)));
        assert_eq!(playlist_meta.mime_type, Some("video/mp4".to_string()));
        assert!(!playlist_meta.is_virtual);
        assert!(!playlist_meta.deleted);
        assert!(playlist_meta.time_added.is_some());

        let library_meta = loaded.files.get(&library_file).unwrap();
        assert_eq!(library_meta.duration, Some(Duration::from_secs(450)));
        assert_eq!(library_meta.mime_type, Some("video/x-matroska".to_string()));
        assert!(!library_meta.is_virtual);
        assert!(library_meta.deleted);
        assert!(library_meta.time_added.is_some());

        let virtual_meta = loaded.files.get(&virtual_file).unwrap();
        assert_eq!(virtual_meta.duration, Some(Duration::from_secs(600)));
        assert_eq!(virtual_meta.mime_type, Some("video/mp4".to_string()));
        assert!(virtual_meta.is_virtual);
        assert!(!virtual_meta.deleted);
        assert!(virtual_meta.time_added.is_some());
    }

    #[tokio::test]
    async fn duration_updated_on_resave() {
        // Given a file with initial duration.
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();

        let (_file_temp, file_path) = create_temp_file_in(temp.path(), "video");
        let file = ItemPath::File(file_path.clone());

        let mut data = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: vec![file.clone()],
            files: HashMap::new(),
        };
        data.files.insert(
            file.clone(),
            FileMetadata {
                duration: Some(Duration::from_secs(100)),
                is_virtual: false,
                deleted: false,
                mime_type: Some("video/mp4".to_string()),
                time_added: None,
                alias: None,
            },
        );

        storage.save(&data).await.unwrap();

        let loaded1 = storage.load(&working_dir).await.unwrap();
        assert_eq!(
            loaded1.files.get(&file).unwrap().duration,
            Some(Duration::from_secs(100))
        );

        // When saving with an updated duration.
        data.files.insert(
            file.clone(),
            FileMetadata {
                duration: Some(Duration::from_secs(200)),
                is_virtual: false,
                deleted: false,
                mime_type: Some("video/mp4".to_string()),
                time_added: None,
                alias: None,
            },
        );

        storage.save(&data).await.unwrap();

        // Then the duration is updated.
        let loaded2 = storage.load(&working_dir).await.unwrap();
        assert_eq!(
            loaded2.files.get(&file).unwrap().duration,
            Some(Duration::from_secs(200))
        );
    }

    #[tokio::test]
    async fn alias_resolution_with_different_path_formats() {
        // Given a file with an alias.
        let storage = create_test_storage().await;

        let temp = TempDir::new().unwrap();
        let workspace = CanonicalPath::from_path(temp.path()).unwrap();

        let (_file_temp, file_canonical) = create_temp_file();
        let file = ItemPath::File(file_canonical.clone());

        let data = PlaylistData {
            working_directory: workspace.clone(),
            playlist: vec![file.clone()],
            files: [(
                file.clone(),
                FileMetadata {
                    duration: Some(Duration::from_secs(100)),
                    is_virtual: false,
                    deleted: false,
                    mime_type: None,
                    time_added: None,
                    alias: None,
                },
            )]
            .into_iter()
            .collect(),
        };
        storage.save(&data).await.unwrap();

        storage
            .upsert_alias(&file_canonical, &workspace, "My Alias")
            .await
            .unwrap();

        // When resolving the alias directly and loading the playlist.
        let alias_same = storage
            .resolve_alias(&file_canonical, &workspace)
            .await
            .unwrap();

        // Then the alias is returned.
        assert_eq!(alias_same, Some("My Alias".to_string()));

        // And the alias is included in loaded metadata.
        let loaded = storage.load(&workspace).await.unwrap();
        let meta = loaded.files.get(&file).unwrap();
        assert_eq!(meta.alias, Some("My Alias".to_string()));
    }

    #[tokio::test]
    async fn get_path_counts_returns_empty_when_no_playlists() {
        // Given an empty storage.
        let storage = create_test_storage().await;

        // When getting path counts.
        let counts = storage.get_path_counts().await.unwrap();

        // Then the result is empty.
        assert!(counts.is_empty());
    }

    #[tokio::test]
    async fn get_path_counts_returns_correct_counts_for_multiple_workspaces() {
        // Given two workspaces with overlapping files.
        let storage = create_test_storage().await;

        let temp1 = TempDir::new().unwrap();
        let temp2 = TempDir::new().unwrap();
        let workspace1 = CanonicalPath::from_path(temp1.path()).unwrap();
        let workspace2 = CanonicalPath::from_path(temp2.path()).unwrap();

        let (_file1_temp, file1_path) = create_temp_file_in(temp1.path(), "file1");
        let (_file2_temp, file2_path) = create_temp_file_in(temp2.path(), "file2");
        let file1 = ItemPath::File(file1_path.clone());
        let file2 = ItemPath::File(file2_path.clone());

        let data1 = PlaylistData {
            working_directory: workspace1.clone(),
            playlist: vec![file1.clone()],
            files: [(
                file1.clone(),
                FileMetadata {
                    duration: None,
                    is_virtual: false,
                    deleted: false,
                    mime_type: None,
                    time_added: None,
                    alias: None,
                },
            )]
            .into_iter()
            .collect(),
        };

        let data2 = PlaylistData {
            working_directory: workspace2.clone(),
            playlist: vec![file1.clone(), file2.clone()],
            files: [
                (
                    file1.clone(),
                    FileMetadata {
                        duration: None,
                        is_virtual: false,
                        deleted: false,
                        mime_type: None,
                        time_added: None,
                        alias: None,
                    },
                ),
                (
                    file2.clone(),
                    FileMetadata {
                        duration: None,
                        is_virtual: false,
                        deleted: false,
                        mime_type: None,
                        time_added: None,
                        alias: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        };

        storage.save(&data1).await.unwrap();
        storage.save(&data2).await.unwrap();

        // When getting path counts.
        let counts = storage.get_path_counts().await.unwrap();

        // Then counts reflect how many workspaces contain each file.
        let file1_id = storage
            .resolve_file_path_id(&file1)
            .await
            .unwrap()
            .unwrap();
        let file2_id = storage
            .resolve_file_path_id(&file2)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(counts.get(&file1_id), Some(&2), "file1 should be in 2 workspaces");
        assert_eq!(counts.get(&file2_id), Some(&1), "file2 should be in 1 workspace");
    }

    #[tokio::test]
    async fn get_path_counts_file_in_single_workspace_has_count_one() {
        // Given a file in a single workspace.
        let storage = create_test_storage().await;

        let temp = TempDir::new().unwrap();
        let workspace = CanonicalPath::from_path(temp.path()).unwrap();

        let (_file_temp, file_path) = create_temp_file_in(temp.path(), "file");
        let file = ItemPath::File(file_path.clone());

        let data = PlaylistData {
            working_directory: workspace.clone(),
            playlist: vec![file.clone()],
            files: [(
                file.clone(),
                FileMetadata {
                    duration: None,
                    is_virtual: false,
                    deleted: false,
                    mime_type: None,
                    time_added: None,
                    alias: None,
                },
            )]
            .into_iter()
            .collect(),
        };

        storage.save(&data).await.unwrap();

        // When getting path counts.
        let counts = storage.get_path_counts().await.unwrap();

        // Then the file has count 1.
        let file_id = storage.resolve_file_path_id(&file).await.unwrap().unwrap();
        assert_eq!(counts.get(&file_id), Some(&1));
    }

    #[tokio::test]
    async fn virtual_files_are_workspace_scoped() {
        // Given two workspaces with different virtual files.
        let storage = create_test_storage().await;

        let temp1 = TempDir::new().unwrap();
        let temp2 = TempDir::new().unwrap();
        let workspace1 = CanonicalPath::from_path(temp1.path()).unwrap();
        let workspace2 = CanonicalPath::from_path(temp2.path()).unwrap();

        let url1 = ItemPath::Url("https://example.com/video1.mp4".to_string());
        let url2 = ItemPath::Url("https://example.com/video2.mp4".to_string());

        let data1 = PlaylistData {
            working_directory: workspace1.clone(),
            playlist: vec![],
            files: [(
                url1.clone(),
                FileMetadata {
                    duration: None,
                    is_virtual: true,
                    deleted: false,
                    mime_type: Some("video/mp4".to_string()),
                    time_added: None,
                    alias: None,
                },
            )]
            .into_iter()
            .collect(),
        };

        let data2 = PlaylistData {
            working_directory: workspace2.clone(),
            playlist: vec![],
            files: [(
                url2.clone(),
                FileMetadata {
                    duration: None,
                    is_virtual: true,
                    deleted: false,
                    mime_type: Some("video/mp4".to_string()),
                    time_added: None,
                    alias: None,
                },
            )]
            .into_iter()
            .collect(),
        };

        // When saving to both workspaces.
        storage.save(&data1).await.unwrap();
        storage.save(&data2).await.unwrap();

        // Then each workspace only sees its own virtual files.
        let loaded1 = storage.load(&workspace1).await.unwrap();
        let loaded2 = storage.load(&workspace2).await.unwrap();

        assert!(
            loaded1.files.contains_key(&url1),
            "workspace1 should contain url1"
        );
        assert!(
            !loaded1.files.contains_key(&url2),
            "workspace1 should NOT contain url2"
        );
        assert!(
            loaded2.files.contains_key(&url2),
            "workspace2 should contain url2"
        );
        assert!(
            !loaded2.files.contains_key(&url1),
            "workspace2 should NOT contain url1"
        );
    }
}
