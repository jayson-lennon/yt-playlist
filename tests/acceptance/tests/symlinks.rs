use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use cucumber::{World, given, then, when};
use tempfile::TempDir;

use shownotes::feat::external_editor::{ExternalEditorService, FakeEditor};
use shownotes::feat::sources::SourceDb;
use shownotes::handle_add_command;
use shownotes::handle_edit_command;
use shownotes::resolve_and_get_file_path;
use shownotes::services::Services;

#[derive(Debug, World)]
#[world(init = Self::new_world)]
pub struct SymlinkWorld {
    services: Services,
    temp_dir: TempDir,
    output: String,
    fake_editor: Arc<FakeEditor>,
    file_paths: HashMap<String, PathBuf>,
}

impl SymlinkWorld {
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
            output: String::new(),
            fake_editor,
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
fn given_real_file(world: &mut SymlinkWorld, filename: String) {
    let full_path = world.temp_dir.path().join(&filename);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).expect("failed to create parent dir");
    }
    std::fs::File::create(&full_path).expect("failed to create file");
    world.file_paths.insert(filename, full_path);
}

#[given(expr = r#"a symlink to {string} at {string}"#)]
fn given_symlink(world: &mut SymlinkWorld, target: String, link: String) {
    let target_path = world.resolve_path(&target);
    let link_path = world.temp_dir.path().join(&link);
    if let Some(parent) = link_path.parent() {
        std::fs::create_dir_all(parent).expect("failed to create parent dir");
    }
    std::os::unix::fs::symlink(&target_path, &link_path).expect("failed to create symlink");
    world.file_paths.insert(link, link_path);
}

#[given(expr = r#"the file {string} has source {string}"#)]
async fn given_file_has_source(world: &mut SymlinkWorld, path: String, url: String) {
    let full_path = world.resolve_path(&path);
    handle_add_command(&world.services, full_path, url)
        .await
        .expect("add command failed");
}

#[when(expr = r#"I run {string}"#)]
async fn when_run_command(world: &mut SymlinkWorld, command: String) {
    let parts: Vec<&str> = command.split_whitespace().collect();
    assert!(parts.len() >= 3, "Invalid command format: {command}");

    match (parts[0], parts[1]) {
        ("sources", "list") => {
            let filename = parts[2].trim_matches('"');
            let full_path = world.resolve_path(filename);

            let (_resolved, file_path_id) = resolve_and_get_file_path(&world.services, &full_path)
                .await
                .expect("failed to resolve path");

            let sources = world
                .services
                .sources
                .get_sources(file_path_id)
                .await
                .expect("failed to get sources");

            world.output = if sources.is_empty() {
                format!("No sources found for: {}", full_path.display())
            } else {
                sources
                    .iter()
                    .map(|s| s.source_url.clone())
                    .collect::<Vec<_>>()
                    .join("\n")
            };
        }
        ("sources", "add") => {
            let filename = parts[2].trim_matches('"');
            let url = parts[3].trim_matches('"');
            let full_path = world.resolve_path(filename);
            handle_add_command(&world.services, full_path, url.to_string())
                .await
                .expect("add command failed");
        }
        _ => panic!("Unknown command: {command}"),
    }
}

#[when(expr = r#"I edit sources for {string} with {string}"#)]
async fn when_edit_sources(world: &mut SymlinkWorld, path: String, content: String) {
    world.fake_editor.set_content(content);
    let full_path = world.resolve_path(&path);
    handle_edit_command(&world.services, full_path)
        .await
        .expect("edit command failed");
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
    let full_path = world.resolve_path(&path);

    let (_resolved, file_path_id) = resolve_and_get_file_path(&world.services, &full_path)
        .await
        .expect("failed to resolve path");

    let sources = world
        .services
        .sources
        .get_sources(file_path_id)
        .await
        .expect("failed to get sources");

    let has_source = sources.iter().any(|s| s.source_url == expected_url);
    assert!(
        has_source,
        "expected file '{path}' to have source '{expected_url}', found: {sources:?}"
    );
}

#[then(expr = r#"the file {string} shows source {string}"#)]
async fn then_file_shows_source(world: &mut SymlinkWorld, path: String, expected_url: String) {
    let full_path = world.resolve_path(&path);

    let (_resolved, file_path_id) = resolve_and_get_file_path(&world.services, &full_path)
        .await
        .expect("failed to resolve path");

    let sources = world
        .services
        .sources
        .get_sources(file_path_id)
        .await
        .expect("failed to get sources");

    let output: String = sources
        .iter()
        .map(|s| s.source_url.clone())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        output.contains(&expected_url),
        "expected list output for '{path}' to contain '{expected_url}', but got: '{output}'"
    );
}

#[tokio::main]
async fn main() {
    SymlinkWorld::run("tests/features/sources_symlinks.feature").await;
}
