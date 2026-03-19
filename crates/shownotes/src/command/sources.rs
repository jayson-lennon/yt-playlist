use error_stack::{Report, ResultExt};

use marked_path::CanonicalPath;

use crate::system_ctx::SystemCtx;
use crate::feat::sources::SourceDb;
use crate::feat::note_db::NoteDb;
use crate::feat::external_editor::ExternalEditor;

use super::CommandError;

pub async fn add(
    ctx: &SystemCtx,
    path: &CanonicalPath,
    url: &str,
) -> Result<(), Report<CommandError>> {
    let path_str = path.as_path().to_string_lossy();
    let file_path_id = ctx
        .services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .change_context(CommandError)?;

    let mut existing = ctx
        .services
        .sources
        .get_sources(file_path_id)
        .await
        .change_context(CommandError)?
        .into_iter()
        .map(|s| s.source_url)
        .collect::<Vec<_>>();
    existing.push(url.to_string());

    ctx
        .services
        .sources
        .set_sources(file_path_id, &existing)
        .await
        .change_context(CommandError)?;

    Ok(())
}

pub async fn list(
    ctx: &SystemCtx,
    path: &CanonicalPath,
) -> Result<Vec<String>, Report<CommandError>> {
    let path_str = path.as_path().to_string_lossy();
    let file_path_id = ctx
        .services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .change_context(CommandError)?;

    let sources = ctx
        .services
        .sources
        .get_sources(file_path_id)
        .await
        .change_context(CommandError)?;

    let urls = sources.into_iter().map(|s| s.source_url).collect();
    Ok(urls)
}

