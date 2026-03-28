use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use error_stack::{Report, ResultExt};
use marked_path::CanonicalPath;

use crate::command::{CommandError, CommandResult};
use crate::feat::playlist::{FileMetadata, PlaylistData};
use crate::system_ctx::SystemCtx;
use crate::common::domain::{get_mime_type, ItemPath, PlaylistItem};

/// # Errors
///
/// Returns an error if storage operations fail.
pub async fn load_playlist(
    ctx: &SystemCtx,
) -> Result<CommandResult, Report<CommandError>> {
    let data = ctx
        .services
        .storage
        .load(&ctx.library_path)
        .await
        .change_context(CommandError)?;

    let playlist_paths: HashSet<_> = data.playlist.iter().cloned().collect();

    let path_counts = ctx
        .services
        .storage
        .get_path_counts()
        .await
        .change_context(CommandError)?;

    let mut path_to_count: HashMap<ItemPath, usize> = HashMap::new();
    for path in &data.playlist {
        let count = ctx
            .services
            .storage
            .resolve_file_path_id(path)
            .await
            .ok()
            .flatten()
            .and_then(|id| path_counts.get(&id).copied())
            .unwrap_or(1);
        path_to_count.insert(path.clone(), count);
    }

    let playlist_items: Vec<PlaylistItem> = data
        .playlist
        .into_iter()
        .map(|path| {
            let metadata = data.files.get(&path);
            let is_virtual = metadata.is_some_and(|m| m.is_virtual);
            let duration = metadata.and_then(|m| m.duration);
            let mime_type = metadata
                .and_then(|m| m.mime_type.clone())
                .or_else(|| get_mime_type(&path));
            let playlist_count = path_to_count.get(&path).copied().unwrap_or(1);
            PlaylistItem {
                path,
                duration,
                alias: metadata.and_then(|m| m.alias.clone()),
                mime_type,
                is_virtual,
                playlist_count,
                has_sources: true,
            }
        })
        .collect();

    let mut virtual_library_items: Vec<PlaylistItem> = data
        .files
        .into_iter()
        .filter(|(path, metadata)| metadata.is_virtual && !playlist_paths.contains(path))
        .map(|(path, metadata)| {
            let item_path = ItemPath::Url(path.to_string_lossy().to_string());
            let mime_type = metadata.mime_type.or_else(|| get_mime_type(&item_path));
            PlaylistItem {
                path: item_path,
                duration: metadata.duration,
                alias: metadata.alias.clone(),
                mime_type,
                is_virtual: true,
                playlist_count: 0,
                has_sources: true,
            }
        })
        .collect();
    virtual_library_items.sort_by(|a, b| a.path.to_string_lossy().cmp(&b.path.to_string_lossy()));

    Ok(CommandResult::PlaylistLoaded {
        playlist_items,
        virtual_library_items,
    })
}

/// # Errors
///
/// Returns an error if storage operations fail.
pub async fn save_playlist(
    ctx: &SystemCtx,
    playlist_items: &[PlaylistItem],
    library_items: &[PlaylistItem],
) -> Result<CommandResult, Report<CommandError>> {
    let mut files = HashMap::new();
    for item in playlist_items {
        files.insert(
            item.path.clone(),
            FileMetadata {
                duration: item.duration,
                is_virtual: item.is_virtual,
                deleted: false,
                mime_type: item.mime_type.clone(),
                time_added: None,
                alias: item.alias.clone(),
            },
        );
    }
    for item in library_items {
        files.insert(
            item.path.clone(),
            FileMetadata {
                duration: item.duration,
                is_virtual: item.is_virtual,
                deleted: false,
                mime_type: item.mime_type.clone(),
                time_added: None,
                alias: item.alias.clone(),
            },
        );
    }
    let playlist_paths: Vec<ItemPath> = playlist_items.iter().map(|item| item.path.clone()).collect();
    let data = PlaylistData {
        working_directory: ctx.library_path.clone(),
        playlist: playlist_paths,
        files,
    };
    ctx.services.storage.save(&data).await.change_context(CommandError)?;
    Ok(CommandResult::PlaylistSaved)
}

