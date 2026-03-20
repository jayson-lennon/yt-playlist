use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use error_stack::{Report, ResultExt};
use wherror::Error;

/// Error type for path operations.
///
/// This error is returned when a path operation fails, such as attempting to
/// create a `MarkedPath<Absolute>` from a relative path, or vice versa.
#[derive(Debug, Error)]
#[error(debug)]
pub struct PathError;

/// Marker type for absolute paths.
///
/// This is a phantom marker type used with [`MarkedPath`] to indicate that
/// the contained path is guaranteed to be absolute. An absolute path starts
/// from the root of the filesystem (e.g., `/path/to/file` on Unix or
/// `C:\path\to\file` on Windows).
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Absolute;

/// Marker type for relative paths.
///
/// This is a phantom marker type used with [`MarkedPath`] to indicate that
/// the contained path is guaranteed to be relative. A relative path does not
/// start from the root of the filesystem (e.g., `path/to/file`).
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Relative;

/// A type-safe path wrapper with an absolute/relative marker.
///
/// This struct provides compile-time guarantees about whether a path is
/// absolute or relative through its generic parameter `M`. The marker type
/// ensures that absolute and relative paths cannot be accidentally mixed.
///
/// # Type Parameters
///
/// * `M` - A marker type indicating the path's nature:
///   - [`Absolute`]: The path is guaranteed to be absolute
///   - [`Relative`]: The path is guaranteed to be relative
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use marked_path::{MarkedPath, Absolute, Relative};
///
/// // Create an absolute path (validated at construction)
/// let abs = MarkedPath::<Absolute>::new(PathBuf::from("/home/user"))?;
///
/// // Create a relative path
/// let rel = MarkedPath::<Relative>::new(PathBuf::from("documents/file.txt"))?;
///
/// // You can push relative paths onto absolute paths
/// let mut abs = MarkedPath::<Absolute>::new(PathBuf::from("/home"))?;
/// abs.push_path(&rel);
/// # Ok::<(), error_stack::Report<marked_path::PathError>>(())
/// ```
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MarkedPath<M> {
    path: PathBuf,
    _marker: PhantomData<M>,
}

impl<M> Clone for MarkedPath<M> {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            _marker: PhantomData,
        }
    }
}

impl<M> fmt::Display for MarkedPath<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.path.display().fmt(f)
    }
}

impl<M> AsRef<Path> for MarkedPath<M> {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl FromStr for MarkedPath<Absolute> {
    type Err = Report<PathError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let path = PathBuf::from(s);
        Self::new(path)
    }
}

impl FromStr for MarkedPath<Relative> {
    type Err = Report<PathError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let path = PathBuf::from(s);
        Self::new(path)
    }
}

impl From<CanonicalPath> for MarkedPath<Absolute> {
    fn from(value: CanonicalPath) -> Self {
        value.0
    }
}

impl<M> MarkedPath<M> {
    /// Returns a reference to the underlying [`Path`].
    pub fn as_path(&self) -> &Path {
        &self.path
    }

    /// Returns a clone of the underlying [`PathBuf`].
    pub fn to_path_buf(&self) -> PathBuf {
        self.path.clone()
    }

    /// Consumes this `MarkedPath` and returns the underlying [`PathBuf`].
    pub fn into_inner(self) -> PathBuf {
        self.path
    }
}

impl MarkedPath<Absolute> {
    /// Creates a new `MarkedPath<Absolute>` from the given path.
    ///
    /// # Errors
    ///
    /// Returns a [`PathError`] if the path is not absolute.
    pub fn new(path: PathBuf) -> Result<Self, Report<PathError>> {
        if path.is_absolute() {
            Ok(Self {
                path,
                _marker: PhantomData,
            })
        } else {
            Err(Report::new(PathError))
        }
    }

    /// Canonicalizes this absolute path, returning a [`CanonicalPath`].
    ///
    /// # Errors
    ///
    /// Returns a [`PathError`] if the path cannot be canonicalized
    /// (e.g., if it doesn't exist or there are permission issues).
    pub fn canonicalize(&self) -> Result<CanonicalPath, Report<PathError>> {
        let canonicalized = self.path.canonicalize().change_context(PathError)?;
        CanonicalPath::new(canonicalized)
    }

