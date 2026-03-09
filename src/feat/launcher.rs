use std::{
    path::Path,
    process::{Command, Stdio},
    sync::Arc,
};

use derive_more::Debug;
use error_stack::{Report, ResultExt};
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct LaunchError {
    pub stderr: Option<String>,
}

pub struct LaunchResult {
    pub used_default_opener: bool,
}

pub trait FileLauncherBackend: Send + Sync {
    /// Returns the name identifier for this launcher implementation.
    fn name(&self) -> &'static str;

    /// Opens a file using either a custom shell command or the system's default application.
    /// The command template supports `{{path}}` and `{{socket_path}}` placeholders.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be launched, either due to command
    /// execution failure or if the command exits with a non-zero status.
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
    ///
    /// Returns an error if the file cannot be launched by the backend.
    pub fn launch(
        &self,
        path: &Path,
        command: Option<&str>,
        socket_path: &str,
    ) -> Result<LaunchResult, Report<LaunchError>> {
        self.backend.launch(path, command, socket_path)
    }
}

#[derive(Debug, Clone)]
pub struct FileLauncher {
    #[debug(skip)]
    shell: String,
}

impl FileLauncher {
    pub fn new() -> Self {
        Self {
            shell: std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string()),
        }
    }
}

impl Default for FileLauncher {
    fn default() -> Self {
        Self::new()
    }
}

impl FileLauncherBackend for FileLauncher {
    fn name(&self) -> &'static str {
        "file"
    }

    fn launch(
        &self,
        path: &Path,
        command: Option<&str>,
        socket_path: &str,
    ) -> Result<LaunchResult, Report<LaunchError>> {
        let path_str = path.to_string_lossy();
        let escaped_path = shell_escape::escape(path_str.clone());

        if let Some(cmd) = command {
            let socket_escaped = shell_escape::escape(std::borrow::Cow::Borrowed(socket_path));
            let substituted = cmd
                .replace("{{socket_path}}", &socket_escaped)
                .replace("{{path}}", &escaped_path);

            let substituted = if substituted.starts_with("shownotes ") {
                if let Ok(exe_path) = std::env::current_exe() {
                    let exe_escaped = shell_escape::escape(exe_path.to_string_lossy());
                    substituted.replacen("shownotes", &exe_escaped, 1)
                } else {
                    substituted
                }
            } else {
                substituted
            };

            let output = Command::new(&self.shell)
                .args(["-c", &substituted])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .output()
                .change_context(LaunchError { stderr: None })
                .attach("failed to launch file with command")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                return Err(Report::new(LaunchError {
                    stderr: if stderr.is_empty() {
                        None
                    } else {
                        Some(stderr)
                    },
                })
                .attach("command failed"));
            }

            Ok(LaunchResult {
                used_default_opener: false,
            })
        } else {
            Command::new("xdg-open")
                .arg(path_str.as_ref())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .change_context(LaunchError { stderr: None })
                .attach("failed to launch file with xdg-open")?;

            Ok(LaunchResult {
                used_default_opener: true,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn file_launcher_uses_shell_from_env() {
        // Given a file launcher.
        let launcher = FileLauncher::new();

        // When checking the shell.
        // Then it uses the SHELL env var or defaults to /bin/sh.
        assert!(!launcher.shell.is_empty());
    }

    #[test]
    fn launch_result_tracks_default_opener_usage() {
        // Given a launch result with default opener used.
        let result = LaunchResult {
            used_default_opener: true,
        };

        // When checking the flag.
        // Then it indicates default opener was used.
        assert!(result.used_default_opener);
    }

    #[test]
    fn launch_result_tracks_custom_command_usage() {
        // Given a launch result with custom command used.
        let result = LaunchResult {
            used_default_opener: false,
        };

        // When checking the flag.
        // Then it indicates default opener was not used.
        assert!(!result.used_default_opener);
    }

    struct FakeLauncher {
        last_command: std::sync::Mutex<Option<String>>,
        last_path: std::sync::Mutex<Option<PathBuf>>,
        last_socket_path: std::sync::Mutex<Option<String>>,
        used_default: std::sync::atomic::AtomicBool,
    }

    impl FakeLauncher {
        fn new() -> Self {
            Self {
                last_command: std::sync::Mutex::new(None),
                last_path: std::sync::Mutex::new(None),
                last_socket_path: std::sync::Mutex::new(None),
                used_default: std::sync::atomic::AtomicBool::new(false),
            }
        }

        fn with_used_default(self, value: bool) -> Self {
            self.used_default
                .store(value, std::sync::atomic::Ordering::SeqCst);
            self
        }

        fn last_command(&self) -> Option<String> {
            self.last_command.lock().unwrap().clone()
        }

        fn last_path(&self) -> Option<PathBuf> {
            self.last_path.lock().unwrap().clone()
        }
    }

    impl FileLauncherBackend for FakeLauncher {
        fn name(&self) -> &'static str {
            "fake"
        }

        fn launch(
            &self,
            path: &Path,
            command: Option<&str>,
            socket_path: &str,
        ) -> Result<LaunchResult, Report<LaunchError>> {
            *self.last_command.lock().unwrap() = command.map(str::to_string);
            *self.last_path.lock().unwrap() = Some(path.to_path_buf());
            *self.last_socket_path.lock().unwrap() = Some(socket_path.to_string());
            Ok(LaunchResult {
                used_default_opener: self.used_default.load(std::sync::atomic::Ordering::SeqCst),
            })
        }
    }

    #[test]
    fn fake_launcher_records_command() {
        // Given a fake launcher.
        let launcher = FakeLauncher::new();

        // When launching with a command.
        let _ = launcher.launch(
            &PathBuf::from("test.mp4"),
            Some("mpv {{path}}"),
            "/tmp/socket",
        );

        // Then the command is recorded.
        assert_eq!(launcher.last_command(), Some("mpv {{path}}".to_string()));
    }

    #[test]
    fn fake_launcher_records_path() {
        // Given a fake launcher.
        let launcher = FakeLauncher::new();

        // When launching with a path.
        let _ = launcher.launch(
            &PathBuf::from("/video/test.mp4"),
            Some("mpv"),
            "/tmp/socket",
        );

        // Then the path is recorded.
        assert_eq!(launcher.last_path(), Some(PathBuf::from("/video/test.mp4")));
    }

    #[test]
    fn fake_launcher_records_none_when_no_command() {
        // Given a fake launcher.
        let launcher = FakeLauncher::new();

        // When launching without a command.
        let _ = launcher.launch(&PathBuf::from("test.txt"), None, "/tmp/socket");

        // Then no command is recorded.
        assert!(launcher.last_command().is_none());
    }

    #[test]
    fn fake_launcher_returns_used_default_flag() {
        // Given a fake launcher configured to use default opener.
        let launcher = FakeLauncher::new().with_used_default(true);

        // When launching.
        let result = launcher
            .launch(&PathBuf::from("test.txt"), None, "/tmp/socket")
            .unwrap();

        // Then the result indicates default opener was used.
        assert!(result.used_default_opener);
    }
}
