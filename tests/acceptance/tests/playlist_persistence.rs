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
use shownotes::command::{Command, CommandResult};
use shownotes::common::domain::{ItemPath, PlaylistItem};

#[derive(Debug, World)]
#[world(init = Self::new_world)]
pub struct PlaylistWorld {
    inner: ShownotesWorld,
    playlist_items: Vec<PlaylistItem>,
    library_items: Vec<PlaylistItem>,
}

impl PlaylistWorld {
    fn new_world() -> Self {
        Self {
            inner: ShownotesWorld::new(),
            playlist_items: Vec::new(),
            library_items: Vec::new(),
        }
    }
}

fn parse_file_list(files: &str) -> Vec<String> {
    files
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .collect()
}

fn create_playlist_item(world: &mut PlaylistWorld, filename: &str) -> PlaylistItem {
    let full_path = world.inner.create_file(filename);
    let canonical = CanonicalPath::from_path(&full_path).expect("failed to canonicalize path");
    PlaylistItem {
        path: ItemPath::File(canonical),
        duration: None,
        alias: None,
        mime_type: None,
        is_virtual: false,
        playlist_count: 0,
        has_sources: true,
    }
}

#[given(regex = r#"a playlist with files (.*)"#)]
fn given_playlist_with_files(world: &mut PlaylistWorld, files: String) {
    let file_names = parse_file_list(&files);
    world.playlist_items = file_names
        .iter()
        .map(|name| create_playlist_item(world, name))
        .collect();
}

#[given(expr = r#"a file {string} in playlist with alias {string}"#)]
fn given_file_with_alias(world: &mut PlaylistWorld, filename: String, alias: String) {
    let item = create_playlist_item(world, &filename);
    let path = match &item.path {
        ItemPath::File(canonical) => canonical.clone(),
        ItemPath::Url(_) => panic!("URL items not supported for alias"),
    };
    world.playlist_items.push(item);

    let workspace = world.inner.app.as_ref().unwrap().ctx.library_path.clone();
    let result = world.inner.execute(Command::AliasSet {
        path,
        workspace,
        alias,
    });

    match result {
        CommandResult::AliasSet { .. } => {}
        _ => panic!("Unexpected result: {result:?}"),
    }
}

#[given(expr = r#"an empty playlist"#)]
fn given_empty_playlist(world: &mut PlaylistWorld) {
    world.playlist_items.clear();
    world.library_items.clear();
}

#[when(expr = r#"I save the playlist"#)]
fn when_save_playlist(world: &mut PlaylistWorld) {
    let result = world.inner.execute(Command::PlaylistSave {
        playlist_items: world.playlist_items.clone(),
        library_items: world.library_items.clone(),
    });

    match result {
        CommandResult::PlaylistSaved => {}
        _ => panic!("Unexpected result: {result:?}"),
    }
}

#[when(expr = r#"I load the playlist"#)]
fn when_load_playlist(world: &mut PlaylistWorld) {
    let result = world.inner.execute(Command::PlaylistLoad);

    match result {
        CommandResult::PlaylistLoaded {
            playlist_items,
            virtual_library_items,
        } => {
            world.playlist_items = playlist_items;
            world.library_items = virtual_library_items;
        }
        _ => panic!("Unexpected result: {result:?}"),
    }
}

#[when(regex = r#"I reorder to (.*)"#)]
fn when_reorder_playlist(world: &mut PlaylistWorld, new_order: String) {
    let ordered_names = parse_file_list(&new_order);
    let mut reordered = Vec::new();

    for name in ordered_names {
        if let Some(item) = world
            .playlist_items
            .iter()
            .find(|item| item.path.to_string_lossy().ends_with(&name))
        {
            reordered.push(item.clone());
        } else {
            panic!("File '{name}' not found in playlist");
        }
    }

    world.playlist_items = reordered;
}

#[then(regex = r#"the playlist contains (.*) in order"#)]
fn then_playlist_contains_in_order(world: &mut PlaylistWorld, expected_files: String) {
    let expected_names = parse_file_list(&expected_files);

    assert_eq!(
        world.playlist_items.len(),
        expected_names.len(),
        "Playlist has {} items, expected {}",
        world.playlist_items.len(),
        expected_names.len()
    );

    for (i, (item, expected_name)) in world
        .playlist_items
        .iter()
        .zip(expected_names.iter())
        .enumerate()
    {
        let item_name = item.path.to_string_lossy();
        assert!(
            item_name.ends_with(expected_name),
            "Item {i} is '{item_name}' but expected to end with '{expected_name}'"
        );
    }
}

#[then(expr = r#"the file {string} has alias {string}"#)]
fn then_file_has_alias(world: &mut PlaylistWorld, filename: String, expected_alias: String) {
    let item = world
        .playlist_items
        .iter()
        .find(|item| item.path.to_string_lossy().ends_with(&filename))
        .unwrap_or_else(|| panic!("File '{filename}' not found in playlist"));

    assert!(
        item.alias.as_ref() == Some(&expected_alias),
        "File '{}' has alias {:?}, expected '{}'",
        filename,
        item.alias,
        expected_alias
    );
}

#[then(expr = r#"the playlist is empty"#)]
fn then_playlist_is_empty(world: &mut PlaylistWorld) {
    assert!(
        world.playlist_items.is_empty(),
        "Playlist should be empty but has {} items",
        world.playlist_items.len()
    );
}

#[tokio::main]
async fn main() {
    PlaylistWorld::run("tests/features/playlist_persistence.feature").await;
}
