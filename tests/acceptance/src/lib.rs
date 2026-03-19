#![allow(clippy::missing_panics_doc)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crossterm::event::Event;
use cucumber::World;
use marked_path::CanonicalPath;
use tempfile::TempDir;

use shownotes::feat::config::Config;
use shownotes::feat::external_editor::{ExternalEditorService, FakeEditor};
use shownotes::services::Services;
use shownotes::{App, Command, CommandResult, SystemCtx};

#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct ShownotesWorld {
    pub app: Option<App>,
    pub temp_dir: TempDir,
    pub file_paths: HashMap<String, PathBuf>,
    pub fake_editor: Arc<FakeEditor>,
}

impl Drop for ShownotesWorld {
    fn drop(&mut self) {
        if let Some(app) = self.app.take() {
            // Drop the app (and its runtime) in a separate thread to avoid
            // "Cannot start a runtime from within a runtime" error
            std::thread::spawn(move || {
                drop(app);
            })
            .join()
            .ok();
        }
    }
}

impl Default for ShownotesWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl ShownotesWorld {
    pub fn new() -> Self {
        tokio::task::block_in_place(|| {
            let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
            let db_path = temp_dir.path().join("test.db");
            let db_path_str = db_path.to_string_lossy().to_string();

            let fake_editor = Arc::new(FakeEditor::new());
            let rt = tokio::runtime::Runtime::new().expect("failed to create runtime");
            let handle = rt.handle().clone();

            let services = rt
                .block_on(Services::new(&db_path_str, handle))
                .expect("failed to create services");

            let services = Services {
                editor: ExternalEditorService::new(fake_editor.clone()),
                ..services
            };

            let library_path = CanonicalPath::from_path(temp_dir.path())
                .expect("failed to canonicalize library path");
            let ctx = SystemCtx {
                services,
                config: Config::default(),
                library_path,
                socket_path: String::new(),
                keymap: shownotes::Keymap::new(),
            };

            let app = App::new(ctx, rt);

            Self {
                app: Some(app),
                temp_dir,
                file_paths: HashMap::new(),
                fake_editor,
            }
        })
    }

    pub fn execute(&mut self, command: Command) -> CommandResult {
        let app = self.app.as_mut().expect("app not initialized");
        tokio::task::block_in_place(|| app.execute(command).expect("command failed"))
    }

    pub fn handle_event(&mut self, event: Event) {
        let app = self.app.as_mut().expect("app not initialized");
        tokio::task::block_in_place(|| app.handle_event(event));
    }

    pub fn resolve_path(&self, relative: &str) -> PathBuf {
        self.file_paths
            .get(relative)
            .cloned()
            .unwrap_or_else(|| self.temp_dir.path().join(relative))
    }

    pub fn create_file(&mut self, filename: &str) -> PathBuf {
        let full_path = self.temp_dir.path().join(filename);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).expect("failed to create parent dir");
        }
        std::fs::File::create(&full_path).expect("failed to create file");
        self.file_paths
            .insert(filename.to_string(), full_path.clone());
        full_path
    }

    pub fn create_symlink(&mut self, target: &str, link: &str) -> PathBuf {
        let target_path = self.resolve_path(target);
        let link_path = self.temp_dir.path().join(link);
        if let Some(parent) = link_path.parent() {
            std::fs::create_dir_all(parent).expect("failed to create parent dir");
        }
        std::os::unix::fs::symlink(&target_path, &link_path).expect("failed to create symlink");
        self.file_paths.insert(link.to_string(), link_path.clone());
        link_path
    }
}

pub mod steps {
    use super::ShownotesWorld;
    use cucumber::given;

    #[given(expr = r#"a real file at {string}"#)]
    pub fn given_real_file(world: &mut ShownotesWorld, filename: String) {
        world.create_file(&filename);
    }

    #[given(expr = r#"a symlink to {string} at {string}"#)]
    pub fn given_symlink(world: &mut ShownotesWorld, target: String, link: String) {
        world.create_symlink(&target, &link);
    }
}
