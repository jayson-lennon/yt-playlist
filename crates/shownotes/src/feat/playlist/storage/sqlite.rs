use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use error_stack::Report;
use marked_path::CanonicalPath;
use sqlx::SqlitePool;
use wherror::Error;

use super::super::{FileMetadata, IoError, PlaylistData, PlaylistStorage};

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
            .map_err(|_| Report::new(IoError))?
        {
            return Ok(id);
        }

        let id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO workspaces (path) VALUES (?) RETURNING id",
        )
        .bind(path_str.as_ref())
        .fetch_one(&self.pool)
        .await
        .map_err(|_| Report::new(IoError))?;

        Ok(id)
    }

    async fn get_workspace(&self, path: &Path) -> Result<Option<i64>, Report<IoError>> {
        let path_str = path.to_string_lossy();

        let id = sqlx::query_scalar::<_, i64>("SELECT id FROM workspaces WHERE path = ?")
            .bind(path_str.as_ref())
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| Report::new(IoError))?;

        Ok(id)
    }

    async fn get_or_create_file_path(&self, path: &Path) -> Result<i64, Report<IoError>> {
        let path_str = path.to_string_lossy();

        if let Some(id) = sqlx::query_scalar::<_, i64>("SELECT id FROM file_paths WHERE path = ?")
            .bind(path_str.as_ref())
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| Report::new(IoError))?
        {
            return Ok(id);
        }

        let id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO file_paths (path) VALUES (?) RETURNING id",
        )
        .bind(path_str.as_ref())
        .fetch_one(&self.pool)
        .await
        .map_err(|_| Report::new(IoError))?;

        Ok(id)
    }

    #[allow(dead_code)]
    async fn resolve_alias(
        &self,
        file_path_id: i64,
        workspace_id: i64,
    ) -> Result<Option<String>, Report<IoError>> {
        if let Some(alias) = sqlx::query_scalar::<_, String>(
            "SELECT alias FROM aliases WHERE file_path_id = ? AND workspace_id = ?",
        )
        .bind(file_path_id)
        .bind(workspace_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| Report::new(IoError))?
        {
            return Ok(Some(alias));
        }

        let fallback = sqlx::query_scalar::<_, String>(
            "SELECT alias FROM aliases WHERE file_path_id = ? ORDER BY updated_at DESC LIMIT 1",
        )
        .bind(file_path_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| Report::new(IoError))?;

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
        .map_err(|_| Report::new(IoError))?;

        Ok(())
    }

    async fn upsert_virtual_file(&self, file_path_id: i64, is_virtual: bool) -> Result<(), Report<IoError>> {
        if is_virtual {
            sqlx::query(
                "INSERT OR IGNORE INTO virtual_files (file_path_id) VALUES (?)",
            )
            .bind(file_path_id)
            .execute(&self.pool)
            .await
            .map_err(|_| Report::new(IoError))?;
        } else {
            sqlx::query("DELETE FROM virtual_files WHERE file_path_id = ?")
                .bind(file_path_id)
                .execute(&self.pool)
                .await
                .map_err(|_| Report::new(IoError))?;
        }

        Ok(())
    }

    #[allow(dead_code)]
    async fn upsert_alias(
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
        .map_err(|_| Report::new(IoError))?;

        Ok(())
    }

    async fn is_virtual_file(&self, file_path_id: i64) -> Result<bool, Report<IoError>> {
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT 1 FROM virtual_files WHERE file_path_id = ?",
        )
        .bind(file_path_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| Report::new(IoError))?;

        Ok(exists.is_some())
    }

    async fn get_file_metadata(
        &self,
        file_path_id: i64,
    ) -> Result<FileMetadata, Report<IoError>> {
        let row = sqlx::query_as::<_, (Option<f64>, Option<String>, i32, Option<String>)>(
            "SELECT duration_seconds, mime_type, deleted, time_added FROM file_metadata WHERE file_path_id = ?",
        )
        .bind(file_path_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| Report::new(IoError))?;

        let is_virtual = self.is_virtual_file(file_path_id).await?;

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
                })
            }
            None => Ok(FileMetadata {
                duration: None,
                is_virtual,
                deleted: false,
                mime_type: None,
                time_added: None,
            }),
        }
    }
}

