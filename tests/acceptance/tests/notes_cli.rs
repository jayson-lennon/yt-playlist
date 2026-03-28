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

#![allow(clippy::missing_panics_doc)]

use acceptance::ShownotesWorld;
use cucumber::{World, given, then, when};
use tempfile::TempDir;

use marked_path::CanonicalPath;
use shownotes::{format_output, Command, CommandResult};

#[derive(Debug, World)]
#[world(init = Self::new_world)]
pub struct NotesWorld {
    inner: ShownotesWorld,
    output: String,
    symlink_dir: TempDir,
}

impl NotesWorld {
    fn new_world() -> Self {
        let inner = ShownotesWorld::new();
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
fn given_file_with_note(world: &mut NotesWorld, filename: String, note: String) {
    let full_path = world.inner.create_file(&filename);
    let canonical = CanonicalPath::from_path(&full_path).expect("failed to canonicalize path");

    world.inner.fake_editor.set_content(note);
    world.inner.execute(Command::NotesAdd {
        paths: vec![canonical],
    });
}

#[when(expr = r#"I add note {string} to {string}"#)]
fn when_add_note(world: &mut NotesWorld, note: String, filename: String) {
    let full_path = world.inner.resolve_path(&filename);
    let canonical = CanonicalPath::from_path(&full_path).expect("failed to canonicalize path");

    world.inner.fake_editor.set_append_mode(true);
    world.inner.fake_editor.set_content(note);

    let result = world.inner.execute(Command::NotesAdd {
        paths: vec![canonical],
    });

    world.output = format_output(&result);
}

#[when(expr = r#"I search notes for {string}"#)]
fn when_search_notes(world: &mut NotesWorld, query: String) {
    let result = world.inner.execute(Command::NotesSearch {
        query,
        create_symlinks: false,
    });

    world.output = format_output(&result);
}

#[when(expr = r#"I search notes for {string} with symlinks"#)]
fn when_search_notes_with_symlinks(world: &mut NotesWorld, query: String) {
    std::env::set_current_dir(world.symlink_dir.path()).expect("failed to change directory");

    let result = world.inner.execute(Command::NotesSearch {
        query,
        create_symlinks: true,
    });

    world.output = format_output(&result);
}

#[then(expr = r#"the file {string} has note {string}"#)]
fn then_file_has_note(world: &mut NotesWorld, filename: String, expected_note: String) {
    let result = world.inner.execute(Command::NotesSearch {
        query: expected_note.clone(),
        create_symlinks: false,
    });

    if let CommandResult::NotesSearch { paths, .. } = result {
        let full_path = world.inner.resolve_path(&filename);
        let path_str = full_path.to_string_lossy();
        assert!(
            paths.iter().any(|p| p.contains(&*path_str)),
            "expected file '{filename}' to have note '{expected_note}'"
        );
    } else {
        panic!("expected NotesSearch result");
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
