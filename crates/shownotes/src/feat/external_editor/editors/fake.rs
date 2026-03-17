use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use error_stack::Report;

use super::super::{ExternalEditor, ExternalEditorError};

#[derive(Debug, Clone)]
pub struct FakeEditor {
    content: Arc<Mutex<Option<String>>>,
    append_mode: Arc<Mutex<bool>>,
}

impl FakeEditor {
    pub fn new() -> Self {
        Self {
            content: Arc::new(Mutex::new(None)),
            append_mode: Arc::new(Mutex::new(false)),
        }
    }

    /// # Panics
    ///
    /// Panics if the mutex is poisoned.
    pub fn set_content(&self, content: String) {
        let mut guard = self.content.lock().unwrap();
        *guard = Some(content);
    }

    pub fn set_append_mode(&self, mode: bool) {
        let mut guard = self.append_mode.lock().unwrap();
        *guard = mode;
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
        initial_content: &str,
    ) -> Result<Option<String>, Report<ExternalEditorError>> {
        let mut content_guard = self.content.lock().unwrap();
        let append_guard = self.append_mode.lock().unwrap();
        
        let result = if *append_guard && !initial_content.is_empty() {
            match content_guard.take() {
                Some(new_content) => Some(format!("{initial_content}\n\n{new_content}")),
                None => Some(initial_content.to_string()),
            }
        } else {
            content_guard.take()
        };
        
        Ok(result)
    }
}
