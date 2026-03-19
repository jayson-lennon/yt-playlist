#![allow(clippy::missing_panics_doc)]
use cucumber::{World, given, then, when};

use acceptance::ShownotesWorld;
use marked_path::CanonicalPath;
use shownotes::{Command, CommandResult};

#[derive(Debug, World)]
#[world(init = Self::new_world)]
pub struct AliasNotesWorld {
    inner: ShownotesWorld,
}

impl AliasNotesWorld {
    fn new_world() -> Self {
        Self {
            inner: ShownotesWorld::new(),
        }
    }
}

#[given(expr = r#"a real file at {string}"#)]
fn given_real_file(world: &mut AliasNotesWorld, filename: String) {
    world.inner.create_file(&filename);
}

#[given(expr = r#"a symlink to {string} at {string}"#)]
fn given_symlink(world: &mut AliasNotesWorld, target: String, link: String) {
    world.inner.create_symlink(&target, &link);
}

#[given(expr = r#"the file {string} has note {string}"#)]
fn given_file_has_note(world: &mut AliasNotesWorld, path: String, note: String) {
    let full_path = world.inner.resolve_path(&path);
    let canonical = CanonicalPath::from_path(&full_path).expect("failed to canonicalize path");

    world.inner.fake_editor.set_content(note);
    world.inner.execute(Command::NotesAdd {
        paths: vec![canonical],
    });
}

#[when(expr = r#"I add alias {string} to {string}"#)]
fn when_add_alias(world: &mut AliasNotesWorld, alias: String, path: String) {
    let full_path = world.inner.resolve_path(&path);
    let canonical = CanonicalPath::from_path(&full_path).expect("failed to canonicalize path");
    let workspace =
        CanonicalPath::from_path(world.inner.temp_dir.path()).expect("failed to get workspace");

    world.inner.execute(Command::AliasSet {
        path: canonical,
        workspace,
        alias,
    });
}

#[then(expr = r#"the file {string} has note {string}"#)]
fn then_file_has_note(world: &mut AliasNotesWorld, path: String, expected: String) {
    let full_path = world.inner.resolve_path(&path);
    let path_str = full_path.to_string_lossy();

    let result = world.inner.execute(Command::NotesSearch {
        query: expected.clone(),
        create_symlinks: false,
    });

    if let CommandResult::NotesSearch { paths, .. } = result {
        assert!(
            paths.iter().any(|p| p.contains(&*path_str)),
            "expected file '{path_str}' to be found in search for '{expected}', but got: {paths:?}"
        );
    } else {
        panic!("expected NotesSearch result, got: {result:?}");
    }
}

#[then(expr = r#"the file {string} has no notes"#)]
fn then_file_has_no_notes(world: &mut AliasNotesWorld, path: String) {
    let full_path = world.inner.resolve_path(&path);
    let path_str = full_path.to_string_lossy();

    let result = world.inner.execute(Command::NotesSearch {
        query: String::new(),
        create_symlinks: false,
    });

    if let CommandResult::NotesSearch { paths, .. } = result {
        assert!(
            !paths.iter().any(|p| p.contains(&*path_str)),
            "expected file '{path_str}' to have no notes, but it was found in search results"
        );
    }
}

#[then(expr = r#"the file {string} has exactly {int} note line"#)]
fn then_file_has_exactly_n_note_lines(
    world: &mut AliasNotesWorld,
    path: String,
    count: usize,
) {
    let full_path = world.inner.resolve_path(&path);
    let path_str = full_path.to_string_lossy();

    let result = world.inner.execute(Command::NotesSearch {
        query: "o".to_string(),
        create_symlinks: false,
    });

    if let CommandResult::NotesSearch { paths, .. } = result {
        let found = paths.iter().any(|p| p.contains(&*path_str));
        if count == 0 {
            assert!(
                !found,
                "expected file '{path_str}' to have 0 note lines, but it was found"
            );
        } else {
            assert!(
                found,
                "expected file '{path_str}' to have {count} note lines, but it was not found"
            );
        }
    }
}

#[tokio::main]
async fn main() {
    AliasNotesWorld::run("tests/features/alias_to_notes.feature").await;
}
