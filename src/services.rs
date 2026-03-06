use crate::media::MediaQuery;
use crate::mpv::MpvClient;
use crate::playlist::PlaylistStorage;

#[derive(Debug, Clone)]
pub struct Services {
    pub mpv: MpvClient,
    pub media: MediaQuery,
    pub storage: PlaylistStorage,
}
