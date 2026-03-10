use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use error_stack::Report;

use super::{ExternalEditor, ExternalEditorError};

#[derive(Debug, Clone)]
pub struct FakeEditor {
    content: Arc<Mutex<Option<String>>>,
}

impl FakeEditor {
    pub fn new() -> Self {
        Self {
            content: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_content(&self, content: String) {
        let mut guard = self.content.lock().unwrap();
        *guard = Some(content);
    }
}

impl Default for FakeEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExternalEditor for FakeEditor {
    async fn open(
        &self,
        _initial_content: &str,
    ) -> Result<Option<String>, Report<ExternalEditorError>> {
        let mut guard = self.content.lock().unwrap();
        Ok(guard.take())
    }
}
