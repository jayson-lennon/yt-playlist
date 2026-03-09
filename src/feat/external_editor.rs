use std::sync::Arc;

use async_trait::async_trait;
use derive_more::Debug;
use dialoguer::Editor;
use error_stack::Report;
use wherror::Error;

#[derive(Debug, Error)]
#[error("failed to open editor")]
pub struct ExternalEditorError;

#[derive(Debug, Error)]
pub enum DialoguerEditorError {
    #[error("failed to open editor")]
    Open,
}

#[derive(Debug, Clone)]
pub struct SystemEditor;

#[async_trait]
pub trait ExternalEditor: Send + Sync {
    async fn open(
        &self,
        initial_content: &str,
    ) -> Result<Option<String>, Report<ExternalEditorError>>;
}

#[async_trait]
impl ExternalEditor for SystemEditor {
    async fn open(
        &self,
        initial_content: &str,
    ) -> Result<Option<String>, Report<ExternalEditorError>> {
        let content = initial_content.to_string();
        tokio::task::spawn_blocking(move || {
            let edited = Editor::new()
                .edit(&content)
                .map_err(|_| Report::new(ExternalEditorError).attach(DialoguerEditorError::Open))?;

            match edited {
                Some(new_content) if new_content != content => Ok(Some(new_content)),
                _ => Ok(None),
            }
        })
        .await
        .map_err(|_| Report::new(ExternalEditorError))?
    }
}

#[derive(Debug, Clone)]
pub struct ExternalEditorService {
    #[debug("<ExternalEditor>")]
    backend: Arc<dyn ExternalEditor>,
}

impl ExternalEditorService {
    pub fn new(backend: Arc<dyn ExternalEditor>) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl ExternalEditor for ExternalEditorService {
    async fn open(
        &self,
        initial_content: &str,
    ) -> Result<Option<String>, Report<ExternalEditorError>> {
        self.backend.open(initial_content).await
    }
}
