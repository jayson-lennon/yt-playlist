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
use cucumber::{World, given, then, when};

use acceptance::ShownotesWorld;
use marked_path::CanonicalPath;
use shownotes::common::ItemPath;
use shownotes::command::{Command, CommandResult};

#[derive(Debug, World)]
#[world(init = Self::new_world)]
pub struct AliasDeletionWorld {
    inner: ShownotesWorld,
}

impl AliasDeletionWorld {
    fn new_world() -> Self {
        Self {
            inner: ShownotesWorld::new(),
        }
    }
}

#[given(expr = r#"a real file at {string}"#)]
fn given_real_file(world: &mut AliasDeletionWorld, filename: String) {
    world.inner.create_file(&filename);
}

#[given(expr = r#"the file {string} has alias {string}"#)]
fn given_file_has_alias(world: &mut AliasDeletionWorld, path: String, alias: String) {
    let full_path = world.inner.resolve_path(&path);
    let canonical = CanonicalPath::from_path(&full_path).expect("failed to canonicalize path");
    let workspace = CanonicalPath::from_path(world.inner.temp_dir.path())
        .expect("failed to canonicalize workspace");

    world.inner.execute(Command::AliasSet {
        path: canonical,
        workspace,
        alias,
    });
}

#[when(expr = r#"I remove the alias from {string}"#)]
fn when_remove_alias(world: &mut AliasDeletionWorld, path: String) {
    let full_path = world.inner.resolve_path(&path);
    let canonical = CanonicalPath::from_path(&full_path).expect("failed to canonicalize path");
    let workspace = CanonicalPath::from_path(world.inner.temp_dir.path())
        .expect("failed to canonicalize workspace");

    world.inner.execute(Command::AliasRemove {
        path: canonical,
        workspace,
    });
}

#[then(expr = r#"the file {string} has no alias"#)]
fn then_file_has_no_alias(world: &mut AliasDeletionWorld, path: String) {
    let full_path = world.inner.resolve_path(&path);
    let canonical = CanonicalPath::from_path(&full_path).expect("failed to canonicalize path");

    let result = world.inner.execute(Command::PlaylistLoad);
    if let CommandResult::PlaylistLoaded { playlist_items, .. } = result {
        let item = playlist_items.iter().find(|i| {
            matches!(&i.path, ItemPath::File(p) if p == &canonical)
        });
        assert!(
            item.is_none_or(|i| i.alias.is_none()),
            "expected file '{path}' to have no alias"
        );
    }
}

#[tokio::main]
async fn main() {
    AliasDeletionWorld::run("tests/features/alias_deletion.feature").await;
}
