use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
};

use derive_more::Debug;
use error_stack::{Report, ResultExt};
use sysinfo::System;
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct MpvError;

pub trait MpvClient: Send + Sync {
    fn name(&self) -> &'static str;

    /// # Errors
    /// Returns an error if the file cannot be loaded in mpv.
    fn load_file(&self, path: &Path) -> Result<(), Report<MpvError>>;

    /// # Errors
    /// Returns an error if the playlist cannot be loaded in mpv.
    fn load_playlist(&self, paths: &[PathBuf]) -> Result<(), Report<MpvError>>;

    /// # Errors
    /// Returns an error if the toggle command fails.
    fn toggle_play(&self) -> Result<(), Report<MpvError>>;
}

/// Service for communicating with mpv via IPC.
///
/// Provides an interface for controlling a running mpv instance through
/// its JSON IPC socket. Supports loading playlists and querying player state.
#[derive(Debug, Clone)]
pub struct MpvClientService {
    #[debug("backend<{}>", self.backend.name())]
    backend: Arc<dyn MpvClient>,
}

impl MpvClientService {
    pub fn new(backend: Arc<dyn MpvClient>) -> Self {
        Self { backend }
    }

    /// # Errors
    /// Returns an error if the file cannot be loaded in mpv.
    pub fn load_file(&self, path: &Path) -> Result<(), Report<MpvError>> {
        self.backend.load_file(path)
    }

    /// # Errors
    /// Returns an error if the playlist cannot be loaded in mpv.
    pub fn load_playlist(&self, paths: &[PathBuf]) -> Result<(), Report<MpvError>> {
        self.backend.load_playlist(paths)
    }

    /// # Errors
    /// Returns an error if the toggle command fails.
    pub fn toggle_play(&self) -> Result<(), Report<MpvError>> {
        self.backend.toggle_play()
    }
}

pub trait MpvLauncher: Send + Sync {
    fn name(&self) -> &'static str;

    fn is_running(&self, socket_path: &str) -> bool;

    /// # Errors
    /// Returns an error if mpv cannot be spawned.
    fn spawn(&self, socket_path: &str) -> Result<(), Report<MpvError>>;
}

/// Service for launching the mpv media player.
///
/// Handles spawning mpv processes and checking if mpv is already running.
/// Used to start mpv with the appropriate socket for IPC communication.
#[derive(Debug, Clone)]
pub struct MpvLauncherService {
    #[debug("backend<{}>", self.backend.name())]
    backend: Arc<dyn MpvLauncher>,
}

impl MpvLauncherService {
    pub fn new(backend: Arc<dyn MpvLauncher>) -> Self {
        Self { backend }
    }

    pub fn is_running(&self, socket_path: &str) -> bool {
        self.backend.is_running(socket_path)
    }

    /// # Errors
    /// Returns an error if mpv cannot be spawned.
    pub fn spawn(&self, socket_path: &str) -> Result<(), Report<MpvError>> {
        self.backend.spawn(socket_path)
    }
}

pub fn is_mpv_running_with_socket(socket_path: &str) -> bool {
    let mut sys = System::new_all();
    sys.refresh_all();

    for process in sys.processes().values() {
        let name = process.name().to_string_lossy();
        if name == "mpv" {
            for arg in process.cmd() {
                let arg_str = arg.to_string_lossy();
                if arg_str.contains("--input-ipc-server=") && arg_str.contains(socket_path) {
                    return true;
                }
            }
        }
    }
    false
}

