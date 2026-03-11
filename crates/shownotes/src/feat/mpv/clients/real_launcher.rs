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
