mod html;
mod markdown;
mod plain;

use std::path::Path;

use derive_more::Debug;

/// A single entry in generated show notes.
///
/// Contains all the information for one media file in the output,
/// including its path, alias, duration, source URLs, and notes content.
#[derive(Debug, Clone)]
pub struct ShowNotesEntry {
    /// The full path to the media file.
    pub path: String,
    /// The basename of the file (e.g., "video.mp4").
    pub filename: String,
    /// Optional display name override for the entry.
    pub alias: Option<String>,
    /// Source URLs associated with this entry.
    pub sources: Vec<String>,
}

impl ShowNotesEntry {
    pub fn display_name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.filename)
    }
}

/// Trait for show notes output formatters.
///
/// Implementations define how to format a collection of show notes entries
/// into a specific output format (e.g., markdown, HTML, plain text).
pub trait ShowNotesFormat: Send + Sync {
    /// Formats the provided entries into a string representation.
    fn format(&self, entries: &[ShowNotesEntry]) -> String;

    /// Returns the name identifier for this format.
    ///
    /// Used for format selection via the registry.
    fn name(&self) -> &'static str;
}

/// Registry of available output formats for show notes.
///
/// Manages the available format implementations (HTML, Markdown, plain text)
/// and provides lookup by format name for the generate command.
pub struct FormatRegistry {
    /// Registered format implementations, mapping format names to their formatters.
    formats: Vec<Box<dyn ShowNotesFormat>>,
}

impl FormatRegistry {
    pub fn new() -> Self {
        Self {
            formats: vec![
                Box::new(markdown::MarkdownFormat),
                Box::new(plain::PlainFormat),
                Box::new(html::HtmlFormat),
            ],
        }
    }

    pub fn get(&self, name: &str) -> Option<&dyn ShowNotesFormat> {
        self.formats
            .iter()
            .find(|f| f.name().eq_ignore_ascii_case(name))
            .map(AsRef::as_ref)
    }

    pub fn available_formats(&self) -> Vec<&'static str> {
        self.formats.iter().map(|f| f.name()).collect()
    }
}

impl Default for FormatRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
pub fn extract_filename(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map_or_else(|| path.to_string(), |n| n.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_filename_returns_basename() {
        assert_eq!(extract_filename("/path/to/video.mp4"), "video.mp4");
        assert_eq!(extract_filename("video.mp4"), "video.mp4");
        assert_eq!(extract_filename("./video.mp4"), "video.mp4");
    }

    #[test]
    fn display_name_uses_alias_when_present() {
        let entry = ShowNotesEntry {
            path: "/path/to/video.mp4".to_string(),
            filename: "video.mp4".to_string(),
            alias: Some("My Cool Video".to_string()),
            sources: vec![],
        };
        assert_eq!(entry.display_name(), "My Cool Video");
    }

    #[test]
    fn display_name_uses_filename_when_no_alias() {
        let entry = ShowNotesEntry {
            path: "/path/to/video.mp4".to_string(),
            filename: "video.mp4".to_string(),
            alias: None,
            sources: vec![],
        };
        assert_eq!(entry.display_name(), "video.mp4");
    }

    #[test]
    fn format_registry_returns_markdown() {
        let registry = FormatRegistry::new();
        let format = registry.get("markdown");
        assert!(format.is_some());
        assert_eq!(format.unwrap().name(), "markdown");
    }

    #[test]
    fn format_registry_is_case_insensitive() {
        let registry = FormatRegistry::new();
        assert!(registry.get("MARKDOWN").is_some());
        assert!(registry.get("Markdown").is_some());
    }

    #[test]
    fn format_registry_returns_none_for_unknown() {
        let registry = FormatRegistry::new();
        assert!(registry.get("unknown").is_none());
    }

    #[test]
    fn format_registry_lists_all_formats() {
        let registry = FormatRegistry::new();
        let formats = registry.available_formats();
        assert!(formats.contains(&"markdown"));
        assert!(formats.contains(&"plain"));
        assert!(formats.contains(&"html"));
    }
}
