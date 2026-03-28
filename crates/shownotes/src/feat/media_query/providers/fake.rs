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
