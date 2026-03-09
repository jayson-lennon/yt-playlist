use std::{os::unix::fs as unix_fs, path::Path};

use error_stack::{Report, ResultExt};

use super::RunError;

/// Creates a symlink to the target in the destination directory.
///
/// If a file with the same name already exists, appends a numeric suffix.
///
/// # Errors
///
/// Returns an error if:
/// - The target has no file name
/// - The target has no file stem (when suffixing is needed)
/// - The symlink cannot be created
pub fn create_symlink_with_suffix(
    target: &Path,
    dest_dir: &Path,
) -> Result<std::path::PathBuf, Report<RunError>> {
    let basename = target.file_name().ok_or_else(|| Report::new(RunError))?;

    let mut dest_path = dest_dir.join(basename);
    let mut suffix = 0;

    while dest_path.exists() || dest_path.symlink_metadata().is_ok() {
        suffix += 1;
        let stem = target.file_stem().ok_or_else(|| Report::new(RunError))?;
        let new_name = if let Some(ext) = target.extension() {
            format!(
                "{}_{}.{}",
                stem.to_string_lossy(),
                suffix,
                ext.to_string_lossy()
            )
        } else {
            format!("{}_{}", stem.to_string_lossy(), suffix)
        };
        dest_path = dest_dir.join(new_name);
    }

    unix_fs::symlink(target, &dest_path).change_context(RunError)?;
    Ok(dest_path)
}
