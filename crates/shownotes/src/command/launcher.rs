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

use error_stack::{Report, ResultExt};
use marked_path::CanonicalPath;

use super::CommandError;
use crate::feat::launcher::LaunchResult;
use crate::system_ctx::SystemCtx;

pub fn launch(
    ctx: &SystemCtx,
    path: &CanonicalPath,
    command: Option<&str>,
    socket_path: &str,
) -> Result<LaunchResult, Report<CommandError>> {
    ctx.services
        .file_launcher
        .launch(path.as_path(), command, socket_path)
        .change_context(CommandError)
        .attach("failed to launch file")
}
