pub mod backend;

use std::sync::Arc;

use derive_more::Debug;
use error_stack::Report;
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct FuzzySearchError(pub String);

pub struct FuzzySearchResult {
    pub selected_paths: Vec<String>,
}

pub trait FuzzySearch: Send + Sync {
    fn name(&self) -> &'static str;

    fn search(
        &self,
        items: &[(String, String)],
    ) -> Result<FuzzySearchResult, Report<FuzzySearchError>>;
}

#[derive(Debug, Clone)]
pub struct FuzzySearchService {
    #[debug("<FuzzySearch>")]
    backend: Arc<dyn FuzzySearch>,
}

impl FuzzySearchService {
    pub fn new(backend: Arc<dyn FuzzySearch>) -> Self {
        Self { backend }
    }

    pub fn search(
        &self,
        items: &[(String, String)],
    ) -> Result<FuzzySearchResult, Report<FuzzySearchError>> {
        self.backend.search(items)
    }
}
