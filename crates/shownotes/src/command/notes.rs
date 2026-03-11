use std::collections::HashMap;
use std::path::Path;

use error_stack::{Report, ResultExt};

use crate::feat::{
    external_editor::ExternalEditor, note_db::NoteDb,
    path_resolver::PathResolver, symlink::create_symlink_with_suffix,
};
use crate::services::Services;

use super::CommandError;

/// # Errors
///
/// Returns an error if path resolution, database operations, or editor invocation fails.
pub async fn add(
    services: &Services,
    paths: Vec<std::path::PathBuf>,
) -> Result<Vec<std::path::PathBuf>, Report<CommandError>> {
    if paths.is_empty() {
        return Err(Report::new(CommandError));
    }

    let mut resolved_paths = Vec::with_capacity(paths.len());
    for path in paths {
        let resolved = services
            .path_resolver
            .resolve(&path)
            .await
            .change_context(CommandError)?;
        resolved_paths.push(resolved);
    }

    if resolved_paths.len() == 1 {
        let resolved_path = &resolved_paths[0];
        let path_str = resolved_path.to_string_lossy();
        let file_path_id = services
            .db
            .get_or_create_file_path(&path_str)
            .await
            .change_context(CommandError)?;

        let existing_note = services
            .db
            .get_note(file_path_id)
            .await
            .change_context(CommandError)?;

        let initial_content = existing_note.unwrap_or_default();
        if let Some(new_content) = services
            .editor
            .open(&initial_content)
            .await
            .change_context(CommandError)?
        {
            services
                .db
                .upsert_note(file_path_id, &new_content)
                .await
                .change_context(CommandError)?;
        }
    } else {
        let Some(new_content) = services.editor.open("").await.change_context(CommandError)? else {
            return Ok(resolved_paths);
        };

        for resolved_path in &resolved_paths {
            upsert_note_with_prepend(services, resolved_path, &new_content).await?;
        }
    }

    Ok(resolved_paths)
}

async fn upsert_note_with_prepend(
    services: &Services,
    resolved_path: &Path,
    new_content: &str,
) -> Result<(), Report<CommandError>> {
    let path_str = resolved_path.to_string_lossy();
    let file_path_id = services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .change_context(CommandError)?;

    let existing_note = services
        .db
        .get_note(file_path_id)
        .await
        .change_context(CommandError)?;

    let final_content = match existing_note {
        Some(existing) => format!("{existing}\n\n{new_content}"),
        None => new_content.to_owned(),
    };

    services
        .db
        .upsert_note(file_path_id, &final_content)
        .await
        .change_context(CommandError)?;

    Ok(())
}

/// # Errors
///
/// Returns an error if database search or current directory retrieval fails.
pub async fn search(
    services: &Services,
    query: &str,
    create_symlinks: bool,
) -> Result<(Vec<String>, usize), Report<CommandError>> {
    let results: Vec<_> = services
        .db
        .search_notes(query)
        .await
        .change_context(CommandError)?
        .into_iter()
        .collect();

    let mut symlinks_created = 0;
    if create_symlinks {
        let cwd = std::env::current_dir().change_context(CommandError)?;
        for path in &results {
            let src = Path::new(path);
            match create_symlink_with_suffix(src, &cwd) {
                Ok(_) => symlinks_created += 1,
                Err(e) => eprintln!("Failed to create symlink for {path}: {e:?}"),
            }
        }
    }

    Ok((results, symlinks_created))
}

/// # Errors
///
/// Returns an error if path resolution or database operations fail.
pub async fn add_alias_as_note(
    services: &Services,
    path: &Path,
    alias: &str,
) -> Result<bool, Report<CommandError>> {
    if alias.is_empty() {
        return Ok(false);
    }

    let resolved = services
        .path_resolver
        .resolve(path)
        .await
        .change_context(CommandError)?;
    let path_str = resolved.to_string_lossy();

    let file_path_id = services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .change_context(CommandError)?;
    let existing = services
        .db
        .get_note(file_path_id)
        .await
        .change_context(CommandError)?;

    if let Some(notes) = &existing {
        for line in notes.lines() {
            if line == alias || line.contains(alias) {
                return Ok(false);
            }
        }
    }

    let new_content = match existing {
        Some(notes) if !notes.is_empty() => format!("{notes}\n{alias}"),
        _ => alias.to_string(),
    };

    services
        .db
        .upsert_note(file_path_id, &new_content)
        .await
        .change_context(CommandError)?;
    Ok(true)
}