    /// Appends a relative path to this absolute path.
    pub fn push_path(&mut self, other: &MarkedPath<Relative>) {
        self.path.push(&other.path);
    }
}

impl MarkedPath<Relative> {
    /// Creates a new `MarkedPath<Relative>` from the given path.
    ///
    /// # Errors
    ///
    /// Returns a [`PathError`] if the path is not relative (i.e., if it's absolute).
    pub fn new(path: PathBuf) -> Result<Self, Report<PathError>> {
        if path.is_relative() {
            Ok(Self {
                path,
                _marker: PhantomData,
            })
        } else {
            Err(Report::new(PathError))
        }
    }

    /// Appends another relative path to this relative path.
    pub fn push_path(&mut self, other: &MarkedPath<Relative>) {
        self.path.push(&other.path);
    }
}

/// A wrapper for canonicalized absolute paths.
///
/// This type represents a path that has been resolved to its canonical form:
/// it is guaranteed to be absolute, with all `.` and `..` components resolved,
/// and all symbolic links followed. The path must exist on the filesystem
/// at the time of construction.
///
/// # Type Safety
///
/// A `CanonicalPath` provides stronger guarantees than [`MarkedPath<Absolute>`]:
/// - The path is absolute and fully resolved
/// - The path existed at construction time
/// - The path can be safely used for comparisons (no `.` or `..` ambiguity)
///
/// # Example
///
/// ```
/// use std::path::Path;
/// use marked_path::CanonicalPath;
///
/// // Create from an existing path
/// let canonical = CanonicalPath::from_path(Path::new("/etc/hosts"))?;
/// println!("Canonical path: {}", canonical.as_path().display());
/// # Ok::<(), error_stack::Report<marked_path::PathError>>(())
/// ```
#[derive(Debug, PartialOrd, Ord)]
pub struct CanonicalPath(MarkedPath<Absolute>);

impl Clone for CanonicalPath {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl CanonicalPath {
    /// Creates a new `CanonicalPath` from a path, validating it is canonical.
    ///
    /// # Errors
    ///
    /// Returns a [`PathError`] if:
    /// - The path does not exist
    /// - The path is not in canonical form (contains `.`, `..`, or is a symlink)
    pub fn new(path: PathBuf) -> Result<Self, Report<PathError>> {
        let canonicalized = path.canonicalize().change_context(PathError)?;
        if canonicalized != path {
            return Err(Report::new(PathError).attach("path is not in canonical form"));
        }
        Ok(Self(MarkedPath {
            path,
            _marker: PhantomData,
        }))
    }

    /// Creates a `CanonicalPath` by canonicalizing the given path.
    ///
    /// # Errors
    ///
    /// Returns a [`PathError`] if the path cannot be canonicalized
    /// (e.g., if it doesn't exist or if there are permission issues).
    pub fn from_path(path: &Path) -> Result<Self, Report<PathError>> {
        let canonicalized = path.canonicalize().change_context(PathError)?;
        CanonicalPath::new(canonicalized)
    }

    /// Returns a reference to the underlying [`Path`].
    pub fn as_path(&self) -> &Path {
        self.0.as_path()
    }

    /// Returns a clone of the underlying [`PathBuf`].
    pub fn to_path_buf(&self) -> PathBuf {
        self.0.to_path_buf()
    }

    /// Consumes this `CanonicalPath` and returns the underlying [`PathBuf`].
    pub fn into_inner(self) -> PathBuf {
        self.0.into_inner()
    }

    /// Appends a relative path to this canonical path.
    ///
    /// Note: After calling this method, the path may no longer be canonical
    /// (it could contain `.` or `..` components).
    pub fn push_path(&mut self, other: &MarkedPath<Relative>) {
        self.0.push_path(other);
    }
}

impl PartialEq for CanonicalPath {
    fn eq(&self, other: &Self) -> bool {
        self.0.path == other.0.path
    }
}

impl Eq for CanonicalPath {}

impl Hash for CanonicalPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.path.hash(state);
    }
}

