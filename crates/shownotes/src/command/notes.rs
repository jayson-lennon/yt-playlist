// Copyright (C) 2026 Jayson Lennon
// 
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// 
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
// 
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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
///
/// This function searches both notes and filenames for matching items.
pub async fn fuzzy(
    ctx: &SystemCtx,
    create_symlinks: bool,
) -> Result<(Vec<String>, usize), Report<CommandError>> {
    let items = ctx
        .services
        .db
        .get_all_paths_for_fuzzy_search()
        .await
        .change_context(CommandError)?;

    if items.is_empty() {
        return Ok((Vec::new(), 0));
    }

    let result = ctx
        .services
        .fuzzy_search
        .search(&items)
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
        // Given a context with a fake editor configured with content.
        let fake_editor = Arc::new(FakeEditor::new());
        fake_editor.set_content("my new note".to_string());
        let ctx = NoteTestContextBuilder::new()
            .editor(fake_editor)
            .build()
            .await;
        let canonical = CanonicalPath::from_path(ctx.temp_file.path()).unwrap();

        // When adding a note for a single path.
        let result = super::add(&ctx.ctx, vec![canonical]).await;

        // Then the note is saved to the database.
        assert!(result.is_ok());
        let note = ctx.ctx.services.db.get_note(ctx.file_path_id).await.unwrap();
        assert_eq!(note, Some("my new note".to_string()));
    }

    #[tokio::test]
    async fn add_multiple_paths_prepends_to_existing_notes() {
        // Given a context with a fake editor and an existing note on one file.
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

        let path_str1 = temp1.path().to_string_lossy();
        let file_path_id1 = ctx.ctx.services.db.get_or_create_file_path(&path_str1).await.unwrap();
        ctx.ctx.services
            .db
            .upsert_note(file_path_id1, "existing note")
            .await
            .unwrap();

        // When adding notes for multiple paths.
        let result = super::add(&ctx.ctx, vec![canonical1, canonical2]).await;

        // Then the new content is prepended to the existing note.
        assert!(result.is_ok());
        let note1 = ctx.ctx.services.db.get_note(file_path_id1).await.unwrap();
        assert_eq!(note1, Some("existing note\n\nnew content".to_string()));
    }

    #[tokio::test]
    async fn add_returns_error_for_empty_paths() {
        // Given a test context.
        let ctx = NoteTestContext::new().await;

        // When adding notes with an empty path list.
        let result = super::add(&ctx.ctx, vec![]).await;

        // Then an error is returned.
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn search_returns_matching_paths_from_database() {
        // Given a context with a note containing a keyword.
        let ctx = NoteTestContext::new().await;
        ctx.ctx.services
            .db
            .upsert_note(ctx.file_path_id, "test note with keyword")
            .await
            .unwrap();

        // When searching for the keyword.
        let result = super::search(&ctx.ctx, "keyword", false).await;

        // Then the matching path is returned.
        assert!(result.is_ok());
        let (paths, _) = result.unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].contains(&ctx.temp_file.path().to_string_lossy().to_string()));
    }

    #[tokio::test]
    async fn search_creates_symlinks_when_requested() {
        // Given a context with a video file that has a note.
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

        // When searching with symlink creation enabled.
        let result = super::search(&ctx.ctx, "test", true).await;

        std::env::set_current_dir(&original_cwd).unwrap();

        // Then a symlink is created.
        assert!(result.is_ok());
        let (_, symlinks_created) = result.unwrap();
        assert_eq!(symlinks_created, 1);
    }

    #[tokio::test]
    async fn fuzzy_returns_selected_paths() {
        // Given a context with a fake fuzzy search configured with selected paths.
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

        // When running fuzzy search without symlink creation.
        let result = super::fuzzy(&ctx.ctx, false).await;

        // Then the selected paths are returned.
        assert!(result.is_ok());
        let (paths, _) = result.unwrap();
        assert_eq!(paths, vec![expected_path]);
    }

    #[tokio::test]
    async fn fuzzy_creates_symlinks_when_requested() {
        // Given a context with a fake fuzzy search and a video file with a note.
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

        // When running fuzzy search with symlink creation enabled.
        let result = super::fuzzy(&ctx.ctx, true).await;

        std::env::set_current_dir(&original_cwd).unwrap();

        // Then a symlink is created.
        assert!(result.is_ok());
        let (_, symlinks_created) = result.unwrap();
        assert_eq!(symlinks_created, 1);
    }

    #[tokio::test]
    async fn add_alias_as_note_appends_to_empty_notes() {
        // Given a context with a file that has an empty note.
        let ctx = NoteTestContext::new().await;
        let canonical = CanonicalPath::from_path(ctx.temp_file.path()).unwrap();
        ctx.ctx.services
            .db
            .upsert_note(ctx.file_path_id, "")
            .await
            .unwrap();

        // When adding an alias as a note.
        let result = super::add_alias_as_note(&ctx.ctx, &canonical, "my-alias")
            .await
            .unwrap();

        // Then the alias is saved as the note.
        assert!(result);
        let note = ctx.ctx.services.db.get_note(ctx.file_path_id).await.unwrap();
        assert_eq!(note, Some("my-alias".to_string()));
    }

    #[tokio::test]
    async fn add_alias_as_note_skips_blank_alias() {
        // Given a context with a file.
        let ctx = NoteTestContext::new().await;
        let canonical = CanonicalPath::from_path(ctx.temp_file.path()).unwrap();

        // When adding a blank alias as a note.
        let result = super::add_alias_as_note(&ctx.ctx, &canonical, "").await.unwrap();

        // Then the operation is skipped.
        assert!(!result);
    }

    #[tokio::test]
    async fn add_alias_as_note_adds_when_no_conflicts() {
        // Given a context with a file that has no existing note.
        let ctx = NoteTestContext::new().await;
        let canonical = CanonicalPath::from_path(ctx.temp_file.path()).unwrap();

        // When adding an alias as a note.
        let result = super::add_alias_as_note(&ctx.ctx, &canonical, "my-alias")
            .await
            .unwrap();

        // Then the alias is saved as the note.
        assert!(result);
        let note = ctx.ctx.services.db.get_note(ctx.file_path_id).await.unwrap();
        assert_eq!(note, Some("my-alias".to_string()));
    }

    #[tokio::test]
    async fn add_alias_as_note_skips_exact_match() {
        // Given a context with a file that has a note matching the alias.
        let ctx = NoteTestContext::new().await;
        let canonical = CanonicalPath::from_path(ctx.temp_file.path()).unwrap();
        ctx.ctx.services
            .db
            .upsert_note(ctx.file_path_id, "foo")
            .await
            .unwrap();

        // When adding an alias that exactly matches the existing note.
        let result = super::add_alias_as_note(&ctx.ctx, &canonical, "foo")
            .await
            .unwrap();

        // Then the operation is skipped and the note is unchanged.
        assert!(!result);
        let note = ctx.ctx.services.db.get_note(ctx.file_path_id).await.unwrap();
        assert_eq!(note, Some("foo".to_string()));
    }

    #[tokio::test]
    async fn add_alias_as_note_skips_substring_match() {
        // Given a context with a file that has a note containing the alias as a substring.
        let ctx = NoteTestContext::new().await;
        let canonical = CanonicalPath::from_path(ctx.temp_file.path()).unwrap();
        ctx.ctx.services
            .db
            .upsert_note(ctx.file_path_id, "foo bar")
            .await
            .unwrap();

        // When adding an alias that is a substring of the existing note.
        let result = super::add_alias_as_note(&ctx.ctx, &canonical, "foo")
            .await
            .unwrap();

        // Then the operation is skipped and the note is unchanged.
        assert!(!result);
        let note = ctx.ctx.services.db.get_note(ctx.file_path_id).await.unwrap();
        assert_eq!(note, Some("foo bar".to_string()));
    }

    #[tokio::test]
    async fn add_alias_as_note_appends_to_existing() {
        // Given a context with a file that has an existing note.
        let ctx = NoteTestContext::new().await;
        let canonical = CanonicalPath::from_path(ctx.temp_file.path()).unwrap();
        ctx.ctx.services
            .db
            .upsert_note(ctx.file_path_id, "first")
            .await
            .unwrap();

        // When adding a new alias as a note.
        let result = super::add_alias_as_note(&ctx.ctx, &canonical, "second")
            .await
            .unwrap();

        // Then the alias is appended to the existing note.
        assert!(result);
        let note = ctx.ctx.services.db.get_note(ctx.file_path_id).await.unwrap();
        assert_eq!(note, Some("first\nsecond".to_string()));
    }

    #[tokio::test]
    async fn migrate_aliases_to_notes_migrates_files_with_aliases() {
        // Given a context with files that have no aliases.
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

        // When migrating aliases to notes.
        let (migrated, skipped) = super::migrate_aliases_to_notes(&ctx.ctx, &files).await;

        // Then no files are migrated or skipped.
        assert_eq!(migrated, 0);
        assert_eq!(skipped, 0);
    }

    #[tokio::test]
    async fn migrate_aliases_to_notes_skips_files_without_aliases() {
        // Given a context with files that have no aliases.
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

        // When migrating aliases to notes.
        let (migrated, skipped) = super::migrate_aliases_to_notes(&ctx.ctx, &files).await;

        // Then no files are migrated or skipped.
        assert_eq!(migrated, 0);
        assert_eq!(skipped, 0);
    }

    #[tokio::test]
    async fn migrate_aliases_to_notes_is_idempotent() {
        // Given a context with a file that has no alias.
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

        // When migrating aliases to notes twice.
        let (migrated1, skipped1) = super::migrate_aliases_to_notes(&ctx.ctx, &files).await;
        let (migrated2, skipped2) = super::migrate_aliases_to_notes(&ctx.ctx, &files).await;

        // Then both calls produce the same result.
        assert_eq!(migrated1, 0);
        assert_eq!(skipped1, 0);
        assert_eq!(migrated2, 0);
        assert_eq!(skipped2, 0);
    }
}
