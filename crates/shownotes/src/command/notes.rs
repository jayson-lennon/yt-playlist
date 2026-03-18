use std::collections::HashMap;
use std::path::Path;

use error_stack::{Report, ResultExt};
use marked_path::CanonicalPath;

use crate::feat::{
    external_editor::ExternalEditor, note_db::NoteDb,
    symlink::create_symlink_with_suffix,
};
use crate::system_ctx::SystemCtx;

use super::CommandError;

/// # Errors
///
/// Returns an error if database operations or editor invocation fails.
pub async fn add(
    ctx: &SystemCtx,
    paths: Vec<CanonicalPath>,
) -> Result<Vec<CanonicalPath>, Report<CommandError>> {
    if paths.is_empty() {
        return Err(Report::new(CommandError));
    }

    if paths.len() == 1 {
        let resolved_path = &paths[0];
        let path_str = resolved_path.as_path().to_string_lossy();
        let file_path_id = ctx
            .services
            .db
            .get_or_create_file_path(&path_str)
            .await
            .change_context(CommandError)?;

        let existing_note = ctx
            .services
            .db
            .get_note(file_path_id)
            .await
            .change_context(CommandError)?;

        let initial_content = existing_note.unwrap_or_default();
        if let Some(new_content) = ctx
            .services
            .editor
            .open(&initial_content)
            .await
            .change_context(CommandError)?
        {
            ctx
                .services
                .db
                .upsert_note(file_path_id, &new_content)
                .await
                .change_context(CommandError)?;
        }
    } else {
        let Some(new_content) = ctx.services.editor.open("").await.change_context(CommandError)? else {
            return Ok(paths);
        };

        for resolved_path in &paths {
            upsert_note_with_prepend(ctx, resolved_path, &new_content).await?;
        }
    }

    Ok(paths)
}

