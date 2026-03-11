use std::sync::Arc;

use async_trait::async_trait;
use derive_more::Debug;
use error_stack::Report;
use wherror::Error;

pub mod editors;

pub use editors::{FakeEditor, SystemEditor};

#[derive(Debug, Error)]
#[error("failed to open editor")]
pub struct ExternalEditorError;

#[derive(Debug, Error)]
pub enum DialoguerEditorError {
    #[error("failed to open editor")]
    Open,
}

#[async_trait]
pub trait ExternalEditor: Send + Sync {
    async fn open(
        &self,
        initial_content: &str,
    ) -> Result<Option<String>, Report<ExternalEditorError>>;
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
