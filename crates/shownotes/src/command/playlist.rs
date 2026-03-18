use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use error_stack::{Report, ResultExt};
use marked_path::CanonicalPath;

use crate::command::{CommandError, CommandResult};
use crate::feat::playlist::{FileMetadata, PlaylistData};
use crate::system_ctx::SystemCtx;
use crate::common::domain::{get_mime_type, ItemPath, PlaylistItem};

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
            }
        })
        .collect();
    virtual_library_items.sort_by(|a, b| a.path.to_string_lossy().cmp(&b.path.to_string_lossy()));

    Ok(CommandResult::PlaylistLoaded {
        playlist_items,
        virtual_library_items,
    })
}

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
                let canonical = CanonicalPath::new(path.canonicalize().unwrap_or_else(|_| path.clone()));
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
            let item_path = ItemPath::File(CanonicalPath::new(canonical.clone()));
            let mime_type = get_mime_type(&item_path);
            let alias = aliases.get(&canonical).cloned().flatten();
            entries.push(PlaylistItem {
                path: item_path,
                duration,
                alias,
                mime_type,
                is_virtual: false,
                playlist_count: 0,
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
    }
}

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
    use super::*;
    use crate::command::CommandResult;
    use crate::test_utils::NoteTestContext;

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
