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

mod backend;

pub use backend::SkimBackend;

use std::sync::Arc;

use derive_more::Debug;
use error_stack::Report;
use wherror::Error;

/// Error type for fuzzy search failures.
///
/// Wraps an error message describing what went wrong during
/// the fuzzy search operation.
#[derive(Debug, Error)]
#[error(debug)]
pub struct FuzzySearchError(pub String);

/// Result from a fuzzy search operation.
///
/// Contains the paths selected by the user from the fuzzy search interface.
pub struct FuzzySearchResult {
    /// The file paths that were selected by the user.
    pub selected_paths: Vec<String>,
}

pub trait FuzzySearch: Send + Sync {
    fn name(&self) -> &'static str;

    /// # Errors
    /// Returns an error if the fuzzy search backend fails.
    fn search(
        &self,
        items: &[(String, String)],
    ) -> Result<FuzzySearchResult, Report<FuzzySearchError>>;
}

/// Service wrapper for fuzzy search operations.
///
/// Provides a type-safe interface around a fuzzy search backend,
/// delegating all search operations to the configured implementation.
/// Uses dynamic dispatch to allow swapping backends (e.g., skim)
/// without changing calling code.
#[derive(Debug, Clone)]
pub struct FuzzySearchService {
    #[debug("<FuzzySearch>")]
    backend: Arc<dyn FuzzySearch>,
}

impl FuzzySearchService {
    pub fn new(backend: Arc<dyn FuzzySearch>) -> Self {
        Self { backend }
    }

    /// # Errors
    /// Returns an error if the fuzzy search backend fails.
    pub fn search(
        &self,
        items: &[(String, String)],
    ) -> Result<FuzzySearchResult, Report<FuzzySearchError>> {
        self.backend.search(items)
    }
}