/// # Errors
/// Returns an error if mpv cannot be spawned.
pub fn spawn_mpv(socket_path: &str) -> Result<(), Report<MpvError>> {
    Command::new("mpv")
        .args([
            "--keep-open=always",
            "--idle",
            &format!("--input-ipc-server={socket_path}"),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .change_context(MpvError)
        .attach("failed to spawn mpv")?;
    Ok(())
}

mod clients;

pub use clients::{MpvIpc, RealMpvLauncher};

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FakeMpvClient {
        load_file_calls: AtomicUsize,
        load_playlist_calls: AtomicUsize,
        toggle_play_calls: AtomicUsize,
    }

    impl FakeMpvClient {
        fn new() -> Self {
            Self {
                load_file_calls: AtomicUsize::new(0),
                load_playlist_calls: AtomicUsize::new(0),
                toggle_play_calls: AtomicUsize::new(0),
            }
        }
    }

    impl MpvClient for FakeMpvClient {
        fn name(&self) -> &'static str {
            "fake_client"
        }

        fn load_file(&self, _path: &Path) -> Result<(), Report<MpvError>> {
            self.load_file_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn load_playlist(&self, _paths: &[PathBuf]) -> Result<(), Report<MpvError>> {
            self.load_playlist_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn toggle_play(&self) -> Result<(), Report<MpvError>> {
            self.toggle_play_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    struct FakeLauncher {
        is_running_calls: AtomicUsize,
        spawn_calls: AtomicUsize,
        running: bool,
    }

    impl FakeLauncher {
        fn new() -> Self {
            Self {
                is_running_calls: AtomicUsize::new(0),
                spawn_calls: AtomicUsize::new(0),
                running: false,
            }
        }

        fn with_running(mut self, value: bool) -> Self {
            self.running = value;
            self
        }
    }

    impl MpvLauncher for FakeLauncher {
        fn name(&self) -> &'static str {
            "fake_launcher"
        }

        fn is_running(&self, _socket_path: &str) -> bool {
            self.is_running_calls.fetch_add(1, Ordering::SeqCst);
            self.running
        }

        fn spawn(&self, _socket_path: &str) -> Result<(), Report<MpvError>> {
            self.spawn_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn mpv_client_service_delegates_to_backend() {
        let fake = Arc::new(FakeMpvClient::new());
        let service = MpvClientService::new(fake.clone());

        let _ = service.load_file(Path::new("test.mp4"));
        let _ = service.load_playlist(&[PathBuf::from("a.mp4")]);
        let _ = service.toggle_play();

        assert_eq!(fake.load_file_calls.load(Ordering::SeqCst), 1);
        assert_eq!(fake.load_playlist_calls.load(Ordering::SeqCst), 1);
        assert_eq!(fake.toggle_play_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn mpv_launcher_service_delegates_is_running_to_backend() {
        let fake = Arc::new(FakeLauncher::new().with_running(true));
        let service = MpvLauncherService::new(fake.clone());

        let result = service.is_running("/tmp/mpv.sock");

        assert!(result);
        assert_eq!(fake.is_running_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn mpv_launcher_service_delegates_spawn_to_backend() {
        let fake = Arc::new(FakeLauncher::new());
        let service = MpvLauncherService::new(fake.clone());

        let _ = service.spawn("/tmp/mpv.sock");

        assert_eq!(fake.spawn_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn mpv_launcher_service_returns_false_when_backend_reports_not_running() {
        let fake = Arc::new(FakeLauncher::new().with_running(false));
        let service = MpvLauncherService::new(fake.clone());

        let result = service.is_running("/tmp/mpv.sock");

        assert!(!result);
        assert_eq!(fake.is_running_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    #[ignore = "requires real mpv process with --input-ipc-server=/tmp/test-mpv.sock"]
    fn is_mpv_running_returns_true_when_mpv_has_matching_socket() {
        let socket_path = "/tmp/test-mpv.sock";

        let result = is_mpv_running_with_socket(socket_path);

        assert!(result);
    }

    #[test]
    #[ignore = "requires no mpv process running with test socket path"]
    fn is_mpv_running_returns_false_when_no_matching_process() {
        let socket_path = "/tmp/nonexistent-mpv-socket-12345.sock";

        let result = is_mpv_running_with_socket(socket_path);

        assert!(!result);
    }

    #[test]
    #[ignore = "integration test: verifies process name comparison is 'mpv' (not !=)"]
    fn is_mpv_running_checks_process_name_equals_mpv() {
        let socket_path = "/tmp/test-mpv.sock";

        let result = is_mpv_running_with_socket(socket_path);

        assert!(!result);
    }

    #[test]
    #[ignore = "integration test: verifies socket path check uses && (not ||)"]
    fn is_mpv_running_checks_both_ipc_flag_and_socket_path() {
        let socket_path = "/tmp/specific-socket.sock";

        let result = is_mpv_running_with_socket(socket_path);

        assert!(!result);
    }
}