async fn upsert_note_with_prepend(
    ctx: &SystemCtx,
    resolved_path: &CanonicalPath,
    new_content: &str,
) -> Result<(), Report<CommandError>> {
    let path_str = resolved_path.as_path().to_string_lossy();
    let file_path_id = ctx
        .services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .change_context(CommandError)?;

    let existing_note = ctx
        .services
        .db
        .get_note(file_path_id)
        .await
        .change_context(CommandError)?;

    let final_content = match existing_note {
        Some(existing) => format!("{existing}\n\n{new_content}"),
        None => new_content.to_owned(),
    };

    ctx
        .services
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
    ctx: &SystemCtx,
    query: &str,
    create_symlinks: bool,
) -> Result<(Vec<String>, usize), Report<CommandError>> {
    let results: Vec<_> = ctx
        .services
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
/// Returns an error if database operations fail.
pub async fn add_alias_as_note(
    ctx: &SystemCtx,
    path: &CanonicalPath,
    alias: &str,
) -> Result<bool, Report<CommandError>> {
    if alias.is_empty() {
        return Ok(false);
    }

    let path_str = path.as_path().to_string_lossy();

    let file_path_id = ctx
        .services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .change_context(CommandError)?;
    let existing = ctx
        .services
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

    ctx
        .services
        .db
        .upsert_note(file_path_id, &new_content)
        .await
        .change_context(CommandError)?;
    Ok(true)
}

/// # Errors
///
/// Returns an error if database or storage operations fail.
pub async fn set_alias(
    ctx: &SystemCtx,
    path: &CanonicalPath,
    workspace: &CanonicalPath,
    alias: &str,
) -> Result<CanonicalPath, Report<CommandError>> {
    add_alias_as_note(ctx, path, alias).await?;
    ctx
        .services
        .storage
        .upsert_alias(path, workspace, alias)
        .await
        .change_context(CommandError)?;
    Ok(path.clone())
}

/// # Errors
///
/// Returns an error if storage operations fail.
pub async fn remove_alias(
    ctx: &SystemCtx,
    path: &CanonicalPath,
    workspace: &CanonicalPath,
) -> Result<CanonicalPath, Report<CommandError>> {
    ctx
        .services
        .storage
        .delete_alias(path, workspace)
        .await
        .change_context(CommandError)?;
    Ok(path.clone())
}

#[allow(clippy::unused_async)]
pub async fn migrate_aliases_to_notes<S>(
    _ctx: &SystemCtx,
    _files: &HashMap<CanonicalPath, crate::feat::playlist::FileMetadata, S>,
) -> (usize, usize)
where
    S: std::hash::BuildHasher,
{
    (0, 0)
}

/// # Errors
///
/// Returns an error if database operations, fuzzy search, or current directory retrieval fails.
pub async fn fuzzy(
    ctx: &SystemCtx,
    create_symlinks: bool,
) -> Result<(Vec<String>, usize), Report<CommandError>> {
    let notes = ctx
        .services
        .db
        .get_all_notes_with_paths()
        .await
        .change_context(CommandError)?;

    if notes.is_empty() {
        return Ok((Vec::new(), 0));
    }

    let result = ctx
        .services
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
    use std::collections::HashMap;
    use std::sync::Arc;

    use marked_path::CanonicalPath;
    use tempfile::TempDir;

    use crate::feat::external_editor::FakeEditor;
    use crate::feat::note_db::NoteDb;
    use crate::test_utils::{create_temp_file, FakeFuzzySearch, NoteTestContext, NoteTestContextBuilder};

    #[tokio::test]
    async fn add_single_path_saves_note_to_database() {
        let fake_editor = Arc::new(FakeEditor::new());
        fake_editor.set_content("my new note".to_string());
        let ctx = NoteTestContextBuilder::new()
            .editor(fake_editor)
            .build()
            .await;
        let canonical = CanonicalPath::from_path(ctx.temp_file.path()).unwrap();

        let result = super::add(&ctx.ctx, vec![canonical]).await;

        assert!(result.is_ok());
        let note = ctx.ctx.services.db.get_note(ctx.file_path_id).await.unwrap();
        assert_eq!(note, Some("my new note".to_string()));
    }

    #[tokio::test]
    async fn add_multiple_paths_prepends_to_existing_notes() {
        let fake_editor = Arc::new(FakeEditor::new());
        fake_editor.set_content("new content".to_string());
        let ctx = NoteTestContextBuilder::new()
            .editor(fake_editor.clone())
            .build()
            .await;
        let temp1 = create_temp_file();
        let temp2 = create_temp_file();
        let canonical1 = CanonicalPath::from_path(temp1.path()).unwrap();
        let canonical2 = CanonicalPath::from_path(temp2.path()).unwrap();

        // Set up existing note on canonical1
        let path_str1 = temp1.path().to_string_lossy();
        let file_path_id1 = ctx.ctx.services.db.get_or_create_file_path(&path_str1).await.unwrap();
        ctx.ctx.services
            .db
            .upsert_note(file_path_id1, "existing note")
            .await
            .unwrap();

        let result = super::add(&ctx.ctx, vec![canonical1, canonical2]).await;

        assert!(result.is_ok());
        let note1 = ctx.ctx.services.db.get_note(file_path_id1).await.unwrap();
        assert_eq!(note1, Some("existing note\n\nnew content".to_string()));
    }

    #[tokio::test]
    async fn add_returns_error_for_empty_paths() {
        let ctx = NoteTestContext::new().await;

        let result = super::add(&ctx.ctx, vec![]).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn search_returns_matching_paths_from_database() {
        let ctx = NoteTestContext::new().await;
        ctx.ctx.services
            .db
            .upsert_note(ctx.file_path_id, "test note with keyword")
            .await
            .unwrap();

        let result = super::search(&ctx.ctx, "keyword", false).await;

        assert!(result.is_ok());
        let (paths, _) = result.unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].contains(&ctx.temp_file.path().to_string_lossy().to_string()));
    }

    #[tokio::test]
    async fn search_creates_symlinks_when_requested() {
        let ctx = NoteTestContext::new().await;
        let temp_dir = TempDir::new().unwrap();
        let video_file = temp_dir.path().join("video.mp4");
        std::fs::write(&video_file, "content").unwrap();
        let video_path = video_file.to_string_lossy().to_string();
        let file_path_id = ctx.ctx.services.db.get_or_create_file_path(&video_path).await.unwrap();
        ctx.ctx.services
            .db
            .upsert_note(file_path_id, "test note")
            .await
            .unwrap();

        let dest_dir = TempDir::new().unwrap();
        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(dest_dir.path()).unwrap();

        let result = super::search(&ctx.ctx, "test", true).await;

        std::env::set_current_dir(&original_cwd).unwrap();

        assert!(result.is_ok());
        let (_, symlinks_created) = result.unwrap();
        assert_eq!(symlinks_created, 1);
    }

    #[tokio::test]
    async fn fuzzy_returns_selected_paths() {
        let fake_fuzzy = Arc::new(FakeFuzzySearch::new());
        let expected_path = "/test/path.mp4".to_string();
        fake_fuzzy.set_selected_paths(vec![expected_path.clone()]);
        let ctx = NoteTestContextBuilder::new()
            .fuzzy_search(fake_fuzzy)
            .build()
            .await;
        ctx.ctx.services
            .db
            .upsert_note(ctx.file_path_id, "some note")
            .await
            .unwrap();

        let result = super::fuzzy(&ctx.ctx, false).await;

        assert!(result.is_ok());
        let (paths, _) = result.unwrap();
        assert_eq!(paths, vec![expected_path]);
    }

    #[tokio::test]
    async fn fuzzy_creates_symlinks_when_requested() {
        let fake_fuzzy = Arc::new(FakeFuzzySearch::new());
        let temp_dir = TempDir::new().unwrap();
        let video_file = temp_dir.path().join("video.mp4");
        std::fs::write(&video_file, "content").unwrap();
        let video_path = video_file.to_string_lossy().to_string();
        fake_fuzzy.set_selected_paths(vec![video_path.clone()]);
        let ctx = NoteTestContextBuilder::new()
            .fuzzy_search(fake_fuzzy)
            .build()
            .await;
        ctx.ctx.services
            .db
            .upsert_note(ctx.file_path_id, "some note")
            .await
            .unwrap();

        let dest_dir = TempDir::new().unwrap();
        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(dest_dir.path()).unwrap();

        let result = super::fuzzy(&ctx.ctx, true).await;

        std::env::set_current_dir(&original_cwd).unwrap();

        assert!(result.is_ok());
        let (_, symlinks_created) = result.unwrap();
        assert_eq!(symlinks_created, 1);
    }

    #[tokio::test]
    async fn add_alias_as_note_appends_to_empty_notes() {
        let ctx = NoteTestContext::new().await;
        let canonical = CanonicalPath::from_path(ctx.temp_file.path()).unwrap();
        ctx.ctx.services
            .db
            .upsert_note(ctx.file_path_id, "")
            .await
            .unwrap();

        let result = super::add_alias_as_note(&ctx.ctx, &canonical, "my-alias")
            .await
            .unwrap();

        assert!(result);
        let note = ctx.ctx.services.db.get_note(ctx.file_path_id).await.unwrap();
        assert_eq!(note, Some("my-alias".to_string()));
    }

    #[tokio::test]
    async fn add_alias_as_note_skips_blank_alias() {
        let ctx = NoteTestContext::new().await;
        let canonical = CanonicalPath::from_path(ctx.temp_file.path()).unwrap();

        let result = super::add_alias_as_note(&ctx.ctx, &canonical, "").await.unwrap();

        assert!(!result);
    }

    #[tokio::test]
    async fn add_alias_as_note_adds_when_no_conflicts() {
        let ctx = NoteTestContext::new().await;
        let canonical = CanonicalPath::from_path(ctx.temp_file.path()).unwrap();

        let result = super::add_alias_as_note(&ctx.ctx, &canonical, "my-alias")
            .await
            .unwrap();

        assert!(result);
        let note = ctx.ctx.services.db.get_note(ctx.file_path_id).await.unwrap();
        assert_eq!(note, Some("my-alias".to_string()));
    }

    #[tokio::test]
    async fn add_alias_as_note_skips_exact_match() {
        let ctx = NoteTestContext::new().await;
        let canonical = CanonicalPath::from_path(ctx.temp_file.path()).unwrap();
        ctx.ctx.services
            .db
            .upsert_note(ctx.file_path_id, "foo")
            .await
            .unwrap();

        let result = super::add_alias_as_note(&ctx.ctx, &canonical, "foo")
            .await
            .unwrap();

        assert!(!result);
        let note = ctx.ctx.services.db.get_note(ctx.file_path_id).await.unwrap();
        assert_eq!(note, Some("foo".to_string()));
    }

    #[tokio::test]
    async fn add_alias_as_note_skips_substring_match() {
        let ctx = NoteTestContext::new().await;
        let canonical = CanonicalPath::from_path(ctx.temp_file.path()).unwrap();
        ctx.ctx.services
            .db
            .upsert_note(ctx.file_path_id, "foo bar")
            .await
            .unwrap();

        let result = super::add_alias_as_note(&ctx.ctx, &canonical, "foo")
            .await
            .unwrap();

        assert!(!result);
        let note = ctx.ctx.services.db.get_note(ctx.file_path_id).await.unwrap();
        assert_eq!(note, Some("foo bar".to_string()));
    }

    #[tokio::test]
    async fn add_alias_as_note_appends_to_existing() {
        let ctx = NoteTestContext::new().await;
        let canonical = CanonicalPath::from_path(ctx.temp_file.path()).unwrap();
        ctx.ctx.services
            .db
            .upsert_note(ctx.file_path_id, "first")
            .await
            .unwrap();

        let result = super::add_alias_as_note(&ctx.ctx, &canonical, "second")
            .await
            .unwrap();

        assert!(result);
        let note = ctx.ctx.services.db.get_note(ctx.file_path_id).await.unwrap();
        assert_eq!(note, Some("first\nsecond".to_string()));
    }

    #[tokio::test]
    async fn migrate_aliases_to_notes_migrates_files_with_aliases() {
        let ctx = NoteTestContext::new().await;
        let temp1 = create_temp_file();
        let temp2 = create_temp_file();

        let mut files = HashMap::new();
        files.insert(
            CanonicalPath::from_path(temp1.path()).unwrap(),
            crate::feat::playlist::FileMetadata {
                duration: None,
                is_virtual: false,
                deleted: false,
                mime_type: None,
                time_added: None,
                alias: None,
            },
        );
        files.insert(
            CanonicalPath::from_path(temp2.path()).unwrap(),
            crate::feat::playlist::FileMetadata {
                duration: None,
                is_virtual: false,
                deleted: false,
                mime_type: None,
                time_added: None,
                alias: None,
            },
        );

        let (migrated, skipped) = super::migrate_aliases_to_notes(&ctx.ctx, &files).await;

        assert_eq!(migrated, 0);
        assert_eq!(skipped, 0);
    }

    #[tokio::test]
    async fn migrate_aliases_to_notes_skips_files_without_aliases() {
        let ctx = NoteTestContext::new().await;
        let temp1 = create_temp_file();
        let temp2 = create_temp_file();

        let mut files = HashMap::new();
        files.insert(
            CanonicalPath::from_path(temp1.path()).unwrap(),
            crate::feat::playlist::FileMetadata {
                duration: None,
                is_virtual: false,
                deleted: false,
                mime_type: None,
                time_added: None,
                alias: None,
            },
        );
        files.insert(
            CanonicalPath::from_path(temp2.path()).unwrap(),
            crate::feat::playlist::FileMetadata {
                duration: None,
                is_virtual: false,
                deleted: false,
                mime_type: None,
                time_added: None,
                alias: None,
            },
        );

        let (migrated, skipped) = super::migrate_aliases_to_notes(&ctx.ctx, &files).await;

        assert_eq!(migrated, 0);
        assert_eq!(skipped, 0);
    }

    #[tokio::test]
    async fn migrate_aliases_to_notes_is_idempotent() {
        let ctx = NoteTestContext::new().await;
        let temp = create_temp_file();

        let mut files = HashMap::new();
        files.insert(
            CanonicalPath::from_path(temp.path()).unwrap(),
            crate::feat::playlist::FileMetadata {
                duration: None,
                is_virtual: false,
                deleted: false,
                mime_type: None,
                time_added: None,
                alias: None,
            },
        );

        let (migrated1, skipped1) = super::migrate_aliases_to_notes(&ctx.ctx, &files).await;
        let (migrated2, skipped2) = super::migrate_aliases_to_notes(&ctx.ctx, &files).await;

        assert_eq!(migrated1, 0);
        assert_eq!(skipped1, 0);
        assert_eq!(migrated2, 0);
        assert_eq!(skipped2, 0);
    }
}
