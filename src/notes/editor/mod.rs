use std::sync::Arc;

use async_trait::async_trait;
use derive_more::Debug;
use dialoguer::Editor;
use error_stack::Report;
use wherror::Error;

use crate::notes::{Editor as EditorTrait, EditorError};

#[derive(Debug, Error)]
pub enum DialoguerEditorError {
    #[error("failed to open editor")]
    Open,
}

#[derive(Debug, Clone)]
pub struct SystemEditor;

#[async_trait]
impl EditorTrait for SystemEditor {
    async fn open(&self, initial_content: &str) -> Result<Option<String>, Report<EditorError>> {
        let content = initial_content.to_string();
        tokio::task::spawn_blocking(move || {
            let edited = Editor::new()
                .edit(&content)
                .map_err(|_| Report::new(EditorError).attach(DialoguerEditorError::Open))?;

            match edited {
                Some(new_content) if new_content != content => Ok(Some(new_content)),
                _ => Ok(None),
            }
        })
        .await
        .map_err(|_| Report::new(EditorError))?
    }
}

#[derive(Debug, Clone)]
pub struct EditorWrapper {
    #[debug("<Editor>")]
    backend: Arc<dyn EditorTrait>,
}

impl EditorWrapper {
    pub fn new(backend: Arc<dyn EditorTrait>) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl EditorTrait for EditorWrapper {
    async fn open(&self, initial_content: &str) -> Result<Option<String>, Report<EditorError>> {
        self.backend.open(initial_content).await
    }
}
