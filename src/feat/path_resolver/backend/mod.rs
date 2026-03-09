use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use derive_more::Debug;
use error_stack::Report;

use super::{PathResolutionError, PathResolver};

pub mod system;

pub use system::SystemPathResolver;

#[derive(Debug, Clone)]
pub struct PathResolverWrapper {
    #[debug("<PathResolver>")]
    backend: Arc<dyn PathResolver>,
}

impl PathResolverWrapper {
    pub fn new(backend: Arc<dyn PathResolver>) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl PathResolver for PathResolverWrapper {
    async fn resolve(&self, path: &Path) -> Result<PathBuf, Report<PathResolutionError>> {
        self.backend.resolve(path).await
    }
}
