use std::path::PathBuf;

use error_stack::{Report, ResultExt};

use crate::feat::create_symlink_with_suffix;

#[derive(Debug, wherror::Error)]
#[error(debug)]
pub struct SymlinkError;

pub fn create_symlinks_for_paths(paths: &[String]) -> Result<(), Report<SymlinkError>> {
    let cwd = std::env::current_dir().change_context(SymlinkError)?;

    for path in paths {
        let src = PathBuf::from(path);
        create_symlink_with_suffix(&src, &cwd)
            .map(|dest| eprintln!("Created symlink: {}", dest.display()))
            .change_context(SymlinkError)?;
    }

    Ok(())
}
