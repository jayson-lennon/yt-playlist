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
