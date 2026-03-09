use std::path::{Path, PathBuf};

use async_trait::async_trait;
use error_stack::Report;
use wherror::Error;

pub mod backend;

pub use backend::{PathResolverWrapper, SystemPathResolver};

#[derive(Debug, Error)]
#[error(debug)]
pub struct PathResolutionError;

#[async_trait]
pub trait PathResolver: Send + Sync {
    async fn resolve(&self, path: &Path) -> Result<PathBuf, Report<PathResolutionError>>;
}
