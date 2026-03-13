use error_stack::Report;
use marked_path::CanonicalPath;

use crate::feat::config::Config;
use crate::feat::keymap::Keymap;
use crate::feat::note_db::SqliteNoteDbError;
use crate::services::Services;

#[derive(Debug, Clone)]
pub struct SystemCtx {
    pub services: Services,
    pub config: Config,
    pub library_path: CanonicalPath,
    pub socket_path: String,
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
