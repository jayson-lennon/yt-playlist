use error_stack::{Report, ResultExt};

use super::CommandError;
use crate::feat::mpv::MpvIpc;
use crate::feat::MpvClient;

pub fn load(socket: &std::path::Path, path: &std::path::Path) -> Result<(), Report<CommandError>> {
    let backend = MpvIpc::new(socket);
    backend.load_file(path).change_context(CommandError)
}
