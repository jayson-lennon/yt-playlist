mod file;

use std::{path::Path, sync::Arc};

use derive_more::Debug;
use error_stack::Report;
use wherror::Error;

pub use file::FileLauncher;

#[derive(Debug, Error)]
#[error(debug)]
pub struct LaunchError {
    pub stderr: Option<String>,
}

pub struct LaunchResult {
    pub used_default_opener: bool,
}

pub trait FileLauncherBackend: Send + Sync {
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

#[derive(Debug, Clone)]
pub struct FileLauncherService {
    #[debug("backend<{}>", self.backend.name())]
    backend: Arc<dyn FileLauncherBackend>,
}

impl FileLauncherService {
    pub fn new(backend: Arc<dyn FileLauncherBackend>) -> Self {
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
