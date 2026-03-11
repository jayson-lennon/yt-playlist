use async_trait::async_trait;
use dialoguer::Editor;
use error_stack::Report;

use super::super::{DialoguerEditorError, ExternalEditor, ExternalEditorError};

#[derive(Debug, Clone)]
pub struct SystemEditor;

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