pub async fn migrate_aliases_to_notes<S>(
    services: &Services,
    files: &HashMap<std::path::PathBuf, crate::feat::playlist::FileMetadata, S>,
) -> (usize, usize)
where
    S: std::hash::BuildHasher,
{
    let mut migrated = 0;
    let mut skipped = 0;

    for (path, metadata) in files {
        if let Some(alias) = &metadata.alias {
            match add_alias_as_note(services, path, alias).await {
                Ok(true) => migrated += 1,
                Ok(false) | Err(_) => skipped += 1,
            }
        }
    }

    (migrated, skipped)
}

/// # Errors
///
/// Returns an error if database operations, fuzzy search, or current directory retrieval fails.
pub async fn fuzzy(
    services: &Services,
    create_symlinks: bool,
) -> Result<(Vec<String>, usize), Report<CommandError>> {
    let notes = services
        .db
        .get_all_notes_with_paths()
        .await
        .change_context(CommandError)?;

    if notes.is_empty() {
        return Ok((Vec::new(), 0));
    }

    let result = services
        .fuzzy_search
        .search(&notes)
        .change_context(CommandError)?;

    let mut symlinks_created = 0;
    if create_symlinks {
        let cwd = std::env::current_dir().change_context(CommandError)?;
        for path in &result.selected_paths {
            let src = Path::new(path);
            match create_symlink_with_suffix(src, &cwd) {
                Ok(_) => symlinks_created += 1,
                Err(e) => eprintln!("Failed to create symlink for {path}: {e:?}"),
            }
        }
    }

    Ok((result.selected_paths, symlinks_created))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use std::collections::HashMap;

    use crate::feat::note_db::{NoteDb, SqliteNoteDb};
    use crate::feat::path_resolver::PathResolverService;
    use crate::feat::path_resolver::SystemPathResolver;
    use crate::services::Services;
    use tempfile::NamedTempFile;

    async fn create_test_services() -> Services {
        let db = Arc::new(SqliteNoteDb::new("sqlite::memory:").await.unwrap());
        let path_resolver = Arc::new(SystemPathResolver);
        let rt = tokio::runtime::Handle::current();

        Services {
            mpv: crate::feat::mpv::MpvClientService::new(Arc::new(
                crate::feat::mpv::MpvIpc::new(&std::path::PathBuf::new()),
            )),
            media: crate::feat::media_query::MediaQueryService::new(Arc::new(
                crate::feat::media_query::Ffprobe,
            )),
            storage: crate::feat::playlist::PlaylistStorageService::new(Arc::new(
                crate::feat::playlist::TomlStorage::new(std::path::PathBuf::new()),
            )),
            mpv_launcher: crate::feat::mpv::MpvLauncherService::new(Arc::new(
                crate::feat::mpv::RealMpvLauncher,
            )),
            file_launcher: crate::feat::launcher::FileLauncherService::new(Arc::new(
                crate::feat::launcher::XdgLauncher::new(),
            )),
            db: crate::feat::note_db::NoteDbService::new(db),
            editor: crate::feat::external_editor::ExternalEditorService::new(Arc::new(
                crate::feat::external_editor::SystemEditor,
            )),
            path_resolver: PathResolverService::new(path_resolver),
            sources: crate::feat::sources::SourceDbService::new(Arc::new(
                crate::feat::sources::db::sqlite::SqliteSourceDb::new(
                    sqlx::SqlitePool::connect_with(sqlx::sqlite::SqliteConnectOptions::new())
                        .await
                        .unwrap(),
                ),
            )),
            fuzzy_search: crate::feat::fuzzy_search::FuzzySearchService::new(Arc::new(
                crate::feat::fuzzy_search::backend::SkimBackend,
            )),
            rt,
        }
    }

    fn create_temp_file() -> NamedTempFile {
        NamedTempFile::new().unwrap()
    }

    #[tokio::test]
    async fn add_alias_as_note_skips_blank_alias() {
        // Given
        let services = create_test_services().await;
        let temp = create_temp_file();

        // When
        let result = super::add_alias_as_note(&services, temp.path(), "").await.unwrap();

        // Then
        assert!(!result);
    }

    #[tokio::test]
    async fn add_alias_as_note_adds_when_no_conflicts() {
        // Given
        let services = create_test_services().await;
        let temp = create_temp_file();
        let path_str = temp.path().to_string_lossy();
        let file_path_id = services.db.get_or_create_file_path(&path_str).await.unwrap();

        // When
        let result = super::add_alias_as_note(&services, temp.path(), "my-alias")
            .await
            .unwrap();

        // Then
        assert!(result);
        let note = services.db.get_note(file_path_id).await.unwrap();
        assert_eq!(note, Some("my-alias".to_string()));
    }

    #[tokio::test]
    async fn add_alias_as_note_skips_exact_match() {
        // Given
        let services = create_test_services().await;
        let temp = create_temp_file();
        let path_str = temp.path().to_string_lossy();
        let file_path_id = services.db.get_or_create_file_path(&path_str).await.unwrap();
        services
            .db
            .upsert_note(file_path_id, "foo")
            .await
            .unwrap();

        // When
        let result = super::add_alias_as_note(&services, temp.path(), "foo")
            .await
            .unwrap();

        // Then
        assert!(!result);
        let note = services.db.get_note(file_path_id).await.unwrap();
        assert_eq!(note, Some("foo".to_string()));
    }

    #[tokio::test]
    async fn add_alias_as_note_skips_substring_match() {
        // Given
        let services = create_test_services().await;
        let temp = create_temp_file();
        let path_str = temp.path().to_string_lossy();
        let file_path_id = services.db.get_or_create_file_path(&path_str).await.unwrap();
        services
            .db
            .upsert_note(file_path_id, "foo bar")
            .await
            .unwrap();

        // When
        let result = super::add_alias_as_note(&services, temp.path(), "foo")
            .await
            .unwrap();

        // Then
        assert!(!result);
        let note = services.db.get_note(file_path_id).await.unwrap();
        assert_eq!(note, Some("foo bar".to_string()));
    }

    #[tokio::test]
    async fn add_alias_as_note_appends_to_existing() {
        // Given
        let services = create_test_services().await;
        let temp = create_temp_file();
        let path_str = temp.path().to_string_lossy();
        let file_path_id = services.db.get_or_create_file_path(&path_str).await.unwrap();
        services
            .db
            .upsert_note(file_path_id, "first")
            .await
            .unwrap();

        // When
        let result = super::add_alias_as_note(&services, temp.path(), "second")
            .await
            .unwrap();

        // Then
        assert!(result);
        let note = services.db.get_note(file_path_id).await.unwrap();
        assert_eq!(note, Some("first\nsecond".to_string()));
    }

    #[tokio::test]
    async fn migrate_aliases_to_notes_migrates_files_with_aliases() {
        // Given services and files with aliases.
        let services = create_test_services().await;
        let temp1 = create_temp_file();
        let temp2 = create_temp_file();

        let mut files = HashMap::new();
        files.insert(
            temp1.path().to_path_buf(),
            crate::feat::playlist::FileMetadata {
                duration: None,
                alias: Some("alias1".to_string()),
                is_virtual: false,
                deleted: false,
                mime_type: None,
            },
        );
        files.insert(
            temp2.path().to_path_buf(),
            crate::feat::playlist::FileMetadata {
                duration: None,
                alias: None,
                is_virtual: false,
                deleted: false,
                mime_type: None,
            },
        );

        // When migrating.
        let (migrated, skipped) = super::migrate_aliases_to_notes(&services, &files).await;

        // Then only files with aliases are migrated.
        assert_eq!(migrated, 1);
        assert_eq!(skipped, 0);
    }

    #[tokio::test]
    async fn migrate_aliases_to_notes_skips_files_without_aliases() {
        // Given services and files HashMap with entries but no aliases.
        let services = create_test_services().await;
        let temp1 = create_temp_file();
        let temp2 = create_temp_file();

        let mut files = HashMap::new();
        files.insert(
            temp1.path().to_path_buf(),
            crate::feat::playlist::FileMetadata {
                duration: None,
                alias: None,
                is_virtual: false,
                deleted: false,
                mime_type: None,
            },
        );
        files.insert(
            temp2.path().to_path_buf(),
            crate::feat::playlist::FileMetadata {
                duration: None,
                alias: None,
                is_virtual: false,
                deleted: false,
                mime_type: None,
            },
        );

        // When calling migrate_aliases_to_notes.
        let (migrated, skipped) = super::migrate_aliases_to_notes(&services, &files).await;

        // Then nothing is migrated and skipped count is 0.
        assert_eq!(migrated, 0);
        assert_eq!(skipped, 0);
    }

    #[tokio::test]
    async fn migrate_aliases_to_notes_is_idempotent() {
        // Given services and files with aliases.
        let services = create_test_services().await;
        let temp = create_temp_file();

        let mut files = HashMap::new();
        files.insert(
            temp.path().to_path_buf(),
            crate::feat::playlist::FileMetadata {
                duration: None,
                alias: Some("alias1".to_string()),
                is_virtual: false,
                deleted: false,
                mime_type: None,
            },
        );

        // When calling migrate_aliases_to_notes twice.
        let (migrated1, skipped1) = super::migrate_aliases_to_notes(&services, &files).await;
        let (migrated2, skipped2) = super::migrate_aliases_to_notes(&services, &files).await;

        // Then the second call skips all (already exists).
        assert_eq!(migrated1, 1);
        assert_eq!(skipped1, 0);
        assert_eq!(migrated2, 0);
        assert_eq!(skipped2, 1);
    }
}
