use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
    },
};

use async_trait::async_trait;
use error_stack::Report;
use jiff::Timestamp;
use marked_path::CanonicalPath;

use super::super::{FileMetadata, IoError, PlaylistData, PlaylistStorage};
use crate::tui::ItemPath;

fn item_path_to_pathbuf(item_path: &ItemPath) -> PathBuf {
    match item_path {
        ItemPath::File(canonical) => canonical.to_path_buf(),
        ItemPath::Url(url) => PathBuf::from(url),
    }
}

fn pathbuf_to_item_path(path: PathBuf) -> ItemPath {
    let path_str = path.to_string_lossy();
    if path_str.starts_with("http://") || path_str.starts_with("https://") {
        ItemPath::Url(path_str.into_owned())
    } else {
        ItemPath::File(CanonicalPath::new(path))
    }
}

#[derive(Debug, Default)]
struct StorageData {
    next_workspace_id: i64,
    workspaces: HashMap<PathBuf, i64>,
    playlists: HashMap<i64, Vec<PathBuf>>,
    metadata: HashMap<PathBuf, FileMetadata>,
    virtual_files: HashSet<PathBuf>,
    aliases: HashMap<(PathBuf, PathBuf), (String, Timestamp)>,
}

pub struct FakeStorageBackend {
    data: Arc<RwLock<StorageData>>,
    pub load_called: AtomicUsize,
    pub save_called: AtomicUsize,
}

