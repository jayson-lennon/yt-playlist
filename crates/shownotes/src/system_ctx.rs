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
