use std::sync::{Arc, Mutex};
use std::{path::{Path, PathBuf}, time::Duration};

use async_trait::async_trait;
use error_stack::Report;
use marked_path::CanonicalPath;

use crate::feat::fuzzy_search::{FuzzySearch, FuzzySearchError, FuzzySearchResult};
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

    fn toggle_play(&self) -> Result<(), Report<MpvError>> {
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

/// Simple stub launcher for tests.
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

/// Simple stub storage backend for tests.
///
/// This is a minimal implementation that returns empty data.
/// For tests that need more control over storage behavior,
/// use [`crate::feat::playlist::FakeStorageBackend`] instead.
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

    async fn upsert_alias(
        &self,
        _file_path: &CanonicalPath,
        _workspace: &CanonicalPath,
        _alias: &str,
    ) -> Result<(), Report<IoError>> {
        Ok(())
    }

    async fn delete_alias(
        &self,
        _file_path: &CanonicalPath,
        _workspace: &CanonicalPath,
    ) -> Result<(), Report<IoError>> {
        Ok(())
    }

    async fn resolve_alias(
        &self,
        _file_path: &CanonicalPath,
        _workspace: &CanonicalPath,
    ) -> Result<Option<String>, Report<IoError>> {
        Ok(None)
    }

    async fn get_path_counts(&self) -> Result<std::collections::HashMap<i64, usize>, Report<IoError>> {
        Ok(std::collections::HashMap::new())
    }

    async fn resolve_file_path_id(&self, _path: &crate::common::domain::ItemPath) -> Result<Option<i64>, Report<IoError>> {
        Ok(None)
    }
}

pub struct FakeFuzzySearch {
    selected_paths: Arc<Mutex<Vec<String>>>,
}

impl FakeFuzzySearch {
    pub fn new() -> Self {
        Self {
            selected_paths: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn set_selected_paths(&self, paths: Vec<String>) {
        let mut guard = self.selected_paths.lock().unwrap();
        *guard = paths;
    }
}

impl Default for FakeFuzzySearch {
    fn default() -> Self {
        Self::new()
    }
}

impl FuzzySearch for FakeFuzzySearch {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn search(
        &self,
        _items: &[(String, String)],
    ) -> Result<FuzzySearchResult, Report<FuzzySearchError>> {
        let guard = self.selected_paths.lock().unwrap();
        Ok(FuzzySearchResult {
            selected_paths: guard.clone(),
        })
    }
}