impl FakeStorageBackend {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(StorageData::default())),
            load_called: AtomicUsize::new(0),
            save_called: AtomicUsize::new(0),
        }
    }

    fn get_or_create_workspace(&self, path: &CanonicalPath) -> i64 {
        let mut data = self.data.write().unwrap();
        let path_buf = path.as_path().to_path_buf();
        if let Some(&id) = data.workspaces.get(&path_buf) {
            return id;
        }
        let id = data.next_workspace_id;
        data.next_workspace_id += 1;
        data.workspaces.insert(path_buf, id);
        id
    }

    fn resolve_alias_internal(&self, file_path: &PathBuf, workspace_path: &PathBuf) -> Option<String> {
        let data = self.data.read().unwrap();

        if let Some((alias, _)) = data.aliases.get(&(file_path.clone(), workspace_path.clone())) {
            return Some(alias.clone());
        }

        let mut most_recent: Option<(&String, &Timestamp)> = None;
        for ((fp, _wp), (alias, ts)) in &data.aliases {
            if fp == file_path {
                if most_recent.is_none() || ts > most_recent.unwrap().1 {
                    most_recent = Some((alias, ts));
                }
            }
        }
        most_recent.map(|(alias, _)| alias.clone())
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn get_workspace_id(&self, path: &CanonicalPath) -> Option<i64> {
        let data = self.data.read().unwrap();
        data.workspaces.get(&path.as_path().to_path_buf()).copied()
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn get_playlist(&self, workspace_id: i64) -> Option<Vec<ItemPath>> {
        let data = self.data.read().unwrap();
        data.playlists.get(&workspace_id).map(|paths| {
            paths.iter().map(|p| pathbuf_to_item_path(p.clone())).collect()
        })
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn get_metadata(&self, path: &ItemPath) -> Option<FileMetadata> {
        let data = self.data.read().unwrap();
        let path_buf = item_path_to_pathbuf(path);
        data.metadata.get(&path_buf).cloned()
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn is_virtual_file(&self, path: &ItemPath) -> bool {
        let data = self.data.read().unwrap();
        let path_buf = item_path_to_pathbuf(path);
        data.virtual_files.contains(&path_buf)
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn get_alias(&self, file_path: &CanonicalPath, workspace_path: &CanonicalPath) -> Option<String> {
        self.resolve_alias_internal(
            &file_path.as_path().to_path_buf(),
            &workspace_path.as_path().to_path_buf(),
        )
    }
}

impl Default for FakeStorageBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PlaylistStorage for FakeStorageBackend {
    fn name(&self) -> &'static str {
        "fake"
    }

    async fn load(&self, working_directory: &CanonicalPath) -> Result<PlaylistData, Report<IoError>> {
        self.load_called.fetch_add(1, Ordering::SeqCst);

        let workspace_id = self.get_or_create_workspace(working_directory);

        let data = self.data.read().unwrap();

        let playlist_paths = data.playlists.get(&workspace_id).cloned().unwrap_or_default();
        let playlist: Vec<ItemPath> = playlist_paths
            .iter()
            .map(|p| pathbuf_to_item_path(p.clone()))
            .collect();

        let workspace_path_buf = working_directory.as_path().to_path_buf();
        let mut files = HashMap::new();
        for path_buf in &playlist_paths {
            let alias = self.resolve_alias_internal(path_buf, &workspace_path_buf);
            let item_path = pathbuf_to_item_path(path_buf.clone());

            if let Some(mut metadata) = data.metadata.get(path_buf).cloned() {
                metadata.alias = alias;
                files.insert(item_path, metadata);
            } else {
                files.insert(
                    item_path,
                    FileMetadata {
                        duration: None,
                        is_virtual: data.virtual_files.contains(path_buf),
                        deleted: false,
                        mime_type: None,
                        time_added: None,
                        alias,
                    },
                );
            }
        }

        Ok(PlaylistData {
            working_directory: working_directory.clone(),
            playlist,
            files,
        })
    }

    async fn save(&self, data: &PlaylistData) -> Result<(), Report<IoError>> {
        self.save_called.fetch_add(1, Ordering::SeqCst);

        let workspace_id = self.get_or_create_workspace(&data.working_directory);

        let mut storage = self.data.write().unwrap();

        let playlist_paths: Vec<PathBuf> = data
            .playlist
            .iter()
            .map(item_path_to_pathbuf)
            .collect();
        storage.playlists.insert(workspace_id, playlist_paths);

        for (item_path, metadata) in &data.files {
            let path_buf = item_path_to_pathbuf(item_path);
            storage.metadata.insert(path_buf.clone(), metadata.clone());
            if metadata.is_virtual {
                storage.virtual_files.insert(path_buf);
            }
        }

        Ok(())
    }

    async fn upsert_alias(
        &self,
        file_path: &CanonicalPath,
        workspace: &CanonicalPath,
        alias: &str,
    ) -> Result<(), Report<IoError>> {
        let mut data = self.data.write().unwrap();
        data.aliases.insert(
            (file_path.as_path().to_path_buf(), workspace.as_path().to_path_buf()),
            (alias.to_string(), Timestamp::now()),
        );
        Ok(())
    }

    async fn resolve_alias(
        &self,
        file_path: &CanonicalPath,
        workspace: &CanonicalPath,
    ) -> Result<Option<String>, Report<IoError>> {
        Ok(self.resolve_alias_internal(
            &file_path.as_path().to_path_buf(),
            &workspace.as_path().to_path_buf(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;

    fn item_path(path: impl Into<PathBuf>) -> ItemPath {
        let path = path.into();
        if path.to_string_lossy().starts_with("http://") || path.to_string_lossy().starts_with("https://") {
            ItemPath::Url(path.to_string_lossy().to_string())
        } else {
            ItemPath::File(CanonicalPath::new(path))
        }
    }

    fn create_test_metadata() -> FileMetadata {
        FileMetadata {
            duration: Some(Duration::from_secs(120)),
            is_virtual: false,
            deleted: false,
            mime_type: Some("audio/mpeg".to_string()),
            time_added: Some(Timestamp::now()),
            alias: None,
        }
    }

    #[tokio::test]
    async fn workspace_isolation() {
        let backend = FakeStorageBackend::new();

        let temp1 = TempDir::new().unwrap();
        let temp2 = TempDir::new().unwrap();
        let workspace1 = CanonicalPath::from_path(temp1.path()).unwrap();
        let workspace2 = CanonicalPath::from_path(temp2.path()).unwrap();

        let file1 = item_path("/workspace1/file1.mp3");
        let file2 = item_path("/workspace2/file2.mp3");

        let data1 = PlaylistData {
            working_directory: workspace1.clone(),
            playlist: vec![file1.clone()],
            files: [(file1.clone(), create_test_metadata())].into_iter().collect(),
        };

        let data2 = PlaylistData {
            working_directory: workspace2.clone(),
            playlist: vec![file2.clone()],
            files: [(file2.clone(), create_test_metadata())].into_iter().collect(),
        };

        backend.save(&data1).await.unwrap();
        backend.save(&data2).await.unwrap();

        let loaded1 = backend.load(&workspace1).await.unwrap();
        let loaded2 = backend.load(&workspace2).await.unwrap();

        assert_eq!(loaded1.playlist.len(), 1);
        assert_eq!(loaded1.playlist[0], file1);
        assert_eq!(loaded2.playlist.len(), 1);
        assert_eq!(loaded2.playlist[0], file2);

        let ws1_id = backend.get_workspace_id(&workspace1).unwrap();
        let ws2_id = backend.get_workspace_id(&workspace2).unwrap();
        assert_ne!(ws1_id, ws2_id);
    }

    #[tokio::test]
    async fn alias_resolution_priority() {
        let backend = FakeStorageBackend::new();

        let workspace1 = CanonicalPath::new(PathBuf::from("/workspace1"));
        let workspace2 = CanonicalPath::new(PathBuf::from("/workspace2"));
        let file = CanonicalPath::new(PathBuf::from("/shared/file.mp3"));

        {
            let mut data = backend.data.write().unwrap();
            data.aliases.insert(
                (file.as_path().to_path_buf(), workspace1.as_path().to_path_buf()),
                ("alias_ws1".to_string(), Timestamp::now()),
            );
            data.aliases.insert(
                (file.as_path().to_path_buf(), workspace2.as_path().to_path_buf()),
                ("alias_ws2".to_string(), Timestamp::now()),
            );
        }

        let alias1 = backend.resolve_alias(&file, &workspace1).await.unwrap();
        let alias2 = backend.resolve_alias(&file, &workspace2).await.unwrap();

        assert_eq!(alias1, Some("alias_ws1".to_string()));
        assert_eq!(alias2, Some("alias_ws2".to_string()));
    }

    #[tokio::test]
    async fn alias_fallback_to_most_recent() {
        let backend = FakeStorageBackend::new();

        let workspace1 = CanonicalPath::new(PathBuf::from("/workspace1"));
        let unknown_workspace = CanonicalPath::new(PathBuf::from("/unknown"));
        let file = CanonicalPath::new(PathBuf::from("/shared/file.mp3"));

        let ts1 = Timestamp::now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let ts2 = Timestamp::now();

        {
            let mut data = backend.data.write().unwrap();
            data.aliases.insert(
                (file.as_path().to_path_buf(), workspace1.as_path().to_path_buf()),
                ("older_alias".to_string(), ts1),
            );
            data.aliases.insert(
                (file.as_path().to_path_buf(), PathBuf::from("/other_workspace")),
                ("newer_alias".to_string(), ts2),
            );
        }

        let alias = backend.resolve_alias(&file, &unknown_workspace).await.unwrap();
        assert_eq!(alias, Some("newer_alias".to_string()));
    }

    #[tokio::test]
    async fn alias_loaded_with_file_metadata() {
        let backend = FakeStorageBackend::new();

        let temp = TempDir::new().unwrap();
        let workspace = CanonicalPath::from_path(temp.path()).unwrap();
        let file = item_path("/workspace/file.mp3");

        let data = PlaylistData {
            working_directory: workspace.clone(),
            playlist: vec![file.clone()],
            files: [(file.clone(), create_test_metadata())].into_iter().collect(),
        };
        backend.save(&data).await.unwrap();

        let file_canonical = CanonicalPath::new(PathBuf::from("/workspace/file.mp3"));
        backend
            .upsert_alias(&file_canonical, &workspace, "My File")
            .await
            .unwrap();

        let loaded = backend.load(&workspace).await.unwrap();
        let meta = loaded.files.get(&file).unwrap();
        assert_eq!(meta.alias, Some("My File".to_string()));
    }

    #[tokio::test]
    async fn virtual_file_handling() {
        let backend = FakeStorageBackend::new();

        let temp = TempDir::new().unwrap();
        let workspace = CanonicalPath::from_path(temp.path()).unwrap();
        let virtual_file = ItemPath::Url("https://example.com/stream.mp3".to_string());
        let regular_file = item_path("/regular/file.mp3");

        let data = PlaylistData {
            working_directory: workspace.clone(),
            playlist: vec![regular_file.clone(), virtual_file.clone()],
            files: [
                (regular_file.clone(), FileMetadata {
                    duration: Some(Duration::from_secs(180)),
                    is_virtual: false,
                    deleted: false,
                    mime_type: None,
                    time_added: None,
                    alias: None,
                }),
                (virtual_file.clone(), FileMetadata {
                    duration: None,
                    is_virtual: true,
                    deleted: false,
                    mime_type: Some("application/x-mpegURL".to_string()),
                    time_added: None,
                    alias: None,
                }),
            ].into_iter().collect(),
        };

        backend.save(&data).await.unwrap();

        assert!(backend.is_virtual_file(&virtual_file));
        assert!(!backend.is_virtual_file(&regular_file));

        let loaded = backend.load(&workspace).await.unwrap();
        assert!(loaded.files.get(&virtual_file).unwrap().is_virtual);
        assert!(!loaded.files.get(&regular_file).unwrap().is_virtual);
    }

    #[tokio::test]
    async fn metadata_persistence() {
        let backend = FakeStorageBackend::new();

        let temp = TempDir::new().unwrap();
        let workspace = CanonicalPath::from_path(temp.path()).unwrap();
        let file = item_path("/workspace/audio.mp3");
        let original_metadata = create_test_metadata();

        let data = PlaylistData {
            working_directory: workspace.clone(),
            playlist: vec![file.clone()],
            files: [(file.clone(), original_metadata.clone())].into_iter().collect(),
        };

        backend.save(&data).await.unwrap();

        let loaded = backend.load(&workspace).await.unwrap();
        let loaded_metadata = loaded.files.get(&file).unwrap();

        assert_eq!(loaded_metadata.duration, original_metadata.duration);
        assert_eq!(loaded_metadata.mime_type, original_metadata.mime_type);
        assert_eq!(loaded_metadata.deleted, original_metadata.deleted);

        let direct_metadata = backend.get_metadata(&file).unwrap();
        assert_eq!(direct_metadata.duration, Some(Duration::from_secs(120)));
    }

    #[tokio::test]
    async fn load_and_save_counters() {
        let backend = FakeStorageBackend::new();
        let temp = TempDir::new().unwrap();
        let workspace = CanonicalPath::from_path(temp.path()).unwrap();

        assert_eq!(backend.load_called.load(Ordering::SeqCst), 0);
        assert_eq!(backend.save_called.load(Ordering::SeqCst), 0);

        backend.load(&workspace).await.unwrap();
        assert_eq!(backend.load_called.load(Ordering::SeqCst), 1);

        let data = PlaylistData {
            working_directory: workspace.clone(),
            playlist: Vec::new(),
            files: HashMap::new(),
        };
        backend.save(&data).await.unwrap();
        assert_eq!(backend.save_called.load(Ordering::SeqCst), 1);

        backend.load(&workspace).await.unwrap();
        backend.load(&workspace).await.unwrap();
        assert_eq!(backend.load_called.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn playlist_order_preserved() {
        let backend = FakeStorageBackend::new();

        let temp = TempDir::new().unwrap();
        let workspace = CanonicalPath::from_path(temp.path()).unwrap();
        let files: Vec<ItemPath> = (0..5)
            .map(|i| item_path(format!("/workspace/file{}.mp3", i)))
            .collect();

        let data = PlaylistData {
            working_directory: workspace.clone(),
            playlist: files.clone(),
            files: files.iter().map(|f| (f.clone(), create_test_metadata())).collect(),
        };

        backend.save(&data).await.unwrap();

        let loaded = backend.load(&workspace).await.unwrap();
        assert_eq!(loaded.playlist, files);
    }

    #[tokio::test]
    async fn empty_workspace_returns_empty_playlist() {
        let backend = FakeStorageBackend::new();
        let temp = TempDir::new().unwrap();
        let workspace = CanonicalPath::from_path(temp.path()).unwrap();

        let loaded = backend.load(&workspace).await.unwrap();
        assert!(loaded.playlist.is_empty());
        assert!(loaded.files.is_empty());
        assert_eq!(loaded.working_directory, workspace);
    }

    #[tokio::test]
    async fn upsert_alias_trait_method() {
        let backend = FakeStorageBackend::new();

        let temp = TempDir::new().unwrap();
        let workspace = CanonicalPath::from_path(temp.path()).unwrap();
        let file = CanonicalPath::new(PathBuf::from("/test/file.mp3"));

        backend
            .upsert_alias(&file, &workspace, "My File")
            .await
            .unwrap();

        let alias = backend
            .resolve_alias(&file, &workspace)
            .await
            .unwrap();
        assert_eq!(alias, Some("My File".to_string()));
    }

    #[tokio::test]
    async fn resolve_alias_returns_none_for_unknown_file() {
        let backend = FakeStorageBackend::new();

        let temp = TempDir::new().unwrap();
        let workspace = CanonicalPath::from_path(temp.path()).unwrap();
        let file = CanonicalPath::new(PathBuf::from("/unknown/file.mp3"));

        let alias = backend.resolve_alias(&file, &workspace).await.unwrap();
        assert!(alias.is_none());
    }
}
