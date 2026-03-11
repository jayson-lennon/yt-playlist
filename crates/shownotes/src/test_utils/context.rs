use tempfile::NamedTempFile;

use crate::feat::note_db::NoteDb;
use crate::services::Services;

use super::services::create_test_services;

pub struct NoteTestContext {
    pub services: Services,
    pub temp_file: NamedTempFile,
    pub file_path_id: i64,
}

impl NoteTestContext {
    pub async fn new() -> Self {
        let services = create_test_services().await;
        let temp_file = NamedTempFile::new().unwrap();
        let path_str = temp_file.path().to_string_lossy();
        let file_path_id = services.db.get_or_create_file_path(&path_str).await.unwrap();
        Self { services, temp_file, file_path_id }
    }
}
