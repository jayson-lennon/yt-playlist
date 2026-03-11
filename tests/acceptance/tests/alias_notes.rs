use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use cucumber::{World, given, then, when};
use tempfile::TempDir;

use shownotes::command::notes::add_alias_as_note;
use shownotes::feat::external_editor::{ExternalEditorService, FakeEditor};
use shownotes::services::Services;
use shownotes::NoteDb;
use shownotes::PathResolver;

#[derive(Debug, World)]
#[world(init = Self::new_world)]
pub struct AliasNotesWorld {
    services: Services,
    temp_dir: TempDir,
    file_paths: HashMap<String, PathBuf>,
}

impl AliasNotesWorld {
    async fn new_world() -> Self {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let db_path_str = db_path.to_string_lossy().to_string();

        let rt = tokio::runtime::Handle::current();
        let services = Services::new(&db_path_str, rt)
            .await
            .expect("failed to create services");

        let fake_editor = Arc::new(FakeEditor::new());
        let services = Services {
            editor: ExternalEditorService::new(fake_editor.clone()),
            ..services
        };

        Self {
            services,
            temp_dir,
            file_paths: HashMap::new(),
        }
    }

    fn resolve_path(&self, relative: &str) -> PathBuf {
        self.file_paths
            .get(relative)
            .cloned()
            .unwrap_or_else(|| self.temp_dir.path().join(relative))
    }
}

#[given(expr = r#"a real file at {string}"#)]
fn given_real_file(world: &mut AliasNotesWorld, filename: String) {
    let full_path = world.temp_dir.path().join(&filename);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).expect("failed to create parent dir");
    }
    std::fs::File::create(&full_path).expect("failed to create file");
    world.file_paths.insert(filename, full_path);
}

#[given(expr = r#"a symlink to {string} at {string}"#)]
fn given_symlink(world: &mut AliasNotesWorld, target: String, link: String) {
    let target_path = world.resolve_path(&target);
    let link_path = world.temp_dir.path().join(&link);
    if let Some(parent) = link_path.parent() {
        std::fs::create_dir_all(parent).expect("failed to create parent dir");
    }
    std::os::unix::fs::symlink(&target_path, &link_path).expect("failed to create symlink");
    world.file_paths.insert(link, link_path);
}

#[given(expr = r#"the file {string} has note {string}"#)]
async fn given_file_has_note(world: &mut AliasNotesWorld, path: String, note: String) {
    let full_path = world.resolve_path(&path);
    let resolved = world
        .services
        .path_resolver
        .resolve(&full_path)
        .await
        .expect("failed to resolve path");
    let path_str = resolved.to_string_lossy().to_string();

    let file_path_id = world
        .services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .expect("failed to get or create file path");

    let existing = world
        .services
        .db
        .get_note(file_path_id)
        .await
        .expect("failed to get note");

    let new_content = match existing {
        Some(notes) if !notes.is_empty() => format!("{notes}\n{note}"),
        _ => note,
    };

    world
        .services
        .db
        .upsert_note(file_path_id, &new_content)
        .await
        .expect("failed to upsert note");
}

#[when(expr = r#"I add alias {string} to {string}"#)]
async fn when_add_alias(world: &mut AliasNotesWorld, alias: String, path: String) {
    let full_path = world.resolve_path(&path);
    add_alias_as_note(&world.services, &full_path, &alias)
        .await
        .expect("add_alias_as_note failed");
}

#[then(expr = r#"the file {string} has note {string}"#)]
async fn then_file_has_note(world: &mut AliasNotesWorld, path: String, expected: String) {
    let full_path = world.resolve_path(&path);
    let resolved = world
        .services
        .path_resolver
        .resolve(&full_path)
        .await
        .expect("failed to resolve path");
    let path_str = resolved.to_string_lossy().to_string();

    let file_path_id = world
        .services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .expect("failed to get or create file path");

    let note = world
        .services
        .db
        .get_note(file_path_id)
        .await
        .expect("failed to get note");

    match note {
        Some(content) => {
            assert!(
                content.contains(&expected),
                "expected note to contain '{}', but got: '{}'",
                expected,
                content
            );
        }
        None => {
            panic!("expected file '{}' to have note '{}', but no note exists", path, expected);
        }
    }
}

#[then(expr = r#"the file {string} has no notes"#)]
async fn then_file_has_no_notes(world: &mut AliasNotesWorld, path: String) {
    let full_path = world.resolve_path(&path);
    let resolved = world
        .services
        .path_resolver
        .resolve(&full_path)
        .await
        .expect("failed to resolve path");
    let path_str = resolved.to_string_lossy().to_string();

    let file_path_id = world
        .services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .expect("failed to get or create file path");

    let note = world
        .services
        .db
        .get_note(file_path_id)
        .await
        .expect("failed to get note");

    assert!(
        note.is_none() || note.as_ref().map_or(true, |n| n.is_empty()),
        "expected file '{}' to have no notes, but found: '{:?}'",
        path,
        note
    );
}

#[then(expr = r#"the file {string} has exactly {int} note line"#)]
async fn then_file_has_exactly_n_note_lines(world: &mut AliasNotesWorld, path: String, count: usize) {
    let full_path = world.resolve_path(&path);
    let resolved = world
        .services
        .path_resolver
        .resolve(&full_path)
        .await
        .expect("failed to resolve path");
    let path_str = resolved.to_string_lossy().to_string();

    let file_path_id = world
        .services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .expect("failed to get or create file path");

    let note = world
        .services
        .db
        .get_note(file_path_id)
        .await
        .expect("failed to get note");

    let line_count = match note {
        Some(content) if !content.is_empty() => content.lines().count(),
        _ => 0,
    };

    assert_eq!(
        line_count, count,
        "expected file '{}' to have exactly {} note lines, but found {}",
        path, count, line_count
    );
}

#[tokio::main]
async fn main() {
    AliasNotesWorld::run("tests/features/alias_to_notes.feature").await;
}
