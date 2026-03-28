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

#![allow(unused_imports)]
pub mod context;
pub mod fakes;
pub mod fixtures;
pub mod services;

pub use context::{NoteTestContext, NoteTestContextBuilder};
pub use fakes::{FakeFuzzySearch, FakeLauncher, FakeMediaBackend, FakeMpvBackend, FakeMpvLauncher};
pub use fixtures::create_temp_file;
pub use services::create_test_services;
