use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use derive_more::Debug;
use error_stack::Report;

use super::{PathResolutionError, PathResolver};

mod system;

pub use system::SystemPathResolver;

/// Service for resolving relative paths.
///
/// Provides an interface for resolving relative file paths to absolute
/// paths, handling symlinks and path normalization. Delegates to a
/// backend implementation for actual path resolution.
#[derive(Debug, Clone)]
pub struct PathResolverService {
    #[debug("<PathResolver>")]
    backend: Arc<dyn PathResolver>,
}

impl PathResolverService {
    pub fn new(backend: Arc<dyn PathResolver>) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl PathResolver for PathResolverService {
    async fn resolve(&self, path: &Path) -> Result<PathBuf, Report<PathResolutionError>> {
        self.backend.resolve(path).await
    }
}
