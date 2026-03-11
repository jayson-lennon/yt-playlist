use std::sync::atomic::{AtomicUsize, Ordering};

use error_stack::Report;

use super::super::{IoError, PlaylistData, PlaylistStorage};

pub struct FakeStorageBackend {
    pub load_called: AtomicUsize,
    pub save_called: AtomicUsize,
}

impl FakeStorageBackend {
    pub fn new() -> Self {
        Self {
            load_called: AtomicUsize::new(0),
            save_called: AtomicUsize::new(0),
        }
    }
}

impl Default for FakeStorageBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaylistStorage for FakeStorageBackend {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn load(&self) -> Result<PlaylistData, Report<IoError>> {
        self.load_called.fetch_add(1, Ordering::SeqCst);
        Ok(PlaylistData::default())
    }

    fn save(&self, _data: &PlaylistData) -> Result<(), Report<IoError>> {
        self.save_called.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