pub async fn edit(
    ctx: &SystemCtx,
    path: &CanonicalPath,
) -> Result<(), Report<CommandError>> {
    let path_str = path.as_path().to_string_lossy();
    let file_path_id = ctx
        .services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .change_context(CommandError)?;

    let existing = ctx
        .services
        .sources
        .get_sources(file_path_id)
        .await
        .change_context(CommandError)?;

    let initial_content = existing
        .iter()
        .map(|s| s.source_url.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    if let Some(new_content) = ctx
        .services
        .editor
        .open(&initial_content)
        .await
        .change_context(CommandError)?
    {
        let urls: Vec<String> = new_content.lines().map(ToString::to_string).collect();
        ctx
            .services
            .sources
            .set_sources(file_path_id, &urls)
            .await
            .change_context(CommandError)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tempfile::NamedTempFile;

    use crate::feat::config::Config;
    use crate::feat::external_editor::{ExternalEditorService, FakeEditor};
    use crate::feat::fuzzy_search::{FuzzySearchService, SkimBackend};
    use crate::feat::launcher::FileLauncherService;
    use crate::feat::media_query::MediaQueryService;
    use crate::feat::mpv::{MpvClientService, MpvLauncherService};
    use crate::feat::note_db::{NoteDbService, SqliteNoteDb};
    use crate::feat::path_resolver::{PathResolverService, SystemPathResolver};
    use crate::feat::playlist::PlaylistStorageService;
    use crate::feat::sources::{SourceDbService, SqliteSourceDb};
    use crate::services::Services;
    use crate::test_utils::fakes::{
        FakeLauncher, FakeMediaBackend, FakeMpvBackend, FakeMpvLauncher, FakeStorageBackend,
    };

    use super::*;

    async fn create_test_services_with_editor(editor: Arc<FakeEditor>) -> Services {
        let rt = tokio::runtime::Handle::current();
        let db = Arc::new(SqliteNoteDb::new("sqlite::memory:").await.unwrap());
        Services {
            mpv: MpvClientService::new(Arc::new(FakeMpvBackend)),
            media: MediaQueryService::new(Arc::new(FakeMediaBackend)),
            storage: PlaylistStorageService::new(Arc::new(FakeStorageBackend)),
            mpv_launcher: MpvLauncherService::new(Arc::new(FakeMpvLauncher::new())),
            file_launcher: FileLauncherService::new(Arc::new(FakeLauncher::new())),
            db: NoteDbService::new(db.clone()),
            editor: ExternalEditorService::new(editor),
            path_resolver: PathResolverService::new(Arc::new(SystemPathResolver)),
            sources: SourceDbService::new(Arc::new(SqliteSourceDb::new(db.pool().clone()))),
            fuzzy_search: FuzzySearchService::new(Arc::new(SkimBackend)),
            rt,
        }
    }

    async fn create_test_context() -> (SystemCtx, NamedTempFile) {
        let editor = Arc::new(FakeEditor::new());
        let services = create_test_services_with_editor(editor).await;
        let temp_file = NamedTempFile::new().unwrap();
        let library_path =
            CanonicalPath::from_path(temp_file.path().parent().unwrap()).unwrap();
        let ctx = SystemCtx {
            services,
            config: Config::default(),
            library_path,
            socket_path: String::new(),
            keymap: crate::feat::keymap::Keymap::new(),
        };
        (ctx, temp_file)
    }

    async fn create_test_context_with_editor(
        editor: Arc<FakeEditor>,
    ) -> (SystemCtx, NamedTempFile) {
        let services = create_test_services_with_editor(editor).await;
        let temp_file = NamedTempFile::new().unwrap();
        let library_path =
            CanonicalPath::from_path(temp_file.path().parent().unwrap()).unwrap();
        let ctx = SystemCtx {
            services,
            config: Config::default(),
            library_path,
            socket_path: String::new(),
            keymap: crate::feat::keymap::Keymap::new(),
        };
        (ctx, temp_file)
    }

    #[tokio::test]
    async fn add_saves_source_to_database() {
        // Given a context with no sources.
        let (ctx, temp_file) = create_test_context().await;
        let path = CanonicalPath::from_path(temp_file.path()).unwrap();

        // When adding a source URL.
        add(&ctx, &path, "https://example.com/source").await.unwrap();

        // Then the source can be retrieved via list.
        let sources = list(&ctx, &path).await.unwrap();
        assert_eq!(sources, vec!["https://example.com/source"]);
    }

    #[tokio::test]
    async fn add_appends_to_existing_sources() {
        // Given a context with an existing source.
        let (ctx, temp_file) = create_test_context().await;
        let path = CanonicalPath::from_path(temp_file.path()).unwrap();
        add(&ctx, &path, "https://example.com/first").await.unwrap();

        // When adding another source URL.
        add(&ctx, &path, "https://example.com/second").await.unwrap();

        // Then both sources are returned in order.
        let sources = list(&ctx, &path).await.unwrap();
        assert_eq!(
            sources,
            vec!["https://example.com/first", "https://example.com/second"]
        );
    }

    #[tokio::test]
    async fn list_returns_all_sources_for_path() {
        // Given a context with multiple sources.
        let (ctx, temp_file) = create_test_context().await;
        let path = CanonicalPath::from_path(temp_file.path()).unwrap();
        add(&ctx, &path, "https://example.com/one").await.unwrap();
        add(&ctx, &path, "https://example.com/two").await.unwrap();
        add(&ctx, &path, "https://example.com/three").await.unwrap();

        // When listing sources.
        let sources = list(&ctx, &path).await.unwrap();

        // Then all sources are returned in order.
        assert_eq!(
            sources,
            vec![
                "https://example.com/one",
                "https://example.com/two",
                "https://example.com/three"
            ]
        );
    }

    #[tokio::test]
    async fn list_returns_empty_for_path_with_no_sources() {
        // Given a context with no sources added.
        let (ctx, temp_file) = create_test_context().await;
        let path = CanonicalPath::from_path(temp_file.path()).unwrap();

        // When listing sources.
        let sources = list(&ctx, &path).await.unwrap();

        // Then an empty vector is returned.
        assert!(sources.is_empty());
    }

    #[tokio::test]
    async fn edit_updates_sources_from_editor_content() {
        // Given a context with existing sources and an editor that returns new content.
        let editor = Arc::new(FakeEditor::new());
        editor.set_content("https://edited.com/one\nhttps://edited.com/two".to_string());
        let (ctx, temp_file) = create_test_context_with_editor(editor).await;
        let path = CanonicalPath::from_path(temp_file.path()).unwrap();
        add(&ctx, &path, "https://original.com/old").await.unwrap();

        // When editing sources.
        edit(&ctx, &path).await.unwrap();

        // Then the sources are updated from editor content.
        let sources = list(&ctx, &path).await.unwrap();
        assert_eq!(
            sources,
            vec!["https://edited.com/one", "https://edited.com/two"]
        );
    }

    #[tokio::test]
    async fn edit_unchanged_when_editor_returns_none() {
        // Given a context with existing sources and an editor that returns None.
        let editor = Arc::new(FakeEditor::new());
        let (ctx, temp_file) = create_test_context_with_editor(editor).await;
        let path = CanonicalPath::from_path(temp_file.path()).unwrap();
        add(&ctx, &path, "https://original.com/source").await.unwrap();

        // When editing sources (editor returns None).
        edit(&ctx, &path).await.unwrap();

        // Then the sources remain unchanged.
        let sources = list(&ctx, &path).await.unwrap();
        assert_eq!(sources, vec!["https://original.com/source"]);
    }
}
