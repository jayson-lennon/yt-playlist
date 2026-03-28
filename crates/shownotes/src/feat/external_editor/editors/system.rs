// Copyright (C) 2026 Jayson Lennon
// 
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// 
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
// 
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use async_trait::async_trait;
use dialoguer::Editor;
use error_stack::{Report, ResultExt};

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
                .change_context(ExternalEditorError).attach(DialoguerEditorError::Open)?;

            match edited {
                Some(new_content) if new_content != content => Ok(Some(new_content)),
                _ => Ok(None),
            }
        })
        .await
        .change_context(ExternalEditorError)?
    }
}
