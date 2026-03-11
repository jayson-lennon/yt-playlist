use std::{
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use error_stack::Report;

use super::super::{MediaError, MediaQuery};

pub struct FakeMediaBackend {
    pub call_count: AtomicUsize,
    duration: Duration,
}

impl FakeMediaBackend {
    pub fn new(duration: Duration) -> Self {
        Self {
            call_count: AtomicUsize::new(0),
            duration,
        }
    }
}

impl MediaQuery for FakeMediaBackend {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn get_duration(&self, _path: &Path) -> Result<Duration, Report<MediaError>> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(self.duration)
    }
}
