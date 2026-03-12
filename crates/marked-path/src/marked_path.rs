use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use error_stack::Report;
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct PathError;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Absolute;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Relative;

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

impl<M> MarkedPath<M> {
    pub fn as_path(&self) -> &Path {
        &self.path
    }

    pub fn to_path_buf(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn into_inner(self) -> PathBuf {
        self.path
    }
}

impl MarkedPath<Absolute> {
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

    pub fn canonicalize(&self) -> Result<CanonicalPath, Report<PathError>> {
        let canonicalized = self
            .path
            .canonicalize()
            .map_err(|_| Report::new(PathError))?;
        Ok(CanonicalPath::new(canonicalized))
    }

    pub fn push_path(&mut self, other: &MarkedPath<Relative>) {
        self.path.push(&other.path);
    }
}

impl MarkedPath<Relative> {
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

    pub fn push_path(&mut self, other: &MarkedPath<Relative>) {
        self.path.push(&other.path);
    }
}

#[derive(Debug, PartialOrd, Ord)]
pub struct CanonicalPath(MarkedPath<Absolute>);

impl Clone for CanonicalPath {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl CanonicalPath {
    pub fn new(path: PathBuf) -> Self {
        Self(MarkedPath {
            path,
            _marker: PhantomData,
        })
    }

    pub fn from_path(path: &Path) -> Result<Self, Report<PathError>> {
        let canonicalized = path.canonicalize().map_err(|_| Report::new(PathError))?;
        Ok(Self::new(canonicalized))
    }

    pub fn as_path(&self) -> &Path {
        self.0.as_path()
    }

    pub fn to_path_buf(&self) -> PathBuf {
        self.0.to_path_buf()
    }

    pub fn into_inner(self) -> PathBuf {
        self.0.into_inner()
    }

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
        let path = if cfg!(windows) {
            PathBuf::from("C:\\some\\path")
        } else {
            PathBuf::from("/some/path")
        };
        let result = MarkedPath::<Absolute>::new(path);
        assert!(result.is_ok());
    }

    #[rstest]
    fn absolute_new_rejects_relative_path() {
        let path = PathBuf::from("some/relative/path");
        let result = MarkedPath::<Absolute>::new(path);
        assert!(result.is_err());
    }

    #[rstest]
    fn relative_new_accepts_relative_path() {
        let path = PathBuf::from("some/relative/path");
        let result = MarkedPath::<Relative>::new(path);
        assert!(result.is_ok());
    }

    #[rstest]
    fn relative_new_rejects_absolute_path() {
        let path = if cfg!(windows) {
            PathBuf::from("C:\\some\\path")
        } else {
            PathBuf::from("/some/path")
        };
        let result = MarkedPath::<Relative>::new(path);
        assert!(result.is_err());
    }

    #[rstest]
    fn push_path_on_absolute_accepts_relative() {
        let base_path = if cfg!(windows) {
            PathBuf::from("C:\\base")
        } else {
            PathBuf::from("/base")
        };
        let mut absolute = MarkedPath::<Absolute>::new(base_path).unwrap();
        let relative = MarkedPath::<Relative>::new(PathBuf::from("subdir/file.txt")).unwrap();
        absolute.push_path(&relative);
        let expected = if cfg!(windows) {
            "C:\\base\\subdir\\file.txt"
        } else {
            "/base/subdir/file.txt"
        };
        assert_eq!(absolute.as_path(), Path::new(expected));
    }

    #[rstest]
    fn push_path_on_relative_accepts_relative() {
        let mut base = MarkedPath::<Relative>::new(PathBuf::from("base")).unwrap();
        let other = MarkedPath::<Relative>::new(PathBuf::from("subdir/file.txt")).unwrap();
        base.push_path(&other);
        assert_eq!(base.as_path(), Path::new("base/subdir/file.txt"));
    }

    #[rstest]
    fn canonical_path_from_existing_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let canonical = CanonicalPath::from_path(path);
        assert!(canonical.is_ok());
        let canonical = canonical.unwrap();
        assert!(canonical.as_path().is_absolute());
    }

    #[rstest]
    fn canonical_path_hash_and_eq() {
        use std::collections::HashSet;

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let canonical1 = CanonicalPath::from_path(path).unwrap();
        let canonical2 = CanonicalPath::from_path(path).unwrap();

        assert_eq!(canonical1, canonical2);

        let mut set = HashSet::new();
        set.insert(canonical1.clone());
        assert!(set.contains(&canonical2));
    }
}