/// # Errors
///
/// Returns an error if storage operations fail.
pub async fn refresh_library(
    ctx: &SystemCtx,
) -> Result<CommandResult, Report<CommandError>> {
    let mut entries = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(ctx.library_path.as_path()) {
        let paths: Vec<_> = read_dir
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect();

        let workspace = ctx.library_path.clone();
        let services = ctx.services.clone();
        let aliases: HashMap<PathBuf, Option<String>> = {
            let mut result = HashMap::new();
            for path in &paths {
                let Ok(canonical) = CanonicalPath::from_path(path.canonicalize().unwrap_or_else(|_| path.clone())) else {
                    continue;
                };
                let alias = services
                    .storage
                    .resolve_alias(&canonical, &workspace)
                    .await
                    .ok()
                    .flatten();
                result.insert(canonical.to_path_buf(), alias);
            }
            result
        };

        for path in paths {
            let canonical = path.canonicalize().unwrap_or(path);
            let duration = ctx.services.media.get_duration(&canonical).ok();
            let Ok(cp) = CanonicalPath::from_path(&canonical) else {
                continue;
            };
            let item_path = ItemPath::File(cp);
            let mime_type = get_mime_type(&item_path);
            let alias = aliases.get(&canonical).cloned().flatten();
            entries.push(PlaylistItem {
                path: item_path,
                duration,
                alias,
                mime_type,
                is_virtual: false,
                playlist_count: 0,
                has_sources: true,
            });
        }
    }

    let item_paths: Vec<ItemPath> = entries.iter().map(|e| e.path.clone()).collect();
    let counts = get_item_counts(ctx, &item_paths).await;

    for entry in &mut entries {
        entry.playlist_count = counts.get(&entry.path).copied().unwrap_or(0);
    }

    entries.sort_by(|a, b| a.path.to_string_lossy().cmp(&b.path.to_string_lossy()));
    Ok(CommandResult::LibraryRefreshed { items: entries })
}

pub fn add_url(url: &str) -> PlaylistItem {
    PlaylistItem {
        path: ItemPath::Url(url.to_string()),
        duration: None,
        alias: None,
        mime_type: Some("url".to_string()),
        is_virtual: true,
        playlist_count: 0,
        has_sources: true,
    }
}

/// # Errors
///
/// Returns an error if storage operations fail.
pub async fn rename_alias(
    ctx: &SystemCtx,
    path: &ItemPath,
    alias: &str,
) -> Result<CommandResult, Report<CommandError>> {
    if let Some(file_path) = path.as_file() {
        ctx.services
            .storage
            .upsert_alias(file_path, &ctx.library_path, alias)
            .await
            .change_context(CommandError)?;
    }
    Ok(CommandResult::AliasRenamed {
        path: path.clone(),
        alias: alias.to_string(),
    })
}

pub async fn get_item_counts(
    ctx: &SystemCtx,
    items: &[ItemPath],
) -> HashMap<ItemPath, usize> {
    let Ok(path_counts) = ctx.services.storage.get_path_counts().await else {
        return HashMap::new();
    };

    let mut result = HashMap::new();
    for path in items {
        let count = ctx.services.storage.resolve_file_path_id(path).await
            .ok().flatten()
            .and_then(|id| path_counts.get(&id).copied())
            .unwrap_or(0);
        result.insert(path.clone(), count);
    }
    result
}

