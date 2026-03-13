#![allow(clippy::missing_panics_doc)]

use acceptance::ShownotesWorld;
use cucumber::{World, given, then, when};

use marked_path::CanonicalPath;
use shownotes::command::{Command, CommandResult, execute, format_output};

#[derive(Debug, World)]
#[world(init = Self::new_world)]
pub struct SymlinkWorld {
    inner: ShownotesWorld,
    output: String,
    last_result: Option<CommandResult>,
}

impl SymlinkWorld {
    async fn new_world() -> Self {
        Self {
            inner: ShownotesWorld::new().await,
            output: String::new(),
            last_result: None,
        }
    }
}

#[given(expr = r#"a real file at {string}"#)]
fn given_real_file(world: &mut SymlinkWorld, filename: String) {
    world.inner.create_file(&filename);
}

#[given(expr = r#"a symlink to {string} at {string}"#)]
fn given_symlink(world: &mut SymlinkWorld, target: String, link: String) {
    world.inner.create_symlink(&target, &link);
}

#[given(expr = r#"the file {string} has source {string}"#)]
async fn given_file_has_source(world: &mut SymlinkWorld, path: String, url: String) {
    let full_path = world.inner.resolve_path(&path);

    execute(
        &world.inner.ctx,
        Command::SourcesAdd {
            path: CanonicalPath::from_path(&full_path).expect("failed to canonicalize path"),
            url,
        },
    )
    .await
    .expect("add command failed");
}

#[when(expr = r#"I run {string}"#)]
async fn when_run_command(world: &mut SymlinkWorld, command: String) {
    let parts: Vec<&str> = command.split_whitespace().collect();
    assert!(parts.len() >= 3, "Invalid command format: {command}");

    let cmd = match (parts[0], parts[1]) {
        ("sources", "list") => {
            let filename = parts[2].trim_matches('"');
            let full_path = world.inner.resolve_path(filename);
            Command::SourcesList {
                path: CanonicalPath::from_path(&full_path).expect("failed to canonicalize path"),
            }
        }
        ("sources", "add") => {
            let filename = parts[2].trim_matches('"');
            let url = parts[3].trim_matches('"');
            let full_path = world.inner.resolve_path(filename);
            Command::SourcesAdd {
                path: CanonicalPath::from_path(&full_path).expect("failed to canonicalize path"),
                url: url.to_string(),
            }
        }
        _ => panic!("Unknown command: {command}"),
    };

    let result = execute(&world.inner.ctx, cmd)
        .await
        .expect("command failed");

    world.output = format_output(&result);
    world.last_result = Some(result);
}

#[when(expr = r#"I edit sources for {string} with {string}"#)]
async fn when_edit_sources(world: &mut SymlinkWorld, path: String, content: String) {
    world.inner.fake_editor.set_content(content);
    let full_path = world.inner.resolve_path(&path);

    let result = execute(
        &world.inner.ctx,
        Command::SourcesEdit {
            path: CanonicalPath::from_path(&full_path).expect("failed to canonicalize path"),
        },
    )
    .await
    .expect("edit command failed");

    world.last_result = Some(result);
}

#[then(expr = r#"the output contains {string}"#)]
fn then_output_contains(world: &mut SymlinkWorld, expected: String) {
    assert!(
        world.output.contains(&expected),
        "expected output to contain '{}', but got: '{}'",
        expected,
        world.output
    );
}

#[then(expr = r#"the file {string} has source {string}"#)]
async fn then_file_has_source(world: &mut SymlinkWorld, path: String, expected_url: String) {
    let full_path = world.inner.resolve_path(&path);

    let result = execute(
        &world.inner.ctx,
        Command::SourcesList {
            path: CanonicalPath::from_path(&full_path).expect("failed to canonicalize path"),
        },
    )
    .await
    .expect("list command failed");

    match result {
        CommandResult::SourcesList { urls, .. } => {
            assert!(
                urls.contains(&expected_url),
                "expected file '{path}' to have source '{expected_url}', found: {urls:?}"
            );
        }
        _ => panic!("Unexpected result type: {result:?}"),
    }
}

#[then(expr = r#"the file {string} shows source {string}"#)]
async fn then_file_shows_source(world: &mut SymlinkWorld, path: String, expected_url: String) {
    let full_path = world.inner.resolve_path(&path);

    let result = execute(
        &world.inner.ctx,
        Command::SourcesList {
            path: CanonicalPath::from_path(&full_path).expect("failed to canonicalize path"),
        },
    )
    .await
    .expect("list command failed");

    match result {
        CommandResult::SourcesList { urls, .. } => {
            let output = urls.join("\n");
            assert!(
                output.contains(&expected_url),
                "expected list output for '{path}' to contain '{expected_url}', but got: '{output}'"
            );
        }
        _ => panic!("Unexpected result type: {result:?}"),
    }
}

#[tokio::main]
async fn main() {
    SymlinkWorld::run("tests/features/sources_symlinks.feature").await;
}
