#![allow(clippy::missing_panics_doc)]

use acceptance::ShownotesWorld;
use cucumber::{World, given, then, when};
use tempfile::TempDir;

use marked_path::CanonicalPath;
use shownotes::command::{Command, execute, format_output};
use shownotes::feat::note_db::NoteDb;

#[derive(Debug, World)]
#[world(init = Self::new_world)]
pub struct NotesWorld {
    inner: ShownotesWorld,
    output: String,
    symlink_dir: TempDir,
}

impl NotesWorld {
    async fn new_world() -> Self {
        let inner = ShownotesWorld::new().await;
        let symlink_dir = tempfile::tempdir().expect("failed to create symlink dir");
        Self {
            inner,
            output: String::new(),
            symlink_dir,
        }
    }
}

#[given(expr = r#"a file {string}"#)]
fn given_file(world: &mut NotesWorld, filename: String) {
    world.inner.create_file(&filename);
}

#[given(expr = r#"a file {string} with note {string}"#)]
async fn given_file_with_note(world: &mut NotesWorld, filename: String, note: String) {
    let full_path = world.inner.create_file(&filename);
    let canonical = CanonicalPath::from_path(&full_path).expect("failed to canonicalize path");
    let path_str = canonical.as_path().to_string_lossy().to_string();

    let file_path_id = world
        .inner
        .ctx
        .services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .expect("failed to get file path id");

    world
        .inner
        .ctx
        .services
        .db
        .upsert_note(file_path_id, &note)
        .await
        .expect("failed to upsert note");
}

#[when(expr = r#"I add note {string} to {string}"#)]
async fn when_add_note(world: &mut NotesWorld, note: String, filename: String) {
    let full_path = world.inner.resolve_path(&filename);
    let canonical = CanonicalPath::from_path(&full_path).expect("failed to canonicalize path");
    let path_str = canonical.as_path().to_string_lossy().to_string();

    let file_path_id = world
        .inner
        .ctx
        .services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .expect("failed to get file path id");

    let existing_note = world
        .inner
        .ctx
        .services
        .db
        .get_note(file_path_id)
        .await
        .expect("failed to get existing note");

    let combined_content = match existing_note {
        Some(existing) => format!("{existing}\n\n{note}"),
        None => note,
    };

    world.inner.fake_editor.set_content(combined_content);

    let result = execute(
        &world.inner.ctx,
        Command::NotesAdd {
            paths: vec![canonical],
        },
    )
    .await
    .expect("add note command failed");

    world.output = format_output(&result);
}

#[when(expr = r#"I search notes for {string}"#)]
async fn when_search_notes(world: &mut NotesWorld, query: String) {
    let result = execute(
        &world.inner.ctx,
        Command::NotesSearch {
            query,
            create_symlinks: false,
        },
    )
    .await
    .expect("search notes command failed");

    world.output = format_output(&result);
}

#[when(expr = r#"I search notes for {string} with symlinks"#)]
async fn when_search_notes_with_symlinks(world: &mut NotesWorld, query: String) {
    std::env::set_current_dir(world.symlink_dir.path()).expect("failed to change directory");

    let result = execute(
        &world.inner.ctx,
        Command::NotesSearch {
            query,
            create_symlinks: true,
        },
    )
    .await
    .expect("search notes command failed");

    world.output = format_output(&result);
}

#[then(expr = r#"the file {string} has note {string}"#)]
async fn then_file_has_note(world: &mut NotesWorld, filename: String, expected_note: String) {
    let full_path = world.inner.resolve_path(&filename);
    let canonical = CanonicalPath::from_path(&full_path).expect("failed to canonicalize path");
    let path_str = canonical.as_path().to_string_lossy().to_string();

    let file_path_id = world
        .inner
        .ctx
        .services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .expect("failed to get file path id");

    let note = world
        .inner
        .ctx
        .services
        .db
        .get_note(file_path_id)
        .await
        .expect("failed to get note");

    match note {
        Some(content) => {
            assert!(
                content.contains(&expected_note),
                "expected file '{filename}' to have note '{expected_note}', but got: '{content}'"
            );
        }
        None => panic!("expected file '{filename}' to have note '{expected_note}', but no note found"),
    }
}

#[then(expr = r#"the output contains {string}"#)]
fn then_output_contains(world: &mut NotesWorld, expected: String) {
    assert!(
        world.output.contains(&expected),
        "expected output to contain '{}', but got: '{}'",
        expected,
        world.output
    );
}

#[then(expr = r#"the output does not contain {string}"#)]
fn then_output_does_not_contain(world: &mut NotesWorld, expected: String) {
    assert!(
        !world.output.contains(&expected),
        "expected output to NOT contain '{}', but got: '{}'",
        expected,
        world.output
    );
}

#[then(expr = r#"the output is empty"#)]
fn then_output_is_empty(world: &mut NotesWorld) {
    assert!(
        world.output.is_empty(),
        "expected output to be empty, but got: '{}'",
        world.output
    );
}

#[then(expr = r#"a symlink to {string} exists in current directory"#)]
fn then_symlink_exists(world: &mut NotesWorld, filename: String) {
    let symlink_path = world.symlink_dir.path().join(&filename);

    let metadata = std::fs::symlink_metadata(&symlink_path)
        .unwrap_or_else(|_| panic!("expected symlink '{}' to exist", symlink_path.display()));

    assert!(
        metadata.file_type().is_symlink(),
        "expected '{}' to be a symlink",
        symlink_path.display()
    );

    let full_path = world.inner.resolve_path(&filename);
    let symlink_target = symlink_path
        .canonicalize()
        .expect("failed to canonicalize symlink");
    let expected_target = full_path
        .canonicalize()
        .expect("failed to canonicalize expected target");

    assert_eq!(
        symlink_target,
        expected_target,
        "symlink target mismatch"
    );
}

#[tokio::main]
async fn main() {
    NotesWorld::run("tests/features/notes_cli.feature").await;
}
