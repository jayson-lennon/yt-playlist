use std::sync::Arc;

use tempfile::NamedTempFile;

use marked_path::CanonicalPath;

use crate::feat::config::Config;
use crate::feat::external_editor::{ExternalEditorService, FakeEditor};
use crate::feat::fuzzy_search::FuzzySearchService;
use crate::feat::note_db::NoteDb;
use crate::services::Services;
use crate::system_ctx::SystemCtx;

use super::fakes::FakeFuzzySearch;
use super::services::create_test_services;

pub struct NoteTestContext {
    pub ctx: SystemCtx,
    pub temp_file: NamedTempFile,
    pub file_path_id: i64,
}

impl NoteTestContext {
    pub async fn new() -> Self {
        let services = create_test_services().await;
        let temp_file = NamedTempFile::new().unwrap();
        let path_str = temp_file.path().to_string_lossy();
        let file_path_id = services.db.get_or_create_file_path(&path_str).await.unwrap();
        let library_path = CanonicalPath::from_path(temp_file.path().parent().unwrap()).unwrap();
        let ctx = SystemCtx {
            services,
            config: Config::default(),
            library_path,
            socket_path: String::new(),
            keymap: crate::feat::keymap::Keymap::new(),
        };
        Self { ctx, temp_file, file_path_id }
    }

    #[allow(dead_code)]
    pub fn builder() -> NoteTestContextBuilder {
        NoteTestContextBuilder::new()
    }
}

pub struct NoteTestContextBuilder {
    editor: Option<Arc<FakeEditor>>,
    fuzzy_search: Option<Arc<FakeFuzzySearch>>,
}

impl NoteTestContextBuilder {
    pub fn new() -> Self {
        Self {
            editor: None,
            fuzzy_search: None,
        }
    }

    pub fn editor(mut self, editor: Arc<FakeEditor>) -> Self {
        self.editor = Some(editor);
        self
    }

    pub fn fuzzy_search(mut self, fuzzy_search: Arc<FakeFuzzySearch>) -> Self {
        self.fuzzy_search = Some(fuzzy_search);
        self
    }

    pub async fn build(self) -> NoteTestContext {
        let mut services = create_test_services().await;

        if let Some(editor) = self.editor {
            services.editor = ExternalEditorService::new(editor);
        }

        if let Some(fuzzy_search) = self.fuzzy_search {
            services.fuzzy_search = FuzzySearchService::new(fuzzy_search);
        }

        let temp_file = NamedTempFile::new().unwrap();
        let path_str = temp_file.path().to_string_lossy();
        let file_path_id = services.db.get_or_create_file_path(&path_str).await.unwrap();
        let library_path = CanonicalPath::from_path(temp_file.path().parent().unwrap()).unwrap();
        let ctx = SystemCtx {
            services,
            config: Config::default(),
            library_path,
            socket_path: String::new(),
            keymap: crate::feat::keymap::Keymap::new(),
        };
        NoteTestContext { ctx, temp_file, file_path_id }
    }
}

impl Default for NoteTestContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}
