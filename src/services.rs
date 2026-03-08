use std::sync::Arc;

use crate::launcher::Launcher;
use crate::media::MediaQuery;
use crate::mpv::{MpvClient, MpvLauncher};
use crate::playlist::PlaylistStorage;

#[derive(Clone)]
pub struct Services {
    pub mpv: MpvClient,
    pub media: MediaQuery,
    pub storage: PlaylistStorage,
    pub mpv_launcher: Arc<dyn MpvLauncher>,
    pub file_launcher: Arc<dyn Launcher>,
}
