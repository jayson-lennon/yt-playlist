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
