use derive_more::Debug;

use crate::feat::launcher::LauncherService;
use crate::feat::media_query::MediaQuery;
use crate::feat::mpv::{MpvClient, MpvLauncherService};
use crate::notes::SystemServicesHandle;
use crate::playlist::PlaylistStorage;

#[derive(Debug, Clone)]
pub struct Services {
    pub mpv: MpvClient,
    pub media: MediaQuery,
    pub storage: PlaylistStorage,
    pub mpv_launcher: MpvLauncherService,
    pub file_launcher: LauncherService,
    pub notes: SystemServicesHandle,
}
