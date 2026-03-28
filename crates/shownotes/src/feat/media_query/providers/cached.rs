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

use std::{collections::HashMap, path::Path, sync::Arc, time::Duration};

use error_stack::Report;
use marked_path::CanonicalPath;

use super::super::{MediaError, MediaQuery};

pub struct CachedMedia {
    cache: HashMap<CanonicalPath, Duration>,
    fallback: Arc<dyn MediaQuery>,
}

impl CachedMedia {
    pub fn new(cache: HashMap<CanonicalPath, Duration>, fallback: Arc<dyn MediaQuery>) -> Self {
        Self { cache, fallback }
    }
}

impl MediaQuery for CachedMedia {
    fn name(&self) -> &'static str {
        "cached"
    }

    fn get_duration(&self, path: &Path) -> Result<Duration, Report<MediaError>> {
        if let Ok(canonical) = CanonicalPath::from_path(path) {
            if let Some(duration) = self.cache.get(&canonical) {
                return Ok(*duration);
            }
        }
        self.fallback.get_duration(path)
    }
}
