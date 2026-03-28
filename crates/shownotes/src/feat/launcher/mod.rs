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

mod xdg;

use std::{path::Path, sync::Arc};

use derive_more::Debug;
use error_stack::Report;
use wherror::Error;

pub use xdg::XdgLauncher;

#[derive(Debug, Error)]
#[error(debug)]
pub struct LaunchError {
    pub stderr: Option<String>,
}

/// Result of a file launch operation.
///
/// Contains information about how the file was opened, including
/// whether the default system opener was used.
pub struct LaunchResult {
    pub used_default_opener: bool,
}

pub trait FileLauncher: Send + Sync {
    fn name(&self) -> &'static str;

    /// # Errors
    /// Returns an error if the file cannot be launched.
    fn launch(
        &self,
        path: &Path,
        command: Option<&str>,
        socket_path: &str,
    ) -> Result<LaunchResult, Report<LaunchError>>;
}

/// Service for launching files with external applications.
///
/// Provides an interface for opening files using either a configured
/// command or the system's default application handler. Delegates
/// to a backend implementation for actual file launching.
#[derive(Debug, Clone)]
pub struct FileLauncherService {
    #[debug("backend<{}>", self.backend.name())]
    backend: Arc<dyn FileLauncher>,
}

impl FileLauncherService {
    pub fn new(backend: Arc<dyn FileLauncher>) -> Self {
        Self { backend }
    }

    /// # Errors
    /// Returns an error if the file cannot be launched.
    pub fn launch(
        &self,
        path: &Path,
        command: Option<&str>,
        socket_path: &str,
    ) -> Result<LaunchResult, Report<LaunchError>> {
        self.backend.launch(path, command, socket_path)
    }
}
