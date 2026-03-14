#![allow(clippy::missing_panics_doc)]

use acceptance::ShownotesWorld;
use cucumber::{World, given, then, when};

use marked_path::CanonicalPath;
use shownotes::command::{Command, CommandResult, execute, format_output};

#[derive(Debug, World)]
#[world(init = Self::new_world)]
pub struct SourcesBasicWorld {
    inner: ShownotesWorld,
    output: String,
    last_urls: Vec<String>,
}

impl SourcesBasicWorld {
    async fn new_world() -> Self {
        Self {
            inner: ShownotesWorld::new().await,
            output: String::new(),
            last_urls: Vec::new(),
        }
    }
}

#[given(expr = r#"a file {string}"#)]
fn given_file(world: &mut SourcesBasicWorld, filename: String) {
    world.inner.create_file(&filename);
}

#[given(expr = r#"a file {string} with source {string}"#)]
async fn given_file_with_source(world: &mut SourcesBasicWorld, filename: String, url: String) {
    world.inner.create_file(&filename);
    let full_path = world.inner.resolve_path(&filename);

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

#[given(expr = r#"the file {string} has source {string}"#)]
async fn given_file_has_source(world: &mut SourcesBasicWorld, path: String, url: String) {
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

#[when(expr = r#"I add source {string} to {string}"#)]
async fn when_add_source(world: &mut SourcesBasicWorld, url: String, path: String) {
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

#[when(expr = r#"I list sources for {string}"#)]
async fn when_list_sources(world: &mut SourcesBasicWorld, path: String) {
    let full_path = world.inner.resolve_path(&path);

    let result = execute(
        &world.inner.ctx,
        Command::SourcesList {
            path: CanonicalPath::from_path(&full_path).expect("failed to canonicalize path"),
        },
    )
    .await
    .expect("list command failed");

    if let CommandResult::SourcesList { urls, .. } = &result {
        world.last_urls.clone_from(urls);
    }
    world.output = format_output(&result);
}

#[when(expr = r#"I edit sources for {string} with {string}"#)]
async fn when_edit_sources(world: &mut SourcesBasicWorld, path: String, content: String) {
    let content = content.replace("\\n", "\n");
    world.inner.fake_editor.set_content(content);
    let full_path = world.inner.resolve_path(&path);

    execute(
        &world.inner.ctx,
        Command::SourcesEdit {
            path: CanonicalPath::from_path(&full_path).expect("failed to canonicalize path"),
        },
    )
    .await
    .expect("edit command failed");
}

#[then(expr = r#"the file {string} has source {string}"#)]
async fn then_file_has_source(world: &mut SourcesBasicWorld, path: String, expected_url: String) {
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

#[then(expr = r#"the file {string} does not have source {string}"#)]
async fn then_file_does_not_have_source(
    world: &mut SourcesBasicWorld,
    path: String,
    expected_url: String,
) {
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
                !urls.contains(&expected_url),
                "expected file '{path}' to NOT have source '{expected_url}', found: {urls:?}"
            );
        }
        _ => panic!("Unexpected result type: {result:?}"),
    }
}

#[then(expr = r#"the output contains {string}"#)]
fn then_output_contains(world: &mut SourcesBasicWorld, expected: String) {
    assert!(
        world.output.contains(&expected),
        "expected output to contain '{}', but got: '{}'",
        expected,
        world.output
    );
}

#[then(expr = r#"the output is empty"#)]
fn then_output_is_empty(world: &mut SourcesBasicWorld) {
    assert!(
        world.last_urls.is_empty(),
        "expected sources list to be empty, but found: {:?}",
        world.last_urls
    );
}

#[tokio::main]
async fn main() {
    SourcesBasicWorld::run("tests/features/sources_basic.feature").await;
}
