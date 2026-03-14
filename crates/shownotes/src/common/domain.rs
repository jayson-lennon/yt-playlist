use std::{borrow::Cow, time::Duration};

use marked_path::CanonicalPath;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ItemPath {
    File(CanonicalPath),
    Url(String),
}

impl ItemPath {
    pub fn as_file(&self) -> Option<&CanonicalPath> {
        match self {
            ItemPath::File(path) => Some(path),
            ItemPath::Url(_) => None,
        }
    }

    pub fn as_url(&self) -> Option<&str> {
        match self {
            ItemPath::File(_) => None,
            ItemPath::Url(url) => Some(url),
        }
    }

    pub fn is_url(&self) -> bool {
        matches!(self, ItemPath::Url(_))
    }

    pub fn display(&self) -> std::path::Display<'_> {
        match self {
            ItemPath::File(path) => path.as_path().display(),
            ItemPath::Url(url) => std::path::Path::new(url).display(),
        }
    }

    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        match self {
            ItemPath::File(path) => path.as_path().to_string_lossy(),
            ItemPath::Url(url) => Cow::Borrowed(url),
        }
    }

    pub fn file_stem(&self) -> Option<&str> {
        match self {
            ItemPath::File(path) => path.as_path().file_stem().and_then(|s| s.to_str()),
            ItemPath::Url(_) => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlaylistItem {
    pub path: ItemPath,
    pub duration: Option<Duration>,
    pub alias: Option<String>,
    pub mime_type: Option<String>,
    pub is_virtual: bool,
}

pub fn get_mime_type(path: &ItemPath) -> Option<String> {
    match path {
        ItemPath::File(canonical_path) => infer::get_from_path(canonical_path.as_path())
            .ok()
            .flatten()
            .map(|t| t.mime_type().to_string()),
        ItemPath::Url(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn item_path_as_file_returns_path_for_file() {
        let path = ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/file.mp4")));
        assert!(path.as_file().is_some());
        assert!(path.as_url().is_none());
    }

    #[test]
    fn item_path_as_url_returns_url_for_url() {
        let path = ItemPath::Url("https://example.com/video.mp4".to_string());
        assert!(path.as_url().is_some());
        assert!(path.as_file().is_none());
    }

    #[test]
    fn item_path_is_url_returns_true_for_url() {
        let path = ItemPath::Url("https://example.com/video.mp4".to_string());
        assert!(path.is_url());
        let file_path = ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/file.mp4")));
        assert!(!file_path.is_url());
    }

    #[test]
    fn item_path_file_stem_returns_filename_without_extension() {
        let path = ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/video.mp4")));
        assert_eq!(path.file_stem(), Some("video"));
    }

    #[test]
    fn item_path_file_stem_returns_none_for_url() {
        let path = ItemPath::Url("https://example.com/video.mp4".to_string());
        assert_eq!(path.file_stem(), None);
    }

    #[test]
    fn item_path_to_string_lossy_returns_url_for_url() {
        let path = ItemPath::Url("https://example.com/video.mp4".to_string());
        assert_eq!(path.to_string_lossy(), "https://example.com/video.mp4");
    }

    #[test]
    fn playlist_item_can_be_created() {
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/file.mp4"))),
            duration: Some(Duration::from_secs(120)),
            alias: Some("My Video".to_string()),
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
        };
        assert_eq!(item.duration, Some(Duration::from_secs(120)));
        assert_eq!(item.alias, Some("My Video".to_string()));
    }
}