/// # Errors
///
/// Returns an error if storage operations fail.
pub async fn analyze_library(
    ctx: &SystemCtx,
) -> Result<CommandResult, Report<CommandError>> {
    use crate::feat::media_duration_analysis::analyze_files;
    use crate::feat::media_query::Ffprobe;

    let data = ctx
        .services
        .storage
        .load(&ctx.library_path)
        .await
        .change_context(CommandError)?;

    let mut files: HashSet<CanonicalPath> = HashSet::new();
    if let Ok(read_dir) = std::fs::read_dir(ctx.library_path.as_path()) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_file() && ctx.config.is_video_or_audio(&path) {
                if let Ok(canonical) = CanonicalPath::from_path(&path) {
                    files.insert(canonical);
                }
            }
        }
    }

    let metadata: HashMap<CanonicalPath, FileMetadata> = data
        .files
        .iter()
        .filter_map(|(k, v)| k.as_file().map(|cp| (cp.clone(), v.clone())))
        .collect();

    let uncached: Vec<_> = files
        .iter()
        .filter(|p| {
            !metadata.contains_key(*p)
                || metadata.get(*p).and_then(|m| m.duration).is_none()
        })
        .cloned()
        .collect();

    let new_files_count = uncached.len();

    let ffprobe = Ffprobe;
    let result = analyze_files(&uncached, metadata, &ffprobe, true)
        .change_context(CommandError)?;

    let mut updated_files = data.files.clone();
    for (path, meta) in result.files {
        updated_files.insert(ItemPath::File(path), meta);
    }

    let updated_data = PlaylistData {
        working_directory: ctx.library_path.clone(),
        playlist: data.playlist,
        files: updated_files,
    };
    ctx.services
        .storage
        .save(&updated_data)
        .await
        .change_context(CommandError)?;

    Ok(CommandResult::LibraryAnalyzed { new_files_count })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::command::CommandResult;
    use crate::feat::config::Config;
    use crate::feat::keymap::Keymap;
    use crate::feat::playlist::{FakeStorageBackend, FileMetadata, PlaylistData, PlaylistStorage};
    use crate::services::Services;
    use crate::test_utils::NoteTestContext;

    async fn create_ctx_with_storage(storage: Arc<FakeStorageBackend>, library_path: CanonicalPath) -> SystemCtx {
        let rt = tokio::runtime::Handle::current();
        let db = Arc::new(crate::feat::note_db::SqliteNoteDb::new("sqlite::memory:").await.unwrap());
        SystemCtx {
            services: Services {
                storage: crate::feat::playlist::PlaylistStorageService::new(storage),
                media: crate::feat::media_query::MediaQueryService::new(Arc::new(
                    crate::test_utils::FakeMediaBackend,
                )),
                mpv: crate::feat::mpv::MpvClientService::new(Arc::new(crate::test_utils::FakeMpvBackend)),
                mpv_launcher: crate::feat::mpv::MpvLauncherService::new(Arc::new(
                    crate::test_utils::FakeMpvLauncher::new(),
                )),
                file_launcher: crate::feat::launcher::FileLauncherService::new(Arc::new(
                    crate::test_utils::FakeLauncher::new(),
                )),
                db: crate::feat::note_db::NoteDbService::new(db.clone()),
                editor: crate::feat::external_editor::ExternalEditorService::new(Arc::new(
                    crate::feat::external_editor::SystemEditor,
                )),
                path_resolver: crate::feat::path_resolver::PathResolverService::new(Arc::new(
                    crate::feat::path_resolver::SystemPathResolver,
                )),
                sources: crate::feat::sources::SourceDbService::new(Arc::new(
                    crate::feat::sources::SqliteSourceDb::new(db.pool().clone()),
                )),
                fuzzy_search: crate::feat::fuzzy_search::FuzzySearchService::new(Arc::new(
                    crate::feat::fuzzy_search::SkimBackend,
                )),
                rt,
            },
            config: Config::default(),
            library_path,
            socket_path: String::new(),
            keymap: Keymap::new(),
        }
    }

    #[tokio::test]
    async fn load_playlist_filters_virtual_items_not_in_playlist() {
        // Given storage with virtual items in files, some in playlist and some not.
        let temp = tempfile::TempDir::new().unwrap();
        let library_path = CanonicalPath::from_path(temp.path()).unwrap();
        let storage = Arc::new(FakeStorageBackend::new());

        let virtual_in_playlist = ItemPath::Url("https://example.com/in-playlist.mp3".to_string());
        let virtual_not_in_playlist = ItemPath::Url("https://example.com/not-in-playlist.mp3".to_string());
        let regular_temp = tempfile::NamedTempFile::new().unwrap();
        let regular_file = ItemPath::File(CanonicalPath::from_path(regular_temp.path()).unwrap());

        let data = PlaylistData {
            working_directory: library_path.clone(),
            playlist: vec![virtual_in_playlist.clone()],
            files: [
                (
                    virtual_in_playlist.clone(),
                    FileMetadata {
                        duration: Some(std::time::Duration::from_secs(100)),
                        is_virtual: true,
                        deleted: false,
                        mime_type: None,
                        time_added: None,
                        alias: None,
                    },
                ),
                (
                    virtual_not_in_playlist.clone(),
                    FileMetadata {
                        duration: Some(std::time::Duration::from_secs(200)),
                        is_virtual: true,
                        deleted: false,
                        mime_type: None,
                        time_added: None,
                        alias: None,
                    },
                ),
                (
                    regular_file.clone(),
                    FileMetadata {
                        duration: Some(std::time::Duration::from_secs(300)),
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
        storage.save(&data).await.unwrap();

        let ctx = create_ctx_with_storage(storage, library_path).await;

        // When loading the playlist.
        let result = load_playlist(&ctx).await.unwrap();

        // Then virtual items in playlist go to playlist_items, virtual items not in playlist go to virtual_library_items.
        if let CommandResult::PlaylistLoaded {
            playlist_items,
            virtual_library_items,
        } = result
        {
            assert_eq!(playlist_items.len(), 1);
            assert_eq!(playlist_items[0].path, virtual_in_playlist);
            assert!(playlist_items[0].is_virtual);

            assert_eq!(virtual_library_items.len(), 1);
            assert_eq!(virtual_library_items[0].path, virtual_not_in_playlist);
            assert!(virtual_library_items[0].is_virtual);
        } else {
            panic!("Expected PlaylistLoaded result");
        }
    }

    #[tokio::test]
    async fn analyze_library_only_processes_uncached_files() {
        // Given a temp directory with media files, some with cached durations.
        let temp = tempfile::TempDir::new().unwrap();
        let library_path = CanonicalPath::from_path(temp.path()).unwrap();
        let storage = Arc::new(FakeStorageBackend::new());

        let cached_file = temp.path().join("cached.mp3");
        let uncached_file = temp.path().join("uncached.mp3");
        std::fs::write(&cached_file, "audio data").unwrap();
        std::fs::write(&uncached_file, "audio data").unwrap();

        let cached_path = ItemPath::File(CanonicalPath::from_path(&cached_file).unwrap());
        let data = PlaylistData {
            working_directory: library_path.clone(),
            playlist: vec![],
            files: [(
                cached_path.clone(),
                FileMetadata {
                    duration: Some(std::time::Duration::from_secs(120)),
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

        let ctx = create_ctx_with_storage(storage, library_path).await;

        // When analyzing the library.
        let result = analyze_library(&ctx).await.unwrap();

        // Then only uncached files are processed (1 new file).
        if let CommandResult::LibraryAnalyzed { new_files_count } = result {
            assert_eq!(new_files_count, 1);
        } else {
            panic!("Expected LibraryAnalyzed result");
        }
    }

    #[tokio::test]
    async fn get_item_counts_returns_correct_counts() {
        // Given storage with path counts for items.
        let temp = tempfile::TempDir::new().unwrap();
        let library_path = CanonicalPath::from_path(temp.path()).unwrap();
        let storage = Arc::new(FakeStorageBackend::new());

        let temp1 = tempfile::NamedTempFile::new().unwrap();
        let temp2 = tempfile::NamedTempFile::new().unwrap();
        let file1 = ItemPath::File(CanonicalPath::from_path(temp1.path()).unwrap());
        let file2 = ItemPath::File(CanonicalPath::from_path(temp2.path()).unwrap());

        let data = PlaylistData {
            working_directory: library_path.clone(),
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
        storage.save(&data).await.unwrap();

        let ctx = create_ctx_with_storage(storage, library_path).await;

        // When getting item counts.
        let counts = get_item_counts(&ctx, &[file1.clone(), file2.clone()]).await;

        // Then correct counts are returned.
        assert_eq!(counts.get(&file1), Some(&1));
        assert_eq!(counts.get(&file2), Some(&1));
    }

    #[tokio::test]
    async fn get_item_counts_returns_zero_for_unknown_paths() {
        // Given storage with some path counts.
        let temp = tempfile::TempDir::new().unwrap();
        let library_path = CanonicalPath::from_path(temp.path()).unwrap();
        let storage = Arc::new(FakeStorageBackend::new());

        let known_temp = tempfile::NamedTempFile::new().unwrap();
        let unknown_temp = tempfile::NamedTempFile::new().unwrap();
        let known_file = ItemPath::File(CanonicalPath::from_path(known_temp.path()).unwrap());
        let unknown_file = ItemPath::File(CanonicalPath::from_path(unknown_temp.path()).unwrap());

        let data = PlaylistData {
            working_directory: library_path.clone(),
            playlist: vec![known_file.clone()],
            files: [(
                known_file.clone(),
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

        let ctx = create_ctx_with_storage(storage, library_path).await;

        // When getting item counts for unknown paths.
        let counts = get_item_counts(&ctx, std::slice::from_ref(&unknown_file)).await;

        // Then zero is returned for unknown paths.
        assert_eq!(counts.get(&unknown_file), Some(&0));
    }

    #[tokio::test]
    async fn analyze_library_returns_zero_for_empty_directory() {
        // Given a temp directory with no media files.
        let temp = tempfile::TempDir::new().unwrap();
        std::fs::write(temp.path().join("readme.txt"), "not a media file").unwrap();

        let ctx = NoteTestContext::new().await;
        let mut ctx = ctx.ctx;
        ctx.library_path = marked_path::CanonicalPath::from_path(temp.path()).unwrap();

        // When analyzing the library.
        let result = analyze_library(&ctx).await.unwrap();

        // Then no new files are analyzed.
        assert!(matches!(result, CommandResult::LibraryAnalyzed { new_files_count: 0 }));
    }

    #[tokio::test]
    async fn analyze_library_returns_zero_for_directory_with_non_media_files() {
        // Given a temp directory with files that are not video/audio.
        let temp = tempfile::TempDir::new().unwrap();
        std::fs::write(temp.path().join("document.pdf"), "pdf content").unwrap();
        std::fs::write(temp.path().join("image.png"), "png content").unwrap();
        std::fs::write(temp.path().join("data.json"), "{}").unwrap();

        let ctx = NoteTestContext::new().await;
        let mut ctx = ctx.ctx;
        ctx.library_path = marked_path::CanonicalPath::from_path(temp.path()).unwrap();

        // When analyzing the library.
        let result = analyze_library(&ctx).await.unwrap();

        // Then no new files are analyzed (non-media files are skipped).
        assert!(matches!(result, CommandResult::LibraryAnalyzed { new_files_count: 0 }));
    }
}
