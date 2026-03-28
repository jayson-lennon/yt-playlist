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

use error_stack::Report;

use crate::feat::mpv::{is_mpv_running_with_socket, spawn_mpv, MpvError, MpvLauncher};

pub struct RealMpvLauncher;

impl MpvLauncher for RealMpvLauncher {
    fn name(&self) -> &'static str {
        "real"
    }

    fn is_running(&self, socket_path: &str) -> bool {
        is_mpv_running_with_socket(socket_path)
    }

    fn spawn(&self, socket_path: &str) -> Result<(), Report<MpvError>> {
        spawn_mpv(socket_path)
    }
}
