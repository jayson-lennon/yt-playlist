use tempfile::NamedTempFile;

use marked_path::CanonicalPath;

use crate::feat::config::Config;
use crate::feat::note_db::NoteDb;
use crate::system_ctx::SystemCtx;

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
}
