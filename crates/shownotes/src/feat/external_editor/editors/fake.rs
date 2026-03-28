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

    /// # Panics
    ///
    /// Panics if the mutex is poisoned.
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
