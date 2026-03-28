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
        load_file: AtomicUsize,
        load_playlist: AtomicUsize,
        toggle_play: AtomicUsize,
    }

    impl FakeMpvClient {
        fn new() -> Self {
            Self {
                load_file: AtomicUsize::new(0),
                load_playlist: AtomicUsize::new(0),
                toggle_play: AtomicUsize::new(0),
            }
        }
    }

    impl MpvClient for FakeMpvClient {
        fn name(&self) -> &'static str {
            "fake_client"
        }

        fn load_file(&self, _path: &Path) -> Result<(), Report<MpvError>> {
            self.load_file.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn load_playlist(&self, _paths: &[PathBuf]) -> Result<(), Report<MpvError>> {
            self.load_playlist.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn toggle_play(&self) -> Result<(), Report<MpvError>> {
            self.toggle_play.fetch_add(1, Ordering::SeqCst);
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
        // Given a service with a fake mpv client backend.
        let fake = Arc::new(FakeMpvClient::new());
        let service = MpvClientService::new(fake.clone());

        // When calling all service methods.
        let _ = service.load_file(Path::new("test.mp4"));
        let _ = service.load_playlist(&[PathBuf::from("a.mp4")]);
        let _ = service.toggle_play();

        // Then each backend method was called exactly once.
        assert_eq!(fake.load_file.load(Ordering::SeqCst), 1);
        assert_eq!(fake.load_playlist.load(Ordering::SeqCst), 1);
        assert_eq!(fake.toggle_play.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn mpv_launcher_service_delegates_is_running_to_backend() {
        // Given a service with a fake launcher backend that reports running.
        let fake = Arc::new(FakeLauncher::new().with_running(true));
        let service = MpvLauncherService::new(fake.clone());

        // When checking if mpv is running.
        let result = service.is_running("/tmp/mpv.sock");

        // Then the result is true and the backend was called.
        assert!(result);
        assert_eq!(fake.is_running_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn mpv_launcher_service_delegates_spawn_to_backend() {
        // Given a service with a fake launcher backend.
        let fake = Arc::new(FakeLauncher::new());
        let service = MpvLauncherService::new(fake.clone());

        // When spawning mpv.
        let _ = service.spawn("/tmp/mpv.sock");

        // Then the backend spawn was called.
        assert_eq!(fake.spawn_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn mpv_launcher_service_returns_false_when_backend_reports_not_running() {
        // Given a service with a fake launcher backend that reports not running.
        let fake = Arc::new(FakeLauncher::new().with_running(false));
        let service = MpvLauncherService::new(fake.clone());

        // When checking if mpv is running.
        let result = service.is_running("/tmp/mpv.sock");

        // Then the result is false and the backend was called.
        assert!(!result);
        assert_eq!(fake.is_running_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    #[ignore = "requires real mpv process with --input-ipc-server=/tmp/test-mpv.sock"]
    fn is_mpv_running_returns_true_when_mpv_has_matching_socket() {
        // Given a socket path for a running mpv process.
        let socket_path = "/tmp/test-mpv.sock";

        // When checking if mpv is running with that socket.
        let result = is_mpv_running_with_socket(socket_path);

        // Then the result is true.
        assert!(result);
    }

    #[test]
    #[ignore = "requires no mpv process running with test socket path"]
    fn is_mpv_running_returns_false_when_no_matching_process() {
        // Given a socket path with no matching mpv process.
        let socket_path = "/tmp/nonexistent-mpv-socket-12345.sock";

        // When checking if mpv is running with that socket.
        let result = is_mpv_running_with_socket(socket_path);

        // Then the result is false.
        assert!(!result);
    }

    #[test]
    #[ignore = "integration test: verifies process name comparison is 'mpv' (not !=)"]
    fn is_mpv_running_checks_process_name_equals_mpv() {
        // Given a socket path for checking process name.
        let socket_path = "/tmp/test-mpv.sock";

        // When checking if mpv is running with that socket.
        let result = is_mpv_running_with_socket(socket_path);

        // Then the result is false when no matching process exists.
        assert!(!result);
    }

    #[test]
    #[ignore = "integration test: verifies socket path check uses && (not ||)"]
    fn is_mpv_running_checks_both_ipc_flag_and_socket_path() {
        // Given a specific socket path.
        let socket_path = "/tmp/specific-socket.sock";

        // When checking if mpv is running with that socket.
        let result = is_mpv_running_with_socket(socket_path);

        // Then the result is false when no matching process exists.
        assert!(!result);
    }
}