impl AsRef<Path> for CanonicalPath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl From<CanonicalPath> for PathBuf {
    fn from(value: CanonicalPath) -> Self {
        value.into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use tempfile::NamedTempFile;

    #[rstest]
    fn absolute_new_accepts_absolute_path() {
        // Given an absolute path.
        let path = if cfg!(windows) {
            PathBuf::from("C:\\some\\path")
        } else {
            PathBuf::from("/some/path")
        };

        // When creating a marked path.
        let result = MarkedPath::<Absolute>::new(path);

        // Then the result is ok.
        assert!(result.is_ok());
    }

    #[rstest]
    fn absolute_new_rejects_relative_path() {
        // Given a relative path.
        let path = PathBuf::from("some/relative/path");

        // When creating an absolute marked path.
        let result = MarkedPath::<Absolute>::new(path);

        // Then the result is an error.
        assert!(result.is_err());
    }

    #[rstest]
    fn relative_new_accepts_relative_path() {
        // Given a relative path.
        let path = PathBuf::from("some/relative/path");

        // When creating a relative marked path.
        let result = MarkedPath::<Relative>::new(path);

        // Then the result is ok.
        assert!(result.is_ok());
    }

    #[rstest]
    fn relative_new_rejects_absolute_path() {
        // Given an absolute path.
        let path = if cfg!(windows) {
            PathBuf::from("C:\\some\\path")
        } else {
            PathBuf::from("/some/path")
        };

        // When creating a relative marked path.
        let result = MarkedPath::<Relative>::new(path);

        // Then the result is an error.
        assert!(result.is_err());
    }

    #[rstest]
    fn push_path_on_absolute_accepts_relative() {
        // Given an absolute marked path and a relative marked path.
        let base_path = if cfg!(windows) {
            PathBuf::from("C:\\base")
        } else {
            PathBuf::from("/base")
        };
        let mut absolute = MarkedPath::<Absolute>::new(base_path).unwrap();
        let relative = MarkedPath::<Relative>::new(PathBuf::from("subdir/file.txt")).unwrap();

        // When pushing the relative path onto the absolute path.
        absolute.push_path(&relative);

        // Then the path is the combined result.
        let expected = if cfg!(windows) {
            "C:\\base\\subdir\\file.txt"
        } else {
            "/base/subdir/file.txt"
        };
        assert_eq!(absolute.as_path(), Path::new(expected));
    }

    #[rstest]
    fn push_path_on_relative_accepts_relative() {
        // Given two relative marked paths.
        let mut base = MarkedPath::<Relative>::new(PathBuf::from("base")).unwrap();
        let other = MarkedPath::<Relative>::new(PathBuf::from("subdir/file.txt")).unwrap();

        // When pushing one path onto the other.
        base.push_path(&other);

        // Then the path is the combined result.
        assert_eq!(base.as_path(), Path::new("base/subdir/file.txt"));
    }

    #[rstest]
    fn canonical_path_from_existing_file() {
        // Given an existing file.
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // When creating a canonical path from the file path.
        let canonical = CanonicalPath::from_path(path);

        // Then the result is ok and the path is absolute.
        assert!(canonical.is_ok());
        let canonical = canonical.unwrap();
        assert!(canonical.as_path().is_absolute());
    }

    #[rstest]
    fn canonical_path_hash_and_eq() {
        // Given two canonical paths to the same file.
        use std::collections::HashSet;

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let canonical1 = CanonicalPath::from_path(path).unwrap();
        let canonical2 = CanonicalPath::from_path(path).unwrap();

        // When comparing them and using in a HashSet.
        assert_eq!(canonical1, canonical2);

        let mut set = HashSet::new();
        set.insert(canonical1.clone());

        // Then they are equal and both hash to the same value.
        assert!(set.contains(&canonical2));
    }
}
