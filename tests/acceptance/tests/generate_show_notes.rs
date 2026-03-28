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
use marked_path::CanonicalPath;
use shownotes::command::{Command, CommandResult, format_output};
use shownotes::common::domain::{ItemPath, PlaylistItem};

#[derive(Debug, World)]
#[world(init = Self::new_world)]
pub struct GenerateWorld {
    inner: ShownotesWorld,
    output: String,
    last_result: Option<CommandResult>,
}

impl GenerateWorld {
    fn new_world() -> Self {
        Self {
            inner: ShownotesWorld::new(),
            output: String::new(),
            last_result: None,
        }
    }
}

#[given(expr = r#"a file {string} with source {string}"#)]
fn given_file_with_source(world: &mut GenerateWorld, filename: String, url: String) {
    let full_path = world.inner.create_file(&filename);
    let canonical = CanonicalPath::from_path(&full_path).expect("failed to canonicalize path");

    world.inner.execute(Command::SourcesAdd {
        path: canonical.clone(),
        url,
    });

    let item = PlaylistItem {
        path: ItemPath::File(canonical),
        duration: None,
        alias: None,
        mime_type: None,
        is_virtual: false,
        playlist_count: 0,
        has_sources: true,
    };

    let existing = world.inner.execute(Command::PlaylistLoad);
    let mut playlist_items = match existing {
        CommandResult::PlaylistLoaded { playlist_items, .. } => playlist_items,
        _ => vec![],
    };

    playlist_items.push(item);

    world.inner.execute(Command::PlaylistSave {
        playlist_items,
        library_items: vec![],
    });
}

#[given(expr = r#"a file {string} exists"#)]
fn given_file_exists(world: &mut GenerateWorld, filename: String) {
    let full_path = world.inner.create_file(&filename);
    let canonical = CanonicalPath::from_path(&full_path).expect("failed to canonicalize path");

    let item = PlaylistItem {
        path: ItemPath::File(canonical),
        duration: None,
        alias: None,
        mime_type: None,
        is_virtual: false,
        playlist_count: 0,
        has_sources: true,
    };

    let existing = world.inner.execute(Command::PlaylistLoad);
    let mut playlist_items = match existing {
        CommandResult::PlaylistLoaded { playlist_items, .. } => playlist_items,
        _ => vec![],
    };

    playlist_items.push(item);

    world.inner.execute(Command::PlaylistSave {
        playlist_items,
        library_items: vec![],
    });
}

#[given(expr = r#"no files in playlist"#)]
fn given_no_files_in_playlist(world: &mut GenerateWorld) {
    world.inner.execute(Command::PlaylistSave {
        playlist_items: vec![],
        library_items: vec![],
    });
}

#[when(expr = r#"I generate show notes in {string} format"#)]
fn when_generate_show_notes(world: &mut GenerateWorld, format: String) {
    let library_path = world.inner.app.as_ref().unwrap().ctx.library_path.clone();
    let result = world.inner.execute(Command::GenerateNotes {
        format,
        working_directory: library_path,
    });

    world.output = format_output(&result);
    world.last_result = Some(result);
}

#[then(expr = r#"the output contains {string}"#)]
fn then_output_contains(world: &mut GenerateWorld, expected: String) {
    assert!(
        world.output.contains(&expected),
        "expected output to contain '{}', but got: '{}'",
        expected,
        world.output
    );
}

#[then(expr = r#"the output does not contain {string}"#)]
fn then_output_does_not_contain(world: &mut GenerateWorld, expected: String) {
    assert!(
        !world.output.contains(&expected),
        "expected output to NOT contain '{}', but got: '{}'",
        expected,
        world.output
    );
}

#[then(expr = r#"the output is empty"#)]
fn then_output_is_empty(world: &mut GenerateWorld) {
    assert!(
        world.output.is_empty(),
        "expected output to be empty, but got: '{}'",
        world.output
    );
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    GenerateWorld::run("tests/features/generate_show_notes.feature").await;
}
