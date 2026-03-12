use std::{path::Path, time::Duration};

use async_trait::async_trait;
use error_stack::Report;
use marked_path::CanonicalPath;
use std::path::PathBuf;

use crate::feat::launcher::{FileLauncher, LaunchResult};
use crate::feat::media_query::{MediaError, MediaQuery};
use crate::feat::mpv::{MpvClient, MpvError, MpvLauncher};
use crate::feat::playlist::{IoError, PlaylistData, PlaylistStorage};

pub struct FakeMpvBackend;

impl MpvClient for FakeMpvBackend {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn load_file(&self, _path: &Path) -> Result<(), Report<MpvError>> {
        Ok(())
    }

    fn load_playlist(&self, _paths: &[PathBuf]) -> Result<(), Report<MpvError>> {
        Ok(())
    }
}

pub struct FakeMpvLauncher {
    running: bool,
}

impl FakeMpvLauncher {
    pub fn new() -> Self {
        Self { running: false }
    }

    pub fn running(mut self, value: bool) -> Self {
        self.running = value;
        self
    }
}

impl Default for FakeMpvLauncher {
    fn default() -> Self {
        Self::new()
    }
}

impl MpvLauncher for FakeMpvLauncher {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn is_running(&self, _socket_path: &str) -> bool {
        self.running
    }

    fn spawn(&self, _socket_path: &str) -> Result<(), Report<MpvError>> {
        Ok(())
    }
}

pub struct FakeMediaBackend;

impl MediaQuery for FakeMediaBackend {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn get_duration(&self, _path: &Path) -> Result<Duration, Report<MediaError>> {
        Ok(Duration::from_secs(120))
    }
}

pub struct FakeStorageBackend;

#[async_trait]
impl PlaylistStorage for FakeStorageBackend {
    fn name(&self) -> &'static str {
        "fake"
    }

    async fn load(&self, working_directory: &CanonicalPath) -> Result<PlaylistData, Report<IoError>> {
        Ok(PlaylistData {
            working_directory: working_directory.clone(),
            playlist: Vec::new(),
            files: std::collections::HashMap::new(),
        })
    }

    async fn save(&self, _data: &PlaylistData) -> Result<(), Report<IoError>> {
        Ok(())
    }
}

pub struct FakeLauncher;

impl FileLauncher for FakeLauncher {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn launch(
        &self,
        _path: &Path,
        _command: Option<&str>,
        _socket_path: &str,
    ) -> Result<LaunchResult, Report<crate::feat::launcher::LaunchError>> {
        Ok(LaunchResult {
            used_default_opener: false,
        })
    }
}
