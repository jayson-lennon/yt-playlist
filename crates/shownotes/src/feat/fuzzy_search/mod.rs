mod backend;

pub use backend::SkimBackend;

use std::sync::Arc;

use derive_more::Debug;
use error_stack::Report;
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct FuzzySearchError(pub String);

/// Result from a fuzzy search operation.
///
/// Contains the matched file path and its associated note content
/// for display in search results.
pub struct FuzzySearchResult {
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

/// Service for fuzzy searching through notes.
///
/// Provides an interface for searching across all stored notes using
/// fuzzy matching. Delegates to a backend implementation (skim) for
/// actual search operations.
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
