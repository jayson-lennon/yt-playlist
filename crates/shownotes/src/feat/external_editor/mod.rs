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

use std::sync::Arc;

use async_trait::async_trait;
use derive_more::Debug;
use error_stack::Report;
use wherror::Error;

mod editors;

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

/// Service for opening external text editors.
///
/// Provides an interface for launching an external editor to edit
/// text content, such as notes or source URLs. Uses the system's
/// configured editor (from $EDITOR or $VISUAL).
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
