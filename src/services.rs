use derive_more::Debug;

use crate::launcher::LauncherService;
use crate::media::MediaQuery;
use crate::mpv::{MpvClient, MpvLauncherService};
use crate::playlist::PlaylistStorage;

#[derive(Debug, Clone)]
pub struct Services {
    pub mpv: MpvClient,
    pub media: MediaQuery,
    pub storage: PlaylistStorage,
    pub mpv_launcher: MpvLauncherService,
    pub file_launcher: LauncherService,
}
