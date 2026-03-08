use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use derive_more::Debug;
use error_stack::Report;
use wherror::Error;

use crate::notes::{PathResolutionError, PathResolver as PathResolverTrait};

#[derive(Debug, Error)]
pub enum CanonicalizeError {
    #[error("path does not exist or cannot be resolved")]
    NotFound,
    #[error("symlink resolution failed")]
    Symlink,
}

#[derive(Debug, Clone)]
pub struct SystemPathResolver;

#[async_trait]
impl PathResolverTrait for SystemPathResolver {
    async fn resolve(&self, path: &Path) -> Result<PathBuf, Report<PathResolutionError>> {
        let path = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            path.canonicalize()
                .map_err(|_| Report::new(PathResolutionError).attach(CanonicalizeError::NotFound))
        })
        .await
        .map_err(|_| Report::new(PathResolutionError))?
    }
}

#[derive(Debug, Clone)]
pub struct PathResolverWrapper {
    #[debug("<PathResolver>")]
    backend: Arc<dyn PathResolverTrait>,
}

impl PathResolverWrapper {
    pub fn new(backend: Arc<dyn PathResolverTrait>) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl PathResolverTrait for PathResolverWrapper {
    async fn resolve(&self, path: &Path) -> Result<PathBuf, Report<PathResolutionError>> {
        self.backend.resolve(path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn resolve_follows_symlink_to_real_path() {
        // Given a file and a symlink pointing to it.
        let temp_dir = TempDir::new().unwrap();
        let real_file = temp_dir.path().join("real_file.txt");
        let symlink_path = temp_dir.path().join("symlink_file.txt");
        fs::write(&real_file, "test content").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&real_file, &symlink_path).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&real_file, &symlink_path).unwrap();

        // When resolving the symlink path.
        let resolver = SystemPathResolver;
        let resolved = resolver.resolve(&symlink_path).await.unwrap();

        // Then it returns the canonical path of the real file.
        assert_eq!(resolved, real_file.canonicalize().unwrap());
    }

    #[tokio::test]
    async fn resolve_returns_canonical_path_for_non_symlink() {
        // Given a regular file.
        let temp_dir = TempDir::new().unwrap();
        let real_file = temp_dir.path().join("real_file.txt");
        fs::write(&real_file, "test content").unwrap();

        // When resolving the file path.
        let resolver = SystemPathResolver;
        let resolved = resolver.resolve(&real_file).await.unwrap();

        // Then it returns the canonical path.
        assert_eq!(resolved, real_file.canonicalize().unwrap());
    }

    #[tokio::test]
    async fn resolve_follows_chained_symlinks() {
        // Given a chain of symlinks: symlink2 -> symlink1 -> real_file.
        let temp_dir = TempDir::new().unwrap();
        let real_file = temp_dir.path().join("real_file.txt");
        let symlink1 = temp_dir.path().join("symlink1.txt");
        let symlink2 = temp_dir.path().join("symlink2.txt");
        fs::write(&real_file, "test content").unwrap();

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&real_file, &symlink1).unwrap();
            std::os::unix::fs::symlink(&symlink1, &symlink2).unwrap();
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(&real_file, &symlink1).unwrap();
            std::os::windows::fs::symlink_file(&symlink1, &symlink2).unwrap();
        }

        // When resolving the final symlink in the chain.
        let resolver = SystemPathResolver;
        let resolved = resolver.resolve(&symlink2).await.unwrap();

        // Then it resolves to the real file's canonical path.
        assert_eq!(resolved, real_file.canonicalize().unwrap());
    }

    #[tokio::test]
    async fn resolve_fails_for_nonexistent_path() {
        // Given a path that does not exist.
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("does_not_exist.txt");

        // When resolving the nonexistent path.
        let resolver = SystemPathResolver;
        let result = resolver.resolve(&nonexistent).await;

        // Then an error is returned.
        assert!(result.is_err());
    }
}
