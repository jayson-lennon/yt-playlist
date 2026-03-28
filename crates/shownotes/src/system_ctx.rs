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
use marked_path::CanonicalPath;

use crate::feat::config::Config;
use crate::feat::keymap::Keymap;
use crate::feat::note_db::SqliteNoteDbError;
use crate::services::Services;

/// Main system context container holding all dependencies.
///
/// Provides centralized access to services, configuration, and paths
/// needed throughout the application.
#[derive(Debug, Clone)]
pub struct SystemCtx {
    /// Service dependencies for database, media, and external integrations.
    pub services: Services,
    /// User configuration settings.
    pub config: Config,
    /// Path to the media library directory.
    pub library_path: CanonicalPath,
    /// Path to the mpv IPC socket.
    pub socket_path: String,
    /// Key mappings for keyboard shortcuts.
    pub keymap: Keymap,
}

impl SystemCtx {
    /// Creates a new system context with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the database initialization fails.
    pub async fn new(
        db_path: &str,
        config: Config,
        library_path: CanonicalPath,
        socket_path: String,
        rt: tokio::runtime::Handle,
    ) -> Result<Self, Report<SqliteNoteDbError>> {
        let services = Services::new(db_path, rt).await?;
        Ok(Self {
            services,
            config,
            library_path,
            socket_path,
            keymap: Keymap::new(),
        })
    }
}
