use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use derive_more::Debug;
use error_stack::Report;

use super::{PathResolutionError, PathResolverBackend};

pub mod system;

pub use system::SystemPathResolver;

#[derive(Debug, Clone)]
pub struct PathResolver {
    #[debug("<PathResolver>")]
    backend: Arc<dyn PathResolverBackend>,
}

impl PathResolver {
    pub fn new(backend: Arc<dyn PathResolverBackend>) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl PathResolverBackend for PathResolver {
    async fn resolve(&self, path: &Path) -> Result<PathBuf, Report<PathResolutionError>> {
        self.backend.resolve(path).await
    }
}
