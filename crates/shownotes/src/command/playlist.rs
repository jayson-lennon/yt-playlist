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

    let mut path_counts: HashMap<ItemPath, usize> = HashMap::new();
    for path in &data.playlist {
        *path_counts.entry(path.clone()).or_insert(0) += 1;
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
            let playlist_count = *path_counts.get(&path).unwrap_or(&1);
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
    _playlist_counts: Option<&HashMap<ItemPath, usize>>,
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
