#![allow(clippy::missing_panics_doc)]

use acceptance::ShownotesWorld;
use cucumber::{World, given, then, when};
use marked_path::CanonicalPath;
use shownotes::command::{Command, CommandResult, execute, format_output};
use shownotes::common::domain::{ItemPath, PlaylistItem};

#[derive(Debug, World)]
#[world(init = Self::new_world)]
pub struct GenerateWorld {
    inner: ShownotesWorld,
    output: String,
    last_result: Option<CommandResult>,
}

impl GenerateWorld {
    async fn new_world() -> Self {
        Self {
            inner: ShownotesWorld::new().await,
            output: String::new(),
            last_result: None,
        }
    }
}

#[given(expr = r#"a file {string} with source {string}"#)]
async fn given_file_with_source(world: &mut GenerateWorld, filename: String, url: String) {
    let full_path = world.inner.create_file(&filename);
    let canonical = CanonicalPath::from_path(&full_path).expect("failed to canonicalize path");

    execute(
        &world.inner.ctx,
        Command::SourcesAdd {
            path: canonical.clone(),
            url,
        },
    )
    .await
    .expect("add source command failed");

    let item = PlaylistItem {
        path: ItemPath::File(canonical),
        duration: None,
        alias: None,
        mime_type: None,
        is_virtual: false,
    };

    let existing_playlist = world.inner.ctx.services.storage.load(&world.inner.ctx.library_path).await.ok();
    let mut playlist_items = existing_playlist
        .map(|data| {
            data.playlist
                .into_iter()
                .map(|path| PlaylistItem {
                    path,
                    duration: None,
                    alias: None,
                    mime_type: None,
                    is_virtual: false,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    playlist_items.push(item);

    execute(
        &world.inner.ctx,
        Command::PlaylistSave {
            playlist_items,
            library_items: vec![],
        },
    )
    .await
    .expect("save playlist command failed");
}

#[given(expr = r#"a file {string} exists"#)]
async fn given_file_exists(world: &mut GenerateWorld, filename: String) {
    let full_path = world.inner.create_file(&filename);
    let canonical = CanonicalPath::from_path(&full_path).expect("failed to canonicalize path");

    let item = PlaylistItem {
        path: ItemPath::File(canonical),
        duration: None,
        alias: None,
        mime_type: None,
        is_virtual: false,
    };

    let existing_playlist = world.inner.ctx.services.storage.load(&world.inner.ctx.library_path).await.ok();
    let mut playlist_items = existing_playlist
        .map(|data| {
            data.playlist
                .into_iter()
                .map(|path| PlaylistItem {
                    path,
                    duration: None,
                    alias: None,
                    mime_type: None,
                    is_virtual: false,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    playlist_items.push(item);

    execute(
        &world.inner.ctx,
        Command::PlaylistSave {
            playlist_items,
            library_items: vec![],
        },
    )
    .await
    .expect("save playlist command failed");
}

#[given(expr = r#"no files in playlist"#)]
async fn given_no_files_in_playlist(world: &mut GenerateWorld) {
    execute(
        &world.inner.ctx,
        Command::PlaylistSave {
            playlist_items: vec![],
            library_items: vec![],
        },
    )
    .await
    .expect("save empty playlist command failed");
}

#[when(expr = r#"I generate show notes in {string} format"#)]
async fn when_generate_show_notes(world: &mut GenerateWorld, format: String) {
    let result = execute(
        &world.inner.ctx,
        Command::GenerateNotes {
            format,
            working_directory: world.inner.ctx.library_path.clone(),
        },
    )
    .await
    .expect("generate notes command failed");

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

#[tokio::main]
async fn main() {
    GenerateWorld::run("tests/features/generate_show_notes.feature").await;
}