#[async_trait]
impl PlaylistStorage for SqliteStorage {
    fn name(&self) -> &'static str {
        "sqlite"
    }

    async fn load(&self, working_directory: &CanonicalPath) -> Result<PlaylistData, Report<IoError>> {
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
        .map_err(|_| Report::new(IoError))?;

        let playlist: Vec<PathBuf> = items.iter().map(|(path, _)| PathBuf::from(path)).collect();
        let playlist_file_ids: HashSet<i64> = items.iter().map(|(_, id)| *id).collect();

        let mut files: HashMap<PathBuf, FileMetadata> = HashMap::new();

        for (path_str, file_path_id) in &items {
            let path = PathBuf::from(path_str);
            let metadata = self.get_file_metadata(*file_path_id).await?;
            if let Ok(canonical) = CanonicalPath::from_path(&path) {
                files.insert(canonical.to_path_buf(), metadata);
            } else {
                files.insert(path, metadata);
            }
        }

        let working_dir_str = working_directory.as_path().to_string_lossy();
        let all_metadata_files = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT fp.path, fm.file_path_id
            FROM file_metadata fm
            JOIN file_paths fp ON fm.file_path_id = fp.id
            LEFT JOIN virtual_files vf ON fm.file_path_id = vf.file_path_id
            WHERE fp.path LIKE ? OR vf.file_path_id IS NOT NULL
            "#,
        )
        .bind(format!("{}%", working_dir_str.as_ref()))
        .fetch_all(&self.pool)
        .await
        .map_err(|_| Report::new(IoError))?;

        for (path_str, file_path_id) in all_metadata_files {
            if playlist_file_ids.contains(&file_path_id) {
                continue;
            }

            let path = PathBuf::from(&path_str);
            let metadata = self.get_file_metadata(file_path_id).await?;
            if let Ok(canonical) = CanonicalPath::from_path(&path) {
                files.insert(canonical.to_path_buf(), metadata);
            } else {
                files.insert(path, metadata);
            }
        }

        Ok(PlaylistData {
            working_directory: working_directory.clone(),
            playlist,
            files,
        })
    }

    async fn save(&self, data: &PlaylistData) -> Result<(), Report<IoError>> {
        let workspace_id = self.get_or_create_workspace(data.working_directory.as_path()).await?;

        let mut file_path_ids: Vec<(PathBuf, i64)> = Vec::new();
        for path in &data.playlist {
            let file_path_id = self.get_or_create_file_path(path).await?;
            file_path_ids.push((path.clone(), file_path_id));
        }

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| Report::new(IoError))?;

        sqlx::query("DELETE FROM playlist_items WHERE workspace_id = ?")
            .bind(workspace_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| Report::new(IoError))?;

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
            .map_err(|_| Report::new(IoError))?;
        }

        tx.commit().await.map_err(|_| Report::new(IoError))?;

        for (path, metadata) in &data.files {
            let file_path_id = self.get_or_create_file_path(path).await?;
            self.upsert_file_metadata(file_path_id, metadata).await?;
            self.upsert_virtual_file(file_path_id, metadata.is_virtual).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feat::note_db::SqliteNoteDb;
    use jiff::Timestamp;
    use tempfile::TempDir;

    async fn create_test_storage() -> SqliteStorage {
        let db = SqliteNoteDb::new("sqlite::memory:").await.unwrap();
        SqliteStorage::new(db.pool().clone())
    }

    #[tokio::test]
    async fn load_empty_workspace_returns_empty_data() {
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();
        let result = storage.load(&working_dir).await;

        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(data.playlist.is_empty());
        assert!(data.files.is_empty());
        assert_eq!(data.working_directory, working_dir);
    }

    #[tokio::test]
    async fn save_and_load_playlist() {
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();

        let file1 = working_dir.as_path().join("video1.mp4");
        let file2 = working_dir.as_path().join("video2.mp4");
        let non_playlist_file = working_dir.as_path().join("other.mp4");

        let data = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: vec![file1.clone(), file2.clone()],
            files: [
                (file1.clone(), FileMetadata {
                    duration: Some(Duration::from_secs(120)),
                    is_virtual: false,
                    deleted: false,
                    mime_type: Some("video/mp4".to_string()),
                    time_added: None,
                }),
                (file2.clone(), FileMetadata {
                    duration: Some(Duration::from_secs(240)),
                    is_virtual: false,
                    deleted: false,
                    mime_type: None,
                    time_added: None,
                }),
                (non_playlist_file.clone(), FileMetadata {
                    duration: Some(Duration::from_secs(300)),
                    is_virtual: false,
                    deleted: false,
                    mime_type: Some("video/mp4".to_string()),
                    time_added: None,
                }),
            ].into_iter().collect(),
        };

        storage.save(&data).await.unwrap();

        let loaded = storage.load(&working_dir).await.unwrap();

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
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();

        let virtual_file = PathBuf::from("https://example.com/video.mp4");
        let non_playlist_virtual = PathBuf::from("https://example.com/other.mp4");

        let data = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: vec![virtual_file.clone()],
            files: [
                (virtual_file.clone(), FileMetadata {
                    duration: Some(Duration::from_secs(60)),
                    is_virtual: true,
                    deleted: false,
                    mime_type: Some("video/mp4".to_string()),
                    time_added: None,
                }),
                (non_playlist_virtual.clone(), FileMetadata {
                    duration: Some(Duration::from_secs(60)),
                    is_virtual: true,
                    deleted: false,
                    mime_type: Some("video/mp4".to_string()),
                    time_added: None,
                }),
            ].into_iter().collect(),
        };

        storage.save(&data).await.unwrap();

        let loaded = storage.load(&working_dir).await.unwrap();

        assert_eq!(loaded.playlist.len(), 1);
        let meta = loaded.files.get(&virtual_file).unwrap();
        assert!(meta.is_virtual);

        let non_playlist_meta = loaded.files.get(&non_playlist_virtual).unwrap();
        assert!(non_playlist_meta.is_virtual);
    }

    #[tokio::test]
    async fn deleted_file_handling() {
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();

        let file = working_dir.as_path().join("deleted.mp4");
        let non_playlist_file = working_dir.as_path().join("other_deleted.mp4");

        let data = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: vec![file.clone()],
            files: [
                (file.clone(), FileMetadata {
                    duration: None,
                    is_virtual: false,
                    deleted: true,
                    mime_type: None,
                    time_added: None,
                }),
                (non_playlist_file.clone(), FileMetadata {
                    duration: None,
                    is_virtual: false,
                    deleted: true,
                    mime_type: None,
                    time_added: None,
                }),
            ].into_iter().collect(),
        };

        storage.save(&data).await.unwrap();

        let loaded = storage.load(&working_dir).await.unwrap();
        let meta = loaded.files.get(&file).unwrap();
        assert!(meta.deleted);

        let non_playlist_meta = loaded.files.get(&non_playlist_file).unwrap();
        assert!(non_playlist_meta.deleted);
    }

    #[tokio::test]
    async fn alias_resolution_priority() {
        let storage = create_test_storage().await;

        let workspace1 = PathBuf::from("/workspace1");
        let workspace2 = PathBuf::from("/workspace2");
        let file = PathBuf::from("/shared/file.mp4");

        let ws1_id = storage.get_or_create_workspace(&workspace1).await.unwrap();
        let ws2_id = storage.get_or_create_workspace(&workspace2).await.unwrap();
        let file_id = storage.get_or_create_file_path(&file).await.unwrap();

        storage.upsert_alias(file_id, ws1_id, "Workspace1 Alias").await.unwrap();
        storage.upsert_alias(file_id, ws2_id, "Workspace2 Alias").await.unwrap();

        let alias_ws1 = storage.resolve_alias(file_id, ws1_id).await.unwrap();
        assert_eq!(alias_ws1, Some("Workspace1 Alias".to_string()));

        let alias_ws2 = storage.resolve_alias(file_id, ws2_id).await.unwrap();
        assert_eq!(alias_ws2, Some("Workspace2 Alias".to_string()));
    }

    #[tokio::test]
    async fn alias_fallback_to_most_recent() {
        let storage = create_test_storage().await;

        let workspace1 = PathBuf::from("/workspace1");
        let workspace2 = PathBuf::from("/workspace2");
        let file = PathBuf::from("/shared/file.mp4");

        let ws1_id = storage.get_or_create_workspace(&workspace1).await.unwrap();
        let ws2_id = storage.get_or_create_workspace(&workspace2).await.unwrap();
        let file_id = storage.get_or_create_file_path(&file).await.unwrap();

        storage.upsert_alias(file_id, ws1_id, "First Alias").await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        storage.upsert_alias(file_id, ws2_id, "Second Alias").await.unwrap();

        let ws3_id = storage.get_or_create_workspace(&PathBuf::from("/workspace3")).await.unwrap();
        let fallback = storage.resolve_alias(file_id, ws3_id).await.unwrap();
        assert!(fallback.is_some());
    }

    #[tokio::test]
    async fn multiple_workspaces_isolation() {
        let storage = create_test_storage().await;

        let temp1 = TempDir::new().unwrap();
        let temp2 = TempDir::new().unwrap();
        let workspace1 = CanonicalPath::from_path(temp1.path()).unwrap();
        let workspace2 = CanonicalPath::from_path(temp2.path()).unwrap();

        let data1 = PlaylistData {
            working_directory: workspace1.clone(),
            playlist: vec![PathBuf::from("/ws1/file1.mp4")],
            files: [(PathBuf::from("/ws1/file1.mp4"), FileMetadata {
                duration: Some(Duration::from_secs(100)),
                is_virtual: false,
                deleted: false,
                mime_type: None,
                time_added: None,
            })].into_iter().collect(),
        };

        let data2 = PlaylistData {
            working_directory: workspace2.clone(),
            playlist: vec![PathBuf::from("/ws2/file2.mp4")],
            files: [(PathBuf::from("/ws2/file2.mp4"), FileMetadata {
                duration: Some(Duration::from_secs(200)),
                is_virtual: false,
                deleted: false,
                mime_type: None,
                time_added: None,
            })].into_iter().collect(),
        };

        storage.save(&data1).await.unwrap();
        storage.save(&data2).await.unwrap();

        let loaded1 = storage.load(&workspace1).await.unwrap();
        let loaded2 = storage.load(&workspace2).await.unwrap();

        assert_eq!(loaded1.playlist.len(), 1);
        assert_eq!(loaded1.playlist[0], PathBuf::from("/ws1/file1.mp4"));

        assert_eq!(loaded2.playlist.len(), 1);
        assert_eq!(loaded2.playlist[0], PathBuf::from("/ws2/file2.mp4"));
    }

    #[tokio::test]
    async fn save_overwrites_existing_playlist() {
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();

        let data1 = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: vec![PathBuf::from("/file1.mp4"), PathBuf::from("/file2.mp4")],
            files: [
                (PathBuf::from("/file1.mp4"), FileMetadata {
                    duration: None,
                    is_virtual: false,
                    deleted: false,
                    mime_type: None,
                    time_added: None,
                }),
                (PathBuf::from("/file2.mp4"), FileMetadata {
                    duration: None,
                    is_virtual: false,
                    deleted: false,
                    mime_type: None,
                    time_added: None,
                }),
            ].into_iter().collect(),
        };

        storage.save(&data1).await.unwrap();

        let data2 = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: vec![PathBuf::from("/file3.mp4")],
            files: [(PathBuf::from("/file3.mp4"), FileMetadata {
                duration: None,
                is_virtual: false,
                deleted: false,
                mime_type: None,
                time_added: None,
            })].into_iter().collect(),
        };

        storage.save(&data2).await.unwrap();

        let loaded = storage.load(&working_dir).await.unwrap();
        assert_eq!(loaded.playlist.len(), 1);
        assert_eq!(loaded.playlist[0], PathBuf::from("/file3.mp4"));
    }

    #[tokio::test]
    async fn playlist_order_preserved() {
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();

        let files: Vec<PathBuf> = (0..10)
            .map(|i| PathBuf::from(format!("/file{}.mp4", i)))
            .collect();

        let data = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: files.clone(),
            files: files.iter().map(|path| {
                (path.clone(), FileMetadata {
                    duration: None,
                    is_virtual: false,
                    deleted: false,
                    mime_type: None,
                    time_added: None,
                })
            }).collect(),
        };

        storage.save(&data).await.unwrap();

        let loaded = storage.load(&working_dir).await.unwrap();
        assert_eq!(loaded.playlist.len(), 10);
        for i in 0..10 {
            assert_eq!(loaded.playlist[i], PathBuf::from(format!("/file{}.mp4", i)));
        }
    }

    #[tokio::test]
    async fn time_added_preserved() {
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();

        let timestamp = Timestamp::now();

        let file = working_dir.as_path().join("file.mp4");
        let non_playlist_file = working_dir.as_path().join("other.mp4");

        let data = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: vec![file.clone()],
            files: [
                (file.clone(), FileMetadata {
                    duration: None,
                    is_virtual: false,
                    deleted: false,
                    mime_type: None,
                    time_added: Some(timestamp),
                }),
                (non_playlist_file.clone(), FileMetadata {
                    duration: None,
                    is_virtual: false,
                    deleted: false,
                    mime_type: None,
                    time_added: Some(timestamp),
                }),
            ].into_iter().collect(),
        };

        storage.save(&data).await.unwrap();

        let loaded = storage.load(&working_dir).await.unwrap();
        let meta = loaded.files.get(&file).unwrap();
        assert!(meta.time_added.is_some());

        let non_playlist_meta = loaded.files.get(&non_playlist_file).unwrap();
        assert!(non_playlist_meta.time_added.is_some());
    }

    #[tokio::test]
    async fn storage_name() {
        let storage = create_test_storage().await;
        assert_eq!(storage.name(), "sqlite");
    }

    #[tokio::test]
    async fn upsert_file_metadata_updates_existing() {
        let storage = create_test_storage().await;

        let file = PathBuf::from("/test/file.mp4");
        let file_id = storage.get_or_create_file_path(&file).await.unwrap();

        let meta1 = FileMetadata {
            duration: Some(Duration::from_secs(100)),
            is_virtual: false,
            deleted: false,
            mime_type: Some("video/mp4".to_string()),
            time_added: None,
        };

        storage.upsert_file_metadata(file_id, &meta1).await.unwrap();

        let loaded1 = storage.get_file_metadata(file_id).await.unwrap();
        assert_eq!(loaded1.duration, Some(Duration::from_secs(100)));

        let meta2 = FileMetadata {
            duration: Some(Duration::from_secs(200)),
            is_virtual: false,
            deleted: true,
            mime_type: None,
            time_added: None,
        };

        storage.upsert_file_metadata(file_id, &meta2).await.unwrap();

        let loaded2 = storage.get_file_metadata(file_id).await.unwrap();
        assert_eq!(loaded2.duration, Some(Duration::from_secs(200)));
        assert!(loaded2.deleted);
        assert!(loaded2.mime_type.is_none());
    }

    #[tokio::test]
    async fn virtual_file_flag_toggles() {
        let storage = create_test_storage().await;

        let file = PathBuf::from("/test/file.mp4");
        let file_id = storage.get_or_create_file_path(&file).await.unwrap();

        storage.upsert_virtual_file(file_id, true).await.unwrap();
        assert!(storage.is_virtual_file(file_id).await.unwrap());

        storage.upsert_virtual_file(file_id, false).await.unwrap();
        assert!(!storage.is_virtual_file(file_id).await.unwrap());
    }

    #[tokio::test]
    async fn playlist_item_duration_is_persisted() {
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();

        let file = PathBuf::from("/test/workspace/video.mp4");

        let data = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: vec![file.clone()],
            files: [(file.clone(), FileMetadata {
                duration: Some(Duration::from_secs(120)),
                is_virtual: false,
                deleted: false,
                mime_type: Some("video/mp4".to_string()),
                time_added: None,
            })].into_iter().collect(),
        };

        storage.save(&data).await.unwrap();

        let loaded = storage.load(&working_dir).await.unwrap();

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
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();

        let playlist_file = working_dir.as_path().join("playlist_item.mp4");
        let library_file = working_dir.as_path().join("library_file.mp4");

        let data = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: vec![playlist_file.clone()],
            files: [
                (playlist_file.clone(), FileMetadata {
                    duration: Some(Duration::from_secs(60)),
                    is_virtual: false,
                    deleted: false,
                    mime_type: Some("video/mp4".to_string()),
                    time_added: None,
                }),
                (library_file.clone(), FileMetadata {
                    duration: Some(Duration::from_secs(180)),
                    is_virtual: false,
                    deleted: false,
                    mime_type: Some("video/mp4".to_string()),
                    time_added: None,
                }),
            ].into_iter().collect(),
        };

        storage.save(&data).await.unwrap();

        let loaded = storage.load(&working_dir).await.unwrap();

        assert_eq!(loaded.playlist.len(), 1);
        assert!(loaded.files.contains_key(&library_file));

        let library_meta = loaded.files.get(&library_file).unwrap();
        assert_eq!(library_meta.duration, Some(Duration::from_secs(180)));
        assert_eq!(library_meta.mime_type, Some("video/mp4".to_string()));
    }

    #[tokio::test]
    async fn full_metadata_preserved_across_save_load_cycle() {
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();
        let timestamp = Timestamp::now();

        let playlist_file = working_dir.as_path().join("playlist.mp4");
        let library_file = working_dir.as_path().join("library.mp4");
        let virtual_file = PathBuf::from("https://example.com/stream.mp4");

        let data = PlaylistData {
            working_directory: working_dir.clone(),
            playlist: vec![playlist_file.clone(), virtual_file.clone()],
            files: [
                (playlist_file.clone(), FileMetadata {
                    duration: Some(Duration::from_secs(300)),
                    is_virtual: false,
                    deleted: false,
                    mime_type: Some("video/mp4".to_string()),
                    time_added: Some(timestamp),
                }),
                (library_file.clone(), FileMetadata {
                    duration: Some(Duration::from_secs(450)),
                    is_virtual: false,
                    deleted: true,
                    mime_type: Some("video/x-matroska".to_string()),
                    time_added: Some(timestamp),
                }),
                (virtual_file.clone(), FileMetadata {
                    duration: Some(Duration::from_secs(600)),
                    is_virtual: true,
                    deleted: false,
                    mime_type: Some("video/mp4".to_string()),
                    time_added: Some(timestamp),
                }),
            ].into_iter().collect(),
        };

        storage.save(&data).await.unwrap();

        let loaded = storage.load(&working_dir).await.unwrap();

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
        let storage = create_test_storage().await;
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();

        let file = PathBuf::from("/test/workspace/video.mp4");

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
            },
        );

        storage.save(&data).await.unwrap();

        let loaded1 = storage.load(&working_dir).await.unwrap();
        assert_eq!(
            loaded1.files.get(&file).unwrap().duration,
            Some(Duration::from_secs(100))
        );

        data.files.insert(
            file.clone(),
            FileMetadata {
                duration: Some(Duration::from_secs(200)),
                is_virtual: false,
                deleted: false,
                mime_type: Some("video/mp4".to_string()),
                time_added: None,
            },
        );

        storage.save(&data).await.unwrap();

        let loaded2 = storage.load(&working_dir).await.unwrap();
        assert_eq!(
            loaded2.files.get(&file).unwrap().duration,
            Some(Duration::from_secs(200))
        );
    }
}
