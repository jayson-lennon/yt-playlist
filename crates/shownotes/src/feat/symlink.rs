use std::{os::unix::fs as unix_fs, path::Path};

use error_stack::{Report, ResultExt};
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct SymlinkError;

pub type SymlinkResult = Result<std::path::PathBuf, Report<SymlinkError>>;

/// Creates a symlink to `target` in `dest_dir`, adding a numeric suffix if the destination exists.
///
/// # Errors
///
/// Returns `SymlinkError` if:
/// - The target has no file name
/// - The target has no file stem (for suffixing)
/// - The symlink creation fails
pub fn create_symlink_with_suffix(target: &Path, dest_dir: &Path) -> SymlinkResult {
    let basename = target
        .file_name()
        .ok_or_else(|| Report::new(SymlinkError))?;

    let mut dest_path = dest_dir.join(basename);
    let mut suffix = 0;

    while dest_path.exists() || dest_path.symlink_metadata().is_ok() {
        suffix += 1;
        let stem = target
            .file_stem()
            .ok_or_else(|| Report::new(SymlinkError))?;
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

    unix_fs::symlink(target, &dest_path).change_context(SymlinkError)?;
    Ok(dest_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn creates_symlink_with_original_name() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("video.mp4");
        std::fs::write(&target, "content").unwrap();

        let dest_dir = TempDir::new().unwrap();
        let result = create_symlink_with_suffix(&target, dest_dir.path());

        assert!(result.is_ok());
        let link = result.unwrap();
        assert_eq!(link.file_name().unwrap().to_str().unwrap(), "video.mp4");
    }

    #[test]
    fn appends_suffix_when_file_exists() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("video.mp4");
        std::fs::write(&target, "content").unwrap();

        let dest_dir = TempDir::new().unwrap();
        std::fs::write(dest_dir.path().join("video.mp4"), "existing").unwrap();

        let result = create_symlink_with_suffix(&target, dest_dir.path());

        assert!(result.is_ok());
        let link = result.unwrap();
        assert_eq!(link.file_name().unwrap().to_str().unwrap(), "video_1.mp4");
    }

    #[test]
    fn appends_incrementing_suffix() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("video.mp4");
        std::fs::write(&target, "content").unwrap();

        let dest_dir = TempDir::new().unwrap();
        std::fs::write(dest_dir.path().join("video.mp4"), "existing").unwrap();
        std::fs::write(dest_dir.path().join("video_1.mp4"), "existing1").unwrap();

        let result = create_symlink_with_suffix(&target, dest_dir.path());

        assert!(result.is_ok());
        let link = result.unwrap();
        assert_eq!(link.file_name().unwrap().to_str().unwrap(), "video_2.mp4");
    }

    #[test]
    fn handles_files_without_extension() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("README");
        std::fs::write(&target, "content").unwrap();

        let dest_dir = TempDir::new().unwrap();
        std::fs::write(dest_dir.path().join("README"), "existing").unwrap();

        let result = create_symlink_with_suffix(&target, dest_dir.path());

        assert!(result.is_ok());
        let link = result.unwrap();
        assert_eq!(link.file_name().unwrap().to_str().unwrap(), "README_1");
    }

    #[test]
    fn fails_when_target_has_no_filename() {
        let dest_dir = TempDir::new().unwrap();
        let result = create_symlink_with_suffix(Path::new("/"), dest_dir.path());

        assert!(result.is_err());
    }
}
