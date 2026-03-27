use super::{ShowNotesEntry, ShowNotesFormat};

pub struct YoutubeFormat;

impl ShowNotesFormat for YoutubeFormat {
    fn format(&self, entries: &[ShowNotesEntry]) -> String {
        entries
            .iter()
            .filter(|e| !e.sources.is_empty())
            .filter_map(|entry| entry.sources.first().map(|url| format!("- {}", url)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn name(&self) -> &'static str {
        "youtube"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, alias: Option<&str>, sources: Vec<&str>) -> ShowNotesEntry {
        ShowNotesEntry {
            path: format!("/path/{name}"),
            filename: name.to_string(),
            alias: alias.map(ToString::to_string),
            sources: sources.iter().map(ToString::to_string).collect(),
        }
    }

    #[test]
    fn format_creates_dashed_list() {
        // Given entries with sources.
        let entries = vec![entry("video.mp4", None, vec!["https://example.com"])];

        // When formatting as youtube.
        let output = YoutubeFormat.format(&entries);

        // Then dashed list is created.
        assert_eq!(output, "- https://example.com");
    }

    #[test]
    fn format_ignores_alias() {
        // Given entry with alias.
        let entries = vec![entry(
            "video.mp4",
            Some("My Video"),
            vec!["https://example.com"],
        )];

        // When formatting as youtube.
        let output = YoutubeFormat.format(&entries);

        // Then alias is not included in output.
        assert_eq!(output, "- https://example.com");
        assert!(!output.contains("My Video"));
    }

    #[test]
    fn format_uses_only_first_source() {
        // Given entry with multiple sources.
        let entries = vec![entry(
            "video.mp4",
            None,
            vec![
                "https://first.com",
                "https://second.com",
                "https://third.com",
            ],
        )];

        // When formatting as youtube.
        let output = YoutubeFormat.format(&entries);

        // Then only first source appears in output.
        assert_eq!(output, "- https://first.com");
        assert!(!output.contains("second.com"));
        assert!(!output.contains("third.com"));
    }

    #[test]
    fn format_skips_entries_without_sources() {
        // Given entries where one has no sources.
        let entries = vec![
            entry("video.mp4", None, vec!["https://example.com"]),
            entry("no-source.mp4", None, vec![]),
        ];

        // When formatting as youtube.
        let output = YoutubeFormat.format(&entries);

        // Then only entry with sources is included.
        assert_eq!(output, "- https://example.com");
    }

    #[test]
    fn format_empty_entries() {
        // Given empty entries list.
        let entries: Vec<ShowNotesEntry> = vec![];

        // When formatting as youtube.
        let output = YoutubeFormat.format(&entries);

        // Then output is empty.
        assert!(output.is_empty());
    }
}
