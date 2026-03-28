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

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use error_stack::{Report, ResultExt};

use super::super::{PathResolutionError, PathResolver};

#[derive(Debug, Clone)]
pub struct SystemPathResolver;

#[async_trait]
impl PathResolver for SystemPathResolver {
    async fn resolve(&self, path: &Path) -> Result<PathBuf, Report<PathResolutionError>> {
        let path = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            path.canonicalize()
                .change_context(PathResolutionError)
        })
        .await
        .change_context(PathResolutionError)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn resolve_follows_symlink_to_real_path() {
        // Given a symlink pointing to a real file.
        let temp_dir = TempDir::new().unwrap();
        let real_file = temp_dir.path().join("real_file.txt");
        let symlink_path = temp_dir.path().join("symlink_file.txt");
        fs::write(&real_file, "test content").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&real_file, &symlink_path).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&real_file, &symlink_path).unwrap();

        // When resolving the symlink.
        let resolver = SystemPathResolver;
        let canonical = resolver.resolve(&symlink_path).await.unwrap();

        // Then the real file path is returned.
        assert_eq!(canonical, real_file.canonicalize().unwrap());
    }

    #[tokio::test]
    async fn resolve_returns_canonical_path_for_non_symlink() {
        // Given a regular file without symlinks.
        let temp_dir = TempDir::new().unwrap();
        let real_file = temp_dir.path().join("real_file.txt");
        fs::write(&real_file, "test content").unwrap();

        // When resolving the file path.
        let resolver = SystemPathResolver;
        let canonical = resolver.resolve(&real_file).await.unwrap();

        // Then the canonical path is returned.
        assert_eq!(canonical, real_file.canonicalize().unwrap());
    }

    #[tokio::test]
    async fn resolve_follows_chained_symlinks() {
        // Given chained symlinks pointing to a real file.
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
        let canonical = resolver.resolve(&symlink2).await.unwrap();

        // Then the real file path is returned.
        assert_eq!(canonical, real_file.canonicalize().unwrap());
    }

    #[tokio::test]
    async fn resolve_fails_for_nonexistent_path() {
        // Given a path to a nonexistent file.
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("does_not_exist.txt");

        // When resolving the nonexistent path.
        let resolver = SystemPathResolver;
        let result = resolver.resolve(&nonexistent).await;

        // Then an error is returned.
        assert!(result.is_err());
    }
}
